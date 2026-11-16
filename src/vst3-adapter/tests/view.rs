#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod common;
use common::{Bundle, CONTROLLER};
use vst3::Steinberg::{Vst::*, *};

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
