use dandrum_instrument_session::SessionSnapshot;
use dandrum_trigger_host::TriggerHostConfiguration;
use dandrum_trigger_host::runtime::{
    ControlEvent, PreparedRuntime, QuiescentReport, RuntimeAudio, RuntimeControl, RuntimeError,
    TimedEvent, runtime_handoff,
};

#[path = "support/editor_notes.rs"]
mod fixture;
use fixture::*;

fn publish_pan(
    fixture: &mut Fixture,
    control: &mut RuntimeControl,
    from: f32,
    to: f32,
) -> SessionSnapshot {
    let yaml = fixture
        .session
        .owner_snapshot()
        .yaml
        .replace(&format!("pan: {from}"), &format!("pan: {to}"));
    let candidate = fixture
        .session
        .prepare_document_replacement_owned(&yaml)
        .unwrap();
    fixture
        .session
        .commit_document_replacement(candidate, control)
        .unwrap()
}

fn render_events(
    audio: &mut RuntimeAudio,
    events: &[TimedEvent],
    swapped: bool,
) -> ([f32; FRAMES], [f32; FRAMES]) {
    let (mut left, mut right) = ([1.0; FRAMES], [1.0; FRAMES]);
    let measured = allocation_counter::measure(|| {
        assert_eq!(
            audio
                .process_events(&mut left, &mut right, events)
                .unwrap()
                .swapped,
            swapped
        );
    });
    assert_eq!(
        (
            measured.count_total,
            measured.count_current,
            measured.bytes_current
        ),
        (0, 0, 0),
        "real callback neither allocates nor destroys an owner"
    );
    (left, right)
}

fn render(audio: &mut RuntimeAudio, note: u8, swapped: bool) -> ([f32; FRAMES], [f32; FRAMES]) {
    render_events(
        audio,
        &[TimedEvent {
            frame_offset: 3,
            event: ControlEvent::NoteOn {
                note,
                velocity: 100,
            },
        }],
        swapped,
    )
}

// Expected gains are explicit fixture expectations, independent of the DSP.
fn assert_sample(output: &([f32; FRAMES], [f32; FRAMES]), left_gain: f32, right_gain: f32) {
    for (plane, gain) in [(&output.0, left_gain), (&output.1, right_gain)] {
        assert_eq!(&plane[..3], &[0.0; 3]);
        assert_eq!(&plane[3..7], &PREFIX_L.map(|value| value * gain));
        assert_eq!(&plane[7..], &[TAIL.0 * gain; 9]);
    }
}

#[test]
fn quiescent_adoption_destroys_the_predecessor_and_restores_capacity_without_a_render() {
    for second_publication in [false, true] {
        let mut fixture = Fixture::new("gated", 4);
        let (mut control, mut audio) = fixture.runtime(4);
        let accepted = publish_pan(&mut fixture, &mut control, -0.5, 0.5);
        let measured = allocation_counter::measure(|| {
            assert_eq!(
                control.service_quiescent(&mut audio),
                Ok(QuiescentReport {
                    adopted: true,
                    reclaimed_runtimes: 1
                })
            );
        });
        assert_eq!(measured.count_total, 0);
        assert!(
            measured.count_current < 0 && measured.bytes_current < 0,
            "the calling control thread destroys the replaced graph"
        );
        assert_eq!(
            fixture.session.owner_snapshot(),
            accepted,
            "servicing an accepted graph is not a new session publication"
        );
        assert!(
            control.reclaim().is_none(),
            "no artificial retirement awaits audio"
        );
        for _ in 0..3 {
            assert_eq!(
                control.service_quiescent(&mut audio),
                Ok(QuiescentReport::default())
            );
        }
        if second_publication {
            // A second actual publication succeeds without any intervening process.
            let second = publish_pan(&mut fixture, &mut control, 0.5, 0.25);
            assert_eq!(second.revision, accepted.revision + 1);
            assert_eq!(
                control.service_quiescent(&mut audio),
                Ok(QuiescentReport {
                    adopted: true,
                    reclaimed_runtimes: 1
                })
            );
        }
        let output = render(&mut audio, 60, false);
        assert_sample(&output, if second_publication { 0.75 } else { 0.5 }, 1.0);
    }
}

#[test]
fn foreign_quiescent_pair_is_rejected_before_either_nontrivial_instance_changes() {
    let mut a = Fixture::new("gated", 4);
    let mut b = Fixture::new("gated", 4);
    let (mut a_control, mut a_audio) = a.runtime(4);
    let (mut b_control, mut b_audio) = b.runtime(4);
    for output in [
        render(&mut a_audio, 60, false),
        render(&mut b_audio, 61, false),
    ] {
        assert_eq!(&output.0[3..7], &PREFIX_L);
        assert_eq!(&output.1[3..7], &PREFIX_R);
    }
    let a_accepted = publish_pan(&mut a, &mut a_control, -0.5, 0.5);
    let b_accepted = publish_pan(&mut b, &mut b_control, -0.5, -0.75);
    let rejected = allocation_counter::measure(|| {
        assert_eq!(
            a_control.service_quiescent(&mut b_audio),
            Err(RuntimeError::ForeignEndpoints)
        );
        assert_eq!(
            b_control.service_quiescent(&mut a_audio),
            Err(RuntimeError::ForeignEndpoints)
        );
    });
    assert_eq!(
        (
            rejected.count_total,
            rejected.count_current,
            rejected.bytes_current
        ),
        (0, 0, 0),
        "rejection neither allocates nor consumes an endpoint's owners"
    );
    assert_eq!(a.session.owner_snapshot(), a_accepted);
    assert_eq!(b.session.owner_snapshot(), b_accepted);
    assert!(
        a_control.offer(a.prepared()).is_err(),
        "A still has its accepted pending runtime"
    );
    assert!(
        b_control.offer(b.prepared()).is_err(),
        "B still has its accepted pending runtime"
    );
    let a_output = render(&mut a_audio, 62, true);
    assert_sample(&a_output, 0.5, 1.0);
    let b_output = render(&mut b_audio, 63, true);
    assert_sample(&b_output, 1.0, 0.25);
    let rejected = allocation_counter::measure(|| {
        assert_eq!(
            a_control.service_quiescent(&mut b_audio),
            Err(RuntimeError::ForeignEndpoints)
        );
        assert_eq!(
            b_control.service_quiescent(&mut a_audio),
            Err(RuntimeError::ForeignEndpoints)
        );
    });
    assert_eq!(
        (
            rejected.count_total,
            rejected.count_current,
            rejected.bytes_current
        ),
        (0, 0, 0),
        "a foreign pair cannot destroy either already-retired owner"
    );
    assert!(a_control.offer(a.prepared()).is_err());
    assert!(b_control.offer(b.prepared()).is_err());
    for (control, audio) in [
        (&mut a_control, &mut a_audio),
        (&mut b_control, &mut b_audio),
    ] {
        assert_eq!(
            control.service_quiescent(audio),
            Ok(QuiescentReport {
                adopted: false,
                reclaimed_runtimes: 1
            })
        );
    }
}
#[test]
fn returned_audio_reclaims_a_real_retirement_on_control_and_accepts_another_publication() {
    let mut fixture = Fixture::new("gated", 4);
    let (mut control, mut audio) = fixture.runtime(4);
    assert_sample(&render(&mut audio, 60, false), 1.0, 0.5);
    let accepted = publish_pan(&mut fixture, &mut control, -0.5, 0.5);
    let control_thread = std::thread::current().id();
    let mut audio = std::thread::spawn(move || {
        assert_ne!(std::thread::current().id(), control_thread);
        assert_sample(&render(&mut audio, 61, true), 0.5, 1.0);
        audio
    })
    .join()
    .unwrap();
    assert!(
        control.offer(fixture.prepared()).is_err(),
        "retirement still occupies activation capacity"
    );
    let reclaimed = allocation_counter::measure(|| {
        assert_eq!(std::thread::current().id(), control_thread);
        assert_eq!(
            control.service_quiescent(&mut audio),
            Ok(QuiescentReport {
                adopted: false,
                reclaimed_runtimes: 1,
            })
        );
    });
    assert_eq!(reclaimed.count_total, 0);
    assert!(
        reclaimed.count_current < 0 && reclaimed.bytes_current < 0,
        "the returned retired graph is destroyed on the original control thread"
    );
    assert_eq!(fixture.session.owner_snapshot(), accepted);
    assert!(
        control.reclaim().is_none(),
        "one bounded service consumed the retirement"
    );
    let second = publish_pan(&mut fixture, &mut control, 0.5, -0.5);
    assert_eq!(second.revision, accepted.revision + 1);
    assert_eq!(
        control.service_quiescent(&mut audio),
        Ok(QuiescentReport {
            adopted: true,
            reclaimed_runtimes: 1,
        })
    );
    assert_sample(&render(&mut audio, 62, false), 1.0, 0.5);
    for _ in 0..3 {
        let idle = allocation_counter::measure(|| {
            assert_eq!(
                control.service_quiescent(&mut audio),
                Ok(QuiescentReport::default())
            );
        });
        assert_eq!(
            (idle.count_total, idle.count_current, idle.bytes_current),
            (0, 0, 0)
        );
    }
    let sustained = render_events(&mut audio, &[], false);
    assert_eq!(sustained, ([TAIL.0; FRAMES], [TAIL.1; FRAMES]));
}
#[test]
fn quiescent_adoption_resumes_a_still_held_editor_note_only_on_an_actual_block() {
    let owner = std::num::NonZeroU64::new(17).unwrap();
    for heard in [false, true] {
        let mut fixture = Fixture::new("gated", 4);
        let (mut control, mut audio) = fixture.runtime(4);
        control.editor_note_on(owner, 60, 100).unwrap();
        if heard {
            let old = render_events(&mut audio, &[], false);
            assert_eq!(&old.0[..4], &PREFIX_L);
            assert_eq!(&old.1[..4], &PREFIX_R);
        }
        publish_pan(&mut fixture, &mut control, -0.5, 0.5);
        assert!(control.service_quiescent(&mut audio).unwrap().adopted);
        let resumed = render_events(&mut audio, &[], false);
        assert_eq!(&resumed.0[..4], &PREFIX_R);
        assert_eq!(&resumed.1[..4], &PREFIX_L);
        assert_eq!(&resumed.0[4..], &[-0.0625; 12]);
        assert_eq!(&resumed.1[4..], &[-0.125; 12]);
        assert_eq!(
            render_events(&mut audio, &[], false),
            ([-0.0625; FRAMES], [-0.125; FRAMES])
        );
        control.editor_note_off(owner, 60).unwrap();
        assert_eq!(
            render_events(&mut audio, &[], false),
            ([0.0; FRAMES], [0.0; FRAMES])
        );
    }
}

#[test]
fn quiescent_adoption_cannot_replay_a_released_cancelled_or_reset_editor_onset() {
    let owner = std::num::NonZeroU64::new(17).unwrap();
    // Every case begins with a real accepted onset; some have already sounded.
    for (heard, release, detach, reset) in [
        (false, true, false, false),
        (true, true, false, false),
        (false, false, true, false),
        (true, false, true, false),
        (false, true, true, false),
        (false, false, false, true),
    ] {
        let mut fixture = Fixture::new("gated", 4);
        let (mut control, mut audio) = fixture.runtime(4);
        control.editor_note_on(owner, 60, 100).unwrap();
        if heard {
            let old = render_events(&mut audio, &[], false);
            assert_eq!(&old.0[..4], &PREFIX_L);
            assert_eq!(&old.1[..4], &PREFIX_R);
        }
        if release {
            control.editor_note_off(owner, 60).unwrap();
        }
        if detach {
            control.detach_editor_notes(owner);
        }
        if reset {
            assert!(!audio.reset().admission_closed);
        }
        publish_pan(&mut fixture, &mut control, -0.5, 0.5);
        assert!(control.service_quiescent(&mut audio).unwrap().adopted);
        for _ in 0..3 {
            assert_eq!(
                render_events(&mut audio, &[], false),
                ([0.0; FRAMES], [0.0; FRAMES])
            );
        }
        // Suppression belongs to the old onset; a genuine new press still works.
        control.editor_note_on(owner, 60, 100).unwrap();
        let fresh = render_events(&mut audio, &[], false);
        assert_eq!(&fresh.0[..4], &PREFIX_R);
        assert_eq!(&fresh.1[..4], &PREFIX_L);
        control.detach_editor_notes(owner);
        assert_eq!(
            render_events(&mut audio, &[], false),
            ([0.0; FRAMES], [0.0; FRAMES])
        );
    }
}

#[test]
fn quiescent_adoption_retains_host_pitch_arbitration_until_a_real_host_release() {
    let owner = std::num::NonZeroU64::new(17).unwrap();
    let mut fixture = Fixture::new("gated", 4);
    let (mut control, mut audio) = fixture.runtime(4);
    assert_sample(&render(&mut audio, 60, false), 1.0, 0.5);
    control.editor_note_on(owner, 60, 100).unwrap();
    publish_pan(&mut fixture, &mut control, -0.5, 0.5);
    assert!(control.service_quiescent(&mut audio).unwrap().adopted);
    // A fresh graph does not resume a host voice, and host ownership still
    // suppresses the desired same-pitch editor note.
    assert_eq!(
        render_events(&mut audio, &[], false),
        ([0.0; FRAMES], [0.0; FRAMES])
    );
    let released = render_events(
        &mut audio,
        &[TimedEvent {
            frame_offset: 7,
            event: ControlEvent::NoteOff { note: 60 },
        }],
        false,
    );
    assert_eq!(
        released,
        ([0.0; FRAMES], [0.0; FRAMES]),
        "host-touched pitch stays suppressed for this block"
    );
    let resumed = render_events(&mut audio, &[], false);
    assert_eq!(&resumed.0[..4], &PREFIX_R);
    assert_eq!(&resumed.1[..4], &PREFIX_L);
    assert_eq!(&resumed.0[4..], &[-0.0625; 12]);
    assert_eq!(&resumed.1[4..], &[-0.125; 12]);
    control.detach_editor_notes(owner);
    assert_eq!(
        render_events(&mut audio, &[], false),
        ([0.0; FRAMES], [0.0; FRAMES])
    );
}

fn configuration() -> TriggerHostConfiguration {
    TriggerHostConfiguration {
        sample_rate_hz: 48_000,
        max_block_frames: FRAMES as u32,
    }
}

fn packaged_host() -> (tempfile::TempDir, dandrum_trigger_host::TriggerHost) {
    use dandrum_trigger_host::{ADVANCED_DRUM_KIT_DOCUMENT, ADVANCED_DRUM_KIT_SAMPLE, TriggerHost};
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("Trigger.resources");
    std::fs::create_dir_all(root.join("instrument/assets")).unwrap();
    let repository = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
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
    let host = TriggerHost::open(root, configuration()).unwrap();
    (temporary, host)
}

#[test]
fn quiescent_adoption_preserves_applied_values_feedback_and_queued_scalar_and_note_events() {
    use dandrum_instrument_session::SetZoneKeyRange;
    use dandrum_trigger_host::runtime::ParameterFeedback;
    let (_package, mut host) = packaged_host();
    let (mut control, mut audio) = runtime_handoff(
        PreparedRuntime::new(host.session().prepared(), configuration()).unwrap(),
        4,
    )
    .unwrap();
    let level = control.parameter_index("level").unwrap();
    let pan = control.parameter_index("pan").unwrap();
    let prior = render_events(
        &mut audio,
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
        false,
    );
    assert_eq!(&prior.0[..5], &[0.0; 5]);
    assert_eq!(&prior.1[..5], &[0.0; 5]);
    assert_eq!((prior.0[5], prior.1[5]), (-0.25, -0.25));
    let (attachment, _) = host.session_mut().attach().unwrap();
    let candidate = host
        .session()
        .prepare_mapping(
            &attachment,
            SetZoneKeyRange {
                map_id: "kit".into(),
                zone_id: "kick".into(),
                key_range: [60, 60],
            },
        )
        .unwrap();
    let accepted = host
        .session_mut()
        .commit_mapping(candidate, &mut control)
        .unwrap();
    for event in [
        ControlEvent::Parameter {
            index: level,
            value: 2.0,
        },
        ControlEvent::Parameter {
            index: pan,
            value: 0.5,
        },
        ControlEvent::NoteOn {
            note: 60,
            velocity: 100,
        },
        ControlEvent::NoteOff { note: 0 },
    ] {
        control.try_send(event).unwrap();
    }
    assert_eq!(
        control.service_quiescent(&mut audio),
        Ok(QuiescentReport {
            adopted: true,
            reclaimed_runtimes: 1
        })
    );
    assert_eq!(control.parameter_index("level"), Some(level));
    assert_eq!(control.parameter_index("pan"), Some(pan));
    assert_eq!(
        control.parameter_value(level),
        Some(0.5),
        "quiescent service does not apply queued scalars"
    );
    assert_eq!(control.parameter_value(pan), Some(0.0));
    assert_eq!(
        control.take_feedback(),
        Some(ParameterFeedback {
            index: level,
            value: 0.5,
            frame_offset: 5
        })
    );
    assert_eq!(control.take_feedback(), None);
    assert_eq!(
        control.try_send(ControlEvent::NoteOn {
            note: 1,
            velocity: 100
        }),
        Err(RuntimeError::QueueFull),
        "service leaves the full ordinary event queue intact"
    );
    let output = render_events(&mut audio, &[], false);
    assert_eq!((output.0[0], output.1[0]), (-0.5, -1.0));
    assert_eq!(
        (output.0[1], output.1[1]),
        (-358.0 / 32768.0, -716.0 / 32768.0)
    );
    assert_eq!(control.parameter_value(level), Some(2.0));
    assert_eq!(control.parameter_value(pan), Some(0.5));
    assert_eq!(
        control.take_feedback(),
        Some(ParameterFeedback {
            index: level,
            value: 2.0,
            frame_offset: 0
        })
    );
    assert_eq!(
        control.take_feedback(),
        Some(ParameterFeedback {
            index: pan,
            value: 0.5,
            frame_offset: 0
        })
    );
    assert_eq!(control.take_feedback(), None);
    assert_eq!(host.session().snapshot(&attachment).unwrap(), accepted);
}
