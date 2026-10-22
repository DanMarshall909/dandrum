use dandrum_trigger_host::parameters::{
    AutomationEvent, ParameterController, ParameterError, ParameterReply,
};
use dandrum_trigger_host::protocol::{
    Admission, AdmittedRequest, EditorCommand, MAX_SAFE_GENERATION, ProtocolError,
};
use dandrum_trigger_host::runtime::{ControlEvent, TimedEvent};
use dandrum_trigger_host::{
    ADVANCED_DRUM_KIT_DOCUMENT, ADVANCED_DRUM_KIT_SAMPLE, TriggerHostConfiguration,
};
use serde_json::json;
use std::{num::NonZeroU64, path::PathBuf};

fn package() -> (tempfile::TempDir, PathBuf) {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("Trigger.resources");
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
    (temporary, root)
}

fn configuration() -> TriggerHostConfiguration {
    TriggerHostConfiguration {
        sample_rate_hz: dandrum_trigger_host::SampleRate::new(48_000.0).unwrap(),
        max_block_frames: 16,
    }
}

fn request(
    controller: &mut ParameterController,
    attachment: &str,
    request: u64,
    method: &str,
    args: serde_json::Value,
) -> AdmittedRequest {
    let message = json!({"version":1,"attachmentId":attachment,"requestId":request.to_string(),"method":method,"args":args}).to_string();
    match controller.admit(&message).unwrap() {
        Admission::New(request) => request,
        Admission::Duplicate { .. } => panic!("fixture must admit a new request"),
    }
}

fn automation(controller: &mut ParameterController) -> Vec<AutomationEvent> {
    let mut events = Vec::new();
    while let Some(event) = controller.peek_automation() {
        events.push(event);
        assert!(controller.acknowledge_automation());
    }
    assert!(!controller.acknowledge_automation());
    events
}

#[test]
fn pending_automation_reads_ordered_events_without_acknowledging_or_freeing_capacity() {
    let (_package, root) = package();
    let (mut controller, _audio) =
        ParameterController::open(root, configuration(), NonZeroU64::new(1).unwrap(), 4, 6)
            .unwrap();
    let empty_read = allocation_counter::measure(|| {
        assert_eq!(controller.pending_automation().next(), None);
    });
    assert_eq!(empty_read.count_total, 0);
    let editor = controller.attach_editor().unwrap();
    let level = controller
        .parameters()
        .iter()
        .position(|parameter| parameter.id == "drums.level")
        .unwrap();
    let pan = controller
        .parameters()
        .iter()
        .position(|parameter| parameter.id == "drums.pan")
        .unwrap();
    for (id, method, args) in [
        (1, "setParameter", json!(["drums.level", 0.375, 1])),
        (2, "beginGesture", json!(["drums.pan", 1])),
        (3, "setParameter", json!(["drums.pan", 0.25, 1])),
    ] {
        let command = request(&mut controller, &editor.attachment_id, id, method, args);
        controller.apply(&command).unwrap();
    }
    let expected = [
        AutomationEvent::Begin {
            parameter_index: level,
        },
        AutomationEvent::Value {
            parameter_index: level,
            value: 1.5,
        },
        AutomationEvent::End {
            parameter_index: level,
        },
        AutomationEvent::Begin {
            parameter_index: pan,
        },
        AutomationEvent::Value {
            parameter_index: pan,
            value: -0.5,
        },
    ];
    let allocations = allocation_counter::measure(|| {
        let mut pending = controller.pending_automation();
        for event in expected {
            assert_eq!(pending.next(), Some(event));
        }
        assert_eq!(pending.next(), None);
    });
    assert_eq!(allocations.count_total, 0);
    assert_eq!(
        controller.pending_automation().take(2).collect::<Vec<_>>(),
        expected[..2]
    );
    assert_eq!(
        controller.pending_automation().collect::<Vec<_>>(),
        expected
    );
    assert_eq!(controller.peek_automation(), Some(expected[0]));
    let blocked = request(
        &mut controller,
        &editor.attachment_id,
        4,
        "setParameter",
        json!(["drums.pan", 0.75, 1]),
    );
    assert_eq!(controller.apply(&blocked), Err(ParameterError::QueueFull));

    assert!(controller.acknowledge_automation());
    assert_eq!(controller.peek_automation(), Some(expected[1]));
    assert_eq!(
        controller.pending_automation().collect::<Vec<_>>(),
        expected[1..]
    );
    for (id, method, args) in [
        (5, "setParameter", json!(["drums.pan", 0.75, 1])),
        (6, "endGesture", json!(["drums.pan", 1])),
    ] {
        let command = request(&mut controller, &editor.attachment_id, id, method, args);
        controller.apply(&command).unwrap();
    }
    let mut remaining = expected[1..].to_vec();
    remaining.extend([
        AutomationEvent::Value {
            parameter_index: pan,
            value: 0.5,
        },
        AutomationEvent::End {
            parameter_index: pan,
        },
    ]);
    assert_eq!(
        controller.pending_automation().collect::<Vec<_>>(),
        remaining
    );
    let blocked = request(
        &mut controller,
        &editor.attachment_id,
        7,
        "setParameter",
        json!(["drums.level", 0.875, 1]),
    );
    assert_eq!(controller.apply(&blocked), Err(ParameterError::QueueFull));
    assert_eq!(automation(&mut controller), remaining);
    assert_eq!(controller.pending_automation().next(), None);
}

#[test]
fn normalized_public_command_emits_balanced_physical_automation_and_real_signed_audio() {
    let (_package, root) = package();
    let (mut controller, mut audio) =
        ParameterController::open(root, configuration(), NonZeroU64::new(1).unwrap(), 4, 8)
            .unwrap();
    let editor = controller.attach_editor().unwrap();
    assert_eq!(controller.parameters().len(), 23);
    let level = controller
        .parameters()
        .iter()
        .position(|parameter| parameter.id == "drums.level")
        .unwrap();
    let descriptor = &controller.parameters()[level];
    assert_eq!(
        (
            descriptor.physical_min,
            descriptor.physical_max,
            descriptor.physical_default
        ),
        (0.0, 4.0, 1.0)
    );
    let write = request(
        &mut controller,
        &editor.attachment_id,
        1,
        "setParameter",
        json!(["drums.level", 0.125, 1]),
    );
    assert_eq!(
        controller.apply(&write).unwrap(),
        ParameterReply::Accepted {
            generation: 1,
            sequence: 1
        }
    );
    let pending = controller.parameter_state();
    assert_eq!((pending.generation, pending.sequence), (1, 0));
    assert_eq!(
        pending.parameters[level].value, 0.25,
        "pending write is not falsely acknowledged by an old scalar snapshot"
    );
    assert_eq!(
        automation(&mut controller),
        [
            AutomationEvent::Begin {
                parameter_index: level
            },
            AutomationEvent::Value {
                parameter_index: level,
                value: 0.5
            },
            AutomationEvent::End {
                parameter_index: level
            },
        ]
    );
    let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
    audio
        .process_events(
            &mut left,
            &mut right,
            &[TimedEvent {
                frame_offset: 0,
                event: ControlEvent::NoteOn {
                    note: 36,
                    velocity: 100,
                },
            }],
        )
        .unwrap();
    assert_eq!((left[0], right[0]), (-0.25, -0.25));
    let applied = controller.parameter_state();
    assert_eq!(applied.sequence, 1);
    assert_eq!(applied.parameters[level].value, 0.125);
    assert_eq!(
        controller.session().snapshot(&write.attachment).unwrap(),
        editor.snapshot
    );
}

#[test]
fn host_feedback_preserves_sample_offset_public_identity_and_applied_command_sequence() {
    let (_package, root) = package();
    let (mut controller, mut audio) =
        ParameterController::open(root, configuration(), NonZeroU64::new(1).unwrap(), 4, 8)
            .unwrap();
    let editor = controller.attach_editor().unwrap();
    let level = controller
        .parameters()
        .iter()
        .position(|parameter| parameter.id == "drums.level")
        .unwrap();
    let root_index = controller.parameters()[level].root_index;
    let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
    audio
        .process_events(
            &mut left,
            &mut right,
            &[
                TimedEvent {
                    frame_offset: 5,
                    event: ControlEvent::Parameter {
                        index: root_index,
                        value: 0.25,
                    },
                },
                TimedEvent {
                    frame_offset: 5,
                    event: ControlEvent::NoteOn {
                        note: 36,
                        velocity: 100,
                    },
                },
            ],
        )
        .unwrap();
    assert_eq!(&left[..5], &[0.0; 5]);
    assert_eq!((left[5], right[5]), (-0.125, -0.125));
    let queued = request(
        &mut controller,
        &editor.attachment_id,
        1,
        "setParameter",
        json!(["drums.level", 0.5, 1]),
    );
    controller.apply(&queued).unwrap();
    let feedback = controller.take_feedback().expect("host feedback");
    assert_eq!(
        (
            feedback.generation,
            feedback.sequence,
            feedback.frame_offset
        ),
        (1, 0, 5)
    );
    assert_eq!(feedback.parameters[0].id, "drums.level");
    assert_eq!(
        feedback.parameters[0].value, 0.0625,
        "physical .25 is normalized .0625"
    );
    assert_eq!(
        controller.parameter_state().sequence,
        0,
        "host feedback cannot acknowledge an unapplied UI command"
    );
    audio.process(&mut left, &mut right).unwrap();
    let applied = controller.take_feedback().unwrap();
    assert_eq!((applied.sequence, applied.parameters[0].value), (1, 0.5));
    assert_eq!(controller.take_feedback(), None);
    assert_eq!(
        controller.session().snapshot(&queued.attachment).unwrap(),
        editor.snapshot
    );
}

#[test]
fn feedback_overflow_recovers_latest_applied_values_without_relabeling_old_feedback() {
    let (_package, root) = package();
    let (mut controller, mut audio) =
        ParameterController::open(root, configuration(), NonZeroU64::new(1).unwrap(), 2, 8)
            .unwrap();
    let editor = controller.attach_editor().unwrap();
    let level = controller
        .parameters()
        .iter()
        .position(|parameter| parameter.id == "drums.level")
        .unwrap();
    let root_index = controller.parameters()[level].root_index;
    let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
    audio
        .process_events(
            &mut left,
            &mut right,
            &[
                TimedEvent {
                    frame_offset: 0,
                    event: ControlEvent::Parameter {
                        index: root_index,
                        value: 0.25,
                    },
                },
                TimedEvent {
                    frame_offset: 7,
                    event: ControlEvent::Parameter {
                        index: root_index,
                        value: 0.75,
                    },
                },
            ],
        )
        .unwrap();
    let queued = request(
        &mut controller,
        &editor.attachment_id,
        1,
        "setParameter",
        json!(["drums.level", 0.125, 1]),
    );
    controller.apply(&queued).unwrap();
    assert_eq!(
        audio
            .process(&mut left, &mut right)
            .unwrap()
            .dropped_feedback,
        1
    );
    for value in [0.0625, 0.1875] {
        let old = controller.take_feedback().unwrap();
        assert_eq!((old.sequence, old.parameters[0].value), (0, value));
    }
    assert_eq!(controller.take_feedback(), None);
    let recovered = controller.parameter_state();
    assert_eq!(
        (recovered.sequence, recovered.parameters[level].value),
        (1, 0.125)
    );
}

#[test]
fn explicit_gesture_reserves_its_end_while_values_and_other_gestures_fill_the_queue() {
    let (_package, root) = package();
    let (mut controller, _audio) =
        ParameterController::open(root, configuration(), NonZeroU64::new(1).unwrap(), 4, 4)
            .unwrap();
    let a = controller.attach_editor().unwrap();
    let b = controller.attach_editor().unwrap();
    let level = controller
        .parameters()
        .iter()
        .position(|parameter| parameter.id == "drums.level")
        .unwrap();
    let pan = controller
        .parameters()
        .iter()
        .position(|parameter| parameter.id == "drums.pan")
        .unwrap();
    let begin_a = request(
        &mut controller,
        &a.attachment_id,
        1,
        "beginGesture",
        json!(["drums.level", 1]),
    );
    controller.apply(&begin_a).unwrap();
    assert_eq!(
        controller.peek_automation(),
        Some(AutomationEvent::Begin {
            parameter_index: level
        })
    );
    assert_eq!(
        controller.peek_automation(),
        Some(AutomationEvent::Begin {
            parameter_index: level
        }),
        "host rejection retains the same event for retry"
    );
    assert!(controller.acknowledge_automation());
    let begin_b = request(
        &mut controller,
        &b.attachment_id,
        1,
        "beginGesture",
        json!(["drums.pan", 1]),
    );
    controller.apply(&begin_b).unwrap();
    let value = request(
        &mut controller,
        &b.attachment_id,
        2,
        "setParameter",
        json!(["drums.pan", 0.25, 1]),
    );
    controller.apply(&value).unwrap();
    let blocked = request(
        &mut controller,
        &a.attachment_id,
        2,
        "setParameter",
        json!(["drums.level", 0.75, 1]),
    );
    assert_eq!(controller.apply(&blocked), Err(ParameterError::QueueFull));
    let blocked_begin = request(
        &mut controller,
        &a.attachment_id,
        3,
        "beginGesture",
        json!(["drums.variation", 1]),
    );
    assert_eq!(
        controller.apply(&blocked_begin),
        Err(ParameterError::QueueFull)
    );
    let end_a = request(
        &mut controller,
        &a.attachment_id,
        4,
        "endGesture",
        json!(["drums.level", 1]),
    );
    controller.apply(&end_a).unwrap();
    let end_b = request(
        &mut controller,
        &b.attachment_id,
        3,
        "endGesture",
        json!(["drums.pan", 1]),
    );
    controller.apply(&end_b).unwrap();
    assert_eq!(
        automation(&mut controller),
        [
            AutomationEvent::Begin {
                parameter_index: pan
            },
            AutomationEvent::Value {
                parameter_index: pan,
                value: -0.5
            },
            AutomationEvent::End {
                parameter_index: level
            },
            AutomationEvent::End {
                parameter_index: pan
            },
        ]
    );
}

#[test]
fn runtime_or_automation_backpressure_never_partially_publishes_a_parameter_command() {
    for (event_capacity, automation_capacity, drain_first) in [(1, 8, true), (4, 3, false)] {
        let (_package, root) = package();
        let (mut controller, mut audio) = ParameterController::open(
            root,
            configuration(),
            NonZeroU64::new(1).unwrap(),
            event_capacity,
            automation_capacity,
        )
        .unwrap();
        let editor = controller.attach_editor().unwrap();
        let first = request(
            &mut controller,
            &editor.attachment_id,
            1,
            "setParameter",
            json!(["drums.level", 0.125, 1]),
        );
        controller.apply(&first).unwrap();
        let accepted_events = if drain_first {
            automation(&mut controller)
        } else {
            Vec::new()
        };
        let blocked = request(
            &mut controller,
            &editor.attachment_id,
            2,
            "setParameter",
            json!(["drums.level", 0.75, 1]),
        );
        assert_eq!(controller.apply(&blocked), Err(ParameterError::QueueFull));
        if drain_first {
            assert_eq!(controller.peek_automation(), None);
        } else {
            assert_eq!(automation(&mut controller).len(), 3);
        }
        assert!(accepted_events.is_empty() || accepted_events.len() == 3);
        let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
        audio
            .process_events(
                &mut left,
                &mut right,
                &[TimedEvent {
                    frame_offset: 0,
                    event: ControlEvent::NoteOn {
                        note: 36,
                        velocity: 100,
                    },
                }],
            )
            .unwrap();
        assert_eq!((left[0], right[0]), (-0.25, -0.25));
        assert_eq!(controller.parameter_state().sequence, 1);
        let retry = request(
            &mut controller,
            &editor.attachment_id,
            3,
            "setParameter",
            json!(["drums.level", 0.25, 1]),
        );
        assert_eq!(
            controller.apply(&retry).unwrap(),
            ParameterReply::Accepted {
                generation: 1,
                sequence: 2
            }
        );
    }
}

#[test]
fn owners_cannot_duplicate_or_steal_gestures_and_detachment_balances_once() {
    let (_package, root) = package();
    let (mut controller, _audio) =
        ParameterController::open(root, configuration(), NonZeroU64::new(1).unwrap(), 4, 3)
            .unwrap();
    let a = controller.attach_editor().unwrap();
    let b = controller.attach_editor().unwrap();
    let level = controller
        .parameters()
        .iter()
        .position(|parameter| parameter.id == "drums.level")
        .unwrap();
    let begin = request(
        &mut controller,
        &a.attachment_id,
        1,
        "beginGesture",
        json!(["drums.level", 1]),
    );
    controller.apply(&begin).unwrap();
    assert_eq!(
        controller.apply(&begin),
        Err(ParameterError::GestureAlreadyActive)
    );
    for (id, method, args) in [
        (1, "beginGesture", json!(["drums.level", 1])),
        (2, "setParameter", json!(["drums.level", 0.5, 1])),
        (3, "endGesture", json!(["drums.level", 1])),
    ] {
        let foreign = request(&mut controller, &b.attachment_id, id, method, args);
        assert_eq!(
            controller.apply(&foreign),
            Err(ParameterError::GestureConflict)
        );
    }
    let value = request(
        &mut controller,
        &a.attachment_id,
        2,
        "setParameter",
        json!(["drums.level", 0.5, 1]),
    );
    controller.apply(&value).unwrap();
    controller.detach_editor(&a.attachment_id).unwrap();
    assert!(controller.detach_editor(&a.attachment_id).is_err());
    assert!(
        controller.apply(&begin).is_err(),
        "retired attachment cannot affect gesture ownership"
    );
    assert_eq!(
        automation(&mut controller),
        [
            AutomationEvent::Begin {
                parameter_index: level
            },
            AutomationEvent::Value {
                parameter_index: level,
                value: 2.0
            },
            AutomationEvent::End {
                parameter_index: level
            },
        ]
    );
    let no_gesture = request(
        &mut controller,
        &b.attachment_id,
        4,
        "endGesture",
        json!(["drums.level", 1]),
    );
    assert_eq!(
        controller.apply(&no_gesture),
        Err(ParameterError::GestureNotActive)
    );
}

#[test]
fn reset_explicitly_settles_cancelled_ui_commands_and_later_feedback_and_writes_remain_live() {
    let (_package, root) = package();
    let (mut controller, mut audio) =
        ParameterController::open(root, configuration(), NonZeroU64::new(1).unwrap(), 4, 8)
            .unwrap();
    let editor = controller.attach_editor().unwrap();
    let level = controller
        .parameters()
        .iter()
        .position(|parameter| parameter.id == "drums.level")
        .unwrap();
    let root_index = controller.parameters()[level].root_index;
    let first = request(
        &mut controller,
        &editor.attachment_id,
        1,
        "setParameter",
        json!(["drums.level", 0.125, 1]),
    );
    controller.apply(&first).unwrap();
    let pending = controller.parameter_state();
    assert_eq!(
        (
            pending.sequence,
            pending.reset_cancelled,
            pending.parameters[level].value
        ),
        (0, false, 0.25)
    );
    let report = audio.reset();
    assert!(!report.admission_closed);
    assert_eq!(
        (report.discarded_events, report.cancelled_parameter_commands),
        (1, 1)
    );
    let cancelled = controller.parameter_state();
    assert_eq!(
        (
            cancelled.sequence,
            cancelled.reset_cancelled,
            cancelled.parameters[level].value
        ),
        (1, true, 0.25)
    );
    let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
    audio
        .process_events(
            &mut left,
            &mut right,
            &[
                TimedEvent {
                    frame_offset: 5,
                    event: ControlEvent::Parameter {
                        index: root_index,
                        value: 0.25,
                    },
                },
                TimedEvent {
                    frame_offset: 5,
                    event: ControlEvent::NoteOn {
                        note: 36,
                        velocity: 100,
                    },
                },
            ],
        )
        .unwrap();
    assert_eq!((left[5], right[5]), (-0.125, -0.125));
    let feedback = controller.take_feedback().unwrap();
    assert_eq!(
        (
            feedback.sequence,
            feedback.reset_cancelled,
            feedback.parameters[0].value
        ),
        (1, true, 0.0625)
    );
    audio.reset();
    let second = request(
        &mut controller,
        &editor.attachment_id,
        2,
        "setParameter",
        json!(["drums.level", 0.125, 1]),
    );
    assert_eq!(
        controller.apply(&second).unwrap(),
        ParameterReply::Accepted {
            generation: 1,
            sequence: 2
        }
    );
    audio
        .process_events(
            &mut left,
            &mut right,
            &[TimedEvent {
                frame_offset: 0,
                event: ControlEvent::NoteOn {
                    note: 36,
                    velocity: 100,
                },
            }],
        )
        .unwrap();
    assert_eq!((left[0], right[0]), (-0.25, -0.25));
    let settled = controller.parameter_state();
    assert_eq!(
        (
            settled.sequence,
            settled.reset_cancelled,
            settled.parameters[level].value
        ),
        (2, false, 0.125)
    );
}

#[test]
fn normalized_ranges_include_level_quarter_and_bipolar_pan_endpoints() {
    let (_package, root) = package();
    let (mut controller, mut audio) =
        ParameterController::open(root, configuration(), NonZeroU64::new(1).unwrap(), 4, 8)
            .unwrap();
    let editor = controller.attach_editor().unwrap();
    for (request_id, id, normalized, physical) in [
        (1, "drums.level", 0.25, 1.0),
        (2, "drums.pan", 0.0, -1.0),
        (3, "drums.pan", 1.0, 1.0),
    ] {
        audio.reset();
        let index = controller
            .parameters()
            .iter()
            .position(|parameter| parameter.id == id)
            .unwrap();
        let write = request(
            &mut controller,
            &editor.attachment_id,
            request_id,
            "setParameter",
            json!([id, normalized, 1]),
        );
        controller.apply(&write).unwrap();
        assert_eq!(
            automation(&mut controller)[1],
            AutomationEvent::Value {
                parameter_index: index,
                value: physical
            }
        );
        let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
        let events = if id == "drums.level" {
            vec![TimedEvent {
                frame_offset: 0,
                event: ControlEvent::NoteOn {
                    note: 36,
                    velocity: 100,
                },
            }]
        } else {
            Vec::new()
        };
        audio
            .process_events(&mut left, &mut right, &events)
            .unwrap();
        if id == "drums.level" {
            assert_eq!((left[0], right[0]), (-0.5, -0.5));
        }
        assert_eq!(
            controller.parameter_state().parameters[index].value,
            normalized
        );
        assert_eq!(
            controller.take_feedback().unwrap().parameters[0].value,
            normalized
        );
    }
}

#[test]
fn accepted_nonbinary_ranges_admit_exact_normalized_endpoints() {
    let (_package, root) = package();
    let path = root.join(ADVANCED_DRUM_KIT_DOCUMENT);
    let yaml = std::fs::read_to_string(&path).unwrap().replacen(
        "default: 1, min: 0, max: 4",
        "default: 0, min: -0.1, max: 0.3",
        1,
    );
    std::fs::write(path, yaml).unwrap();
    let (mut controller, mut audio) =
        ParameterController::open(root, configuration(), NonZeroU64::new(1).unwrap(), 4, 8)
            .unwrap();
    let editor = controller.attach_editor().unwrap();
    let index = controller
        .parameters()
        .iter()
        .position(|p| p.id == "drums.level")
        .unwrap();
    for (request_id, normalized, physical) in [(1, 0.0, -0.1), (2, 1.0, 0.3)] {
        let write = request(
            &mut controller,
            &editor.attachment_id,
            request_id,
            "setParameter",
            json!(["drums.level", normalized, 1]),
        );
        controller.apply(&write).unwrap();
        assert_eq!(
            automation(&mut controller)[1],
            AutomationEvent::Value {
                parameter_index: index,
                value: physical,
            }
        );
        let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
        audio.process_events(&mut left, &mut right, &[]).unwrap();
        assert_eq!(
            controller.parameter_state().parameters[index].value,
            normalized
        );
        assert_eq!(
            controller.take_feedback().unwrap().parameters[0].value,
            normalized
        );
    }
}

#[test]
fn invalid_commands_and_stale_or_foreign_owners_preserve_both_instances() {
    let (_package, root) = package();
    let (mut a, mut audio_a) =
        ParameterController::open(&root, configuration(), NonZeroU64::new(1).unwrap(), 4, 16)
            .unwrap();
    let (mut b, mut audio_b) =
        ParameterController::open(&root, configuration(), NonZeroU64::new(1).unwrap(), 4, 16)
            .unwrap();
    let editor_a = a.attach_editor().unwrap();
    let editor_b = b.attach_editor().unwrap();
    let before_a = a.parameter_state();
    let before_b = b.parameter_state();
    assert_eq!(
        a.admit("{").err(),
        Some(ParameterError::Protocol(ProtocolError::Malformed))
    );
    let read = request(
        &mut a,
        &editor_a.attachment_id,
        1,
        "getParameterState",
        json!([]),
    );
    assert_eq!(
        a.apply(&read).unwrap(),
        ParameterReply::State(before_a.clone())
    );
    let unsupported = request(
        &mut a,
        &editor_a.attachment_id,
        2,
        "noteHeartbeat",
        json!([]),
    );
    assert_eq!(
        a.apply(&unsupported),
        Err(ParameterError::UnsupportedCommand)
    );
    let stale = request(
        &mut a,
        &editor_a.attachment_id,
        3,
        "endGesture",
        json!(["drums.level", 0]),
    );
    assert_eq!(a.apply(&stale), Err(ParameterError::StaleGeneration));
    let unknown = request(
        &mut a,
        &editor_a.attachment_id,
        4,
        "setParameter",
        json!(["other.level", 0.25, 1]),
    );
    assert_eq!(a.apply(&unknown), Err(ParameterError::UnknownParameter));
    for (id, args, error) in [
        (
            5,
            json!(["drums.level", 0]),
            ParameterError::StaleGeneration,
        ),
        (
            6,
            json!(["other.level", 1]),
            ParameterError::UnknownParameter,
        ),
    ] {
        let begin = request(&mut a, &editor_a.attachment_id, id, "beginGesture", args);
        assert_eq!(a.apply(&begin), Err(error));
        assert_eq!(a.parameter_state(), before_a);
        assert!(a.peek_automation().is_none());
    }
    for (id, method) in [(7, "beginGesture"), (8, "endGesture")] {
        let command = request(
            &mut a,
            &editor_a.attachment_id,
            id,
            method,
            json!(["drums.level", 1]),
        );
        assert_eq!(
            a.apply(&command),
            Ok(ParameterReply::Accepted {
                generation: 1,
                sequence: 0,
            })
        );
    }
    let level = a
        .parameters()
        .iter()
        .position(|parameter| parameter.id == "drums.level")
        .unwrap();
    assert_eq!(
        automation(&mut a),
        [
            AutomationEvent::Begin {
                parameter_index: level
            },
            AutomationEvent::End {
                parameter_index: level
            }
        ]
    );
    let mut invalid = request(
        &mut a,
        &editor_a.attachment_id,
        9,
        "setParameter",
        json!(["drums.level", 0.25, 1]),
    );
    for value in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        invalid.command = EditorCommand::SetParameter {
            parameter_id: "drums.level".into(),
            value,
            generation: 1,
        };
        assert_eq!(
            a.apply(&invalid),
            Err(ParameterError::Protocol(ProtocolError::StaleRequest))
        );
    }
    let write_a = request(
        &mut a,
        &editor_a.attachment_id,
        10,
        "setParameter",
        json!(["drums.level", 0.125, 1]),
    );
    assert_eq!(
        b.apply(&write_a),
        Err(ParameterError::Protocol(ProtocolError::StaleAttachment))
    );
    assert_eq!(a.parameter_state(), before_a);
    assert_eq!(b.parameter_state(), before_b);
    assert!(a.peek_automation().is_none() && b.peek_automation().is_none());
    a.apply(&write_a).unwrap();
    let write_b = request(
        &mut b,
        &editor_b.attachment_id,
        1,
        "setParameter",
        json!(["drums.level", 0.375, 1]),
    );
    b.apply(&write_b).unwrap();
    for (audio, expected) in [(&mut audio_a, -0.25), (&mut audio_b, -0.75)] {
        let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
        audio
            .process_events(
                &mut left,
                &mut right,
                &[TimedEvent {
                    frame_offset: 0,
                    event: ControlEvent::NoteOn {
                        note: 36,
                        velocity: 100,
                    },
                }],
            )
            .unwrap();
        assert_eq!((left[0], right[0]), (expected, expected));
    }
    a.detach_editor(&editor_a.attachment_id).unwrap();
    audio_a.reset();
    assert_eq!(
        b.session().snapshot(&write_b.attachment).unwrap(),
        editor_b.snapshot
    );
    let b_level = b
        .parameters()
        .iter()
        .position(|parameter| parameter.id == "drums.level")
        .unwrap();
    assert_eq!(b.parameter_state().parameters[b_level].value, 0.375);
}

#[test]
fn constructor_rejects_invalid_capacity_resources_and_nonbijective_or_unbounded_surfaces() {
    let (_package, root) = package();
    for capacity in [0, 2, 1_025] {
        assert!(matches!(
            ParameterController::open(
                &root,
                configuration(),
                NonZeroU64::new(1).unwrap(),
                4,
                capacity
            ),
            Err(ParameterError::InvalidCapacity)
        ));
    }
    assert!(matches!(
        ParameterController::open(
            root.join("missing"),
            configuration(),
            NonZeroU64::new(1).unwrap(),
            4,
            8
        ),
        Err(ParameterError::Host(_))
    ));
    assert!(matches!(
        ParameterController::open(&root, configuration(), NonZeroU64::new(1).unwrap(), 0, 8),
        Err(ParameterError::Runtime(_))
    ));
    let document = root.join(ADVANCED_DRUM_KIT_DOCUMENT);
    let yaml = std::fs::read_to_string(&document).unwrap();
    let invalids = [
        yaml.replacen("default: 1, min: 0, max: 4", "default: 1, max: 4", 1),
        yaml.replacen("default: 1, min: 0, max: 4", "default: 1, min: 0", 1),
        yaml.replacen(
            "default: 1, min: 0, max: 4",
            "default: 1, min: 1, max: 1",
            1,
        ),
        yaml.replacen(
            "default: 1, min: 0, max: 4",
            "default: 1, min: 0, max: 1.0e100",
            1,
        ),
        yaml.replace(
            "name: drums.pan, maps_to: pan",
            "name: drums.pan, maps_to: level",
        ),
        yaml.replace("    - { name: drums.pan, maps_to: pan }\n", ""),
    ];
    for invalid in invalids {
        std::fs::write(&document, invalid).unwrap();
        assert!(matches!(
            ParameterController::open(&root, configuration(), NonZeroU64::new(1).unwrap(), 4, 8),
            Err(ParameterError::InvalidSurface)
        ));
    }
}

#[test]
fn a_valid_session_with_two_stereo_outputs_is_rejected_by_the_trigger_runtime() {
    let (_package, root) = package();
    let document = root.join(ADVANCED_DRUM_KIT_DOCUMENT);
    let yaml = std::fs::read_to_string(&document).unwrap();
    let master = "  - { name: master, direction: output, signal: audio, channels: 2, maps_from: player.audio }";
    assert!(yaml.contains(master));
    let yaml = yaml.replace(master, &format!("{master}\n  - {{ name: auxiliary, direction: output, signal: audio, channels: 2, maps_from: player.audio }}"));
    std::fs::write(document, yaml).unwrap();
    let host = dandrum_trigger_host::TriggerHost::open(&root, configuration())
        .expect("the broader Session model accepts two named planar stereo outputs");
    assert_eq!(host.session().owner_snapshot().revision, 0);
    assert!(matches!(
        ParameterController::open(&root, configuration(), NonZeroU64::MIN, 4, 8),
        Err(ParameterError::Runtime(
            dandrum_trigger_host::runtime::RuntimeError::UnsupportedLayout
        ))
    ));
}

#[test]
fn public_generation_never_overflows_or_loses_javascript_integer_precision() {
    use dandrum_trigger_host::parameters::document_generation;
    assert_eq!(document_generation(0), Ok(1));
    assert_eq!(
        document_generation(MAX_SAFE_GENERATION - 1),
        Ok(MAX_SAFE_GENERATION)
    );
    assert_eq!(
        document_generation(MAX_SAFE_GENERATION),
        Err(ParameterError::IdentityExhausted)
    );
    assert_eq!(
        document_generation(u64::MAX),
        Err(ParameterError::IdentityExhausted)
    );
}

#[test]
fn alias_order_is_frozen_at_open_and_projects_to_the_correct_root_control() {
    let (_package, root) = package();
    let path = root.join(ADVANCED_DRUM_KIT_DOCUMENT);
    let yaml = std::fs::read_to_string(&path)
        .unwrap()
        .replace("name: drums.level, maps_to: level", "ALIAS_PLACEHOLDER")
        .replace(
            "name: drums.pan, maps_to: pan",
            "name: drums.level, maps_to: level",
        )
        .replace("ALIAS_PLACEHOLDER", "name: drums.pan, maps_to: pan");
    std::fs::write(path, yaml).unwrap();
    let (mut controller, mut audio) =
        ParameterController::open(root, configuration(), NonZeroU64::new(1).unwrap(), 4, 8)
            .unwrap();
    let editor = controller.attach_editor().unwrap();
    let level = controller
        .parameters()
        .iter()
        .position(|parameter| parameter.id == "drums.level")
        .unwrap();
    assert_eq!((level, controller.parameters()[level].root_index), (3, 2));
    let write = request(
        &mut controller,
        &editor.attachment_id,
        1,
        "setParameter",
        json!(["drums.level", 0.125, 1]),
    );
    controller.apply(&write).unwrap();
    assert_eq!(
        automation(&mut controller)[1],
        AutomationEvent::Value {
            parameter_index: 3,
            value: 0.5
        }
    );
    let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
    audio
        .process_events(
            &mut left,
            &mut right,
            &[TimedEvent {
                frame_offset: 0,
                event: ControlEvent::NoteOn {
                    note: 36,
                    velocity: 100,
                },
            }],
        )
        .unwrap();
    assert_eq!((left[0], right[0]), (-0.25, -0.25));
    assert_eq!(
        controller.take_feedback().unwrap().parameters[0].id,
        "drums.level"
    );
}

#[test]
fn sequenced_public_writes_and_reset_cancellation_allocate_or_free_nothing_on_audio() {
    let (_package, root) = package();
    let (mut controller, mut audio) =
        ParameterController::open(root, configuration(), NonZeroU64::new(1).unwrap(), 4, 8)
            .unwrap();
    let editor = controller.attach_editor().unwrap();
    let write = request(
        &mut controller,
        &editor.attachment_id,
        1,
        "setParameter",
        json!(["drums.level", 0.125, 1]),
    );
    controller.apply(&write).unwrap();
    let worker = std::thread::spawn(move || {
        let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
        let measured = allocation_counter::measure(|| {
            audio
                .process_events(
                    &mut left,
                    &mut right,
                    &[TimedEvent {
                        frame_offset: 0,
                        event: ControlEvent::NoteOn {
                            note: 36,
                            velocity: 100,
                        },
                    }],
                )
                .unwrap();
            assert_eq!((left[0], right[0]), (-0.25, -0.25));
        });
        assert_eq!(
            (
                measured.count_total,
                measured.count_current,
                measured.bytes_current
            ),
            (0, 0, 0)
        );
        audio
    });
    let mut audio = worker.join().unwrap();
    let queued = request(
        &mut controller,
        &editor.attachment_id,
        2,
        "setParameter",
        json!(["drums.level", 0.25, 1]),
    );
    controller.apply(&queued).unwrap();
    let mut report = None;
    let measured = allocation_counter::measure(|| report = Some(audio.reset()));
    assert_eq!(
        (
            measured.count_total,
            measured.count_current,
            measured.bytes_current
        ),
        (0, 0, 0)
    );
    assert_eq!(report.unwrap().cancelled_parameter_commands, 1);
    let state = controller.parameter_state();
    assert_eq!((state.sequence, state.reset_cancelled), (2, true));
}

#[test]
fn public_index_reads_exact_applied_physical_atoms_without_allocating() {
    let (_package, root) = package();
    let path = root.join(ADVANCED_DRUM_KIT_DOCUMENT);
    let yaml = std::fs::read_to_string(&path)
        .unwrap()
        .replace("name: drums.level, maps_to: level", "ALIAS_PLACEHOLDER")
        .replace(
            "name: drums.pan, maps_to: pan",
            "name: drums.level, maps_to: level",
        )
        .replace("ALIAS_PLACEHOLDER", "name: drums.pan, maps_to: pan");
    std::fs::write(path, yaml).unwrap();
    let (mut controller, mut audio) =
        ParameterController::open(root, configuration(), NonZeroU64::new(1).unwrap(), 4, 8)
            .unwrap();
    let level = controller
        .parameters()
        .iter()
        .position(|p| p.id == "drums.level")
        .unwrap();
    assert_eq!((level, controller.parameters()[level].root_index), (3, 2));
    assert_eq!(controller.physical_value(level), Some(1.0));
    let editor = controller.attach_editor().unwrap();
    let write = request(
        &mut controller,
        &editor.attachment_id,
        1,
        "setParameter",
        json!(["drums.level", 0.125, 1]),
    );
    controller.apply(&write).unwrap();
    assert_eq!(controller.physical_value(level), Some(1.0));
    audio.flush_parameters(&[]).unwrap();
    assert_eq!(controller.physical_value(level), Some(0.5));
    let exact = 0.312_345_678_901_234_5;
    audio
        .flush_parameters(&[ControlEvent::Parameter {
            index: 2,
            value: exact,
        }])
        .unwrap();
    let measured = allocation_counter::measure(|| {
        assert_eq!(controller.physical_value(level), Some(exact));
        assert_eq!(controller.physical_value(23), None);
        assert_eq!(controller.physical_value(usize::MAX), None);
    });
    assert_eq!(
        (
            measured.count_total,
            measured.count_current,
            measured.bytes_current
        ),
        (0, 0, 0)
    );
}
