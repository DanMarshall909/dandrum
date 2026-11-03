//! Host-owned SDK stream; successful short I/O/EOF follows pinned MemoryStream.
use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
    rc::Rc,
};
use vst3::{Class, ComPtr, ComWrapper, Steinberg::*};
#[derive(Clone, Copy, Default)]
pub enum ReplyCount {
    #[default]
    Actual,
    Negative,
    Excess,
}
#[derive(Default)]
pub struct State {
    pub bytes: RefCell<Vec<u8>>,
    pub position: Cell<usize>,
    pub maximum: Cell<usize>,
    pub fail: Cell<bool>,
    pub reply_count: Cell<ReplyCount>,
    pub on_io: RefCell<Option<Rc<dyn Fn()>>>,
    pub calls: Cell<usize>,
    pub limit: Cell<usize>,
}
pub struct Stream(pub Rc<State>);
impl Class for Stream {
    type Interfaces = (IBStream,);
}
impl Stream {
    fn before(&self) {
        let calls = self.0.calls.get() + 1;
        self.0.calls.set(calls);
        assert!(
            calls <= self.0.limit.get(),
            "bounded foreign stream progress"
        );
        let callback = self.0.on_io.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
    }
    unsafe fn count(&self, output: *mut i32, requested: i32, actual: usize) {
        if !output.is_null() {
            unsafe {
                *output = match self.0.reply_count.get() {
                    ReplyCount::Actual => actual as i32,
                    ReplyCount::Negative => -1,
                    ReplyCount::Excess => requested + 1,
                };
            }
        }
    }
}
impl IBStreamTrait for Stream {
    unsafe fn read(&self, buffer: *mut c_void, requested: i32, actual: *mut i32) -> tresult {
        assert!(requested > 0 && !buffer.is_null());
        self.before();
        if self.0.fail.get() {
            unsafe { self.count(actual, requested, 0) };
            return kInternalError;
        }
        let bytes = self.0.bytes.borrow();
        let position = self.0.position.get();
        let n = (requested as usize)
            .min(self.0.maximum.get())
            .min(bytes.len().saturating_sub(position));
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr().add(position), buffer.cast(), n);
            self.count(actual, requested, n);
        }
        self.0.position.set(position + n);
        kResultOk
    }
    unsafe fn write(&self, buffer: *mut c_void, requested: i32, actual: *mut i32) -> tresult {
        assert!(requested > 0 && !buffer.is_null());
        self.before();
        if self.0.fail.get() {
            unsafe { self.count(actual, requested, 0) };
            return kInternalError;
        }
        let n = (requested as usize).min(self.0.maximum.get());
        let position = self.0.position.get();
        let mut bytes = self.0.bytes.borrow_mut();
        let length = bytes.len().max(position + n);
        bytes.resize(length, 0);
        bytes[position..position + n]
            .copy_from_slice(unsafe { std::slice::from_raw_parts(buffer.cast::<u8>(), n) });
        self.0.position.set(position + n);
        unsafe { self.count(actual, requested, n) };
        kResultOk
    }
    unsafe fn seek(&self, _: i64, _: i32, _: *mut i64) -> tresult {
        panic!("codec must not seek foreign streams")
    }
    unsafe fn tell(&self, position: *mut i64) -> tresult {
        assert!(!position.is_null());
        unsafe {
            *position = self.0.position.get() as i64;
        }
        kResultOk
    }
}
pub fn stream(bytes: Vec<u8>, maximum: usize) -> (Rc<State>, ComPtr<IBStream>) {
    let state = Rc::new(State {
        bytes: RefCell::new(bytes),
        maximum: Cell::new(maximum),
        limit: Cell::new(200000),
        ..State::default()
    });
    let stream = ComWrapper::new(Stream(state.clone()))
        .to_com_ptr::<IBStream>()
        .unwrap();
    (state, stream)
}
