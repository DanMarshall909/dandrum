#[path = "../support/portable_state.rs"]
mod portable_state;
#[path = "../support/portable_state_edit.rs"]
mod portable_state_edit;
use super::*;
use crate::runtime::{ControlEvent, TimedEvent};
use crate::state::StateError;
use crate::{ADVANCED_DRUM_KIT_DOCUMENT, ADVANCED_DRUM_KIT_SAMPLE};
use dandrum_instrument_session::{
    MappingAdmission, MappingRequest, RequestId, SessionError, SetZoneKeyRange,
};

fn fixture() -> (tempfile::TempDir, ParameterController, RuntimeAudio) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("Trigger.resources");
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
    let (controller, audio) = ParameterController::open(
        root,
        TriggerHostConfiguration {
            sample_rate_hz: crate::SampleRate::new(48_000.0).unwrap(),
            max_block_frames: 16,
        },
        NonZeroU64::new(1).unwrap(),
        32,
        8,
    )
    .unwrap();
    (directory, controller, audio)
}
fn mapping(note: u8) -> SetZoneKeyRange {
    SetZoneKeyRange {
        map_id: "kit".into(),
        zone_id: "kick".into(),
        key_range: [note, note],
    }
}

#[test]
fn rejected_native_state_preserves_nonempty_real_history_and_successful_external_restore_resets_it()
{
    let (_directory, mut controller, mut audio) = fixture();
    // Real native authoring operations establish the journal precondition; no
    // premature implementation of the later editor mapping command is needed.
    let (attachment, _) = controller.host.session_mut().attach().unwrap();
    for note in [60, 61] {
        let candidate = controller
            .host
            .session()
            .prepare_mapping(&attachment, mapping(note))
            .unwrap();
        let accepted = controller
            .host
            .session_mut()
            .commit_mapping(candidate, &mut controller.runtime)
            .unwrap();
        controller.generation = document_generation(accepted.revision).unwrap();
        audio.process(&mut [0.0; 16], &mut [0.0; 16]).unwrap();
        drop(controller.runtime.reclaim());
    }
    let before = controller.host.session().owner_snapshot();
    assert_eq!(
        (before.revision, before.undo_len, before.redo_len),
        (2, 2, 0)
    );
    let mut saved = Vec::new();
    controller.save_state(&mut saved).unwrap();
    let invalid = portable_state_edit::edit(&saved, |state| {
        state["parameters"][22]["value"] = 20.into();
    });
    assert_eq!(
        controller.load_state(&mut invalid.as_slice()),
        Err(StateError::Invalid)
    );
    assert_eq!(controller.host.session().owner_snapshot(), before);
    controller
        .runtime
        .offer(
            PreparedRuntime::new(
                controller.host.session().prepared(),
                TriggerHostConfiguration {
                    sample_rate_hz: crate::SampleRate::new(48_000.0).unwrap(),
                    max_block_frames: 16,
                },
            )
            .unwrap(),
        )
        .ok()
        .unwrap();
    assert_eq!(
        controller.load_state(&mut saved.as_slice()),
        Err(StateError::Runtime(RuntimeError::Busy))
    );
    assert_eq!(controller.host.session().owner_snapshot(), before);
    let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
    audio
        .process_events(
            &mut left,
            &mut right,
            &[TimedEvent {
                frame_offset: 0,
                event: ControlEvent::NoteOn {
                    note: 61,
                    velocity: 100,
                },
            }],
        )
        .unwrap();
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
    controller.load_state(&mut saved.as_slice()).unwrap();
    let restored = controller.host.session().owner_snapshot();
    assert_eq!(restored.yaml, before.yaml);
    assert_eq!(
        (restored.revision, restored.undo_len, restored.redo_len),
        (3, 0, 0)
    );
}
// The job API retains printable activation errors. This valid setup requires
// successful activation; preparation/publication uses the real runtime lease.
struct JobHost<'a>(&'a mut crate::runtime::RuntimeControl);
impl dandrum_instrument_session::ActivationHost for JobHost<'_> {
    type Error = std::convert::Infallible;
    type Lease<'a>
        = crate::runtime::RuntimeLease<'a>
    where
        Self: 'a;
    fn activate<'a>(
        &'a mut self,
        prepared: &dandrum_engine::preparation::PreparedKernelInstrument,
        next: &dandrum_instrument_session::SessionSnapshot,
    ) -> Result<Self::Lease<'a>, Self::Error> {
        Ok(
            dandrum_instrument_session::ActivationHost::activate(self.0, prepared, next)
                .expect("this valid job fixture has an idle activation slot"),
        )
    }
}

#[test]
fn rejected_restore_does_not_retire_an_admitted_native_mapping_job() {
    let (_directory, mut controller, mut audio) = fixture();
    let (attachment, initial) = controller.host.session_mut().attach().unwrap();
    let mut saved = Vec::new();
    controller.save_state(&mut saved).unwrap();
    let MappingAdmission::Admitted(job) = controller
        .host
        .session_mut()
        .admit_mapping(
            &attachment,
            MappingRequest {
                instance: initial.instance.clone(),
                request_id: RequestId(1),
                expected_revision: initial.revision,
                intent: mapping(60),
            },
        )
        .unwrap()
    else {
        panic!("new admitted job")
    };
    assert_eq!(
        controller.load_state(&mut saved.as_slice()),
        Err(StateError::Session(SessionError::Busy))
    );
    assert_eq!(controller.host.session().owner_snapshot(), initial);
    let outcome = controller
        .host
        .session_mut()
        .complete_mapping(job.prepare(), &mut JobHost(&mut controller.runtime))
        .unwrap();
    let accepted = outcome.result.as_ref().unwrap();
    assert_eq!((accepted.revision, accepted.undo_len), (1, 1));
    let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
    audio
        .process_events(
            &mut left,
            &mut right,
            &[TimedEvent {
                frame_offset: 0,
                event: ControlEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                },
            }],
        )
        .unwrap();
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
}

#[test]
fn mixed_state_save_respects_unrelated_live_resources_and_native_reservations() {
    use crate::samples::NativeSampleFile;
    use serde_json::json;
    use std::sync::Arc;
    let input = |seed: u16| {
        let mut file = tempfile::tempfile().unwrap();
        dandrum_engine::wav::write_wav_stereo_i16(
            &mut file,
            32_000,
            &[-0.25 + f32::from(seed) / 32768., -0.5],
            &[-0.25, -0.5],
        )
        .unwrap();
        NativeSampleFile::new(format!("retained-{seed}.wav"), file).unwrap()
    };
    let (_directory, mut controller, mut audio) = fixture();
    let mut retained = Vec::new();
    for batch in 0..16 {
        let files = (batch * 16..batch * 16 + 16).map(input).collect();
        let job = controller.host.sample_catalog.begin(files).unwrap();
        retained.extend(
            controller
                .host
                .sample_catalog
                .accept(job.prepare())
                .unwrap()
                .unwrap(),
        );
    }
    assert_eq!(controller.host.sample_catalog.usage().resource_count, 256);
    let sample = Arc::clone(&retained[0].sample);
    let mut graph: serde_yaml::Value =
        serde_yaml::from_str(&controller.session().owner_snapshot().yaml).unwrap();
    graph["assets"]["sample_sources"]
        .as_sequence_mut()
        .unwrap()
        .push(
            serde_yaml::to_value(json!({"id":"selected","path":sample.resource_id(),
            "regions":[{"id":"hit","start_frame":0,"end_frame":2}]}))
            .unwrap(),
        );
    graph["assets"]["sample_maps"][0]["zones"]
        .as_sequence_mut()
        .unwrap()
        .push(
            serde_yaml::to_value(json!({"id":"selected_hit","region":"selected.hit",
            "key_range":[60,60],"velocity_range":[1,127],"control_group":1}))
            .unwrap(),
        );
    let yaml = serde_yaml::to_string(&graph).unwrap();
    let candidate = controller
        .host
        .session()
        .prepare_document_replacement_with_resources_owned(
            &yaml,
            [(std::path::PathBuf::from(sample.resource_id()), sample)],
        )
        .unwrap();
    controller
        .host
        .session_mut()
        .commit_document_replacement(candidate, &mut controller.runtime)
        .unwrap();
    controller.generation =
        document_generation(controller.session().owner_snapshot().revision).unwrap();
    audio.process(&mut [0.; 16], &mut [0.; 16]).unwrap();
    drop(controller.runtime.reclaim());
    let accepted = controller.session().owner_snapshot();
    let parameters = controller.parameter_state();
    let mut output = Vec::new();
    assert_eq!(
        controller.save_state(&mut output),
        Err(StateError::TooLarge),
        "unrelated live resources leave no slot for an additional factory snapshot"
    );
    assert!(output.is_empty());
    assert_eq!(controller.session().owner_snapshot(), accepted);
    assert_eq!(controller.parameter_state(), parameters);
    drop(retained);
    assert_eq!(controller.host.sample_catalog.usage().resource_count, 1);
    let pending = controller
        .host
        .sample_catalog
        .begin(vec![input(256)])
        .unwrap();
    assert_eq!(
        controller.save_state(&mut output),
        Err(StateError::Busy),
        "a native reservation cannot be bypassed by packaged save decoding"
    );
    assert!(output.is_empty());
    assert_eq!(controller.session().owner_snapshot(), accepted);
    assert_eq!(controller.parameter_state(), parameters);
    for (note, expected) in [(36, -0.5), (60, -0.25)] {
        audio.reset();
        let (mut left, mut right) = ([0.; 16], [0.; 16]);
        audio
            .process_events(
                &mut left,
                &mut right,
                &[TimedEvent {
                    frame_offset: 0,
                    event: ControlEvent::NoteOn {
                        note,
                        velocity: 127,
                    },
                }],
            )
            .unwrap();
        assert_eq!((left[0], right[0]), (expected, expected));
    }
    drop(pending);
    controller.save_state(&mut output).unwrap();
    assert_eq!(&output[..8], b"DTRGSTAT");
    assert_eq!(controller.session().owner_snapshot(), accepted);
    assert_eq!(controller.parameter_state(), parameters);
    assert_eq!(
        controller.host.sample_catalog.usage().resource_count,
        1,
        "temporary factory export ownership ends with the save"
    );
}

#[test]
fn buffered_state_preflight_preserves_native_busy_ownership_and_rechecks_after_input() {
    use crate::protocol::Admission;
    use crate::samples::NativeSampleFile;
    use serde_json::json;
    let (_directory, mut controller, _audio) = fixture();
    let legacy: serde_json::Value =
        serde_json::from_slice(include_bytes!("../fixtures/trigger-v2-state.json")).unwrap();
    let metadata = serde_json::to_vec(&json!({"product":legacy["product"],
        "document":legacy["document"],"parameters":legacy["parameters"],
        "resources":[{"path":"assets/advanced-drums.wav","encoded_bytes":102044,
            "sha256":"22d9f1c5bec480566906ca4aa7542b58800a7d16d4ee8c52039a978532f08503"}]}))
    .unwrap();
    let mut prefix = b"DTRGSTAT".to_vec();
    prefix.extend(3_u32.to_le_bytes());
    prefix.extend(u32::try_from(metadata.len()).unwrap().to_le_bytes());
    prefix.extend(102_044_u64.to_le_bytes());
    prefix.extend(metadata);
    let header = || crate::state::read_header(&mut prefix.as_slice()).unwrap();
    let accepted = controller.session().owner_snapshot();
    let parameters = controller.parameter_state();
    let path = controller
        .host
        .resource_root()
        .join(ADVANCED_DRUM_KIT_SAMPLE);
    let job = controller
        .host
        .sample_catalog
        .begin(vec![
            NativeSampleFile::new("pending.wav".into(), std::fs::File::open(&path).unwrap())
                .unwrap(),
        ])
        .unwrap();
    assert_eq!(controller.preflight_state(&header()), Err(StateError::Busy));
    assert_eq!(controller.session().owner_snapshot(), accepted);
    assert_eq!(controller.parameter_state(), parameters);
    drop(job);
    let ready = header();
    controller.preflight_state(&ready).unwrap();
    // A native edit may arrive during foreign I/O after that successful check.
    let connection = controller.attach_editor().unwrap();
    let negotiate = json!({"version":1,"attachmentId":connection.attachment_id,
        "requestId":"1","method":"negotiateEditorProtocol","args":[2]})
    .to_string();
    let Admission::New(request) = controller.admit(&negotiate).unwrap() else {
        panic!("new request")
    };
    controller.negotiate_editor_protocol(&request).unwrap();
    let frame = json!({"version":2,"attachmentId":connection.attachment_id,
        "requestId":"2","method":"prepareInstrumentSessionPatch",
        "args":["1",connection.attachment_id,"71",accepted.revision.to_string(),accepted.yaml]})
    .to_string();
    let Admission::New(request) = controller.admit(&frame).unwrap() else {
        panic!("new request")
    };
    controller.admit_session_command(&request).unwrap();
    assert_eq!(controller.preflight_state(&header()), Err(StateError::Busy));
    let mut payload = std::fs::File::open(path).unwrap();
    let buffered = ready.buffer_payload(&mut payload).unwrap();
    assert_eq!(
        controller.load_buffered_state(buffered),
        Err(StateError::Busy)
    );
    assert_eq!(controller.session().owner_snapshot(), accepted);
    assert_eq!(controller.parameter_state(), parameters);
    let query = json!({"version":1,"attachmentId":connection.attachment_id,
        "requestId":"3","method":"getInstrumentSessionJobStatus","args":["71"]})
    .to_string();
    let Admission::New(query) = controller.admit(&query).unwrap() else {
        panic!("new query")
    };
    let status: serde_json::Value =
        serde_json::from_str(&controller.session_job_status(&query).unwrap()).unwrap();
    assert_eq!(status["result"]["state"], "running");
    assert_eq!(status["result"]["request_id"], "71");
}
