#[path = "support/editor_commands.rs"]
mod fixture;
#[path = "support/portable_state.rs"]
mod portable_state;
use dandrum_trigger_host::editor_commands::dispatch;
use dandrum_trigger_host::{
    ADVANCED_DRUM_KIT_DOCUMENT, ADVANCED_DRUM_KIT_SAMPLE, TriggerHostConfiguration,
    parameters::ParameterController, runtime::RuntimeAudio,
};
use fixture::{assert_signed_audio, call, controller, controller_with_document};
use serde_json::{Value, json};
use std::{num::NonZeroU64, path::PathBuf};

#[test]
fn stamped_parameter_reply_keeps_the_existing_wire_shape_and_fences_queued_feedback() {
    use dandrum_trigger_host::editor_commands::dispatch_with_publication;
    use dandrum_trigger_host::runtime::ControlEvent;
    let (_package, mut host, mut audio, attachment) = controller(8, 8);
    let history = host.session().owner_snapshot();
    let level = host
        .parameters()
        .iter()
        .find(|p| p.id == "drums.level")
        .unwrap()
        .root_index;
    audio
        .flush_parameters(&[ControlEvent::Parameter {
            index: level,
            value: 0.5,
        }])
        .unwrap();
    let request = json!({"version":1,"attachmentId":attachment,"requestId":"1","method":"getParameterState","args":[]}).to_string();
    let (frame, publication) =
        dispatch_with_publication(&mut host, &attachment, "1", &request).unwrap();
    let reply: Value = serde_json::from_str(&frame).unwrap();
    assert_eq!(reply["status"], "accepted");
    assert_eq!(reply["requestId"], "1");
    assert_eq!(
        reply["result"].as_object().unwrap().len(),
        4,
        "publication metadata stays outside the canonical editor reply"
    );
    assert_eq!(reply["result"]["generation"], 1);
    assert_eq!(reply["result"]["sequence"], 0);
    assert_eq!(reply["result"]["reset_cancelled"], false);
    let level = reply["result"]["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "drums.level")
        .unwrap();
    assert_eq!(level["value"], 0.125);
    let (_, queued) = host.take_feedback_with_publication().unwrap();
    assert!(queued <= publication.unwrap());
    let request=json!({"version":1,"attachmentId":attachment,"requestId":"2","method":"getSamplerSession","args":[]}).to_string();
    let (frame, publication) =
        dispatch_with_publication(&mut host, &attachment, "2", &request).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&frame).unwrap()["status"],
        "accepted"
    );
    assert_eq!(publication, None);
    let request = json!({"version":1,"attachmentId":attachment,"requestId":"3","method":"getParameterState","args":[]}).to_string();
    assert_eq!(
        dispatch_with_publication(&mut host, "999", "3", &request),
        Err(dandrum_trigger_host::parameters::ParameterError::Protocol(
            dandrum_trigger_host::protocol::ProtocolError::StaleAttachment
        )),
        "a coherent snapshot cannot be encoded into a foreign namespace"
    );
    assert_eq!(host.session().owner_snapshot(), history);
}

#[test]
fn whole_patch_publication_survives_view_loss_and_native_undo_preserves_physical_values() {
    use dandrum_trigger_host::parameters::{SessionJobProgress, SessionServiceWake};
    use dandrum_trigger_host::runtime::{ControlEvent, TimedEvent};
    use std::{
        sync::{Arc, mpsc},
        time::Duration,
    };

    struct Wake(mpsc::SyncSender<std::thread::ThreadId>);
    impl SessionServiceWake for Wake {
        fn request_service(&self) {
            self.0.try_send(std::thread::current().id()).unwrap();
        }
    }
    let (_package, mut host, mut audio, attachment) = controller(8, 8);
    let original = host.session().owner_snapshot().yaml;
    let candidate = original
        .replace("Advanced Drum Kit", "Whole Patch Edit")
        .replace("key_range: [36, 36]", "key_range: [60, 60]")
        .replace(
            "name: level, direction: input, signal: control, channels: 1, default: 1,",
            "name: level, direction: input, signal: control, channels: 1, default: 2,",
        );
    assert_ne!(candidate, original);
    dandrum_engine::kernel::document::load_kernel_patch_str(&candidate).unwrap();
    let (tx, rx) = mpsc::sync_channel(1);
    host.start_session_worker(Arc::new(Wake(tx))).unwrap();
    for (id, method, args) in [
        (1, "beginGesture", json!(["drums.level", 1])),
        (2, "setParameter", json!(["drums.level", 0.125, 1])),
        (3, "endGesture", json!(["drums.level", 1])),
    ] {
        assert_eq!(
            call(&mut host, &attachment, id, method, args)["status"],
            "accepted"
        );
    }
    let render = |audio: &mut RuntimeAudio, note| {
        std::thread::scope(|scope| {
            scope
                .spawn(move || {
                    let (mut left, mut right) = ([123.0; 16], [123.0; 16]);
                    let report = audio
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
                    (left, right, report.swapped)
                })
                .join()
                .unwrap()
        })
    };
    let (left, right, _) = render(&mut audio, 36);
    assert_eq!((left[0], right[0]), (-0.25, -0.25));
    assert_eq!(
        call(
            &mut host,
            &attachment,
            4,
            "negotiateEditorProtocol",
            json!([2])
        )["status"],
        "accepted"
    );
    let frame = json!({"version":2,"attachmentId":attachment,"requestId":"5",
        "method":"prepareInstrumentSessionPatch","args":["19",attachment,"41","0",candidate]})
    .to_string();
    let reply: Value =
        serde_json::from_str(&dispatch(&mut host, &attachment, "5", &frame).unwrap()).unwrap();
    assert_eq!(
        reply["status"], "accepted",
        "whole document work must be admitted: {reply}"
    );
    assert_eq!(reply["result"]["job_id"], "41");
    host.detach_editor(&attachment).unwrap();
    assert_ne!(
        rx.recv_timeout(Duration::from_secs(5)).unwrap(),
        std::thread::current().id()
    );
    let fresh = host.attach_editor().unwrap().attachment_id;
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    let accepted = host.session().owner_snapshot();
    assert_eq!(accepted.yaml, candidate);
    assert_eq!(
        (accepted.revision, accepted.undo_len, accepted.redo_len),
        (1, 1, 0)
    );
    // Compatible authored defaults now save through the self-contained codec.
    let mut output = vec![0xa5, 0x5a];
    host.save_state(&mut output).unwrap();
    assert_eq!(&output[..2], &[0xa5, 0x5a], "append at the writer position");
    let wire = portable_state::metadata(&output[2..]);
    assert_eq!(wire["document"], candidate);
    assert_eq!(wire["parameters"][2]["value"], 0.5);
    let (_fresh_package, mut restored, mut fresh_audio, _) = controller(8, 8);
    restored.load_state(&mut &output[2..]).unwrap();
    assert_eq!(restored.session().owner_snapshot().yaml, candidate);
    let (left, right, swapped) = render(&mut fresh_audio, 60);
    assert!(swapped);
    assert_eq!((left[0], right[0]), (-0.25, -0.25));
    assert_eq!(host.session().owner_snapshot(), accepted);
    audio.reset();
    let (left, right, swapped) = render(&mut audio, 60);
    assert!(swapped);
    assert_eq!((left[0], right[0]), (-0.25, -0.25));
    audio.reset();
    let (left, right, _) = render(&mut audio, 36);
    assert_eq!((left, right), ([0.0; 16], [0.0; 16]));
    assert_eq!(
        call(
            &mut host,
            &fresh,
            1,
            "getInstrumentSessionJobStatus",
            json!(["41"])
        )["result"]["state"],
        "completed"
    );
    assert_eq!(
        call(
            &mut host,
            &fresh,
            2,
            "undoInstrumentSession",
            json!(["19", fresh, "42", "1"])
        )["status"],
        "accepted"
    );
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    let restored = host.session().owner_snapshot();
    assert_eq!(restored.yaml, original);
    assert_eq!(
        (restored.revision, restored.undo_len, restored.redo_len),
        (2, 0, 1)
    );
    audio.reset();
    let (left, right, swapped) = render(&mut audio, 36);
    assert!(swapped);
    assert_eq!((left[0], right[0]), (-0.25, -0.25));
    audio.reset();
    let (left, right, _) = render(&mut audio, 60);
    assert_eq!((left, right), ([0.0; 16], [0.0; 16]));
    assert_eq!(host.parameter_state().parameters[2].value, 0.125);
    let mut saved = Vec::new();
    host.save_state(&mut saved).unwrap();
    let saved = portable_state::metadata(&saved);
    assert_eq!(saved["document"], original);
    let level = saved["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|parameter| parameter["id"] == "drums.level")
        .unwrap();
    assert_eq!(level["value"], 0.5);
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Idle
    );
    assert_eq!(
        call(
            &mut host,
            &fresh,
            3,
            "redoInstrumentSession",
            json!(["19", fresh, "43", "2"])
        )["status"],
        "accepted"
    );
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    let redone = host.session().owner_snapshot();
    assert_eq!(redone.yaml, candidate);
    assert_eq!(
        (redone.revision, redone.undo_len, redone.redo_len),
        (3, 1, 0)
    );
    audio.reset();
    let (left, right, swapped) = render(&mut audio, 60);
    assert!(swapped);
    assert_eq!((left[0], right[0]), (-0.25, -0.25));
    audio.reset();
    let (left, right, _) = render(&mut audio, 36);
    assert_eq!((left, right), ([0.0; 16], [0.0; 16]));
    assert_eq!(host.parameter_state().parameters[2].value, 0.125);
    // Retained action identity survives the now-empty redo cursor, including
    // while a new native chooser has exclusive admission for fresh work.
    let chooser_frame = json!({"version":1,"attachmentId":fresh,"requestId":"4",
        "method":"chooseSamplerFiles","args":[]})
    .to_string();
    let dandrum_trigger_host::protocol::Admission::New(choose) =
        host.admit(&chooser_frame).unwrap()
    else {
        panic!("fresh chooser transport");
    };
    let (chooser, _) = host.begin_sample_selection(&choose).unwrap();
    for (transport, method, native_id, revision) in [
        (5, "undoInstrumentSession", "42", "1"),
        (6, "redoInstrumentSession", "43", "2"),
    ] {
        let recovered = call(
            &mut host,
            &fresh,
            transport,
            method,
            json!(["19", fresh, native_id, revision]),
        );
        assert_eq!(
            recovered["status"], "accepted",
            "exact {method} retry: {recovered}"
        );
        assert_eq!(recovered["result"]["job_id"], native_id);
        assert_eq!(
            host.service_session_jobs().unwrap(),
            SessionJobProgress::Idle
        );
        assert_eq!(host.session().owner_snapshot(), redone);
    }
    host.cancel_sample_selection(&chooser).unwrap();
    let stale = call(
        &mut host,
        &fresh,
        7,
        "undoInstrumentSession",
        json!(["19", fresh, "44", "2"]),
    );
    assert_eq!(stale["status"], "rejected");
    assert_eq!(stale["error"]["code"], "staleRequest");
    assert_eq!(host.session().next_request_id().unwrap().0, 44);
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Idle
    );
    assert_eq!(host.session().owner_snapshot(), redone);
}

#[test]
fn near_limit_patch_reserves_later_host_values_before_runtime_publication() {
    use dandrum_trigger_host::parameters::SessionJobProgress;
    use dandrum_trigger_host::protocol::PATCH_OUTBOUND_BYTES_LIMIT;

    let (_package, mut host, mut audio, attachment) = controller(128, 128);
    for (id, method, args) in [
        (1, "beginGesture", json!(["drums.level", 1])),
        (2, "setParameter", json!(["drums.level", 0.125, 1])),
        (3, "endGesture", json!(["drums.level", 1])),
    ] {
        assert_eq!(
            call(&mut host, &attachment, id, method, args)["status"],
            "accepted"
        );
    }
    let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
    audio.process_events(&mut left, &mut right, &[]).unwrap();
    let before = host.session().owner_snapshot();
    let physical = host.parameter_state();
    let baseline = call(&mut host, &attachment, 4, "getSamplerSession", json!([]));
    let read = |host: &mut ParameterController, attachment: &str, id: u64| -> Value {
        let id = id.to_string();
        let frame = json!({"version":2,"attachmentId":attachment,"requestId":id,
            "method":"getSamplerSession","args":[]})
        .to_string();
        let encoded = dispatch(host, attachment, &id, &frame).unwrap();
        assert!(encoded.len() <= PATCH_OUTBOUND_BYTES_LIMIT);
        let value: Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(
            value["status"], "accepted",
            "fixture must have a complete bounded snapshot"
        );
        value
    };
    // Long but supported group labels make the projected zones large without
    // a large source file. Every added zone is independently named and uses
    // the same real decoded kick at MIDI0, leaving the MIDI36 control intact.
    let label = "\\x01".repeat(256);
    let mut base = before
        .yaml
        .replacen(
            "round_robin_group: hard_snare",
            &format!("round_robin_group: &rr \"{label}\""),
            1,
        )
        .replace("round_robin_group: hard_snare", "round_robin_group: *rr")
        .replacen(
            "choke_group: hats",
            &format!("choke_group: &choke \"{label}\""),
            1,
        )
        .replace("choke_group: hats", "choke_group: *choke");
    let zone = |index| {
        format!(
            "        - {{ id: budget_{index}, region: drums.kick, key_range: [0, 0], velocity_range: [1, 127], round_robin_group: *rr, choke_group: *choke }}\n"
        )
    };
    let zones = (0..300).map(zone).collect::<String>();
    base = base.replacen("ports:\n", &(zones + "ports:\n"), 1);
    let (_probe_package, mut probe, mut probe_audio, probe_attachment) =
        controller_with_document(128, 128, Some(&base));
    assert_signed_audio(&mut probe_audio);
    assert_eq!(
        call(
            &mut probe,
            &probe_attachment,
            1,
            "negotiateEditorProtocol",
            json!([2])
        )["status"],
        "accepted"
    );
    let mut current = read(&mut probe, &probe_attachment, 2);
    current["result"]["document"]["parameters"] =
        baseline["result"]["document"]["parameters"].clone();
    current["result"]["sourcePath"] = baseline["result"]["sourcePath"].clone();
    // Match the existing correlation/selection reservation, using only public
    // observed DTO fields. Revision/history/generation are one digit here and
    // after the first publication, so they contribute the same byte count.
    for field in ["instanceId", "attachmentId", "requestId"] {
        current[field] = json!(u64::MAX.to_string());
    }
    for field in ["instanceId", "attachmentId", "nextRequestId"] {
        current["result"]["connection"][field] = json!(u64::MAX.to_string());
    }
    current["result"]["sampleSelections"] = (0..33)
        .map(|_| {
            json!({
                "selectionId":u64::MAX.to_string(),"phase":"preparing"
            })
        })
        .collect();
    let measured = serde_json::to_vec(&current).unwrap().len();
    let padding = PATCH_OUTBOUND_BYTES_LIMIT
        .checked_sub(measured + 103)
        .expect("qualified base must fit with room for a modest comment");
    let candidate = format!("{base}\n#{}", "x".repeat(padding));
    assert!(candidate.len() > 16_384 && candidate.len() <= 65_536);
    dandrum_engine::kernel::document::load_kernel_patch_str(&candidate).unwrap();
    current["result"]["yaml"] = json!(candidate);
    assert_eq!(
        serde_json::to_vec(&current).unwrap().len(),
        PATCH_OUTBOUND_BYTES_LIMIT - 100
    );

    // Establish actual later spellings through the public parameter commands;
    // the test does not assume a particular float formatter or value mapper.
    let thirds =
        |host: &mut ParameterController, attachment: &str, mut transport: u64, generation: u64| {
            let ids = host
                .parameters()
                .iter()
                .map(|parameter| parameter.id.clone())
                .collect::<Vec<_>>();
            for id in ids {
                for (method, args) in [
                    ("beginGesture", json!([id, generation])),
                    ("setParameter", json!([id, 1.0 / 3.0, generation])),
                    ("endGesture", json!([id, generation])),
                ] {
                    assert_eq!(
                        call(host, attachment, transport, method, args)["status"],
                        "accepted"
                    );
                    transport += 1;
                }
            }
            transport
        };
    let transport = thirds(&mut probe, &probe_attachment, 3, 1);
    probe_audio
        .process_events(&mut left, &mut right, &[])
        .unwrap();
    let later = read(&mut probe, &probe_attachment, transport);
    let mut future = current.clone();
    future["result"]["document"]["parameters"] = later["result"]["document"]["parameters"].clone();
    let future_bytes = serde_json::to_vec(&future).unwrap().len();
    assert!(
        future_bytes > PATCH_OUTBOUND_BYTES_LIMIT,
        "valid later host values must demonstrate the missing reservation: {future_bytes}"
    );
    eprintln!(
        "PROJECTION_BUDGET source={} current={} later={future_bytes}",
        candidate.len(),
        measured + padding + 3
    );

    assert_eq!(
        call(
            &mut host,
            &attachment,
            5,
            "negotiateEditorProtocol",
            json!([2])
        )["status"],
        "accepted"
    );
    let frame = json!({"version":2,"attachmentId":attachment,"requestId":"6",
        "method":"prepareInstrumentSessionPatch","args":["19",attachment,"41","0",candidate]})
    .to_string();
    let reply: Value =
        serde_json::from_str(&dispatch(&mut host, &attachment, "6", &frame).unwrap()).unwrap();
    assert_eq!(
        reply["status"], "accepted",
        "bounded preparation must own a job: {reply}"
    );
    let preparation = host.take_session_preparation().unwrap();
    let completion = std::thread::spawn(move || preparation.prepare())
        .join()
        .unwrap();
    host.submit_session_completion(completion).unwrap();
    assert_eq!(
        host.service_session_jobs().unwrap(),
        SessionJobProgress::Completed
    );
    let status = call(
        &mut host,
        &attachment,
        7,
        "getInstrumentSessionJobStatus",
        json!(["41"]),
    );
    assert_eq!(
        status["result"]["state"], "failed",
        "later numeric feedback must fit before activation, rather than stranding a newly accepted patch: {status}"
    );
    assert_eq!(host.session().owner_snapshot(), before);
    assert_eq!(host.parameter_state(), physical);
    let (mut left, mut right) = ([123.0; 16], [123.0; 16]);
    use dandrum_trigger_host::runtime::{ControlEvent, TimedEvent};
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
    assert_eq!((left[0], right[0]), (-0.25, -0.25));

    // A close neighbour with two fewer zones fits the full reservation. It
    // must publish, retain current physical values on its first block, and
    // remain readable after actual later host values have been consumed.
    let smaller = candidate.replace(&zone(298), "").replace(&zone(299), "");
    assert!(smaller.len() < candidate.len());
    let frame = json!({"version":2,"attachmentId":attachment,"requestId":"8",
        "method":"prepareInstrumentSessionPatch","args":["19",attachment,"42","0",smaller]})
    .to_string();
    let reply: Value =
        serde_json::from_str(&dispatch(&mut host, &attachment, "8", &frame).unwrap()).unwrap();
    assert_eq!(reply["status"], "accepted");
    let preparation = host.take_session_preparation().unwrap();
    let completion = std::thread::spawn(move || preparation.prepare())
        .join()
        .unwrap();
    host.submit_session_completion(completion).unwrap();
    host.service_session_jobs().unwrap();
    assert_eq!(
        call(
            &mut host,
            &attachment,
            9,
            "getInstrumentSessionJobStatus",
            json!(["42"])
        )["result"]["state"],
        "completed"
    );
    assert_eq!(host.session().owner_snapshot().yaml, smaller);
    audio.reset();
    let report = audio
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
    assert!(report.swapped);
    assert_eq!((left[0], right[0]), (-0.25, -0.25));
    assert_eq!(host.parameter_state().parameters, physical.parameters);
    assert_eq!(read(&mut host, &attachment, 10)["result"]["yaml"], smaller);
    let transport = thirds(&mut host, &attachment, 11, 2);
    audio.process_events(&mut left, &mut right, &[]).unwrap();
    let widened = read(&mut host, &attachment, transport);
    assert_eq!(widened["result"]["yaml"], smaller);
    assert_eq!(host.session().owner_snapshot().revision, 1);
}

#[test]
fn whole_patch_rejects_unsupported_profile_before_owning_a_job() {
    let original = include_str!("../../../examples/patches/advanced-drum-kit.yaml");
    let extra =
        |body: &str| original.replacen("modules:\n", &format!("modules:\n  - {{ {body} }}\n"), 1);
    let long = "x".repeat(257);
    let mut cases = vec![
        ("audio channels", extra("id: extra, type: audio_mixer, static: { channels: 3 }")),
        ("control channels", extra("id: extra, type: control_mixer, static: { channels: 2 }")),
        ("channel expression", extra("id: extra, type: audio_mixer, static: { channels: '$missing + 1' }")),
        ("static reference", extra("id: extra, type: audio_mixer, static: { channels: '$missing' }")),
        ("static resource", extra("id: extra, type: audio_mixer, static: { unsupported: { kind: sample, path: assets/advanced-drums.wav } }")),
        ("static key bytes", extra(&format!("id: extra, type: audio_mixer, static: {{ {long}: 1 }}"))),
        ("zero voices", original.replace("max_voices: 8", "max_voices: 0")),
        ("voice budget", original.replace("max_voices: 8", "max_voices: 129")),
        ("second MIDI input", extra("id: other_midi, type: midi_input")),
        ("second player", extra("id: other_player, type: sample_map_player, static: { sample_map: kit, channels: 2 }")),
        ("unsupported primitive", extra("id: extra, type: gain")),
        ("node name bytes", extra(&format!("id: {long}, type: control_mixer"))),
        ("display name bytes", original.replace("name: Advanced Drum Kit", &format!("name: {long}"))),
        ("instrument ID bytes", original.replace("id: dandrum.advanced-drum-kit", &format!("id: {long}"))),
        ("missing instrument", original.replace("instrument: { id: dandrum.advanced-drum-kit, preset_schema_version: 1 }\n", "")),
        ("root statics", original.replacen("ports:\n", "static_params: [{ name: extra, type: int, default: 1 }]\nports:\n", 1)),
        ("defined module", original.replacen("modules:\n", "module_definitions: [{ type: extra, ports: [], modules: [], connections: [] }]\nmodules:\n", 1)),
        ("missing alias", original.replace("    - { name: drums.level, maps_to: level }\n", "")),
        ("alias identity", original.replace("name: drums.level,", "name: other.level,")),
        ("alias binding", original.replace("maps_to: pitch_ratio }", "maps_to: SWAP }").replace("maps_to: start_offset }", "maps_to: pitch_ratio }").replace("maps_to: SWAP }", "maps_to: start_offset }")),
        ("physical range", original.replace("min: 0, max: 4,", "min: 0, max: 5,")),
        ("port name bytes", original.replacen("ports:\n", &format!("ports:\n  - {{ name: {long}, direction: input, signal: control, channels: 1 }}\n"), 1)),
    ];
    let nodes = (0..63)
        .map(|i| format!("  - {{ id: extra_{i}, type: control_mixer }}\n"))
        .collect::<String>();
    cases.push((
        "node budget",
        original.replacen("modules:\n", &format!("modules:\n{nodes}"), 1),
    ));
    let ports = (0..41)
        .map(|i| {
            format!("  - {{ name: extra_{i}, direction: input, signal: control, channels: 1 }}\n")
        })
        .collect::<String>();
    cases.push((
        "port budget",
        original.replacen("ports:\n", &format!("ports:\n{ports}"), 1),
    ));
    let sources = (0..256)
        .map(|i| format!("    - {{ id: extra_{i}, path: assets/advanced-drums.wav }}\n"))
        .collect::<String>();
    cases.push((
        "source budget",
        original.replacen(
            "  sample_sources:\n",
            &format!("  sample_sources:\n{sources}"),
            1,
        ),
    ));
    let maps = (0..64).map(|i| format!("    - {{ id: extra_{i}, zones: [{{ region: drums.kick, key_range: [0, 0], velocity_range: [1, 127] }}] }}\n")).collect::<String>();
    cases.push((
        "map budget",
        original.replacen("  sample_maps:\n", &format!("  sample_maps:\n{maps}"), 1),
    ));
    let regions = (0..1019)
        .map(|i| format!("        - {{ id: r{i}, start_frame: 0, end_frame: 16 }}\n"))
        .collect::<String>();
    cases.push((
        "region budget",
        original.replacen("      regions:\n", &format!("      regions:\n{regions}"), 1),
    ));
    let zones =
        "        - &extra { region: drums.kick, key_range: [0, 0], velocity_range: [1, 127] }\n"
            .to_string()
            + "        - *extra\n".repeat(1017).as_str();
    cases.push((
        "zone budget",
        original.replacen("ports:\n", &(zones + "ports:\n"), 1),
    ));
    let nodes = (0..43)
        .map(|i| format!("  - {{ id: src_{i}, type: audio_mixer, static: {{ channels: 2 }} }}\n"))
        .chain((0..3).map(|i| {
            format!("  - {{ id: dst_{i}, type: audio_mixer, static: {{ channels: 2 }} }}\n")
        }))
        .collect::<String>();
    let cables = (0..128)
        .map(|i| {
            format!(
                "  - {{ from: src_{}.audio, to: dst_{}.inputs }}\n",
                i / 3,
                i % 3
            )
        })
        .collect::<String>();
    cases.push((
        "cable budget",
        original.replacen("modules:\n", &format!("modules:\n{nodes}"), 1) + cables.as_str(),
    ));
    for (kind, candidate) in cases {
        let (_package, mut host, mut audio, attachment) = controller(8, 8);
        let before = host.session().owner_snapshot();
        assert_ne!(
            candidate, before.yaml,
            "{kind}: fixture must change the document"
        );
        assert!(
            candidate.len() <= 65_536,
            "{kind}: decoded fixture must reach the profile"
        );
        dandrum_engine::kernel::document::load_kernel_patch_str(&candidate)
            .unwrap_or_else(|error| panic!("{kind}: valid generic fixture required: {error:?}"));
        assert_eq!(
            call(
                &mut host,
                &attachment,
                1,
                "negotiateEditorProtocol",
                json!([2])
            )["status"],
            "accepted"
        );
        let frame = json!({"version":2,"attachmentId":attachment,"requestId":"2",
            "method":"prepareInstrumentSessionPatch","args":["19",attachment,"41","0",candidate]})
        .to_string();
        let reply: Value =
            serde_json::from_str(&dispatch(&mut host, &attachment, "2", &frame).unwrap()).unwrap();
        assert_eq!(
            reply["status"], "rejected",
            "unsupported {kind} must be unadmitted: {reply}"
        );
        assert_eq!(host.session().next_request_id().unwrap().0, 1);
        assert!(host.take_session_preparation().is_none());
        assert_eq!(host.session().owner_snapshot(), before);
        assert_signed_audio(&mut audio);
    }
}

#[test]
fn whole_patch_identifier_limits_measure_utf8_bytes_across_sample_metadata() {
    for kind in ["round_robin", "choke", "slice", "cue"] {
        for (identifier, accepted) in [("é".repeat(128), true), ("é".repeat(128) + "x", false)] {
            let (_package, mut host, _audio, attachment) = controller(8, 8);
            let before = host.session().owner_snapshot();
            let candidate = match kind {
                "round_robin" => before.yaml.replace("hard_snare", &identifier),
                "choke" => before.yaml.replace("choke_group: hats", &format!("choke_group: {identifier}")),
                "slice" => before.yaml.replace("      regions:", &format!("      slices: [{{ id: {identifier}, start_frame: 0, end_frame: 12000 }}]\n      regions:")),
                "cue" => before.yaml.replace("      regions:", &format!("      cues: [{{ id: {identifier}, frame: 0 }}]\n      regions:")),
                _ => unreachable!(),
            };
            dandrum_engine::kernel::document::load_kernel_patch_str(&candidate).unwrap();
            assert_eq!(
                call(
                    &mut host,
                    &attachment,
                    1,
                    "negotiateEditorProtocol",
                    json!([2])
                )["status"],
                "accepted"
            );
            let frame = json!({"version":2,"attachmentId":attachment,"requestId":"2",
                "method":"prepareInstrumentSessionPatch","args":["19",attachment,"41","0",candidate]}).to_string();
            let reply: Value =
                serde_json::from_str(&dispatch(&mut host, &attachment, "2", &frame).unwrap())
                    .unwrap();
            assert_eq!(
                reply["status"],
                if accepted { "accepted" } else { "rejected" },
                "{kind}, {} UTF-8 bytes: {reply}",
                identifier.len()
            );
            if accepted {
                let completion = host.take_session_preparation().unwrap().prepare();
                host.submit_session_completion(completion).unwrap();
                host.service_session_jobs().unwrap();
                assert_eq!(host.session().owner_snapshot().yaml, candidate);
            } else {
                assert_eq!(host.session().next_request_id().unwrap().0, 1);
                assert!(host.take_session_preparation().is_none());
                assert_eq!(host.session().owner_snapshot(), before);
            }
        }
    }
}

#[test]
fn full_patch_protocol_requires_explicit_negotiation_without_changing_the_instrument() {
    let (_package, mut controller, mut audio, attachment) = controller(8, 8);
    let before = call(
        &mut controller,
        &attachment,
        1,
        "getSamplerSession",
        json!([]),
    );
    assert_eq!(before["status"], "accepted");
    assert_signed_audio(&mut audio);
    let snapshot_frame = |id: &str, attachment: &str| {
        json!({"version":2,"attachmentId":attachment,"requestId":id,
            "method":"getSamplerSession","args":[]})
        .to_string()
    };
    let unnegotiated: Value = serde_json::from_str(
        &dispatch(
            &mut controller,
            &attachment,
            "2",
            &snapshot_frame("2", &attachment),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(unnegotiated["error"]["code"], "unsupportedVersion");
    let profile = call(
        &mut controller,
        &attachment,
        2,
        "negotiateEditorProtocol",
        json!([2]),
    );
    assert_eq!(
        profile["status"], "accepted",
        "the explicit full-patch framing handshake must be supported: {profile}"
    );
    assert_eq!(profile["version"], 1);
    assert_eq!(
        profile["result"],
        json!({
            "wireVersion":2,
            "methods":["prepareInstrumentSessionPatch","getSamplerSession"],
            "limits":{"documentBytes":65536,"requestBytes":524288,
                "replyBytes":1048576,"replayDocumentBytes":262144}
        })
    );
    let snapshot: Value = serde_json::from_str(
        &dispatch(
            &mut controller,
            &attachment,
            "3",
            &snapshot_frame("3", &attachment),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(snapshot["version"], 2);
    assert_eq!(snapshot["status"], "accepted");
    assert_eq!(snapshot["result"], before["result"]);
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            4,
            "getSamplerSession",
            json!([])
        )["result"],
        before["result"]
    );
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            5,
            "negotiateEditorProtocol",
            json!([2])
        )["result"],
        profile["result"]
    );
    let other = controller.attach_editor().unwrap().attachment_id;
    let rejected: Value = serde_json::from_str(
        &dispatch(&mut controller, &other, "1", &snapshot_frame("1", &other)).unwrap(),
    )
    .unwrap();
    assert_eq!(rejected["error"]["code"], "unsupportedVersion");
    assert_signed_audio(&mut audio);
}

#[test]
fn rejected_negotiated_requests_reply_in_the_request_version_without_publishing() {
    let (_package, mut controller, _audio, attachment) = controller(8, 8);
    let before = controller.session().owner_snapshot();
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            1,
            "negotiateEditorProtocol",
            json!([2])
        )["status"],
        "accepted"
    );
    let frame = json!({"version":2,"attachmentId":attachment,"requestId":"2","method":"getSamplerSession","args":[1]}).to_string();
    let reply: Value =
        serde_json::from_str(&dispatch(&mut controller, &attachment, "2", &frame).unwrap())
            .unwrap();
    assert_eq!(reply["status"], "rejected");
    assert_eq!(reply["error"]["code"], "invalidArguments");
    assert_eq!(
        reply["version"], 2,
        "a v2 caller must be able to correlate a rejected request"
    );
    let legacy = call(
        &mut controller,
        &attachment,
        3,
        "getSamplerSession",
        json!([1]),
    );
    assert_eq!(legacy["version"], 1);
    assert_eq!(legacy["error"]["code"], "invalidArguments");
    assert_eq!(controller.session().owner_snapshot(), before);
}

#[test]
fn a_large_accepted_snapshot_returns_a_small_version_one_error_and_a_complete_version_two_reply() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/patches/advanced-drum-kit.yaml");
    let document = std::fs::read_to_string(path).unwrap() + "\n#" + "x".repeat(70_000).as_str();
    let (_package, mut controller, _audio, attachment) =
        controller_with_document(8, 8, Some(&document));
    let before = controller.session().owner_snapshot();
    let frame = json!({"version":1,"attachmentId":attachment,"requestId":"1","method":"getSamplerSession","args":[]}).to_string();
    let outcome = dispatch(&mut controller, &attachment, "1", &frame);
    assert!(
        outcome.is_ok(),
        "an unrepresentable v1 snapshot must return a bounded rejection rather than leave its promise pending: {outcome:?}"
    );
    let encoded = outcome.unwrap();
    assert!(encoded.len() < 1024);
    let reply: Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(reply["version"], 1);
    assert_eq!(reply["status"], "rejected");
    assert_eq!(reply["error"]["code"], "oversized");
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            2,
            "negotiateEditorProtocol",
            json!([2])
        )["status"],
        "accepted"
    );
    let frame = json!({"version":2,"attachmentId":attachment,"requestId":"3","method":"getSamplerSession","args":[]}).to_string();
    let reply: Value =
        serde_json::from_str(&dispatch(&mut controller, &attachment, "3", &frame).unwrap())
            .unwrap();
    assert_eq!(reply["version"], 2);
    assert_eq!(reply["status"], "accepted");
    assert_eq!(reply["result"]["yaml"], document);
    assert_eq!(controller.session().owner_snapshot(), before);
}

#[test]
fn unrepresentable_native_resource_roots_return_a_bounded_error_and_preserve_signed_audio() {
    use std::os::unix::ffi::OsStringExt;
    let (package, _source_controller, _source_audio, _) = controller(8, 8);
    let root = package
        .path()
        .join(std::ffi::OsString::from_vec(b"resources-\xff".to_vec()));
    for resource in [ADVANCED_DRUM_KIT_DOCUMENT, ADVANCED_DRUM_KIT_SAMPLE] {
        let destination = root.join(resource);
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::copy(package.path().join(resource), destination).unwrap();
    }
    let (mut controller, mut audio) = ParameterController::open(
        &root,
        TriggerHostConfiguration {
            sample_rate_hz: dandrum_trigger_host::SampleRate::new(48_000.0).unwrap(),
            max_block_frames: 16,
        },
        NonZeroU64::new(20).unwrap(),
        8,
        8,
    )
    .unwrap();
    let attachment = controller.attach_editor().unwrap().attachment_id;
    let before = controller.session().owner_snapshot();
    assert_signed_audio(&mut audio);
    let reply = call(
        &mut controller,
        &attachment,
        1,
        "getSamplerSession",
        json!([]),
    );
    assert_eq!(reply["version"], 1);
    assert_eq!(reply["status"], "rejected");
    assert_eq!(reply["error"]["code"], "commandRejected");
    assert_eq!(controller.session().owner_snapshot(), before);
    assert_signed_audio(&mut audio);
}
#[test]
fn current_dispatch_preserves_snapshot_notes_admission_and_capability_errors() {
    let (_package, mut controller, mut audio, attachment) = controller(8, 8);
    let snapshot = call(
        &mut controller,
        &attachment,
        1,
        "getSamplerSession",
        json!([]),
    );
    assert_eq!(snapshot["status"], "accepted");
    assert_eq!(snapshot["instanceId"], "19");
    assert_eq!(snapshot["attachmentId"], attachment);
    assert_eq!(snapshot["requestId"], "1");
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            2,
            "noteOn",
            json!([36, 0.5, 1])
        )["result"],
        Value::Null
    );
    assert_eq!(
        call(&mut controller, &attachment, 3, "noteOff", json!([36, 1]))["status"],
        "accepted"
    );
    assert_eq!(
        call(&mut controller, &attachment, 4, "noteHeartbeat", json!([]))["status"],
        "accepted"
    );
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            5,
            "setParameter",
            json!(["drums.level", 2, 1])
        )["error"]["code"],
        "invalidArguments"
    );
    assert_eq!(
        call(&mut controller, &attachment, 4, "noteHeartbeat", json!([]))["error"]["code"],
        "duplicateRequest"
    );
    assert_eq!(
        call(&mut controller, &attachment, 6, "noteHeartbeat", json!([1]))["error"]["code"],
        "invalidArguments"
    );
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            7,
            "requestSourceWaveform",
            json!([])
        )["error"]["code"],
        "capabilityUnavailable"
    );
    assert_eq!(controller.parameter_state().sequence, 0);
    // The earlier accepted one-shot tap may still sound after NoteOff.
    // Clear that prior intent before checking that a rejected note has no onset.
    audio.reset();
    let (mut left, mut right) = ([123.0; 16], [123.0; 16]);
    audio.process_events(&mut left, &mut right, &[]).unwrap();
    assert_eq!((left, right), ([0.0; 16], [0.0; 16]));
    let accepted = controller.session().owner_snapshot();
    let parameters = controller.parameter_state();
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            8,
            "noteOn",
            json!([36, 1.0, 0])
        )["error"]["code"],
        "invalidArguments",
        "a newly admitted stale-generation note is rejected by the native controller"
    );
    assert_eq!(controller.session().owner_snapshot(), accepted);
    assert_eq!(controller.parameter_state(), parameters);
    assert!(controller.peek_automation().is_none());
    audio.process_events(&mut left, &mut right, &[]).unwrap();
    assert_eq!((left, right), ([0.0; 16], [0.0; 16]));
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            9,
            "noteOn",
            json!([36, 1.0, 1])
        )["status"],
        "accepted"
    );
    audio.process_events(&mut left, &mut right, &[]).unwrap();
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
}

#[test]
fn real_parameter_dispatch_preserves_admission_settlement_and_ordered_gestures() {
    let (_package, mut controller, mut audio, attachment) = controller(8, 8);
    let initial = call(
        &mut controller,
        &attachment,
        1,
        "getParameterState",
        json!([]),
    );
    assert_eq!(initial["status"], "accepted");
    assert_eq!(initial["result"]["generation"], 1);
    assert_eq!(initial["result"]["sequence"], 0);
    assert_eq!(
        initial["result"]["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == "drums.level")
            .unwrap(),
        &json!({"id":"drums.level","name":"drums.level","value":0.25})
    );
    for (id, method, args, sequence) in [
        (2, "beginGesture", json!(["drums.level", 1]), 0),
        (3, "setParameter", json!(["drums.level", 0.125, 1]), 1),
        (4, "setParameter", json!(["drums.level", 0.375, 1]), 2),
        (5, "endGesture", json!(["drums.level", 1]), 2),
    ] {
        let reply = call(&mut controller, &attachment, id, method, args);
        assert_eq!(reply["status"], "accepted");
        assert_eq!(
            reply["result"],
            json!({"status":"accepted","generation":1,"sequence":sequence})
        );
    }
    // Admitted writes are not yet settled; the real runtime is the only scalar authority.
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            6,
            "getParameterState",
            json!([])
        )["result"]["sequence"],
        0
    );
    use dandrum_trigger_host::parameters::AutomationEvent::*;
    assert_eq!(
        controller.pending_automation().collect::<Vec<_>>(),
        [
            Begin { parameter_index: 2 },
            Value {
                parameter_index: 2,
                value: 0.5
            },
            Value {
                parameter_index: 2,
                value: 1.5
            },
            End { parameter_index: 2 }
        ]
    );
    audio.flush_parameters(&[]).unwrap();
    let settled = call(
        &mut controller,
        &attachment,
        7,
        "getParameterState",
        json!([]),
    );
    assert_eq!(settled["result"]["sequence"], 2);
    assert_eq!(settled["result"]["parameters"][2]["value"], 0.375);
}
#[test]
fn parameter_dispatch_reports_terminal_busy_duplicate_and_existing_admission_errors() {
    let (_package, mut controller, mut audio, attachment) = controller(4, 3);
    for (id, args) in [
        (1, json!(["drums.level", 0.125, 2])),
        (2, json!(["missing", 0.125, 1])),
    ] {
        assert_eq!(
            call(&mut controller, &attachment, id, "setParameter", args)["error"]["code"],
            "commandRejected"
        );
    }
    let write = || json!(["drums.level", 0.125, 1]);
    assert_eq!(
        call(&mut controller, &attachment, 3, "setParameter", write())["status"],
        "accepted"
    );
    assert_eq!(
        call(&mut controller, &attachment, 3, "setParameter", write())["error"]["code"],
        "duplicateRequest"
    );
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            3,
            "setParameter",
            json!(["drums.level", 0.25, 1])
        )["error"]["code"],
        "conflictingRequest"
    );
    assert_eq!(
        call(&mut controller, &attachment, 4, "setParameter", write())["error"]["code"],
        "busy"
    );
    audio.flush_parameters(&[]).unwrap();
    assert_eq!(
        controller.parameter_state().sequence,
        1,
        "duplicate never redispatches"
    );
    assert_eq!(
        controller.pending_automation().count(),
        3,
        "settlement does not release host delivery credit"
    );
    // Preserved input/admission errors take precedence over dispatch semantics.
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            5,
            "setParameter",
            json!(["drums.level", 2, 1])
        )["error"]["code"],
        "invalidArguments"
    );
}

#[test]
fn foreign_gesture_and_runtime_capacity_reject_without_partial_parameter_publication() {
    let (_package, mut owner, _audio, attachment) = controller(1, 8);
    let other = owner.attach_editor().unwrap().attachment_id;
    assert_eq!(
        call(
            &mut owner,
            &attachment,
            1,
            "beginGesture",
            json!(["drums.level", 1])
        )["status"],
        "accepted"
    );
    assert_eq!(
        call(
            &mut owner,
            &other,
            1,
            "setParameter",
            json!(["drums.level", 0.125, 1])
        )["error"]["code"],
        "commandRejected"
    );
    assert_eq!(
        call(
            &mut owner,
            &attachment,
            2,
            "setParameter",
            json!(["drums.level", 0.125, 1])
        )["status"],
        "accepted"
    );
    assert_eq!(
        call(
            &mut owner,
            &attachment,
            3,
            "setParameter",
            json!(["drums.level", 0.375, 1])
        )["error"]["code"],
        "busy"
    );
    assert_eq!(owner.pending_automation().count(), 2);
    assert_eq!(
        call(
            &mut owner,
            &attachment,
            4,
            "endGesture",
            json!(["drums.level", 1])
        )["status"],
        "accepted"
    );
    assert_eq!(
        call(
            &mut owner,
            &attachment,
            5,
            "endGesture",
            json!(["drums.level", 1])
        )["error"]["code"],
        "commandRejected"
    );
}

#[test]
fn dispatcher_replies_to_selection_and_unknown_job_queries_without_native_file_authority() {
    let (_package, mut controller, mut audio, attachment) = controller(8, 8);
    let before = controller.session().owner_snapshot();
    let values = controller.parameter_state();
    for (id, method, args, error) in [
        (
            1,
            "getSamplerSelectionStatus",
            json!(["1"]),
            "unknownSelection",
        ),
        (
            2,
            "releaseSamplerSelection",
            json!(["1"]),
            "unknownSelection",
        ),
        (3, "chooseSamplerFiles", json!([]), "capabilityUnavailable"),
        (
            4,
            "getInstrumentSessionJobStatus",
            json!(["41"]),
            "commandRejected",
        ),
    ] {
        let reply = call(&mut controller, &attachment, id, method, args);
        assert_eq!(reply["status"], "rejected", "{method}: {reply}");
        assert_eq!(reply["error"]["code"], error, "{method}: {reply}");
        assert_eq!(reply["requestId"], id.to_string());
        assert_eq!(controller.session().owner_snapshot(), before);
        assert_eq!(controller.parameter_state(), values);
    }
    assert!(controller.peek_automation().is_none());
    assert_signed_audio(&mut audio);
}
