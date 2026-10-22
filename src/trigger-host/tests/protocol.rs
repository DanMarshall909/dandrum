use dandrum_engine::{patch::RenderSettings, preparation::PreparationContext};
use dandrum_instrument_session::InstrumentSession;
use dandrum_trigger_host::protocol::{EditorProtocol, PROTOCOL_VERSION};
use std::path::PathBuf;

fn protocol() -> EditorProtocol {
    EditorProtocol::new(std::num::NonZeroU64::MIN)
}

fn session() -> InstrumentSession {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/patches");
    let yaml = std::fs::read_to_string(root.join("advanced-drum-kit.yaml")).unwrap();
    InstrumentSession::new(
        &yaml,
        PreparationContext::new(root, 48000),
        RenderSettings {
            sample_rate_hz: 48000,
            block_size_frames: 16,
            duration_frames: 16,
        },
    )
    .unwrap()
}

#[test]
fn editor_connections_use_distinct_serializable_ids_and_native_accepted_snapshots() {
    let mut session = session();
    let mut protocol = protocol();
    let first = protocol
        .attach(&mut session)
        .expect("first editor connection must attach");
    let second = protocol
        .attach(&mut session)
        .expect("second editor connection must attach");
    assert_eq!(PROTOCOL_VERSION, 1);
    assert_eq!(first.attachment_id, "1");
    assert_eq!(second.attachment_id, "2");
    assert_eq!(first.snapshot, second.snapshot);
    assert!(
        first
            .snapshot
            .yaml
            .contains("id: dandrum.advanced-drum-kit")
    );
}

use dandrum_trigger_host::protocol::{
    Admission, EditorCommand, MESSAGE_BYTES_LIMIT, ProtocolError,
};
use serde_json::json;

#[test]
fn native_selection_commands_use_existing_bounds_without_browser_file_authority() {
    let mut session = session();
    let mut protocol = protocol();
    let connection = protocol.attach(&mut session).unwrap();
    for (index, (method, args)) in [
        ("chooseSamplerFiles", json!([])),
        ("getSamplerSelectionStatus", json!(["1"])),
        ("releaseSamplerSelection", json!(["1"])),
    ]
    .into_iter()
    .enumerate()
    {
        let frame = message(
            &connection.attachment_id,
            &(index + 1).to_string(),
            method,
            args,
        );
        let admitted = protocol.admit(&session, &frame);
        assert!(
            matches!(admitted, Ok(Admission::New(_))),
            "bounded native selection method {method} must be admitted: {admitted:?}"
        );
        assert!(matches!(
            protocol.admit(&session, &frame),
            Ok(Admission::Duplicate { .. })
        ));
    }
    for (method, args) in [
        ("chooseSamplerFiles", json!(["/tmp/browser-selected.wav"])),
        ("chooseSamplerFiles", json!([{"sampleRateHz":48000}])),
        ("getSamplerSelectionStatus", json!([1])),
        ("getSamplerSelectionStatus", json!(["01"])),
        ("getSamplerSelectionStatus", json!(["0"])),
        ("releaseSamplerSelection", json!(["18446744073709551616"])),
    ] {
        let frame = message(&connection.attachment_id, "4", method, args);
        assert!(matches!(
            protocol.admit(&session, &frame),
            Err(ProtocolError::InvalidArguments)
        ));
    }
    let mut wrong_version: serde_json::Value = serde_json::from_str(&message(
        &connection.attachment_id,
        "4",
        "chooseSamplerFiles",
        json!([]),
    ))
    .unwrap();
    wrong_version["version"] = json!(2);
    assert!(matches!(
        protocol.admit(&session, &wrong_version.to_string()),
        Err(ProtocolError::UnsupportedVersion)
    ));
}
fn message(attachment: &str, request: &str, method: &str, args: serde_json::Value) -> String {
    json!({"version":1,"attachmentId":attachment,"requestId":request,"method":method,"args":args})
        .to_string()
}
#[test]
fn versioned_snapshot_requests_are_typed_and_invalid_envelopes_preserve_accepted_state() {
    let mut session = session();
    let mut protocol = protocol();
    let connection = protocol.attach(&mut session).unwrap();
    let valid = message(
        &connection.attachment_id,
        "1",
        "getSamplerSession",
        json!([]),
    );
    let admitted = protocol
        .admit(&session, &valid)
        .expect("valid v1 snapshot must be admitted");
    assert!(
        matches!(admitted,Admission::New(ref request) if request.command==EditorCommand::GetSamplerSession && request.request_id==1 && request.attachment_id==1)
    );
    let Admission::New(request) = admitted else {
        panic!("new request expected")
    };
    assert_eq!(
        session.snapshot(&request.attachment).unwrap(),
        connection.snapshot
    );
    for (input, error) in [
        ("{".into(), ProtocolError::Malformed),
        ("[1]".into(), ProtocolError::Malformed),
        (
            valid.replace("\"version\":1", "\"version\":2"),
            ProtocolError::UnsupportedVersion,
        ),
        (
            valid.replace("\"requestId\":\"1\"", "\"requestId\":1"),
            ProtocolError::Malformed,
        ),
        (
            valid.replace("\"requestId\":\"1\"", "\"requestId\":\"01\""),
            ProtocolError::InvalidArguments,
        ),
        (
            message("01", "2", "getSamplerSession", json!([])),
            ProtocolError::InvalidArguments,
        ),
        (
            message("0", "2", "getSamplerSession", json!([])),
            ProtocolError::InvalidArguments,
        ),
        (
            message("18446744073709551616", "2", "getSamplerSession", json!([])),
            ProtocolError::InvalidArguments,
        ),
        (
            valid.replace("\"requestId\":\"1\"", "\"requestId\":\"0\""),
            ProtocolError::InvalidArguments,
        ),
        (
            valid.replace(
                "\"requestId\":\"1\"",
                "\"requestId\":\"18446744073709551616\"",
            ),
            ProtocolError::InvalidArguments,
        ),
        (
            valid.replace("\"version\":1", "\"version\":1,\"extra\":true"),
            ProtocolError::Malformed,
        ),
        (
            valid.replace("\"version\":1", "\"version\":1,\"version\":1"),
            ProtocolError::Malformed,
        ),
        (
            " ".repeat(MESSAGE_BYTES_LIMIT + 1),
            ProtocolError::Oversized,
        ),
        (
            message(&connection.attachment_id, "2", "unimplemented", json!([])),
            ProtocolError::CapabilityUnavailable,
        ),
        (
            message(
                &connection.attachment_id,
                "2",
                "getSamplerSession",
                json!([1]),
            ),
            ProtocolError::InvalidArguments,
        ),
        (
            message("99", "2", "getSamplerSession", json!([])),
            ProtocolError::StaleAttachment,
        ),
    ] {
        assert_eq!(
            protocol.admit(&session, &input).unwrap_err(),
            error,
            "input: {input:.100}"
        );
        assert_eq!(
            session.snapshot(&request.attachment).unwrap(),
            connection.snapshot
        );
        assert_preserved_sound_and_controls(&session);
    }
    assert!(matches!(
        protocol.admit(
            &session,
            &message(
                &connection.attachment_id,
                "2",
                "getSamplerSession",
                json!([])
            ),
        ),
        Ok(Admission::New(_))
    ));
    assert_eq!(
        serde_json::to_value(ProtocolError::CapabilityUnavailable).unwrap(),
        json!({"code":"capabilityUnavailable"})
    );
}

fn assert_preserved_sound_and_controls(session: &InstrumentSession) {
    use dandrum_engine::{
        PreparedSamplerAssets, graph::PortDirection, graph_processor::RealtimeGraphProcessor,
        kernel::ChannelCount,
    };
    let prepared = session.prepared();
    let ports = prepared.root_port_metadata();
    let inputs = ports
        .iter()
        .filter(|port| port.direction() == PortDirection::Input)
        .map(|port| {
            let ChannelCount::Literal(channels) = port.channels() else {
                panic!("prepared concrete channels")
            };
            vec![
                vec![port.control_default().map_or(0.0, |value| value.default()) as f32; 16];
                *channels as usize
            ]
        })
        .collect::<Vec<_>>();
    for name in [
        "level",
        "kick_level",
        "snare_level",
        "closed_hat_level",
        "open_hat_level",
    ] {
        let port = ports
            .iter()
            .find(|port| port.name() == name)
            .unwrap_or_else(|| {
                panic!(
                    "missing public control {name}: {:?}",
                    ports.iter().map(|p| p.name()).collect::<Vec<_>>()
                )
            });
        assert_eq!(
            port.control_default().unwrap().default(),
            1.0,
            "public {name} default remains accepted"
        );
    }
    let mut runtime = RealtimeGraphProcessor::from_compiled_patch(
        prepared.compiled_patch().clone(),
        48000.0,
        &PreparedSamplerAssets::empty(),
        16,
    );
    let mut outputs = vec![vec![vec![0.0; 16]; 2]];
    assert!(runtime.try_note_on_at(36, 100, 0));
    assert_eq!(runtime.render_root_buses(&inputs, &mut outputs), 16);
    assert_eq!(outputs[0][0][0], -0.5);
    assert_eq!(outputs[0][1][0], -0.5);
}

#[test]
fn supported_methods_preserve_distinct_session_job_parameter_and_note_values() {
    use dandrum_instrument_session::{RequestId, SetZoneKeyRange};
    use dandrum_trigger_host::protocol::SessionCommandIdentity;
    let mut session = session();
    let mut protocol = protocol();
    let c = protocol.attach(&mut session).unwrap();
    let identity = SessionCommandIdentity {
        instance_id: 1,
        attachment_id: 1,
        request_id: RequestId(17),
        expected_revision: 0,
    };
    let cases = [
        (
            "getParameterState",
            json!([]),
            EditorCommand::GetParameterState,
        ),
        (
            "setInstrumentZoneKeyRange",
            json!(["1", "1", "17", "0", "kit", "kick", 60, 63]),
            EditorCommand::SetZoneKeyRange {
                identity: identity.clone(),
                intent: SetZoneKeyRange {
                    map_id: "kit".into(),
                    zone_id: "kick".into(),
                    key_range: [60, 63],
                },
            },
        ),
        (
            "undoInstrumentSession",
            json!(["1", "1", "17", "0"]),
            EditorCommand::UndoInstrumentSession { identity },
        ),
        (
            "getInstrumentSessionJobStatus",
            json!(["19"]),
            EditorCommand::GetInstrumentSessionJobStatus {
                request_id: RequestId(19),
            },
        ),
        (
            "beginGesture",
            json!(["drums.level", 3]),
            EditorCommand::BeginGesture {
                parameter_id: "drums.level".into(),
                generation: 3,
            },
        ),
        (
            "setParameter",
            json!(["drums.kick.level", 0.37, 4]),
            EditorCommand::SetParameter {
                parameter_id: "drums.kick.level".into(),
                value: 0.37,
                generation: 4,
            },
        ),
        (
            "endGesture",
            json!(["drums.snare.level", 5]),
            EditorCommand::EndGesture {
                parameter_id: "drums.snare.level".into(),
                generation: 5,
            },
        ),
        (
            "noteOn",
            json!([62, 0.75, 6]),
            EditorCommand::NoteOn {
                note: 62,
                velocity: 0.75,
                generation: 6,
            },
        ),
        (
            "noteOff",
            json!([63, 7]),
            EditorCommand::NoteOff {
                note: 63,
                generation: 7,
            },
        ),
        ("noteHeartbeat", json!([]), EditorCommand::NoteHeartbeat),
    ];
    for (index, (method, args, expected)) in cases.into_iter().enumerate() {
        let Admission::New(request) = protocol
            .admit(
                &session,
                &message(&c.attachment_id, &(index + 1).to_string(), method, args),
            )
            .unwrap()
        else {
            panic!("new typed request expected")
        };
        assert_eq!(request.command, expected, "method {method}");
        assert_eq!(request.request_id, (index + 1) as u64);
        assert_eq!(session.snapshot(&request.attachment).unwrap(), c.snapshot);
    }
}

#[test]
fn invalid_method_arguments_and_stale_session_intents_cannot_change_controls_or_sound() {
    let mut session = session();
    let mut protocol = protocol();
    let c = protocol.attach(&mut session).unwrap();
    let Admission::New(probe) = protocol
        .admit(
            &session,
            &message(&c.attachment_id, "1", "getSamplerSession", json!([])),
        )
        .unwrap()
    else {
        panic!()
    };
    let mapping = |instance: &str,
                   attachment: &str,
                   request: &str,
                   revision: &str,
                   map: &str,
                   zone: &str,
                   low: u8,
                   high: u8| {
        json!([
            instance, attachment, request, revision, map, zone, low, high
        ])
    };
    let cases = [
        (
            "setInstrumentZoneKeyRange",
            json!(["1", "1", "17", "0", "kit", "kick", 60]),
            ProtocolError::InvalidArguments,
        ),
        (
            "setInstrumentZoneKeyRange",
            mapping("01", "1", "17", "0", "kit", "kick", 60, 60),
            ProtocolError::InvalidArguments,
        ),
        (
            "setInstrumentZoneKeyRange",
            mapping("1", "0", "17", "0", "kit", "kick", 60, 60),
            ProtocolError::InvalidArguments,
        ),
        (
            "undoInstrumentSession",
            json!(["1", "1", "17"]),
            ProtocolError::InvalidArguments,
        ),
        (
            "getInstrumentSessionJobStatus",
            json!([17]),
            ProtocolError::InvalidArguments,
        ),
        (
            "beginGesture",
            json!(["drums.level"]),
            ProtocolError::InvalidArguments,
        ),
        ("noteOff", json!([36, "1"]), ProtocolError::InvalidArguments),
        (
            "noteOn",
            json!([128, 0.5, 1]),
            ProtocolError::InvalidArguments,
        ),
        (
            "noteOn",
            json!([-1, 0.5, 1]),
            ProtocolError::InvalidArguments,
        ),
        (
            "noteOn",
            json!([36, 0.0, 1]),
            ProtocolError::InvalidArguments,
        ),
        (
            "noteOn",
            json!([36, 1.01, 1]),
            ProtocolError::InvalidArguments,
        ),
        ("noteOff", json!([128, 1]), ProtocolError::InvalidArguments),
        (
            "noteOff",
            json!([36, 9007199254740992u64]),
            ProtocolError::InvalidArguments,
        ),
        (
            "beginGesture",
            json!(["", 1]),
            ProtocolError::InvalidArguments,
        ),
        (
            "endGesture",
            json!(["a".repeat(257), 1]),
            ProtocolError::InvalidArguments,
        ),
        (
            "setParameter",
            json!(["bad\u{0000}id", 0.5, 1]),
            ProtocolError::InvalidArguments,
        ),
        (
            "setParameter",
            json!(["drums.level", -0.1, 1]),
            ProtocolError::InvalidArguments,
        ),
        (
            "setParameter",
            json!(["drums.level", 1.1, 1]),
            ProtocolError::InvalidArguments,
        ),
        (
            "setParameter",
            json!(["drums.level", "0.5", 1]),
            ProtocolError::InvalidArguments,
        ),
        (
            "setParameter",
            json!(["drums.level", 0.5, 1, 99]),
            ProtocolError::InvalidArguments,
        ),
        (
            "setParameter",
            json!({"id":"drums.level","value":0.5}),
            ProtocolError::InvalidArguments,
        ),
        (
            "getParameterState",
            json!([1]),
            ProtocolError::InvalidArguments,
        ),
        ("noteHeartbeat", json!([1]), ProtocolError::InvalidArguments),
        (
            "setInstrumentZoneKeyRange",
            mapping("1", "1", "17", "0", "kit", "kick", 64, 60),
            ProtocolError::InvalidArguments,
        ),
        (
            "setInstrumentZoneKeyRange",
            mapping("1", "1", "17", "0", "kit", "kick", 60, 128),
            ProtocolError::InvalidArguments,
        ),
        (
            "setInstrumentZoneKeyRange",
            mapping("1", "1", "17", "0", "", "kick", 60, 60),
            ProtocolError::InvalidArguments,
        ),
        (
            "setInstrumentZoneKeyRange",
            mapping("1", "1", "17", "0", "kit", "", 60, 60),
            ProtocolError::InvalidArguments,
        ),
        (
            "setInstrumentZoneKeyRange",
            mapping("2", "1", "17", "0", "kit", "kick", 60, 60),
            ProtocolError::StaleAttachment,
        ),
        (
            "setInstrumentZoneKeyRange",
            mapping("1", "2", "17", "0", "kit", "kick", 60, 60),
            ProtocolError::StaleAttachment,
        ),
        (
            "undoInstrumentSession",
            json!(["1", "1", "17", "1"]),
            ProtocolError::StaleRequest,
        ),
        (
            "undoInstrumentSession",
            json!(["1", "1", "0", "0"]),
            ProtocolError::InvalidArguments,
        ),
        (
            "undoInstrumentSession",
            json!(["1", "1", "17", "00"]),
            ProtocolError::InvalidArguments,
        ),
        (
            "getInstrumentSessionJobStatus",
            json!([""]),
            ProtocolError::InvalidArguments,
        ),
        (
            "getInstrumentSessionJobStatus",
            json!(["+1"]),
            ProtocolError::InvalidArguments,
        ),
        (
            "getInstrumentSessionJobStatus",
            json!([" 1"]),
            ProtocolError::InvalidArguments,
        ),
    ];
    for (method, args, error) in cases {
        assert_eq!(
            protocol
                .admit(&session, &message(&c.attachment_id, "2", method, args))
                .unwrap_err(),
            error,
            "method {method}"
        );
        assert_eq!(session.snapshot(&probe.attachment).unwrap(), c.snapshot);
        assert_preserved_sound_and_controls(&session);
    }
    let mut boundary = message(&c.attachment_id, "2", "getSamplerSession", json!([]));
    boundary.push_str(&" ".repeat(MESSAGE_BYTES_LIMIT - boundary.len()));
    assert!(matches!(
        protocol.admit(&session, &boundary),
        Ok(Admission::New(_))
    ));
}

#[test]
fn transport_retries_never_repeat_admission_conflicting_or_retired_ids_are_rejected() {
    use dandrum_trigger_host::protocol::REPLAY_LIMIT;
    let mut session = session();
    let mut protocol = protocol();
    let c = protocol.attach(&mut session).unwrap();
    let original = message(
        &c.attachment_id,
        "1",
        "setParameter",
        json!(["drums.level", 0.3, 1]),
    );
    let Admission::New(probe) = protocol.admit(&session, &original).unwrap() else {
        panic!()
    };
    assert!(matches!(
        protocol.admit(&session, &original),
        Ok(Admission::Duplicate { request_id: 1 })
    ));
    assert_eq!(
        protocol
            .admit(
                &session,
                &message(
                    &c.attachment_id,
                    "1",
                    "setParameter",
                    json!(["drums.level", 0.4, 1])
                )
            )
            .unwrap_err(),
        ProtocolError::ConflictingRequest
    );
    for id in 2..=REPLAY_LIMIT + 1 {
        assert!(matches!(
            protocol.admit(
                &session,
                &message(
                    &c.attachment_id,
                    &id.to_string(),
                    "getSamplerSession",
                    json!([])
                )
            ),
            Ok(Admission::New(_))
        ));
    }
    assert_eq!(
        protocol.admit(&session, &original).unwrap_err(),
        ProtocolError::RetiredRequest
    );
    assert!(matches!(
        protocol.admit(
            &session,
            &message(&c.attachment_id, "2", "getSamplerSession", json!([]))
        ),
        Ok(Admission::Duplicate { request_id: 2 })
    ));
    let second = protocol.attach(&mut session).unwrap();
    assert!(matches!(
        protocol.admit(
            &session,
            &message(&second.attachment_id, "1", "getSamplerSession", json!([]))
        ),
        Ok(Admission::New(_))
    ));
    assert_eq!(
        session.next_request_id().unwrap().0,
        1,
        "transport requests do not consume structural job ids"
    );
    assert_eq!(session.snapshot(&probe.attachment).unwrap(), c.snapshot);
    assert_preserved_sound_and_controls(&session);
}

#[test]
fn retired_editor_ids_and_foreign_native_sessions_cannot_recover_authority() {
    use std::num::NonZeroU64;
    let mut session = session();
    let mut foreign = self::session();
    let mut protocol = EditorProtocol::new(NonZeroU64::new(41).unwrap());
    let c = protocol.attach(&mut session).unwrap();
    assert_eq!(c.instance_id, "41");
    let first = message(&c.attachment_id, "1", "getSamplerSession", json!([]));
    let Admission::New(probe) = protocol.admit(&session, &first).unwrap() else {
        panic!()
    };
    assert_eq!(
        protocol.admit(&foreign, &first).unwrap_err(),
        ProtocolError::StaleAttachment
    );
    assert_eq!(
        protocol.attach(&mut foreign).err(),
        Some(ProtocolError::StaleAttachment)
    );
    assert_eq!(
        protocol.detach(&mut foreign, &c.attachment_id),
        Err(ProtocolError::StaleAttachment)
    );
    assert!(matches!(
        protocol.admit(&session, &first),
        Ok(Admission::Duplicate { request_id: 1 })
    ));
    for invalid in ["01", "0", "18446744073709551616"] {
        assert_eq!(
            protocol.detach(&mut session, invalid),
            Err(ProtocolError::InvalidArguments)
        );
        assert_eq!(session.snapshot(&probe.attachment).unwrap(), c.snapshot);
        assert!(matches!(
            protocol.admit(&session, &first),
            Ok(Admission::Duplicate { request_id: 1 })
        ));
        assert_preserved_sound_and_controls(&session);
    }
    protocol.detach(&mut session, &c.attachment_id).unwrap();
    assert!(
        session.snapshot(&probe.attachment).is_err(),
        "retaining native handle cannot revive explicit detach"
    );
    assert_eq!(
        protocol.admit(&session, &first).unwrap_err(),
        ProtocolError::StaleAttachment
    );
    assert_eq!(
        protocol.detach(&mut session, &c.attachment_id),
        Err(ProtocolError::StaleAttachment)
    );
    let next = protocol.attach(&mut session).unwrap();
    assert_eq!(next.attachment_id, "2");
    assert_eq!(next.snapshot, c.snapshot);
    for _ in 1..16 {
        protocol.attach(&mut session).unwrap();
    }
    assert!(matches!(
        protocol.attach(&mut session),
        Err(ProtocolError::Session(
            dandrum_instrument_session::SessionError::AttachmentLimit
        ))
    ));
    protocol.detach(&mut session, &next.attachment_id).unwrap();
    assert_eq!(protocol.attach(&mut session).unwrap().attachment_id, "18");
    assert_preserved_sound_and_controls(&session);
}

#[test]
fn native_attachment_capacity_rejection_does_not_consume_a_protocol_identity() {
    let mut session = session();
    let mut protocol = protocol();
    let mut native = (0..dandrum_instrument_session::ATTACHMENT_LIMIT)
        .map(|_| session.attach().unwrap().0)
        .collect::<Vec<_>>();
    let before = session.owner_snapshot();
    assert!(matches!(
        protocol.attach(&mut session),
        Err(ProtocolError::Session(
            dandrum_instrument_session::SessionError::AttachmentLimit
        ))
    ));
    assert_eq!(session.owner_snapshot(), before);
    assert_preserved_sound_and_controls(&session);
    session.detach(&native.pop().unwrap()).unwrap();
    let editor = protocol.attach(&mut session).unwrap();
    assert_eq!(editor.attachment_id, "1");
    assert_eq!(editor.snapshot, before);
    assert!(matches!(
        protocol.admit(&session, &message("1", "1", "getSamplerSession", json!([]))),
        Ok(Admission::New(_))
    ));
}

#[test]
fn replies_and_events_are_versioned_correlated_bounded_and_retired_with_the_editor() {
    use dandrum_trigger_host::protocol::OUTBOUND_BYTES_LIMIT;
    let mut session = session();
    let mut protocol = protocol();
    let c = protocol.attach(&mut session).unwrap();
    let reply = protocol
        .encode_reply(
            &session,
            &c.attachment_id,
            "23",
            Ok(json!({"revision":"7","value":0.37})),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&reply).unwrap(),
        json!({"version":1,"instanceId":"1","attachmentId":"1","requestId":"23","status":"accepted","result":{"revision":"7","value":0.37}})
    );
    for (error, code) in [
        (ProtocolError::Malformed, "malformed"),
        (ProtocolError::Oversized, "oversized"),
        (ProtocolError::UnsupportedVersion, "unsupportedVersion"),
        (ProtocolError::StaleAttachment, "staleAttachment"),
        (ProtocolError::InvalidArguments, "invalidArguments"),
        (ProtocolError::ConflictingRequest, "conflictingRequest"),
        (ProtocolError::StaleRequest, "staleRequest"),
        (ProtocolError::RetiredRequest, "retiredRequest"),
        (ProtocolError::IdentityExhausted, "identityExhausted"),
        (
            ProtocolError::Session(dandrum_instrument_session::SessionError::Busy),
            "session",
        ),
    ] {
        let encoded = protocol
            .encode_reply(&session, &c.attachment_id, "24", Err(error))
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&encoded).unwrap()["error"],
            json!({"code":code})
        );
    }
    let error = protocol
        .encode_reply(
            &session,
            &c.attachment_id,
            "24",
            Err(ProtocolError::CapabilityUnavailable),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&error).unwrap(),
        json!({"version":1,"instanceId":"1","attachmentId":"1","requestId":"24","status":"rejected","error":{"code":"capabilityUnavailable"}})
    );
    let empty = protocol
        .encode_reply(&session, &c.attachment_id, "25", Ok(json!("")))
        .unwrap();
    let boundary = "x".repeat(OUTBOUND_BYTES_LIMIT - empty.len());
    assert_eq!(
        protocol
            .encode_reply(&session, &c.attachment_id, "25", Ok(json!(boundary)))
            .unwrap()
            .len(),
        OUTBOUND_BYTES_LIMIT
    );
    assert_eq!(
        protocol
            .encode_reply(
                &session,
                &c.attachment_id,
                "25",
                Ok(json!(format!("{boundary}x")))
            )
            .unwrap_err(),
        ProtocolError::Oversized
    );
    assert_eq!(
        protocol
            .encode_reply(
                &session,
                &c.attachment_id,
                "25",
                Ok(json!("x".repeat(OUTBOUND_BYTES_LIMIT)))
            )
            .unwrap_err(),
        ProtocolError::Oversized
    );
    assert_eq!(
        protocol
            .encode_reply(&session, &c.attachment_id, "0", Ok(json!(null)))
            .unwrap_err(),
        ProtocolError::InvalidArguments
    );
    assert_eq!(
        protocol
            .encode_event(&session, &c.attachment_id, "unimplemented", json!({}))
            .unwrap_err(),
        ProtocolError::CapabilityUnavailable
    );
    assert_eq!(
        protocol
            .encode_event(
                &session,
                &c.attachment_id,
                "parameterStateChanged",
                json!("x".repeat(OUTBOUND_BYTES_LIMIT))
            )
            .unwrap_err(),
        ProtocolError::Oversized
    );
    let event = protocol
        .encode_event(
            &session,
            &c.attachment_id,
            "parameterStateChanged",
            json!({"generation":3,"parameters":[{"id":"drums.level","value":0.37}]}),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&event).unwrap(),
        json!({"version":1,"instanceId":"1","attachmentId":"1","sequence":"1","event":"parameterStateChanged","payload":{"generation":3,"parameters":[{"id":"drums.level","value":0.37}]}})
    );
    let next = protocol
        .encode_event(
            &session,
            &c.attachment_id,
            "instrumentSessionChanged",
            json!({"revision":"8"}),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&next).unwrap()["sequence"],
        "2"
    );
    let second = protocol.attach(&mut session).unwrap();
    let second_event = protocol
        .encode_event(
            &session,
            &second.attachment_id,
            "instrumentSessionChanged",
            json!({}),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&second_event).unwrap()["sequence"],
        "1"
    );
    protocol.detach(&mut session, &c.attachment_id).unwrap();
    assert_eq!(
        protocol
            .encode_reply(&session, &c.attachment_id, "23", Ok(json!({})))
            .unwrap_err(),
        ProtocolError::StaleAttachment
    );
    assert_eq!(
        protocol
            .encode_event(
                &session,
                &c.attachment_id,
                "parameterStateChanged",
                json!({})
            )
            .unwrap_err(),
        ProtocolError::StaleAttachment
    );
    assert_eq!(
        protocol
            .encode_event(
                &self::session(),
                &second.attachment_id,
                "parameterStateChanged",
                json!({})
            )
            .unwrap_err(),
        ProtocolError::StaleAttachment
    );
    assert_preserved_sound_and_controls(&session);
}
