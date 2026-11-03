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
    calls: std::rc::Rc<Cell<usize>>,
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

struct ReentrantQueue {
    processor: ComPtr<IComponent>,
    calls: std::rc::Rc<Cell<usize>>,
}
impl Class for ReentrantQueue {
    type Interfaces = (IParamValueQueue,);
}
impl IParamValueQueueTrait for ReentrantQueue {
    unsafe fn getParameterId(&self) -> u32 {
        0x1003
    }
    unsafe fn getPointCount(&self) -> i32 {
        1
    }
    unsafe fn getPoint(&self, index: i32, offset: *mut i32, value: *mut f64) -> tresult {
        assert_eq!(index, 0);
        assert_eq!(self.calls.replace(self.calls.get() + 1), 0);
        assert_eq!(
            unsafe { self.processor.terminate() },
            kResultFalse,
            "point getter cannot destroy borrowed endpoint"
        );
        unsafe {
            *offset = 0;
            *value = 0.25;
        }
        kResultOk
    }
    unsafe fn addPoint(&self, _: i32, _: f64, _: *mut i32) -> tresult {
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
    let event_calls = std::rc::Rc::new(Cell::new(0));
    let point_calls = std::rc::Rc::new(Cell::new(0));
    let queue = ComWrapper::new(ReentrantQueue {
        processor: (*p).clone(),
        calls: point_calls.clone(),
    })
    .to_com_ptr::<IParamValueQueue>()
    .unwrap();
    let changes = common::parameters::changes(vec![queue]);
    let events = ComWrapper::new(ReentrantEvents {
        processor: (*p).clone(),
        calls: event_calls.clone(),
    })
    .to_com_ptr::<IEventList>()
    .unwrap();
    let mut block = Block::new(0);
    let mut data = block.data();
    data.inputEvents = events.as_ptr();
    data.inputParameterChanges = changes.as_ptr();
    assert_eq!(unsafe { a.process(&mut data) }, kResultOk);
    assert_eq!(event_calls.get(), 1);
    assert_eq!(point_calls.get(), 1);
    assert_eq!(
        unsafe { p.terminate() },
        kResultOk,
        "callback returned audio ownership"
    );
}
