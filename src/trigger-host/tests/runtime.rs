use dandrum_engine::graph::PortDirection;
use dandrum_engine::kernel::{ChannelCount, document::load_kernel_patch_str};
use dandrum_engine::patch::RenderSettings;
use dandrum_engine::preparation::{HostBuses, PreparationContext};
use dandrum_instrument_session::{ActivationHost, CommitError, InstrumentSession, SetZoneKeyRange};
use dandrum_trigger_host::TriggerHostConfiguration;
use dandrum_trigger_host::runtime::{
    ControlEvent, ParameterFeedback, PreparedRuntime, RuntimeError, TimedEvent, runtime_handoff,
};
use std::path::PathBuf;

fn configuration() -> TriggerHostConfiguration {
    TriggerHostConfiguration {
        sample_rate_hz: dandrum_trigger_host::SampleRate::new(48_000.0).unwrap(),
        max_block_frames: 16,
    }
}

fn session(note: u8) -> InstrumentSession {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let yaml = std::fs::read_to_string(repository.join("examples/patches/advanced-drum-kit.yaml"))
        .unwrap()
        .replace(
            "key_range: [36, 36]",
            &format!("key_range: [{note}, {note}]"),
        );
    planar_session(&yaml)
}

fn planar_session(yaml: &str) -> InstrumentSession {
    planar_session_at(yaml, configuration())
}

fn planar_session_at(yaml: &str, configuration: TriggerHostConfiguration) -> InstrumentSession {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let document = load_kernel_patch_str(yaml).unwrap();
    let mut buses = HostBuses::new();
    for port in document.root().ports() {
        let ChannelCount::Literal(channels) = port.channels() else {
            panic!("literal root channel count");
        };
        buses = match port.direction() {
            PortDirection::Input => buses.with_input(port.name(), *channels as usize),
            PortDirection::Output => buses.with_output(port.name(), *channels as usize),
        };
    }
    InstrumentSession::new_realtime_for_planar_host(
        yaml,
        PreparationContext::new_realtime(
            repository.join("examples/patches"),
            configuration.sample_rate_hz,
        ),
        dandrum_engine::render_settings::RealtimeRenderSettings {
            sample_rate_hz: configuration.sample_rate_hz,
            block_size_frames: configuration.max_block_frames,
            duration_frames: u64::from(configuration.max_block_frames),
        },
        buses,
    )
    .unwrap()
}

#[test]
fn a_changed_parameter_surface_is_refused_before_session_publication() {
    let mut original = session(36);
    let (_, snapshot) = original.attach().unwrap();
    let changed = planar_session(
        &snapshot
            .yaml
            .replace("min: -1, max: 1", "min: -0.5, max: 0.5"),
    );
    let (mut control, _audio) = runtime_handoff(prepared(36), 4).unwrap();
    let candidate = PreparedRuntime::new(changed.prepared(), configuration()).unwrap();
    assert!(control.offer(candidate).is_err());
    assert!(matches!(
        control.activate(changed.prepared(), &snapshot),
        Err(RuntimeError::IncompatibleRuntime)
    ));
    assert!(
        control.offer(prepared(60)).is_ok(),
        "failure leaves the slot available"
    );
}

fn prepared(note: u8) -> PreparedRuntime {
    PreparedRuntime::new(session(note).prepared(), configuration()).unwrap()
}

#[test]
fn invalid_configuration_or_foreign_layout_cannot_replace_the_running_runtime() {
    let session = session(36);
    for configuration in [
        TriggerHostConfiguration {
            sample_rate_hz: dandrum_trigger_host::SampleRate::new(48_000.0).unwrap(),
            max_block_frames: 0,
        },
        TriggerHostConfiguration {
            sample_rate_hz: dandrum_trigger_host::SampleRate::new(48_000.0).unwrap(),
            max_block_frames: 16_385,
        },
        TriggerHostConfiguration {
            sample_rate_hz: dandrum_trigger_host::SampleRate::new(44_100.0).unwrap(),
            max_block_frames: 16,
        },
        TriggerHostConfiguration {
            sample_rate_hz: dandrum_trigger_host::SampleRate::new(48_000.0).unwrap(),
            max_block_frames: 8,
        },
    ] {
        assert!(matches!(
            PreparedRuntime::new(session.prepared(), configuration),
            Err(RuntimeError::InvalidConfiguration)
        ));
    }
    let (mut control, _audio) = runtime_handoff(prepared(36), 4).unwrap();
    let changed_configuration = TriggerHostConfiguration {
        sample_rate_hz: dandrum_trigger_host::SampleRate::new(44_100.0).unwrap(),
        max_block_frames: 16,
    };
    let changed_session = planar_session_at(
        &std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/patches/advanced-drum-kit.yaml"),
        )
        .unwrap(),
        changed_configuration,
    );
    let changed_settings =
        PreparedRuntime::new(changed_session.prepared(), changed_configuration).unwrap();
    assert!(control.offer(changed_settings).is_err());

    let yaml = include_str!("../../clap-adapter/fixtures/signed-gain.yaml");
    let foreign = InstrumentSession::new(
        yaml,
        PreparationContext::new(".", 48_000),
        RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 16,
            duration_frames: 16,
        },
    )
    .unwrap();
    assert!(matches!(
        PreparedRuntime::new(foreign.prepared(), configuration()),
        Err(RuntimeError::UnsupportedLayout)
    ));
    assert!(control.offer(prepared(60)).is_ok());
}

#[test]
fn handoff_capacity_is_bounded_by_both_the_protocol_limit_and_dsp_queue() {
    for capacity in [0, 65, 1_025] {
        assert!(matches!(
            runtime_handoff(prepared(36), capacity),
            Err(RuntimeError::InvalidConfiguration)
        ));
    }
    assert!(runtime_handoff(prepared(36), 64).is_ok());
}

#[test]
fn invalid_blocks_are_silent_and_preserve_the_pending_replacement() {
    let (mut control, mut audio) = runtime_handoff(prepared(36), 4).unwrap();
    assert!(control.offer(prepared(60)).is_ok());
    for (left_frames, right_frames) in [(0, 0), (16, 15), (17, 17)] {
        let mut left = vec![1.0; left_frames];
        let mut right = vec![1.0; right_frames];
        assert_eq!(
            audio.process(&mut left, &mut right),
            Err(RuntimeError::InvalidBlock)
        );
        assert!(left.iter().chain(&right).all(|&value| value == 0.0));
        assert!(control.reclaim().is_none());
    }
    audio.process(&mut [1.0; 16], &mut [1.0; 16]).unwrap();
    assert!(control.reclaim().is_some());
}

#[test]
fn replacement_is_block_boundary_only_and_backpressure_lasts_until_reclamation() {
    let (mut control, mut audio) = runtime_handoff(prepared(36), 4).unwrap();
    assert!(control.reclaim().is_none());
    assert!(control.offer(prepared(60)).is_ok());
    let rejected = control
        .offer(prepared(72))
        .expect_err("one in-flight replacement");
    assert!(control.reclaim().is_none(), "no swap before process");
    audio.process(&mut [0.0; 16], &mut [0.0; 16]).unwrap();
    assert!(
        control.offer(rejected).is_err(),
        "retirement must first be reclaimed"
    );
    assert!(control.reclaim().is_some());
    assert!(control.reclaim().is_none());
    assert!(control.offer(prepared(72)).is_ok());
}

#[test]
fn bounded_note_handoff_reports_backpressure_and_preserves_the_rejected_note() {
    let (mut control, mut audio) = runtime_handoff(prepared(36), 4).unwrap();
    for _ in 0..4 {
        assert_eq!(control.try_send(ControlEvent::NoteOff { note: 0 }), Ok(()));
    }
    let hit = ControlEvent::NoteOn {
        note: 36,
        velocity: 100,
    };
    assert_eq!(control.try_send(hit), Err(RuntimeError::QueueFull));
    let (mut left, mut right) = ([1.0; 16], [1.0; 16]);
    audio.process(&mut left, &mut right).unwrap();
    assert_eq!(left, [0.0; 16]);
    assert_eq!(right, [0.0; 16]);
    control.try_send(hit).unwrap();
    audio.process(&mut left, &mut right).unwrap();
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
}

#[test]
fn host_notes_are_sample_accurate_and_invalid_batches_preserve_pending_work() {
    let (mut control, mut audio) = runtime_handoff(prepared(36), 4).unwrap();
    assert!(control.offer(prepared(60)).is_ok());
    let note = |frame_offset, note| TimedEvent {
        frame_offset,
        event: ControlEvent::NoteOn {
            note,
            velocity: 100,
        },
    };
    for invalid in [
        vec![note(16, 60)],
        vec![note(3, 60), note(2, 60)],
        vec![note(0, 128)],
        vec![note(0, 60); 5],
    ] {
        let (mut left, mut right) = ([1.0; 16], [1.0; 16]);
        assert_eq!(
            audio.process_events(&mut left, &mut right, &invalid),
            Err(RuntimeError::InvalidEvent)
        );
        assert_eq!((left, right), ([0.0; 16], [0.0; 16]));
        assert!(control.reclaim().is_none());
    }
    let (mut left, mut right) = ([1.0; 16], [1.0; 16]);
    audio
        .process_events(&mut left, &mut right, &[note(5, 60)])
        .unwrap();
    assert_eq!(&left[..5], &[0.0; 5]);
    assert_eq!(&right[..5], &[0.0; 5]);
    assert_eq!((left[5], right[5]), (-0.5, -0.5));
    assert!(control.reclaim().is_some());
    assert_eq!(
        control.try_send(ControlEvent::NoteOn {
            note: 36,
            velocity: 0
        }),
        Err(RuntimeError::InvalidEvent)
    );
    assert_eq!(
        control.try_send(ControlEvent::NoteOff { note: 128 }),
        Err(RuntimeError::InvalidEvent)
    );
}

#[test]
fn parameter_handoff_updates_signed_audio_and_reports_nonzero_host_offset() {
    let (mut control, mut audio) = runtime_handoff(prepared(36), 4).unwrap();
    let level = control
        .parameter_index("level")
        .expect("named control input");
    assert_eq!(control.parameter_index("missing"), None);
    assert_eq!(control.parameter_value(usize::MAX), None);
    assert_eq!(control.parameter_value(level), Some(1.0));
    control
        .try_send(ControlEvent::Parameter {
            index: level,
            value: 0.25,
        })
        .unwrap();
    assert_eq!(
        control.parameter_value(level),
        Some(1.0),
        "pending is not applied"
    );
    control
        .try_send(ControlEvent::NoteOn {
            note: 36,
            velocity: 100,
        })
        .unwrap();
    let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
    audio.process(&mut left, &mut right).unwrap();
    assert_eq!((left[0], right[0]), (-0.125, -0.125));
    assert_eq!(control.parameter_value(level), Some(0.25));
    assert_eq!(
        control.take_feedback(),
        Some(ParameterFeedback {
            index: level,
            value: 0.25,
            frame_offset: 0
        })
    );
    assert_eq!(control.take_feedback(), None);

    assert!(control.offer(prepared(60)).is_ok());
    control
        .try_send(ControlEvent::NoteOn {
            note: 60,
            velocity: 100,
        })
        .unwrap();
    audio.process(&mut left, &mut right).unwrap();
    assert_eq!(
        (left[0], right[0]),
        (-0.125, -0.125),
        "replacement retains host-owned scalar"
    );

    let (mut control, mut audio) = runtime_handoff(prepared(36), 4).unwrap();
    audio
        .process_events(
            &mut left,
            &mut right,
            &[
                TimedEvent {
                    frame_offset: 5,
                    event: ControlEvent::Parameter {
                        index: level,
                        value: 0.5,
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
    assert_eq!((left[5], right[5]), (-0.25, -0.25));
    assert_eq!(control.parameter_value(level), Some(0.5));
    assert_eq!(
        control.take_feedback(),
        Some(ParameterFeedback {
            index: level,
            value: 0.5,
            frame_offset: 5
        })
    );
}

#[test]
fn invalid_parameter_events_preserve_the_scalar_values_and_signed_audio() {
    let (mut control, mut audio) = runtime_handoff(prepared(36), 4).unwrap();
    let level = control.parameter_index("level").unwrap();
    for (index, value) in [
        (usize::MAX, 1.0),
        (level, f64::NAN),
        (level, f64::INFINITY),
        (level, -0.1),
        (level, 4.1),
    ] {
        let invalid = ControlEvent::Parameter { index, value };
        assert_eq!(control.try_send(invalid), Err(RuntimeError::InvalidEvent));
        let (mut left, mut right) = ([1.0; 16], [1.0; 16]);
        assert_eq!(
            audio.process_events(
                &mut left,
                &mut right,
                &[
                    TimedEvent {
                        frame_offset: 0,
                        event: ControlEvent::Parameter {
                            index: level,
                            value: 0.125
                        }
                    },
                    TimedEvent {
                        frame_offset: 3,
                        event: invalid
                    },
                ]
            ),
            Err(RuntimeError::InvalidEvent)
        );
        assert_eq!(control.parameter_value(level), Some(1.0));
        assert_eq!((left, right), ([0.0; 16], [0.0; 16]));
        assert_eq!(control.take_feedback(), None);
    }
    control
        .try_send(ControlEvent::NoteOn {
            note: 36,
            velocity: 100,
        })
        .unwrap();
    let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
    audio.process(&mut left, &mut right).unwrap();
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
}

#[test]
fn feedback_overflow_is_explicit_and_the_latest_scalar_remains_recoverable() {
    let (mut control, mut audio) = runtime_handoff(prepared(36), 2).unwrap();
    let level = control.parameter_index("level").unwrap();
    let mut left = [0.0; 16];
    let mut right = [0.0; 16];
    for (values, expected_dropped) in [([0.25, 0.5], 0), ([0.75, 0.125], 2)] {
        let events = values.map(|value| TimedEvent {
            frame_offset: 0,
            event: ControlEvent::Parameter {
                index: level,
                value,
            },
        });
        let report = audio
            .process_events(&mut left, &mut right, &events)
            .unwrap();
        assert_eq!(
            (report.parameter_updates, report.dropped_feedback),
            (2, expected_dropped)
        );
    }
    assert_eq!(control.parameter_value(level), Some(0.125));
    assert_eq!(control.take_feedback().unwrap().value, 0.25);
    assert_eq!(control.take_feedback().unwrap().value, 0.5);
    assert_eq!(control.take_feedback(), None);
    control
        .try_send(ControlEvent::NoteOn {
            note: 36,
            velocity: 100,
        })
        .unwrap();
    audio.process(&mut left, &mut right).unwrap();
    assert_eq!((left[0], right[0]), (-0.0625, -0.0625));
}

#[test]
fn reset_stops_active_and_queued_notes_and_retains_host_owned_values() {
    let (mut control, mut audio) = runtime_handoff(prepared(36), 4).unwrap();
    let level = control.parameter_index("level").unwrap();
    control
        .try_send(ControlEvent::Parameter {
            index: level,
            value: 0.5,
        })
        .unwrap();
    control
        .try_send(ControlEvent::NoteOn {
            note: 36,
            velocity: 100,
        })
        .unwrap();
    let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
    audio.process(&mut left, &mut right).unwrap();
    assert_eq!((left[0], right[0]), (-0.25, -0.25));
    control
        .try_send(ControlEvent::NoteOn {
            note: 36,
            velocity: 100,
        })
        .unwrap();
    control
        .try_send(ControlEvent::Parameter {
            index: level,
            value: 0.25,
        })
        .unwrap();
    audio.reset();
    audio.process(&mut left, &mut right).unwrap();
    assert_eq!((left, right), ([0.0; 16], [0.0; 16]));
    assert_eq!(control.parameter_value(level), Some(0.5));
    assert!(control.offer(prepared(60)).is_ok());
    audio.reset();
    control
        .try_send(ControlEvent::NoteOn {
            note: 60,
            velocity: 100,
        })
        .unwrap();
    assert!(audio.process(&mut left, &mut right).unwrap().swapped);
    assert_eq!((left[0], right[0]), (-0.25, -0.25));
    assert!(control.reclaim().is_some());
}

#[test]
fn unbound_control_roots_are_rejected_before_exposing_an_audio_runtime() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let yaml = std::fs::read_to_string(repository.join("examples/patches/advanced-drum-kit.yaml"))
        .unwrap();
    let unbound = InstrumentSession::new(
        &yaml,
        PreparationContext::new(repository.join("examples/patches"), 48_000),
        RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 16,
            duration_frames: 16,
        },
    )
    .unwrap();
    assert!(matches!(
        PreparedRuntime::new(unbound.prepared(), configuration()),
        Err(RuntimeError::UnsupportedLayout)
    ));
}

#[test]
fn session_publication_uses_a_cancellable_lease_and_rolls_back_under_backpressure() {
    let mut session = session(36);
    // Decode this fixed RIFF fixture independently of the production sampler.
    // Its 12,000-frame kick outlasts the bounded paused-producer observation.
    let wav = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/patches/assets/advanced-drums.wav"),
    )
    .unwrap();
    assert_eq!(&wav[36..40], b"data");
    let waveform: Vec<f32> = wav[44..]
        .chunks_exact(2)
        .map(|word| f32::from(i16::from_le_bytes([word[0], word[1]])) / 32768.0)
        .collect();
    assert_eq!(
        &waveform[..4],
        &[-0.5, -358.0 / 32768.0, 1554.0 / 32768.0, 4137.0 / 32768.0]
    );
    let initial = PreparedRuntime::new(session.prepared(), configuration()).unwrap();
    let (mut control, mut audio) = runtime_handoff(initial, 4).unwrap();
    let (attachment, original) = session.attach().unwrap();
    let intent = |note| SetZoneKeyRange {
        map_id: "kit".into(),
        zone_id: "kick".into(),
        key_range: [note, note],
    };
    let candidate = session.prepare_mapping(&attachment, intent(60)).unwrap();
    control
        .try_send(ControlEvent::NoteOn {
            note: 36,
            velocity: 100,
        })
        .unwrap();
    let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
    audio.process(&mut left, &mut right).unwrap();
    for frame in 0..16 {
        assert_eq!(
            (left[frame], right[frame]),
            (waveform[frame], waveform[frame])
        );
    }
    let lease = control.activate(candidate.prepared(), &original).unwrap();
    let producer = std::thread::current().id();
    // The control owner retains the unpublished lease while another actual
    // thread repeatedly completes callbacks. join() pauses the producer until
    // the reader has independently proved continued signed playback.
    let expected = waveform[16..528].to_vec();
    let mut audio = std::thread::spawn(move || {
        assert_ne!(std::thread::current().id(), producer);
        let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
        let callback = allocation_counter::measure(|| {
            for block in 0..32 {
                let report = audio.process(&mut left, &mut right).unwrap();
                assert!(
                    !report.admission_closed,
                    "a reserved immutable mapping must not silence audio"
                );
                assert!(
                    !report.swapped,
                    "an unpublished candidate must stay invisible"
                );
                for frame in 0..16 {
                    assert_eq!(
                        (left[frame], right[frame]),
                        (expected[block * 16 + frame], expected[block * 16 + frame])
                    );
                }
            }
        });
        assert_eq!(
            (
                callback.count_total,
                callback.count_current,
                callback.bytes_current
            ),
            (0, 0, 0)
        );
        audio
    })
    .join()
    .unwrap();
    let cancelled = allocation_counter::measure(|| drop(lease));
    assert_eq!(cancelled.count_total, 0);
    assert!(
        cancelled.count_current < 0,
        "cancelled candidate is destroyed off audio"
    );
    assert_eq!(session.snapshot(&attachment).unwrap(), original);
    audio.process(&mut left, &mut right).unwrap();
    for frame in 0..16 {
        assert_eq!(
            (left[frame], right[frame]),
            (waveform[528 + frame], waveform[528 + frame])
        );
    }
    audio.reset();
    let accepted = session.commit_mapping(candidate, &mut control).unwrap();
    assert_eq!((accepted.revision, accepted.undo_len), (1, 1));
    let blocked = session.prepare_mapping(&attachment, intent(72)).unwrap();
    assert!(matches!(
        session.commit_mapping(blocked, &mut control),
        Err(CommitError::Activation(RuntimeError::Busy))
    ));
    assert_eq!(session.snapshot(&attachment).unwrap(), accepted);
    assert!(control.reclaim().is_none());
    let report = audio
        .process_events(
            &mut left,
            &mut right,
            &[TimedEvent {
                frame_offset: 3,
                event: ControlEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                },
            }],
        )
        .unwrap();
    assert!(report.swapped);
    assert_eq!(&left[..3], &[0.0; 3]);
    assert_eq!((left[3], right[3]), (-0.5, -0.5));
    let blocked = session.prepare_mapping(&attachment, intent(72)).unwrap();
    assert!(matches!(
        session.commit_mapping(blocked, &mut control),
        Err(CommitError::Activation(RuntimeError::Busy))
    ));
    assert_eq!(session.snapshot(&attachment).unwrap(), accepted);
    drop(
        control
            .reclaim()
            .expect("retired runtime returned off audio"),
    );
    let retry = session.prepare_mapping(&attachment, intent(72)).unwrap();
    assert_eq!(
        session
            .commit_mapping(retry, &mut control)
            .unwrap()
            .revision,
        2
    );
}

#[test]
fn busy_runtime_mapping_completion_retains_failure_without_stealing_pending_audio() {
    use dandrum_instrument_session::{MappingAdmission, MappingRequest, RequestFailure, RequestId};

    let mut session = session(36);
    let (attachment, original) = session.attach().unwrap();
    let (mut control, mut audio) = runtime_handoff(
        PreparedRuntime::new(session.prepared(), configuration()).unwrap(),
        4,
    )
    .unwrap();
    let render = |audio: &mut dandrum_trigger_host::runtime::RuntimeAudio, note, swapped| {
        let (mut left, mut right) = ([1.0; 16], [1.0; 16]);
        let callback = allocation_counter::measure(|| {
            assert!(!audio.reset().admission_closed);
            let report = audio
                .process_events(
                    &mut left,
                    &mut right,
                    &[TimedEvent {
                        frame_offset: 0,
                        event: ControlEvent::NoteOn {
                            note,
                            velocity: 100,
                        },
                    }],
                )
                .unwrap();
            assert_eq!(report.swapped, swapped);
            assert!(!report.admission_closed);
        });
        assert_eq!(
            (
                callback.count_total,
                callback.count_current,
                callback.bytes_current
            ),
            (0, 0, 0)
        );
        (left, right)
    };
    let initial = render(&mut audio, 36, false);
    assert_eq!((initial.0[0], initial.1[0]), (-0.5, -0.5));
    assert_eq!(render(&mut audio, 60, false), ([0.0; 16], [0.0; 16]));

    let request = MappingRequest {
        instance: original.instance.clone(),
        request_id: RequestId(1),
        expected_revision: original.revision,
        intent: SetZoneKeyRange {
            map_id: "kit".into(),
            zone_id: "kick".into(),
            key_range: [60, 60],
        },
    };
    let MappingAdmission::Admitted(job) =
        session.admit_mapping(&attachment, request.clone()).unwrap()
    else {
        panic!("fresh mapping request must own preparation");
    };
    let completion = job.prepare();
    assert!(control.offer(prepared(36)).is_ok());
    let outcome = session.complete_mapping(completion, &mut control).unwrap();
    assert_eq!(outcome.request_id, request.request_id);
    assert_eq!(
        outcome.result,
        Err(RequestFailure::Activation("Busy".into()))
    );
    assert_eq!(session.snapshot(&attachment).unwrap(), original);
    assert_eq!(
        session
            .request_outcome(&attachment, request.request_id)
            .unwrap(),
        Some(outcome.clone())
    );
    assert!(
        control.reclaim().is_none(),
        "pending replacement was not consumed"
    );

    // The pre-existing candidate, not the rejected mapping, owns this swap.
    let swapped = render(&mut audio, 36, true);
    assert_eq!((swapped.0[0], swapped.1[0]), (-0.5, -0.5));
    assert_eq!(render(&mut audio, 60, false), ([0.0; 16], [0.0; 16]));
    assert!(
        control.offer(prepared(60)).is_err(),
        "retirement still owns the slot"
    );
    drop(
        control
            .reclaim()
            .expect("retired runtime returns off audio"),
    );
    assert!(control.reclaim().is_none());
    assert_eq!(session.snapshot(&attachment).unwrap(), original);
    let MappingAdmission::Completed(retained) =
        session.admit_mapping(&attachment, request).unwrap()
    else {
        panic!("failed request must recover its terminal outcome, not retry activation");
    };
    assert_eq!(retained, outcome);
}

#[test]
fn allocation_audit_independently_detects_allocation_and_preexisting_destruction() {
    let allocated = allocation_counter::measure(|| {
        std::hint::black_box(Box::new([0u8; 64]));
    });
    assert!(allocated.count_total > 0);
    let existing = Box::new([0u8; 64]);
    let destroyed = allocation_counter::measure(|| drop(std::hint::black_box(existing)));
    assert_eq!(destroyed.count_total, 0);
    assert!(destroyed.count_current < 0);
    assert!(destroyed.bytes_current < 0);
}

#[test]
fn first_callback_swaps_events_feedback_reset_and_short_blocks_allocate_or_free_nothing() {
    let session = session(36);
    let mut initial = None;
    let construction = allocation_counter::measure(|| {
        initial = Some(PreparedRuntime::new(session.prepared(), configuration()).unwrap());
    });
    assert!(
        construction.count_total > 0,
        "construction is an off-audio operation"
    );
    let (mut control, mut audio) = runtime_handoff(initial.unwrap(), 4).unwrap();
    let level = control.parameter_index("level").unwrap();
    assert!(control.offer(prepared(60)).is_ok());
    control.try_send(ControlEvent::NoteOff { note: 0 }).unwrap();
    control
        .try_send(ControlEvent::Parameter {
            index: level,
            value: 0.25,
        })
        .unwrap();
    let main_thread = std::thread::current().id();
    let audio = std::thread::spawn(move || {
        assert_ne!(std::thread::current().id(), main_thread);
        let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
        let callback = allocation_counter::measure(|| {
            let report = audio
                .process_events(
                    &mut left,
                    &mut right,
                    &[
                        TimedEvent {
                            frame_offset: 5,
                            event: ControlEvent::Parameter {
                                index: level,
                                value: 0.5,
                            },
                        },
                        TimedEvent {
                            frame_offset: 5,
                            event: ControlEvent::NoteOn {
                                note: 60,
                                velocity: 100,
                            },
                        },
                    ],
                )
                .unwrap();
            assert!(report.swapped);
            assert_eq!(&left[..5], &[0.0; 5]);
            assert_eq!((left[5], right[5]), (-0.25, -0.25));
            for frames in [1, 7, 16, 3, 16] {
                audio
                    .process(&mut left[..frames], &mut right[..frames])
                    .unwrap();
            }
            for _ in 0..4 {
                audio
                    .process_events(
                        &mut left,
                        &mut right,
                        &[
                            TimedEvent {
                                frame_offset: 0,
                                event: ControlEvent::Parameter {
                                    index: level,
                                    value: 0.75,
                                },
                            },
                            TimedEvent {
                                frame_offset: 15,
                                event: ControlEvent::Parameter {
                                    index: level,
                                    value: 0.125,
                                },
                            },
                        ],
                    )
                    .unwrap();
            }
            assert_eq!(
                audio.process_events(
                    &mut left,
                    &mut right,
                    &[TimedEvent {
                        frame_offset: 16,
                        event: ControlEvent::NoteOff { note: 60 }
                    },]
                ),
                Err(RuntimeError::InvalidEvent)
            );
            audio.reset();
            audio.process(&mut left, &mut right).unwrap();
            assert_eq!((left, right), ([0.0; 16], [0.0; 16]));
        });
        assert_eq!(
            (
                callback.count_total,
                callback.count_current,
                callback.bytes_current
            ),
            (0, 0, 0)
        );
        audio
    })
    .join()
    .unwrap();
    let retired = control
        .reclaim()
        .expect("retired ownership returns to main");
    let reclamation = allocation_counter::measure(|| drop(retired));
    assert_eq!(reclamation.count_total, 0);
    assert!(
        reclamation.count_current < 0,
        "retired DSP is destroyed off audio"
    );
    let stopped = allocation_counter::measure(|| drop(audio));
    assert!(
        stopped.count_current < 0,
        "active DSP is destroyed off audio after stop"
    );
}

#[test]
fn dropping_the_control_endpoint_first_never_reclaims_resources_in_process() {
    let (mut control, mut audio) = runtime_handoff(prepared(36), 2).unwrap();
    let level = control.parameter_index("level").unwrap();
    assert!(control.offer(prepared(60)).is_ok());
    control
        .try_send(ControlEvent::Parameter {
            index: level,
            value: 0.5,
        })
        .unwrap();
    control
        .try_send(ControlEvent::NoteOn {
            note: 60,
            velocity: 100,
        })
        .unwrap();
    drop(control);
    let audio = std::thread::spawn(move || {
        let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
        let callback = allocation_counter::measure(|| {
            assert!(audio.process(&mut left, &mut right).unwrap().swapped);
            assert_eq!((left[0], right[0]), (-0.25, -0.25));
            assert!(!audio.process(&mut left, &mut right).unwrap().swapped);
            audio.reset();
            audio.process(&mut left, &mut right).unwrap();
            assert_eq!((left, right), ([0.0; 16], [0.0; 16]));
        });
        assert_eq!(
            (
                callback.count_total,
                callback.count_current,
                callback.bytes_current
            ),
            (0, 0, 0)
        );
        audio
    })
    .join()
    .unwrap();
    let stopped = allocation_counter::measure(|| drop(audio));
    assert!(stopped.count_current < 0);
}

#[test]
fn unsupported_realtime_schedules_are_rejected_off_audio_before_activation() {
    let script = planar_session(
        r#"
metadata: { name: offline only }
ports:
  - { name: master, direction: output, signal: audio, channels: 2, maps_from: amp.audio_out }
module_definitions:
  - type: script_control
    implementation: script
    static_params:
      - { name: language, type: enum, default: rhai, allowed_values: [rhai] }
      - { name: source, type: string }
    ports:
      - { name: value, direction: output, signal: control, channels: 1 }
modules:
  - id: source
    type: script_control
    static:
      source: |
        fn process(ctx) { ctx.control("value", 0.5); }
  - { id: amp, type: gain, static: { channels: 2 } }
connections:
  - { from: source.value, to: amp.gain }
"#,
    );
    assert!(matches!(
        PreparedRuntime::new(script.prepared(), configuration()),
        Err(RuntimeError::UnsupportedLayout)
    ));
    let (mut control, _audio) = runtime_handoff(prepared(36), 4).unwrap();
    let (_, snapshot) = session(36).attach().unwrap();
    assert!(matches!(
        control.activate(script.prepared(), &snapshot),
        Err(RuntimeError::UnsupportedLayout)
    ));
    assert!(control.offer(prepared(60)).is_ok());
}
