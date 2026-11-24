#![cfg(all(
    target_os = "linux",
    target_arch = "x86_64",
    feature = "trigger-editor"
))]
mod common;
use common::{runloop, *};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};
use vst3::Steinberg::{Vst::*, *};
use x11rb::{
    COPY_DEPTH_FROM_PARENT,
    connection::Connection,
    protocol::xproto::{ConnectionExt, CreateWindowAux, ImageFormat, MapState, WindowClass},
};

use common::native::paint_package;

struct ResizeState {
    acknowledgment: Cell<Option<tresult>>,
    result: Cell<tresult>,
    during_resize: RefCell<Option<Box<dyn FnOnce()>>>,
}

struct ResizingFrame(Rc<ResizeState>);

impl vst3::Class for ResizingFrame {
    type Interfaces = (IPlugFrame,);
}

impl IPlugFrameTrait for ResizingFrame {
    unsafe fn resizeView(&self, raw: *mut IPlugView, rect: *mut ViewRect) -> tresult {
        let view = unsafe { vst3::ComRef::from_raw(raw) }.unwrap();
        self.0.acknowledgment.set(Some(unsafe { view.onSize(rect) }));
        let during_resize = self.0.during_resize.borrow_mut().take();
        if let Some(during_resize) = during_resize {
            during_resize();
        }
        self.0.result.get()
    }
}

#[test]
#[ignore = "real X11/WebKit boundary lane; run serially with an explicit display"]
fn native_view_paints_and_recreates_without_destroying_its_host_parent() {
    let bundle = Bundle::new();
    paint_package(&bundle);
    let before = std::fs::read_to_string("/proc/self/maps").unwrap();
    assert!(
        !before.contains("libwebkit2gtk"),
        "loader-only host must not preload WebKit"
    );
    let bundle=std::thread::spawn(move || {
    let module = bundle.load();
    let (clock, factory, _host) = runloop::host(&module);
    let processor = module.class_from::<IComponent>(&factory, &PROCESSOR);
    let controller = module.class_from::<IEditController>(&factory, &CONTROLLER);
    unsafe {
        assert_eq!(processor.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(controller.initialize(std::ptr::null_mut()), kResultOk);
    }
    let p = processor.cast::<IConnectionPoint>().unwrap();
    let c = controller.cast::<IConnectionPoint>().unwrap();
    unsafe {
        assert_eq!(p.connect(c.as_ptr()), kResultOk);
        assert_eq!(c.connect(p.as_ptr()), kResultOk);
    }
    clock.tick();
    let (x, screen) = x11rb::connect(None).unwrap();
    let root = &x.setup().roots[screen];
    assert_eq!(
        root.root_depth, 24,
        "the painting oracle requires qualified 24-bit X11"
    );
    let parent = x.generate_id().unwrap();
    x.create_window(
        COPY_DEPTH_FROM_PARENT,
        parent,
        root.root,
        0,
        0,
        1200,
        800,
        0,
        WindowClass::INPUT_OUTPUT,
        0,
        &CreateWindowAux::new().background_pixel(0xffffff),
    )
    .unwrap()
    .check()
    .unwrap();
    x.map_window(parent).unwrap().check().unwrap();
    let view =
        unsafe { vst3::ComPtr::from_raw(controller.createView(c"editor".as_ptr())) }.unwrap();
    let file = bundle.resources.join("editor/app.js");
    let missing = file.with_extension("missing");
    std::fs::rename(&file, &missing).unwrap();
    assert_eq!(
        unsafe { view.attached(parent as usize as *mut _, kPlatformTypeX11EmbedWindowID) },
        kResultFalse,
        "incomplete editor resources refuse native creation"
    );
    std::fs::rename(&missing, &file).unwrap();
    assert!(
        x.query_tree(parent)
            .unwrap()
            .reply()
            .unwrap()
            .children
            .is_empty()
    );
    clock.reject_register.set(true);
    assert_eq!(
        unsafe { view.attached(parent as usize as *mut _, kPlatformTypeX11EmbedWindowID) },
        kResultFalse,
        "refused host service cannot advertise a working view"
    );
    clock.reject_register.set(false);
    let dead_parent = x.generate_id().unwrap();
    assert_eq!(
        unsafe {
            view.attached(
                dead_parent as usize as *mut _,
                kPlatformTypeX11EmbedWindowID,
            )
        },
        kResultFalse,
        "invalid native parent releases partial service registration"
    );
    assert_eq!(
        clock.handlers.borrow().len(),
        1,
        "only processor service remains after partial creation"
    );
    let reentry = std::rc::Rc::new(std::cell::Cell::new(None));
    let observed = reentry.clone();
    let nested_controller = (*controller).clone();
    let nested_view = view.clone();
    *clock.register_callback.borrow_mut() = Some(Box::new(move || {
        observed.set(Some(unsafe {
            (nested_controller.terminate(), nested_view.removed())
        }));
    }));
    for cycle in 0..4 {
        let view = if cycle == 0 {
            view.clone()
        } else {
            unsafe { vst3::ComPtr::from_raw(controller.createView(c"editor".as_ptr())) }.unwrap()
        };
        let factor = if cycle % 2 == 0 { 1.0 } else { 2.0 };
        let (width, height) = if factor == 1.0 { (1200, 800) } else { (2400, 1600) };
        let scale = view.cast::<IPlugViewContentScaleSupport>().expect("host content scale interface");
        assert_eq!(unsafe { scale.setContentScaleFactor(factor) }, kResultOk);
        x.configure_window(parent, &x11rb::protocol::xproto::ConfigureWindowAux::new().width(width).height(height)).unwrap().check().unwrap();
        assert_eq!(
            unsafe { view.attached(parent as usize as *mut _, kPlatformTypeX11EmbedWindowID) },
            kResultOk,
            "a connected native view must attach in cycle {cycle}"
        );
        assert_eq!(unsafe { scale.setContentScaleFactor(1.5) }, kResultFalse, "an attached view cannot change host size without a frame");
        let children = x.query_tree(parent).unwrap().reply().unwrap().children;
        assert_eq!(children.len(), 1, "exactly one direct native child");
        if cycle == 0 {
            assert_eq!(
                reentry.get(),
                Some((kResultFalse, kResultFalse)),
                "registration reentry cannot terminate an attachment in progress"
            );
        }
        assert_eq!(
            unsafe { view.attached(parent as usize as *mut _, kPlatformTypeX11EmbedWindowID) },
            kResultFalse,
            "an attached view cannot create a second child"
        );
        let other =
            unsafe { vst3::ComPtr::from_raw(controller.createView(c"editor".as_ptr())) }.unwrap();
        assert_eq!(
            unsafe { other.attached(parent as usize as *mut _, kPlatformTypeX11EmbedWindowID) },
            kResultFalse,
            "one controller has one active native projection"
        );
        drop(other);
        let child = children[0];
        let geometry = x.get_geometry(child).unwrap().reply().unwrap();
        assert_eq!(
            (geometry.x, geometry.y, geometry.width, geometry.height),
            (0, 0, width as u16, height as u16)
        );
        assert_eq!(
            x.get_window_attributes(child)
                .unwrap()
                .reply()
                .unwrap()
                .map_state,
            MapState::VIEWABLE
        );
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut painted = false;
        while Instant::now() < deadline {
            clock.tick();
            let pixels = x
                .get_image(ImageFormat::Z_PIXMAP, parent, (width - 12) as i16, (height - 12) as i16, 1, 1, u32::MAX)
                .unwrap()
                .reply()
                .unwrap()
                .data;
            // Qualified Linux X11 is little endian BGRX at depth24.
            painted = pixels.get(..3) == Some(&[0x56, 0x34, 0x12]);
            if painted {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            painted,
            "the actual WebKit child must paint the specified CSS colour"
        );
        if cycle == 0 {
            let resize = Rc::new(ResizeState {
                acknowledgment: Cell::new(None),
                result: Cell::new(kResultFalse),
                during_resize: RefCell::new(None),
            });
            let frame = vst3::ComWrapper::new(ResizingFrame(resize.clone()))
            .to_com_ptr::<IPlugFrame>()
            .unwrap();
            assert_eq!(unsafe { view.setFrame(frame.as_ptr()) }, kResultOk);
            // Rounding this factor gives 1600x1066. Its aspect ratio differs
            // from the unchanged 1200x800 parent, so a provisional scale would
            // resize the actual child rather than inheriting the parent size.
            assert_eq!(unsafe { scale.setContentScaleFactor(1.333) }, kResultFalse);
            assert_eq!(resize.acknowledgment.get(), Some(kResultOk));
            let geometry = x.get_geometry(child).unwrap().reply().unwrap();
            assert_eq!(
                (geometry.width, geometry.height),
                (1200, 800),
                "a refusal after onSize must preserve the native child geometry"
            );

            let nested_clock = clock.clone();
            *resize.during_resize.borrow_mut() = Some(Box::new(move || {
                let (x, _) = x11rb::connect(None).unwrap();
                use x11rb::protocol::xproto::ConfigureWindowAux;
                x.configure_window(parent, &ConfigureWindowAux::new().width(2400).height(1600))
                    .unwrap()
                    .check()
                    .unwrap();
                nested_clock.tick();
                x.configure_window(parent, &ConfigureWindowAux::new().width(1200).height(800))
                    .unwrap()
                    .check()
                    .unwrap();
            }));
            assert_eq!(unsafe { scale.setContentScaleFactor(1.5) }, kResultFalse);
            assert_eq!(resize.acknowledgment.get(), Some(kResultOk));
            let geometry = x.get_geometry(child).unwrap().reply().unwrap();
            assert_eq!(
                (geometry.width, geometry.height),
                (1200, 800),
                "a timer inside resizeView must not adopt a provisional parent size"
            );

            resize.result.set(kResultOk);
            assert_eq!(unsafe { scale.setContentScaleFactor(1.333) }, kResultOk);
            let geometry = x.get_geometry(child).unwrap().reply().unwrap();
            assert_eq!(
                (geometry.width, geometry.height),
                (1600, 1066),
                "the same acknowledged size takes effect when the host accepts"
            );
            assert_eq!(unsafe { scale.setContentScaleFactor(1.0) }, kResultOk);
            assert_eq!(unsafe { view.setFrame(std::ptr::null_mut()) }, kResultOk);

            // REAPER's Linux high-DPI path reports a 1200x800 ViewRect but
            // realizes its X11 embedding parent at 2400x1600. Simulate the
            // post-attachment host resize, without an explicit scale-2 call.
            x.configure_window(parent, &x11rb::protocol::xproto::ConfigureWindowAux::new().width(2400).height(1600)).unwrap().check().unwrap();
            let mut host_rect = ViewRect { left: 0, top: 0, right: 1200, bottom: 800 };
            clock.tick();
            let geometry = x.get_geometry(child).unwrap().reply().unwrap();
            assert_eq!((geometry.width, geometry.height), (2400, 1600), "a host resize completed after onSize must scale on the next host tick");
            assert_eq!(unsafe { view.onSize(&mut host_rect) }, kResultOk);
            assert_eq!(unsafe { view.getSize(&mut host_rect) }, kResultOk);
            assert_eq!((host_rect.right, host_rect.bottom), (1200, 800), "retain the host's coordinate space");
            x.configure_window(parent, &x11rb::protocol::xproto::ConfigureWindowAux::new().width(1200).height(800)).unwrap().check().unwrap();
            assert_eq!(unsafe { view.onSize(&mut host_rect) }, kResultOk);
            let geometry = x.get_geometry(child).unwrap().reply().unwrap();
            assert_eq!((geometry.width, geometry.height), (1200, 800), "returning to scale one restores physical geometry");
            clock.reject_unregister.set(true);
            assert_eq!(
                unsafe { view.removed() },
                kResultFalse,
                "refused unregister retains a retryable cleanup token"
            );
            assert_eq!(clock.handlers.borrow().len(), 2);
            let blocked =
                unsafe { vst3::ComPtr::from_raw(controller.createView(c"editor".as_ptr())) }
                    .unwrap();
            assert_eq!(
                unsafe {
                    blocked.attached(parent as usize as *mut _, kPlatformTypeX11EmbedWindowID)
                },
                kResultFalse,
                "pending cleanup cannot be superseded by another native view"
            );
            assert!(
                x.query_tree(parent)
                    .unwrap()
                    .reply()
                    .unwrap()
                    .children
                    .is_empty()
            );
            drop(blocked);
            clock.reject_unregister.set(false);
            assert_eq!(unsafe { view.removed() }, kResultOk);
        } else if cycle == 1 {
            assert_eq!(unsafe { view.removed() }, kResultOk);
            let disposable =
                unsafe { vst3::ComPtr::from_raw(controller.createView(c"editor".as_ptr())) }
                    .unwrap();
            assert_eq!(
                unsafe {
                    disposable.attached(parent as usize as *mut _, kPlatformTypeX11EmbedWindowID)
                },
                kResultOk
            );
            assert_eq!(
                x.query_tree(parent)
                    .unwrap()
                    .reply()
                    .unwrap()
                    .children
                    .len(),
                1
            );
            drop(disposable);
            assert!(
                x.query_tree(parent)
                    .unwrap()
                    .reply()
                    .unwrap()
                    .children
                    .is_empty(),
                "final IPlugView release cleans up without an explicit removed call"
            );
        } else if cycle == 2 {
            assert_eq!(
                unsafe { c.disconnect(p.as_ptr()) },
                kResultOk,
                "controller disconnection revokes the actual native projection"
            );
        } else {
            assert_eq!(
                unsafe { controller.terminate() },
                kResultOk,
                "controller termination revokes the child even while IPlugView is retained"
            );
        }
        assert!(
            x.query_tree(parent)
                .unwrap()
                .reply()
                .unwrap()
                .children
                .is_empty(),
            "removal destroys the native child synchronously"
        );
        assert!(
            x.get_geometry(child).unwrap().reply().is_err(),
            "the previous child XID is destroyed"
        );
        assert_eq!(
            x.get_window_attributes(parent)
                .unwrap()
                .reply()
                .unwrap()
                .map_state,
            MapState::VIEWABLE
        );
        assert_eq!(
            unsafe { view.removed() },
            kResultOk,
            "repeated removal is harmless"
        );
        drop(view);
        clock.tick();
        if cycle == 2 {
            unsafe {
                assert_eq!(p.connect(c.as_ptr()), kResultOk);
                assert_eq!(c.connect(p.as_ptr()), kResultOk);
            }
        }
    }
    unsafe {
        assert_eq!(controller.terminate(), kResultOk);
        assert_eq!(processor.terminate(), kResultOk);
    }
    assert!(
        clock.handlers.borrow().is_empty(),
        "all owned host timers are released"
    );
    let dying_processor = module.class_from::<IComponent>(&factory, &PROCESSOR);
    let surviving_controller = module.class_from::<IEditController>(&factory, &CONTROLLER);
    unsafe {
        assert_eq!(dying_processor.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(
            surviving_controller.initialize(std::ptr::null_mut()),
            kResultOk
        );
    }
    let dying = dying_processor.cast::<IConnectionPoint>().unwrap();
    let surviving = surviving_controller.cast::<IConnectionPoint>().unwrap();
    unsafe {
        assert_eq!(dying.connect(surviving.as_ptr()), kResultOk);
        assert_eq!(surviving.connect(dying.as_ptr()), kResultOk);
    }
    let retained_view =
        unsafe { vst3::ComPtr::from_raw(surviving_controller.createView(c"editor".as_ptr())) }
            .unwrap();
    let closing_parent = x.generate_id().unwrap();
    x.create_window(
        COPY_DEPTH_FROM_PARENT,
        closing_parent,
        root.root,
        0,
        0,
        1200,
        800,
        0,
        WindowClass::INPUT_OUTPUT,
        0,
        &CreateWindowAux::new(),
    )
    .unwrap()
    .check()
    .unwrap();
    x.map_window(closing_parent).unwrap().check().unwrap();
    assert_eq!(
        unsafe {
            retained_view.attached(closing_parent as usize as *mut _, kPlatformTypeX11EmbedWindowID)
        },
        kResultOk
    );
    let children = x.query_tree(closing_parent).unwrap().reply().unwrap().children;
    let [closing_child] = children.as_slice() else {
        panic!("expected one live editor child");
    };
    let (closing_child, root_window) = (*closing_child, root.root);
    let resize = Rc::new(ResizeState {
        acknowledgment: Cell::new(None),
        result: Cell::new(kResultOk),
        during_resize: RefCell::new(Some(Box::new(move || {
            let (x, _) = x11rb::connect(None).unwrap();
            // Keep the plug alive so the view can still dispose it normally.
            x.reparent_window(closing_child, root_window, 0, 0)
                .unwrap()
                .check()
                .unwrap();
            x.destroy_window(closing_parent).unwrap().check().unwrap();
        }))),
    });
    let frame = vst3::ComWrapper::new(ResizingFrame(resize.clone()))
        .to_com_ptr::<IPlugFrame>()
        .unwrap();
    let scale = retained_view.cast::<IPlugViewContentScaleSupport>().unwrap();
    let mut rect = ViewRect {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    unsafe {
        assert_eq!(retained_view.setFrame(frame.as_ptr()), kResultOk);
        assert_eq!(
            scale.setContentScaleFactor(1.5),
            kResultFalse,
            "a host acknowledgment cannot commit a resize after native synchronization fails"
        );
        assert_eq!(resize.acknowledgment.get(), Some(kResultOk));
        assert_eq!(retained_view.getSize(&mut rect), kResultOk);
        assert_eq!((rect.right, rect.bottom), (1200, 800));
        assert_eq!(retained_view.setFrame(std::ptr::null_mut()), kResultOk);
    }
    let geometry = x.get_geometry(closing_child).unwrap().reply().unwrap();
    assert_eq!((geometry.width, geometry.height), (1200, 800));
    assert_eq!(unsafe { retained_view.removed() }, kResultOk);
    assert!(x.get_geometry(closing_child).unwrap().reply().is_err());

    x.configure_window(
        parent,
        &x11rb::protocol::xproto::ConfigureWindowAux::new()
            .width(1200)
            .height(800),
    )
    .unwrap()
    .check()
    .unwrap();
    assert_eq!(
        unsafe { retained_view.attached(parent as usize as *mut _, kPlatformTypeX11EmbedWindowID) },
        kResultOk
    );
    let children = x.query_tree(parent).unwrap().reply().unwrap().children;
    let [outer_child] = children.as_slice() else {
        panic!("expected one live editor child");
    };
    let accepted = x.get_geometry(*outer_child).unwrap().reply().unwrap();
    assert_eq!((accepted.width, accepted.height), (1200, 800));
    let children = x.query_tree(*outer_child).unwrap().reply().unwrap().children;
    let [gtk_plug] = children.as_slice() else {
        panic!("expected one live GTK plug");
    };
    let gtk_plug = *gtk_plug;
    *resize.during_resize.borrow_mut() = Some(Box::new(move || {
        let (x, _) = x11rb::connect(None).unwrap();
        x.destroy_window(gtk_plug).unwrap().check().unwrap();
    }));
    assert_eq!(unsafe { retained_view.setFrame(frame.as_ptr()) }, kResultOk);
    let gdk = unsafe {
        libloading::os::unix::Library::open(
            Some("libgdk-3.so.0"),
            libloading::os::unix::RTLD_NOW | libc::RTLD_NOLOAD,
        )
    }
    .unwrap();
    type GdkDisplay = *mut std::ffi::c_void;
    type DisplayTrap = unsafe extern "C" fn(GdkDisplay);
    let (result, sdk_size_result, geometry, terminate_result) = unsafe {
        let default_display = gdk
            .get::<unsafe extern "C" fn() -> GdkDisplay>(b"gdk_display_get_default\0")
            .unwrap();
        let trap_push = gdk
            .get::<DisplayTrap>(b"gdk_x11_display_error_trap_push\0")
            .unwrap();
        let trap_pop = gdk
            .get::<DisplayTrap>(b"gdk_x11_display_error_trap_pop_ignored\0")
            .unwrap();
        let display = default_display();
        assert!(!display.is_null());
        // The GTK connection can also observe the deliberately destroyed plug.
        // Trap those errors while the product's checked X11 requests stay live.
        trap_push(display);
        let result = scale.setContentScaleFactor(1.333);
        let sdk_size_result = retained_view.getSize(&mut rect);
        let geometry = x.get_geometry(*outer_child).unwrap().reply().unwrap();
        let terminate_result = dying_processor.terminate();
        trap_pop(display);
        (result, sdk_size_result, geometry, terminate_result)
    };
    assert_eq!(resize.acknowledgment.get(), Some(kResultOk));
    assert_eq!(result, kResultFalse);
    assert_eq!(sdk_size_result, kResultOk);
    assert_eq!((rect.right, rect.bottom), (1200, 800));
    assert_eq!(
        (geometry.width, geometry.height),
        (1200, 800),
        "a checked GTK plug resize failure must restore the surviving outer child"
    );
    assert_eq!(terminate_result, kResultOk);
    assert!(
        x.get_geometry(*outer_child).unwrap().reply().is_err(),
        "processor termination revokes the real child while its controller and view are retained"
    );
    drop(scale);
    assert_eq!(unsafe { surviving_controller.terminate() }, kResultOk);
    assert!(clock.handlers.borrow().is_empty());
    x.destroy_window(parent).unwrap().check().unwrap();
    drop((retained_view,surviving,dying,surviving_controller,dying_processor,view,p,c,controller,processor,factory,_host));
    // Take only a balanced reference to the already-loaded GLib dependency;
    // the host never loads or retains WebKit independently of the product.
    let glib=unsafe{libloading::os::unix::Library::open(Some("libglib-2.0.so.0"),libloading::os::unix::RTLD_NOW|libc::RTLD_NOLOAD)}.unwrap();
    let iteration=unsafe{*glib.get::<unsafe extern "C" fn(*mut std::ffi::c_void,i32)->i32>(b"g_main_context_iteration\0").unwrap()};
    drop(module);
    for _ in 0..32 {unsafe{iteration(std::ptr::null_mut(),0);}}
    glib.close().unwrap();
    bundle
    }).join().expect("native owner thread and its TLS destructors complete");
    let mapped = std::fs::read_to_string("/proc/self/maps").unwrap();
    assert!(
        mapped.contains(bundle.binary.to_str().unwrap()),
        "exact product code remains after final module release and UI-thread TLS destruction"
    );
    assert!(
        mapped.contains("libwebkit2gtk"),
        "WebKit dependency residency outlives the native owner thread"
    );
    let balanced = unsafe {
        libloading::os::unix::Library::open(
            Some(&bundle.binary),
            libloading::os::unix::RTLD_NOW | libc::RTLD_NOLOAD,
        )
    }
    .expect("balanced NOLOAD rescan of exact already-mapped product");
    balanced.close().unwrap();
    println!(
        "loader-only VST3: native objects/timers destroyed, module released, GLib dispatched, UI thread/TLS exited, product and WebKit remain resident after balanced NOLOAD rescan"
    );
}
