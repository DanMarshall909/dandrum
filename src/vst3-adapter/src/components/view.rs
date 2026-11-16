//! Affine VST3 view projection. The instrument authority remains in Processor.
use super::{ControllerCore, platform};
use dandrum_trigger_editor::NativeEditor;
use std::{
    ffi::c_char,
    rc::{Rc, Weak},
};
use vst3::{Class, ComWrapper, Steinberg::*};

const WIDTH: i32 = NativeEditor::WIDTH as i32;
const HEIGHT: i32 = NativeEditor::HEIGHT as i32;

pub(super) fn create(controller: &Rc<ControllerCore>) -> *mut IPlugView {
    ComWrapper::new(View {
        thread: controller.thread,
        controller: Rc::downgrade(controller),
    })
    .to_com_ptr::<IPlugView>()
    .expect("view includes the standard interface")
    .into_raw()
}

/// Compare only the known SDK name, including its NUL. Never scan an unknown
/// host string without a bound, and stop before reading beyond a shorter name.
pub(super) unsafe fn matches(value: *const c_char, expected: &[u8]) -> bool {
    !value.is_null()
        && expected.iter().enumerate().all(|(index, byte)| {
            // SAFETY: the caller supplies a NUL-terminated SDK string. Matching
            // stops at its first differing byte, including a shorter NUL.
            unsafe { *value.add(index) as u8 == *byte }
        })
}

struct View {
    thread: libc::pid_t,
    controller: Weak<ControllerCore>,
}
impl Class for View {
    type Interfaces = (IPlugView,);
}
impl View {
    fn live(&self) -> bool {
        // Reject a foreign thread before upgrading or inspecting affine Rc data.
        self.thread == platform::thread_id()
            && self.controller.upgrade().is_some_and(|controller| {
                controller.main()
                    && !controller.closing.get()
                    && controller
                        .projection
                        .try_borrow()
                        .is_ok_and(|p| p.is_some())
            })
    }
}
impl IPlugViewTrait for View {
    unsafe fn isPlatformTypeSupported(&self, kind: FIDString) -> tresult {
        if self.live()
            // SAFETY: the host provides the optional SDK platform name.
            && unsafe { matches(kind, b"X11EmbedWindowID\0") }
        {
            kResultOk
        } else {
            kResultFalse
        }
    }
    unsafe fn attached(&self, parent: *mut std::ffi::c_void, kind: FIDString) -> tresult {
        // SAFETY: same live host platform string contract as the query above.
        if unsafe { self.isPlatformTypeSupported(kind) } != kResultOk {
            return kResultFalse;
        }
        if parent.is_null() {
            return kInvalidArgument;
        }
        // Native attachment is a separate capability from view metadata.
        kResultFalse
    }
    unsafe fn removed(&self) -> tresult {
        kResultOk
    }
    unsafe fn onWheel(&self, _distance: f32) -> tresult {
        kResultFalse
    }
    unsafe fn onKeyDown(&self, _key: char16, _code: int16, _modifiers: int16) -> tresult {
        kResultFalse
    }
    unsafe fn onKeyUp(&self, _key: char16, _code: int16, _modifiers: int16) -> tresult {
        kResultFalse
    }
    unsafe fn getSize(&self, size: *mut ViewRect) -> tresult {
        if !self.live() {
            return kResultFalse;
        }
        if size.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: one writable SDK output, checked non-null above.
        unsafe {
            *size = ViewRect {
                left: 0,
                top: 0,
                right: WIDTH,
                bottom: HEIGHT,
            };
        }
        kResultOk
    }
    unsafe fn onSize(&self, size: *mut ViewRect) -> tresult {
        if !self.live() {
            return kResultFalse;
        }
        // SAFETY: the host supplies one readable SDK rectangle or null.
        let Some(size) = (unsafe { size.as_ref() }) else {
            return kInvalidArgument;
        };
        if (
            size.right.checked_sub(size.left),
            size.bottom.checked_sub(size.top),
        ) == (Some(WIDTH), Some(HEIGHT))
        {
            kResultOk
        } else {
            kInvalidArgument
        }
    }
    unsafe fn onFocus(&self, _state: TBool) -> tresult {
        if self.live() { kResultOk } else { kResultFalse }
    }
    unsafe fn setFrame(&self, _frame: *mut IPlugFrame) -> tresult {
        kResultOk
    }
    unsafe fn canResize(&self) -> tresult {
        kResultFalse
    }
    unsafe fn checkSizeConstraint(&self, rect: *mut ViewRect) -> tresult {
        if !self.live() {
            return kResultFalse;
        }
        // SAFETY: the host supplies one exclusive writable SDK rectangle or null.
        let Some(rect) = (unsafe { rect.as_mut() }) else {
            return kInvalidArgument;
        };
        let (Some(right), Some(bottom)) =
            (rect.left.checked_add(WIDTH), rect.top.checked_add(HEIGHT))
        else {
            return kInvalidArgument;
        };
        rect.right = right;
        rect.bottom = bottom;
        kResultOk
    }
}
