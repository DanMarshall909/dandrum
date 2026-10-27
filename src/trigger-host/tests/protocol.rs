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
    for version in [0, 2, 3, u32::MAX] {
        wrong_version["version"] = json!(version);
        assert!(matches!(
            protocol.admit(&session, &wrong_version.to_string()),
            Err(ProtocolError::UnsupportedVersion)
        ));
    }
}
fn message(attachment: &str, request: &str, method: &str, args: serde_json::Value) -> String {
    json!({"version":1,"attachmentId":attachment,"requestId":request,"method":method,"args":args})
        .to_string()
}

#[test]
fn native_negotiation_requires_the_retained_owner_and_explicit_execution() {
    let mut session = session();
    let mut protocol = protocol();
    let connection = protocol.attach(&mut session).unwrap();
    let frame = message(
        &connection.attachment_id,
        "1",
        "getSamplerSession",
        json!([]),
    );
    let Admission::New(mut read) = protocol.admit(&session, &frame).unwrap() else {
        panic!("new read")
    };
    assert_eq!(
        protocol.negotiate(&session, &read),
        Err(ProtocolError::InvalidArguments)
    );
    let frame = message(
        &connection.attachment_id,
        "2",
        "negotiateEditorProtocol",
        json!([2]),
    );
    let Admission::New(negotiation) = protocol.admit(&session, &frame).unwrap() else {
        panic!("new negotiation")
    };
    let snapshot = json!({"version":2,"attachmentId":connection.attachment_id,"requestId":"3","method":"getSamplerSession","args":[]}).to_string();
    assert_eq!(
        protocol.admit(&session, &snapshot).unwrap_err(),
        ProtocolError::UnsupportedVersion,
        "transport admission alone does not grant a profile"
    );
    let mut other_session = self::session();
    let mut other = self::protocol();
    let coincident = other.attach(&mut other_session).unwrap();
    assert_eq!(coincident.attachment_id, connection.attachment_id);
    assert_eq!(
        other.negotiate(&other_session, &negotiation),
        Err(ProtocolError::StaleAttachment)
    );
    assert_eq!(
        other.admit(&other_session, &snapshot).unwrap_err(),
        ProtocolError::UnsupportedVersion
    );
    let own_frame = message(
        &coincident.attachment_id,
        "1",
        "negotiateEditorProtocol",
        json!([2]),
    );
    let Admission::New(stale) = other.admit(&other_session, &own_frame).unwrap() else {
        panic!("own negotiation")
    };
    other_session.detach(&stale.attachment).unwrap();
    assert_eq!(
        other.negotiate(&other_session, &stale),
        Err(ProtocolError::StaleAttachment)
    );
    let reply: serde_json::Value =
        serde_json::from_str(&protocol.negotiate(&session, &negotiation).unwrap()).unwrap();
    assert_eq!(reply["version"], 1);
    assert_eq!(reply["result"]["wireVersion"], 2);
    assert!(matches!(
        protocol.admit(&session, &snapshot),
        Ok(Admission::New(_))
    ));
    let cross_version = json!({"version":2,"attachmentId":connection.attachment_id,"requestId":"1","method":"getSamplerSession","args":[]}).to_string();
    assert_eq!(
        protocol.admit(&session, &cross_version).unwrap_err(),
        ProtocolError::ConflictingRequest
    );
    read.command = EditorCommand::NegotiateEditorProtocol;
    assert_eq!(
        protocol.negotiate(&session, &read),
        Err(ProtocolError::StaleRequest),
        "changing public request data cannot forge a retained native command"
    );
    assert_eq!(session.owner_snapshot(), connection.snapshot);
    protocol
        .detach(&mut session, &connection.attachment_id)
        .unwrap();
    assert_eq!(
        protocol.negotiate(&session, &negotiation),
        Err(ProtocolError::StaleAttachment)
    );
    let fresh = protocol.attach(&mut session).unwrap();
    let frame = json!({"version":2,"attachmentId":fresh.attachment_id,"requestId":"1","method":"getSamplerSession","args":[]}).to_string();
    assert_eq!(
        protocol.admit(&session, &frame).unwrap_err(),
        ProtocolError::UnsupportedVersion
    );
}

#[test]
fn invalid_profile_choices_and_version_two_small_methods_preserve_version_one() {
    let mut session = session();
    let mut protocol = protocol();
    let connection = protocol.attach(&mut session).unwrap();
    for (args, error) in [
        (json!([]), ProtocolError::InvalidArguments),
        (json!(["2"]), ProtocolError::InvalidArguments),
        (json!([2, 2]), ProtocolError::InvalidArguments),
        (json!([-1]), ProtocolError::InvalidArguments),
        (json!([4294967296_u64]), ProtocolError::InvalidArguments),
        (json!([1]), ProtocolError::UnsupportedVersion),
        (json!([3]), ProtocolError::UnsupportedVersion),
    ] {
        assert_eq!(
            protocol
                .admit(
                    &session,
                    &message(
                        &connection.attachment_id,
                        "1",
                        "negotiateEditorProtocol",
                        args
                    )
                )
                .unwrap_err(),
            error
        );
    }
    let Admission::New(negotiation) = protocol
        .admit(
            &session,
            &message(
                &connection.attachment_id,
                "1",
                "negotiateEditorProtocol",
                json!([2]),
            ),
        )
        .unwrap()
    else {
        panic!("new negotiation")
    };
    protocol.negotiate(&session, &negotiation).unwrap();
    for method in [
        "getParameterState",
        "chooseSamplerFiles",
        "noteHeartbeat",
        "negotiateEditorProtocol",
    ] {
        let frame = json!({"version":2,"attachmentId":connection.attachment_id,"requestId":"2","method":method,"args":[]}).to_string();
        assert_eq!(
            protocol.admit(&session, &frame).unwrap_err(),
            ProtocolError::UnsupportedVersion
        );
    }
    assert!(matches!(
        protocol.admit(
            &session,
            &message(
                &connection.attachment_id,
                "2",
                "getParameterState",
                json!([])
            )
        ),
        Ok(Admission::New(_))
    ));
    assert_eq!(session.owner_snapshot(), connection.snapshot);
    assert_preserved_sound_and_controls(&session);
}

#[test]
fn negotiated_patch_frames_admit_real_larger_documents_and_retain_exact_replay_identity() {
    let mut session = session();
    let mut protocol = protocol();
    let connection = protocol.attach(&mut session).unwrap();
    let regions = (0..256)
        .map(|index| {
            format!(
                "        - {{ id: library_{index}, start_frame: {}, end_frame: {} }}\n",
                index * 32,
                index * 32 + 16,
            )
        })
        .collect::<String>();
    let yaml = connection.snapshot.yaml.replacen(
        "      regions:\n",
        &format!("      regions:\n{regions}"),
        1,
    );
    assert!(yaml.len() > 16_384 && yaml.len() < 65_536);
    dandrum_engine::kernel::document::load_kernel_patch_str(&yaml)
        .expect("meaningful larger sample-region document");
    let patch_frame = |version, id: &str, yaml: &str| {
        json!({
        "version":version,"attachmentId":connection.attachment_id,"requestId":id,
        "method":"prepareInstrumentSessionPatch","args":["1",connection.attachment_id,"41","0",yaml]
    }).to_string()
    };
    assert_eq!(
        protocol
            .admit(&session, &patch_frame(1, "2", &yaml))
            .unwrap_err(),
        ProtocolError::Oversized
    );
    assert_eq!(
        protocol
            .admit(&session, &patch_frame(2, "2", &yaml))
            .unwrap_err(),
        ProtocolError::Oversized
    );
    let Admission::New(negotiation) = protocol
        .admit(
            &session,
            &message(
                &connection.attachment_id,
                "1",
                "negotiateEditorProtocol",
                json!([2]),
            ),
        )
        .unwrap()
    else {
        panic!("new negotiation")
    };
    protocol.negotiate(&session, &negotiation).unwrap();
    let frame = patch_frame(2, "2", &yaml);
    let admitted = protocol.admit(&session, &frame);
    assert!(
        matches!(&admitted, Ok(Admission::New(_))),
        "a real negotiated patch above the old16KiB limit must reach native admission: {admitted:?}"
    );
    let Admission::New(request) = admitted.unwrap() else {
        panic!("new patch")
    };
    assert_eq!(request.request_id, 2);
    let EditorCommand::PrepareInstrumentSessionPatch { identity, document } = request.command
    else {
        panic!("typed patch")
    };
    assert_eq!(
        identity.request_id.0, 41,
        "session and transport cursors stay independent"
    );
    assert_eq!(&*document, yaml);
    assert!(matches!(
        protocol.admit(&session, &frame),
        Ok(Admission::Duplicate { request_id: 2 })
    ));
    assert_eq!(
        protocol
            .admit(
                &session,
                &patch_frame(2, "2", &(yaml.clone() + "\n# different exact request\n"))
            )
            .unwrap_err(),
        ProtocolError::ConflictingRequest
    );
    assert_eq!(session.owner_snapshot(), connection.snapshot);
    assert_preserved_sound_and_controls(&session);
}

fn negotiated_protocol() -> (
    InstrumentSession,
    EditorProtocol,
    dandrum_trigger_host::protocol::EditorConnection,
) {
    let mut session = session();
    let mut protocol = protocol();
    let connection = protocol.attach(&mut session).unwrap();
    let Admission::New(request) = protocol
        .admit(
            &session,
            &message(
                &connection.attachment_id,
                "1",
                "negotiateEditorProtocol",
                json!([2]),
            ),
        )
        .unwrap()
    else {
        panic!("new negotiation")
    };
    protocol.negotiate(&session, &request).unwrap();
    (session, protocol, connection)
}

fn full_patch_frame(
    connection: &dandrum_trigger_host::protocol::EditorConnection,
    id: &str,
    document: &str,
) -> String {
    json!({"version":2,"attachmentId":connection.attachment_id,"requestId":id,
        "method":"prepareInstrumentSessionPatch","args":[connection.instance_id,connection.attachment_id,id,"0",document]}).to_string()
}

#[test]
fn negotiated_wire_document_and_reply_bounds_are_independent_of_version_one() {
    let (session, mut protocol, connection) = negotiated_protocol();
    let base = connection.snapshot.yaml.clone() + "\n#";
    let document = format!("{base}{}", "x".repeat(65_536 - base.len()));
    let oversize = full_patch_frame(&connection, "2", &(document.clone() + "x"));
    assert!(
        oversize.len() < 524_288,
        "decoded document limit is not the wire limit"
    );
    assert_eq!(
        protocol.admit(&session, &oversize).unwrap_err(),
        ProtocolError::Oversized
    );
    let frame = full_patch_frame(&connection, "2", &document);
    let exact = format!("{frame}{}", " ".repeat(524_288 - frame.len()));
    assert_eq!(
        protocol
            .admit(&session, &(exact.clone() + " "))
            .unwrap_err(),
        ProtocolError::Oversized
    );
    assert!(
        matches!(protocol.admit(&session, &exact), Ok(Admission::New(_))),
        "an exact bounded frame with a bounded decoded document must be admitted"
    );
    let empty = protocol
        .encode_reply(&session, &connection.attachment_id, "2", Ok(json!("")))
        .unwrap();
    let payload = "x".repeat(1_048_576 - empty.len());
    let reply = protocol
        .encode_reply(&session, &connection.attachment_id, "2", Ok(json!(payload)))
        .unwrap();
    assert_eq!(reply.len(), 1_048_576);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&reply).unwrap()["version"],
        2
    );
    assert_eq!(
        protocol.encode_reply(
            &session,
            &connection.attachment_id,
            "2",
            Ok(json!(payload + "x"))
        ),
        Err(ProtocolError::Oversized)
    );
    assert_eq!(
        protocol.encode_reply(
            &session,
            &connection.attachment_id,
            "1",
            Ok(json!("x".repeat(65_536)))
        ),
        Err(ProtocolError::Oversized),
        "negotiation never enlarges a v1 reply"
    );
    assert_eq!(session.owner_snapshot(), connection.snapshot);
}

#[test]
fn document_replay_byte_pressure_retires_old_documents_without_evicting_small_requests() {
    let (session, mut protocol, connection) = negotiated_protocol();
    let small = message(
        &connection.attachment_id,
        "2",
        "getParameterState",
        json!([]),
    );
    assert!(matches!(
        protocol.admit(&session, &small),
        Ok(Admission::New(_))
    ));
    let mut frames = Vec::new();
    for id in 3..=7 {
        let base = format!("{}\n# edit{id}:", connection.snapshot.yaml);
        let document = format!("{base}{}", "x".repeat(65_536 - base.len()));
        let frame = full_patch_frame(&connection, &id.to_string(), &document);
        assert!(matches!(
            protocol.admit(&session, &frame),
            Ok(Admission::New(_))
        ));
        frames.push(frame);
    }
    assert_eq!(
        protocol.admit(&session, &frames[0]).unwrap_err(),
        ProtocolError::RetiredRequest,
        "the fifth64KiB document retires the oldest document rather than retaining unbounded payloads"
    );
    assert!(
        matches!(
            protocol.admit(&session, &small),
            Ok(Admission::Duplicate { request_id: 2 })
        ),
        "byte pressure does not evict a small request that owns no document bytes"
    );
    assert!(matches!(
        protocol.admit(&session, &frames[4]),
        Ok(Admission::Duplicate { request_id: 7 })
    ));
    assert_eq!(session.owner_snapshot(), connection.snapshot);
}

#[test]
fn patch_argument_authority_and_utf8_limits_reject_without_consuming_the_request() {
    let (session, mut protocol, connection) = negotiated_protocol();
    for (args, error) in [
        (json!([]), ProtocolError::InvalidArguments),
        (
            json!(["1", connection.attachment_id, "2", "0", 42]),
            ProtocolError::InvalidArguments,
        ),
        (
            json!(["1", connection.attachment_id, "2", "0", ""]),
            ProtocolError::InvalidArguments,
        ),
        (
            json!(["2", connection.attachment_id, "2", "0", "yaml"]),
            ProtocolError::StaleAttachment,
        ),
        (
            json!(["1", "2", "2", "0", "yaml"]),
            ProtocolError::StaleAttachment,
        ),
        (
            json!(["1", connection.attachment_id, "0", "0", "yaml"]),
            ProtocolError::InvalidArguments,
        ),
        (
            json!(["1", connection.attachment_id, "2", "1", "yaml"]),
            ProtocolError::StaleRequest,
        ),
        (
            json!(["1", connection.attachment_id, "2", "0", "é".repeat(32_769)]),
            ProtocolError::Oversized,
        ),
    ] {
        let frame = json!({"version":2,"attachmentId":connection.attachment_id,"requestId":"2",
            "method":"prepareInstrumentSessionPatch","args":args})
        .to_string();
        assert_eq!(protocol.admit(&session, &frame).unwrap_err(), error);
    }
    // Framing admits bytes; YAML/resource validation belongs to the worker.
    // These inputs distinguish UTF-8 byte length from characters and prove
    // worst-case JSON escaping independently of the meaningful document test.
    for (id, text) in [("2", "é".repeat(32_768)), ("3", "\u{0001}".repeat(65_536))] {
        let frame = full_patch_frame(&connection, id, &text);
        assert!(frame.len() < 524_288);
        let Admission::New(request) = protocol.admit(&session, &frame).unwrap() else {
            panic!("new bounded frame")
        };
        let EditorCommand::PrepareInstrumentSessionPatch { document, identity } = request.command
        else {
            panic!("typed patch")
        };
        assert_eq!(&*document, text);
        assert_eq!(document.len(), 65_536);
        assert_eq!(identity.instance_id, 1);
        assert_eq!(identity.attachment_id.to_string(), connection.attachment_id);
        assert_eq!(identity.request_id.0.to_string(), id);
        assert_eq!(identity.expected_revision, 0);
    }
    assert_eq!(session.owner_snapshot(), connection.snapshot);
    assert_preserved_sound_and_controls(&session);
}

#[test]
fn rejected_reply_versions_require_the_same_live_attachment_and_request() {
    let (mut session, mut protocol, connection) = negotiated_protocol();
    let other = protocol.attach(&mut session).unwrap();
    let frame = |attachment: &str, id: &str| {
        json!({"version":2,"attachmentId":attachment,
        "requestId":id,"method":"getSamplerSession","args":[1]})
        .to_string()
    };
    let rejected = |protocol: &EditorProtocol, attachment: &str, id: &str, message: &str| {
        serde_json::from_str::<serde_json::Value>(
            &protocol
                .encode_rejection(
                    &session,
                    attachment,
                    id,
                    message,
                    ProtocolError::InvalidArguments,
                )
                .unwrap(),
        )
        .unwrap()
    };
    let message = frame(&connection.attachment_id, "2");
    assert_eq!(
        rejected(&protocol, &connection.attachment_id, "2", &message)["version"],
        2
    );
    assert_eq!(
        rejected(&protocol, &connection.attachment_id, "3", &message)["version"],
        1,
        "another request's header does not select v2"
    );
    assert_eq!(
        rejected(&protocol, &other.attachment_id, "2", &message)["version"],
        1,
        "another attachment's header does not select v2"
    );
    assert_eq!(
        rejected(
            &protocol,
            &other.attachment_id,
            "2",
            &frame(&other.attachment_id, "2")
        )["version"],
        1,
        "an unnegotiated attachment stays v1"
    );
    let mut conflict: serde_json::Value =
        serde_json::from_str(&frame(&connection.attachment_id, "1")).unwrap();
    conflict["args"] = json!([]);
    let conflict = conflict.to_string();
    assert_eq!(
        protocol.admit(&session, &conflict).unwrap_err(),
        ProtocolError::ConflictingRequest
    );
    assert_eq!(
        rejected(&protocol, &connection.attachment_id, "1", &conflict)["version"],
        2,
        "rejection uses the permitted incoming profile, not an old retained v1 command"
    );
    assert_eq!(
        protocol.encode_rejection(
            &session,
            &connection.attachment_id,
            "0",
            &message,
            ProtocolError::Malformed
        ),
        Err(ProtocolError::InvalidArguments)
    );
    assert_eq!(
        protocol.encode_rejection(
            &self::session(),
            &connection.attachment_id,
            "2",
            &message,
            ProtocolError::Malformed
        ),
        Err(ProtocolError::StaleAttachment)
    );
    let valid = json!({"version":2,"attachmentId":connection.attachment_id,"requestId":"2","method":"getSamplerSession","args":[]}).to_string();
    assert!(
        matches!(protocol.admit(&session, &valid), Ok(Admission::New(_))),
        "encoding rejection never reserves transport admission"
    );
    protocol
        .detach(&mut session, &connection.attachment_id)
        .unwrap();
    assert_eq!(
        protocol.encode_rejection(
            &session,
            &connection.attachment_id,
            "2",
            &message,
            ProtocolError::Malformed
        ),
        Err(ProtocolError::StaleAttachment)
    );
    assert_eq!(session.owner_snapshot(), connection.snapshot);
}

#[test]
fn larger_messages_cannot_borrow_a_profile_from_unmatched_headers_or_native_sessions() {
    let (mut session, mut protocol, connection) = negotiated_protocol();
    let document = connection.snapshot.yaml.clone() + "\n#" + "x".repeat(16_384).as_str();
    let frame: serde_json::Value =
        serde_json::from_str(&full_patch_frame(&connection, "2", &document)).unwrap();
    for (field, value) in [
        ("version", json!(1)),
        ("version", json!(3)),
        ("attachmentId", json!("0")),
        ("attachmentId", json!("01")),
        ("attachmentId", json!("99")),
        ("method", json!("noteHeartbeat")),
    ] {
        let mut malformed = frame.clone();
        malformed[field] = value;
        assert_eq!(
            protocol
                .admit(&session, &malformed.to_string())
                .unwrap_err(),
            ProtocolError::Oversized
        );
    }
    let frame = frame.to_string();
    assert_eq!(
        protocol.admit(&self::session(), &frame).unwrap_err(),
        ProtocolError::Oversized
    );
    let other = protocol.attach(&mut session).unwrap();
    let mut foreign: serde_json::Value = serde_json::from_str(&frame).unwrap();
    foreign["attachmentId"] = json!(other.attachment_id);
    assert_eq!(
        protocol.admit(&session, &foreign.to_string()).unwrap_err(),
        ProtocolError::Oversized
    );
    let Admission::New(request) = protocol.admit(&session, &frame).unwrap() else {
        panic!("own negotiated frame")
    };
    session.detach(&request.attachment).unwrap();
    assert_eq!(
        protocol.admit(&session, &frame).unwrap_err(),
        ProtocolError::Oversized
    );
    assert_eq!(session.owner_snapshot(), connection.snapshot);
    assert_preserved_sound_and_controls(&session);
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
            EditorCommand::UndoInstrumentSession {
                identity: identity.clone(),
            },
        ),
        (
            "redoInstrumentSession",
            json!(["1", "1", "17", "0"]),
            EditorCommand::RedoInstrumentSession { identity },
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
            "redoInstrumentSession",
            json!(["1", "1", "17"]),
            ProtocolError::InvalidArguments,
        ),
        (
            "redoInstrumentSession",
            json!(["1", "1", "17", "1"]),
            ProtocolError::StaleRequest,
        ),
        (
            "redoInstrumentSession",
            json!(["1", "1", "0", "0"]),
            ProtocolError::InvalidArguments,
        ),
        (
            "redoInstrumentSession",
            json!(["1", "1", "17", "00"]),
            ProtocolError::InvalidArguments,
        ),
        (
            "redoInstrumentSession",
            json!(["2", "1", "17", "0"]),
            ProtocolError::StaleAttachment,
        ),
        (
            "redoInstrumentSession",
            json!(["1", "2", "17", "0"]),
            ProtocolError::StaleAttachment,
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
