use super::dispatch;
use dandrum_trigger_host::{
    ADVANCED_DRUM_KIT_DOCUMENT, ADVANCED_DRUM_KIT_SAMPLE, TriggerHostConfiguration,
    parameters::ParameterController, runtime::RuntimeAudio,
};
use serde_json::{Value, json};
use std::{num::NonZeroU64, path::PathBuf};

fn controller(
    events: usize,
    automation: usize,
) -> (tempfile::TempDir, ParameterController, RuntimeAudio, String) {
    let package = tempfile::tempdir().unwrap();
    let root = package.path();
    std::fs::create_dir_all(root.join("instrument/assets")).unwrap();
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (source, destination) in [
        (
            "examples/patches/advanced-drum-kit.yaml",
            ADVANCED_DRUM_KIT_DOCUMENT,
        ),
        (
            "examples/patches/assets/advanced-drums.wav",
            ADVANCED_DRUM_KIT_SAMPLE,
        ),
    ] {
        std::fs::copy(repository.join(source), root.join(destination)).unwrap();
    }
    let (mut controller, audio) = ParameterController::open(
        root,
        TriggerHostConfiguration {
            sample_rate_hz: 48000,
            max_block_frames: if events > 64 { (events * 2) as u32 } else { 16 },
        },
        NonZeroU64::new(19).unwrap(),
        events,
        automation,
    )
    .unwrap();
    let attachment = controller.attach_editor().unwrap().attachment_id;
    (package, controller, audio, attachment)
}
fn call(
    controller: &mut ParameterController,
    attachment: &str,
    id: u64,
    method: &str,
    args: Value,
) -> Value {
    let id = id.to_string();
    let message =
        json!({"version":1,"attachmentId":attachment,"requestId":id,"method":method,"args":args})
            .to_string();
    serde_json::from_str(&dispatch(controller, attachment, &id, &message).unwrap()).unwrap()
}
#[test]
fn current_dispatch_preserves_snapshot_notes_admission_and_capability_errors() {
    let (_package, mut controller, _audio, attachment) = controller(8, 8);
    let snapshot = call(
        &mut controller,
        &attachment,
        1,
        "getSamplerSession",
        json!([]),
    );
    assert_eq!(snapshot["status"], "accepted");
    assert_eq!(snapshot["instanceId"], "19");
    assert_eq!(snapshot["attachmentId"], attachment);
    assert_eq!(snapshot["requestId"], "1");
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            2,
            "noteOn",
            json!([36, 0.5, 1])
        )["result"],
        Value::Null
    );
    assert_eq!(
        call(&mut controller, &attachment, 3, "noteOff", json!([36, 1]))["status"],
        "accepted"
    );
    assert_eq!(
        call(&mut controller, &attachment, 4, "noteHeartbeat", json!([]))["error"]["code"],
        "capabilityUnavailable"
    );
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            5,
            "setParameter",
            json!(["drums.level", 2, 1])
        )["error"]["code"],
        "invalidArguments"
    );
    assert_eq!(controller.parameter_state().sequence, 0);
}

#[test]
fn real_parameter_dispatch_preserves_admission_settlement_and_ordered_gestures() {
    let (_package, mut controller, mut audio, attachment) = controller(8, 8);
    let initial = call(
        &mut controller,
        &attachment,
        1,
        "getParameterState",
        json!([]),
    );
    assert_eq!(initial["status"], "accepted");
    assert_eq!(initial["result"]["generation"], 1);
    assert_eq!(initial["result"]["sequence"], 0);
    assert_eq!(
        initial["result"]["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == "drums.level")
            .unwrap(),
        &json!({"id":"drums.level","name":"drums.level","value":0.25})
    );
    for (id, method, args, sequence) in [
        (2, "beginGesture", json!(["drums.level", 1]), 0),
        (3, "setParameter", json!(["drums.level", 0.125, 1]), 1),
        (4, "setParameter", json!(["drums.level", 0.375, 1]), 2),
        (5, "endGesture", json!(["drums.level", 1]), 2),
    ] {
        let reply = call(&mut controller, &attachment, id, method, args);
        assert_eq!(reply["status"], "accepted");
        assert_eq!(
            reply["result"],
            json!({"status":"accepted","generation":1,"sequence":sequence})
        );
    }
    // Admitted writes are not yet settled; the real runtime is the only scalar authority.
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            6,
            "getParameterState",
            json!([])
        )["result"]["sequence"],
        0
    );
    use dandrum_trigger_host::parameters::AutomationEvent::*;
    assert_eq!(
        controller.pending_automation().collect::<Vec<_>>(),
        [
            Begin { parameter_index: 2 },
            Value {
                parameter_index: 2,
                value: 0.5
            },
            Value {
                parameter_index: 2,
                value: 1.5
            },
            End { parameter_index: 2 }
        ]
    );
    audio.flush_parameters(&[]).unwrap();
    let settled = call(
        &mut controller,
        &attachment,
        7,
        "getParameterState",
        json!([]),
    );
    assert_eq!(settled["result"]["sequence"], 2);
    assert_eq!(settled["result"]["parameters"][2]["value"], 0.375);
}
#[test]
fn parameter_dispatch_reports_terminal_busy_duplicate_and_existing_admission_errors() {
    let (_package, mut controller, mut audio, attachment) = controller(4, 3);
    for (id, args) in [
        (1, json!(["drums.level", 0.125, 2])),
        (2, json!(["missing", 0.125, 1])),
    ] {
        assert_eq!(
            call(&mut controller, &attachment, id, "setParameter", args)["error"]["code"],
            "commandRejected"
        );
    }
    let write = || json!(["drums.level", 0.125, 1]);
    assert_eq!(
        call(&mut controller, &attachment, 3, "setParameter", write())["status"],
        "accepted"
    );
    assert_eq!(
        call(&mut controller, &attachment, 3, "setParameter", write())["error"]["code"],
        "duplicateRequest"
    );
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            3,
            "setParameter",
            json!(["drums.level", 0.25, 1])
        )["error"]["code"],
        "conflictingRequest"
    );
    assert_eq!(
        call(&mut controller, &attachment, 4, "setParameter", write())["error"]["code"],
        "busy"
    );
    audio.flush_parameters(&[]).unwrap();
    assert_eq!(
        controller.parameter_state().sequence,
        1,
        "duplicate never redispatches"
    );
    assert_eq!(
        controller.pending_automation().count(),
        3,
        "settlement does not release host delivery credit"
    );
    // Preserved input/admission errors take precedence over dispatch semantics.
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            5,
            "setParameter",
            json!(["drums.level", 2, 1])
        )["error"]["code"],
        "invalidArguments"
    );
}

#[test]
fn foreign_gesture_and_runtime_capacity_reject_without_partial_parameter_publication() {
    let (_package, mut owner, _audio, attachment) = controller(1, 8);
    let other = owner.attach_editor().unwrap().attachment_id;
    assert_eq!(
        call(
            &mut owner,
            &attachment,
            1,
            "beginGesture",
            json!(["drums.level", 1])
        )["status"],
        "accepted"
    );
    assert_eq!(
        call(
            &mut owner,
            &other,
            1,
            "setParameter",
            json!(["drums.level", 0.125, 1])
        )["error"]["code"],
        "commandRejected"
    );
    assert_eq!(
        call(
            &mut owner,
            &attachment,
            2,
            "setParameter",
            json!(["drums.level", 0.125, 1])
        )["status"],
        "accepted"
    );
    assert_eq!(
        call(
            &mut owner,
            &attachment,
            3,
            "setParameter",
            json!(["drums.level", 0.375, 1])
        )["error"]["code"],
        "busy"
    );
    assert_eq!(owner.pending_automation().count(), 2);
    assert_eq!(
        call(
            &mut owner,
            &attachment,
            4,
            "endGesture",
            json!(["drums.level", 1])
        )["status"],
        "accepted"
    );
    assert_eq!(
        call(
            &mut owner,
            &attachment,
            5,
            "endGesture",
            json!(["drums.level", 1])
        )["error"]["code"],
        "commandRejected"
    );
}

#[test]
fn output_mirror_acknowledges_only_host_accepted_prefix_and_retries_the_rejected_head() {
    let (_package, mut controller, mut audio, attachment) = controller(8, 8);
    let mut saved = Vec::new();
    controller.save_state(&mut saved).unwrap();
    call(
        &mut controller,
        &attachment,
        1,
        "setParameter",
        json!(["drums.level", 0.125, 1]),
    );
    let expected = controller.pending_automation().collect::<Vec<_>>();
    let (mut mirror, mut output) = crate::trigger::automation::pair();
    mirror.service(&mut controller);
    let mut attempts = 0;
    output.deliver(|_| {
        attempts += 1;
        false
    });
    assert_eq!(attempts, 1, "a rejected head ends this callback");
    assert_eq!(
        controller.pending_automation().collect::<Vec<_>>(),
        expected
    );
    let mut accepted = Vec::new();
    output.deliver(|event| {
        if accepted.len() == 1 {
            return false;
        }
        accepted.push(event);
        true
    });
    assert_eq!(accepted, &expected[..1]);
    assert_eq!(
        controller.pending_automation().count(),
        3,
        "host acceptance is not yet a collected credit"
    );
    audio.flush_parameters(&[]).unwrap();
    assert_eq!(
        controller.load_state(&mut saved.as_slice()),
        Err(dandrum_trigger_host::state::StateError::Busy),
        "settlement and uncollected credit cannot bypass restore Busy"
    );
    mirror.service(&mut controller);
    assert_eq!(
        controller.pending_automation().collect::<Vec<_>>(),
        expected[1..]
    );
    output.deliver(|event| {
        accepted.push(event);
        true
    });
    assert_eq!(accepted, expected);
    assert_eq!(controller.pending_automation().count(), 2);
    mirror.service(&mut controller);
    assert!(controller.pending_automation().next().is_none());
    assert!(!mirror.pending());
    controller.load_state(&mut saved.as_slice()).unwrap();
    controller.service_quiescent(&mut audio).unwrap();
    assert_eq!(controller.parameter_state().generation, 2);
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            2,
            "setParameter",
            json!(["drums.level", 0.375, 1])
        )["error"]["code"],
        "commandRejected"
    );
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            3,
            "setParameter",
            json!(["drums.level", 0.375, 2])
        )["status"],
        "accepted"
    );
    mirror.service(&mut controller);
    let mut restored = Vec::new();
    output.deliver(|event| {
        restored.push(event);
        true
    });
    mirror.service(&mut controller);
    use dandrum_trigger_host::parameters::AutomationEvent::*;
    assert_eq!(
        restored,
        [
            Begin { parameter_index: 2 },
            Value {
                parameter_index: 2,
                value: 1.5
            },
            End { parameter_index: 2 }
        ]
    );
    assert!(!mirror.pending());
}
#[test]
fn output_mirror_stages_more_than_capacity_and_detach_ends_without_overtaking() {
    let (_package, mut controller, mut audio, attachment) = controller(128, 256);
    call(
        &mut controller,
        &attachment,
        1,
        "beginGesture",
        json!(["drums.level", 1]),
    );
    for id in 2..102 {
        assert_eq!(
            call(
                &mut controller,
                &attachment,
                id,
                "setParameter",
                json!(["drums.level", (id % 8) as f64 / 8.0, 1])
            )["status"],
            "accepted"
        );
    }
    controller.detach_editor(&attachment).unwrap();
    let expected = controller.pending_automation().collect::<Vec<_>>();
    assert_eq!(expected.len(), 102);
    let (mut mirror, mut output) = crate::trigger::automation::pair();
    mirror.service(&mut controller);
    let mut accepted = Vec::new();
    output.deliver(|event| {
        accepted.push(event);
        true
    });
    assert_eq!(accepted, expected[..64]);
    assert_eq!(controller.pending_automation().count(), 102);
    mirror.service(&mut controller);
    assert_eq!(controller.pending_automation().count(), 38);
    output.deliver(|event| {
        accepted.push(event);
        true
    });
    mirror.service(&mut controller);
    assert_eq!(accepted, expected);
    assert!(!mirror.pending());
    audio.flush_parameters(&[]).unwrap();
}
#[test]
fn output_callback_attempt_budget_includes_concurrent_refill() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let (_package, mut controller, _audio, attachment) = controller(128, 256);
    for id in 1..41 {
        call(
            &mut controller,
            &attachment,
            id,
            "setParameter",
            json!(["drums.level", 0.125, 1]),
        );
    }
    let expected = controller.pending_automation().collect::<Vec<_>>();
    let (mut mirror, mut output) = crate::trigger::automation::pair();
    mirror.service(&mut controller);
    let ready = Arc::new(AtomicBool::new(false));
    let refilled = Arc::new(AtomicBool::new(false));
    let ready_audio = Arc::clone(&ready);
    let refilled_audio = Arc::clone(&refilled);
    let delivery = std::thread::spawn(move || {
        let mut accepted = Vec::new();
        output.deliver(|event| {
            if accepted.len() == 1 {
                ready_audio.store(true, Ordering::Release);
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
                while !refilled_audio.load(Ordering::Acquire) {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "test main must stage concurrent refill"
                    );
                    std::thread::yield_now();
                }
            }
            assert!(
                accepted.len() < 64,
                "callback must stop at64 even with refill"
            );
            accepted.push(event);
            true
        });
        (output, accepted)
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !ready.load(Ordering::Acquire) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    mirror.service(&mut controller);
    refilled.store(true, Ordering::Release);
    let (mut output, mut accepted) = delivery.join().unwrap();
    assert_eq!(accepted, expected[..64]);
    mirror.service(&mut controller);
    output.deliver(|event| {
        accepted.push(event);
        true
    });
    mirror.service(&mut controller);
    assert_eq!(accepted, expected);
    assert!(!mirror.pending());
}

#[test]
fn two_live_mirrors_keep_refused_heads_and_accepted_credits_independent() {
    use dandrum_trigger_host::parameters::AutomationEvent::*;
    let (_package_a, mut a, mut audio_a, attachment_a) = controller(8, 8);
    let (_package_b, mut b, mut audio_b, attachment_b) = controller(8, 8);
    let (mut main_a, mut output_a) = crate::trigger::automation::pair();
    let (mut main_b, mut output_b) = crate::trigger::automation::pair();
    call(
        &mut a,
        &attachment_a,
        1,
        "setParameter",
        json!(["drums.level", 0.125, 1]),
    );
    call(
        &mut b,
        &attachment_b,
        1,
        "setParameter",
        json!(["drums.level", 0.375, 1]),
    );
    main_a.service(&mut a);
    main_b.service(&mut b);
    let a_expected = [
        Begin { parameter_index: 2 },
        Value {
            parameter_index: 2,
            value: 0.5,
        },
        End { parameter_index: 2 },
    ];
    let b_expected = [
        Begin { parameter_index: 2 },
        Value {
            parameter_index: 2,
            value: 1.5,
        },
        End { parameter_index: 2 },
    ];
    assert_eq!(output_a.deliver(|_| false), 0);
    let mut accepted_b = Vec::new();
    assert_eq!(
        output_b.deliver(|event| {
            accepted_b.push(event);
            true
        }),
        3
    );
    main_b.service(&mut b);
    assert_eq!(accepted_b, b_expected);
    assert!(!main_b.pending());
    assert_eq!(a.pending_automation().collect::<Vec<_>>(), a_expected);
    assert_eq!(b.pending_automation().count(), 0);
    audio_a.flush_parameters(&[]).unwrap();
    audio_b.flush_parameters(&[]).unwrap();
    assert_eq!(a.physical_value(2), Some(0.5));
    assert_eq!(b.physical_value(2), Some(1.5));
    let mut accepted_a = Vec::new();
    output_a.deliver(|event| {
        accepted_a.push(event);
        true
    });
    main_a.service(&mut a);
    assert_eq!(accepted_a, a_expected);
    assert!(!main_a.pending());
    assert_eq!(
        output_b.deliver(|_| panic!("empty independent output must not call host")),
        0
    );
    assert_eq!(b.pending_automation().count(), 0);
    assert_eq!(b.physical_value(2), Some(1.5));
}
#[test]
fn empty_output_calls_no_host_and_real_clap_output_projects_global_physical_values() {
    use clack_plugin::{
        events::{
            event_types::{ParamGestureBeginEvent, ParamGestureEndEvent, ParamValueEvent},
            io::EventBuffer,
        },
        prelude::*,
    };
    let (_package, mut controller, _audio, attachment) = controller(8, 8);
    let (mut mirror, mut output) = crate::trigger::automation::pair();
    assert_eq!(
        output.deliver(|_| panic!("empty output cannot invoke host")),
        0
    );
    call(
        &mut controller,
        &attachment,
        1,
        "setParameter",
        json!(["drums.level", 0.125, 1]),
    );
    mirror.service(&mut controller);
    let mut buffer = EventBuffer::with_capacity(64);
    assert_eq!(
        crate::trigger::deliver_automation(&mut output, &mut buffer.as_output()),
        3
    );
    assert_eq!(buffer.len(), 3);
    assert_eq!(
        buffer[0]
            .as_event::<ParamGestureBeginEvent>()
            .unwrap()
            .param_id(),
        Some(ClapId::new(3))
    );
    let value = buffer[1].as_event::<ParamValueEvent>().unwrap();
    assert_eq!(value.param_id(), Some(ClapId::new(3)));
    assert_eq!(value.value(), 0.5);
    assert!(value.pckn().matches_all());
    assert_eq!(
        buffer[2]
            .as_event::<ParamGestureEndEvent>()
            .unwrap()
            .param_id(),
        Some(ClapId::new(3))
    );
    for event in buffer.iter() {
        assert_eq!(event.header().time(), 0);
    }
    mirror.service(&mut controller);
    assert!(!mirror.pending());
}

fn configured_core(events: usize) -> (tempfile::TempDir, std::rc::Rc<super::Core>) {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    let (package, controller, audio, _attachment) = controller(events, events);
    let (automation, output) = super::super::automation::pair();
    let core = Rc::new(super::Core {
        controller: RefCell::new(controller),
        parked: Cell::new(Some(audio)),
        automation: RefCell::new(automation),
        parked_output: Cell::new(Some(output)),
        resource_root: package.path().to_owned(),
        plugin_path: PathBuf::new(),
        gui: RefCell::new(super::EditorState::default()),
    });
    (package, core)
}

#[test]
fn view_independent_service_has_one_weak_source_and_final_cancellation() {
    use std::rc::Rc;
    let (_package, core) = configured_core(8);
    let context = gtk::glib::MainContext::default();
    let _guard = context.acquire().unwrap();
    assert!(
        core.gui.borrow().source.is_none(),
        "configuration alone must not register a GLib source"
    );
    core.ensure_service_source();
    let source = context
        .find_source_by_id(core.gui.borrow().source.as_ref().unwrap())
        .unwrap();
    let id = core.gui.borrow().source.as_ref().unwrap().as_raw();
    core.ensure_service_source();
    assert_eq!(core.gui.borrow().source.as_ref().unwrap().as_raw(), id);
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(20);
    while std::time::Instant::now() < deadline {
        context.iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    core.destroy_editor();
    assert!(
        !source.is_destroyed(),
        "disposable view destruction retains the instance source"
    );
    core.cancel_service_source();
    assert!(source.is_destroyed());
    core.cancel_service_source(); // A successful cancellation is never repeated.
    core.ensure_service_source();
    let final_source = context
        .find_source_by_id(core.gui.borrow().source.as_ref().unwrap())
        .unwrap();
    let weak = Rc::downgrade(&core);
    drop(core);
    assert!(
        weak.upgrade().is_none(),
        "source callback must not retain the Core"
    );
    assert!(
        final_source.is_destroyed(),
        "final owner cancels before releasing Core resources"
    );
}

#[test]
fn detached_feedback_service_is_bounded_and_defers_busy_owners_without_consuming() {
    use dandrum_trigger_host::runtime::ControlEvent;
    let (_package, core) = configured_core(128);
    let index = core.controller.borrow().parameters()[2].root_index;
    let mut audio = core.parked.take().unwrap();
    audio
        .flush_parameters(&[0.5, 1.0, 1.5].map(|value| ControlEvent::Parameter { index, value }))
        .unwrap();
    {
        let _busy = core.gui.borrow_mut();
        core.service_editor();
    }
    assert_eq!(
        core.controller
            .borrow_mut()
            .take_feedback()
            .unwrap()
            .parameters[0]
            .value,
        0.125,
        "busy view owner defers feedback consumption"
    );
    {
        let _busy = core.controller.borrow_mut();
        core.service_editor();
    }
    assert_eq!(
        core.controller
            .borrow_mut()
            .take_feedback()
            .unwrap()
            .parameters[0]
            .value,
        0.25,
        "busy controller defers feedback consumption"
    );
    core.service_editor();
    assert!(
        core.controller.borrow_mut().take_feedback().is_none(),
        "detached view does not leave feedback blocking the bounded producer"
    );
    let values = (0..128)
        .map(|i| ControlEvent::Parameter {
            index,
            value: f64::from(i) / 32.0,
        })
        .collect::<Vec<_>>();
    audio.flush_parameters(&values).unwrap();
    core.parked.set(Some(audio));
    core.service_editor();
    let next = core.controller.borrow_mut().take_feedback().unwrap();
    assert_eq!(next.sequence, 0);
    assert_eq!(
        next.parameters[0].value, 0.5,
        "one service consumes exactly the first64 of an available128 feedback events"
    );
    core.service_editor();
    assert!(core.controller.borrow_mut().take_feedback().is_none());
}
