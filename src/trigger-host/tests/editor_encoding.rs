use dandrum_trigger_host::parameters::{ParameterController, ParameterError};
use dandrum_trigger_host::protocol::{Admission, ProtocolError};
use dandrum_trigger_host::runtime::{ControlEvent, RuntimeAudio, TimedEvent};
use dandrum_trigger_host::{
    ADVANCED_DRUM_KIT_DOCUMENT, ADVANCED_DRUM_KIT_SAMPLE, TriggerHostConfiguration,
};
use serde_json::{Value, json};
use std::{num::NonZeroU64, path::PathBuf};

const WIRE_BYTES_LIMIT: usize = 65_536;

struct Package(tempfile::TempDir);
impl Package {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(directory.path().join("instrument/assets")).unwrap();
        let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        std::fs::copy(
            repository.join("examples/patches/advanced-drum-kit.yaml"),
            directory.path().join(ADVANCED_DRUM_KIT_DOCUMENT),
        )
        .unwrap();
        let mut samples = vec![-0.125; 51_000];
        samples[..4].copy_from_slice(&[-0.5, 0.25, -0.25, 0.5]);
        dandrum_engine::wav::write_wav_stereo_i16(
            std::fs::File::create(directory.path().join(ADVANCED_DRUM_KIT_SAMPLE)).unwrap(),
            48_000,
            &samples,
            &samples,
        )
        .unwrap();
        Self(directory)
    }

    fn open(&self, instance: u64) -> (ParameterController, RuntimeAudio) {
        ParameterController::open(
            self.0.path(),
            TriggerHostConfiguration {
                sample_rate_hz: 48_000,
                max_block_frames: 16,
            },
            NonZeroU64::new(instance).unwrap(),
            4,
            8,
        )
        .unwrap()
    }
}

fn decoded(message: String) -> Value {
    serde_json::from_str(&message).unwrap()
}

fn start_audio(audio: &mut RuntimeAudio) {
    let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
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
    assert_eq!(
        (&left[4..], &right[4..]),
        (&[-0.125; 12][..], &[-0.125; 12][..])
    );
}

fn assert_audio_continues(audio: &mut RuntimeAudio) {
    let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
    audio.process_events(&mut left, &mut right, &[]).unwrap();
    assert_eq!((left, right), ([-0.125; 16], [-0.125; 16]));
}

#[test]
fn replies_use_the_controllers_live_namespace_without_admitting_or_dispatching() {
    let package = Package::new();
    let (mut controller, mut audio) = package.open(37);
    let first = controller.attach_editor().unwrap();
    let second = controller.attach_editor().unwrap();
    let snapshot = controller.session().owner_snapshot();
    let parameters = controller.parameter_state();
    start_audio(&mut audio);
    assert_eq!(
        decoded(
            controller
                .encode_editor_reply(
                    &first.attachment_id,
                    "91",
                    Ok(json!({"level":0.37,"revision":"12"}))
                )
                .unwrap()
        ),
        json!({"version":1,"instanceId":"37","attachmentId":"1","requestId":"91","status":"accepted","result":{"level":0.37,"revision":"12"}})
    );
    assert_eq!(
        decoded(
            controller
                .encode_editor_reply(
                    &second.attachment_id,
                    "18446744073709551615",
                    Err(ProtocolError::CapabilityUnavailable)
                )
                .unwrap()
        ),
        json!({"version":1,"instanceId":"37","attachmentId":"2","requestId":"18446744073709551615","status":"rejected","error":{"code":"capabilityUnavailable"}})
    );
    // Encoding a response has not admitted this request or advanced its watermark.
    let request = json!({"version":1,"attachmentId":first.attachment_id,"requestId":"91","method":"getParameterState","args":[]}).to_string();
    assert!(matches!(
        controller.admit(&request).unwrap(),
        Admission::New(_)
    ));
    assert_eq!(controller.session().owner_snapshot(), snapshot);
    assert_eq!(controller.parameter_state(), parameters);
    assert_eq!(controller.peek_automation(), None);
    assert_eq!(controller.take_feedback(), None);
    assert_audio_continues(&mut audio);
}

#[test]
fn reply_validation_and_exact_output_bound_preserve_the_owned_session_and_audio() {
    let package = Package::new();
    let (mut controller, mut audio) = package.open(37);
    let editor = controller.attach_editor().unwrap();
    let snapshot = controller.session().owner_snapshot();
    let parameters = controller.parameter_state();
    start_audio(&mut audio);
    for invalid in ["", "0", "01", "+1", " 1", "1.0", "18446744073709551616"] {
        assert_eq!(
            controller.encode_editor_reply(&editor.attachment_id, invalid, Ok(json!(null))),
            Err(ParameterError::Protocol(ProtocolError::InvalidArguments)),
            "request {invalid:?}"
        );
        assert_eq!(
            controller.encode_editor_reply(invalid, "73", Ok(json!(null))),
            Err(ParameterError::Protocol(ProtocolError::InvalidArguments)),
            "attachment {invalid:?}"
        );
    }
    assert_eq!(
        controller.encode_editor_reply("999", "73", Ok(json!(null))),
        Err(ParameterError::Protocol(ProtocolError::StaleAttachment))
    );
    let empty = controller
        .encode_editor_reply(&editor.attachment_id, "73", Ok(json!("")))
        .unwrap();
    let boundary = "x".repeat(WIRE_BYTES_LIMIT - empty.len());
    assert_eq!(
        controller
            .encode_editor_reply(&editor.attachment_id, "73", Ok(json!(boundary)))
            .unwrap()
            .len(),
        WIRE_BYTES_LIMIT
    );
    assert_eq!(
        controller.encode_editor_reply(
            &editor.attachment_id,
            "73",
            Ok(json!(format!("{boundary}x")))
        ),
        Err(ParameterError::Protocol(ProtocolError::Oversized))
    );
    controller.detach_editor(&editor.attachment_id).unwrap();
    assert_eq!(
        controller.encode_editor_reply(
            &editor.attachment_id,
            "73",
            Err(ProtocolError::InvalidArguments)
        ),
        Err(ParameterError::Protocol(ProtocolError::StaleAttachment))
    );
    assert_eq!(controller.session().owner_snapshot(), snapshot);
    assert_eq!(controller.parameter_state(), parameters);
    assert_eq!(controller.peek_automation(), None);
    assert_audio_continues(&mut audio);
}

#[test]
fn events_share_the_live_counter_while_replies_failures_and_other_editors_do_not_consume_it() {
    let package = Package::new();
    let (mut controller, mut audio) = package.open(37);
    let (mut other, mut other_audio) = package.open(83);
    let first = controller.attach_editor().unwrap();
    let second = controller.attach_editor().unwrap();
    let foreign = other.attach_editor().unwrap();
    let snapshot = controller.session().owner_snapshot();
    let other_snapshot = other.session().owner_snapshot();
    let parameters = controller.parameter_state();
    start_audio(&mut audio);
    start_audio(&mut other_audio);
    controller
        .encode_editor_reply(&first.attachment_id, "91", Ok(json!(null)))
        .unwrap();
    assert_eq!(
        decoded(
            controller
                .encode_editor_event(
                    &first.attachment_id,
                    "parameterStateChanged",
                    json!({"generation":7,"parameters":[{"id":"drums.level","value":0.37}]})
                )
                .unwrap()
        ),
        json!({"version":1,"instanceId":"37","attachmentId":"1","sequence":"1","event":"parameterStateChanged","payload":{"generation":7,"parameters":[{"id":"drums.level","value":0.37}]}})
    );
    controller
        .encode_editor_reply(
            &first.attachment_id,
            "92",
            Err(ProtocolError::InvalidArguments),
        )
        .unwrap();
    assert_eq!(
        controller.encode_editor_event(&first.attachment_id, "unsupported", json!({})),
        Err(ParameterError::Protocol(
            ProtocolError::CapabilityUnavailable
        ))
    );
    assert_eq!(
        controller.encode_editor_event(
            &first.attachment_id,
            "instrumentSessionChanged",
            json!("x".repeat(WIRE_BYTES_LIMIT))
        ),
        Err(ParameterError::Protocol(ProtocolError::Oversized))
    );
    assert_eq!(
        decoded(
            controller
                .encode_editor_event(
                    &first.attachment_id,
                    "instrumentSessionChanged",
                    json!({"revision":"12","undoCount":3})
                )
                .unwrap()
        ),
        json!({"version":1,"instanceId":"37","attachmentId":"1","sequence":"2","event":"instrumentSessionChanged","payload":{"revision":"12","undoCount":3}})
    );
    assert_eq!(
        decoded(
            controller
                .encode_editor_event(
                    &second.attachment_id,
                    "instrumentSessionChanged",
                    json!({"revision":"17"})
                )
                .unwrap()
        ),
        json!({"version":1,"instanceId":"37","attachmentId":"2","sequence":"1","event":"instrumentSessionChanged","payload":{"revision":"17"}})
    );
    assert_eq!(
        decoded(
            other
                .encode_editor_event(
                    &foreign.attachment_id,
                    "parameterStateChanged",
                    json!({"value":0.83})
                )
                .unwrap()
        ),
        json!({"version":1,"instanceId":"83","attachmentId":"1","sequence":"1","event":"parameterStateChanged","payload":{"value":0.83}})
    );
    assert_eq!(
        decoded(
            controller
                .encode_editor_event(&first.attachment_id, "parameterStateChanged", json!(null))
                .unwrap()
        )["sequence"],
        "3"
    );
    assert_eq!(controller.session().owner_snapshot(), snapshot);
    assert_eq!(other.session().owner_snapshot(), other_snapshot);
    assert_eq!(controller.parameter_state(), parameters);
    assert_eq!(controller.peek_automation(), None);
    assert_eq!(controller.take_feedback(), None);
    assert_audio_continues(&mut audio);
    assert_audio_continues(&mut other_audio);
}

#[test]
fn event_validation_and_detach_reconnect_preserve_audio_and_start_a_fresh_attachment_sequence() {
    let package = Package::new();
    let (mut controller, mut audio) = package.open(37);
    let editor = controller.attach_editor().unwrap();
    let snapshot = controller.session().owner_snapshot();
    let parameters = controller.parameter_state();
    start_audio(&mut audio);
    for invalid in ["", "0", "01", "+1", " 1", "1.0", "18446744073709551616"] {
        assert_eq!(
            controller.encode_editor_event(invalid, "parameterStateChanged", json!(null)),
            Err(ParameterError::Protocol(ProtocolError::InvalidArguments)),
            "attachment {invalid:?}"
        );
    }
    assert_eq!(
        controller.encode_editor_event("999", "parameterStateChanged", json!(null)),
        Err(ParameterError::Protocol(ProtocolError::StaleAttachment))
    );
    let empty = controller
        .encode_editor_event(&editor.attachment_id, "parameterStateChanged", json!(""))
        .unwrap();
    assert_eq!(decoded(empty.clone())["sequence"], "1");
    let boundary = "x".repeat(WIRE_BYTES_LIMIT - empty.len());
    assert_eq!(
        controller
            .encode_editor_event(
                &editor.attachment_id,
                "parameterStateChanged",
                json!(boundary)
            )
            .unwrap()
            .len(),
        WIRE_BYTES_LIMIT
    );
    assert_eq!(
        controller.encode_editor_event(
            &editor.attachment_id,
            "parameterStateChanged",
            json!(format!("{boundary}x"))
        ),
        Err(ParameterError::Protocol(ProtocolError::Oversized))
    );
    controller.detach_editor(&editor.attachment_id).unwrap();
    assert_eq!(
        controller.encode_editor_event(&editor.attachment_id, "parameterStateChanged", json!(null)),
        Err(ParameterError::Protocol(ProtocolError::StaleAttachment))
    );
    let fresh = controller.attach_editor().unwrap();
    assert_eq!(
        decoded(
            controller
                .encode_editor_event(
                    &fresh.attachment_id,
                    "parameterStateChanged",
                    json!({"value":0.71})
                )
                .unwrap()
        ),
        json!({"version":1,"instanceId":"37","attachmentId":"2","sequence":"1","event":"parameterStateChanged","payload":{"value":0.71}})
    );
    assert_eq!(controller.session().owner_snapshot(), snapshot);
    assert_eq!(controller.parameter_state(), parameters);
    assert_eq!(controller.peek_automation(), None);
    assert_audio_continues(&mut audio);
}
