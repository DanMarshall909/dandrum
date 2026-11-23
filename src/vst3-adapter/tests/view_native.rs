#![cfg(all(
    target_os = "linux",
    target_arch = "x86_64",
    feature = "trigger-editor"
))]
mod common;
use common::{runloop, *};
use std::time::{Duration, Instant};
use vst3::Steinberg::{Vst::*, *};
use x11rb::{
    COPY_DEPTH_FROM_PARENT,
    connection::Connection,
    protocol::xproto::{ConnectionExt, CreateWindowAux, ImageFormat, MapState, WindowClass},
};

use common::native::paint_package;

#[test]
#[ignore = "real X11/WebKit boundary lane; run serially with an explicit display"]
fn native_view_paints_and_recreates_without_destroying_its_host_parent() {
    let bundle = Bundle::new();
    paint_package(&bundle);
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
    for cycle in 0..3 {
        let view = if cycle == 0 {
            view.clone()
        } else {
            unsafe { vst3::ComPtr::from_raw(controller.createView(c"editor".as_ptr())) }.unwrap()
        };
        assert_eq!(
            unsafe { view.attached(parent as usize as *mut _, kPlatformTypeX11EmbedWindowID) },
            kResultOk,
            "a connected native view must attach in cycle {cycle}"
        );
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
            (0, 0, 1200, 800)
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
                .get_image(ImageFormat::Z_PIXMAP, parent, 12, 12, 1, 1, u32::MAX)
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
            clock.reject_unregister.set(true);
            assert_eq!(
                unsafe { view.removed() },
                kResultFalse,
                "refused unregister retains a retryable cleanup token"
            );
            assert_eq!(clock.handlers.borrow().len(), 2);
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
    }
    unsafe {
        assert_eq!(controller.terminate(), kResultOk);
        assert_eq!(processor.terminate(), kResultOk);
    }
    assert!(
        clock.handlers.borrow().is_empty(),
        "all owned host timers are released"
    );
    x.destroy_window(parent).unwrap().check().unwrap();
}
