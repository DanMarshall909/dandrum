use dandrum_engine::graph_processor::render_kernel_offline_named;
use dandrum_engine::preparation::{HostBuses, PreparationContext, PreparedKernelInstrument};
use dandrum_engine::render_settings::{RealtimeRenderSettings, SampleRate};
use dandrum_engine::script::ScriptEvent;
use dandrum_engine::{PreparedSamplerAssets, core::TimedInputEvent};
use dandrum_instrument_session::{
    ActivationHost, ActivationLease, InstrumentSession, MappingAdmission, MappingRequest,
    Publication, RequestId, SessionSnapshot, SetZoneKeyRange,
};

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

// Session unit tests accept publication; concrete runtime ownership is tested
// separately through TriggerHost's capacity-one handoff and callback sentinel.
struct AcceptPublication;
impl ActivationHost for AcceptPublication {
    type Error = &'static str;
    type Lease<'a> = Self;
    fn activate<'a>(
        &'a mut self,
        _: &PreparedKernelInstrument,
        _: &SessionSnapshot,
    ) -> Result<Self::Lease<'a>, Self::Error> {
        Ok(Self)
    }
}
impl ActivationLease for AcceptPublication {
    fn publish(self, _: Publication<'_>) {}
}

fn assert_audio(session: &InstrumentSession, hz: f64, note: u8, sounding: bool) {
    assert_eq!(
        session
            .prepared()
            .compiled_patch()
            .render_settings()
            .sample_rate_hz
            .hz(),
        hz
    );
    let output = render_kernel_offline_named(
        session.prepared(),
        vec![TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note,
                velocity: 100,
            },
        )],
        &PreparedSamplerAssets::empty(),
    )
    .unwrap();
    for channel in &output[0].1 {
        assert_eq!(channel[0], if sounding { -0.75 } else { 0.0 });
        for (frame, &value) in channel.iter().enumerate() {
            let position = frame as f64 * 48_000.0 / hz;
            let expected = if sounding && position < 1_024.0 {
                (-0.75 + position.min(1_023.0) / 4_096.0) as f32
            } else {
                0.0
            };
            assert!(
                (value - expected).abs() <= 0.000_000_2,
                "{hz}, note {note}, frame {frame}: {value} != {expected}"
            );
        }
        if sounding {
            let frames = (1_024.0 * hz / 48_000.0).ceil() as usize;
            assert_ne!(channel[frames - 1], 0.0);
            assert_eq!(channel[frames], 0.0);
        }
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
    fn session(&self, hz: f64) -> InstrumentSession {
        InstrumentSession::new_realtime_for_planar_host(
            PATCH,
            PreparationContext::new_realtime(self.0.path(), SampleRate::new(hz).unwrap()),
            settings(hz),
            HostBuses::new().with_output("master", 2),
        )
        .unwrap()
    }
}
fn settings(hz: f64) -> RealtimeRenderSettings {
    RealtimeRenderSettings {
        sample_rate_hz: SampleRate::new(hz).unwrap(),
        block_size_frames: 64,
        duration_frames: 1_026,
    }
}

#[test]
fn detached_mapping_jobs_and_undo_keep_the_exact_session_clock() {
    let fixture = Fixture::new();
    for hz in [1_234.567_8, 48_000.5, 48_000.000_001] {
        let mut session = fixture.session(hz);
        assert_audio(&session, hz, 36, true);
        let (editor, before) = session.attach().unwrap();
        let request = MappingRequest {
            instance: before.instance.clone(),
            request_id: RequestId(1),
            expected_revision: before.revision,
            intent: SetZoneKeyRange {
                map_id: "kit".into(),
                zone_id: "kick".into(),
                key_range: [60, 60],
            },
        };
        let MappingAdmission::Admitted(job) = session.admit_mapping(&editor, request).unwrap()
        else {
            panic!("mapping admitted")
        };
        session.detach(&editor).unwrap();
        let completion = std::thread::spawn(move || job.prepare()).join().unwrap();
        let outcome = session
            .complete_mapping(completion, &mut AcceptPublication)
            .unwrap();
        let accepted = outcome.result.as_ref().unwrap();
        assert_eq!(accepted.undo_len, 1);
        assert_audio(&session, hz, 36, false);
        assert_audio(&session, hz, 60, true);
        let (fresh, recovered) = session.attach().unwrap();
        assert_eq!(&recovered, accepted);
        let MappingAdmission::Admitted(undo) = session
            .admit_undo(&fresh, RequestId(2), recovered.revision)
            .unwrap()
        else {
            panic!("undo admitted")
        };
        assert!(
            session
                .complete_mapping(undo.prepare(), &mut AcceptPublication)
                .unwrap()
                .result
                .is_ok()
        );
        assert_audio(&session, hz, 36, true);
        assert_audio(&session, hz, 60, false);
    }
}

struct RejectPublication;
impl ActivationHost for RejectPublication {
    type Error = &'static str;
    type Lease<'a> = AcceptPublication;
    fn activate<'a>(
        &'a mut self,
        _: &PreparedKernelInstrument,
        _: &SessionSnapshot,
    ) -> Result<Self::Lease<'a>, Self::Error> {
        Err("candidate rejected")
    }
}

#[test]
fn precise_reconfiguration_preserves_history_rollback_and_another_session() {
    use dandrum_instrument_session::CommitError;
    let fixture = Fixture::new();
    let mut session = fixture.session(48_000.0);
    let other = fixture.session(44_100.0);
    let other_before = other.owner_snapshot();
    let (editor, _) = session.attach().unwrap();
    let candidate = session
        .prepare_mapping(
            &editor,
            SetZoneKeyRange {
                map_id: "kit".into(),
                zone_id: "kick".into(),
                key_range: [60, 60],
            },
        )
        .unwrap();
    session
        .commit_mapping(candidate, &mut AcceptPublication)
        .unwrap();
    let mut accepted_hz = 48_000.0;
    for hz in [1_234.567_8, 48_000.5, 48_000.000_001] {
        let before = session.owner_snapshot();
        let candidate = session
            .prepare_realtime_render_configuration_owned(
                settings(hz),
                HostBuses::new().with_output("master", 2),
            )
            .unwrap();
        assert_eq!(
            session.commit_render_configuration(candidate, &mut RejectPublication),
            Err(CommitError::Activation("candidate rejected")),
        );
        assert_eq!(session.owner_snapshot(), before);
        assert_audio(&session, accepted_hz, 60, true);
        assert_audio(&session, accepted_hz, 36, false);
        let candidate = session
            .prepare_realtime_render_configuration_owned(
                settings(hz),
                HostBuses::new().with_output("master", 2),
            )
            .unwrap();
        assert_eq!(
            session
                .commit_render_configuration(candidate, &mut AcceptPublication)
                .unwrap(),
            before
        );
        assert_eq!(session.owner_snapshot(), before);
        assert_audio(&session, hz, 60, true);
        assert_audio(&session, hz, 36, false);
        let undone = session.undo(&editor, &mut AcceptPublication).unwrap();
        assert_eq!((undone.undo_len, undone.redo_len), (0, 1));
        assert_audio(&session, hz, 36, true);
        assert_audio(&session, hz, 60, false);
        let redone = session.redo(&editor, &mut AcceptPublication).unwrap();
        assert_eq!((redone.undo_len, redone.redo_len), (1, 0));
        assert_audio(&session, hz, 60, true);
        assert_eq!(other.owner_snapshot(), other_before);
        assert_audio(&other, 44_100.0, 36, true);
        assert_audio(&other, 44_100.0, 60, false);
        accepted_hz = hz;
    }
}

#[test]
fn invalid_legacy_clock_and_busy_precise_preparation_preserve_the_session() {
    use dandrum_engine::patch::RenderSettings;
    use dandrum_instrument_session::SessionError;
    let fixture = Fixture::new();
    let zero = || RenderSettings {
        sample_rate_hz: 0,
        block_size_frames: 64,
        duration_frames: 1_026,
    };
    let error = InstrumentSession::new_for_planar_host(
        PATCH,
        PreparationContext::new(fixture.0.path(), 0),
        zero(),
        HostBuses::new().with_output("master", 2),
    )
    .err()
    .expect("zero cannot construct a session");
    assert!(
        matches!(error, SessionError::PreparationFailed(message) if message.contains("sample rate"))
    );
    let mut session = fixture.session(48_000.5);
    let (editor, before) = session.attach().unwrap();
    let error = session
        .prepare_render_configuration_owned(zero(), HostBuses::new().with_output("master", 2))
        .err()
        .expect("zero cannot construct a replacement");
    assert!(
        matches!(error, SessionError::PreparationFailed(message) if message.contains("sample rate"))
    );
    assert_eq!(session.owner_snapshot(), before);
    assert_audio(&session, 48_000.5, 36, true);
    let MappingAdmission::Admitted(job) = session
        .admit_mapping(
            &editor,
            MappingRequest {
                instance: before.instance.clone(),
                request_id: RequestId(1),
                expected_revision: before.revision,
                intent: SetZoneKeyRange {
                    map_id: "kit".into(),
                    zone_id: "kick".into(),
                    key_range: [60, 60],
                },
            },
        )
        .unwrap()
    else {
        panic!("mapping admitted")
    };
    assert!(
        matches!(
            session.prepare_render_configuration_owned(
                zero(),
                HostBuses::new().with_output("master", 2)
            ),
            Err(SessionError::Busy)
        ),
        "legacy preparation keeps its existing busy precedence"
    );
    assert!(matches!(
        session.prepare_realtime_render_configuration_owned(
            settings(1_234.567_8),
            HostBuses::new().with_output("master", 2)
        ),
        Err(SessionError::Busy)
    ));
    assert_eq!(session.owner_snapshot(), before);
    assert_audio(&session, 48_000.5, 36, true);
    assert!(
        session
            .complete_mapping(job.prepare(), &mut AcceptPublication)
            .unwrap()
            .result
            .is_ok()
    );
    assert_audio(&session, 48_000.5, 60, true);
}

#[test]
fn admitted_selected_content_survives_real_session_reconfiguration_after_source_loss() {
    use dandrum_engine::sample_snapshot::SampleSnapshot;
    use std::sync::Arc;
    let fixture = Fixture::new();
    let root = fixture.0.path().to_owned();
    let snapshot =
        Arc::new(SampleSnapshot::decode(std::fs::read(root.join("hit.wav")).unwrap()).unwrap());
    let yaml = PATCH.replace(
        "path: hit.wav",
        &format!("path: {}", snapshot.resource_id()),
    );
    let mut session = InstrumentSession::new_realtime_for_planar_host(
        &yaml,
        PreparationContext::new_realtime(&root, SampleRate::new(48_000.0).unwrap())
            .with_selected_samples([snapshot]),
        settings(48_000.0),
        HostBuses::new().with_output("master", 2),
    )
    .unwrap();
    fixture.0.close().unwrap();
    assert!(!root.exists(), "original WAV and authoring root are gone");
    let before = session.owner_snapshot();
    assert_audio(&session, 48_000.0, 36, true);
    for hz in [44_100.5, 1_234.567_8] {
        let candidate = session
            .prepare_realtime_render_configuration_owned(
                settings(hz),
                HostBuses::new().with_output("master", 2),
            )
            .expect("session reconfiguration retains admitted selected authority");
        let accepted = session
            .commit_render_configuration(candidate, &mut AcceptPublication)
            .unwrap();
        assert_eq!(
            accepted, before,
            "render configuration preserves authoring state"
        );
        assert_eq!(session.owner_snapshot(), before);
        // Existing independent linear PCM/time oracle checks every rendered frame,
        // both channels, precise clock, note mapping, and the sample's exact end.
        assert_audio(&session, hz, 36, true);
        assert_audio(&session, hz, 60, false);
    }
}
