use dandrum_trigger_host::parameters::ParameterController;
use dandrum_trigger_host::runtime::{ControlEvent, TimedEvent};
use dandrum_trigger_host::{
    ADVANCED_DRUM_KIT_DOCUMENT, ADVANCED_DRUM_KIT_SAMPLE, TriggerHostConfiguration,
};
use std::{num::NonZeroU64, path::PathBuf};

const VALUES: [(&str, f64); 23] = [
    ("drums.pitch_ratio", 1.5),
    ("drums.start_offset", 0.03125),
    ("drums.level", 0.5),
    ("drums.pan", -0.25),
    ("drums.variation", 0.4375),
    ("drums.kick.pitch_ratio", 2.0),
    ("drums.kick.start_offset", 0.09375),
    ("drums.kick.level", 0.75),
    ("drums.kick.pan", 0.25),
    ("drums.snare.pitch_ratio", 2.5),
    ("drums.snare.start_offset", 0.15625),
    ("drums.snare.level", 1.25),
    ("drums.snare.pan", -0.375),
    ("drums.snare.variation", 0.5625),
    ("drums.closed_hat.pitch_ratio", 3.0),
    ("drums.closed_hat.start_offset", 0.21875),
    ("drums.closed_hat.level", 1.75),
    ("drums.closed_hat.pan", -0.5),
    ("drums.open_hat.pitch_ratio", 3.5),
    ("drums.open_hat.start_offset", 0.28125),
    ("drums.open_hat.level", 2.25),
    ("drums.open_hat.pan", -0.625),
    ("drums.open_hat.variation", 0.6875),
];
fn configuration() -> TriggerHostConfiguration {
    TriggerHostConfiguration {
        sample_rate_hz: 48_000,
        max_block_frames: 16,
    }
}
fn package() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("Trigger.resources");
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
    (directory, root)
}
fn open(
    root: &std::path::Path,
) -> (
    ParameterController,
    dandrum_trigger_host::runtime::RuntimeAudio,
) {
    ParameterController::open(root, configuration(), NonZeroU64::new(1).unwrap(), 32, 8).unwrap()
}
#[test]
fn native_state_restores_changed_mapping_all_named_physical_values_and_two_signed_frames_into_a_fresh_instance()
 {
    let (_source_package, source_root) = package();
    let path = source_root.join(ADVANCED_DRUM_KIT_DOCUMENT);
    let changed = std::fs::read_to_string(&path).unwrap().replacen(
        "key_range: [36, 36]",
        "key_range: [60, 60]",
        1,
    );
    std::fs::write(path, &changed).unwrap();
    // This PCM oracle reads the packaged WAV directly, independently of DSP/state code.
    let wav = std::fs::read(source_root.join(ADVANCED_DRUM_KIT_SAMPLE)).unwrap();
    assert_eq!(&wav[36..40], b"data");
    for (frame, word) in [
        (1499, -12734i16),
        (1500, -12698),
        (1502, -12661),
        (1503, -12621),
    ] {
        let start = 44 + frame * 2;
        assert_eq!(i16::from_le_bytes([wav[start], wav[start + 1]]), word);
    }
    let (source, mut source_audio) = open(&source_root);
    let events: Vec<_> = VALUES
        .iter()
        .enumerate()
        .map(|(index, (id, value))| {
            let descriptor = source.parameters().iter().find(|p| p.id == *id).unwrap();
            assert_eq!(
                descriptor.root_index, index,
                "independent alias/root binding for {id}"
            );
            assert_ne!(*value, descriptor.physical_default);
            TimedEvent {
                frame_offset: 0,
                event: ControlEvent::Parameter {
                    index,
                    value: *value,
                },
            }
        })
        .collect();
    source_audio
        .process_events(&mut [0.0; 16], &mut [0.0; 16], &events)
        .unwrap();
    let mut saved = Vec::new();
    source.save_state(&mut saved).unwrap();
    let wire: serde_json::Value = serde_json::from_slice(&saved).unwrap();
    assert_eq!(
        (
            wire["version"].as_u64(),
            wire["product"].as_str(),
            wire["resource"].as_str()
        ),
        (
            Some(2),
            Some("org.dandrum.trigger"),
            Some("dandrum.trigger.advanced-drums.v1")
        )
    );
    assert_eq!(wire["document"].as_str(), Some(changed.as_str()));
    assert_eq!(wire["parameters"].as_array().unwrap().len(), VALUES.len());
    for (id, value) in VALUES {
        assert_eq!(
            wire["parameters"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["id"] == id)
                .unwrap()["value"]
                .as_f64(),
            Some(value)
        );
    }
    drop(source_audio);
    drop(source);
    let (_fresh_package, fresh_root) = package();
    let (mut fresh, mut audio) = open(&fresh_root);
    fresh.load_state(&mut saved.as_slice()).unwrap();
    let accepted = fresh.session().owner_snapshot();
    assert_eq!(accepted.yaml, changed);
    assert_eq!((accepted.undo_len, accepted.redo_len), (0, 0));
    assert_eq!(fresh.parameter_state().generation, 2);
    // Native save/load also works before any disposable editor attachment exists.
    let mut restored = Vec::new();
    fresh.save_state(&mut restored).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&restored).unwrap(),
        wire
    );
    let (mut left, mut right) = ([1.0; 16], [1.0; 16]);
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
    assert_eq!((left, right), ([0.0; 16], [0.0; 16]));
    audio
        .process_events(
            &mut left,
            &mut right,
            &[TimedEvent {
                frame_offset: 5,
                event: ControlEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                },
            }],
        )
        .unwrap();
    assert_eq!((&left[..5], &right[..5]), (&[0.0; 5][..], &[0.0; 5][..]));
    // Position1499.875 then1502.875 (pitch1.5*2); PCM/32768, gain.5*.75, pan-.25+.25.
    assert_eq!((left[5], right[5]), (-0.145_368_58, -0.145_368_58));
    assert_eq!((left[6], right[6]), (-0.144_493_1, -0.144_493_1));
}

fn parameter_request(
    controller: &mut ParameterController,
    attachment: &str,
    request_id: u64,
    value: f64,
    generation: u64,
) {
    let envelope = serde_json::json!({"version":1,"attachmentId":attachment,"requestId":request_id.to_string(),"method":"setParameter","args":["drums.level",value,generation]}).to_string();
    let dandrum_trigger_host::protocol::Admission::New(request) =
        controller.admit(&envelope).unwrap()
    else {
        panic!("new request")
    };
    controller.apply(&request).unwrap();
}
fn acknowledge_automation(controller: &mut ParameterController) {
    while controller.peek_automation().is_some() {
        assert!(controller.acknowledge_automation());
    }
}
#[test]
fn successful_restore_discards_old_pending_parameter_work_and_feedback_but_keeps_later_activity() {
    let (_package, root) = package();
    let (mut controller, mut audio) = open(&root);
    let editor = controller.attach_editor().unwrap();
    let mut saved = Vec::new();
    controller.save_state(&mut saved).unwrap();
    let mut state: serde_json::Value = serde_json::from_slice(&saved).unwrap();
    state["document"] = state["document"]
        .as_str()
        .unwrap()
        .replacen("key_range: [36, 36]", "key_range: [60, 60]", 1)
        .into();
    state["parameters"][2]["value"] = 0.5.into();
    let saved = serde_json::to_vec(&state).unwrap();
    audio
        .process_events(
            &mut [0.0; 16],
            &mut [0.0; 16],
            &[TimedEvent {
                frame_offset: 3,
                event: ControlEvent::Parameter {
                    index: 2,
                    value: 0.25,
                },
            }; 32],
        )
        .unwrap();
    parameter_request(&mut controller, &editor.attachment_id, 1, 0.75, 1);
    acknowledge_automation(&mut controller); // The host already received a balanced gesture.
    controller.load_state(&mut saved.as_slice()).unwrap();
    assert!(
        controller.take_feedback().is_none(),
        "pre-restore feedback cannot acquire the new generation"
    );
    assert_eq!(
        (
            controller.parameter_state().generation,
            controller.parameter_state().sequence
        ),
        (2, 0)
    );
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
    assert_eq!(
        (left[0], right[0]),
        (-0.25, -0.25),
        "old queued UI value cannot overwrite restored scalars"
    );
    assert!(controller.take_feedback().is_none());
    audio.reset();
    parameter_request(&mut controller, &editor.attachment_id, 2, 0.375, 2);
    acknowledge_automation(&mut controller);
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
    assert_eq!((left[0], right[0]), (-0.75, -0.75));
    let feedback = controller.take_feedback().unwrap();
    assert_eq!(
        (
            feedback.generation,
            feedback.sequence,
            feedback.parameters[0].value
        ),
        (2, 1, 0.375)
    );
}

#[test]
fn native_save_reads_a_complete_scalar_vector_while_audio_automates_without_silencing_callbacks() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    };
    let (_package, root) = package();
    let (controller, mut audio) = open(&root);
    let stop = Arc::new(AtomicBool::new(false));
    let writer_stop = stop.clone();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (start_tx, start_rx) = mpsc::channel();
    let completed = Arc::new(AtomicU64::new(0));
    let writer_completed = completed.clone();
    let writer = std::thread::spawn(move || {
        let mut events: Vec<_> = (0..23)
            .map(|index| TimedEvent {
                frame_offset: 0,
                event: ControlEvent::Parameter {
                    index,
                    value: 0.125,
                },
            })
            .collect();
        let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
        audio
            .process_events(&mut left, &mut right, &events)
            .unwrap();
        ready_tx.send(()).unwrap();
        start_rx.recv().unwrap();
        let mut blocks = 0;
        while blocks < 2_000 && !writer_stop.load(Ordering::Relaxed) {
            let value = if blocks % 2 == 0 { 0.25 } else { 0.125 };
            for event in &mut events {
                if let ControlEvent::Parameter {
                    value: ref mut destination,
                    ..
                } = event.event
                {
                    *destination = value;
                }
            }
            let report = audio
                .process_events(&mut left, &mut right, &events)
                .unwrap();
            assert!(
                !report.admission_closed,
                "ordinary saves must never close callback admission"
            );
            blocks += 1;
            writer_completed.fetch_add(1, Ordering::Release);
            std::thread::yield_now();
        }
        blocks
    });
    ready_rx.recv().unwrap();
    let mut mixed = None;
    let before = completed.load(Ordering::Acquire);
    start_tx.send(()).unwrap();
    for _ in 0..100 {
        let mut saved = Vec::new();
        controller.save_state(&mut saved).unwrap();
        let state: serde_json::Value = serde_json::from_slice(&saved).unwrap();
        let parameters = state["parameters"].as_array().unwrap();
        let value = parameters[0]["value"].as_f64().unwrap();
        if ![0.125, 0.25].contains(&value)
            || !parameters
                .iter()
                .all(|parameter| parameter["value"].as_f64() == Some(value))
        {
            mixed = Some(parameters.clone());
            break;
        }
    }
    let after = completed.load(Ordering::Acquire);
    stop.store(true, Ordering::Relaxed);
    assert!(writer.join().unwrap() > 0);
    assert!(
        after > before,
        "actual callback blocks must complete during the save interval"
    );
    assert!(
        mixed.is_none(),
        "save must observe all23 values from one completed block: {mixed:?}"
    );
}

#[test]
fn restore_rejects_existing_but_unrecognized_sample_resources_before_publication() {
    let (_package, root) = package();
    std::fs::copy(
        root.join(ADVANCED_DRUM_KIT_SAMPLE),
        root.join("instrument/assets/alternate.wav"),
    )
    .unwrap();
    let (mut controller, mut audio) = open(&root);
    let mut saved = Vec::new();
    controller.save_state(&mut saved).unwrap();
    let original: serde_json::Value = serde_json::from_slice(&saved).unwrap();
    let before = controller.session().owner_snapshot();
    let document = original["document"].as_str().unwrap();
    for yaml in [
        document.replace(
            "path: assets/advanced-drums.wav",
            "path: assets/alternate.wav",
        ),
        document.replace("path: assets/advanced-drums.wav", &format!("path: {}",root.join(ADVANCED_DRUM_KIT_SAMPLE).display())),
        document.replace("path: assets/advanced-drums.wav", "path: examples/patches/assets/advanced-drums.wav"),
        document.replace("- { id: midi, type: midi_input }", "- { id: midi, type: midi_input }\n  - { id: extra, type: sampler, static: { sample: { kind: sample, path: assets/alternate.wav } } }"),
        document
            .replace("id: drums\n", "id: impostor\n")
            .replace("region: drums.", "region: impostor."),
        format!(
            "static_params:\n  - {{ name: other, type: resource, resource_kind: sample, default: {{ kind: sample, path: assets/alternate.wav }} }}\n{document}"
        ),
    ] {
        let mut invalid = original.clone();
        invalid["document"] = yaml.into();
        let bytes = serde_json::to_vec(&invalid).unwrap();
        assert_eq!(
            controller.load_state(&mut bytes.as_slice()),
            Err(dandrum_trigger_host::state::StateError::Invalid),
            "the resource policy rejects another identity/path before engine preparation"
        );
        assert_eq!(controller.session().owner_snapshot(), before);
        let mut after = Vec::new();
        controller.save_state(&mut after).unwrap();
        assert_eq!(after, saved);
    }
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
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
}

fn saved(controller: &ParameterController) -> Vec<u8> {
    let mut bytes = Vec::new();
    controller.save_state(&mut bytes).unwrap();
    bytes
}
fn candidate(controller: &ParameterController) -> serde_json::Value {
    let mut state: serde_json::Value = serde_json::from_slice(&saved(controller)).unwrap();
    state["document"] = state["document"]
        .as_str()
        .unwrap()
        .replacen("key_range: [36, 36]", "key_range: [60, 60]", 1)
        .into();
    for parameter in state["parameters"].as_array_mut().unwrap() {
        parameter["value"] = VALUES
            .iter()
            .find(|(id, _)| parameter["id"] == *id)
            .unwrap()
            .1
            .into();
    }
    state
}
#[test]
fn invalid_states_and_failing_streams_preserve_every_accepted_field_and_pending_feedback_and_commands()
 {
    use dandrum_trigger_host::state::StateError;
    let (_package, root) = package();
    let (mut controller, mut audio) = open(&root);
    let editor = controller.attach_editor().unwrap();
    audio
        .process_events(
            &mut [0.0; 16],
            &mut [0.0; 16],
            &[TimedEvent {
                frame_offset: 3,
                event: ControlEvent::Parameter {
                    index: 2,
                    value: 0.75,
                },
            }],
        )
        .unwrap();
    parameter_request(&mut controller, &editor.attachment_id, 1, 0.125, 1);
    acknowledge_automation(&mut controller);
    let before = controller.session().owner_snapshot();
    let parameters = controller.parameter_state();
    let before_save = saved(&controller);
    let valid = candidate(&controller);
    let mut invalids = Vec::new();
    for (field, value) in [
        ("version", serde_json::json!(1)),
        ("version", serde_json::json!(3)),
        ("product", serde_json::json!("foreign")),
        ("resource", serde_json::json!("foreign")),
        ("surprise", serde_json::json!(true)),
        ("document", serde_json::json!("[invalid YAML")),
    ] {
        let mut invalid = valid.clone();
        invalid[field] = value;
        invalids.push(serde_json::to_vec(&invalid).unwrap());
    }
    for field in ["version", "product", "resource", "document", "parameters"] {
        let mut invalid = valid.clone();
        invalid.as_object_mut().unwrap().remove(field);
        invalids.push(serde_json::to_vec(&invalid).unwrap());
    }
    for value in [
        serde_json::json!(-0.01),
        serde_json::json!(1.01),
        serde_json::Value::Null,
    ] {
        let mut invalid = valid.clone();
        invalid["parameters"][22]["value"] = value;
        invalids.push(serde_json::to_vec(&invalid).unwrap());
    }
    for action in 0..5 {
        let mut invalid = valid.clone();
        let list = invalid["parameters"].as_array_mut().unwrap();
        match action {
            0 => {
                list.pop();
            }
            1 => {
                let duplicate = list[0].clone();
                list.push(duplicate);
            }
            2 => list[22]["id"] = "unknown".into(),
            3 => list[22]["id"] = "drums.level".into(),
            _ => list[22]["unexpected"] = true.into(),
        }
        invalids.push(serde_json::to_vec(&invalid).unwrap());
    }
    let document = valid["document"].as_str().unwrap();
    for yaml in [
        document.replace(
            "name: drums.level, maps_to: level",
            "name: renamed, maps_to: level",
        ),
        document.replace(
            "name: drums.pan, maps_to: pan",
            "name: drums.pan, maps_to: level",
        ),
        document.replace("default: 1, min: 0, max: 4", "default: 1, min: 0, max: 5"),
        document.replace("region: drums.kick", "region: drums.missing"),
        document.replace("type: sample_map_player", "type: unknown_player"),
    ] {
        let mut invalid = valid.clone();
        invalid["document"] = yaml.into();
        invalids.push(serde_json::to_vec(&invalid).unwrap());
    }
    invalids.push(b"{".to_vec());
    invalids.push(vec![b' '; 65_537]);
    let encoded = serde_json::to_string(&valid).unwrap();
    invalids.push(format!("{{\"version\":2,{}", &encoded[1..]).into_bytes());
    for invalid in invalids {
        assert!(controller.load_state(&mut invalid.as_slice()).is_err());
        assert_eq!(controller.session().owner_snapshot(), before);
        assert_eq!(controller.parameter_state(), parameters);
        assert_eq!(saved(&controller), before_save);
        assert!(controller.peek_automation().is_none());
    }
    struct BrokenRead;
    impl std::io::Read for BrokenRead {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("injected stream failure"))
        }
    }
    assert_eq!(controller.load_state(&mut BrokenRead), Err(StateError::Io));
    struct BrokenWrite;
    impl std::io::Write for BrokenWrite {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("injected output failure"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    assert_eq!(controller.save_state(&mut BrokenWrite), Err(StateError::Io));
    assert_eq!(controller.session().owner_snapshot(), before);
    assert_eq!(saved(&controller), before_save);
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
    assert_eq!(
        (left[0], right[0]),
        (-0.25, -0.25),
        "rejected restore retains the queued UI value and original mapping"
    );
    let old = controller.take_feedback().unwrap();
    assert_eq!(
        (
            old.generation,
            old.sequence,
            old.frame_offset,
            old.parameters[0].value
        ),
        (1, 0, 3, 0.1875)
    );
    let applied = controller.take_feedback().unwrap();
    assert_eq!(
        (
            applied.generation,
            applied.sequence,
            applied.parameters[0].value
        ),
        (1, 1, 0.125)
    );
    assert_eq!(controller.session().owner_snapshot(), before);
}
#[test]
fn native_save_and_load_work_at_full_editor_capacity_and_busy_automation_keeps_every_promised_end()
{
    use dandrum_trigger_host::state::StateError;
    let (_package, root) = package();
    let (mut controller, mut audio) = open(&root);
    let state = candidate(&controller);
    let bytes = serde_json::to_vec(&state).unwrap();
    let editors: Vec<_> = (0..16)
        .map(|_| controller.attach_editor().unwrap())
        .collect();
    assert!(controller.attach_editor().is_err());
    assert!(!saved(&controller).is_empty());
    let begin=serde_json::json!({"version":1,"attachmentId":editors[0].attachment_id,"requestId":"1","method":"beginGesture","args":["drums.level",1]}).to_string();
    let dandrum_trigger_host::protocol::Admission::New(request) = controller.admit(&begin).unwrap()
    else {
        panic!("new request")
    };
    controller.apply(&request).unwrap();
    assert_eq!(
        controller.load_state(&mut bytes.as_slice()),
        Err(StateError::Busy)
    );
    assert!(matches!(
        controller.peek_automation(),
        Some(dandrum_trigger_host::parameters::AutomationEvent::Begin { .. })
    ));
    assert!(controller.acknowledge_automation());
    assert_eq!(
        controller.load_state(&mut bytes.as_slice()),
        Err(StateError::Busy),
        "a delivered Begin still owns a reserved End"
    );
    controller.detach_editor(&editors[0].attachment_id).unwrap();
    assert_eq!(
        controller.load_state(&mut bytes.as_slice()),
        Err(StateError::Busy)
    );
    assert!(matches!(
        controller.peek_automation(),
        Some(dandrum_trigger_host::parameters::AutomationEvent::End { .. })
    ));
    assert!(controller.acknowledge_automation());
    assert!(controller.peek_automation().is_none());
    let _replacement_editor = controller.attach_editor().unwrap();
    assert!(
        controller.attach_editor().is_err(),
        "successful native restore runs with all16 editor slots occupied"
    );
    controller.load_state(&mut bytes.as_slice()).unwrap();
    assert_eq!(controller.parameter_state().generation, 2);
    assert!(
        controller.load_state(&mut bytes.as_slice()).is_err(),
        "an unconsumed replacement retains capacity backpressure"
    );
    assert!(
        audio
            .process(&mut [0.0; 16], &mut [0.0; 16])
            .unwrap()
            .swapped
    );
}

#[test]
fn named_parameter_permutations_and_repeated_restores_preserve_an_independent_host() {
    let (_package, root) = package();
    let (mut a, mut audio_a) = open(&root);
    let (mut b, mut audio_b) = open(&root);
    let mut state = candidate(&a);
    state["parameters"].as_array_mut().unwrap().reverse();
    let yaml = state["document"]
        .as_str()
        .unwrap()
        .replace("name: drums.level, maps_to: level", "ALIAS_PLACEHOLDER")
        .replace(
            "name: drums.pan, maps_to: pan",
            "name: drums.level, maps_to: level",
        )
        .replace("ALIAS_PLACEHOLDER", "name: drums.pan, maps_to: pan");
    state["document"] = yaml.into();
    let bytes = serde_json::to_vec(&state).unwrap();
    let b_before = b.session().owner_snapshot();
    let b_values = b.parameter_state();
    let b_saved = saved(&b);
    a.load_state(&mut bytes.as_slice()).unwrap();
    let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
    audio_a
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
    assert_eq!((left[0], right[0]), (-0.145_368_58, -0.145_368_58));
    let roundtrip: serde_json::Value = serde_json::from_slice(&saved(&a)).unwrap();
    for (id, value) in VALUES {
        assert_eq!(
            roundtrip["parameters"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["id"] == id)
                .unwrap()["value"]
                .as_f64(),
            Some(value)
        );
    }
    state["document"] = state["document"]
        .as_str()
        .unwrap()
        .replacen("key_range: [60, 60]", "key_range: [61, 61]", 1)
        .into();
    let second = serde_json::to_vec(&state).unwrap();
    a.load_state(&mut second.as_slice()).unwrap();
    audio_a
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
    assert_eq!((left[0], right[0]), (-0.145_368_58, -0.145_368_58));
    assert_eq!(a.parameter_state().generation, 3);
    assert_eq!(b.session().owner_snapshot(), b_before);
    assert_eq!(b.parameter_state(), b_values);
    assert_eq!(saved(&b), b_saved);
    assert!(b.take_feedback().is_none());
    audio_b
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
    assert_eq!((left[0], right[0]), (-0.5, -0.5));
}

#[test]
fn state_byte_limits_short_reads_and_packaged_sample_failures_are_explicit_and_atomic() {
    use dandrum_trigger_host::state::{STATE_LIMIT, StateError};
    let (_package, root) = package();
    let (mut controller, mut audio) = open(&root);
    let state = candidate(&controller);
    let bytes = serde_json::to_vec(&state).unwrap();
    let before = controller.session().owner_snapshot();
    let original = saved(&controller);
    struct ShortRead<'a> {
        remaining: &'a [u8],
    }
    impl std::io::Read for ShortRead<'_> {
        fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
            let count = out.len().min(self.remaining.len()).min(7);
            out[..count].copy_from_slice(&self.remaining[..count]);
            self.remaining = &self.remaining[count..];
            Ok(count)
        }
    }
    let mut truncated = ShortRead {
        remaining: &bytes[..bytes.len() - 3],
    };
    assert_eq!(
        controller.load_state(&mut truncated),
        Err(StateError::Invalid)
    );
    let sample = root.join(ADVANCED_DRUM_KIT_SAMPLE);
    let hidden = root.join("instrument/assets/hidden.wav");
    std::fs::rename(&sample, &hidden).unwrap();
    assert!(controller.load_state(&mut bytes.as_slice()).is_err());
    assert_eq!(controller.session().owner_snapshot(), before);
    assert_eq!(saved(&controller), original);
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&hidden, &sample).unwrap();
        assert!(controller.load_state(&mut bytes.as_slice()).is_err());
        std::fs::remove_file(&sample).unwrap();
    }
    std::fs::rename(hidden, sample).unwrap();
    let mut exact = bytes.clone();
    exact.resize(STATE_LIMIT as usize, b' ');
    let mut chunks = ShortRead { remaining: &exact };
    controller.load_state(&mut chunks).unwrap();
    assert_eq!(
        controller.session().owner_snapshot().yaml,
        state["document"].as_str().unwrap()
    );
    let accepted = controller.session().owner_snapshot();
    exact.push(b' ');
    assert_eq!(
        controller.load_state(&mut exact.as_slice()),
        Err(StateError::TooLarge)
    );
    assert_eq!(controller.session().owner_snapshot(), accepted);
    audio.process(&mut [0.0; 16], &mut [0.0; 16]).unwrap();
    let (_large_package, large_root) = package();
    let path = large_root.join(ADVANCED_DRUM_KIT_DOCUMENT);
    let yaml = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        path,
        format!("#{}\n{yaml}", "x".repeat(STATE_LIMIT as usize)),
    )
    .unwrap();
    let (large, _audio) = open(&large_root);
    let before = large.session().owner_snapshot();
    let mut output = Vec::new();
    assert_eq!(large.save_state(&mut output), Err(StateError::TooLarge));
    assert!(output.is_empty());
    assert_eq!(large.session().owner_snapshot(), before);
}
#[test]
fn restored_swaps_scalar_writes_and_reset_stamps_allocate_or_free_nothing_on_audio() {
    let (_package, root) = package();
    let (mut controller, mut audio) = open(&root);
    let bytes = serde_json::to_vec(&candidate(&controller)).unwrap();
    controller.load_state(&mut bytes.as_slice()).unwrap();
    let audio = std::thread::spawn(move || {
        let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
        let allocation = allocation_counter::measure(|| {
            let report = audio
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
            assert!(report.swapped);
            assert_eq!((left[0], right[0]), (-0.145_368_58, -0.145_368_58));
            audio.reset();
        });
        assert_eq!(
            (
                allocation.count_total,
                allocation.count_current,
                allocation.bytes_current
            ),
            (0, 0, 0)
        );
        audio
    })
    .join()
    .unwrap();
    drop(audio);
}
