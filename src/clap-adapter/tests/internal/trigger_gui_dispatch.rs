#[path = "../../../trigger-host/tests/support/portable_state.rs"]
mod portable_state;
use super::dispatch;
use dandrum_trigger_host::{
    ADVANCED_DRUM_KIT_DOCUMENT, ADVANCED_DRUM_KIT_SAMPLE, TriggerHostConfiguration,
    parameters::ParameterController, runtime::RuntimeAudio,
};
use serde_json::{Value, json};
use std::{num::NonZeroU64, path::PathBuf};

fn controller(
    events: usize,
    automation: usize,
) -> (tempfile::TempDir, ParameterController, RuntimeAudio, String) {
    controller_with_document(events, automation, None)
}
fn controller_with_document(
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
fn call(
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
fn assert_signed_audio(audio: &mut RuntimeAudio) {
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
    let original = include_str!("../../../../examples/patches/advanced-drum-kit.yaml");
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
fn output_mirror_acknowledges_only_host_accepted_prefix_and_retries_the_rejected_head() {
    let (_package, mut controller, mut audio, attachment) = controller(8, 8);
    let mut saved = Vec::new();
    controller.save_state(&mut saved).unwrap();
    call(
        &mut controller,
        &attachment,
        1,
        "setParameter",
        json!(["drums.level", 0.125, 1]),
    );
    let expected = controller.pending_automation().collect::<Vec<_>>();
    let (mut mirror, mut output) = crate::trigger::automation::pair();
    mirror.service(&mut controller);
    let mut attempts = 0;
    output.deliver(|_| {
        attempts += 1;
        false
    });
    assert_eq!(attempts, 1, "a rejected head ends this callback");
    assert_eq!(
        controller.pending_automation().collect::<Vec<_>>(),
        expected
    );
    let mut accepted = Vec::new();
    output.deliver(|event| {
        if accepted.len() == 1 {
            return false;
        }
        accepted.push(event);
        true
    });
    assert_eq!(accepted, &expected[..1]);
    assert_eq!(
        controller.pending_automation().count(),
        3,
        "host acceptance is not yet a collected credit"
    );
    audio.flush_parameters(&[]).unwrap();
    assert_eq!(
        controller.load_state(&mut saved.as_slice()),
        Err(dandrum_trigger_host::state::StateError::Busy),
        "settlement and uncollected credit cannot bypass restore Busy"
    );
    mirror.service(&mut controller);
    assert_eq!(
        controller.pending_automation().collect::<Vec<_>>(),
        expected[1..]
    );
    output.deliver(|event| {
        accepted.push(event);
        true
    });
    assert_eq!(accepted, expected);
    assert_eq!(controller.pending_automation().count(), 2);
    mirror.service(&mut controller);
    assert!(controller.pending_automation().next().is_none());
    assert!(!mirror.pending());
    controller.load_state(&mut saved.as_slice()).unwrap();
    controller.service_quiescent(&mut audio).unwrap();
    assert_eq!(controller.parameter_state().generation, 2);
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            2,
            "setParameter",
            json!(["drums.level", 0.375, 1])
        )["error"]["code"],
        "commandRejected"
    );
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            3,
            "setParameter",
            json!(["drums.level", 0.375, 2])
        )["status"],
        "accepted"
    );
    mirror.service(&mut controller);
    let mut restored = Vec::new();
    output.deliver(|event| {
        restored.push(event);
        true
    });
    mirror.service(&mut controller);
    use dandrum_trigger_host::parameters::AutomationEvent::*;
    assert_eq!(
        restored,
        [
            Begin { parameter_index: 2 },
            Value {
                parameter_index: 2,
                value: 1.5
            },
            End { parameter_index: 2 }
        ]
    );
    assert!(!mirror.pending());
}
#[test]
fn output_mirror_stages_more_than_capacity_and_detach_ends_without_overtaking() {
    let (_package, mut controller, mut audio, attachment) = controller(128, 256);
    call(
        &mut controller,
        &attachment,
        1,
        "beginGesture",
        json!(["drums.level", 1]),
    );
    for id in 2..102 {
        assert_eq!(
            call(
                &mut controller,
                &attachment,
                id,
                "setParameter",
                json!(["drums.level", (id % 8) as f64 / 8.0, 1])
            )["status"],
            "accepted"
        );
    }
    controller.detach_editor(&attachment).unwrap();
    let expected = controller.pending_automation().collect::<Vec<_>>();
    assert_eq!(expected.len(), 102);
    let (mut mirror, mut output) = crate::trigger::automation::pair();
    mirror.service(&mut controller);
    let mut accepted = Vec::new();
    output.deliver(|event| {
        accepted.push(event);
        true
    });
    assert_eq!(accepted, expected[..64]);
    assert_eq!(controller.pending_automation().count(), 102);
    mirror.service(&mut controller);
    assert_eq!(controller.pending_automation().count(), 38);
    output.deliver(|event| {
        accepted.push(event);
        true
    });
    mirror.service(&mut controller);
    assert_eq!(accepted, expected);
    assert!(!mirror.pending());
    audio.flush_parameters(&[]).unwrap();
}
#[test]
fn output_callback_attempt_budget_includes_concurrent_refill() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let (_package, mut controller, _audio, attachment) = controller(128, 256);
    for id in 1..41 {
        call(
            &mut controller,
            &attachment,
            id,
            "setParameter",
            json!(["drums.level", 0.125, 1]),
        );
    }
    let expected = controller.pending_automation().collect::<Vec<_>>();
    let (mut mirror, mut output) = crate::trigger::automation::pair();
    mirror.service(&mut controller);
    let ready = Arc::new(AtomicBool::new(false));
    let refilled = Arc::new(AtomicBool::new(false));
    let ready_audio = Arc::clone(&ready);
    let refilled_audio = Arc::clone(&refilled);
    let delivery = std::thread::spawn(move || {
        let mut accepted = Vec::new();
        output.deliver(|event| {
            if accepted.len() == 1 {
                ready_audio.store(true, Ordering::Release);
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
                while !refilled_audio.load(Ordering::Acquire) {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "test main must stage concurrent refill"
                    );
                    std::thread::yield_now();
                }
            }
            assert!(
                accepted.len() < 64,
                "callback must stop at64 even with refill"
            );
            accepted.push(event);
            true
        });
        (output, accepted)
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !ready.load(Ordering::Acquire) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    mirror.service(&mut controller);
    refilled.store(true, Ordering::Release);
    let (mut output, mut accepted) = delivery.join().unwrap();
    assert_eq!(accepted, expected[..64]);
    mirror.service(&mut controller);
    output.deliver(|event| {
        accepted.push(event);
        true
    });
    mirror.service(&mut controller);
    assert_eq!(accepted, expected);
    assert!(!mirror.pending());
}

#[test]
fn two_live_mirrors_keep_refused_heads_and_accepted_credits_independent() {
    use dandrum_trigger_host::parameters::AutomationEvent::*;
    let (_package_a, mut a, mut audio_a, attachment_a) = controller(8, 8);
    let (_package_b, mut b, mut audio_b, attachment_b) = controller(8, 8);
    let (mut main_a, mut output_a) = crate::trigger::automation::pair();
    let (mut main_b, mut output_b) = crate::trigger::automation::pair();
    call(
        &mut a,
        &attachment_a,
        1,
        "setParameter",
        json!(["drums.level", 0.125, 1]),
    );
    call(
        &mut b,
        &attachment_b,
        1,
        "setParameter",
        json!(["drums.level", 0.375, 1]),
    );
    main_a.service(&mut a);
    main_b.service(&mut b);
    let a_expected = [
        Begin { parameter_index: 2 },
        Value {
            parameter_index: 2,
            value: 0.5,
        },
        End { parameter_index: 2 },
    ];
    let b_expected = [
        Begin { parameter_index: 2 },
        Value {
            parameter_index: 2,
            value: 1.5,
        },
        End { parameter_index: 2 },
    ];
    assert_eq!(output_a.deliver(|_| false), 0);
    let mut accepted_b = Vec::new();
    assert_eq!(
        output_b.deliver(|event| {
            accepted_b.push(event);
            true
        }),
        3
    );
    main_b.service(&mut b);
    assert_eq!(accepted_b, b_expected);
    assert!(!main_b.pending());
    assert_eq!(a.pending_automation().collect::<Vec<_>>(), a_expected);
    assert_eq!(b.pending_automation().count(), 0);
    audio_a.flush_parameters(&[]).unwrap();
    audio_b.flush_parameters(&[]).unwrap();
    assert_eq!(a.physical_value(2), Some(0.5));
    assert_eq!(b.physical_value(2), Some(1.5));
    let mut accepted_a = Vec::new();
    output_a.deliver(|event| {
        accepted_a.push(event);
        true
    });
    main_a.service(&mut a);
    assert_eq!(accepted_a, a_expected);
    assert!(!main_a.pending());
    assert_eq!(
        output_b.deliver(|_| panic!("empty independent output must not call host")),
        0
    );
    assert_eq!(b.pending_automation().count(), 0);
    assert_eq!(b.physical_value(2), Some(1.5));
}
#[test]
fn empty_output_calls_no_host_and_real_clap_output_projects_global_physical_values() {
    use clack_plugin::{
        events::{
            event_types::{ParamGestureBeginEvent, ParamGestureEndEvent, ParamValueEvent},
            io::EventBuffer,
        },
        prelude::*,
    };
    let (_package, mut controller, _audio, attachment) = controller(8, 8);
    let (mut mirror, mut output) = crate::trigger::automation::pair();
    assert_eq!(
        output.deliver(|_| panic!("empty output cannot invoke host")),
        0
    );
    call(
        &mut controller,
        &attachment,
        1,
        "setParameter",
        json!(["drums.level", 0.125, 1]),
    );
    mirror.service(&mut controller);
    let mut buffer = EventBuffer::with_capacity(64);
    assert_eq!(
        crate::trigger::deliver_automation(&mut output, &mut buffer.as_output()),
        3
    );
    assert_eq!(buffer.len(), 3);
    assert_eq!(
        buffer[0]
            .as_event::<ParamGestureBeginEvent>()
            .unwrap()
            .param_id(),
        Some(ClapId::new(3))
    );
    let value = buffer[1].as_event::<ParamValueEvent>().unwrap();
    assert_eq!(value.param_id(), Some(ClapId::new(3)));
    assert_eq!(value.value(), 0.5);
    assert!(value.pckn().matches_all());
    assert_eq!(
        buffer[2]
            .as_event::<ParamGestureEndEvent>()
            .unwrap()
            .param_id(),
        Some(ClapId::new(3))
    );
    for event in buffer.iter() {
        assert_eq!(event.header().time(), 0);
    }
    mirror.service(&mut controller);
    assert!(!mirror.pending());
}

fn configured_core(events: usize) -> (tempfile::TempDir, std::rc::Rc<super::Core>) {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    let (package, controller, audio, _attachment) = controller(events, events);
    let (automation, output) = super::super::automation::pair();
    let core = Rc::new(super::Core {
        controller: RefCell::new(controller),
        parked: Cell::new(Some(audio)),
        automation: RefCell::new(automation),
        parked_output: Cell::new(Some(output)),
        resource_root: package.path().to_owned(),
        plugin_path: PathBuf::new(),
        gui: RefCell::new(super::EditorState::default()),
    });
    (package, core)
}

#[test]
fn view_independent_service_has_one_weak_source_and_final_cancellation() {
    use std::rc::Rc;
    let (_package, core) = configured_core(8);
    let context = gtk::glib::MainContext::default();
    let _guard = context.acquire().unwrap();
    assert!(
        core.gui.borrow().source.is_none(),
        "configuration alone must not register a GLib source"
    );
    core.ensure_service_source();
    let source = context
        .find_source_by_id(core.gui.borrow().source.as_ref().unwrap())
        .unwrap();
    let id = core.gui.borrow().source.as_ref().unwrap().as_raw();
    core.ensure_service_source();
    assert_eq!(core.gui.borrow().source.as_ref().unwrap().as_raw(), id);
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(20);
    while std::time::Instant::now() < deadline {
        context.iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    core.destroy_editor();
    assert!(
        !source.is_destroyed(),
        "disposable view destruction retains the instance source"
    );
    core.cancel_service_source();
    assert!(source.is_destroyed());
    core.cancel_service_source(); // A successful cancellation is never repeated.
    core.ensure_service_source();
    let final_source = context
        .find_source_by_id(core.gui.borrow().source.as_ref().unwrap())
        .unwrap();
    let weak = Rc::downgrade(&core);
    drop(core);
    assert!(
        weak.upgrade().is_none(),
        "source callback must not retain the Core"
    );
    assert!(
        final_source.is_destroyed(),
        "final owner cancels before releasing Core resources"
    );
}

#[test]
fn detached_feedback_service_is_bounded_and_defers_busy_owners_without_consuming() {
    use dandrum_trigger_host::runtime::ControlEvent;
    let (_package, core) = configured_core(128);
    let index = core.controller.borrow().parameters()[2].root_index;
    let mut audio = core.parked.take().unwrap();
    audio
        .flush_parameters(&[0.5, 1.0, 1.5].map(|value| ControlEvent::Parameter { index, value }))
        .unwrap();
    {
        let _busy = core.gui.borrow_mut();
        core.service_editor();
    }
    assert_eq!(
        core.controller
            .borrow_mut()
            .take_feedback()
            .unwrap()
            .parameters[0]
            .value,
        0.125,
        "busy view owner defers feedback consumption"
    );
    {
        let _busy = core.controller.borrow_mut();
        core.service_editor();
    }
    assert_eq!(
        core.controller
            .borrow_mut()
            .take_feedback()
            .unwrap()
            .parameters[0]
            .value,
        0.25,
        "busy controller defers feedback consumption"
    );
    core.service_editor();
    assert!(
        core.controller.borrow_mut().take_feedback().is_none(),
        "detached view does not leave feedback blocking the bounded producer"
    );
    let values = (0..128)
        .map(|i| ControlEvent::Parameter {
            index,
            value: f64::from(i) / 32.0,
        })
        .collect::<Vec<_>>();
    audio.flush_parameters(&values).unwrap();
    core.parked.set(Some(audio));
    core.service_editor();
    let next = core.controller.borrow_mut().take_feedback().unwrap();
    assert_eq!(next.sequence, 0);
    assert_eq!(
        next.parameters[0].value, 0.5,
        "one service consumes exactly the first64 of an available128 feedback events"
    );
    core.service_editor();
    assert!(core.controller.borrow_mut().take_feedback().is_none());
}

#[test]
fn mapping_dispatch_rejects_stale_and_stopped_worker_without_changing_history() {
    let (_package, mut controller, _audio, attachment) = controller(8, 8);
    let editor = super::EditorState::default();
    editor.start_worker(&mut controller).unwrap();
    let mapping =
        |revision: &str| json!(["19", attachment, "7001", revision, "kit", "kick", 60, 60]);
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            1,
            "setInstrumentZoneKeyRange",
            mapping("9")
        )["error"]["code"],
        "staleRequest"
    );
    controller.shutdown_session_worker().unwrap();
    assert_eq!(
        call(
            &mut controller,
            &attachment,
            2,
            "setInstrumentZoneKeyRange",
            mapping("0")
        )["error"]["code"],
        "commandRejected"
    );
    let snapshot = call(
        &mut controller,
        &attachment,
        3,
        "getSamplerSession",
        json!([]),
    );
    assert_eq!(snapshot["result"]["connection"]["revision"], "0");
    assert_eq!(
        snapshot["result"]["connection"]["history"],
        json!({"undoCount":0,"redoCount":0})
    );
}

#[test]
fn native_acquisition_rejects_nonregular_paths_and_reads_exact_os_bytes() {
    use dandrum_trigger_host::samples::{SampleCatalog, SampleError};
    use std::os::unix::{ffi::OsStrExt, fs::symlink};
    let directory = tempfile::tempdir().unwrap();
    let ordinary = directory.path().join("ordinary.wav");
    let mut encoded = Vec::new();
    dandrum_engine::wav::write_wav_stereo_i16(&mut encoded, 32_000, &[-0.75, 0.25], &[-0.25, 0.5])
        .unwrap();
    std::fs::write(&ordinary, &encoded).unwrap();
    let exact = directory
        .path()
        .join(std::ffi::OsStr::from_bytes(b"selected-\xff.wav"));
    std::fs::write(&exact, &encoded).unwrap();
    let mut catalog = SampleCatalog::new();
    for (path, expected_name) in [
        (&ordinary, "ordinary.wav"),
        (&exact, "selected-\u{fffd}.wav"),
    ] {
        let files =
            super::native_sample_files(vec![path.clone()]).expect("open the exact regular OS path");
        let job = catalog.begin(files).unwrap();
        let selected = catalog.accept(job.prepare()).unwrap().unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].name, expected_name);
        assert_eq!(selected[0].sample.encoded_bytes(), encoded);
        assert_eq!(selected[0].sample.sample().frame_count(), 2);
        assert_eq!(selected[0].sample.sample().sample_rate_hz(), 32_000);
    }
    let link = directory.path().join("final-symlink.wav");
    symlink(&ordinary, &link).unwrap();
    for path in [link, directory.path().to_owned()] {
        assert!(matches!(
            super::native_sample_files(vec![path]),
            Err(SampleError::InvalidFile)
        ));
    }
    assert!(
        matches!(
            super::native_sample_files(vec![ordinary.clone(), directory.path().to_owned()]),
            Err(SampleError::InvalidFile)
        ),
        "one nonregular member rejects the whole opened batch"
    );
    assert!(matches!(
        super::native_sample_files(vec![directory.path().join("absent"); 17]),
        Err(SampleError::FileCount)
    ));
    assert!(
        super::native_sample_files(vec![ordinary]).is_ok(),
        "rejection does not poison a later valid open"
    );
}

#[test]
fn native_acquisition_rejects_fifo_without_waiting_for_a_writer() {
    use std::{
        os::unix::ffi::OsStrExt,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let directory = tempfile::tempdir().unwrap();
    let fifo = directory.path().join("not-a-sample.wav");
    let marker = directory.path().join("fifo-rejected");
    let fifo_c = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
    // SAFETY: the NUL-terminated owned pathname stays live and names only a
    // fresh FIFO in this test's exclusive temporary directory.
    assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) }, 0);
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "trigger::gui::dispatch_tests::native_file_fifo_child",
            "--exact",
            "--ignored",
            "--test-threads=1",
        ])
        .env("DANDRUM_TEST_NATIVE_FIFO", &fifo)
        .env("DANDRUM_TEST_NATIVE_FIFO_RESULT", &marker)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    let result = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            other => {
                // This exact owned child has no descendants. Reap it before
                // reporting timeout/error, even if opening a FIFO blocked.
                let _ = child.kill();
                child.wait().expect("reap the owned FIFO boundary process");
                break Err(format!(
                    "FIFO acquisition failed to finish without a writer: {other:?}"
                ));
            }
        }
    };
    assert!(result.expect("bounded native FIFO rejection").success());
    assert_eq!(
        std::fs::read(&marker).unwrap(),
        b"rejected without writer",
        "a zero-test child exit cannot count as acquisition evidence"
    );
}

#[test]
#[ignore = "only the owned FIFO boundary parent may launch this exact child"]
fn native_file_fifo_child() {
    use dandrum_trigger_host::samples::SampleError;
    use std::os::unix::fs::FileTypeExt;
    let fifo =
        PathBuf::from(std::env::var_os("DANDRUM_TEST_NATIVE_FIFO").expect("owned FIFO parent"));
    let marker = PathBuf::from(
        std::env::var_os("DANDRUM_TEST_NATIVE_FIFO_RESULT").expect("owned FIFO result marker"),
    );
    assert!(
        std::fs::symlink_metadata(&fifo)
            .unwrap()
            .file_type()
            .is_fifo()
    );
    assert!(matches!(
        super::native_sample_files(vec![fifo]),
        Err(SampleError::InvalidFile)
    ));
    std::fs::write(marker, b"rejected without writer").unwrap();
}
