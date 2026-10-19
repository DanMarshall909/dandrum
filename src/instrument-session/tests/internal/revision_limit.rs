use super::*;

struct NoActivation;
struct NoLease;
impl ActivationHost for NoActivation {
    type Error = &'static str;
    type Lease<'a> = NoLease;
    fn activate<'a>(
        &'a mut self,
        _: &PreparedKernelInstrument,
        _: &SessionSnapshot,
    ) -> Result<Self::Lease<'a>, Self::Error> {
        panic!("revision exhaustion must reject before host activation")
    }
}
impl ActivationLease for NoLease {
    fn publish(self, _: Publication<'_>) {
        panic!("an exhausted revision cannot publish")
    }
}

#[test]
fn revision_exhaustion_rejects_without_wrapping_accepted_identity_or_activating() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/patches");
    let yaml = std::fs::read_to_string(root.join("advanced-drum-kit.yaml")).unwrap();
    let mut session = InstrumentSession::new(
        &yaml,
        PreparationContext::new(root, 48_000),
        RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 16,
            duration_frames: 16,
        },
    )
    .unwrap();
    // Arrange the otherwise impractical lifetime boundary; all actions and
    // assertions below use the normal public admission/publication API.
    session.accepted.revision = u64::MAX;
    let (connection, accepted) = session.attach().unwrap();
    let candidate = session
        .prepare_mapping(
            &connection,
            SetZoneKeyRange {
                map_id: "kit".into(),
                zone_id: "kick".into(),
                key_range: [60, 60],
            },
        )
        .unwrap();
    assert_eq!(
        session.commit_mapping(candidate, &mut NoActivation),
        Err(CommitError::Session(SessionError::RevisionExhausted))
    );
    assert_eq!(session.snapshot(&connection).unwrap(), accepted);
    assert_eq!(accepted.revision, u64::MAX);
    assert_eq!(accepted.yaml, yaml);
    assert_eq!((accepted.undo_len, accepted.redo_len), (0, 0));
}

#[test]
fn admitted_request_retains_revision_exhaustion_without_activating() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/patches");
    let yaml = std::fs::read_to_string(root.join("advanced-drum-kit.yaml")).unwrap();
    let mut session = InstrumentSession::new(
        &yaml,
        PreparationContext::new(root, 48_000),
        RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 16,
            duration_frames: 16,
        },
    )
    .unwrap();
    session.accepted.revision = u64::MAX;
    let (connection, accepted) = session.attach().unwrap();
    let request = MappingRequest {
        instance: accepted.instance.clone(),
        request_id: RequestId(1),
        expected_revision: accepted.revision,
        intent: SetZoneKeyRange {
            map_id: "kit".into(),
            zone_id: "kick".into(),
            key_range: [60, 60],
        },
    };
    let job = match session.admit_mapping(&connection, request.clone()) {
        Ok(MappingAdmission::Admitted(job)) => job,
        Ok(_) => panic!("request should be newly admitted"),
        Err(error) => panic!("request should be admitted: {error:?}"),
    };
    let outcome = session
        .complete_mapping(job.prepare(), &mut NoActivation)
        .unwrap();

    assert_eq!(
        outcome.result,
        Err(RequestFailure::Session(SessionError::RevisionExhausted))
    );
    let queried = session
        .request_outcome(&connection, RequestId(1))
        .unwrap()
        .unwrap();
    let retried = match session.admit_mapping(&connection, request).unwrap() {
        MappingAdmission::Completed(outcome) => outcome,
        _ => panic!("revision exhaustion should be a retained terminal outcome"),
    };
    assert!(Arc::ptr_eq(&outcome, &queried));
    assert!(Arc::ptr_eq(&outcome, &retried));
    assert_eq!(session.snapshot(&connection).unwrap(), accepted);
}
