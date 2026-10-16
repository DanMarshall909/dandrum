use dandrum_engine::PreparedSamplerAssets;
use dandrum_engine::graph::PortDirection;
use dandrum_engine::graph_processor::RealtimeGraphProcessor;
use dandrum_engine::kernel::{ChannelCount, document::load_kernel_patch_str};
use dandrum_engine::patch::RenderSettings;
use dandrum_engine::preparation::{
    PreparationContext, PreparedKernelInstrument, prepare_kernel_patch_with_context,
};
use std::path::PathBuf;

fn fixture() -> (String, PreparationContext, RenderSettings) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/patches/advanced-drum-kit.yaml");
    (
        std::fs::read_to_string(&path).unwrap(),
        PreparationContext::new(path.parent().unwrap(), 48_000),
        RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 16,
            duration_frames: 16,
        },
    )
}

fn render_note(prepared: &PreparedKernelInstrument, note: u8) -> Vec<Vec<f32>> {
    let metadata = prepared.root_port_metadata();
    let inputs = metadata
        .iter()
        .filter(|p| p.direction() == PortDirection::Input)
        .map(|p| {
            let ChannelCount::Literal(channels) = p.channels() else {
                panic!("prepared channel count")
            };
            vec![
                vec![p.control_default().map_or(0.0, |c| c.default()) as f32; 16];
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
    assert!(runtime.try_note_on_at(note, 100, 0));
    let mut output = vec![vec![vec![0.0; 16]; 2]];
    assert_eq!(runtime.render_root_buses(&inputs, &mut output), 16);
    output.remove(0)
}

#[test]
fn existing_sampler_baseline_uses_real_assets_and_signed_key_mapping() {
    let (yaml, context, settings) = fixture();
    let patch = load_kernel_patch_str(&yaml).unwrap();
    let prepared = prepare_kernel_patch_with_context(&patch, &settings, &context).unwrap();
    let hit = render_note(&prepared, 36);
    assert_eq!(hit[0][0], -0.5);
    assert_eq!(hit[1][0], -0.5);
    assert_eq!(render_note(&prepared, 60), vec![vec![0.0; 16]; 2]);
    assert_eq!(patch.sample_assets().sample_maps[0].id, "kit");
    assert_eq!(
        patch.sample_assets().sample_maps[0].zones[0].id.as_deref(),
        Some("kick")
    );
}

use dandrum_instrument_session::{InstrumentSession, SessionError};

#[test]
fn accepted_document_survives_detach_and_reattach_without_reloading_samples() {
    let (yaml, _, settings) = fixture();
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/patches/assets/advanced-drums.wav");
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("assets")).unwrap();
    let owned_sample = directory.path().join("assets/advanced-drums.wav");
    std::fs::copy(source, &owned_sample).unwrap();
    let mut session = InstrumentSession::new(
        &yaml,
        PreparationContext::new(directory.path(), 48_000),
        settings,
    )
    .unwrap();
    let (old, before) = session.attach().unwrap();
    assert_eq!(before.revision, 0);
    assert_eq!((before.undo_len, before.redo_len), (0, 0));
    assert_eq!(before.yaml, yaml);
    assert_eq!(render_note(session.prepared(), 36)[0][0], -0.5);
    session.detach(&old).unwrap();
    assert_eq!(
        session.snapshot(&old).unwrap_err(),
        SessionError::InvalidAttachment
    );
    std::fs::remove_file(&owned_sample).unwrap();
    let (fresh, after) = session.attach().unwrap();
    assert_eq!(before, after);
    assert_eq!(session.snapshot(&fresh).unwrap(), after);
    assert_eq!(render_note(session.prepared(), 36)[0][0], -0.5);
    assert_eq!(render_note(session.prepared(), 60), vec![vec![0.0; 16]; 2]);
    let mut copied = after;
    copied.yaml.clear();
    assert_eq!(
        session.snapshot(&fresh).unwrap().yaml,
        yaml,
        "snapshot owns its copy"
    );
}

#[test]
fn foreign_and_retired_connections_cannot_read_another_session() {
    let (yaml, context, settings) = fixture();
    let mut first = InstrumentSession::new(&yaml, context.clone(), settings.clone()).unwrap();
    let mut second = InstrumentSession::new(&yaml, context, settings).unwrap();
    let (a, one) = first.attach().unwrap();
    let (b, two) = second.attach().unwrap();
    assert_ne!(one.instance, two.instance);
    assert_eq!(
        first.snapshot(&b).unwrap_err(),
        SessionError::InvalidAttachment
    );
    assert_eq!(
        second.snapshot(&a).unwrap_err(),
        SessionError::InvalidAttachment
    );
    assert_eq!(first.detach(&b), Err(SessionError::InvalidAttachment));
    let retained = a.clone();
    first.detach(&a).unwrap();
    assert_eq!(
        first.snapshot(&retained).unwrap_err(),
        SessionError::InvalidAttachment
    );
    assert_eq!(
        first.detach(&retained),
        Err(SessionError::InvalidAttachment)
    );
    assert_eq!(second.snapshot(&b).unwrap(), two);
    assert_eq!(render_note(second.prepared(), 36)[0][0], -0.5);
}

#[test]
fn attachment_limit_reclaims_dropped_and_explicitly_retired_connections() {
    let (yaml, context, settings) = fixture();
    let mut session = InstrumentSession::new(&yaml, context, settings).unwrap();
    let mut connections = (0..16)
        .map(|_| session.attach().unwrap().0)
        .collect::<Vec<_>>();
    assert!(matches!(
        session.attach(),
        Err(SessionError::AttachmentLimit)
    ));
    let retired = connections.pop().unwrap();
    session.detach(&retired).unwrap();
    let (replacement, _) = session.attach().unwrap();
    assert!(matches!(
        session.attach(),
        Err(SessionError::AttachmentLimit)
    ));
    drop(connections.pop().unwrap());
    session.attach().unwrap();
    assert!(session.snapshot(&replacement).is_ok());
    assert_eq!(
        session.snapshot(&retired).unwrap_err(),
        SessionError::InvalidAttachment
    );
}

#[test]
fn unusable_documents_or_missing_samples_cannot_create_accepted_sessions() {
    let (yaml, context, settings) = fixture();
    assert!(matches!(
        InstrumentSession::new("not: a patch", context.clone(), settings.clone()),
        Err(SessionError::InvalidDocument(_))
    ));
    let missing = yaml.replace(
        "assets/advanced-drums.wav",
        "assets/missing-session-sample.wav",
    );
    assert!(matches!(
        InstrumentSession::new(&missing, context, settings),
        Err(SessionError::PreparationFailed(_))
    ));
}
