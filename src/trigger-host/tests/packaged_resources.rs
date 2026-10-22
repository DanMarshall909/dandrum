use dandrum_engine::PreparedSamplerAssets;
use dandrum_engine::graph::PortDirection;
use dandrum_engine::graph_processor::RealtimeGraphProcessor;
use dandrum_engine::kernel::ChannelCount;
use dandrum_engine::preparation::PreparedKernelInstrument;
use dandrum_instrument_session::SessionError;
use dandrum_trigger_host::{
    ADVANCED_DRUM_KIT_DOCUMENT, ADVANCED_DRUM_KIT_SAMPLE, PackagedResourceError, PackagedResources,
    TriggerHost, TriggerHostConfiguration, TriggerHostError,
};
use std::path::{Path, PathBuf};

fn copied_package() -> (tempfile::TempDir, PathBuf) {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("installed/DandrumTrigger.resources");
    let instrument = root.join("instrument");
    std::fs::create_dir_all(instrument.join("assets")).unwrap();
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::copy(
        repository.join("examples/patches/advanced-drum-kit.yaml"),
        root.join(ADVANCED_DRUM_KIT_DOCUMENT),
    )
    .unwrap();
    std::fs::copy(
        repository.join("examples/patches/assets/advanced-drums.wav"),
        root.join(ADVANCED_DRUM_KIT_SAMPLE),
    )
    .unwrap();
    assert!(!root.starts_with(repository.canonicalize().unwrap()));
    (temporary, root)
}

fn configuration() -> TriggerHostConfiguration {
    TriggerHostConfiguration {
        sample_rate_hz: dandrum_trigger_host::SampleRate::new(48_000.0).unwrap(),
        max_block_frames: 16,
    }
}

fn render_note(prepared: &PreparedKernelInstrument, note: u8) -> Vec<Vec<f32>> {
    let inputs = prepared
        .root_port_metadata()
        .iter()
        .filter(|port| port.direction() == PortDirection::Input)
        .map(|port| {
            let ChannelCount::Literal(channels) = port.channels() else {
                panic!("prepared channel count must be concrete")
            };
            vec![
                vec![port.control_default().map_or(0.0, |value| value.default()) as f32; 16];
                *channels as usize
            ]
        })
        .collect::<Vec<_>>();
    let mut runtime = RealtimeGraphProcessor::from_compiled_patch(
        prepared.compiled_patch().clone(),
        48_000.0,
        &PreparedSamplerAssets::empty(),
        16,
    );
    let mut output = vec![vec![vec![0.0; 16]; 2]];
    assert!(runtime.try_note_on_at(note, 100, 0));
    assert_eq!(runtime.render_root_buses(&inputs, &mut output), 16);
    output.remove(0)
}

#[test]
fn packaged_host_binds_public_controls_to_the_rendered_sample_without_editing_history() {
    use dandrum_trigger_host::runtime::{ControlEvent, PreparedRuntime, runtime_handoff};
    let (_temporary, root) = copied_package();
    let mut host = TriggerHost::open(&root, configuration()).unwrap();
    let (attachment, before) = host.session_mut().attach().unwrap();
    let prepared = PreparedRuntime::new(host.session().prepared(), configuration()).unwrap();
    let (mut control, mut audio) = runtime_handoff(prepared, 4).unwrap();
    let level = control.parameter_index("level").unwrap();
    control
        .try_send(ControlEvent::Parameter {
            index: level,
            value: 0.25,
        })
        .unwrap();
    control
        .try_send(ControlEvent::NoteOn {
            note: 36,
            velocity: 100,
        })
        .unwrap();
    let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
    audio.process(&mut left, &mut right).unwrap();
    assert_eq!(left[0], -0.125);
    assert_eq!(right[0], -0.125);
    assert_eq!(control.parameter_value(level), Some(0.25));
    assert_eq!(host.session().snapshot(&attachment).unwrap(), before);
}

#[test]
fn copied_package_owns_the_real_kit_and_renders_known_signed_midi_36_output() {
    let (_temporary, root) = copied_package();
    let mut host = TriggerHost::open(&root, configuration()).unwrap();
    assert_eq!(host.resource_root(), root.canonicalize().unwrap());

    let (attachment, snapshot) = host.session_mut().attach().unwrap();
    assert_eq!(snapshot.revision, 0);
    assert!(snapshot.yaml.contains("id: dandrum.advanced-drum-kit"));
    assert_eq!(host.session().snapshot(&attachment).unwrap(), snapshot);

    let hit = render_note(host.session().prepared(), 36);
    assert_eq!(hit[0][0], -0.5);
    assert_eq!(hit[1][0], -0.5);
    assert_eq!(
        render_note(host.session().prepared(), 60),
        vec![vec![0.0; 16]; 2]
    );
}

#[test]
fn invalid_or_missing_packaged_resources_preserve_the_accepted_session_and_sound() {
    let (_temporary, root) = copied_package();
    let mut host = TriggerHost::open(&root, configuration()).unwrap();
    let (attachment, accepted) = host.session_mut().attach().unwrap();
    let document = root.join(ADVANCED_DRUM_KIT_DOCUMENT);
    let original_document = std::fs::read_to_string(&document).unwrap();

    std::fs::write(&document, "metadata: [").unwrap();
    assert!(matches!(
        host.prepare_resource_reload(&attachment),
        Err(TriggerHostError::Session(SessionError::InvalidDocument(_)))
    ));
    assert_eq!(host.session().snapshot(&attachment).unwrap(), accepted);
    assert_eq!(render_note(host.session().prepared(), 36)[0][0], -0.5);

    std::fs::write(&document, original_document).unwrap();
    std::fs::remove_file(root.join(ADVANCED_DRUM_KIT_SAMPLE)).unwrap();
    assert!(matches!(
        host.prepare_resource_reload(&attachment),
        Err(TriggerHostError::Resource(PackagedResourceError::Missing {
            relative: ADVANCED_DRUM_KIT_SAMPLE
        }))
    ));
    assert_eq!(host.session().snapshot(&attachment).unwrap(), accepted);
    assert_eq!(render_note(host.session().prepared(), 36)[1][0], -0.5);
}

#[test]
fn unavailable_roots_nonregular_files_and_unreadable_documents_are_explicit() {
    let temporary = tempfile::tempdir().unwrap();
    let missing = temporary.path().join("missing.resources");
    assert!(matches!(
        PackagedResources::open(&missing),
        Err(PackagedResourceError::MissingRoot)
    ));

    let file_root = temporary.path().join("file.resources");
    std::fs::write(&file_root, b"not a directory").unwrap();
    assert!(matches!(
        PackagedResources::open(&file_root),
        Err(PackagedResourceError::InvalidRoot)
    ));

    let (_package, root) = copied_package();
    let document = root.join(ADVANCED_DRUM_KIT_DOCUMENT);
    std::fs::remove_file(&document).unwrap();
    std::fs::create_dir(&document).unwrap();
    assert!(matches!(
        PackagedResources::open(&root),
        Err(PackagedResourceError::Invalid {
            relative: ADVANCED_DRUM_KIT_DOCUMENT
        })
    ));
    std::fs::remove_dir(&document).unwrap();
    std::fs::write(&document, [0xff, 0xfe]).unwrap();
    assert!(matches!(
        PackagedResources::open(&root),
        Err(PackagedResourceError::Read {
            relative: ADVANCED_DRUM_KIT_DOCUMENT,
            ..
        })
    ));

    let (_package, root) = copied_package();
    std::fs::write(root.join(ADVANCED_DRUM_KIT_DOCUMENT), "metadata: [").unwrap();
    assert!(matches!(
        TriggerHost::open(&root, configuration()),
        Err(TriggerHostError::Session(SessionError::InvalidDocument(_)))
    ));
}

#[cfg(unix)]
#[test]
fn symbolic_resource_files_are_rejected_instead_of_followed() {
    use std::os::unix::fs::symlink;

    let (_temporary, root) = copied_package();
    let sample = root.join(ADVANCED_DRUM_KIT_SAMPLE);
    std::fs::remove_file(&sample).unwrap();
    let repository_sample = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/patches/assets/advanced-drums.wav");
    symlink(repository_sample, &sample).unwrap();
    assert!(matches!(
        PackagedResources::open(&root),
        Err(PackagedResourceError::Invalid {
            relative: ADVANCED_DRUM_KIT_SAMPLE
        })
    ));
}

#[test]
fn resource_and_session_errors_have_stable_actionable_diagnostics() {
    let cases = [
        (
            PackagedResourceError::MissingRoot,
            "Trigger resource root does not exist",
        ),
        (
            PackagedResourceError::InvalidRoot,
            "Trigger resource root is not a directory",
        ),
        (
            PackagedResourceError::Missing {
                relative: ADVANCED_DRUM_KIT_SAMPLE,
            },
            "missing Trigger resource instrument/assets/advanced-drums.wav",
        ),
        (
            PackagedResourceError::Invalid {
                relative: ADVANCED_DRUM_KIT_SAMPLE,
            },
            "Trigger resource is not a regular local file: instrument/assets/advanced-drums.wav",
        ),
        (
            PackagedResourceError::Read {
                relative: ADVANCED_DRUM_KIT_DOCUMENT,
                message: "invalid UTF-8".into(),
            },
            "cannot read Trigger resource instrument/advanced-drum-kit.yaml: invalid UTF-8",
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
    }
    assert_eq!(
        TriggerHostError::Resource(PackagedResourceError::MissingRoot).to_string(),
        "Trigger resource root does not exist"
    );
    assert_eq!(
        TriggerHostError::Session(SessionError::InvalidDocument("bad YAML".into())).to_string(),
        "cannot prepare Trigger session: InvalidDocument(\"bad YAML\")"
    );
}

#[cfg(unix)]
#[test]
fn permission_denials_are_reported_without_searching_elsewhere() {
    use std::os::unix::fs::PermissionsExt;

    let temporary = tempfile::tempdir().unwrap();
    let locked_parent = temporary.path().join("locked");
    std::fs::create_dir(&locked_parent).unwrap();
    std::fs::set_permissions(&locked_parent, std::fs::Permissions::from_mode(0o000)).unwrap();
    let unavailable = PackagedResources::open(locked_parent.join("child"));
    std::fs::set_permissions(&locked_parent, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(matches!(
        unavailable,
        Err(PackagedResourceError::InvalidRoot)
    ));

    let (_temporary, root) = copied_package();
    let instrument = root.join("instrument");
    std::fs::set_permissions(&instrument, std::fs::Permissions::from_mode(0o000)).unwrap();
    let unavailable = PackagedResources::open(&root);
    std::fs::set_permissions(&instrument, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(matches!(
        unavailable,
        Err(PackagedResourceError::Invalid {
            relative: ADVANCED_DRUM_KIT_SAMPLE
        })
    ));
}

#[test]
fn missing_installed_sample_never_falls_back_to_the_checkout() {
    let (_temporary, root) = copied_package();
    std::fs::remove_file(root.join(ADVANCED_DRUM_KIT_SAMPLE)).unwrap();
    let result = TriggerHost::open(&root, configuration());
    assert!(matches!(
        result,
        Err(TriggerHostError::Resource(PackagedResourceError::Missing {
            relative: ADVANCED_DRUM_KIT_SAMPLE
        }))
    ));
    assert!(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/patches/assets/advanced-drums.wav")
            .is_file()
    );
}

#[test]
fn a_regular_corrupt_sample_fails_preparation_without_checkout_fallback_and_can_be_repaired() {
    let (_temporary, root) = copied_package();
    let sample = root.join(ADVANCED_DRUM_KIT_SAMPLE);
    let original = std::fs::read(&sample).unwrap();
    std::fs::write(&sample, b"not a WAV file").unwrap();
    assert!(
        PackagedResources::open(&root).is_ok(),
        "files are present and regular"
    );
    assert!(matches!(
        TriggerHost::open(&root, configuration()),
        Err(TriggerHostError::Session(SessionError::PreparationFailed(
            _
        )))
    ));
    std::fs::write(sample, original).unwrap();
    let host = TriggerHost::open(&root, configuration()).unwrap();
    let rendered = render_note(host.session().prepared(), 36);
    assert_eq!((rendered[0][0], rendered[1][0]), (-0.5, -0.5));
}
