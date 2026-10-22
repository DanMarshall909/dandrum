use dandrum_engine::preparation::{HostBuses, PreparationContext};
use dandrum_engine::render_settings::{RealtimeRenderSettings, SampleRate};
use dandrum_instrument_session::InstrumentSession;
use dandrum_trigger_host::TriggerHostConfiguration;
use dandrum_trigger_host::runtime::{
    ControlEvent, PreparedRuntime, RuntimeAudio, RuntimeError, TimedEvent, runtime_handoff,
};

const BLOCK: usize = 64;
const TOTAL: usize = 1_026;
type Wave = [[f32; TOTAL]; 2];
const PATCH: &str = r#"
assets:
  sample_sources:
    - id: hit
      path: hit.wav
      regions: [{ id: body, start_frame: 0, end_frame: 1024 }]
  sample_maps:
    - id: kit
      zones: [{ id: kick, region: hit.body, key_range: [36, 36], velocity_range: [1, 127] }]
ports:
  - { name: master, direction: output, signal: audio, channels: 2, maps_from: player.audio }
modules:
  - { id: midi, type: midi_input }
  - { id: player, type: sample_map_player, static: { sample_map: kit, max_voices: 1, channels: 2 } }
connections:
  - { from: midi.events, to: player.note }
"#;

fn configuration(hz: f64) -> TriggerHostConfiguration {
    TriggerHostConfiguration {
        sample_rate_hz: SampleRate::new(hz).unwrap(),
        max_block_frames: BLOCK as u32,
    }
}
struct Fixture(tempfile::TempDir);
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let values: Vec<f32> = (0..1_024)
            .map(|frame| -0.75 + frame as f32 / 4_096.0)
            .collect();
        dandrum_engine::wav::write_wav_stereo_i16(
            std::fs::File::create(directory.path().join("hit.wav")).unwrap(),
            48_000,
            &values,
            &values,
        )
        .unwrap();
        Self(directory)
    }
    fn session(&self, hz: f64, note: u8) -> InstrumentSession {
        let rate = SampleRate::new(hz).unwrap();
        InstrumentSession::new_realtime_for_planar_host(
            &PATCH.replace(
                "key_range: [36, 36]",
                &format!("key_range: [{note}, {note}]"),
            ),
            PreparationContext::new_realtime(self.0.path(), rate),
            RealtimeRenderSettings {
                sample_rate_hz: rate,
                block_size_frames: BLOCK as u32,
                duration_frames: TOTAL as u64,
            },
            HostBuses::new().with_output("master", 2),
        )
        .unwrap()
    }
    fn runtime(&self, hz: f64, note: u8) -> PreparedRuntime {
        PreparedRuntime::new(self.session(hz, note).prepared(), configuration(hz)).unwrap()
    }
}
fn callback(
    audio: &mut RuntimeAudio,
    left: &mut [f32],
    right: &mut [f32],
    note: Option<u8>,
    swapped: bool,
) {
    let event = note.map(|note| TimedEvent {
        frame_offset: 0,
        event: ControlEvent::NoteOn {
            note,
            velocity: 100,
        },
    });
    let audit = allocation_counter::measure(|| {
        assert_eq!(
            audio
                .process_events(left, right, event.as_slice())
                .unwrap()
                .swapped,
            swapped
        );
    });
    assert_eq!(
        audit,
        allocation_counter::AllocationInfo::default(),
        "callback neither allocates nor deallocates"
    );
}
fn full_on_worker(mut audio: RuntimeAudio, note: u8, swapped: bool) -> (RuntimeAudio, Wave) {
    let control = std::thread::current().id();
    std::thread::spawn(move || {
        assert_ne!(std::thread::current().id(), control);
        let mut output = [[0.0; TOTAL]; 2];
        let [left, right] = &mut output;
        for start in (0..TOTAL).step_by(BLOCK) {
            let end = (start + BLOCK).min(TOTAL);
            callback(
                &mut audio,
                &mut left[start..end],
                &mut right[start..end],
                (start == 0).then_some(note),
                start == 0 && swapped,
            );
        }
        (audio, output)
    })
    .join()
    .unwrap()
}
fn block_on_worker(
    mut audio: RuntimeAudio,
    note: Option<u8>,
    swapped: bool,
    reset: bool,
    frames: usize,
) -> (RuntimeAudio, [[f32; BLOCK]; 2]) {
    std::thread::spawn(move || {
        if reset {
            let audit = allocation_counter::measure(|| {
                assert!(!audio.reset().admission_closed);
            });
            assert_eq!(audit, allocation_counter::AllocationInfo::default());
        }
        let mut output = [[0.0; BLOCK]; 2];
        let [left, right] = &mut output;
        callback(
            &mut audio,
            &mut left[..frames],
            &mut right[..frames],
            note,
            swapped,
        );
        (audio, output)
    })
    .join()
    .unwrap()
}
fn expected(frame: usize, hz: f64) -> f32 {
    let position = frame as f64 * 48_000.0 / hz;
    if position < 1_024.0 {
        (-0.75 + position.min(1_023.0) / 4_096.0) as f32
    } else {
        0.0
    }
}
fn assert_wave(wave: &Wave, hz: f64) {
    for plane in wave {
        assert_eq!(plane[0], -0.75);
        for (frame, &actual) in plane.iter().enumerate() {
            assert!(
                (actual - expected(frame, hz)).abs() <= 0.000_000_2,
                "{hz}Hz frame{frame}: {actual}"
            );
        }
        let sounding = (1_024.0 * hz / 48_000.0).ceil() as usize;
        assert_ne!(plane[sounding - 1], 0.0);
        assert_eq!(plane[sounding], 0.0);
    }
}

#[test]
fn fractional_runtime_swaps_reclaim_on_control_and_preserve_reset_and_another_instance() {
    let fixture = Fixture::new();
    let control_thread = std::thread::current().id();
    for hz in [1_234.567_8, 48_000.5, 48_000.000_001] {
        let (mut control, audio) = runtime_handoff(fixture.runtime(hz, 36), 4).unwrap();
        let (other_control, other_audio) =
            runtime_handoff(fixture.runtime(44_100.0, 36), 4).unwrap();
        let (other_audio, prefix) = block_on_worker(other_audio, Some(36), false, false, BLOCK);
        assert_eq!((prefix[0][0], prefix[1][0]), (-0.75, -0.75));
        let (audio, before) = full_on_worker(audio, 36, false);
        assert_wave(&before, hz);
        assert!(control.offer(fixture.runtime(hz, 60)).is_ok());
        let rejected = control
            .offer(fixture.runtime(hz, 72))
            .expect_err("one pending replacement");
        assert!(
            control.reclaim().is_none(),
            "no block boundary has consumed it"
        );
        let (audio, after) = full_on_worker(audio, 60, true);
        assert_wave(&after, hz);
        let rejected = control
            .offer(rejected)
            .expect_err("retirement still occupies capacity");
        let audit = allocation_counter::measure(|| {
            assert_eq!(std::thread::current().id(), control_thread);
            drop(
                control
                    .reclaim()
                    .expect("retired graph returned from the render thread"),
            );
        });
        assert_eq!(audit.count_total, 0);
        assert!(
            audit.count_current < 0 && audit.bytes_current < 0,
            "retired graph destroyed on control"
        );
        assert!(
            control.offer(rejected).is_ok(),
            "reclamation restores activation capacity"
        );
        let (audio, active) = block_on_worker(audio, Some(72), true, false, 1);
        assert_eq!((active[0][0], active[1][0]), (-0.75, -0.75));
        drop(control.reclaim().unwrap());
        let (audio, reset) = block_on_worker(audio, None, false, true, BLOCK);
        assert_eq!(reset, [[0.0; BLOCK]; 2], "reset clears a sounding voice");
        let (audio, restarted) = full_on_worker(audio, 72, false);
        assert_wave(&restarted, hz);
        let (other_audio, continued) = block_on_worker(other_audio, None, false, false, BLOCK);
        for plane in continued {
            for (frame, actual) in plane.into_iter().enumerate() {
                assert!(
                    (actual - expected(BLOCK + frame, 44_100.0)).abs() <= 0.000_000_2,
                    "other instance retains its cursor"
                );
            }
        }
        drop((audio, control, other_audio, other_control));
    }
}

#[test]
fn runtime_rejects_a_different_clock_even_below_f32_precision() {
    let fixture = Fixture::new();
    let session = fixture.session(48_000.000_001, 36);
    assert_eq!(48_000.000_001_f64 as f32, 48_000.0_f32);
    assert!(matches!(
        PreparedRuntime::new(session.prepared(), configuration(48_000.0)),
        Err(RuntimeError::InvalidConfiguration)
    ));
    assert!(PreparedRuntime::new(session.prepared(), configuration(48_000.000_001)).is_ok());
}
