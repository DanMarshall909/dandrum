#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod common;
use common::{Bundle, CONTROLLER};
use vst3::Steinberg::{Vst::*, *};

#[cfg(feature = "trigger-editor")]
#[test]
fn content_scaling_negotiates_with_the_frame_and_preserves_refused_size() {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    struct Frame {
        refuse: Rc<Cell<bool>>,
        acknowledge: Rc<Cell<bool>>,
        requests: Rc<RefCell<Vec<(i32, i32)>>>,
        dropped: Rc<Cell<bool>>,
    }
    impl Drop for Frame {
        fn drop(&mut self) {
            self.dropped.set(true);
        }
    }
    impl vst3::Class for Frame {
        type Interfaces = (IPlugFrame,);
    }
    impl IPlugFrameTrait for Frame {
        unsafe fn resizeView(&self, raw: *mut IPlugView, rect: *mut ViewRect) -> tresult {
            let view = unsafe { vst3::ComRef::from_raw(raw) }.unwrap();
            let rect = unsafe { rect.as_ref() }.unwrap();
            self.requests.borrow_mut().push((rect.right, rect.bottom));
            let mut current = ViewRect {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            assert_eq!(unsafe { view.getSize(&mut current) }, kResultOk);
            assert_eq!(
                (current.right, current.bottom),
                (1200, 800),
                "getSize stays at the old physical size until onSize"
            );
            assert_eq!(
                unsafe {
                    view.cast::<IPlugViewContentScaleSupport>()
                        .unwrap()
                        .setContentScaleFactor(1.5)
                },
                kResultFalse,
                "recursive scaling is rejected"
            );
            let mut constrained = ViewRect {
                left: 0,
                top: 0,
                right: rect.right,
                bottom: rect.bottom,
            };
            assert_eq!(
                unsafe { view.checkSizeConstraint(&mut constrained) },
                kResultOk
            );
            assert_eq!(
                (constrained.right, constrained.bottom),
                (rect.right, rect.bottom),
                "constraints reflect the requested scale while getSize still reports the old size"
            );
            if self.refuse.get() {
                return kResultFalse;
            }
            if !self.acknowledge.get() {
                return kResultOk;
            }
            let mut accepted = ViewRect {
                left: 0,
                top: 0,
                right: rect.right,
                bottom: rect.bottom,
            };
            unsafe { view.onSize(&mut accepted) }
        }
    }
    let bundle = Bundle::new();
    let module = bundle.load();
    let controller = module.class::<IEditController>(&CONTROLLER);
    let refuse = Rc::new(Cell::new(true));
    let acknowledge = Rc::new(Cell::new(false));
    let requests = Rc::new(RefCell::new(Vec::new()));
    let dropped = Rc::new(Cell::new(false));
    let frame = vst3::ComWrapper::new(Frame {
        refuse: refuse.clone(),
        acknowledge: acknowledge.clone(),
        requests: requests.clone(),
        dropped: dropped.clone(),
    })
    .to_com_ptr::<IPlugFrame>()
    .unwrap();
    unsafe {
        assert_eq!(controller.initialize(std::ptr::null_mut()), kResultOk);
        let view = vst3::ComPtr::from_raw(controller.createView(c"editor".as_ptr())).unwrap();
        let scale = view.cast::<IPlugViewContentScaleSupport>().unwrap();
        assert_eq!(view.setFrame(frame.as_ptr()), kResultOk);
        drop(frame);
        assert!(!dropped.get(), "the view retains its host frame");
        assert_eq!(scale.setContentScaleFactor(2.0), kResultFalse);
        let mut rect = ViewRect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        assert_eq!(view.getSize(&mut rect), kResultOk);
        assert_eq!((rect.right, rect.bottom), (1200, 800));
        assert_eq!(
            view.onSize(&mut rect),
            kResultOk,
            "a refused scale did not change constraints"
        );
        refuse.set(false);
        assert_eq!(
            scale.setContentScaleFactor(2.0),
            kResultFalse,
            "an unacknowledged resize cannot change the accepted geometry"
        );
        assert_eq!(view.getSize(&mut rect), kResultOk);
        assert_eq!((rect.right, rect.bottom), (1200, 800));
        acknowledge.set(true);
        assert_eq!(scale.setContentScaleFactor(2.0), kResultOk);
        assert_eq!(view.getSize(&mut rect), kResultOk);
        assert_eq!((rect.right, rect.bottom), (2400, 1600));
        assert_eq!(
            &*requests.borrow(),
            &[(2400, 1600), (2400, 1600), (2400, 1600)]
        );
        assert_eq!(view.setFrame(std::ptr::null_mut()), kResultOk);
        assert!(
            dropped.get(),
            "clearing the frame releases exactly its retained lifetime"
        );
        assert_eq!(controller.terminate(), kResultOk);
    }
}

#[cfg(feature = "trigger-editor")]
#[test]
fn host_content_scale_preserves_logical_layout_and_rejects_invalid_geometry() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let controller = module.class::<IEditController>(&CONTROLLER);
    unsafe {
        assert_eq!(controller.initialize(std::ptr::null_mut()), kResultOk);
        let view = vst3::ComPtr::from_raw(controller.createView(c"editor".as_ptr())).unwrap();
        let scale = view
            .cast::<IPlugViewContentScaleSupport>()
            .expect("native view must accept the host's content scale");
        let mut rect = ViewRect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        for (factor, width, height) in [
            (2.0, 2400, 1600),
            (1.333, 1600, 1066),
            (1.25, 1500, 1000),
            (1.0, 1200, 800),
        ] {
            assert_eq!(scale.setContentScaleFactor(factor), kResultOk);
            assert_eq!(view.getSize(&mut rect), kResultOk);
            assert_eq!((rect.right, rect.bottom), (width, height));
            assert_eq!(view.onSize(&mut rect), kResultOk);
            rect.left = 3;
            rect.top = 7;
            assert_eq!(view.checkSizeConstraint(&mut rect), kResultOk);
            assert_eq!((rect.right, rect.bottom), (3 + width, 7 + height));
            rect.left = 0;
            rect.top = 0;
        }
        for factor in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::MAX, 0.00001] {
            assert_eq!(scale.setContentScaleFactor(factor), kInvalidArgument);
            assert_eq!(view.getSize(&mut rect), kResultOk);
            assert_eq!((rect.right, rect.bottom), (1200, 800));
        }
        let raw_scale = scale.as_ptr() as usize;
        assert_eq!(
            std::thread::spawn(move || {
                let scale =
                    vst3::ComRef::from_raw(raw_scale as *mut IPlugViewContentScaleSupport).unwrap();
                scale.setContentScaleFactor(2.0)
            })
            .join()
            .unwrap(),
            kResultFalse,
            "a foreign thread cannot change UI geometry"
        );
        assert_eq!(view.getSize(&mut rect), kResultOk);
        assert_eq!((rect.right, rect.bottom), (1200, 800));
        assert_eq!(controller.terminate(), kResultOk);
        assert_eq!(scale.setContentScaleFactor(2.0), kResultFalse);
    }
}

#[cfg(not(feature = "trigger-editor"))]
#[test]
fn headless_adapter_does_not_advertise_a_native_view() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let controller = module.class::<IEditController>(&CONTROLLER);
    // SAFETY: the fixture retains the module/controller; names are live SDK
    // strings or the explicit missing-name case.
    unsafe {
        assert!(controller.createView(c"editor".as_ptr()).is_null());
        assert_eq!(controller.initialize(std::ptr::null_mut()), kResultOk);
        for name in [c"editor".as_ptr(), c"unknown".as_ptr(), std::ptr::null()] {
            assert!(controller.createView(name).is_null());
        }
        assert_eq!(controller.terminate(), kResultOk);
        assert!(controller.createView(c"editor".as_ptr()).is_null());
    }
}

#[cfg(feature = "trigger-editor")]
#[test]
fn initialized_native_controller_exposes_an_affine_fixed_x11_view_without_owning_the_session() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let controller = module.class::<IEditController>(&CONTROLLER);
    // SAFETY: the module/controller and all SDK buffers remain live throughout
    // these calls. The explicit null cases exercise optional/missing inputs.
    let view = unsafe {
        assert!(controller.createView(c"editor".as_ptr()).is_null());
        assert_eq!(controller.initialize(std::ptr::null_mut()), kResultOk);
        for name in [
            c"".as_ptr(),
            c"other".as_ptr(),
            c"editors".as_ptr(),
            std::ptr::null(),
        ] {
            assert!(controller.createView(name).is_null());
        }
        vst3::ComPtr::from_raw(controller.createView(c"editor".as_ptr()))
            .expect("native-enabled initialized controller must expose the production X11 view")
    };
    let mut size = ViewRect {
        left: -1,
        top: -1,
        right: -1,
        bottom: -1,
    };
    unsafe {
        assert_eq!(
            view.isPlatformTypeSupported(kPlatformTypeX11EmbedWindowID),
            kResultOk
        );
        assert_eq!(view.isPlatformTypeSupported(c"HWND".as_ptr()), kResultFalse);
        assert_eq!(view.isPlatformTypeSupported(std::ptr::null()), kResultFalse);
        assert_eq!(
            view.attached(std::ptr::null_mut(), kPlatformTypeX11EmbedWindowID),
            kInvalidArgument
        );
        assert_eq!(
            view.attached(std::ptr::dangling_mut(), c"HWND".as_ptr()),
            kResultFalse
        );
        assert_eq!(
            view.attached(std::ptr::dangling_mut(), kPlatformTypeX11EmbedWindowID),
            kResultFalse,
            "this controller has no native host-loop service"
        );
        assert_eq!(
            view.attached(usize::MAX as *mut _, kPlatformTypeX11EmbedWindowID),
            kInvalidArgument,
            "a parent that cannot be an X11 window ID is rejected"
        );
        assert_eq!(view.getSize(std::ptr::null_mut()), kInvalidArgument);
        assert_eq!(view.getSize(&mut size), kResultOk);
        assert_eq!(
            (size.left, size.top, size.right, size.bottom),
            (0, 0, 1200, 800)
        );
        assert_eq!(view.canResize(), kResultFalse);
        assert_eq!(view.onSize(std::ptr::null_mut()), kInvalidArgument);
        for (width, height) in [(640, 800), (1200, 480)] {
            let mut wrong = ViewRect {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            };
            assert_eq!(view.onSize(&mut wrong), kInvalidArgument);
        }
        assert_eq!(view.getSize(&mut size), kResultOk);
        assert_eq!((size.right, size.bottom), (1200, 800));
        assert_eq!(view.onSize(&mut size), kResultOk);
        assert_eq!(
            view.checkSizeConstraint(std::ptr::null_mut()),
            kInvalidArgument
        );
        let mut constrained = ViewRect {
            left: 3,
            top: 7,
            right: 0,
            bottom: 0,
        };
        assert_eq!(view.checkSizeConstraint(&mut constrained), kResultOk);
        assert_eq!(
            (
                constrained.left,
                constrained.top,
                constrained.right,
                constrained.bottom
            ),
            (3, 7, 1203, 807)
        );
        for (left, top) in [(i32::MAX, 0), (0, i32::MAX)] {
            let mut overflow = ViewRect {
                left,
                top,
                right: 1,
                bottom: 2,
            };
            assert_eq!(view.checkSizeConstraint(&mut overflow), kInvalidArgument);
            assert_eq!((overflow.right, overflow.bottom), (1, 2));
        }
        assert_eq!(view.onWheel(1.0), kResultFalse);
        assert_eq!(view.onKeyDown(65, 0, 0), kResultFalse);
        assert_eq!(view.onKeyUp(65, 0, 0), kResultFalse);
        assert_eq!(view.onFocus(1), kResultOk);
        assert_eq!(view.onFocus(0), kResultOk);
        assert_eq!(view.setFrame(std::ptr::null_mut()), kResultOk);
        assert_eq!(view.removed(), kResultOk);
    }
    // The main thread retains the reference while a host calls a pure metadata
    // method on another thread. Rejection must happen before touching Rc state.
    let raw = view.as_ptr() as usize;
    assert_eq!(
        std::thread::spawn(move || unsafe {
            let view = vst3::ComRef::from_raw(raw as *mut IPlugView).unwrap();
            view.isPlatformTypeSupported(kPlatformTypeX11EmbedWindowID)
        })
        .join()
        .unwrap(),
        kResultFalse
    );
    unsafe {
        assert_eq!(controller.terminate(), kResultOk);
        assert_eq!(view.getSize(&mut size), kResultFalse);
        assert_eq!(view.setFrame(std::ptr::null_mut()), kResultFalse);
        assert_eq!(view.onSize(&mut size), kResultFalse);
        assert_eq!(view.checkSizeConstraint(&mut size), kResultFalse);
        assert_eq!(view.onFocus(0), kResultFalse);
        assert!(controller.createView(c"editor".as_ptr()).is_null());
    }
    drop(controller);
    assert_eq!(
        unsafe { view.isPlatformTypeSupported(kPlatformTypeX11EmbedWindowID) },
        kResultFalse
    );
}

#[cfg(feature = "trigger-editor")]
#[test]
fn connected_view_without_host_loop_fails_without_native_creation() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let processor = module.class::<IComponent>(&common::PROCESSOR);
    let controller = module.class::<IEditController>(&CONTROLLER);
    unsafe {
        assert_eq!(processor.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(controller.initialize(std::ptr::null_mut()), kResultOk);
    }
    let p = processor.cast::<IConnectionPoint>().unwrap();
    let c = controller.cast::<IConnectionPoint>().unwrap();
    unsafe {
        assert_eq!(p.connect(c.as_ptr()), kResultOk);
        assert_eq!(c.connect(p.as_ptr()), kResultOk);
        let view = vst3::ComPtr::from_raw(controller.createView(c"editor".as_ptr())).unwrap();
        assert_eq!(
            view.attached(std::ptr::dangling_mut(), kPlatformTypeX11EmbedWindowID),
            kResultFalse
        );
        assert_eq!(view.removed(), kResultOk);
        assert_eq!(controller.terminate(), kResultOk);
        assert_eq!(processor.terminate(), kResultOk);
    }
}

#[cfg(feature = "trigger-editor")]
#[test]
fn terminated_native_controller_releases_host_context_while_its_interface_is_retained() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let (clock, factory, host) = common::runloop::host(&module);
    let controller = module.class_from::<IEditController>(&factory, &CONTROLLER);
    unsafe {
        assert_eq!(controller.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(
            factory
                .cast::<IPluginFactory3>()
                .unwrap()
                .setHostContext(std::ptr::null_mut()),
            kResultOk
        );
    }
    drop(host);
    assert_eq!(
        clock.dropped.get(),
        0,
        "live native controller retains the host service capability"
    );
    assert_eq!(unsafe { controller.terminate() }, kResultOk);
    assert_eq!(
        clock.dropped.get(),
        1,
        "a retained dead interface must not keep the host context alive"
    );
}
