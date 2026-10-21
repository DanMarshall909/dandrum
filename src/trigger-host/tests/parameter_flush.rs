use dandrum_engine::graph::PortDirection;
use dandrum_engine::kernel::document::load_kernel_patch_str;
use dandrum_engine::patch::RenderSettings;
use dandrum_engine::preparation::{HostBuses, PreparationContext};
use dandrum_instrument_session::InstrumentSession;
use dandrum_trigger_host::TriggerHostConfiguration;
use dandrum_trigger_host::runtime::{
    ControlEvent, ParameterFeedback, PreparedRuntime, ProcessReport, RuntimeAudio, RuntimeControl,
    TimedEvent, runtime_handoff,
};

fn config() -> TriggerHostConfiguration {
    TriggerHostConfiguration {
        sample_rate_hz: 48_000,
        max_block_frames: 16,
    }
}
fn session(note: u8) -> InstrumentSession {
    let yaml = include_str!("../../../examples/patches/advanced-drum-kit.yaml").replacen(
        "key_range: [36, 36]",
        &format!("key_range: [{note}, {note}]"),
        1,
    );
    let patch = load_kernel_patch_str(&yaml).unwrap();
    let buses = patch
        .root()
        .ports()
        .iter()
        .fold(HostBuses::new(), |buses, port| match port.direction() {
            PortDirection::Input => buses.with_input(port.name(), 1),
            PortDirection::Output => buses.with_output(port.name(), 2),
        });
    InstrumentSession::new_for_planar_host(
        &yaml,
        PreparationContext::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/patches"),
            48_000,
        ),
        RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 16,
            duration_frames: 16,
        },
        buses,
    )
    .unwrap()
}
fn prepared(note: u8) -> PreparedRuntime {
    PreparedRuntime::new(session(note).prepared(), config()).unwrap()
}
fn runtime(capacity: usize) -> (RuntimeControl, RuntimeAudio) {
    runtime_handoff(prepared(36), capacity).unwrap()
}
fn parameter(value: f64) -> ControlEvent {
    ControlEvent::Parameter { index: 2, value }
}
fn note() -> ControlEvent {
    ControlEvent::NoteOn {
        note: 36,
        velocity: 100,
    }
}
fn render<const N: usize>(
    audio: &mut RuntimeAudio,
    events: &[TimedEvent],
) -> ([f32; N], [f32; N], ProcessReport) {
    let (mut left, mut right) = ([1.0; N], [1.0; N]);
    let mut report = ProcessReport::default();
    let audit = allocation_counter::measure(|| {
        report = audio.process_events(&mut left, &mut right, events).unwrap();
    });
    assert_eq!(audit, allocation_counter::AllocationInfo::default());
    (left, right, report)
}
fn feedback(control: &mut RuntimeControl) -> Vec<ParameterFeedback> {
    std::iter::from_fn(|| control.take_feedback()).collect()
}

#[test]
fn ordinary_process_drains_fifo_scalars_and_note_before_timed_host_value_without_replaying_the_note()
 {
    let (mut control, mut audio) = runtime(4);
    for event in [parameter(0.5), note(), parameter(1.5)] {
        control.try_send(event).unwrap();
    }
    let output = render::<4>(
        &mut audio,
        &[TimedEvent {
            frame_offset: 3,
            event: parameter(1.0),
        }],
    );
    assert_eq!(
        output.0,
        [-0.75, -537.0 / 32768.0, 2331.0 / 32768.0, 4137.0 / 32768.0]
    );
    assert_eq!(output.0, output.1);
    assert_eq!(output.2.parameter_updates, 3);
    assert_eq!(
        feedback(&mut control),
        vec![
            ParameterFeedback {
                index: 2,
                value: 0.5,
                frame_offset: 0
            },
            ParameterFeedback {
                index: 2,
                value: 1.5,
                frame_offset: 0
            },
            ParameterFeedback {
                index: 2,
                value: 1.0,
                frame_offset: 3
            }
        ]
    );
    assert_eq!(control.parameter_value(2), Some(1.0));
    assert_eq!(
        render::<2>(&mut audio, &[]).0,
        [-2136.0 / 32768.0, 1664.0 / 32768.0]
    );
}

#[test]
fn invalid_complete_parameter_batch_preserves_values_feedback_fifo_and_pending_runtime() {
    use dandrum_trigger_host::runtime::{ParameterFlushReport, RuntimeError};
    let (mut control, mut audio) = runtime(4);
    control.try_send(parameter(0.25)).unwrap();
    render::<1>(&mut audio, &[]);
    for event in [parameter(0.5), note(), parameter(1.5)] {
        control.try_send(event).unwrap();
    }
    assert!(control.offer(prepared(36)).is_ok());
    let mut invalid = vec![
        vec![parameter(1.0); 5],
        vec![note()],
        vec![ControlEvent::NoteOff { note: 36 }],
        vec![ControlEvent::Parameter {
            index: usize::MAX,
            value: 1.0,
        }],
    ];
    for value in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MAX,
        -0.1,
        4.1,
    ] {
        invalid.push(vec![parameter(1.0), parameter(value)]);
    }
    for batch in invalid {
        let mut result = None;
        let audit = allocation_counter::measure(|| {
            result = Some(audio.flush_parameters(&batch));
        });
        assert_eq!(audit, allocation_counter::AllocationInfo::default());
        assert_eq!(result.unwrap(), Err(RuntimeError::InvalidEvent));
        assert_eq!(control.parameter_value(2), Some(0.25));
    }
    assert_eq!(
        feedback(&mut control),
        vec![ParameterFeedback {
            index: 2,
            value: 0.25,
            frame_offset: 0
        }]
    );
    let output = render::<4>(&mut audio, &[]);
    assert!(output.2.swapped);
    assert_eq!(output.2.parameter_updates, 2);
    assert_eq!(
        output.0,
        [-0.75, -537.0 / 32768.0, 2331.0 / 32768.0, 6205.5 / 32768.0]
    );
    assert_eq!(output.0, output.1);
    assert!(control.reclaim().is_some());
    assert_eq!(
        feedback(&mut control),
        vec![
            ParameterFeedback {
                index: 2,
                value: 0.5,
                frame_offset: 0
            },
            ParameterFeedback {
                index: 2,
                value: 1.5,
                frame_offset: 0
            }
        ]
    );
    let (_, mut empty) = runtime(4);
    assert_eq!(
        empty.flush_parameters(&[]),
        Ok(ParameterFlushReport::default())
    );
}

fn flush(
    audio: &mut RuntimeAudio,
    host: &[ControlEvent],
) -> dandrum_trigger_host::runtime::ParameterFlushReport {
    let mut report = None;
    let audit = allocation_counter::measure(|| {
        report = Some(audio.flush_parameters(host).unwrap());
    });
    assert_eq!(audit, allocation_counter::AllocationInfo::default());
    report.unwrap()
}

#[test]
fn stopped_scalar_prefix_then_host_values_cannot_overtake_a_note_and_later_note_renders_once() {
    let (mut control, mut audio) = runtime(4);
    for event in [parameter(0.5), note(), parameter(1.5)] {
        control.try_send(event).unwrap();
    }
    let first = flush(&mut audio, &[parameter(1.0)]);
    assert_eq!(
        (
            first.parameter_updates,
            first.dropped_feedback,
            first.deferred_at_note
        ),
        (2, 0, true)
    );
    assert_eq!(control.parameter_value(2), Some(1.0));
    let repeated = flush(&mut audio, &[]);
    assert_eq!(
        (repeated.parameter_updates, repeated.deferred_at_note),
        (0, true)
    );
    assert_eq!(
        feedback(&mut control),
        vec![
            ParameterFeedback {
                index: 2,
                value: 0.5,
                frame_offset: 0
            },
            ParameterFeedback {
                index: 2,
                value: 1.0,
                frame_offset: 0
            }
        ]
    );
    let output = render::<2>(&mut audio, &[]);
    assert_eq!(output.0, [-0.75, -537.0 / 32768.0]);
    assert_eq!(output.0, output.1);
    assert_eq!(output.2.parameter_updates, 1);
    assert_eq!(control.parameter_value(2), Some(1.5));
    assert_eq!(
        feedback(&mut control),
        vec![ParameterFeedback {
            index: 2,
            value: 1.5,
            frame_offset: 0
        }]
    );
    let next = render::<2>(&mut audio, &[]);
    assert_eq!(next.0, [2331.0 / 32768.0, 6205.5 / 32768.0]);
    assert_eq!(next.2.parameter_updates, 0);
    control
        .try_send(ControlEvent::NoteOff { note: 36 })
        .unwrap();
    assert!(flush(&mut audio, &[]).deferred_at_note);
    assert_eq!(
        render::<2>(&mut audio, &[]).0,
        [-3204.0 / 32768.0, 2496.0 / 32768.0],
        "one-shot note-off retains the natural tail"
    );
}

#[test]
fn host_duplicates_and_full_scalar_prefix_count_feedback_overflow_without_advancing_position_or_other_instance()
 {
    let (mut control, mut audio) = runtime(2);
    let (mut other, mut other_audio) = runtime(2);
    assert_eq!(
        render::<2>(
            &mut audio,
            &[TimedEvent {
                frame_offset: 0,
                event: note()
            }]
        )
        .0,
        [-0.5, -358.0 / 32768.0]
    );
    other.try_send(parameter(2.0)).unwrap();
    render::<1>(&mut other_audio, &[]);
    for event in [parameter(0.5), parameter(1.5)] {
        control.try_send(event).unwrap();
    }
    let result = flush(&mut audio, &[parameter(1.0), parameter(0.25)]);
    assert_eq!(
        (
            result.parameter_updates,
            result.dropped_feedback,
            result.deferred_at_note
        ),
        (4, 2, false)
    );
    assert_eq!(control.parameter_value(2), Some(0.25));
    assert_eq!(other.parameter_value(2), Some(2.0));
    assert_eq!(
        feedback(&mut control),
        vec![
            ParameterFeedback {
                index: 2,
                value: 0.5,
                frame_offset: 0
            },
            ParameterFeedback {
                index: 2,
                value: 1.5,
                frame_offset: 0
            }
        ]
    );
    assert_eq!(
        render::<2>(&mut audio, &[]).0,
        [388.5 / 32768.0, 1034.25 / 32768.0]
    );
    assert_eq!(
        feedback(&mut other),
        vec![ParameterFeedback {
            index: 2,
            value: 2.0,
            frame_offset: 0
        }]
    );
}

#[test]
fn flush_preserves_pending_replacement_and_unheard_editor_intent_until_a_real_callback() {
    let (mut control, mut audio) = runtime(4);
    control
        .editor_note_on(std::num::NonZeroU64::new(7).unwrap(), 36, 100)
        .unwrap();
    assert!(control.offer(prepared(60)).is_ok());
    let report = flush(&mut audio, &[parameter(0.5)]);
    assert_eq!(report.parameter_updates, 1);
    assert!(
        control.offer(prepared(36)).is_err(),
        "flush cannot restore activation capacity"
    );
    let output = render::<4>(&mut audio, &[]);
    assert!(output.2.swapped);
    assert_eq!(
        output.0, [0.0; 4],
        "held36 has no mapping after the real swap to60"
    );
    assert!(control.reclaim().is_some());
    // With no replacement, the unheard editor onset still begins at sample0.
    let (mut control, mut audio) = runtime(4);
    control
        .editor_note_on(std::num::NonZeroU64::new(8).unwrap(), 36, 100)
        .unwrap();
    flush(&mut audio, &[parameter(0.5)]);
    flush(&mut audio, &[]);
    let output = render::<2>(&mut audio, &[]);
    assert_eq!(output.0, [-0.25, -179.0 / 32768.0]);
}

#[test]
fn observed_scalar_prefix_is_bounded_during_real_producer_refill_and_feedback_saturation() {
    const E: usize = 32;
    const TOTAL: usize = 128;
    let (mut control, mut audio) = runtime(E);
    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
    let (done_tx, done_rx) = std::sync::mpsc::sync_channel(1);
    let producer = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        for index in 0..TOTAL {
            let event = parameter((index + 1) as f64 / 64.0);
            loop {
                match control.try_send(event) {
                    Ok(()) => break,
                    Err(dandrum_trigger_host::runtime::RuntimeError::QueueFull) => {
                        assert!(
                            std::time::Instant::now() < deadline,
                            "finite refill producer stalled"
                        );
                        std::thread::yield_now();
                    }
                    Err(error) => panic!("valid scalar refill rejected: {error:?}"),
                }
            }
            if index + 1 == E {
                ready_tx.send(()).unwrap();
            }
        }
        done_tx.send(control).unwrap();
    });
    ready_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    let first = flush(&mut audio, &[]);
    assert_eq!(first.parameter_updates, E);
    let (mut applied, mut dropped) = (first.parameter_updates, first.dropped_feedback);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut control = loop {
        if let Ok(control) = done_rx.try_recv() {
            break control;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "finite producer must finish"
        );
        let report = flush(&mut audio, &[]);
        assert!(report.parameter_updates <= E);
        assert!(!report.deferred_at_note);
        applied += report.parameter_updates;
        dropped += report.dropped_feedback;
        std::thread::yield_now();
    };
    producer.join().unwrap();
    let last = flush(&mut audio, &[]);
    assert!(last.parameter_updates <= E);
    applied += last.parameter_updates;
    dropped += last.dropped_feedback;
    assert_eq!((applied, dropped), (TOTAL, TOTAL - E));
    assert_eq!(control.parameter_value(2), Some(2.0));
    let actual = feedback(&mut control);
    assert_eq!(actual.len(), E);
    for (index, event) in actual.into_iter().enumerate() {
        assert_eq!(
            event,
            ParameterFeedback {
                index: 2,
                value: (index + 1) as f64 / 64.0,
                frame_offset: 0
            }
        );
    }
}

#[test]
fn controller_flush_preserves_real_epoch_history_save_snapshot_and_reset_settlement_without_audio_progress()
 {
    use dandrum_trigger_host::parameters::{
        ParameterController, ParameterReply, SessionJobProgress,
    };
    use dandrum_trigger_host::protocol::Admission;
    use serde_json::{Value, json};
    let package = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(package.path().join("instrument/assets")).unwrap();
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for path in [
        dandrum_trigger_host::ADVANCED_DRUM_KIT_DOCUMENT,
        dandrum_trigger_host::ADVANCED_DRUM_KIT_SAMPLE,
    ] {
        let source = if path.ends_with(".yaml") {
            "examples/patches/advanced-drum-kit.yaml"
        } else {
            "examples/patches/assets/advanced-drums.wav"
        };
        std::fs::copy(repository.join(source), package.path().join(path)).unwrap();
    }
    let (mut host, mut audio) = ParameterController::open(
        package.path(),
        config(),
        std::num::NonZeroU64::new(1).unwrap(),
        4,
        64,
    )
    .unwrap();
    let editor = host.attach_editor().unwrap();
    let mut transport = 0u64;
    let mut request = |host: &mut ParameterController, method: &str, args: Value| {
        transport += 1;
        let message=json!({"version":1,"attachmentId":editor.attachment_id,"requestId":transport.to_string(),"method":method,"args":args}).to_string();
        match host.admit(&message).unwrap() {
            Admission::New(request) => request,
            _ => panic!("new admitted request required"),
        }
    };
    let mut saved = Vec::new();
    host.save_state(&mut saved).unwrap();
    host.load_state(&mut saved.as_slice()).unwrap();
    host.service_quiescent(&mut audio).unwrap();
    let mapping = request(
        &mut host,
        "setInstrumentZoneKeyRange",
        json!([
            editor.instance_id,
            editor.attachment_id,
            "1",
            "1",
            "kit",
            "kick",
            60,
            60
        ]),
    );
    host.admit_session_command(&mapping).unwrap();
    let prepared = host.take_session_preparation().unwrap().prepare();
    host.submit_session_completion(prepared).unwrap();
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    host.service_quiescent(&mut audio).unwrap();
    let accepted = host.session().owner_snapshot();
    assert_eq!(
        (accepted.revision, accepted.undo_len),
        (2, 1),
        "restore epoch1 differs from current revision2"
    );
    let first = request(&mut host, "setParameter", json!(["drums.level", 0.125, 3]));
    assert_eq!(
        host.apply(&first).unwrap(),
        ParameterReply::Accepted {
            generation: 3,
            sequence: 1
        }
    );
    let report = flush(
        &mut audio,
        &[
            parameter(1.0),
            ControlEvent::Parameter {
                index: 3,
                value: 0.5,
            },
        ],
    );
    assert_eq!((report.parameter_updates, report.dropped_feedback), (3, 0));
    let state = host.parameter_state();
    assert_eq!(
        (
            state.generation,
            state.sequence,
            state.reset_cancelled,
            state.parameters[2].value
        ),
        (3, 1, false, 0.25)
    );
    let snapshot = request(&mut host, "getSamplerSession", json!([]));
    let projection: Value =
        serde_json::from_str(&host.sampler_session_reply(&snapshot).unwrap()).unwrap();
    assert_eq!(
        projection["result"]["document"]["parameters"][2]["normalisedDefaultValue"],
        0.25
    );
    let cancelled = request(&mut host, "setParameter", json!(["drums.level", 0.375, 3]));
    assert_eq!(
        host.apply(&cancelled).unwrap(),
        ParameterReply::Accepted {
            generation: 3,
            sequence: 2
        }
    );
    assert_eq!(audio.reset().cancelled_parameter_commands, 1);
    assert_eq!(flush(&mut audio, &[parameter(0.25)]).parameter_updates, 1);
    let state = host.parameter_state();
    assert_eq!(
        (
            state.sequence,
            state.reset_cancelled,
            state.parameters[2].value
        ),
        (2, true, 0.0625)
    );
    let next = request(&mut host, "setParameter", json!(["drums.level", 0.25, 3]));
    assert_eq!(
        host.apply(&next).unwrap(),
        ParameterReply::Accepted {
            generation: 3,
            sequence: 3
        }
    );
    let report = flush(&mut audio, &[]);
    assert_eq!((report.parameter_updates, report.dropped_feedback), (1, 1));
    let state = host.parameter_state();
    assert_eq!(
        (
            state.sequence,
            state.reset_cancelled,
            state.parameters[2].value
        ),
        (3, false, 0.25)
    );
    let feedback: Vec<_> = std::iter::from_fn(|| host.take_feedback()).collect();
    assert_eq!(
        feedback
            .iter()
            .map(|event| (
                event.sequence,
                event.reset_cancelled,
                event.frame_offset,
                event.parameters[0].value
            ))
            .collect::<Vec<_>>(),
        vec![
            (1, false, 0, 0.125),
            (1, false, 0, 0.25),
            (1, false, 0, 0.75),
            (2, true, 0, 0.0625)
        ]
    );
    assert_eq!(host.session().owner_snapshot(), accepted);
    let mut saved = Vec::new();
    host.save_state(&mut saved).unwrap();
    let saved: Value = serde_json::from_slice(&saved).unwrap();
    assert_eq!(saved["document"], accepted.yaml.as_ref());
    assert_eq!(saved["parameters"][2]["value"], 1.0);
    let output = render::<16>(
        &mut audio,
        &[TimedEvent {
            frame_offset: 3,
            event: ControlEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
        }],
    );
    assert_eq!((output.0[3], output.1[3]), (-0.25, -0.5));
    assert_eq!(output.2.parameter_updates, 0);
    let next = request(&mut host, "setParameter", json!(["drums.level", 0.125, 3]));
    assert_eq!(
        host.apply(&next).unwrap(),
        ParameterReply::Accepted {
            generation: 3,
            sequence: 4
        }
    );
    let undo = request(
        &mut host,
        "undoInstrumentSession",
        json!([editor.instance_id, editor.attachment_id, "2", "2"]),
    );
    host.admit_session_command(&undo).unwrap();
    let prepared = host.take_session_preparation().unwrap().prepare();
    host.submit_session_completion(prepared).unwrap();
    host.service_session_jobs().unwrap();
    host.service_quiescent(&mut audio).unwrap();
    assert_eq!(
        (
            host.session().owner_snapshot().undo_len,
            host.session().owner_snapshot().redo_len
        ),
        (0, 1)
    );
    assert_eq!(
        render::<16>(
            &mut audio,
            &[TimedEvent {
                frame_offset: 3,
                event: note()
            }]
        )
        .0[3],
        -0.125
    );
}
