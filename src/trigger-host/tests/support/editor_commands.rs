use dandrum_trigger_host::editor_commands::dispatch;
use dandrum_trigger_host::{
    ADVANCED_DRUM_KIT_DOCUMENT, ADVANCED_DRUM_KIT_SAMPLE, TriggerHostConfiguration,
    parameters::ParameterController, runtime::RuntimeAudio,
};
use serde_json::{Value, json};
use std::{num::NonZeroU64, path::PathBuf};

pub fn controller(
    events: usize,
    automation: usize,
) -> (tempfile::TempDir, ParameterController, RuntimeAudio, String) {
    controller_with_document(events, automation, None)
}
pub fn controller_with_document(
    events: usize,
    automation: usize,
    document: Option<&str>,
) -> (tempfile::TempDir, ParameterController, RuntimeAudio, String) {
    let package = tempfile::tempdir().unwrap();
    let root = package.path();
    std::fs::create_dir_all(root.join("instrument/assets")).unwrap();
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
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
    if let Some(document) = document {
        std::fs::write(root.join(ADVANCED_DRUM_KIT_DOCUMENT), document).unwrap();
    }
    let (mut controller, audio) = ParameterController::open(
        root,
        TriggerHostConfiguration {
            sample_rate_hz: dandrum_trigger_host::SampleRate::new(48_000.0).unwrap(),
            max_block_frames: if events > 64 { (events * 2) as u32 } else { 16 },
        },
        NonZeroU64::new(19).unwrap(),
        events,
        automation,
    )
    .unwrap();
    let attachment = controller.attach_editor().unwrap().attachment_id;
    (package, controller, audio, attachment)
}
pub fn call(
    controller: &mut ParameterController,
    attachment: &str,
    id: u64,
    method: &str,
    args: Value,
) -> Value {
    let id = id.to_string();
    let message =
        json!({"version":1,"attachmentId":attachment,"requestId":id,"method":method,"args":args})
            .to_string();
    serde_json::from_str(&dispatch(controller, attachment, &id, &message).unwrap()).unwrap()
}
#[allow(dead_code)] // Some adapter-only fixture consumers do not render this oracle.
pub fn assert_signed_audio(audio: &mut RuntimeAudio) {
    use dandrum_trigger_host::runtime::{ControlEvent, TimedEvent};
    audio.reset();
    let (mut left, mut right) = ([123.0; 16], [123.0; 16]);
    audio
        .process_events(
            &mut left,
            &mut right,
            &[TimedEvent {
                frame_offset: 0,
                event: ControlEvent::NoteOn {
                    note: 36,
                    velocity: 127,
                },
            }],
        )
        .unwrap();
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
}
