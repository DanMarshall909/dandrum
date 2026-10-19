use dandrum_engine::PreparedSamplerAssets;
use dandrum_engine::graph::PortDirection;
use dandrum_engine::graph_processor::RealtimeGraphProcessor;
use dandrum_engine::kernel::{ChannelCount, document::load_kernel_patch_str};
use dandrum_engine::patch::RenderSettings;
use dandrum_engine::preparation::{
    HostBuses, PreparationContext, PreparedKernelInstrument, prepare_kernel_patch_with_context,
};
use dandrum_instrument_session::{
    ACTIVE_REQUEST_LIMIT, ATTACHMENT_LIMIT, CONNECTION_CONTRACT_VERSION, HISTORY_LIMIT,
    InstrumentSession, OUTCOME_LIMIT, SessionError, SetZoneKeyRange,
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

fn owned_sample() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("assets")).unwrap();
    let path = directory.path().join("assets/advanced-drums.wav");
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/patches/assets/advanced-drums.wav");
    std::fs::copy(source, &path).unwrap();
    (directory, path)
}

fn planar_host_buses(yaml: &str) -> HostBuses {
    let patch = load_kernel_patch_str(yaml).unwrap();
    patch.preset_surface().parameters().iter().fold(
        HostBuses::new().with_output("master", 2),
        |buses, parameter| buses.with_input(parameter.port_name(), 1),
    )
}

struct NoteRenderer {
    runtime: RealtimeGraphProcessor,
    inputs: Vec<Vec<Vec<f32>>>,
    output: Vec<Vec<Vec<f32>>>,
}
impl NoteRenderer {
    fn new(prepared: &PreparedKernelInstrument) -> Self {
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
        let runtime = RealtimeGraphProcessor::from_compiled_patch(
            prepared.compiled_patch().clone(),
            48_000.0,
            &PreparedSamplerAssets::empty(),
            16,
        );
        Self {
            runtime,
            inputs,
            output: vec![vec![vec![0.0; 16]; 2]],
        }
    }
    fn render(&mut self, note: u8) -> Vec<Vec<f32>> {
        // Independent single-hit oracle; continuing-voice safety belongs to the
        // concrete host lifecycle tests rather than this mapping protocol fixture.
        self.runtime.reset();
        assert!(self.runtime.try_note_on_at(note, 100, 0));
        assert_eq!(
            self.runtime
                .render_root_buses(&self.inputs, &mut self.output),
            16
        );
        self.output[0].clone()
    }
}
fn render_note(prepared: &PreparedKernelInstrument, note: u8) -> Vec<Vec<f32>> {
    NoteRenderer::new(prepared).render(note)
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

#[test]
fn connection_contract_version_and_resource_bounds_are_stable() {
    assert_eq!(CONNECTION_CONTRACT_VERSION, 1);
    assert_eq!(ATTACHMENT_LIMIT, 16);
    assert_eq!(HISTORY_LIMIT, 100);
    assert_eq!(OUTCOME_LIMIT, 128);
    assert_eq!(ACTIVE_REQUEST_LIMIT, 1);
}

#[test]
fn mapping_candidate_derives_inverse_and_prepares_only_the_requested_zone_change() {
    let (yaml, context, settings) = fixture();
    let mut document: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
    let mut other_map = document["assets"]["sample_maps"][0].clone();
    other_map["id"] = "other".into();
    other_map["zones"][0]["key_range"] = serde_yaml::to_value([12, 12]).unwrap();
    let maps = document["assets"]["sample_maps"].as_sequence_mut().unwrap();
    maps[0]["zones"].as_sequence_mut().unwrap().swap(0, 1);
    maps.insert(0, other_map);
    let yaml = serde_yaml::to_string(&document).unwrap();
    let mut session = InstrumentSession::new(&yaml, context, settings).unwrap();
    let (connection, accepted) = session.attach().unwrap();
    let intent = SetZoneKeyRange {
        map_id: "kit".into(),
        zone_id: "kick".into(),
        key_range: [60, 60],
    };
    let candidate = session.prepare_mapping(&connection, intent).unwrap();
    assert_eq!(candidate.change().map_id, "kit");
    assert_eq!(candidate.change().zone_id, "kick");
    assert_eq!(candidate.change().before, [36, 36]);
    assert_eq!(candidate.change().after, [60, 60]);
    let mut expected: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
    expected["assets"]["sample_maps"][1]["zones"][1]["key_range"] =
        serde_yaml::to_value([60, 60]).unwrap();
    let actual: serde_yaml::Value = serde_yaml::from_str(candidate.yaml()).unwrap();
    assert_eq!(actual, expected, "all unrelated authored values survive");
    assert_eq!(render_note(candidate.prepared(), 60)[0][0], -0.5);
    assert_eq!(render_note(candidate.prepared(), 60)[1][0], -0.5);
    assert_eq!(
        render_note(candidate.prepared(), 36),
        vec![vec![0.0; 16]; 2]
    );
    assert_eq!(session.snapshot(&connection).unwrap(), accepted);
    assert_eq!(render_note(session.prepared(), 36)[0][0], -0.5);
    assert_eq!(render_note(session.prepared(), 60), vec![vec![0.0; 16]; 2]);
}

#[test]
fn invalid_or_ambiguous_mapping_intents_preserve_the_accepted_instrument() {
    let (yaml, context, settings) = fixture();
    let mut session = InstrumentSession::new(&yaml, context.clone(), settings.clone()).unwrap();
    let (connection, before) = session.attach().unwrap();
    for (map_id, zone_id, key_range) in [
        ("kit", "kick", [61, 60]),
        ("kit", "kick", [0, 128]),
        ("kit", "kick", [255, 255]),
        ("missing", "kick", [60, 60]),
        ("kit", "missing", [60, 60]),
        ("", "kick", [60, 60]),
        ("kit", "", [60, 60]),
    ] {
        assert!(
            matches!(
                session.prepare_mapping(
                    &connection,
                    SetZoneKeyRange {
                        map_id: map_id.into(),
                        zone_id: zone_id.into(),
                        key_range,
                    }
                ),
                Err(SessionError::InvalidMapping)
            ),
            "intent {map_id}/{zone_id}/{key_range:?}"
        );
        assert_eq!(session.snapshot(&connection).unwrap(), before);
    }
    let duplicate = yaml.replace("id: snare_soft, region:", "id: kick, region:");
    let mut ambiguous = InstrumentSession::new(&duplicate, context, settings).unwrap();
    let (attachment, accepted) = ambiguous.attach().unwrap();
    assert!(matches!(
        ambiguous.prepare_mapping(
            &attachment,
            SetZoneKeyRange {
                map_id: "kit".into(),
                zone_id: "kick".into(),
                key_range: [60, 60],
            }
        ),
        Err(SessionError::InvalidMapping)
    ));
    assert_eq!(ambiguous.snapshot(&attachment).unwrap(), accepted);
    assert_eq!(render_note(session.prepared(), 36)[0][0], -0.5);
    assert_eq!(render_note(ambiguous.prepared(), 36)[0][0], -0.5);
}

#[test]
fn mapping_admission_rejects_foreign_and_retired_connections() {
    let (yaml, context, settings) = fixture();
    let mut first = InstrumentSession::new(&yaml, context.clone(), settings.clone()).unwrap();
    let mut second = InstrumentSession::new(&yaml, context, settings).unwrap();
    let (a, accepted) = first.attach().unwrap();
    let (b, _) = second.attach().unwrap();
    let intent = SetZoneKeyRange {
        map_id: "kit".into(),
        zone_id: "kick".into(),
        key_range: [60, 60],
    };
    assert!(matches!(
        first.prepare_mapping(&b, intent.clone()),
        Err(SessionError::InvalidAttachment)
    ));
    assert_admission_error(
        first.admit_mapping(&b, mapping_request(&accepted, 1, 60)),
        SessionError::InvalidAttachment,
    );
    assert_eq!(first.snapshot(&a).unwrap(), accepted);
    first.detach(&a).unwrap();
    assert!(matches!(
        first.prepare_mapping(&a, intent.clone()),
        Err(SessionError::InvalidAttachment)
    ));
    assert_admission_error(
        first.admit_mapping(&a, mapping_request(&accepted, 2, 61)),
        SessionError::InvalidAttachment,
    );
    assert_eq!(render_note(first.prepared(), 36)[0][0], -0.5);
    assert_eq!(render_note(second.prepared(), 36)[0][0], -0.5);
}

#[test]
fn inclusive_midi_boundaries_and_nondefault_inverse_come_from_native_state() {
    let (yaml, context, settings) = fixture();
    let yaml = yaml.replace("key_range: [36, 36]", "key_range: [35, 37]");
    let mut session = InstrumentSession::new(&yaml, context, settings).unwrap();
    let (attachment, accepted) = session.attach().unwrap();
    let candidate = session
        .prepare_mapping(
            &attachment,
            SetZoneKeyRange {
                map_id: "kit".into(),
                zone_id: "kick".into(),
                key_range: [0, 127],
            },
        )
        .unwrap();
    assert_eq!(candidate.change().before, [35, 37]);
    assert_eq!(candidate.change().after, [0, 127]);
    assert_eq!(render_note(candidate.prepared(), 0)[0][0], -0.5);
    assert_eq!(render_note(candidate.prepared(), 127)[1][0], -0.5);
    assert_eq!(session.snapshot(&attachment).unwrap(), accepted);
}

#[test]
fn failed_sample_repreparation_preserves_the_working_document_and_sound() {
    let (yaml, _, settings) = fixture();
    let (directory, owned_sample) = owned_sample();
    let mut session = InstrumentSession::new(
        &yaml,
        PreparationContext::new(directory.path(), 48_000),
        settings,
    )
    .unwrap();
    let (attachment, accepted) = session.attach().unwrap();
    std::fs::remove_file(owned_sample).unwrap();
    assert!(matches!(
        session.prepare_mapping(
            &attachment,
            SetZoneKeyRange {
                map_id: "kit".into(),
                zone_id: "kick".into(),
                key_range: [60, 60],
            }
        ),
        Err(SessionError::PreparationFailed(_))
    ));
    assert_eq!(session.snapshot(&attachment).unwrap(), accepted);
    assert_eq!(render_note(session.prepared(), 36)[0][0], -0.5);
    assert_eq!(render_note(session.prepared(), 60), vec![vec![0.0; 16]; 2]);
}

#[test]
fn documents_without_sample_maps_reject_mapping_without_changing_accepted_state() {
    let (_, context, settings) = fixture();
    let yaml = include_str!("../../../examples/patches/tb303-acid.yaml");
    let mut session = InstrumentSession::new(yaml, context, settings).unwrap();
    let (attachment, accepted) = session.attach().unwrap();
    assert!(matches!(
        session.prepare_mapping(
            &attachment,
            SetZoneKeyRange {
                map_id: "kit".into(),
                zone_id: "kick".into(),
                key_range: [60, 60],
            }
        ),
        Err(SessionError::InvalidMapping)
    ));
    assert_eq!(session.snapshot(&attachment).unwrap(), accepted);
}

#[test]
fn accepted_document_survives_detach_and_reattach_without_reloading_samples() {
    let (yaml, _, settings) = fixture();
    let (directory, owned_sample) = owned_sample();
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

use dandrum_instrument_session::{
    ActivationHost, ActivationLease, CommitError, MappingAdmission, MappingJob, MappingRequest,
    Publication, RequestFailure, RequestId, RequestOutcome, SessionSnapshot,
};
use std::sync::Arc;

// Protocol fixture only: real graph execution, no claim about a concrete host's
// reader synchronization or editor lifecycle. Those require the native tests.
struct RuntimeHost {
    runtime: NoteRenderer,
    published: SessionSnapshot,
    kick_level: f32,
    closed: bool,
    reject: bool,
    attempts: usize,
}
struct RuntimeLease<'a> {
    host: &'a mut RuntimeHost,
    previous: Option<NoteRenderer>,
    next: Option<SessionSnapshot>,
    published: bool,
}
impl ActivationHost for RuntimeHost {
    type Error = &'static str;
    type Lease<'a> = RuntimeLease<'a>;
    fn activate<'a>(
        &'a mut self,
        prepared: &PreparedKernelInstrument,
        next: &SessionSnapshot,
    ) -> Result<Self::Lease<'a>, Self::Error> {
        let mut runtime = NoteRenderer::new(prepared);
        assert!(runtime.runtime.set_numeric_parameter_by_target(
            "player",
            "group_1_level",
            self.kick_level,
        ));
        let next = next.clone();
        self.closed = true;
        let previous = std::mem::replace(&mut self.runtime, runtime);
        self.attempts += 1;
        let lease = RuntimeLease {
            host: self,
            previous: Some(previous),
            next: Some(next),
            published: false,
        };
        if lease.host.reject {
            return Err("activation rejected");
        }
        Ok(lease)
    }
}
impl ActivationLease for RuntimeLease<'_> {
    fn publish(mut self, accepted: Publication<'_>) {
        assert!(
            self.host.closed,
            "lease must hold admission during publication"
        );
        let next = self.next.take().unwrap();
        assert_eq!(accepted.revision, next.revision);
        assert_eq!(accepted.yaml, next.yaml);
        assert_eq!(
            (accepted.undo_len, accepted.redo_len),
            (next.undo_len, next.redo_len)
        );
        assert_eq!(accepted.undo, next.undo.as_ref());
        assert_eq!(accepted.redo, next.redo.as_ref());
        self.host.published = next;
        self.host.closed = false;
        self.published = true;
    }
}
impl Drop for RuntimeLease<'_> {
    fn drop(&mut self) {
        if !self.published {
            self.host.runtime = self.previous.take().unwrap();
            self.host.closed = false;
        }
    }
}
impl RuntimeHost {
    fn new(session: &InstrumentSession, snapshot: &SessionSnapshot) -> Self {
        Self {
            runtime: NoteRenderer::new(session.prepared()),
            published: snapshot.clone(),
            kick_level: 1.0,
            closed: false,
            reject: false,
            attempts: 0,
        }
    }
    fn render(&mut self, note: u8) -> Vec<Vec<f32>> {
        assert!(!self.closed, "audio admission closed");
        self.runtime.render(note)
    }
    fn automate_kick_level(&mut self, value: f32) {
        self.kick_level = value;
        assert!(self.runtime.runtime.set_numeric_parameter_by_target(
            "player",
            "group_1_level",
            value,
        ));
    }
}

#[test]
fn accepted_mapping_and_native_undo_redo_publish_document_history_and_signed_audio() {
    let (yaml, context, settings) = fixture();
    let mut session = InstrumentSession::new(&yaml, context.clone(), settings.clone()).unwrap();
    let mut other = InstrumentSession::new(&yaml, context, settings).unwrap();
    let (old, initial) = session.attach().unwrap();
    let (other_connection, other_initial) = other.attach().unwrap();
    let mut host = RuntimeHost::new(&session, &initial);
    let candidate = session
        .prepare_mapping(
            &old,
            SetZoneKeyRange {
                map_id: "kit".into(),
                zone_id: "kick".into(),
                key_range: [60, 60],
            },
        )
        .unwrap();
    session.detach(&old).unwrap();
    let accepted = session.commit_mapping(candidate, &mut host).unwrap();
    let (fresh, reattached) = session.attach().unwrap();
    assert_eq!(accepted, reattached);
    assert_eq!(session.snapshot(&fresh).unwrap(), accepted);
    assert_eq!(accepted.instance, initial.instance);
    assert_eq!(accepted.revision, 1);
    assert_eq!((accepted.undo_len, accepted.redo_len), (1, 0));
    let entry = accepted.undo.as_ref().unwrap();
    assert_eq!(entry.version, 1);
    assert_eq!(entry.label, "Change Key Range");
    assert_eq!(entry.change.map_id, "kit");
    assert_eq!(entry.change.zone_id, "kick");
    assert_eq!(entry.change.before, [36, 36]);
    assert_eq!(entry.change.after, [60, 60]);
    assert_eq!(host.published, accepted);
    assert_eq!(host.render(60)[0][0], -0.5);
    assert_eq!(host.render(60)[1][0], -0.5);
    assert_eq!(host.render(36), vec![vec![0.0; 16]; 2]);
    assert_eq!(render_note(session.prepared(), 60)[0][0], -0.5);
    let undone = session.undo(&fresh, &mut host).unwrap();
    assert_eq!(undone.revision, 2);
    assert_eq!((undone.undo_len, undone.redo_len), (0, 1));
    assert_eq!(undone.redo, accepted.undo);
    let original: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(
        serde_yaml::from_str::<serde_yaml::Value>(&undone.yaml).unwrap(),
        original
    );
    assert_eq!(host.published, undone);
    assert_eq!(host.render(36)[0][0], -0.5);
    assert_eq!(host.render(60), vec![vec![0.0; 16]; 2]);
    let redone = session.redo(&fresh, &mut host).unwrap();
    assert_eq!(redone.revision, 3);
    assert_eq!((redone.undo_len, redone.redo_len), (1, 0));
    assert_eq!(redone.yaml, accepted.yaml);
    assert_eq!(redone.undo, accepted.undo);
    assert_eq!(host.published, redone);
    assert_eq!(host.render(60)[0][0], -0.5);
    assert_eq!(host.render(36), vec![vec![0.0; 16]; 2]);
    assert_eq!(other.snapshot(&other_connection).unwrap(), other_initial);
    assert_eq!(render_note(other.prepared(), 36)[0][0], -0.5);
    assert_eq!(render_note(other.prepared(), 60), vec![vec![0.0; 16]; 2]);
}

fn kick_mapping(note: u8) -> SetZoneKeyRange {
    SetZoneKeyRange {
        map_id: "kit".into(),
        zone_id: "kick".into(),
        key_range: [note, note],
    }
}

#[test]
fn activation_failure_rolls_back_runtime_document_and_every_history_transition() {
    let (yaml, context, settings) = fixture();
    let mut session = InstrumentSession::new(&yaml, context, settings).unwrap();
    let (connection, initial) = session.attach().unwrap();
    let mut host = RuntimeHost::new(&session, &initial);
    host.reject = true;
    let candidate = session
        .prepare_mapping(&connection, kick_mapping(60))
        .unwrap();
    assert_eq!(
        session.commit_mapping(candidate, &mut host),
        Err(CommitError::Activation("activation rejected"))
    );
    assert_eq!(session.snapshot(&connection).unwrap(), initial);
    assert_eq!(host.published, initial);
    assert_eq!(host.render(36)[0][0], -0.5);
    assert_eq!(host.render(60), vec![vec![0.0; 16]; 2]);
    host.reject = false;
    let candidate = session
        .prepare_mapping(&connection, kick_mapping(60))
        .unwrap();
    let accepted = session.commit_mapping(candidate, &mut host).unwrap();
    host.reject = true;
    assert_eq!(
        session.undo(&connection, &mut host),
        Err(CommitError::Activation("activation rejected"))
    );
    assert_eq!(session.snapshot(&connection).unwrap(), accepted);
    assert_eq!(host.published, accepted);
    assert_eq!(host.render(60)[0][0], -0.5);
    assert_eq!(host.render(36), vec![vec![0.0; 16]; 2]);
    host.reject = false;
    let undone = session.undo(&connection, &mut host).unwrap();
    host.reject = true;
    assert_eq!(
        session.redo(&connection, &mut host),
        Err(CommitError::Activation("activation rejected"))
    );
    assert_eq!(session.snapshot(&connection).unwrap(), undone);
    assert_eq!(host.published, undone);
    assert_eq!(host.render(36)[0][0], -0.5);
    assert_eq!(host.render(60), vec![vec![0.0; 16]; 2]);
}

#[test]
fn empty_history_foreign_connections_and_obsolete_candidates_never_activate() {
    let (yaml, context, settings) = fixture();
    let mut session = InstrumentSession::new(&yaml, context.clone(), settings.clone()).unwrap();
    let mut other = InstrumentSession::new(&yaml, context, settings).unwrap();
    let (connection, initial) = session.attach().unwrap();
    let (foreign, other_initial) = other.attach().unwrap();
    let mut host = RuntimeHost::new(&session, &initial);
    assert_eq!(
        session.undo(&connection, &mut host),
        Err(CommitError::Session(SessionError::NothingToUndo))
    );
    assert_eq!(
        session.redo(&connection, &mut host),
        Err(CommitError::Session(SessionError::NothingToRedo))
    );
    for direction in [true, false] {
        let result = if direction {
            session.undo(&foreign, &mut host)
        } else {
            session.redo(&foreign, &mut host)
        };
        assert_eq!(
            result,
            Err(CommitError::Session(SessionError::InvalidAttachment))
        );
    }
    let foreign_candidate = other.prepare_mapping(&foreign, kick_mapping(61)).unwrap();
    assert_eq!(
        session.commit_mapping(foreign_candidate, &mut host),
        Err(CommitError::Session(SessionError::ForeignCandidate))
    );
    assert_eq!(host.attempts, 0);
    let candidate = session
        .prepare_mapping(&connection, kick_mapping(60))
        .unwrap();
    let obsolete = session
        .prepare_mapping(&connection, kick_mapping(62))
        .unwrap();
    let accepted = session.commit_mapping(candidate, &mut host).unwrap();
    assert_eq!(
        session.commit_mapping(obsolete, &mut host),
        Err(CommitError::Session(SessionError::StaleCandidate))
    );
    session.detach(&connection).unwrap();
    assert_eq!(
        session.undo(&connection, &mut host),
        Err(CommitError::Session(SessionError::InvalidAttachment))
    );
    assert_eq!(
        session.redo(&connection, &mut host),
        Err(CommitError::Session(SessionError::InvalidAttachment))
    );
    assert_eq!(host.attempts, 1);
    let (fresh, state) = session.attach().unwrap();
    assert_eq!(state, accepted);
    assert_eq!(session.snapshot(&fresh).unwrap(), accepted);
    assert_eq!(host.published, accepted);
    assert_eq!(host.render(60)[0][0], -0.5);
    assert_eq!(other.snapshot(&foreign).unwrap(), other_initial);
    assert_eq!(render_note(other.prepared(), 36)[0][0], -0.5);
}

#[test]
fn failed_undo_preparation_preserves_accepted_mapping_and_native_history() {
    let (yaml, _, settings) = fixture();
    let (directory, sample) = owned_sample();
    let mut session = InstrumentSession::new(
        &yaml,
        PreparationContext::new(directory.path(), 48_000),
        settings,
    )
    .unwrap();
    let (connection, initial) = session.attach().unwrap();
    let mut host = RuntimeHost::new(&session, &initial);
    let candidate = session
        .prepare_mapping(&connection, kick_mapping(60))
        .unwrap();
    let accepted = session.commit_mapping(candidate, &mut host).unwrap();
    std::fs::remove_file(&sample).unwrap();
    assert!(matches!(
        session.undo(&connection, &mut host),
        Err(CommitError::Session(SessionError::PreparationFailed(_)))
    ));
    assert_eq!(session.snapshot(&connection).unwrap(), accepted);
    assert_eq!(host.attempts, 1);
    assert_eq!(host.published, accepted);
    assert_eq!(host.render(60)[0][0], -0.5);
    assert_eq!(host.render(36), vec![vec![0.0; 16]; 2]);
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/patches/assets/advanced-drums.wav"),
        &sample,
    )
    .unwrap();
    let undone = session.undo(&connection, &mut host).unwrap();
    std::fs::remove_file(sample).unwrap();
    assert!(matches!(
        session.redo(&connection, &mut host),
        Err(CommitError::Session(SessionError::PreparationFailed(_)))
    ));
    assert_eq!(session.snapshot(&connection).unwrap(), undone);
    assert_eq!(host.attempts, 2);
    assert_eq!(host.published, undone);
    assert_eq!(host.render(36)[0][0], -0.5);
    assert_eq!(host.render(60), vec![vec![0.0; 16]; 2]);
}

fn mapping_request(snapshot: &SessionSnapshot, request_id: u64, note: u8) -> MappingRequest {
    MappingRequest {
        instance: snapshot.instance.clone(),
        request_id: RequestId(request_id),
        expected_revision: snapshot.revision,
        intent: kick_mapping(note),
    }
}

fn admitted(result: Result<MappingAdmission, SessionError>) -> MappingJob {
    match result {
        Ok(MappingAdmission::Admitted(job)) => job,
        Ok(_) => panic!("request should be newly admitted"),
        Err(error) => panic!("request should be admitted: {error:?}"),
    }
}

fn assert_admission_error(result: Result<MappingAdmission, SessionError>, expected: SessionError) {
    match result {
        Err(error) => assert_eq!(error, expected),
        Ok(_) => panic!("request should have been rejected"),
    }
}

fn accepted_outcome(outcome: &RequestOutcome) -> &SessionSnapshot {
    match &outcome.result {
        Ok(snapshot) => snapshot,
        Err(error) => panic!("request should have succeeded: {error:?}"),
    }
}

#[test]
fn admitted_worker_survives_attachment_loss_and_retry_recovers_one_accepted_edit() {
    fn assert_send<T: Send>() {}
    assert_send::<MappingJob>();

    let (yaml, context, settings) = fixture();
    let mut session = InstrumentSession::new(&yaml, context.clone(), settings.clone()).unwrap();
    let mut other = InstrumentSession::new(&yaml, context, settings).unwrap();
    let (connection, initial) = session.attach().unwrap();
    let (other_connection, other_initial) = other.attach().unwrap();
    let mut host = RuntimeHost::new(&session, &initial);
    let request = mapping_request(&initial, 7, 60);
    let direct = session
        .prepare_mapping(&connection, kick_mapping(62))
        .unwrap();

    let job = admitted(session.admit_mapping(&connection, request.clone()));
    assert!(matches!(
        session.admit_mapping(&connection, request.clone()),
        Ok(MappingAdmission::Pending)
    ));
    let mut pending_conflict = request.clone();
    pending_conflict.intent = kick_mapping(63);
    assert_admission_error(
        session.admit_mapping(&connection, pending_conflict),
        SessionError::ConflictingRequest,
    );
    assert_admission_error(
        session.admit_mapping(&connection, mapping_request(&initial, 8, 61)),
        SessionError::Busy,
    );
    assert!(matches!(
        session.prepare_mapping(&connection, kick_mapping(64)),
        Err(SessionError::Busy)
    ));
    assert_eq!(
        session.commit_mapping(direct, &mut host),
        Err(CommitError::Session(SessionError::Busy))
    );
    assert_eq!(
        session.undo(&connection, &mut host),
        Err(CommitError::Session(SessionError::Busy))
    );
    assert_eq!(
        session.redo(&connection, &mut host),
        Err(CommitError::Session(SessionError::Busy))
    );
    assert_eq!(host.attempts, 0);
    session.detach(&connection).unwrap();

    let completion = std::thread::spawn(|| job.prepare()).join().unwrap();
    drop(session.complete_mapping(completion, &mut host).unwrap());
    let (fresh, accepted) = session.attach().unwrap();
    assert_eq!(accepted.revision, 1);
    assert_eq!((accepted.undo_len, accepted.redo_len), (1, 0));
    assert_eq!(host.attempts, 1);
    assert_eq!(host.render(60)[0][0], -0.5);
    assert_eq!(host.render(36), vec![vec![0.0; 16]; 2]);
    assert_eq!(
        session.request_outcome(&connection, RequestId(7)),
        Err(SessionError::InvalidAttachment)
    );
    assert!(
        session
            .request_outcome(&fresh, RequestId(999))
            .unwrap()
            .is_none()
    );

    let recovered = session
        .request_outcome(&fresh, RequestId(7))
        .unwrap()
        .unwrap();
    assert_eq!(accepted_outcome(&recovered), &accepted);
    let retried = match session.admit_mapping(&fresh, request.clone()).unwrap() {
        MappingAdmission::Completed(outcome) => outcome,
        _ => panic!("retry should recover the terminal outcome"),
    };
    assert!(Arc::ptr_eq(&recovered, &retried));
    assert_eq!(host.attempts, 1, "retry must not reactivate");

    let mut conflicting = request;
    conflicting.intent = kick_mapping(61);
    assert_admission_error(
        session.admit_mapping(&fresh, conflicting),
        SessionError::ConflictingRequest,
    );
    assert_admission_error(
        session.admit_mapping(&fresh, mapping_request(&initial, 8, 61)),
        SessionError::StaleRequest,
    );
    assert_admission_error(
        other.admit_mapping(&other_connection, mapping_request(&initial, 9, 61)),
        SessionError::InvalidRequestInstance,
    );
    assert_eq!(session.snapshot(&fresh).unwrap(), accepted);
    assert_eq!(other.snapshot(&other_connection).unwrap(), other_initial);
    assert_eq!(render_note(other.prepared(), 36)[0][0], -0.5);
}

#[test]
fn failed_worker_preparation_is_retained_and_retry_does_not_prepare_or_activate_again() {
    let (yaml, _, settings) = fixture();
    let (directory, sample) = owned_sample();
    let mut session = InstrumentSession::new(
        &yaml,
        PreparationContext::new(directory.path(), 48_000),
        settings,
    )
    .unwrap();
    let (connection, initial) = session.attach().unwrap();
    let mut host = RuntimeHost::new(&session, &initial);
    let request = mapping_request(&initial, 11, 60);
    let job = admitted(session.admit_mapping(&connection, request.clone()));
    session.detach(&connection).unwrap();
    std::fs::remove_file(sample).unwrap();

    let outcome = session.complete_mapping(job.prepare(), &mut host).unwrap();
    assert!(matches!(
        &outcome.result,
        Err(RequestFailure::Session(SessionError::PreparationFailed(_)))
    ));
    assert_eq!(host.attempts, 0);
    let (fresh, current) = session.attach().unwrap();
    assert_eq!(current, initial);
    assert_eq!(host.render(36)[0][0], -0.5);
    assert_eq!(host.render(60), vec![vec![0.0; 16]; 2]);
    let queried = session
        .request_outcome(&fresh, RequestId(11))
        .unwrap()
        .unwrap();
    let retried = match session.admit_mapping(&fresh, request).unwrap() {
        MappingAdmission::Completed(outcome) => outcome,
        _ => panic!("failed request should retain its terminal outcome"),
    };
    assert!(Arc::ptr_eq(&outcome, &queried));
    assert!(Arc::ptr_eq(&outcome, &retried));
    assert_eq!(host.attempts, 0);
}

#[test]
fn explicit_worker_failure_before_or_after_preparation_clears_pending_without_activation() {
    let (yaml, context, settings) = fixture();
    let mut session = InstrumentSession::new(&yaml, context, settings).unwrap();
    let (connection, initial) = session.attach().unwrap();
    let mut host = RuntimeHost::new(&session, &initial);

    let admitted_job =
        admitted(session.admit_mapping(&connection, mapping_request(&initial, 1, 60)));
    let before_preparation = session
        .complete_mapping(
            admitted_job.fail(SessionError::PreparationFailed(
                "cancelled before prepare".into(),
            )),
            &mut host,
        )
        .unwrap();
    assert!(matches!(
        &before_preparation.result,
        Err(RequestFailure::Session(SessionError::PreparationFailed(message)))
            if message == "cancelled before prepare"
    ));

    let prepared_job =
        admitted(session.admit_mapping(&connection, mapping_request(&initial, 2, 61))).prepare();
    let after_preparation = session
        .complete_mapping(
            prepared_job.fail(SessionError::PreparationFailed(
                "cancelled after prepare".into(),
            )),
            &mut host,
        )
        .unwrap();
    assert!(matches!(
        &after_preparation.result,
        Err(RequestFailure::Session(SessionError::PreparationFailed(message)))
            if message == "cancelled after prepare"
    ));
    assert_eq!(session.snapshot(&connection).unwrap(), initial);
    assert_eq!(host.published, initial);
    assert_eq!(host.attempts, 0);
    assert_eq!(host.render(36)[0][0], -0.5);
}

#[test]
fn foreign_worker_completion_cannot_mutate_another_session() {
    let (yaml, context, settings) = fixture();
    let mut first = InstrumentSession::new(&yaml, context.clone(), settings.clone()).unwrap();
    let mut second = InstrumentSession::new(&yaml, context, settings).unwrap();
    let (first_connection, first_initial) = first.attach().unwrap();
    let (_, second_initial) = second.attach().unwrap();
    let mut first_host = RuntimeHost::new(&first, &first_initial);
    let mut second_host = RuntimeHost::new(&second, &second_initial);
    let job =
        admitted(first.admit_mapping(&first_connection, mapping_request(&first_initial, 31, 60)));

    let route_error = second
        .complete_mapping(job.prepare(), &mut second_host)
        .unwrap_err();
    assert_eq!(route_error.reason(), &SessionError::InvalidJob);
    assert_eq!(
        format!("{route_error:?}"),
        "CompletionRouteError { reason: InvalidJob, .. }"
    );
    assert_eq!(second_host.published, second_initial);
    assert_eq!(second_host.attempts, 0);
    assert_eq!(second_host.render(36)[0][0], -0.5);
    assert_eq!(second_host.render(60), vec![vec![0.0; 16]; 2]);

    let accepted = first
        .complete_mapping(route_error.into_completion(), &mut first_host)
        .unwrap();
    assert_eq!(accepted_outcome(&accepted).revision, 1);
    assert_eq!(accepted_outcome(&accepted).undo_len, 1);
    assert_eq!(first_host.attempts, 1);
    assert_eq!(first_host.render(60)[0][0], -0.5);
    assert_eq!(first_host.render(36), vec![vec![0.0; 16]; 2]);
    assert_eq!(second_host.published, second_initial);
}

#[test]
fn activation_failure_is_terminal_and_clears_the_pending_slot_for_new_work() {
    let (yaml, context, settings) = fixture();
    let mut session = InstrumentSession::new(&yaml, context, settings).unwrap();
    let (connection, initial) = session.attach().unwrap();
    let mut host = RuntimeHost::new(&session, &initial);
    host.reject = true;
    let failed_request = mapping_request(&initial, 21, 60);
    let job = admitted(session.admit_mapping(&connection, failed_request.clone()));

    let failed = session.complete_mapping(job.prepare(), &mut host).unwrap();
    assert_eq!(
        failed.result,
        Err(RequestFailure::Activation("activation rejected".into()))
    );
    assert_eq!(session.snapshot(&connection).unwrap(), initial);
    assert_eq!(host.attempts, 1);
    host.reject = false;
    let retried = match session.admit_mapping(&connection, failed_request).unwrap() {
        MappingAdmission::Completed(outcome) => outcome,
        _ => panic!("activation failure should be terminal"),
    };
    assert!(Arc::ptr_eq(&failed, &retried));
    assert_eq!(host.attempts, 1);

    let next_request = mapping_request(&initial, 22, 61);
    let next_job = admitted(session.admit_mapping(&connection, next_request));
    let accepted = session
        .complete_mapping(next_job.prepare(), &mut host)
        .unwrap();
    assert_eq!(accepted_outcome(&accepted).revision, 1);
    assert_eq!(host.attempts, 2);
    assert_eq!(host.render(61)[0][0], -0.5);
    assert_eq!(host.render(36), vec![vec![0.0; 16]; 2]);
    assert!(Arc::ptr_eq(
        &failed,
        &session
            .request_outcome(&connection, RequestId(21))
            .unwrap()
            .unwrap()
    ));
}

#[test]
fn terminal_request_outcomes_are_capped_at_one_hundred_twenty_eight() {
    let (yaml, _, settings) = fixture();
    let (directory, sample) = owned_sample();
    let mut session = InstrumentSession::new(
        &yaml,
        PreparationContext::new(directory.path(), 48_000),
        settings,
    )
    .unwrap();
    let (connection, initial) = session.attach().unwrap();
    let mut host = RuntimeHost::new(&session, &initial);
    std::fs::remove_file(&sample).unwrap();

    for request_id in 0..129 {
        let request = mapping_request(&initial, request_id, 60);
        let job = admitted(session.admit_mapping(&connection, request));
        let outcome = session.complete_mapping(job.prepare(), &mut host).unwrap();
        assert!(matches!(
            &outcome.result,
            Err(RequestFailure::Session(SessionError::PreparationFailed(_)))
        ));
    }

    assert!(
        session
            .request_outcome(&connection, RequestId(0))
            .unwrap()
            .is_none()
    );
    assert!(
        session
            .request_outcome(&connection, RequestId(1))
            .unwrap()
            .is_some()
    );
    assert!(
        session
            .request_outcome(&connection, RequestId(128))
            .unwrap()
            .is_some()
    );
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/patches/assets/advanced-drums.wav"),
        &sample,
    )
    .unwrap();
    assert_admission_error(
        session.admit_mapping(&connection, mapping_request(&initial, 0, 60)),
        SessionError::RetiredRequest,
    );
    assert_admission_error(
        session.admit_mapping(&connection, mapping_request(&initial, 0, 61)),
        SessionError::RetiredRequest,
    );
    assert_eq!(session.snapshot(&connection).unwrap(), initial);
    assert_eq!(host.attempts, 0);
    assert_eq!(host.render(36)[0][0], -0.5);
    assert_eq!(host.render(60), vec![vec![0.0; 16]; 2]);
}

fn yaml_with_kick_mapping(yaml: &str, note: u8) -> String {
    let mut document: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    let map = document["assets"]["sample_maps"]
        .as_sequence_mut()
        .unwrap()
        .iter_mut()
        .find(|map| map["id"].as_str() == Some("kit"))
        .unwrap();
    let zone = map["zones"]
        .as_sequence_mut()
        .unwrap()
        .iter_mut()
        .find(|zone| zone["id"].as_str() == Some("kick"))
        .unwrap();
    zone["key_range"] = serde_yaml::to_value([note, note]).unwrap();
    serde_yaml::to_string(&document).unwrap()
}

#[test]
fn history_retains_one_hundred_entries_and_branching_preserves_then_retires_redo() {
    let (yaml, context, settings) = fixture();
    let mut session = InstrumentSession::new(&yaml, context, settings).unwrap();
    let (connection, initial) = session.attach().unwrap();
    let mut host = RuntimeHost::new(&session, &initial);

    let mut accepted = initial;
    for edit in 0..101 {
        let note = if edit % 2 == 0 { 60 } else { 61 };
        let candidate = session
            .prepare_mapping(&connection, kick_mapping(note))
            .unwrap();
        accepted = session.commit_mapping(candidate, &mut host).unwrap();
    }
    assert_eq!(accepted.revision, 101);
    assert_eq!((accepted.undo_len, accepted.redo_len), (100, 0));
    assert_eq!(accepted.undo.as_ref().unwrap().change.before, [61, 61]);
    assert_eq!(accepted.undo.as_ref().unwrap().change.after, [60, 60]);
    assert_eq!(host.render(60)[0][0], -0.5);

    let undone = session.undo(&connection, &mut host).unwrap();
    assert_eq!((undone.undo_len, undone.redo_len), (99, 1));
    assert_eq!(host.render(61)[0][0], -0.5);
    let branch = session
        .prepare_mapping(&connection, kick_mapping(62))
        .unwrap();
    host.reject = true;
    assert_eq!(
        session.commit_mapping(branch, &mut host),
        Err(CommitError::Activation("activation rejected"))
    );
    assert_eq!(session.snapshot(&connection).unwrap(), undone);
    assert_eq!(host.render(61)[0][0], -0.5);

    host.reject = false;
    let branch = session
        .prepare_mapping(&connection, kick_mapping(62))
        .unwrap();
    let branched = session.commit_mapping(branch, &mut host).unwrap();
    assert_eq!((branched.undo_len, branched.redo_len), (100, 0));
    assert_eq!(host.render(62)[0][0], -0.5);
    assert_eq!(
        session.redo(&connection, &mut host),
        Err(CommitError::Session(SessionError::NothingToRedo))
    );

    let mut fully_undone = branched;
    for _ in 0..100 {
        fully_undone = session.undo(&connection, &mut host).unwrap();
    }
    assert_eq!((fully_undone.undo_len, fully_undone.redo_len), (0, 100));
    assert_eq!(host.render(60)[0][0], -0.5);
    assert_eq!(host.render(36), vec![vec![0.0; 16]; 2]);
    assert_eq!(
        session.undo(&connection, &mut host),
        Err(CommitError::Session(SessionError::NothingToUndo))
    );
}

#[test]
fn accepted_document_replacement_resets_history_and_requests_but_preserves_host_automation() {
    let (yaml, context, settings) = fixture();
    let mut session = InstrumentSession::new(&yaml, context, settings).unwrap();
    let (connection, initial) = session.attach().unwrap();
    let mut host = RuntimeHost::new(&session, &initial);
    let request = mapping_request(&initial, 7, 60);
    let job = admitted(session.admit_mapping(&connection, request.clone()));
    let terminal = session.complete_mapping(job.prepare(), &mut host).unwrap();
    let accepted = accepted_outcome(&terminal).clone();
    assert_eq!((accepted.undo_len, accepted.redo_len), (1, 0));

    host.automate_kick_level(0.5);
    assert_eq!(session.snapshot(&connection).unwrap(), accepted);
    assert_eq!(host.render(60)[0][0], -0.25);

    let replacement_yaml = yaml_with_kick_mapping(&yaml, 64);
    let replacement = session
        .prepare_document_replacement(&connection, &replacement_yaml)
        .unwrap();
    host.reject = true;
    assert_eq!(
        session.commit_document_replacement(replacement, &mut host),
        Err(CommitError::Activation("activation rejected"))
    );
    assert_eq!(session.snapshot(&connection).unwrap(), accepted);
    assert!(
        session
            .request_outcome(&connection, RequestId(7))
            .unwrap()
            .is_some()
    );
    assert_eq!(host.render(60)[0][0], -0.25);

    host.reject = false;
    let replacement = session
        .prepare_document_replacement(&connection, &replacement_yaml)
        .unwrap();
    let replaced = session
        .commit_document_replacement(replacement, &mut host)
        .unwrap();
    assert_eq!(replaced.revision, 2);
    assert_eq!(replaced.yaml, replacement_yaml);
    assert_eq!((replaced.undo_len, replaced.redo_len), (0, 0));
    assert_eq!(host.published, replaced);
    assert_eq!(host.render(64)[0][0], -0.25);
    assert_eq!(host.render(60), vec![vec![0.0; 16]; 2]);
    assert!(
        session
            .request_outcome(&connection, RequestId(7))
            .unwrap()
            .is_none()
    );
    assert_admission_error(
        session.admit_mapping(&connection, request),
        SessionError::RetiredRequest,
    );
    assert_admission_error(
        session.admit_mapping(&connection, mapping_request(&accepted, 8, 65)),
        SessionError::StaleRequest,
    );
    assert_eq!(
        session.undo(&connection, &mut host),
        Err(CommitError::Session(SessionError::NothingToUndo))
    );

    let current_request = mapping_request(&replaced, 8, 65);
    let current_job = admitted(session.admit_mapping(&connection, current_request));
    let current = session
        .complete_mapping(current_job.prepare(), &mut host)
        .unwrap();
    assert_eq!(accepted_outcome(&current).undo_len, 1);
    assert_eq!(host.render(65)[0][0], -0.25);
}

#[test]
fn render_reconfiguration_preserves_accepted_document_history_and_failure_rollback() {
    let (yaml, context, settings) = fixture();
    let buses = planar_host_buses(&yaml);
    let mut session = InstrumentSession::new_for_planar_host(
        &yaml,
        context.clone(),
        settings.clone(),
        buses.clone(),
    )
    .unwrap();
    let mut other =
        InstrumentSession::new_for_planar_host(&yaml, context, settings, buses.clone()).unwrap();
    let (connection, initial) = session.attach().unwrap();
    let (foreign, _) = other.attach().unwrap();
    let mut host = RuntimeHost::new(&session, &initial);
    let foreign_candidate = other
        .prepare_render_configuration(
            &foreign,
            RenderSettings {
                sample_rate_hz: 44_100,
                block_size_frames: 128,
                duration_frames: 128,
            },
            buses.clone(),
        )
        .unwrap();
    assert_eq!(
        session.commit_render_configuration(foreign_candidate, &mut host),
        Err(CommitError::Session(SessionError::ForeignCandidate))
    );
    let stale_candidate = session
        .prepare_render_configuration(
            &connection,
            RenderSettings {
                sample_rate_hz: 44_100,
                block_size_frames: 128,
                duration_frames: 128,
            },
            buses.clone(),
        )
        .unwrap();
    let mapping = session
        .prepare_mapping(&connection, kick_mapping(60))
        .unwrap();
    let accepted = session.commit_mapping(mapping, &mut host).unwrap();
    assert_eq!(
        session.commit_render_configuration(stale_candidate, &mut host),
        Err(CommitError::Session(SessionError::StaleCandidate))
    );
    assert_eq!(host.attempts, 1);

    let render_settings = RenderSettings {
        sample_rate_hz: 44_100,
        block_size_frames: 128,
        duration_frames: 128,
    };
    let candidate = session
        .prepare_render_configuration(&connection, render_settings.clone(), buses.clone())
        .unwrap();
    let reconfigured = session
        .commit_render_configuration(candidate, &mut host)
        .unwrap();
    assert_eq!(reconfigured, accepted);
    assert_eq!(host.published, accepted);
    assert_eq!(host.render(60)[0][0], -0.5);
    assert_eq!(render_note(session.prepared(), 60)[0][0], -0.5);

    host.reject = true;
    let rejected = session
        .prepare_render_configuration(
            &connection,
            RenderSettings {
                sample_rate_hz: 96_000,
                block_size_frames: 64,
                duration_frames: 64,
            },
            buses,
        )
        .unwrap();
    assert_eq!(
        session.commit_render_configuration(rejected, &mut host),
        Err(CommitError::Activation("activation rejected"))
    );
    assert_eq!(session.snapshot(&connection).unwrap(), accepted);
    assert_eq!(host.published, accepted);
    assert_eq!(host.render(60)[0][0], -0.5);
}

#[test]
fn document_replacement_rejects_invalid_foreign_stale_and_busy_work_before_activation() {
    let (yaml, context, settings) = fixture();
    let mut session = InstrumentSession::new(&yaml, context.clone(), settings.clone()).unwrap();
    let mut other = InstrumentSession::new(&yaml, context, settings).unwrap();
    let (connection, initial) = session.attach().unwrap();
    let (foreign, other_initial) = other.attach().unwrap();
    let mut host = RuntimeHost::new(&session, &initial);
    let replacement_yaml = yaml_with_kick_mapping(&yaml, 64);

    assert!(matches!(
        session.prepare_document_replacement(&foreign, &replacement_yaml),
        Err(SessionError::InvalidAttachment)
    ));
    assert!(matches!(
        session.prepare_document_replacement(&connection, "not: a patch"),
        Err(SessionError::InvalidDocument(_))
    ));
    let missing = replacement_yaml.replace(
        "assets/advanced-drums.wav",
        "assets/missing-replacement-sample.wav",
    );
    assert!(matches!(
        session.prepare_document_replacement(&connection, &missing),
        Err(SessionError::PreparationFailed(_))
    ));
    assert_eq!(session.snapshot(&connection).unwrap(), initial);

    let foreign_replacement = other
        .prepare_document_replacement(&foreign, &replacement_yaml)
        .unwrap();
    assert_eq!(
        session.commit_document_replacement(foreign_replacement, &mut host),
        Err(CommitError::Session(SessionError::ForeignCandidate))
    );
    let stale_replacement = session
        .prepare_document_replacement(&connection, &replacement_yaml)
        .unwrap();
    let mapping = session
        .prepare_mapping(&connection, kick_mapping(60))
        .unwrap();
    let accepted = session.commit_mapping(mapping, &mut host).unwrap();
    assert_eq!(
        session.commit_document_replacement(stale_replacement, &mut host),
        Err(CommitError::Session(SessionError::StaleCandidate))
    );

    let blocked_replacement = session
        .prepare_document_replacement(&connection, &replacement_yaml)
        .unwrap();
    let job = admitted(session.admit_mapping(&connection, mapping_request(&accepted, 1, 61)));
    assert!(matches!(
        session.prepare_document_replacement(&connection, &replacement_yaml),
        Err(SessionError::Busy)
    ));
    assert_eq!(
        session.commit_document_replacement(blocked_replacement, &mut host),
        Err(CommitError::Session(SessionError::Busy))
    );
    let completed = session.complete_mapping(job.prepare(), &mut host).unwrap();
    assert_eq!(accepted_outcome(&completed).revision, 2);
    assert_eq!(host.attempts, 2);
    assert_eq!(host.render(61)[0][0], -0.5);
    assert_eq!(other.snapshot(&foreign).unwrap(), other_initial);
}
