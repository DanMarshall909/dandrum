#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod common;
use common::runloop::*;
use common::*;
use std::{cell::Cell, rc::Rc};
use vst3::{
    ComWrapper,
    Steinberg::{Linux::ITimerHandlerTrait, Vst::*, *},
};
#[test]
fn factory_context_services_processor_without_controller_or_editor_and_releases_registration() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let state = Rc::new(LoopState::default());
    let host = ComWrapper::new(RunLoop(state.clone()))
        .to_com_ptr::<FUnknown>()
        .unwrap();
    let factory = module.factory();
    let f3 = factory.cast::<IPluginFactory3>().unwrap();
    unsafe {
        assert_eq!(f3.setHostContext(host.as_ptr()), kResultOk);
    }
    let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
    }
    assert_eq!(
        state.registered.get(),
        1,
        "one processor-owned main-thread service"
    );
    let timer = state.handlers.borrow()[0].clone();
    unsafe {
        for _ in 0..3 {
            timer.onTimer();
        }
        assert_eq!(p.getBusCount(0, 1), 1);
        assert_eq!(p.terminate(), kResultOk);
    }
    assert_eq!(state.unregistered.get(), 1);
    assert!(state.handlers.borrow().is_empty());
    drop(p);
    drop(f3);
    drop(factory);
    drop(host);
    assert_eq!(
        state.dropped.get(),
        1,
        "processor and factory release the host, even while an old timer survives"
    );
    drop(module);
    unsafe {
        timer.onTimer();
    }
}

#[test]
fn failed_registration_publishes_no_partial_instance_and_can_retry() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let (state, factory, _host) = host(&module);
    let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
    let visible_during_register = Rc::new(Cell::new(-1));
    let observed = visible_during_register.clone();
    let reentrant = p.cast::<IComponent>().unwrap();
    *state.register_callback.borrow_mut() = Some(Box::new(move || {
        observed.set(unsafe { reentrant.getBusCount(0, 1) });
    }));
    state.reject_register.set(true);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultFalse);
        assert_eq!(p.getBusCount(0, 1), 0);
    }
    assert_eq!(
        visible_during_register.get(),
        0,
        "registration must not expose a partial sound owner"
    );
    assert!(state.handlers.borrow().is_empty());
    state.reject_register.set(false);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(p.getBusCount(0, 1), 1);
        assert_eq!(p.terminate(), kResultOk);
    }
    assert_eq!(state.registered.get(), 2);
    assert_eq!(state.unregistered.get(), 1);
}

#[test]
fn rejected_unregister_retains_a_disabled_token_until_explicit_cleanup_retry() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let (state, factory, _host) = host(&module);
    let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
    }
    let timer = state.handlers.borrow()[0].clone();
    state.reject_unregister.set(true);
    unsafe {
        assert_eq!(
            p.terminate(),
            kResultFalse,
            "cleanup refusal must be visible"
        );
        timer.onTimer();
        assert_eq!(p.getBusCount(0, 1), 0);
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultFalse);
    }
    assert_eq!(state.handlers.borrow().len(), 1);
    state.reject_unregister.set(false);
    unsafe {
        assert_eq!(p.terminate(), kResultOk);
        timer.onTimer();
    }
    assert_eq!(state.registered.get(), 1);
    assert_eq!(state.unregistered.get(), 1);
    assert!(state.handlers.borrow().is_empty());
}

#[test]
fn termination_during_registration_rolls_back_the_foreign_timer_once() {
    for (unimplemented, refuse_cleanup) in [(false, false), (false, true), (true, false)] {
        let bundle = Bundle::new();
        let module = bundle.load();
        let (state, factory, _host) = host(&module);
        state.unimplemented.set(unimplemented);
        state.reject_unregister.set(refuse_cleanup);
        let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
        let reentrant = p.cast::<IComponent>().unwrap();
        *state.register_callback.borrow_mut() = Some(Box::new(move || {
            assert_eq!(unsafe { reentrant.terminate() }, kResultOk);
        }));
        unsafe {
            assert_eq!(p.initialize(std::ptr::null_mut()), kResultFalse);
            assert_eq!(p.getBusCount(0, 1), 0);
            assert_eq!(state.handlers.borrow().len(), usize::from(refuse_cleanup));
            state.reject_unregister.set(false);
            assert_eq!(p.terminate(), kResultOk);
        }
        assert_eq!(state.registered.get(), u32::from(!unimplemented));
        assert_eq!(state.unregistered.get(), u32::from(!unimplemented));
        assert!(state.handlers.borrow().is_empty());
    }
}

#[test]
fn foreign_thread_cannot_clear_factory_context_or_remove_processor_service() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let (state, factory, _host) = host(&module);
    let f3 = factory.cast::<IPluginFactory3>().unwrap();
    let address = f3.as_ptr() as usize;
    std::thread::spawn(move || {
        // The main test retains the factory and performs no concurrent access.
        let factory = unsafe { vst3::ComRef::from_raw(address as *mut IPluginFactory3) }.unwrap();
        assert_eq!(
            unsafe { factory.setHostContext(std::ptr::null_mut()) },
            kResultFalse
        );
    })
    .join()
    .unwrap();
    let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
    }
    assert_eq!(state.registered.get(), 1);
    unsafe {
        assert_eq!(p.terminate(), kResultOk);
    }
    assert_eq!(state.unregistered.get(), 1);
}

#[test]
fn advertised_unimplemented_loop_preserves_headless_bus_and_signed_audio() {
    use common::audio::*;
    let bundle = Bundle::new();
    let module = bundle.load();
    let (clock, factory, _host) = host(&module);
    clock.unimplemented.set(true);
    let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
    assert_eq!(
        unsafe { p.initialize(std::ptr::null_mut()) },
        kResultOk,
        "SDK dummy loop means absent optional service, not a partial instrument"
    );
    assert_eq!(unsafe { p.getBusCount(0, 1) }, 1);
    assert!(clock.handlers.borrow().is_empty());
    let audio = p.cast::<IAudioProcessor>().unwrap();
    start(&p, &audio, 48000.0, 16);
    let notes = events(vec![note_on(5, 36, 1.0)]);
    let mut block = Block::new(16);
    let mut data = block.data();
    data.inputEvents = notes.as_ptr();
    assert_eq!(unsafe { audio.process(&mut data) }, kResultOk);
    assert_eq!((block.left[5], block.right[5]), (-0.5, -0.5));
    assert_eq!(unsafe { audio.setProcessing(0) }, kResultOk);
    let c = p.cast::<IConnectionPoint>().unwrap();
    let peer = common::owner::Peer::attach(&c, 71);
    let selection = peer.send(&c, 1, "chooseSamplerFiles", serde_json::json!([]));
    assert_eq!(
        selection["status"], "rejected",
        "unserviced acquisition is not advertised"
    );
    assert_eq!(unsafe { p.terminate() }, kResultOk);
    assert!(clock.handlers.borrow().is_empty());
}

#[test]
fn terminated_processor_can_restart_with_fresh_service_and_signed_default_sound() {
    use common::{audio::*, parameters::*};
    let bundle = Bundle::new();
    let module = bundle.load();
    let (clock, factory, _host) = host(&module);
    let p = module.class_from::<IComponent>(&factory, &PROCESSOR);
    let audio = p.cast::<IAudioProcessor>().unwrap();
    for cycle in 0..2 {
        assert_eq!(
            unsafe { p.initialize(std::ptr::null_mut()) },
            kResultOk,
            "same component restarts in cycle{cycle}"
        );
        assert_eq!(clock.handlers.borrow().len(), 1);
        assert_eq!(
            unsafe { audio.setProcessing(1) },
            kNotInitialized,
            "a new lifetime needs fresh processing setup"
        );
        start(&p, &audio, 48000.0, 16);
        let notes = events(vec![note_on(5, 36, 1.0)]);
        let mut block = Block::new(16);
        let mut data = block.data();
        data.inputEvents = notes.as_ptr();
        let controls = changes(vec![queue(0x1003, vec![(0, 0.125)])]);
        if cycle == 0 {
            data.inputParameterChanges = controls.as_ptr();
        }
        assert_eq!(unsafe { audio.process(&mut data) }, kResultOk);
        assert_eq!(
            (block.left[5], block.right[5]),
            if cycle == 0 {
                (-0.25, -0.25)
            } else {
                (-0.5, -0.5)
            }
        );
        assert_eq!(unsafe { audio.setProcessing(0) }, kResultOk);
        assert_eq!(unsafe { p.terminate() }, kResultOk);
        assert!(clock.handlers.borrow().is_empty());
        assert_eq!(unsafe { p.getBusCount(0, 1) }, 0);
    }
    assert_eq!(clock.registered.get(), 2);
    assert_eq!(clock.unregistered.get(), 2);
}
