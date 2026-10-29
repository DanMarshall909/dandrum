#[path = "support/portable_state.rs"]
mod portable_state;
#[path = "support/portable_state_edit.rs"]
mod portable_state_edit;
use dandrum_trigger_host::parameters::{ParameterController, ParameterError};
use dandrum_trigger_host::protocol::{Admission, AdmittedRequest};
use dandrum_trigger_host::runtime::{ControlEvent, RuntimeAudio, TimedEvent};
use dandrum_trigger_host::{
    ADVANCED_DRUM_KIT_DOCUMENT, ADVANCED_DRUM_KIT_SAMPLE, TriggerHostConfiguration,
};
use serde_json::{Value, json};
use std::{
    num::NonZeroU64,
    sync::mpsc,
    time::{Duration, Instant},
};

struct Fixture {
    directory: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(directory.path().join("instrument/assets")).unwrap();
        let mut samples = vec![-0.125; 16384];
        samples[..4].copy_from_slice(&[-0.5, 0.25, -0.25, 0.5]);
        dandrum_engine::wav::write_wav_stereo_i16(
            std::fs::File::create(directory.path().join(ADVANCED_DRUM_KIT_SAMPLE)).unwrap(),
            48000,
            &samples,
            &samples,
        )
        .unwrap();
        let document = r#"
instrument: { id: dandrum.advanced-drum-kit, preset_schema_version: 1 }
assets:
  sample_sources:
    - id: drums
      path: assets/advanced-drums.wav
      regions:
        - { id: body, start_frame: 0, end_frame: 16384 }
        - { id: click, start_frame: 0, end_frame: 1 }
  sample_maps:
    - id: kit
      zones:
        - { id: kick, region: drums.click, key_range: [36, 36], velocity_range: [1, 127] }
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
      - { id: player, type: sample_player, static: { source: drums, region: body, channels: 2, mode: gated }, defaults: { pan: 0 } }
    connections:
      - { from: voice.gate, to: player.trigger }
      - { from: voice.gate, to: player.gate }
      - { from: voice.velocity, to: player.level }
modules:
  - { id: midi, type: midi_input }
  - { id: voices, type: poly, static: { definition: sample_voice, max_voices: 128, allocation: reject-new } }
  - { id: mapped, type: sample_map_player, static: { sample_map: kit, channels: 2, max_voices: 8 } }
  - { id: mixer, type: audio_mixer, static: { channels: 2 } }
  - { id: gain, type: gain, static: { channels: 2 } }
connections:
  - { from: midi.events, to: voices.notes }
  - { from: midi.events, to: mapped.note }
  - { from: voices.audio, to: mixer.inputs }
  - { from: mapped.audio, to: mixer.inputs }
  - { from: mixer.mix, to: gain.audio_in }
"#;
        std::fs::write(directory.path().join(ADVANCED_DRUM_KIT_DOCUMENT), document).unwrap();
        Self { directory }
    }
    fn open(&self) -> (ParameterController, Audio) {
        let (controller, audio) = ParameterController::open(
            self.directory.path(),
            TriggerHostConfiguration {
                sample_rate_hz: dandrum_trigger_host::SampleRate::new(48000.0).unwrap(),
                max_block_frames: 16,
            },
            NonZeroU64::new(1).unwrap(),
            4,
            8,
        )
        .unwrap();
        (controller, Audio::new(audio))
    }
}
enum AudioCommand {
    Render(Vec<TimedEvent>),
    Reset,
}
struct Audio {
    commands: mpsc::Sender<AudioCommand>,
    results: mpsc::Receiver<([f32; 16], [f32; 16])>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Audio {
    fn new(mut audio: RuntimeAudio) -> Self {
        let (commands, input) = mpsc::channel();
        let (output, results) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            while let Ok(command) = input.recv() {
                let (mut left, mut right) = ([0.; 16], [0.; 16]);
                let audit = allocation_counter::measure(|| match &command {
                    AudioCommand::Render(events) => {
                        audio.process_events(&mut left, &mut right, events).unwrap();
                    }
                    AudioCommand::Reset => {
                        audio.reset();
                    }
                });
                assert_eq!(
                    audit,
                    allocation_counter::AllocationInfo::default(),
                    "audio must neither allocate nor deallocate"
                );
                output.send((left, right)).unwrap();
            }
        });
        Self {
            commands,
            results,
            thread: Some(thread),
        }
    }
    fn command(&self, command: AudioCommand) -> ([f32; 16], [f32; 16]) {
        self.commands.send(command).unwrap();
        self.results.recv_timeout(Duration::from_secs(10)).unwrap()
    }
    fn render(&self, events: &[TimedEvent]) -> ([f32; 16], [f32; 16]) {
        self.command(AudioCommand::Render(events.to_vec()))
    }
    fn reset(&self) {
        self.command(AudioCommand::Reset);
    }
}
impl Drop for Audio {
    fn drop(&mut self) {
        let (dummy, _) = mpsc::channel();
        drop(std::mem::replace(&mut self.commands, dummy));
        self.thread.take().unwrap().join().unwrap();
    }
}
fn event(note: u8, velocity: u8) -> TimedEvent {
    TimedEvent {
        frame_offset: 0,
        event: ControlEvent::NoteOn { note, velocity },
    }
}
fn request(
    host: &mut ParameterController,
    attachment: &str,
    id: u64,
    method: &str,
    args: Value,
) -> AdmittedRequest {
    let message=json!({"version":1,"attachmentId":attachment,"requestId":id.to_string(),"method":method,"args":args}).to_string();
    match host.admit(&message).unwrap() {
        Admission::New(request) => request,
        _ => panic!("new request required"),
    }
}
fn note_on(
    host: &mut ParameterController,
    attachment: &str,
    id: u64,
    note: u8,
    velocity: f64,
    generation: u64,
) -> AdmittedRequest {
    request(
        host,
        attachment,
        id,
        "noteOn",
        json!([note, velocity, generation]),
    )
}
fn note_off(
    host: &mut ParameterController,
    attachment: &str,
    id: u64,
    note: u8,
    generation: u64,
) -> AdmittedRequest {
    request(host, attachment, id, "noteOff", json!([note, generation]))
}
fn heartbeat(host: &mut ParameterController, attachment: &str, id: u64) -> AdmittedRequest {
    request(host, attachment, id, "noteHeartbeat", json!([]))
}
fn silent(output: ([f32; 16], [f32; 16])) {
    assert_eq!(output, ([0.; 16], [0.; 16]));
}
fn tail(output: ([f32; 16], [f32; 16]), expected: f32) {
    assert_eq!((output.0[15], output.1[15]), (expected, expected));
}
#[test]
fn public_packaged_gated_sample_calibrates_signed_hold_release_and_velocity() {
    let fixture = Fixture::new();
    let (_host, audio) = fixture.open();
    let full = audio.render(&[event(60, 127)]);
    assert_eq!((full.0[0], full.1[0]), (-0.5, -0.5));
    tail(full, -0.125);
    tail(audio.render(&[]), -0.125);
    silent(audio.render(&[TimedEvent {
        frame_offset: 0,
        event: ControlEvent::NoteOff { note: 60 },
    }]));
    let half = audio.render(&[event(60, 64)]);
    assert_eq!(
        (half.0[0], half.1[0]),
        (-0.5 * (64.0 / 127.0), -0.5 * (64.0 / 127.0))
    );
    tail(half, -0.125 * (64.0 / 127.0));
}
#[test]
fn authenticated_owner_press_release_and_detach_preserve_host_voice_under_full_queue() {
    let fixture = Fixture::new();
    let (mut host, audio) = fixture.open();
    let owner = host.attach_editor().unwrap();
    let other = host.attach_editor().unwrap();
    let now = Instant::now();
    let on = note_on(&mut host, &owner.attachment_id, 1, 60, 1.0, 1);
    host.apply_editor_note(&on, now).unwrap();
    assert_eq!(audio.render(&[]).0[0], -0.5);
    tail(audio.render(&[]), -0.125);
    let foreign = note_off(&mut host, &other.attachment_id, 1, 60, 1);
    assert!(host.apply_editor_note(&foreign, now).is_err());
    tail(audio.render(&[]), -0.125);
    let off = note_off(&mut host, &owner.attachment_id, 2, 60, 1);
    host.apply_editor_note(&off, now).unwrap();
    silent(audio.render(&[]));
    let unknown = note_off(&mut host, &owner.attachment_id, 3, 61, 1);
    host.apply_editor_note(&unknown, now).unwrap();
    let on = note_on(&mut host, &owner.attachment_id, 4, 60, 0.5, 1);
    host.apply_editor_note(&on, now).unwrap();
    let output = audio.render(&[event(62, 127)]);
    assert_eq!(output.0[0], -0.5 - 0.5 * (64.0 / 127.0));
    // Saturate the ordinary runtime queue via actual parameter commands.
    for id in 5..9 {
        let value = request(
            &mut host,
            &owner.attachment_id,
            id,
            "setParameter",
            json!(["drums.level", 0.25, 1]),
        );
        host.apply(&value).unwrap();
        while host.acknowledge_automation() {}
    }
    let full = request(
        &mut host,
        &owner.attachment_id,
        9,
        "setParameter",
        json!(["drums.level", 0.25, 1]),
    );
    assert_eq!(host.apply(&full), Err(ParameterError::QueueFull));
    host.detach_editor(&owner.attachment_id).unwrap();
    tail(audio.render(&[]), -0.125);
    tail(audio.render(&[]), -0.125);
    assert!(host.apply_editor_note(&on, now).is_err());
    assert_eq!(host.parameter_state().sequence, 4);
    assert_eq!(host.session().owner_snapshot().revision, 0);
    let stale = note_on(&mut host, &other.attachment_id, 20, 61, 1.0, 0);
    assert_eq!(
        host.apply_editor_note(&stale, now),
        Err(ParameterError::StaleGeneration)
    );
    let unsupported = request(
        &mut host,
        &other.attachment_id,
        21,
        "getParameterState",
        json!([]),
    );
    assert_eq!(
        host.apply_editor_note(&unsupported, now),
        Err(ParameterError::UnsupportedCommand)
    );
    audio.reset();
    silent(audio.render(&[]));
}
#[test]
fn sealed_note_dispatch_rejects_alteration_foreign_replay_and_overtaken_requests() {
    let fixture = Fixture::new();
    let (mut host, audio) = fixture.open();
    let (mut foreign, _other_audio) = fixture.open();
    let owner = host.attach_editor().unwrap();
    let other = host.attach_editor().unwrap();
    let foreign_owner = foreign.attach_editor().unwrap();
    let now = Instant::now();
    let foreign_request = note_on(&mut foreign, &foreign_owner.attachment_id, 1, 60, 1.0, 1);
    assert!(host.apply_editor_note(&foreign_request, now).is_err());
    let mut original = note_on(&mut host, &owner.attachment_id, 1, 60, 1.0, 1);
    let original_command = original.command.clone();
    original.command = dandrum_trigger_host::protocol::EditorCommand::NoteOn {
        note: 61,
        velocity: 1.0,
        generation: 1,
    };
    assert!(host.apply_editor_note(&original, now).is_err());
    original.command = original_command;
    silent(audio.render(&[]));
    host.apply_editor_note(&original, now).unwrap();
    assert_eq!(audio.render(&[]).0[0], -0.5);
    assert!(
        host.apply_editor_note(&original, now).is_err(),
        "retained requests must never retrigger"
    );
    tail(audio.render(&[]), -0.125);
    let conflict = note_on(&mut host, &other.attachment_id, 1, 60, 1.0, 1);
    assert!(matches!(
        host.apply_editor_note(&conflict, now),
        Err(ParameterError::Runtime(_))
    ));
    host.detach_editor(&owner.attachment_id).unwrap();
    silent(audio.render(&[]));
    assert!(
        host.apply_editor_note(&conflict, now).is_err(),
        "semantic rejection is terminal even after conflict clears"
    );
    let older = note_on(&mut host, &other.attachment_id, 2, 61, 1.0, 1);
    let newer = note_off(&mut host, &other.attachment_id, 3, 61, 1);
    host.apply_editor_note(&newer, now).unwrap();
    assert!(host.apply_editor_note(&older, now).is_err());
    silent(audio.render(&[]));
    let fresh = note_on(&mut host, &other.attachment_id, 4, 61, 0.001, 1);
    host.apply_editor_note(&fresh, now).unwrap();
    assert_eq!(audio.render(&[]).0[0], -0.5 / 127.0);
    host.detach_editor(&other.attachment_id).unwrap();
    silent(audio.render(&[]));
}
#[test]
fn native_lease_deadline_renewal_backward_time_and_replay_are_monotonic() {
    let fixture = Fixture::new();
    let (mut host, audio) = fixture.open();
    let owner = host.attach_editor().unwrap();
    let t = Instant::now();
    let deadline = t + Duration::from_secs(2);
    let on = note_on(&mut host, &owner.attachment_id, 1, 60, 1.0, 1);
    host.apply_editor_note(&on, t).unwrap();
    tail(audio.render(&[]), -0.125);
    assert_eq!(
        host.service_editor_notes(deadline - Duration::from_nanos(1))
            .unwrap(),
        0
    );
    tail(audio.render(&[]), -0.125);
    assert!(host.service_editor_notes(t).is_err());
    let backwards = heartbeat(&mut host, &owner.attachment_id, 2);
    assert!(host.apply_editor_note(&backwards, t).is_err());
    assert!(
        host.apply_editor_note(&backwards, deadline - Duration::from_nanos(1))
            .is_err(),
        "backward semantic failure consumes transport identity"
    );
    assert_eq!(host.service_editor_notes(deadline).unwrap(), 1);
    silent(audio.render(&[]));
    assert!(host.apply_editor_note(&on, deadline).is_err());
    let late = heartbeat(&mut host, &owner.attachment_id, 3);
    host.apply_editor_note(&late, deadline).unwrap();
    silent(audio.render(&[]));
    assert_eq!(
        host.service_editor_notes(deadline + Duration::from_secs(2))
            .unwrap(),
        0
    );
    let on = note_on(&mut host, &owner.attachment_id, 4, 60, 1.0, 1);
    host.apply_editor_note(&on, deadline + Duration::from_secs(2))
        .unwrap();
    tail(audio.render(&[]), -0.125);
    let beat = heartbeat(&mut host, &owner.attachment_id, 5);
    host.apply_editor_note(&beat, deadline + Duration::from_secs(3))
        .unwrap();
    assert!(
        host.apply_editor_note(&beat, deadline + Duration::from_secs(4))
            .is_err()
    );
    assert_eq!(
        host.service_editor_notes(deadline + Duration::from_secs(4))
            .unwrap(),
        0
    );
    tail(audio.render(&[]), -0.125);
    assert_eq!(
        host.service_editor_notes(deadline + Duration::from_secs(5))
            .unwrap(),
        1
    );
    silent(audio.render(&[]));
    // A heartbeat arriving exactly at expiry services cleanup before renewal.
    let on = note_on(&mut host, &owner.attachment_id, 6, 60, 1.0, 1);
    host.apply_editor_note(&on, deadline + Duration::from_secs(5))
        .unwrap();
    let late = heartbeat(&mut host, &owner.attachment_id, 7);
    host.apply_editor_note(&late, deadline + Duration::from_secs(7))
        .unwrap();
    silent(audio.render(&[]));
    assert_eq!(host.parameter_state().sequence, 0);
    assert_eq!(host.session().owner_snapshot().revision, 0);
}
#[test]
fn rejected_note_command_never_services_or_renews_authenticated_activity() {
    let fixture = Fixture::new();
    let (mut host, audio) = fixture.open();
    let owner = host.attach_editor().unwrap();
    let t = Instant::now();
    let on = note_on(&mut host, &owner.attachment_id, 1, 60, 1.0, 1);
    host.apply_editor_note(&on, t).unwrap();
    tail(audio.render(&[]), -0.125);
    let stale = note_on(&mut host, &owner.attachment_id, 2, 61, 1.0, 0);
    assert_eq!(
        host.apply_editor_note(&stale, t + Duration::from_secs(2)),
        Err(ParameterError::StaleGeneration)
    );
    tail(audio.render(&[]), -0.125);
    assert_eq!(
        host.service_editor_notes(t + Duration::from_secs(2))
            .unwrap(),
        1
    );
    silent(audio.render(&[]));
}
#[test]
fn accepted_restore_clears_activity_preserves_dispatch_and_failed_restore_keeps_notes() {
    let fixture = Fixture::new();
    let (mut host, audio) = fixture.open();
    let owner = host.attach_editor().unwrap();
    let t = Instant::now();
    let mut saved = Vec::new();
    host.save_state(&mut saved).unwrap();
    let on = note_on(&mut host, &owner.attachment_id, 1, 60, 1.0, 1);
    host.apply_editor_note(&on, t).unwrap();
    tail(audio.render(&[]), -0.125);
    assert!(host.load_state(&mut &b"invalid"[..]).is_err());
    tail(audio.render(&[]), -0.125);
    let begin = request(
        &mut host,
        &owner.attachment_id,
        2,
        "beginGesture",
        json!(["drums.level", 1]),
    );
    host.apply(&begin).unwrap();
    assert_eq!(
        host.load_state(&mut saved.as_slice()),
        Err(dandrum_trigger_host::state::StateError::Busy)
    );
    tail(audio.render(&[]), -0.125);
    let end = request(
        &mut host,
        &owner.attachment_id,
        3,
        "endGesture",
        json!(["drums.level", 1]),
    );
    host.apply(&end).unwrap();
    while host.acknowledge_automation() {}
    let mut during_note = Vec::new();
    host.save_state(&mut during_note).unwrap();
    assert_eq!(during_note, saved, "note policy is not serialized");
    let invalid = portable_state_edit::edit(&saved, |state| {
        state["document"] = "modules: [".into();
    });
    assert!(host.load_state(&mut invalid.as_slice()).is_err());
    tail(audio.render(&[]), -0.125);
    host.load_state(&mut saved.as_slice()).unwrap();
    silent(audio.render(&[]));
    assert!(
        host.apply_editor_note(&on, t).is_err(),
        "accepted restore must not erase replay watermark"
    );
    let beat = heartbeat(&mut host, &owner.attachment_id, 4);
    host.apply_editor_note(&beat, t).unwrap();
    silent(audio.render(&[]));
    assert_eq!(
        host.service_editor_notes(t + Duration::from_secs(2))
            .unwrap(),
        0,
        "restored activity must be cleared, not kept alive by heartbeat"
    );
    let fresh = note_on(&mut host, &owner.attachment_id, 5, 60, 1.0, 2);
    host.apply_editor_note(&fresh, t + Duration::from_secs(2))
        .unwrap();
    tail(audio.render(&[]), -0.125);
}
fn mapping(
    host: &mut ParameterController,
    attachment: &str,
    transport: u64,
    job: u64,
    revision: u64,
    note: u8,
) -> AdmittedRequest {
    request(
        host,
        attachment,
        transport,
        "setInstrumentZoneKeyRange",
        json!([
            "1",
            attachment,
            job.to_string(),
            revision.to_string(),
            "kit",
            "kick",
            note,
            note
        ]),
    )
}
fn finish(host: &mut ParameterController) {
    let preparation = host.take_session_preparation().unwrap();
    let completion = std::thread::spawn(move || preparation.prepare())
        .join()
        .unwrap();
    host.submit_session_completion(completion).unwrap();
    assert_eq!(
        host.service_session_jobs().unwrap(),
        dandrum_trigger_host::parameters::SessionJobProgress::Completed
    );
}
#[test]
fn real_mapping_and_undo_preserve_original_press_generation_and_protect_newer_press() {
    let fixture = Fixture::new();
    let (mut host, audio) = fixture.open();
    let owner = host.attach_editor().unwrap();
    let t = Instant::now();
    let on = note_on(&mut host, &owner.attachment_id, 1, 60, 1.0, 1);
    host.apply_editor_note(&on, t).unwrap();
    assert_eq!(audio.render(&[]).0[0], -0.5);
    let change = mapping(&mut host, &owner.attachment_id, 2, 1, 0, 60);
    host.admit_session_command(&change).unwrap();
    finish(&mut host);
    assert_eq!(host.parameter_state().generation, 2);
    assert_eq!(host.session().owner_snapshot().undo_len, 1);
    assert_eq!(
        audio.render(&[]).0[0],
        -1.0,
        "real mapping routes the signed click to the held pitch on new graph"
    );
    tail(audio.render(&[]), -0.125);
    let off = note_off(&mut host, &owner.attachment_id, 3, 60, 1);
    host.apply_editor_note(&off, t).unwrap();
    silent(audio.render(&[]));
    let on = note_on(&mut host, &owner.attachment_id, 4, 60, 1.0, 2);
    host.apply_editor_note(&on, t).unwrap();
    assert_eq!(audio.render(&[]).0[0], -1.0);
    let old = note_off(&mut host, &owner.attachment_id, 5, 60, 1);
    assert_eq!(
        host.apply_editor_note(&old, t),
        Err(ParameterError::EditorNoteConflict)
    );
    tail(audio.render(&[]), -0.125);
    let undo = request(
        &mut host,
        &owner.attachment_id,
        6,
        "undoInstrumentSession",
        json!(["1", owner.attachment_id, "2", "1"]),
    );
    host.admit_session_command(&undo).unwrap();
    finish(&mut host);
    assert_eq!(host.parameter_state().generation, 3);
    assert_eq!(
        audio.render(&[]).0[0],
        -0.5,
        "undo removes the mapped click from pitch60"
    );
    let off = note_off(&mut host, &owner.attachment_id, 7, 60, 2);
    host.apply_editor_note(&off, t).unwrap();
    silent(audio.render(&[]));
    let on = note_on(&mut host, &owner.attachment_id, 8, 60, 1.0, 3);
    host.apply_editor_note(&on, t).unwrap();
    assert_eq!(audio.render(&[]).0[0], -0.5);
    let old = note_off(&mut host, &owner.attachment_id, 9, 60, 2);
    assert_eq!(
        host.apply_editor_note(&old, t),
        Err(ParameterError::EditorNoteConflict)
    );
    tail(audio.render(&[]), -0.125);
    let snapshot = host.session().owner_snapshot();
    assert_eq!(
        (snapshot.revision, snapshot.undo_len, snapshot.redo_len),
        (2, 0, 1)
    );
    assert_eq!(
        host.service_editor_notes(t + Duration::from_secs(2))
            .unwrap(),
        1
    );
    silent(audio.render(&[]));
}
#[test]
fn unheard_taps_detach_expiry_reset_and_two_instances_keep_distinct_lifetimes() {
    let fixture = Fixture::new();
    for released in [false, true] {
        for detach in [false, true] {
            let (mut host, audio) = fixture.open();
            let owner = host.attach_editor().unwrap();
            let t = Instant::now();
            let on = note_on(&mut host, &owner.attachment_id, 1, 60, 1.0, 1);
            host.apply_editor_note(&on, t).unwrap();
            if released {
                let off = note_off(&mut host, &owner.attachment_id, 2, 60, 1);
                host.apply_editor_note(&off, t + Duration::from_secs(1))
                    .unwrap();
                assert_eq!(
                    host.service_editor_notes(t + Duration::from_secs(2))
                        .unwrap(),
                    0
                );
            }
            if detach {
                host.detach_editor(&owner.attachment_id).unwrap();
                assert_eq!(
                    host.service_editor_notes(t + Duration::from_secs(4))
                        .unwrap(),
                    0
                );
            } else {
                assert_eq!(
                    host.service_editor_notes(t + Duration::from_secs(4))
                        .unwrap(),
                    1
                );
            }
            silent(audio.render(&[]));
            silent(audio.render(&[]));
        }
    }
    let (mut host, audio) = fixture.open();
    let (mut other, other_audio) = fixture.open();
    let owner = host.attach_editor().unwrap();
    let other_owner = other.attach_editor().unwrap();
    let t = Instant::now();
    let on = note_on(&mut host, &owner.attachment_id, 1, 60, 1.0, 1);
    host.apply_editor_note(&on, t).unwrap();
    let other_on = note_on(&mut other, &other_owner.attachment_id, 1, 60, 0.5, 1);
    other.apply_editor_note(&other_on, t).unwrap();
    tail(audio.render(&[]), -0.125);
    tail(other_audio.render(&[]), -0.125 * (64.0 / 127.0));
    audio.reset();
    let beat = heartbeat(&mut host, &owner.attachment_id, 2);
    host.apply_editor_note(&beat, t + Duration::from_secs(1))
        .unwrap();
    silent(audio.render(&[]));
    tail(other_audio.render(&[]), -0.125 * (64.0 / 127.0));
    let fresh = note_on(&mut host, &owner.attachment_id, 3, 60, 1.0, 1);
    host.apply_editor_note(&fresh, t + Duration::from_secs(1))
        .unwrap();
    tail(audio.render(&[]), -0.125);
    host.detach_editor(&owner.attachment_id).unwrap();
    silent(audio.render(&[]));
    tail(other_audio.render(&[]), -0.125 * (64.0 / 127.0));
    assert_eq!(
        other
            .service_editor_notes(t + Duration::from_secs(2))
            .unwrap(),
        1
    );
    silent(other_audio.render(&[]));
}
#[test]
fn attachment_capacity_and_native_record_reuse_match_the_existing_limit() {
    let fixture = Fixture::new();
    let (mut host, audio) = fixture.open();
    let editors: Vec<_> = (0..16).map(|_| host.attach_editor().unwrap()).collect();
    assert!(host.attach_editor().is_err());
    let t = Instant::now();
    for (index, editor) in editors.iter().enumerate() {
        let on = note_on(
            &mut host,
            &editor.attachment_id,
            1,
            60 + u8::try_from(index).unwrap(),
            1.0,
            1,
        );
        host.apply_editor_note(&on, t).unwrap();
    }
    tail(audio.render(&[]), -2.0);
    assert_eq!(
        host.service_editor_notes(t + Duration::from_secs(2))
            .unwrap(),
        16
    );
    silent(audio.render(&[]));
    for editor in editors {
        host.detach_editor(&editor.attachment_id).unwrap();
    }
    let fresh = host.attach_editor().unwrap();
    let on = note_on(&mut host, &fresh.attachment_id, 1, 127, 1.0, 1);
    host.apply_editor_note(&on, t + Duration::from_secs(2))
        .unwrap();
    tail(audio.render(&[]), -0.125);
}
struct Wake(mpsc::SyncSender<std::thread::ThreadId>);
impl dandrum_trigger_host::parameters::SessionServiceWake for Wake {
    fn request_service(&self) {
        self.0.try_send(std::thread::current().id()).unwrap();
    }
}
#[test]
fn admitted_real_worker_survives_expiry_and_detach_without_reviving_notes() {
    let fixture = Fixture::new();
    let (mut host, audio) = fixture.open();
    let (wake_tx, wake_rx) = mpsc::sync_channel(1);
    host.start_session_worker(std::sync::Arc::new(Wake(wake_tx)))
        .unwrap();
    let owner = host.attach_editor().unwrap();
    let t = Instant::now();
    let on = note_on(&mut host, &owner.attachment_id, 1, 60, 1.0, 1);
    host.apply_editor_note(&on, t).unwrap();
    tail(audio.render(&[]), -0.125);
    let change = mapping(&mut host, &owner.attachment_id, 2, 1, 0, 60);
    host.admit_session_command(&change).unwrap();
    assert_eq!(
        host.service_editor_notes(t + Duration::from_secs(2))
            .unwrap(),
        1
    );
    silent(audio.render(&[]));
    host.detach_editor(&owner.attachment_id).unwrap();
    assert_ne!(
        wake_rx.recv_timeout(Duration::from_secs(10)).unwrap(),
        std::thread::current().id()
    );
    assert_eq!(
        host.service_session_jobs().unwrap(),
        dandrum_trigger_host::parameters::SessionJobProgress::Completed
    );
    silent(audio.render(&[]));
    let fresh = host.attach_editor().unwrap();
    let status = request(
        &mut host,
        &fresh.attachment_id,
        1,
        "getInstrumentSessionJobStatus",
        json!(["1"]),
    );
    let reply: Value = serde_json::from_str(&host.session_job_status(&status).unwrap()).unwrap();
    assert_eq!(reply["result"]["state"], "completed");
    assert_eq!(
        audio.render(&[event(60, 127)]).0[0],
        -1.0,
        "worker published actual new routing after old editor and notes retired"
    );
    assert_eq!(host.session().owner_snapshot().undo_len, 1);
    assert_eq!(host.parameter_state().sequence, 0);
    host.shutdown_session_worker().unwrap();
}
#[test]
fn failed_and_deferred_structural_work_preserves_note_lease_and_original_release() {
    let fixture = Fixture::new();
    let (mut host, audio) = fixture.open();
    let owner = host.attach_editor().unwrap();
    let t = Instant::now();
    let on = note_on(&mut host, &owner.attachment_id, 1, 60, 1.0, 1);
    host.apply_editor_note(&on, t).unwrap();
    tail(audio.render(&[]), -0.125);
    let failed = request(
        &mut host,
        &owner.attachment_id,
        2,
        "setInstrumentZoneKeyRange",
        json!(["1", owner.attachment_id, "1", "0", "kit", "missing", 60, 60]),
    );
    host.admit_session_command(&failed).unwrap();
    finish(&mut host);
    tail(audio.render(&[]), -0.125);
    assert_eq!(host.parameter_state().generation, 1);
    let first = mapping(&mut host, &owner.attachment_id, 3, 2, 0, 61);
    host.admit_session_command(&first).unwrap();
    finish(&mut host);
    let second = mapping(&mut host, &owner.attachment_id, 4, 3, 1, 62);
    host.admit_session_command(&second).unwrap();
    let completion = std::thread::spawn({
        let preparation = host.take_session_preparation().unwrap();
        move || preparation.prepare()
    })
    .join()
    .unwrap();
    host.submit_session_completion(completion).unwrap();
    assert_eq!(
        host.service_session_jobs().unwrap(),
        dandrum_trigger_host::parameters::SessionJobProgress::Pending
    );
    assert_eq!(host.parameter_state().generation, 2);
    tail(audio.render(&[]), -0.125);
    assert_eq!(
        host.service_session_jobs().unwrap(),
        dandrum_trigger_host::parameters::SessionJobProgress::Completed
    );
    tail(audio.render(&[]), -0.125);
    let off = note_off(&mut host, &owner.attachment_id, 5, 60, 1);
    host.apply_editor_note(&off, t).unwrap();
    silent(audio.render(&[]));
    assert_eq!(
        host.service_editor_notes(t + Duration::from_secs(2))
            .unwrap(),
        1
    );
}
#[test]
fn callback_audit_detects_allocation_and_preexisting_destruction_on_another_thread() {
    std::thread::spawn(|| {
        let allocated = allocation_counter::measure(|| {
            std::hint::black_box(vec![1_u8; 128]);
        });
        assert!(allocated.count_total > 0);
        let existing = vec![1_u8; 128];
        let destroyed = allocation_counter::measure(|| drop(std::hint::black_box(existing)));
        assert!(destroyed.count_current < 0);
        assert!(destroyed.bytes_current < 0);
    })
    .join()
    .unwrap();
}
#[test]
fn all_pitch_cleanup_respects_host_reserved_graph_budget_until_next_callback() {
    let fixture = Fixture::new();
    let (mut host, audio) = fixture.open();
    let owner = host.attach_editor().unwrap();
    let t = Instant::now();
    for pitch in 0_u8..=127 {
        let on = note_on(
            &mut host,
            &owner.attachment_id,
            u64::from(pitch) + 1,
            pitch,
            1.0,
            1,
        );
        host.apply_editor_note(&on, t).unwrap();
    }
    tail(audio.render(&[]), -16.0);
    tail(audio.render(&[]), -16.0);
    host.detach_editor(&owner.attachment_id).unwrap();
    let parameter = TimedEvent {
        frame_offset: 0,
        event: ControlEvent::Parameter {
            index: host.parameters()[0].root_index,
            value: 1.0,
        },
    };
    tail(audio.render(&[parameter; 4]), -0.5);
    silent(audio.render(&[]));
    silent(audio.render(&[]));
}
#[test]
fn rejected_on_conflict_does_not_expire_another_attachments_due_activity() {
    let fixture = Fixture::new();
    let (mut host, audio) = fixture.open();
    let a = host.attach_editor().unwrap();
    let b = host.attach_editor().unwrap();
    let c = host.attach_editor().unwrap();
    let t = Instant::now();
    let on_c = note_on(&mut host, &c.attachment_id, 1, 62, 1.0, 1);
    host.apply_editor_note(&on_c, t).unwrap();
    let on_a = note_on(&mut host, &a.attachment_id, 1, 60, 1.0, 1);
    host.apply_editor_note(&on_a, t + Duration::from_secs(1))
        .unwrap();
    tail(audio.render(&[]), -0.25);
    let conflict = note_on(&mut host, &b.attachment_id, 1, 60, 1.0, 1);
    assert_eq!(
        host.apply_editor_note(&conflict, t + Duration::from_secs(2)),
        Err(ParameterError::Runtime(
            dandrum_trigger_host::runtime::RuntimeError::EditorOwnerConflict
        ))
    );
    tail(audio.render(&[]), -0.25);
    assert_eq!(
        host.service_editor_notes(t + Duration::from_secs(2))
            .unwrap(),
        1
    );
    tail(audio.render(&[]), -0.125);
    assert!(
        host.apply_editor_note(&conflict, t + Duration::from_secs(2))
            .is_err(),
        "semantic rejection remains terminal"
    );
}
