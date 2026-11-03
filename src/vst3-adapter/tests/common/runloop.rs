use super::{Handle, Module};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use vst3::{
    Class, ComPtr, ComRef, ComWrapper,
    Steinberg::{Linux::*, *},
};
#[derive(Default)]
pub struct LoopState {
    pub registered: Cell<u32>,
    pub unregistered: Cell<u32>,
    pub handlers: RefCell<Vec<Rc<ComPtr<ITimerHandler>>>>,
    pub dropped: Cell<u32>,
    pub reject_register: Cell<bool>,
    pub reject_unregister: Cell<bool>,
    pub register_callback: RefCell<Option<Box<dyn FnOnce()>>>,
}
pub struct RunLoop(pub Rc<LoopState>);
impl Drop for RunLoop {
    fn drop(&mut self) {
        self.0.dropped.set(self.0.dropped.get() + 1);
    }
}
impl Class for RunLoop {
    type Interfaces = (IRunLoop,);
}
impl IRunLoopTrait for RunLoop {
    unsafe fn registerEventHandler(
        &self,
        _handler: *mut IEventHandler,
        _fd: FileDescriptor,
    ) -> tresult {
        kNotImplemented
    }
    unsafe fn unregisterEventHandler(&self, _handler: *mut IEventHandler) -> tresult {
        kNotImplemented
    }
    unsafe fn registerTimer(
        &self,
        handler: *mut ITimerHandler,
        milliseconds: TimerInterval,
    ) -> tresult {
        assert_eq!(milliseconds, 10);
        let handler = Rc::new(unsafe { ComRef::from_raw(handler) }.unwrap().to_com_ptr());
        self.0.registered.set(self.0.registered.get() + 1);
        let callback = self.0.register_callback.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
        if self.0.reject_register.get() {
            return kResultFalse;
        }
        // A conforming host is allowed to call us immediately from registration.
        unsafe {
            handler.onTimer();
        }
        self.0.handlers.borrow_mut().push(handler);
        kResultOk
    }
    unsafe fn unregisterTimer(&self, handler: *mut ITimerHandler) -> tresult {
        if self.0.reject_unregister.get() {
            return kResultFalse;
        }
        let old = {
            let mut handlers = self.0.handlers.borrow_mut();
            let index = handlers
                .iter()
                .position(|h| h.as_ptr() == handler)
                .expect("registered identity");
            handlers.remove(index)
        };
        self.0.unregistered.set(self.0.unregistered.get() + 1);
        // Reentrant stale timer invocation must do nothing during termination.
        unsafe {
            old.onTimer();
        }
        drop(old);
        kResultOk
    }
}
pub fn host<'m>(
    module: &'m Module<'_>,
) -> (Rc<LoopState>, Handle<'m, IPluginFactory>, ComPtr<FUnknown>) {
    let state = Rc::new(LoopState::default());
    let host = ComWrapper::new(RunLoop(state.clone()))
        .to_com_ptr::<FUnknown>()
        .unwrap();
    let factory = module.factory();
    unsafe {
        assert_eq!(
            factory
                .cast::<IPluginFactory3>()
                .unwrap()
                .setHostContext(host.as_ptr()),
            kResultOk
        );
    }
    (state, factory, host)
}

impl LoopState {
    pub fn tick(&self) {
        let handlers = self.handlers.borrow().clone();
        for timer in handlers {
            unsafe {
                timer.onTimer();
            }
        }
    }
}
