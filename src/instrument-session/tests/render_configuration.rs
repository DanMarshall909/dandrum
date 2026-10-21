use dandrum_engine::PreparedSamplerAssets;
use dandrum_engine::graph_processor::RealtimeGraphProcessor;
use dandrum_engine::kernel::document::load_kernel_patch_str;
use dandrum_engine::patch::RenderSettings;
use dandrum_engine::preparation::{HostBuses, PreparationContext, PreparedKernelInstrument};
use dandrum_instrument_session::{
    ActivationHost, ActivationLease, CommitError, InstrumentSession, MappingAdmission,
    MappingRequest, Publication, RequestId, SessionError, SessionSnapshot, SetZoneKeyRange,
};

fn fixture() -> (tempfile::TempDir, InstrumentSession, HostBuses) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("assets")).unwrap();
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/patches/assets/advanced-drums.wav"),
        directory.path().join("assets/advanced-drums.wav"),
    )
    .unwrap();
    let yaml = include_str!("../../../examples/patches/advanced-drum-kit.yaml");
    let patch = load_kernel_patch_str(yaml).unwrap();
    let buses = patch.preset_surface().parameters().iter().fold(
        HostBuses::new().with_output("master", 2),
        |buses, parameter| buses.with_input(parameter.port_name(), 1),
    );
    let session = InstrumentSession::new_for_planar_host(
        yaml,
        PreparationContext::new(directory.path(), 48_000),
        RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 16,
            duration_frames: 16,
        },
        buses.clone(),
    )
    .unwrap();
    (directory, session, buses)
}
fn target() -> RenderSettings {
    RenderSettings {
        sample_rate_hz: 44_100,
        block_size_frames: 32,
        duration_frames: 32,
    }
}
fn pending(
    session: &mut InstrumentSession,
    attachment: &dandrum_instrument_session::Attachment,
    snapshot: &SessionSnapshot,
) -> MappingRequest {
    let request = MappingRequest {
        instance: snapshot.instance.clone(),
        request_id: RequestId(1),
        expected_revision: snapshot.revision,
        intent: SetZoneKeyRange {
            map_id: "kit".into(),
            zone_id: "kick".into(),
            key_range: [60, 60],
        },
    };
    assert!(matches!(
        session.admit_mapping(attachment, request.clone()).unwrap(),
        MappingAdmission::Admitted(_)
    ));
    request
}
fn assert_original_audio(session: &InstrumentSession) {
    let mut processor = RealtimeGraphProcessor::from_compiled_patch(
        session.prepared().compiled_patch().clone(),
        48_000.0,
        &PreparedSamplerAssets::empty(),
        16,
    );
    let inputs: Vec<_> = session
        .prepared()
        .root_port_metadata()
        .iter()
        .filter(|port| port.direction() == dandrum_engine::graph::PortDirection::Input)
        .map(|port| vec![vec![port.control_default().unwrap().default() as f32; 16]])
        .collect();
    let mut outputs = vec![vec![vec![0.0; 16]; 2]];
    assert!(processor.try_note_on_at(36, 100, 0));
    assert_eq!(processor.render_root_buses(&inputs, &mut outputs), 16);
    assert_eq!((outputs[0][0][0], outputs[0][1][0]), (-0.5, -0.5));
}
struct RejectingHost;
enum Unpublished {}
impl ActivationLease for Unpublished {
    fn publish(self, _: Publication<'_>) {
        match self {}
    }
}
impl ActivationHost for RejectingHost {
    type Error = &'static str;
    type Lease<'a> = Unpublished;
    fn activate<'a>(
        &'a mut self,
        _: &PreparedKernelInstrument,
        _: &SessionSnapshot,
    ) -> Result<Self::Lease<'a>, Self::Error> {
        Err("activation cannot publish in this rejection test")
    }
}

#[test]
fn render_preparation_while_a_native_job_is_pending_preserves_accepted_audio() {
    let (_directory, mut session, buses) = fixture();
    let (attachment, before) = session.attach().unwrap();
    let request = pending(&mut session, &attachment, &before);
    assert_eq!(
        session
            .prepare_render_configuration_owned(target(), buses)
            .err(),
        Some(SessionError::Busy)
    );
    assert!(matches!(
        session.admit_mapping(&attachment, request).unwrap(),
        MappingAdmission::Pending
    ));
    assert_eq!(session.owner_snapshot(), before);
    assert_original_audio(&session);
}

#[test]
fn failed_render_resource_preparation_preserves_the_loaded_instrument() {
    let (directory, mut session, buses) = fixture();
    let (_, before) = session.attach().unwrap();
    std::fs::remove_file(directory.path().join("assets/advanced-drums.wav")).unwrap();
    assert!(matches!(
        session.prepare_render_configuration_owned(target(), buses),
        Err(SessionError::PreparationFailed(_))
    ));
    assert_eq!(session.owner_snapshot(), before);
    assert_original_audio(&session);
}

#[test]
fn render_commit_cannot_consume_or_publish_over_an_admitted_native_job() {
    let (_directory, mut session, buses) = fixture();
    let (attachment, before) = session.attach().unwrap();
    let candidate = session
        .prepare_render_configuration_owned(target(), buses)
        .unwrap();
    let request = pending(&mut session, &attachment, &before);
    assert_eq!(
        session.commit_render_configuration(candidate, &mut RejectingHost),
        Err(CommitError::Session(SessionError::Busy))
    );
    assert!(matches!(
        session.admit_mapping(&attachment, request).unwrap(),
        MappingAdmission::Pending
    ));
    assert_eq!(session.owner_snapshot(), before);
    assert_original_audio(&session);
}
