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
    let mut invalid: serde_json::Value = serde_json::from_slice(&saved).unwrap();
    invalid["parameters"][22]["value"] = 20.into();
    assert_eq!(
        controller.load_state(&mut serde_json::to_vec(&invalid).unwrap().as_slice()),
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
