#[path = "support/portable_state.rs"]
mod portable_state;
#[path = "support/portable_state_edit.rs"]
mod portable_state_edit;
use dandrum_trigger_host::parameters::{ParameterController, ParameterError};
use dandrum_trigger_host::protocol::{
    Admission, AdmittedRequest, OUTBOUND_BYTES_LIMIT, ProtocolError,
};
use dandrum_trigger_host::runtime::{ControlEvent, TimedEvent};
use dandrum_trigger_host::{
    ADVANCED_DRUM_KIT_DOCUMENT, ADVANCED_DRUM_KIT_SAMPLE, TriggerHostConfiguration,
};
use serde_json::{Value, json};
use std::{num::NonZeroU64, path::PathBuf};

fn package() -> (tempfile::TempDir, PathBuf) {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("Trigger.resources");
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
    (temporary, root)
}
fn open(
    root: &std::path::Path,
) -> (
    ParameterController,
    dandrum_trigger_host::runtime::RuntimeAudio,
) {
    ParameterController::open(
        root,
        TriggerHostConfiguration {
            sample_rate_hz: dandrum_trigger_host::SampleRate::new(48_000.0).unwrap(),
            max_block_frames: 16,
        },
        NonZeroU64::new(9_007_199_254_740_993).unwrap(),
        4,
        8,
    )
    .unwrap()
}
fn request(
    controller: &mut ParameterController,
    attachment: &str,
    id: u64,
    method: &str,
    args: Value,
) -> AdmittedRequest {
    match controller.admit(&json!({"version":1,"attachmentId":attachment,"requestId":id.to_string(),"method":method,"args":args}).to_string()).unwrap() {
        Admission::New(request) => request,
        Admission::Duplicate { .. } => panic!("expected a fresh transport request"),
    }
}
fn reply(controller: &ParameterController, request: &AdmittedRequest) -> Value {
    let encoded = controller.sampler_session_reply(request).unwrap();
    assert!(encoded.len() <= OUTBOUND_BYTES_LIMIT);
    serde_json::from_str(&encoded).unwrap()
}
#[test]
fn accepted_snapshot_projects_real_kit_geometry_and_applied_values_without_changing_session() {
    let (_package, root) = package();
    let (mut controller, mut audio) = open(&root);
    let editor = controller.attach_editor().unwrap();
    let read = request(
        &mut controller,
        &editor.attachment_id,
        90,
        "getSamplerSession",
        json!([]),
    );
    let envelope = reply(&controller, &read);
    assert_eq!(envelope["status"], "accepted");
    assert_eq!(envelope["requestId"], "90");
    assert_eq!(envelope["instanceId"], "9007199254740993");
    let session = &envelope["result"];
    assert_eq!(session["yaml"], editor.snapshot.yaml);
    assert_eq!(
        session["sourcePath"],
        root.join(ADVANCED_DRUM_KIT_DOCUMENT).to_str().unwrap()
    );
    assert_eq!(
        session["connection"],
        json!({"version":1,"instanceId":"9007199254740993","attachmentId":"1","revision":"0","nextRequestId":"1","limits":{"attachments":16,"history":100,"outcomes":128,"activeRequests":1},"history":{"undoCount":0,"redoCount":0}})
    );
    let document = &session["document"];
    assert_eq!(document["generation"], 1);
    assert_eq!(document["instrumentId"], "dandrum.advanced-drum-kit");
    assert_eq!(document["parameters"].as_array().unwrap().len(), 23);
    let params = document["parameters"].as_array().unwrap();
    assert_eq!(
        params
            .iter()
            .map(|parameter| parameter["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "drums.pitch_ratio",
            "drums.start_offset",
            "drums.level",
            "drums.pan",
            "drums.variation",
            "drums.kick.pitch_ratio",
            "drums.kick.start_offset",
            "drums.kick.level",
            "drums.kick.pan",
            "drums.snare.pitch_ratio",
            "drums.snare.start_offset",
            "drums.snare.level",
            "drums.snare.pan",
            "drums.snare.variation",
            "drums.closed_hat.pitch_ratio",
            "drums.closed_hat.start_offset",
            "drums.closed_hat.level",
            "drums.closed_hat.pan",
            "drums.open_hat.pitch_ratio",
            "drums.open_hat.start_offset",
            "drums.open_hat.level",
            "drums.open_hat.pan",
            "drums.open_hat.variation",
        ]
    );
    for (index, param) in params.iter().enumerate() {
        let group = match index {
            0..=4 => None,
            5..=8 => Some(1),
            9..=13 => Some(2),
            14..=17 => Some(3),
            _ => Some(4),
        };
        assert_eq!(
            param["scope"],
            if group.is_some() {
                "sampleGroup"
            } else {
                "instrument"
            }
        );
        assert_eq!(
            param.get("controlGroup"),
            group.as_ref().map(|group| json!(group)).as_ref()
        );
        let id = param["id"].as_str().unwrap();
        let (min, max, default) = if id.ends_with("pitch_ratio") {
            (0.125, 8.0, 1.0 / 9.0)
        } else if id.ends_with("level") {
            (0.0, 4.0, 0.25)
        } else if id.ends_with("pan") {
            (-1.0, 1.0, 0.5)
        } else {
            (0.0, 1.0, 0.0)
        };
        assert_eq!(param["minValue"], min);
        assert_eq!(param["maxValue"], max);
        assert_eq!(param["normalisedDefaultValue"], default);
        assert_eq!(param["normalisedValue"], default);
    }
    let source = &document["sources"][0];
    assert_eq!(
        (source["id"].as_str(), source["frameCount"].as_str()),
        (Some("drums"), Some("51000"))
    );
    assert_eq!(source["sampleRateHz"], 48000);
    assert_eq!(source["channelCount"], 1);
    let regions = source["regions"].as_array().unwrap();
    assert_eq!(regions.len(), 6);
    for (region, (id, start, end)) in regions.iter().zip([
        ("kick", "0", "12000"),
        ("snare_soft", "12000", "20000"),
        ("snare_hard_a", "20000", "28000"),
        ("snare_hard_b", "28000", "36000"),
        ("hat_closed", "36000", "39000"),
        ("hat_open", "39000", "51000"),
    ]) {
        assert_eq!(
            (
                region["id"].as_str(),
                region["startFrame"].as_str(),
                region["endFrame"].as_str()
            ),
            (Some(id), Some(start), Some(end))
        );
        assert_eq!(region["reverse"], false);
        assert!(region.get("loop").is_none());
    }
    let map = &document["maps"][0];
    assert_eq!(map["id"], "kit");
    assert_eq!(map["selectionMode"], "round_robin");
    assert_eq!(map["selectionSeed"], "2026");
    let zones = map["zones"].as_array().unwrap();
    assert_eq!(zones.len(), 7);
    for (zone, (region, key, low, high, group)) in zones.iter().zip([
        (0, 36, 1, 127, 1),
        (1, 38, 1, 63, 2),
        (2, 38, 64, 127, 2),
        (3, 38, 64, 127, 2),
        (4, 42, 1, 127, 3),
        (5, 46, 1, 127, 4),
        (5, 46, 1, 127, 4),
    ]) {
        assert_eq!(zone["sourceIndex"], 0);
        assert_eq!(zone["regionIndex"], region);
        assert_eq!(zone["keyLow"], key);
        assert_eq!(zone["keyHigh"], key);
        assert_eq!(zone["velocityLow"], low);
        assert_eq!(zone["velocityHigh"], high);
        assert_eq!(zone["controlGroup"], group);
    }
    assert_eq!(zones[2]["roundRobinGroup"], "hard_snare");
    assert_eq!(zones[3]["roundRobinGroup"], "hard_snare");
    assert_eq!(zones[5]["roundRobinGroup"], "open_hats");
    assert_eq!(zones[6]["gainDb"], -6.0);
    assert_eq!(zones[4]["chokeGroup"], "hats");
    let write = request(
        &mut controller,
        &editor.attachment_id,
        91,
        "setParameter",
        json!(["drums.level", 0.5, 1]),
    );
    controller.apply(&write).unwrap();
    let pending = reply(&controller, &read);
    let level = |value: &Value| {
        value["result"]["document"]["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|parameter| parameter["id"] == "drums.level")
            .unwrap()["normalisedValue"]
            .as_f64()
            .unwrap()
    };
    assert_eq!(level(&pending), 0.25);
    let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
    audio
        .process_events(
            &mut left,
            &mut right,
            &[TimedEvent {
                frame_offset: 0,
                event: ControlEvent::NoteOn {
                    note: 36,
                    velocity: 100,
                },
            }],
        )
        .unwrap();
    assert_eq!((left[0], right[0]), (-1.0, -1.0));
    let applied = reply(&controller, &read);
    assert_eq!(level(&applied), 0.5);
    assert_eq!(
        applied["result"]["document"]["parameters"][2]["normalisedDefaultValue"],
        0.25
    );
    assert_eq!(controller.parameter_state().parameters[2].value, 0.5);
    assert_eq!(
        controller.session().snapshot(&read.attachment).unwrap(),
        editor.snapshot
    );
    assert_eq!(controller.attach_editor().unwrap().attachment_id, "2");
}

#[test]
fn a_public_control_fanning_out_to_global_and_group_gain_has_instrument_scope_and_drives_both() {
    let (_package, root) = package();
    let path = root.join(ADVANCED_DRUM_KIT_DOCUMENT);
    let yaml = std::fs::read_to_string(&path).unwrap();
    let global_port = "  - { name: level, direction: input, signal: control, channels: 1, default: 1, min: 0, max: 4, maps_to: player.level }\n";
    let global_alias = "    - { name: drums.level, maps_to: level }\n";
    assert!(yaml.contains(global_port) && yaml.contains(global_alias));
    let yaml = yaml
        .replace(global_port, "")
        .replace(global_alias, "")
        .replacen(
            "maps_to: player.group_1_level",
            "maps_to: [player.level, player.group_1_level]",
            1,
        );
    std::fs::write(path, yaml).unwrap();
    let (mut controller, mut audio) = open(&root);
    let editor = controller.attach_editor().unwrap();
    let accepted = controller.session().owner_snapshot();
    let read = request(
        &mut controller,
        &editor.attachment_id,
        1,
        "getSamplerSession",
        json!([]),
    );
    let initial = reply(&controller, &read);
    let parameters = initial["result"]["document"]["parameters"]
        .as_array()
        .unwrap();
    assert_eq!(parameters.len(), 22);
    let public_index = controller
        .parameters()
        .iter()
        .position(|parameter| parameter.id == "drums.kick.level")
        .unwrap();
    let projected = parameters
        .iter()
        .find(|parameter| parameter["id"] == "drums.kick.level")
        .unwrap();
    assert_eq!(projected["scope"], "instrument");
    assert!(projected.get("controlGroup").is_none());
    assert_eq!(projected["normalisedValue"], 0.25);

    let onset = [TimedEvent {
        frame_offset: 3,
        event: ControlEvent::NoteOn {
            note: 36,
            velocity: 100,
        },
    }];
    let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
    audio.process_events(&mut left, &mut right, &onset).unwrap();
    assert_eq!((&left[..3], &right[..3]), (&[0.0; 3][..], &[0.0; 3][..]));
    assert_eq!((left[3], right[3]), (-0.5, -0.5));
    audio.reset();
    let set = request(
        &mut controller,
        &editor.attachment_id,
        2,
        "setParameter",
        json!(["drums.kick.level", 0.125, 1]),
    );
    assert_eq!(
        controller.apply(&set).unwrap(),
        dandrum_trigger_host::parameters::ParameterReply::Accepted {
            generation: 1,
            sequence: 1
        }
    );
    audio.process_events(&mut left, &mut right, &onset).unwrap();
    assert_eq!(controller.physical_value(public_index), Some(0.5));
    assert_eq!((&left[..3], &right[..3]), (&[0.0; 3][..], &[0.0; 3][..]));
    // The sample's -0.5 onset passes through two independently authored gain targets.
    assert_eq!((left[3], right[3]), (-0.125, -0.125));
    let changed = reply(&controller, &read);
    let projected = changed["result"]["document"]["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|parameter| parameter["id"] == "drums.kick.level")
        .unwrap();
    assert_eq!(projected["scope"], "instrument");
    assert!(projected.get("controlGroup").is_none());
    assert_eq!(projected["normalisedValue"], 0.125);
    assert_eq!(controller.session().owner_snapshot(), accepted);
}

#[test]
fn snapshot_reply_rejects_foreign_retired_and_forged_admission_without_leaking_data() {
    let (_package, root) = package();
    let (mut first, _audio) = open(&root);
    let (mut second, _audio2) = open(&root);
    let editor = first.attach_editor().unwrap();
    let other = first.attach_editor().unwrap();
    let other_instance = second.attach_editor().unwrap();
    let mut read = request(
        &mut first,
        &editor.attachment_id,
        1,
        "getSamplerSession",
        json!([]),
    );
    let foreign = request(
        &mut second,
        &other_instance.attachment_id,
        1,
        "getSamplerSession",
        json!([]),
    );
    assert_eq!(
        first.sampler_session_reply(&foreign),
        Err(ParameterError::Protocol(ProtocolError::StaleAttachment))
    );
    read.attachment_id = other.attachment_id.parse().unwrap();
    assert_eq!(
        first.sampler_session_reply(&read),
        Err(ParameterError::Protocol(ProtocolError::StaleAttachment))
    );
    read.attachment_id = 1;
    read.request_id = 2;
    assert_eq!(
        first.sampler_session_reply(&read),
        Err(ParameterError::Protocol(ProtocolError::StaleRequest))
    );
    read.request_id = 1;
    first.detach_editor(&editor.attachment_id).unwrap();
    assert_eq!(
        first.sampler_session_reply(&read),
        Err(ParameterError::Protocol(ProtocolError::StaleAttachment))
    );
    assert_eq!(
        reply(&second, &foreign)["result"]["connection"]["revision"],
        "0"
    );
}

#[test]
fn snapshot_reply_does_not_dispatch_other_commands_and_returns_no_oversized_message() {
    let (_package, root) = package();
    let (mut controller, _audio) = open(&root);
    let editor = controller.attach_editor().unwrap();
    let read = request(
        &mut controller,
        &editor.attachment_id,
        1,
        "getParameterState",
        json!([]),
    );
    assert_eq!(
        controller.sampler_session_reply(&read),
        Err(ParameterError::UnsupportedCommand)
    );
    let path = root.join(ADVANCED_DRUM_KIT_DOCUMENT);
    let yaml = std::fs::read_to_string(&path).unwrap().replace(
        "name: Advanced Drum Kit",
        &format!("name: {}", "x".repeat(OUTBOUND_BYTES_LIMIT)),
    );
    std::fs::write(&path, yaml).unwrap();
    let (mut large, _audio2) = open(&root);
    let editor = large.attach_editor().unwrap();
    let read = request(
        &mut large,
        &editor.attachment_id,
        1,
        "getSamplerSession",
        json!([]),
    );
    assert_eq!(
        large.sampler_session_reply(&read),
        Err(ParameterError::Protocol(ProtocolError::Oversized))
    );
    assert_eq!(
        large.session().snapshot(&read.attachment).unwrap(),
        editor.snapshot
    );
}

#[test]
fn accepted_authored_metadata_and_target_groups_survive_projection_and_state_replacement() {
    let (_package, root) = package();
    let path = root.join(ADVANCED_DRUM_KIT_DOCUMENT);
    let yaml = std::fs::read_to_string(&path).unwrap()
        .replace("{ id: kick, start_frame: 0, end_frame: 12000 }", "{ id: kick, start_frame: 0, end_frame: 12000, root_note: 36, gain_db: -3, pan: 0.25, reverse: true, fade_in_ms: 1, fade_out_ms: 2, loop: { mode: forward, start_frame: 100, end_frame: 500, crossfade_ms: 0.5 } }")
        .replace("id: kick, region: drums.kick", "region: drums.kick, region_override: { start_frame: 100, end_frame: 11000 }, weight: 3, pan: -0.25, pitch_semitones: 7")
        .replace("name: drums.pitch_ratio,", "name: misleading.group_4_tune,")
        .replace("name: drums.kick.pitch_ratio,", "name: plain_tune,");
    std::fs::write(&path, &yaml).unwrap();
    let (mut controller, mut audio) = open(&root);
    let editor = controller.attach_editor().unwrap();
    let read = request(
        &mut controller,
        &editor.attachment_id,
        1,
        "getSamplerSession",
        json!([]),
    );
    let initial = reply(&controller, &read);
    let document = &initial["result"]["document"];
    assert_eq!(document["parameters"][0]["id"], "misleading.group_4_tune");
    assert_eq!(document["parameters"][0]["scope"], "instrument");
    assert!(document["parameters"][0].get("controlGroup").is_none());
    assert_eq!(document["parameters"][5]["id"], "plain_tune");
    assert_eq!(document["parameters"][5]["scope"], "sampleGroup");
    assert_eq!(document["parameters"][5]["controlGroup"], 1);
    assert_eq!(
        document["sources"][0]["regions"][0],
        json!({"id":"kick","startFrame":"0","endFrame":"12000","rootNote":36,"gainDb":-3.0,"pan":0.25,"reverse":true,"fadeInMs":1.0,"fadeOutMs":2.0,"loop":{"mode":"forward","startFrame":"100","endFrame":"500","crossfadeMs":0.5}})
    );
    let zone = &document["maps"][0]["zones"][0];
    assert_eq!(
        zone["id"], "",
        "an absent authored identity is not fabricated"
    );
    assert_eq!(zone["startFrame"], "100");
    assert_eq!(zone["endFrame"], "11000");
    assert_eq!(zone["weight"], 3);
    assert_eq!(zone["pan"], -0.25);
    assert_eq!(zone["pitchSemitones"], 7.0);
    let mut bytes = Vec::new();
    controller.save_state(&mut bytes).unwrap();
    let changed = portable_state_edit::edit(&bytes, |state| {
        state["document"] = yaml
            .replace("Advanced Drum Kit", "Accepted replacement")
            .replace("key_range: [36, 36]", "key_range: [60, 60]")
            .into();
    });
    controller.load_state(&mut changed.as_slice()).unwrap();
    audio.process(&mut [0.0; 16], &mut [0.0; 16]).unwrap();
    let restored = reply(&controller, &read);
    assert!(
        restored["result"]["yaml"]
            .as_str()
            .unwrap()
            .contains("Accepted replacement")
    );
    assert_eq!(restored["result"]["connection"]["revision"], "1");
    assert_eq!(restored["result"]["document"]["generation"], 2);
    assert_eq!(
        restored["result"]["document"]["maps"][0]["zones"][0]["keyLow"],
        60
    );
    assert_eq!(
        restored["result"]["connection"]["history"],
        json!({"undoCount":0,"redoCount":0})
    );
}

#[cfg(unix)]
#[test]
fn unrepresentable_resource_provenance_returns_an_error_without_changing_the_session() {
    use std::os::unix::ffi::OsStringExt;
    let (_package, root) = package();
    let non_utf8 = root.with_file_name(std::ffi::OsString::from_vec(
        b"Trigger-\xff.resources".to_vec(),
    ));
    std::fs::rename(&root, &non_utf8).unwrap();
    let (mut controller, _audio) = open(&non_utf8);
    let editor = controller.attach_editor().unwrap();
    let read = request(
        &mut controller,
        &editor.attachment_id,
        1,
        "getSamplerSession",
        json!([]),
    );
    assert!(matches!(
        controller.sampler_session_reply(&read),
        Err(ParameterError::Host(_))
    ));
    assert_eq!(
        controller.session().snapshot(&read.attachment).unwrap(),
        editor.snapshot
    );
}
