use dandrum_trigger_host::TriggerHostConfiguration;
use dandrum_trigger_host::runtime::{
    ControlEvent, PreparedRuntime, RuntimeAudio, RuntimeControl, TimedEvent, runtime_handoff,
};

#[path = "support/editor_notes.rs"]
mod fixture;
use fixture::*;

fn on(note: u8, frame_offset: u32) -> TimedEvent {
    TimedEvent {
        frame_offset,
        event: ControlEvent::NoteOn {
            note,
            velocity: 100,
        },
    }
}
fn off(note: u8, frame_offset: u32) -> TimedEvent {
    TimedEvent {
        frame_offset,
        event: ControlEvent::NoteOff { note },
    }
}

fn render(audio: &mut RuntimeAudio, events: &[TimedEvent]) -> ([f32; FRAMES], [f32; FRAMES]) {
    let (mut left, mut right) = ([1.0; FRAMES], [1.0; FRAMES]);
    let allocations = allocation_counter::measure(|| {
        audio.process_events(&mut left, &mut right, events).unwrap();
    });
    assert_eq!(
        (
            allocations.count_total,
            allocations.count_current,
            allocations.bytes_current
        ),
        (0, 0, 0),
        "callback must neither allocate nor destroy ownership"
    );
    (left, right)
}

fn assert_tail(output: &([f32; FRAMES], [f32; FRAMES]), voices: usize) {
    assert_eq!(output.0, [TAIL.0 * voices as f32; FRAMES]);
    assert_eq!(output.1, [TAIL.1 * voices as f32; FRAMES]);
}

#[test]
fn signed_gated_child_releases_at_host_offset_but_one_shot_keeps_its_tail() {
    for mode in ["gated", "one_shot"] {
        let fixture = Fixture::new(mode, 4);
        let (_, mut audio) = fixture.runtime(4);
        let first = render(&mut audio, &[on(60, 0)]);
        assert_eq!(&first.0[..4], &PREFIX_L);
        assert_eq!(&first.1[..4], &PREFIX_R);
        assert_eq!(&first.0[4..], &[TAIL.0; 12]);
        assert_eq!(&first.1[4..], &[TAIL.1; 12]);
        assert_tail(&render(&mut audio, &[]), 1);
        let released = render(&mut audio, &[off(60, 7)]);
        assert_eq!(&released.0[..7], &[TAIL.0; 7]);
        assert_eq!(&released.1[..7], &[TAIL.1; 7]);
        let tail = if mode == "gated" { (0.0, 0.0) } else { TAIL };
        assert_eq!(&released.0[7..], &[tail.0; 9]);
        assert_eq!(&released.1[7..], &[tail.1; 9]);
        assert_tail(&render(&mut audio, &[]), usize::from(mode == "one_shot"));
    }
}

#[test]
fn detaching_editor_releases_its_gated_child_and_preserves_unrelated_host_note_across_threads() {
    let fixture = Fixture::new("gated", 4);
    let (mut control, mut audio) = fixture.runtime(4);
    let owner = std::num::NonZeroU64::new(17).unwrap();
    control.editor_note_on(owner, 60, 100).unwrap();
    let (output_tx, output_rx) = std::sync::mpsc::channel();
    let (continue_tx, continue_rx) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        output_tx.send(render(&mut audio, &[on(61, 0)])).unwrap();
        continue_rx.recv().unwrap();
        output_tx.send(render(&mut audio, &[])).unwrap();
        output_tx.send(render(&mut audio, &[off(61, 7)])).unwrap();
        audio // Return all graph ownership for destruction on control.
    });
    let first = output_rx.recv().unwrap();
    assert_eq!(&first.0[..4], &PREFIX_L.map(|value| value * 2.0));
    assert_eq!(&first.1[..4], &PREFIX_R.map(|value| value * 2.0));
    assert_eq!(&first.0[4..], &[TAIL.0 * 2.0; 12]);
    control.detach_editor_notes(owner);
    control.detach_editor_notes(owner);
    continue_tx.send(()).unwrap();
    let detached = output_rx.recv().unwrap();
    let host_released = output_rx.recv().unwrap();
    let audio = worker.join().unwrap();
    assert_tail(&detached, 1);
    assert_eq!(&host_released.0[..7], &[TAIL.0; 7]);
    assert_eq!(&host_released.1[..7], &[TAIL.1; 7]);
    assert_eq!(&host_released.0[7..], &[0.0; 9]);
    assert_eq!(&host_released.1[7..], &[0.0; 9]);
    drop(audio);
}

#[test]
fn rejected_editor_values_and_conflicting_owner_preserve_the_held_note() {
    use dandrum_trigger_host::runtime::RuntimeError;
    let fixture = Fixture::new("gated", 4);
    let (mut control, mut audio) = fixture.runtime(4);
    let owner = std::num::NonZeroU64::new(17).unwrap();
    let other = std::num::NonZeroU64::new(18).unwrap();
    control.editor_note_on(owner, 60, 100).unwrap();
    for (note, velocity) in [(128, 100), (60, 0), (60, 128)] {
        assert_eq!(
            control.editor_note_on(owner, note, velocity),
            Err(RuntimeError::InvalidEvent)
        );
    }
    assert_eq!(
        control.editor_note_on(other, 60, 100),
        Err(RuntimeError::EditorOwnerConflict)
    );
    assert_eq!(
        control.editor_note_off(owner, 128),
        Err(RuntimeError::InvalidEvent)
    );
    assert_eq!(
        control.editor_note_off(other, 60),
        Err(RuntimeError::EditorOwnerConflict)
    );
    control.detach_editor_notes(other);
    let first = render(&mut audio, &[]);
    assert_eq!(&first.0[..4], &PREFIX_L);
    assert_eq!(&first.1[..4], &PREFIX_R);
    assert_tail(&render(&mut audio, &[]), 1);
    control.editor_note_on(owner, 60, 100).unwrap();
    let retriggered = render(&mut audio, &[]);
    assert_eq!(&retriggered.0[..4], &PREFIX_L);
    assert_eq!(&retriggered.1[..4], &PREFIX_R);
    control.detach_editor_notes(owner);
    assert_tail(&render(&mut audio, &[]), 0);
}

#[test]
fn ordinary_unheard_tap_plays_once_but_detach_cancels_held_and_released_onsets() {
    let owner = std::num::NonZeroU64::new(17).unwrap();
    for mode in ["gated", "one_shot"] {
        let fixture = Fixture::new(mode, 4);
        for (release, detach) in [(true, false), (false, true), (true, true)] {
            let (mut control, mut audio) = fixture.runtime(4);
            control.editor_note_on(owner, 60, 100).unwrap();
            if release {
                control.editor_note_off(owner, 60).unwrap();
                control.editor_note_off(owner, 60).unwrap();
            }
            if detach {
                control.detach_editor_notes(owner);
            }
            let first = render(&mut audio, &[]);
            if detach {
                assert_tail(&first, 0);
            } else {
                assert_eq!(&first.0[..4], &PREFIX_L);
                assert_eq!(&first.1[..4], &PREFIX_R);
                assert_eq!(&first.0[4..], &[TAIL.0; 12]);
                assert_eq!(&first.1[4..], &[TAIL.1; 12]);
            }
            for _ in 0..3 {
                assert_tail(
                    &render(&mut audio, &[]),
                    usize::from(!detach && mode == "one_shot"),
                );
            }
        }
    }
}

#[test]
fn host_takes_over_editor_pitch_at_its_offset_and_editor_cleanup_cannot_release_host() {
    let fixture = Fixture::new("gated", 4);
    let (mut control, mut audio) = fixture.runtime(4);
    let owner = std::num::NonZeroU64::new(17).unwrap();
    control.editor_note_on(owner, 60, 100).unwrap();
    let first = render(&mut audio, &[]);
    assert_eq!(&first.0[..4], &PREFIX_L);
    let takeover = render(&mut audio, &[on(60, 5)]);
    assert_eq!(&takeover.0[..5], &[TAIL.0; 5]);
    assert_eq!(&takeover.1[..5], &[TAIL.1; 5]);
    assert_eq!(&takeover.0[5..9], &PREFIX_L);
    assert_eq!(&takeover.1[5..9], &PREFIX_R);
    assert_eq!(&takeover.0[9..], &[TAIL.0; 7]);
    assert_eq!(&takeover.1[9..], &[TAIL.1; 7]);
    control.editor_note_off(owner, 60).unwrap();
    control.detach_editor_notes(owner);
    assert_tail(&render(&mut audio, &[]), 1);
    let released = render(&mut audio, &[off(60, 7)]);
    assert_eq!(&released.0[..7], &[TAIL.0; 7]);
    assert_eq!(&released.1[..7], &[TAIL.1; 7]);
    assert_eq!(&released.0[7..], &[0.0; 9]);
    assert_eq!(&released.1[7..], &[0.0; 9]);
}

#[test]
fn host_held_and_touched_pitch_suppresses_editor_until_later_untouched_block() {
    let fixture = Fixture::new("gated", 4);
    let owner = std::num::NonZeroU64::new(17).unwrap();
    for keep_held in [true, false] {
        let (mut control, mut audio) = fixture.runtime(4);
        let first = render(&mut audio, &[on(60, 0)]);
        assert_eq!(&first.0[..4], &PREFIX_L);
        control.editor_note_on(owner, 60, 100).unwrap();
        if !keep_held {
            control.editor_note_off(owner, 60).unwrap();
        }
        assert_tail(&render(&mut audio, &[]), 1);
        let released = render(&mut audio, &[off(60, 7)]);
        assert_eq!(&released.0[..7], &[TAIL.0; 7]);
        assert_eq!(&released.1[..7], &[TAIL.1; 7]);
        assert_eq!(&released.0[7..], &[0.0; 9]);
        assert_eq!(&released.1[7..], &[0.0; 9]);
        let next = render(&mut audio, &[]);
        if keep_held {
            assert_eq!(&next.0[..4], &PREFIX_L);
            assert_eq!(&next.1[..4], &PREFIX_R);
        } else {
            assert_tail(&next, 0);
        }
        control.detach_editor_notes(owner);
        assert_tail(&render(&mut audio, &[]), 0);
    }
    let (mut control, mut audio) = fixture.runtime(4);
    control.editor_note_on(owner, 60, 100).unwrap();
    let touched = render(&mut audio, &[on(60, 3), off(60, 11)]);
    assert_eq!(&touched.0[..3], &[0.0; 3]);
    assert_eq!(&touched.1[..3], &[0.0; 3]);
    assert_eq!(&touched.0[3..7], &PREFIX_L);
    assert_eq!(&touched.1[3..7], &PREFIX_R);
    assert_eq!(&touched.0[7..11], &[TAIL.0; 4]);
    assert_eq!(&touched.1[7..11], &[TAIL.1; 4]);
    assert_eq!(&touched.0[11..], &[0.0; 5]);
    let resumed = render(&mut audio, &[]);
    assert_eq!(&resumed.0[..4], &PREFIX_L);
    assert_eq!(&resumed.1[..4], &PREFIX_R);
}

#[test]
fn stale_attachment_cleanup_cannot_cancel_new_owner_or_release_its_sounding_child() {
    use dandrum_trigger_host::runtime::RuntimeError;
    let fixture = Fixture::new("gated", 4);
    let old = std::num::NonZeroU64::new(17).unwrap();
    let new = std::num::NonZeroU64::new(18).unwrap();
    for heard in [false, true] {
        let (mut control, mut audio) = fixture.runtime(4);
        control.editor_note_on(old, 60, 100).unwrap();
        if heard {
            let first = render(&mut audio, &[]);
            assert_eq!(&first.0[..4], &PREFIX_L);
            assert_eq!(&first.1[..4], &PREFIX_R);
        }
        control.editor_note_off(old, 60).unwrap();
        control.editor_note_on(new, 60, 90).unwrap();
        control.detach_editor_notes(old);
        let replacement = render(&mut audio, &[]);
        assert_eq!(&replacement.0[..4], &PREFIX_L);
        assert_eq!(&replacement.1[..4], &PREFIX_R);
        assert_eq!(
            control.editor_note_off(old, 60),
            Err(RuntimeError::EditorOwnerConflict)
        );
        control.detach_editor_notes(old);
        assert_tail(&render(&mut audio, &[]), 1);
        control.detach_editor_notes(new);
        assert_tail(&render(&mut audio, &[]), 0);
    }
}

#[test]
fn saturation_fixture_calibrates_real_graph_capacity_and_sixty_four_gated_children() {
    let fixture = Fixture::new("gated", 128);
    let graph = dandrum_engine::graph_processor::RealtimeGraphProcessor::from_compiled_patch(
        fixture.session.prepared().compiled_patch().clone(),
        48_000.0,
        &dandrum_engine::PreparedSamplerAssets::empty(),
        FRAMES,
    );
    assert_eq!(graph.pending_event_capacity(), 128);
    let (_, mut audio) = fixture.runtime(64);
    let notes: Vec<_> = (0..64).map(|note| on(note, 0)).collect();
    let first = render(&mut audio, &notes);
    assert_eq!(&first.0[..4], &PREFIX_L.map(|value| value * 64.0));
    assert_eq!(&first.1[..4], &PREFIX_R.map(|value| value * 64.0));
    assert_tail(&render(&mut audio, &[]), 64);
    let releases: Vec<_> = (0..64).map(|note| off(note, 7)).collect();
    let released = render(&mut audio, &releases);
    assert_eq!(&released.0[..7], &[TAIL.0 * 64.0; 7]);
    assert_eq!(&released.1[..7], &[TAIL.1 * 64.0; 7]);
    assert_eq!(&released.0[7..], &[0.0; 9]);
    assert_eq!(&released.1[7..], &[0.0; 9]);
}

fn fill_ordinary_queue(control: &mut RuntimeControl) {
    for _ in 0..64 {
        control
            .try_send(ControlEvent::NoteOff { note: 127 })
            .unwrap();
    }
    assert_eq!(
        control.try_send(ControlEvent::NoteOff { note: 127 }),
        Err(dandrum_trigger_host::runtime::RuntimeError::QueueFull)
    );
}

#[test]
fn saturated_cleanup_precedes_ordinary_queue_and_preserves_all_valid_host_events() {
    let fixture = Fixture::new("gated", 128);
    let (mut control, mut audio) = fixture.runtime(64);
    let owner = std::num::NonZeroU64::new(17).unwrap();
    for note in 0..64 {
        control.editor_note_on(owner, note, 100).unwrap();
    }
    let first = render(&mut audio, &[]);
    assert_eq!(&first.0[..4], &PREFIX_L.map(|value| value * 64.0));
    assert_tail(&render(&mut audio, &[]), 64);
    fill_ordinary_queue(&mut control);
    control.detach_editor_notes(owner);
    assert_tail(&render(&mut audio, &[off(127, 0); 64]), 0);
    assert_eq!(
        control.try_send(ControlEvent::NoteOff { note: 127 }),
        Err(dandrum_trigger_host::runtime::RuntimeError::QueueFull)
    );
    assert_tail(&render(&mut audio, &[]), 0);
    control
        .try_send(ControlEvent::NoteOff { note: 127 })
        .unwrap();
}

#[test]
fn saturated_takeover_and_cleanup_share_release_credit_once_at_host_offset() {
    let fixture = Fixture::new("gated", 128);
    let owner = std::num::NonZeroU64::new(17).unwrap();
    for detach in [false, true] {
        let (mut control, mut audio) = fixture.runtime(64);
        for note in 0..64 {
            control.editor_note_on(owner, note, 100).unwrap();
        }
        let first = render(&mut audio, &[]);
        assert_eq!(&first.0[..4], &PREFIX_L.map(|value| value * 64.0));
        fill_ordinary_queue(&mut control);
        if detach {
            control.detach_editor_notes(owner);
        }
        let host: Vec<_> = (0..64).map(|note| on(note, 5)).collect();
        let takeover = render(&mut audio, &host);
        let before = if detach {
            (0.0, 0.0)
        } else {
            (TAIL.0 * 64.0, TAIL.1 * 64.0)
        };
        assert_eq!(&takeover.0[..5], &[before.0; 5]);
        assert_eq!(&takeover.1[..5], &[before.1; 5]);
        assert_eq!(&takeover.0[5..9], &PREFIX_L.map(|value| value * 64.0));
        assert_eq!(&takeover.1[5..9], &PREFIX_R.map(|value| value * 64.0));
        assert_eq!(&takeover.0[9..], &[TAIL.0 * 64.0; 7]);
        assert_eq!(
            control.try_send(ControlEvent::NoteOff { note: 127 }),
            Err(dandrum_trigger_host::runtime::RuntimeError::QueueFull)
        );
        control.detach_editor_notes(owner);
        assert_tail(&render(&mut audio, &[]), 64);
        let releases: Vec<_> = (0..64).map(|note| off(note, 0)).collect();
        assert_tail(&render(&mut audio, &releases), 0);
    }
}

#[test]
fn all_128_pitch_intents_coalesce_and_cleanup_finishes_within_two_saturated_blocks() {
    let fixture = Fixture::new("gated", 128);
    let (mut control, mut audio) = fixture.runtime(64);
    let owner = std::num::NonZeroU64::new(17).unwrap();
    for note in 0..128 {
        control.editor_note_on(owner, note, 100).unwrap();
        control.editor_note_on(owner, note, 90).unwrap();
    }
    let first = render(&mut audio, &[]);
    assert_eq!(&first.0[..4], &PREFIX_L.map(|value| value * 128.0));
    assert_eq!(&first.1[..4], &PREFIX_R.map(|value| value * 128.0));
    assert_tail(&render(&mut audio, &[]), 128);
    control.detach_editor_notes(owner);
    let partial = render(&mut audio, &[off(127, 0); 64]);
    // The 64 cleanup credits release 64 pitches; host Off127 can release one
    // additional pitch if it was outside that scan. Both outcomes are bounded.
    assert!(partial.0 == [TAIL.0 * 63.0; FRAMES] || partial.0 == [TAIL.0 * 64.0; FRAMES]);
    assert!(partial.1 == [TAIL.1 * 63.0; FRAMES] || partial.1 == [TAIL.1 * 64.0; FRAMES]);
    assert_tail(&render(&mut audio, &[off(127, 0); 64]), 0);
    assert_tail(&render(&mut audio, &[]), 0);
}

#[test]
fn ordinary_and_host_saturation_defer_an_unheard_tap_and_detach_can_still_cancel_it() {
    let fixture = Fixture::new("gated", 4);
    let owner = std::num::NonZeroU64::new(17).unwrap();
    for detach in [false, true] {
        let (mut control, mut audio) = fixture.runtime(64);
        fill_ordinary_queue(&mut control);
        control.editor_note_on(owner, 60, 100).unwrap();
        control.editor_note_off(owner, 60).unwrap();
        assert_tail(&render(&mut audio, &[off(127, 0); 64]), 0);
        if detach {
            control.detach_editor_notes(owner);
        }
        let later = render(&mut audio, &[]);
        if detach {
            assert_tail(&later, 0);
        } else {
            assert_eq!(&later.0[..4], &PREFIX_L);
            assert_eq!(&later.1[..4], &PREFIX_R);
        }
        assert_tail(&render(&mut audio, &[]), 0);
    }
}

#[test]
fn accepting_128_pitch_intents_does_not_promise_more_than_the_prepared_voice_limit() {
    let fixture = Fixture::new("gated", 4);
    let (mut control, mut audio) = fixture.runtime(64);
    let owner = std::num::NonZeroU64::new(17).unwrap();
    for note in 0..128 {
        control.editor_note_on(owner, note, 100).unwrap();
    }
    let first = render(&mut audio, &[]);
    assert_eq!(&first.0[..4], &PREFIX_L.map(|value| value * 4.0));
    assert_eq!(&first.1[..4], &PREFIX_R.map(|value| value * 4.0));
    assert_tail(&render(&mut audio, &[]), 4);
    control.detach_editor_notes(owner);
    assert_tail(&render(&mut audio, &[]), 0);
}

#[test]
fn panic_reset_suppresses_held_and_unheard_editor_intent_until_a_new_press() {
    let fixture = Fixture::new("gated", 4);
    let owner = std::num::NonZeroU64::new(17).unwrap();
    for (heard, released) in [(true, false), (false, false), (false, true)] {
        let (mut control, mut audio) = fixture.runtime(4);
        control.editor_note_on(owner, 60, 100).unwrap();
        if heard {
            let first = render(&mut audio, &[]);
            assert_eq!(&first.0[..4], &PREFIX_L);
        }
        if released {
            control.editor_note_off(owner, 60).unwrap();
        }
        assert!(control.offer(fixture.prepared()).is_ok());
        let reset = allocation_counter::measure(|| {
            assert!(!audio.reset().admission_closed);
        });
        assert_eq!(
            (reset.count_total, reset.count_current, reset.bytes_current),
            (0, 0, 0)
        );
        for _ in 0..3 {
            assert_tail(&render(&mut audio, &[]), 0);
        }
        control.editor_note_on(owner, 60, 100).unwrap();
        let new_press = render(&mut audio, &[]);
        assert_eq!(&new_press.0[..4], &PREFIX_L);
        assert_eq!(&new_press.1[..4], &PREFIX_R);
    }
    let (mut control, mut audio) = fixture.runtime(4);
    let host = render(&mut audio, &[on(60, 0)]);
    assert_eq!(&host.0[..4], &PREFIX_L);
    control.editor_note_on(owner, 60, 100).unwrap();
    audio.reset();
    assert_tail(&render(&mut audio, &[]), 0);
    control.editor_note_on(owner, 60, 100).unwrap();
    let new_press = render(&mut audio, &[]);
    assert_eq!(&new_press.0[..4], &PREFIX_L);
    assert_eq!(&new_press.1[..4], &PREFIX_R);
}

#[test]
fn ordinary_swap_resumes_only_still_desired_pitch_on_the_new_graph() {
    let original = Fixture::new("gated", 4);
    let changed = Fixture::with_pan("gated", 4, 0.5);
    let owner = std::num::NonZeroU64::new(17).unwrap();
    for (heard, release, detach) in [
        (true, false, false),
        (true, true, false),
        (false, true, false),
        (false, false, true),
    ] {
        let (mut control, mut audio) = original.runtime(4);
        control.editor_note_on(owner, 60, 100).unwrap();
        if heard {
            let old = render(&mut audio, &[]);
            assert_eq!(&old.0[..4], &PREFIX_L);
            assert_eq!(&old.1[..4], &PREFIX_R);
        }
        if release {
            control.editor_note_off(owner, 60).unwrap();
        }
        if detach {
            control.detach_editor_notes(owner);
        }
        assert!(control.offer(changed.prepared()).is_ok());
        let swapped = render(&mut audio, &[]);
        if !release && !detach {
            assert_eq!(&swapped.0[..4], &PREFIX_R);
            assert_eq!(&swapped.1[..4], &PREFIX_L);
            let sustained = render(&mut audio, &[]);
            assert_eq!(sustained, ([-0.0625; FRAMES], [-0.125; FRAMES]));
        } else {
            assert_tail(&swapped, 0);
            assert_tail(&render(&mut audio, &[]), 0);
        }
        assert!(control.reclaim().is_some());
    }
}

// Independent PCM oracles: decoded WAV values times velocity/127 and pan -0.5.
// Fractions are reduced directly; expected values are never read from a render.
fn velocity_pcm(velocity: u8) -> ([f32; 4], [f32; 4], (f32, f32)) {
    match velocity {
        32 => (
            [
                8191.0 / 130048.0,
                -16.0 / 127.0,
                24575.0 / 130048.0,
                -8.0 / 127.0,
            ],
            [
                8191.0 / 260096.0,
                -8.0 / 127.0,
                24575.0 / 260096.0,
                -4.0 / 127.0,
            ],
            (-4.0 / 127.0, -2.0 / 127.0),
        ),
        96 => (
            [
                24573.0 / 130048.0,
                -48.0 / 127.0,
                73725.0 / 130048.0,
                -24.0 / 127.0,
            ],
            [
                24573.0 / 260096.0,
                -24.0 / 127.0,
                73725.0 / 260096.0,
                -12.0 / 127.0,
            ],
            (-12.0 / 127.0, -6.0 / 127.0),
        ),
        _ => panic!("only independently calibrated velocities have PCM oracles"),
    }
}

fn assert_pcm(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() <= 1.0e-7,
            "sample {index}: {actual} != {expected}"
        );
    }
}

#[test]
fn velocity_sensitive_child_calibrates_signed_pcm_and_release_from_host_events() {
    let fixture = Fixture::with_voice_velocity("gated", 4, -0.5, true);
    for velocity in [32, 96] {
        let (_, mut audio) = fixture.runtime(4);
        let (left, right, tail) = velocity_pcm(velocity);
        let first = render(
            &mut audio,
            &[TimedEvent {
                frame_offset: 0,
                event: ControlEvent::NoteOn { note: 60, velocity },
            }],
        );
        assert_pcm(&first.0[..4], &left);
        assert_pcm(&first.1[..4], &right);
        assert_pcm(&first.0[4..], &[tail.0; 12]);
        assert_pcm(&first.1[4..], &[tail.1; 12]);
        let held = render(&mut audio, &[]);
        assert_pcm(&held.0, &[tail.0; FRAMES]);
        assert_pcm(&held.1, &[tail.1; FRAMES]);
        let released = render(&mut audio, &[off(60, 7)]);
        assert_pcm(&released.0[..7], &[tail.0; 7]);
        assert_pcm(&released.1[..7], &[tail.1; 7]);
        assert_eq!(&released.0[7..], &[0.0; 9]);
        assert_eq!(&released.1[7..], &[0.0; 9]);
        assert_tail(&render(&mut audio, &[]), 0);
    }
}

#[test]
fn editor_onset_keeps_latest_coalesced_velocity_even_after_an_unheard_release() {
    let fixture = Fixture::with_voice_velocity("gated", 4, -0.5, true);
    let owner = std::num::NonZeroU64::new(17).unwrap();
    for (first_velocity, latest_velocity) in [(32, 96), (96, 32)] {
        for released_before_audio in [false, true] {
            let (mut control, mut audio) = fixture.runtime(4);
            control.editor_note_on(owner, 60, first_velocity).unwrap();
            control.editor_note_on(owner, 60, latest_velocity).unwrap();
            if released_before_audio {
                control.editor_note_off(owner, 60).unwrap();
            }
            let (left, right, tail) = velocity_pcm(latest_velocity);
            let first = render(&mut audio, &[]);
            assert_pcm(&first.0[..4], &left);
            assert_pcm(&first.1[..4], &right);
            assert_pcm(&first.0[4..], &[tail.0; 12]);
            assert_pcm(&first.1[4..], &[tail.1; 12]);
            if !released_before_audio {
                let held = render(&mut audio, &[]);
                assert_pcm(&held.0, &[tail.0; FRAMES]);
                assert_pcm(&held.1, &[tail.1; FRAMES]);
                control.editor_note_off(owner, 60).unwrap();
            }
            // A released unheard tap gets one onset block; it neither replays
            // nor becomes a held note on later callbacks.
            for _ in 0..3 {
                assert_tail(&render(&mut audio, &[]), 0);
            }
        }
    }
}
