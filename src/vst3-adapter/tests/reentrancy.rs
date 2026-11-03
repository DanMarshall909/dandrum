mod common;
use common::{audio::*, *};
use std::{cell::Cell, io::Write};
use vst3::{
    Class, ComPtr, ComWrapper,
    Steinberg::{Vst::*, *},
};

#[test]
fn stopped_flush_foreign_callback_cannot_destroy_its_borrowed_endpoint() {
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "stopped_flush_reentrant_termination_child",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let output = finish_child(child, 3, "callback");
    assert!(
        output.status.success(),
        "reentrant callback: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

struct ReentrantEvents {
    processor: ComPtr<IComponent>,
    calls: Cell<usize>,
}
impl Class for ReentrantEvents {
    type Interfaces = (IEventList,);
}
impl IEventListTrait for ReentrantEvents {
    unsafe fn getEventCount(&self) -> i32 {
        assert_eq!(
            self.calls.replace(self.calls.get() + 1),
            0,
            "one bounded event-list query"
        );
        println!("entering host event-list termination reentry");
        std::io::stdout().flush().unwrap();
        assert_eq!(
            unsafe { self.processor.terminate() },
            kResultFalse,
            "borrowed endpoint cannot be terminated"
        );
        0
    }
    unsafe fn getEvent(&self, _: i32, _: *mut Event) -> tresult {
        panic!("empty list must not be read")
    }
    unsafe fn addEvent(&self, _: *mut Event) -> tresult {
        kNotImplemented
    }
}

#[test]
#[ignore = "only the owned deadline parent launches this exact child"]
fn stopped_flush_reentrant_termination_child() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let a = p.cast::<IAudioProcessor>().unwrap();
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    let events = ComWrapper::new(ReentrantEvents {
        processor: (*p).clone(),
        calls: Cell::new(0),
    })
    .to_com_ptr::<IEventList>()
    .unwrap();
    let mut block = Block::new(0);
    let mut data = block.data();
    data.inputEvents = events.as_ptr();
    assert_eq!(unsafe { a.process(&mut data) }, kResultOk);
    assert_eq!(
        unsafe { p.terminate() },
        kResultOk,
        "callback returned audio ownership"
    );
}
