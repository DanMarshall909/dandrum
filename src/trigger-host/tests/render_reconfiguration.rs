use dandrum_trigger_host::parameters::{
    AutomationEvent, ParameterController, ParameterError, ParameterReply, SessionJobProgress,
};
use dandrum_trigger_host::protocol::{Admission, AdmittedRequest};
use dandrum_trigger_host::runtime::{ControlEvent, RuntimeAudio, RuntimeError, TimedEvent};
use dandrum_trigger_host::{
    ADVANCED_DRUM_KIT_DOCUMENT, ADVANCED_DRUM_KIT_SAMPLE, TriggerHostConfiguration,
};
use serde_json::{Value, json};
use std::{num::NonZeroU64, path::Path};

fn configuration(rate: u32, frames: u32) -> TriggerHostConfiguration {
    TriggerHostConfiguration {
        sample_rate_hz: rate,
        max_block_frames: frames,
    }
}
struct Fixture(tempfile::TempDir);
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(directory.path().join("instrument/assets")).unwrap();
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
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
            std::fs::copy(repository.join(source), directory.path().join(destination)).unwrap();
        }
        Self(directory)
    }
    fn open(
        &self,
        config: TriggerHostConfiguration,
        events: usize,
    ) -> (ParameterController, RuntimeAudio) {
        ParameterController::open(
            self.0.path(),
            config,
            NonZeroU64::new(1).unwrap(),
            events,
            256,
        )
        .unwrap()
    }
}
struct Editor {
    instance: String,
    attachment: String,
    request: u64,
}
impl Editor {
    fn attach(host: &mut ParameterController) -> Self {
        let connection = host.attach_editor().unwrap();
        Self {
            instance: connection.instance_id,
            attachment: connection.attachment_id,
            request: 0,
        }
    }
    fn command(
        &mut self,
        host: &mut ParameterController,
        method: &str,
        args: Value,
    ) -> AdmittedRequest {
        self.request += 1;
        let message = json!({"version":1,"attachmentId":self.attachment,"requestId":self.request.to_string(),"method":method,"args":args}).to_string();
        match host.admit(&message).unwrap() {
            Admission::New(request) => request,
            _ => panic!("new public request required"),
        }
    }
    fn set(&mut self, host: &mut ParameterController, id: &str, normalized: f64) -> ParameterReply {
        let generation = host.parameter_state().generation;
        let request = self.command(host, "setParameter", json!([id, normalized, generation]));
        host.apply(&request).unwrap()
    }
    fn map(&mut self, host: &mut ParameterController, note: u8) -> u64 {
        let id = host.session().next_request_id().unwrap().0;
        let revision = host.session().owner_snapshot().revision;
        let request = self.command(
            host,
            "setInstrumentZoneKeyRange",
            json!([
                self.instance,
                self.attachment,
                id.to_string(),
                revision.to_string(),
                "kit",
                "kick",
                note,
                note
            ]),
        );
        host.admit_session_command(&request).unwrap();
        id
    }
}
fn ready(host: &mut ParameterController) {
    let completion = host.take_session_preparation().unwrap().prepare();
    host.submit_session_completion(completion).unwrap();
}
fn automation(host: &mut ParameterController) -> Vec<AutomationEvent> {
    let mut events = Vec::new();
    while let Some(event) = host.peek_automation() {
        events.push(event);
        assert!(host.acknowledge_automation());
    }
    events
}
fn on(note: u8, frame_offset: u32) -> TimedEvent {
    TimedEvent {
        frame_offset,
        event: ControlEvent::NoteOn {
            note,
            velocity: 100,
        },
    }
}
fn render<const N: usize>(audio: &mut RuntimeAudio, events: &[TimedEvent]) -> ([f32; N], [f32; N]) {
    let (mut left, mut right) = ([1.0; N], [1.0; N]);
    let audit = allocation_counter::measure(|| {
        assert!(
            !audio
                .process_events(&mut left, &mut right, events)
                .unwrap()
                .admission_closed
        );
    });
    assert_eq!(
        audit,
        allocation_counter::AllocationInfo::default(),
        "actual callbacks allocate and destroy nothing"
    );
    (left, right)
}

#[test]
fn paired_controller_service_preserves_nonzero_restore_epoch_and_ui_settlement_without_servicing_jobs()
 {
    let fixture = Fixture::new();
    let (mut host, mut audio) = fixture.open(configuration(48_000, 16), 4);
    let mut editor = Editor::attach(&mut host);
    let mut saved = Vec::new();
    host.save_state(&mut saved).unwrap();
    host.load_state(&mut saved.as_slice()).unwrap();
    assert!(host.service_quiescent(&mut audio).unwrap().adopted);
    assert_eq!(
        editor.set(&mut host, "drums.level", 0.125),
        ParameterReply::Accepted {
            generation: 2,
            sequence: 1
        }
    );
    automation(&mut host);
    let first = render::<16>(
        &mut audio,
        &[
            TimedEvent {
                frame_offset: 3,
                event: ControlEvent::Parameter {
                    index: 3,
                    value: 0.5,
                },
            },
            on(36, 3),
        ],
    );
    assert_eq!((first.0[3], first.1[3]), (-0.125, -0.25));
    assert_eq!(
        editor.set(&mut host, "drums.level", 0.375),
        ParameterReply::Accepted {
            generation: 2,
            sequence: 2
        }
    );
    automation(&mut host);
    editor.map(&mut host, 60);
    ready(&mut host);
    let prior = host.session().owner_snapshot();
    let values = host.parameter_state();
    assert!(!host.service_quiescent(&mut audio).unwrap().adopted);
    assert_eq!(
        host.session().owner_snapshot(),
        prior,
        "runtime service cannot publish a ready structural job"
    );
    assert_eq!(
        host.parameter_state(),
        values,
        "runtime service cannot settle queued UI work"
    );
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    let accepted = host.session().owner_snapshot();
    assert_eq!(
        accepted.revision, 2,
        "accepted revision differs from the real restore epoch1"
    );
    let before = host.parameter_state();
    assert_eq!(
        (before.generation, before.sequence, before.reset_cancelled),
        (3, 1, false)
    );
    assert_eq!(before.parameters[2].value, 0.125);
    assert!(host.service_quiescent(&mut audio).unwrap().adopted);
    assert_eq!(host.parameter_state(), before);
    assert_eq!(host.session().owner_snapshot(), accepted);
    let prior_ui = host.take_feedback().unwrap();
    assert_eq!(
        (
            prior_ui.sequence,
            prior_ui.frame_offset,
            prior_ui.parameters[0].value
        ),
        (1, 0, 0.125)
    );
    let prior_host = host.take_feedback().unwrap();
    assert_eq!(
        (
            prior_host.sequence,
            prior_host.frame_offset,
            prior_host.parameters[0].value
        ),
        (1, 3, 0.75)
    );
    assert!(host.take_feedback().is_none());
    let changed = render::<16>(&mut audio, &[on(60, 3)]);
    assert_eq!(&changed.0[..3], &[0.0; 3]);
    assert_eq!(&changed.1[..3], &[0.0; 3]);
    assert_eq!((changed.0[3], changed.1[3]), (-0.375, -0.75));
    let settled = host.parameter_state();
    assert_eq!(
        (
            settled.generation,
            settled.sequence,
            settled.parameters[2].value
        ),
        (3, 2, 0.375)
    );
    assert_eq!(host.take_feedback().unwrap().sequence, 2);
    assert_eq!(
        editor.set(&mut host, "drums.level", 0.25),
        ParameterReply::Accepted {
            generation: 3,
            sequence: 3
        }
    );
    let (mut foreign, mut foreign_audio) = fixture.open(configuration(48_000, 16), 4);
    assert_eq!(
        host.service_quiescent(&mut foreign_audio),
        Err(ParameterError::Runtime(RuntimeError::ForeignEndpoints))
    );
    assert_eq!(
        foreign.service_quiescent(&mut audio),
        Err(ParameterError::Runtime(RuntimeError::ForeignEndpoints))
    );
    assert_eq!(host.session().owner_snapshot(), accepted);
}

#[test]
fn foreign_identity_and_invalid_bounds_precede_busy_without_consuming_structural_work() {
    let fixture = Fixture::new();
    let (mut host, mut audio) = fixture.open(configuration(48_000, 16), 4);
    let (mut other, mut other_audio) = fixture.open(configuration(48_000, 16), 4);
    assert_eq!(
        render::<2>(&mut other_audio, &[on(36, 0)]).0,
        [-0.5, -358.0 / 32768.0]
    );
    let mut editor = Editor::attach(&mut host);
    editor.map(&mut host, 60);
    let before = host.session().owner_snapshot();
    for invalid in [
        configuration(0, 16),
        configuration(48_000, 0),
        configuration(48_000, 16_385),
    ] {
        assert_eq!(
            host.reconfigure_quiescent(&mut other_audio, invalid),
            Err(ParameterError::Runtime(RuntimeError::ForeignEndpoints))
        );
        assert_eq!(
            host.reconfigure_quiescent(&mut audio, invalid),
            Err(ParameterError::Runtime(RuntimeError::InvalidConfiguration))
        );
    }
    assert_eq!(
        other.reconfigure_quiescent(&mut audio, configuration(44_100, 32)),
        Err(ParameterError::Runtime(RuntimeError::ForeignEndpoints))
    );
    assert_eq!(host.session().owner_snapshot(), before);
    ready(&mut host);
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    assert!(host.service_quiescent(&mut audio).unwrap().adopted);
    let rendered = render::<16>(&mut audio, &[on(60, 3)]);
    assert_eq!((rendered.0[3], rendered.1[3]), (-0.5, -0.5));
    let untouched = render::<2>(&mut other_audio, &[]);
    assert_eq!(untouched.0, [1554.0 / 32768.0, 4137.0 / 32768.0]);
    assert_eq!(untouched.0, untouched.1);
}

#[test]
fn structural_and_runtime_ownership_busy_requires_separate_service_even_for_equal_configuration() {
    let fixture = Fixture::new();
    let config = configuration(48_000, 16);
    let (mut host, mut audio) = fixture.open(config, 4);
    let mut editor = Editor::attach(&mut host);
    editor.map(&mut host, 60);
    let before = host.session().owner_snapshot();
    let values = host.parameter_state();
    assert_eq!(
        host.reconfigure_quiescent(&mut audio, config),
        Err(ParameterError::Runtime(RuntimeError::Busy))
    );
    let preparation = host
        .take_session_preparation()
        .expect("queued job survived Busy");
    assert_eq!(
        host.reconfigure_quiescent(&mut audio, config),
        Err(ParameterError::Runtime(RuntimeError::Busy))
    );
    host.submit_session_completion(preparation.prepare())
        .unwrap();
    assert_eq!(
        host.reconfigure_quiescent(&mut audio, config),
        Err(ParameterError::Runtime(RuntimeError::Busy))
    );
    assert_eq!(host.session().owner_snapshot(), before);
    assert_eq!(host.parameter_state(), values);
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    let accepted = host.session().owner_snapshot();
    assert_eq!(
        host.reconfigure_quiescent(&mut audio, config),
        Err(ParameterError::Runtime(RuntimeError::Busy)),
        "accepted runtime remains pending"
    );
    let rendered = render::<16>(&mut audio, &[on(60, 3)]);
    assert_eq!((rendered.0[3], rendered.1[3]), (-0.5, -0.5));
    assert_eq!(
        host.reconfigure_quiescent(&mut audio, config),
        Err(ParameterError::Runtime(RuntimeError::Busy)),
        "real callback retirement remains owned by control"
    );
    let report = host.service_quiescent(&mut audio).unwrap();
    assert_eq!((report.adopted, report.reclaimed_runtimes), (false, 1));
    assert_eq!(
        host.reconfigure_quiescent(&mut audio, config).unwrap(),
        dandrum_trigger_host::parameters::ReconfigurationReport::Unchanged
    );
    assert_eq!(host.session().owner_snapshot(), accepted);
}

#[test]
fn equal_configuration_continues_real_dsp_without_resource_io_allocation_or_destruction() {
    let fixture = Fixture::new();
    let config = configuration(48_000, 16);
    let (mut host, mut audio) = fixture.open(config, 4);
    let before = host.session().owner_snapshot();
    let first = render::<2>(&mut audio, &[on(36, 0)]);
    assert_eq!(first.0, [-0.5, -358.0 / 32768.0]);
    std::fs::remove_file(fixture.0.path().join(ADVANCED_DRUM_KIT_SAMPLE)).unwrap();
    let audit = allocation_counter::measure(|| {
        assert_eq!(
            host.reconfigure_quiescent(&mut audio, config).unwrap(),
            dandrum_trigger_host::parameters::ReconfigurationReport::Unchanged
        );
    });
    assert_eq!(audit, allocation_counter::AllocationInfo::default());
    assert_eq!(host.session().owner_snapshot(), before);
    let next = render::<2>(&mut audio, &[]);
    assert_eq!(next.0, [1554.0 / 32768.0, 4137.0 / 32768.0]);
    assert_eq!(next.0, next.1);
}

fn finish(host: &mut ParameterController, audio: &mut RuntimeAudio) {
    ready(host);
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    assert!(host.service_quiescent(audio).unwrap().adopted);
}
impl Editor {
    fn history(&mut self, host: &mut ParameterController, method: &str) -> u64 {
        let id = host.session().next_request_id().unwrap().0;
        let revision = host.session().owner_snapshot().revision;
        let request = self.command(
            host,
            method,
            json!([
                self.instance,
                self.attachment,
                id.to_string(),
                revision.to_string()
            ]),
        );
        host.admit_session_command(&request).unwrap();
        id
    }
    fn status(&mut self, host: &mut ParameterController, id: u64) -> Value {
        let request = self.command(
            host,
            "getInstrumentSessionJobStatus",
            json!([id.to_string()]),
        );
        let reply: Value =
            serde_json::from_str(&host.session_job_status(&request).unwrap()).unwrap();
        assert_eq!(reply["status"], "accepted");
        reply["result"].clone()
    }
}
fn assert_fractional_sample(output: &([f32; 32], [f32; 32]), left_gain: f32, right_gain: f32) {
    // Independently decoded PCM16 and positions 0,160/147,320/147 at 44.1 kHz.
    let samples = [
        -0.5,
        -27770.0 / (147.0 * 32768.0),
        295596.0 / (147.0 * 32768.0),
    ];
    for (plane, gain) in [(&output.0, left_gain), (&output.1, right_gain)] {
        assert_eq!(&plane[..3], &[0.0; 3]);
        for (actual, expected) in plane[3..6].iter().zip(samples) {
            assert!(
                (actual - expected * gain).abs() < 1e-6,
                "signed fractional PCM {actual} != {}",
                expected * gain
            );
        }
    }
}

#[test]
fn changed_configuration_publishes_session_and_runtime_together_preserving_history_epoch_and_automation()
 {
    use dandrum_trigger_host::parameters::ReconfigurationReport;
    let fixture = Fixture::new();
    let (mut host, mut audio) = fixture.open(configuration(48_000, 16), 4);
    let mut editor = Editor::attach(&mut host);
    let mut saved = Vec::new();
    host.save_state(&mut saved).unwrap();
    host.load_state(&mut saved.as_slice()).unwrap();
    host.service_quiescent(&mut audio).unwrap();
    let first_job = editor.map(&mut host, 60);
    finish(&mut host, &mut audio);
    editor.map(&mut host, 61);
    finish(&mut host, &mut audio);
    editor.history(&mut host, "undoInstrumentSession");
    finish(&mut host, &mut audio);
    let accepted = host.session().owner_snapshot();
    assert_eq!(
        (accepted.revision, accepted.undo_len, accepted.redo_len),
        (4, 1, 1)
    );
    let outcome = editor.status(&mut host, first_job);
    let descriptors = host.parameters().to_vec();
    let next_native = host.session().next_request_id().unwrap();
    assert_eq!(
        editor.set(&mut host, "drums.level", 0.125),
        ParameterReply::Accepted {
            generation: 5,
            sequence: 1
        }
    );
    automation(&mut host);
    let initial = render::<5>(
        &mut audio,
        &[
            TimedEvent {
                frame_offset: 3,
                event: ControlEvent::Parameter {
                    index: 3,
                    value: 0.5,
                },
            },
            on(60, 3),
        ],
    );
    assert_eq!((initial.0[3], initial.1[3]), (-0.125, -0.25));
    assert_eq!(
        editor.set(&mut host, "drums.level", 0.375),
        ParameterReply::Accepted {
            generation: 5,
            sequence: 2
        }
    );
    automation(&mut host);
    assert_eq!(audio.reset().cancelled_parameter_commands, 1);
    let begin = editor.command(&mut host, "beginGesture", json!(["drums.level", 5]));
    host.apply(&begin).unwrap();
    assert_eq!(
        editor.set(&mut host, "drums.level", 0.25),
        ParameterReply::Accepted {
            generation: 5,
            sequence: 3
        }
    );
    let before = host.parameter_state();
    assert_eq!(
        (
            before.sequence,
            before.reset_cancelled,
            before.parameters[2].value
        ),
        (2, true, 0.125)
    );
    let outbound = host.peek_automation();
    let target = configuration(44_100, 32);
    assert_eq!(
        host.reconfigure_quiescent(&mut audio, target).unwrap(),
        ReconfigurationReport::Changed
    );
    assert_eq!(host.session().owner_snapshot(), accepted);
    assert_eq!(host.parameters(), descriptors);
    assert_eq!(host.parameter_state(), before);
    assert_eq!(host.peek_automation(), outbound);
    assert_eq!(host.session().next_request_id().unwrap(), next_native);
    assert_eq!(editor.status(&mut host, first_job), outcome);
    let ui = host.take_feedback().unwrap();
    assert_eq!(
        (ui.sequence, ui.frame_offset, ui.reset_cancelled),
        (1, 0, false)
    );
    let pan = host.take_feedback().unwrap();
    assert_eq!(
        (pan.sequence, pan.frame_offset, pan.parameters[0].value),
        (1, 3, 0.75)
    );
    assert!(host.take_feedback().is_none());
    assert_fractional_sample(&render::<32>(&mut audio, &[on(60, 3)]), 0.5, 1.0);
    // The prior block rendered 29 samples. The next source positions are
    // 4640/147 and 4800/147, independently interpolated from PCM frames 31..33.
    let continuation = render::<2>(&mut audio, &[]);
    let expected = [
        (11539.0 * 64.0 + 13385.0 * 83.0) / (147.0 * 32768.0),
        (13385.0 * 51.0 + 19805.0 * 96.0) / (147.0 * 32768.0),
    ];
    for (index, sample) in expected.into_iter().enumerate() {
        assert!((continuation.0[index] - sample * 0.5).abs() < 1e-6);
        assert!((continuation.1[index] - sample).abs() < 1e-6);
    }
    let applied = host.parameter_state();
    assert_eq!(
        (
            applied.generation,
            applied.sequence,
            applied.reset_cancelled
        ),
        (5, 3, false)
    );
    assert_eq!(host.take_feedback().unwrap().sequence, 3);
    let end = editor.command(&mut host, "endGesture", json!(["drums.level", 5]));
    host.apply(&end).unwrap();
    assert_eq!(
        automation(&mut host),
        vec![
            AutomationEvent::Begin { parameter_index: 2 },
            AutomationEvent::Value {
                parameter_index: 2,
                value: 1.0
            },
            AutomationEvent::End { parameter_index: 2 }
        ]
    );
    assert_eq!(
        editor.set(&mut host, "drums.level", 0.125),
        ParameterReply::Accepted {
            generation: 5,
            sequence: 4
        }
    );
    automation(&mut host);
    editor.history(&mut host, "undoInstrumentSession");
    finish(&mut host, &mut audio);
    assert_eq!(
        (
            host.session().owner_snapshot().revision,
            host.session().owner_snapshot().redo_len
        ),
        (5, 2)
    );
    assert_fractional_sample(&render::<32>(&mut audio, &[on(36, 3)]), 0.25, 0.5);
    let (mut left, mut right) = ([1.0; 33], [1.0; 33]);
    assert_eq!(
        audio.process_events(&mut left, &mut right, &[]),
        Err(RuntimeError::InvalidBlock)
    );
    assert_eq!((left, right), ([0.0; 33], [0.0; 33]));
    assert_eq!(
        host.reconfigure_quiescent(&mut audio, target).unwrap(),
        ReconfigurationReport::Unchanged
    );
}

#[test]
fn retained_event_capacity_rejects_shrink_before_commit_and_keeps_old_dsp_and_queued_value() {
    let fixture = Fixture::new();
    let (mut host, mut audio) = fixture.open(configuration(48_000, 130), 65);
    let mut editor = Editor::attach(&mut host);
    assert_eq!(
        render::<2>(&mut audio, &[on(36, 0)]).0,
        [-0.5, -358.0 / 32768.0]
    );
    editor.set(&mut host, "drums.level", 0.125);
    let before = host.session().owner_snapshot();
    let values = host.parameter_state();
    let outbound = host.peek_automation();
    for frames in [128, 16] {
        assert_eq!(
            host.reconfigure_quiescent(&mut audio, configuration(44_100, frames)),
            Err(ParameterError::Runtime(RuntimeError::InvalidConfiguration))
        );
        assert_eq!(host.session().owner_snapshot(), before);
        assert_eq!(host.parameter_state(), values);
        assert_eq!(host.peek_automation(), outbound);
    }
    assert_eq!(
        render::<2>(&mut audio, &[]).0,
        [1554.0 / 65536.0, 4137.0 / 65536.0]
    );
    assert_eq!(host.parameter_state().sequence, 1);
    assert!(!host.service_quiescent(&mut audio).unwrap().adopted);
    audio.reset();
    let old = render::<130>(&mut audio, &[on(36, 3)]);
    assert_eq!((old.0[3], old.1[3]), (-0.25, -0.25));
}

#[test]
fn missing_or_corrupt_resources_leave_the_loaded_runtime_and_configuration_accepted() {
    for corrupt in [false, true] {
        let fixture = Fixture::new();
        let (mut host, mut audio) = fixture.open(configuration(48_000, 16), 4);
        let before = host.session().owner_snapshot();
        let values = host.parameter_state();
        assert_eq!(
            render::<2>(&mut audio, &[on(36, 0)]).0,
            [-0.5, -358.0 / 32768.0]
        );
        let sample = fixture.0.path().join(ADVANCED_DRUM_KIT_SAMPLE);
        if corrupt {
            std::fs::write(&sample, b"not a wave").unwrap();
        } else {
            std::fs::remove_file(&sample).unwrap();
        }
        assert!(matches!(
            host.reconfigure_quiescent(&mut audio, configuration(44_100, 32)),
            Err(ParameterError::Session(
                dandrum_instrument_session::SessionError::PreparationFailed(_)
            ))
        ));
        assert_eq!(host.session().owner_snapshot(), before);
        assert_eq!(host.parameter_state(), values);
        assert_eq!(
            render::<2>(&mut audio, &[]).0,
            [1554.0 / 32768.0, 4137.0 / 32768.0]
        );
        assert_eq!(
            host.reconfigure_quiescent(&mut audio, configuration(48_000, 16))
                .unwrap(),
            dandrum_trigger_host::parameters::ReconfigurationReport::Unchanged
        );
    }
}

impl Fixture {
    fn gated() -> Self {
        let fixture = Self::new();
        let mut samples = vec![-0.125; 16384];
        samples[..4].copy_from_slice(&[-0.5, 0.25, -0.25, 0.5]);
        dandrum_engine::wav::write_wav_stereo_i16(
            std::fs::File::create(fixture.0.path().join(ADVANCED_DRUM_KIT_SAMPLE)).unwrap(),
            48_000,
            &samples,
            &samples,
        )
        .unwrap();
        let yaml = r#"
instrument: { id: dandrum.advanced-drum-kit, preset_schema_version: 1 }
assets:
  sample_sources:
    - id: drums
      path: assets/advanced-drums.wav
      regions: [{ id: body, start_frame: 0, end_frame: 16384 }]
ports:
  - { name: level, direction: input, signal: control, channels: 1, default: 1, min: 0, max: 4, maps_to: gain.gain }
  - { name: master, direction: output, signal: audio, channels: 2, maps_from: gain.audio_out }
preset_surface:
  parameters:
    - { name: drums.level, maps_to: level }
module_definitions:
  - type: sample_voice
    ports:
      - { name: audio, direction: output, signal: audio, channels: 2, maps_from: player.audio }
    modules:
      - { id: player, type: sample_player, static: { source: drums, region: body, channels: 2, mode: gated } }
    connections:
      - { from: voice.gate, to: player.trigger }
      - { from: voice.gate, to: player.gate }
      - { from: voice.velocity, to: player.level }
modules:
  - { id: midi, type: midi_input }
  - { id: voices, type: poly, static: { definition: sample_voice, max_voices: 4, allocation: reject-new } }
  - { id: gain, type: gain, static: { channels: 2 } }
connections:
  - { from: midi.events, to: voices.notes }
  - { from: voices.audio, to: gain.audio_in }
"#;
        std::fs::write(fixture.0.path().join(ADVANCED_DRUM_KIT_DOCUMENT), yaml).unwrap();
        fixture
    }
}
impl Editor {
    fn note(
        &mut self,
        host: &mut ParameterController,
        method: &str,
        args: Value,
        now: std::time::Instant,
    ) -> Result<(), ParameterError> {
        let request = self.command(host, method, args);
        host.apply_editor_note(&request, now)
    }
}

#[test]
fn changed_configuration_resumes_only_live_editor_intent_and_retains_press_generation_and_activity()
{
    let now = std::time::Instant::now();
    for disposition in ["held", "released", "detached", "reset"] {
        let fixture = Fixture::gated();
        let (mut host, mut audio) = fixture.open(configuration(48_000, 16), 4);
        let mut editor = Editor::attach(&mut host);
        let onset = editor.command(&mut host, "noteOn", json!([60, 1.0, 1]));
        host.apply_editor_note(&onset, now).unwrap();
        if disposition == "held" {
            assert_eq!(render::<16>(&mut audio, &[]).0[0], -0.5);
        }
        match disposition {
            "released" => editor
                .note(&mut host, "noteOff", json!([60, 1]), now)
                .unwrap(),
            "detached" => host.detach_editor(&editor.attachment).unwrap(),
            "reset" => {
                audio.reset();
            }
            _ => (),
        }
        assert_eq!(
            host.reconfigure_quiescent(&mut audio, configuration(48_000, 32))
                .unwrap(),
            dandrum_trigger_host::parameters::ReconfigurationReport::Changed
        );
        if disposition == "held" {
            assert_eq!(
                host.apply_editor_note(&onset, now),
                Err(ParameterError::ReplayedEditorNote)
            );
        }
        let after = render::<32>(&mut audio, &[]);
        if disposition == "held" {
            assert_eq!(
                &after.0[..4],
                &[-0.5, 8191.0 / 32768.0, -0.25, 16383.0 / 32768.0]
            );
            assert_eq!(after.0[31], -0.125);
            let mut foreign = Editor::attach(&mut host);
            assert_eq!(
                foreign.note(&mut host, "noteOff", json!([60, 1]), now),
                Err(ParameterError::EditorNoteConflict)
            );
            assert_eq!(
                host.service_editor_notes(now + std::time::Duration::from_millis(1999))
                    .unwrap(),
                0
            );
            assert_eq!(
                host.service_editor_notes(now + std::time::Duration::from_secs(2))
                    .unwrap(),
                1,
                "configuration cannot renew native activity"
            );
            assert_eq!(render::<32>(&mut audio, &[]), ([0.0; 32], [0.0; 32]));
        } else {
            assert_eq!(
                after,
                ([0.0; 32], [0.0; 32]),
                "{disposition} intent cannot replay"
            );
            if disposition != "detached" {
                editor
                    .note(&mut host, "noteOn", json!([60, 1.0, 1]), now)
                    .unwrap();
                assert_eq!(render::<32>(&mut audio, &[]).0[0], -0.5);
                editor
                    .note(&mut host, "noteOff", json!([60, 1]), now)
                    .unwrap();
                assert_eq!(render::<32>(&mut audio, &[]), ([0.0; 32], [0.0; 32]));
            }
        }
        assert_eq!(host.parameter_state().generation, 1);
    }
}

#[test]
fn returned_audio_configuration_preserves_host_pitch_arbitration_and_destroys_old_owners_on_control()
 {
    let fixture = Fixture::gated();
    let (mut host, audio) = fixture.open(configuration(48_000, 16), 4);
    let (mut audio, original) = std::thread::spawn(move || {
        let mut audio = audio;
        let rendered = render::<16>(&mut audio, &[on(60, 3)]);
        (audio, rendered)
    })
    .join()
    .unwrap();
    assert_eq!(
        (original.0[3], original.1[3]),
        (-0.5 * (100.0 / 127.0), -0.5 * (100.0 / 127.0))
    );
    let now = std::time::Instant::now();
    let mut editor = Editor::attach(&mut host);
    editor
        .note(&mut host, "noteOn", json!([60, 1.0, 1]), now)
        .unwrap();
    let mut result = None;
    let ownership = allocation_counter::measure(|| {
        result = Some(host.reconfigure_quiescent(&mut audio, configuration(44_100, 16)));
    });
    assert_eq!(
        result.unwrap().unwrap(),
        dandrum_trigger_host::parameters::ReconfigurationReport::Changed
    );
    assert!(
        ownership.count_total > 0,
        "complete preparation allocates on control"
    );
    assert!(
        ownership.count_current < 0,
        "signed live-allocation decrease witnesses old-owner destruction on this control thread: {ownership:?}"
    );
    let mut audio = std::thread::spawn(move || {
        assert_eq!(
            render::<16>(&mut audio, &[]),
            ([0.0; 16], [0.0; 16]),
            "host-held arbitration survives; no host voice resumption is promised"
        );
        let off = TimedEvent {
            frame_offset: 0,
            event: ControlEvent::NoteOff { note: 60 },
        };
        assert_eq!(render::<16>(&mut audio, &[off]), ([0.0; 16], [0.0; 16]));
        assert_eq!(
            render::<16>(&mut audio, &[]).0[0],
            -0.5,
            "held editor intent resumes only after real host release"
        );
        audio
    })
    .join()
    .unwrap();
    editor
        .note(&mut host, "noteOff", json!([60, 1]), now)
        .unwrap();
    assert_eq!(render::<16>(&mut audio, &[]), ([0.0; 16], [0.0; 16]));
}

#[test]
fn completed_worker_result_remains_busy_and_publishes_once_only_through_separate_job_service() {
    struct Wake(std::sync::mpsc::SyncSender<std::thread::ThreadId>);
    impl dandrum_trigger_host::parameters::SessionServiceWake for Wake {
        fn request_service(&self) {
            self.0.try_send(std::thread::current().id()).unwrap();
        }
    }
    let fixture = Fixture::new();
    let (mut host, mut audio) = fixture.open(configuration(48_000, 16), 4);
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    host.start_session_worker(std::sync::Arc::new(Wake(sender)))
        .unwrap();
    let mut editor = Editor::attach(&mut host);
    let job = editor.map(&mut host, 60);
    assert_ne!(
        receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap(),
        std::thread::current().id()
    );
    let before = host.session().owner_snapshot();
    let values = host.parameter_state();
    let target = configuration(44_100, 32);
    assert_eq!(
        host.reconfigure_quiescent(&mut audio, target),
        Err(ParameterError::Runtime(RuntimeError::Busy))
    );
    assert!(!host.service_quiescent(&mut audio).unwrap().adopted);
    assert_eq!(host.session().owner_snapshot(), before);
    assert_eq!(host.parameter_state(), values);
    assert_eq!(editor.status(&mut host, job)["state"], "running");
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    let accepted = host.session().owner_snapshot();
    assert_eq!((accepted.revision, accepted.undo_len), (1, 1));
    assert_eq!(
        host.reconfigure_quiescent(&mut audio, target),
        Err(ParameterError::Runtime(RuntimeError::Busy))
    );
    assert!(host.service_quiescent(&mut audio).unwrap().adopted);
    assert_eq!(
        host.reconfigure_quiescent(&mut audio, target).unwrap(),
        dandrum_trigger_host::parameters::ReconfigurationReport::Changed
    );
    assert_eq!(host.session().owner_snapshot(), accepted);
    assert_eq!(editor.status(&mut host, job)["state"], "completed");
    assert_fractional_sample(&render::<32>(&mut audio, &[on(60, 3)]), 1.0, 1.0);
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Idle
    );
    host.shutdown_session_worker().unwrap();
}
