#![cfg(target_os = "linux")]
mod common;
use common::{audio::*, *};
use std::{cell::Cell, io::Write};
use vst3::{
    Class, ComPtr, ComWrapper,
    Steinberg::{Vst::*, *},
};

fn child_case(name: &str) {
    // Parent owns the complete disposable namespace even after a failed child.
    let scratch = tempfile::tempdir().unwrap();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            name,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("TMPDIR", scratch.path())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let output = finish_child(child, 3, "callback");
    assert!(
        output.status.success(),
        "{name}: {}: {}{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
fn stopped_flush_foreign_callback_cannot_destroy_its_borrowed_endpoint() {
    child_case("stopped_flush_reentrant_termination_child");
}
#[test]
fn numeric_output_extent_rejection_clears_only_valid_planes_and_preserves_time() {
    child_case("invalid_extent_child");
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
        let audio = self.processor.cast::<IAudioProcessor>().unwrap();
        assert_eq!(
            unsafe { audio.setProcessing(0) },
            kResultFalse,
            "nested reset cannot acquire the outer callback's endpoint"
        );
        let mut nested = Block::new(16);
        assert_eq!(unsafe { audio.process(&mut nested.data()) }, kResultFalse);
        assert_eq!(nested.left, [0.0; 16]);
        assert_eq!(nested.right, [0.0; 16]);
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
    start(&p, &a, 48_000.0, 16);
    let onset = common::audio::events(vec![note_on(0, 36, 1.0)]);
    let mut first = Block::new(16);
    let mut data = first.data();
    data.inputEvents = onset.as_ptr();
    assert_eq!(unsafe { a.process(&mut data) }, kResultOk);
    assert_eq!((first.left[0], first.right[0]), (-0.5, -0.5));
    let nested_calls = std::rc::Rc::new(Cell::new(0));
    let nested = ComWrapper::new(ReentrantEvents {
        processor: (*p).clone(),
        calls: nested_calls.clone(),
    })
    .to_com_ptr::<IEventList>()
    .unwrap();
    let mut data = first.data();
    data.inputEvents = nested.as_ptr();
    assert_eq!(unsafe { a.process(&mut data) }, kResultOk);
    assert_eq!(nested_calls.get(), 1);
    assert_eq!(
        (first.left[0], first.right[0]),
        (11869.0 / 32768.0, 11869.0 / 32768.0),
        "rejected nested process/reset leaves the outer sounding cursor intact"
    );
    assert_eq!(unsafe { a.setProcessing(0) }, kResultOk);
    assert_eq!(unsafe { a.setProcessing(1) }, kResultOk);
    assert_eq!(unsafe { a.process(&mut first.data()) }, kResultOk);
    assert_eq!(first.left, [0.0; 16]);
    assert_eq!(first.right, [0.0; 16]);
    assert_eq!(unsafe { a.setProcessing(0) }, kResultOk);
    assert_eq!(
        unsafe { p.terminate() },
        kResultOk,
        "callback returned audio ownership"
    );
}

#[test]
#[ignore = "only the owned callback deadline parent launches this exact child"]
fn invalid_extent_child() {
    // Deliberately fail an existing arithmetic guard; retain diagnostics, not a
    // crash dump. No unknown nonwrapping host pointer is dereferenced by design.
    assert_eq!(
        unsafe {
            libc::setrlimit(
                libc::RLIMIT_CORE,
                &libc::rlimit {
                    rlim_cur: 0,
                    rlim_max: 0,
                },
            )
        },
        0
    );
    assert_eq!(unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) }, 0);
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let a = p.cast::<IAudioProcessor>().unwrap();
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    start(&p, &a, 48000.0, 16);
    let notes = events(vec![note_on(0, 36, 1.0)]);
    let mut first = Block::new(16);
    let mut data = first.data();
    data.inputEvents = notes.as_ptr();
    assert_eq!(unsafe { a.process(&mut data) }, kResultOk);
    assert_eq!(first.left[0], -0.5);
    let mut invalid = Block::new(16);
    invalid.pointers[0] = (usize::MAX & !3usize) as *mut f32;
    println!("entering f32 overflow rejection with one accessible plane");
    std::io::stdout().flush().unwrap();
    assert_eq!(unsafe { a.process(&mut invalid.data()) }, kInvalidArgument);
    assert_eq!(invalid.left, [123.0; 16]);
    assert_eq!(invalid.right, [0.0; 16]);
    let mut right = [123.0f64; 16];
    let mut pointers = [(usize::MAX & !7usize) as *mut f64, right.as_mut_ptr()];
    let mut bus = AudioBusBuffers {
        numChannels: 2,
        silenceFlags: 0,
        __field0: AudioBusBuffers__type0 {
            channelBuffers64: pointers.as_mut_ptr(),
        },
    };
    for pointer in [
        pointers[0],
        std::ptr::null_mut(),
        right.as_mut_ptr().cast::<u8>().wrapping_add(1).cast(),
    ] {
        pointers[0] = pointer;
        right.fill(123.0);
        let mut data = first.data();
        data.symbolicSampleSize = 1;
        data.outputs = &mut bus;
        assert_eq!(unsafe { a.process(&mut data) }, kInvalidArgument);
        assert_eq!(right, [0.0; 16]);
    }
    bus.__field0.channelBuffers64 = std::ptr::null_mut();
    let mut data = first.data();
    data.symbolicSampleSize = 1;
    data.outputs = &mut bus;
    assert_eq!(unsafe { a.process(&mut data) }, kInvalidArgument);
    let mut next = Block::new(16);
    assert_eq!(unsafe { a.process(&mut next.data()) }, kResultOk);
    assert_eq!(
        (next.left[0], next.right[0]),
        (11869.0 / 32768.0, 11869.0 / 32768.0)
    );
    assert_eq!(unsafe { a.setProcessing(0) }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
}
