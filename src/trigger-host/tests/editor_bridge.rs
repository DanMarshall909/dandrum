use dandrum_trigger_host::{
    editor_bridge::{browser_request, delivery_script, initialization_script},
    protocol::PATCH_MESSAGE_BYTES_LIMIT,
};
use serde_json::{Value, json};
use std::{
    io::Write,
    num::NonZeroU64,
    process::{Command, Stdio},
};

#[test]
fn browser_gate_preserves_canonical_identity_without_admitting_semantic_commands() {
    let frame = |id: Value| json!({"version":1,"attachmentId":"7","requestId":id,"method":"chooseSamplerFiles","args":[]});
    for id in ["1", "9007199254740993", "18446744073709551615"] {
        let wire = frame(json!(id)).to_string();
        let request = browser_request("7", &wire).unwrap();
        assert_eq!(request.request_id(), id);
        assert!(request.is_native_selection());
        assert_eq!(request.message(), wire);
        assert_eq!(
            browser_request("7", &wire).unwrap().request_id(),
            id,
            "framing consumes no request"
        );
    }
    let escaped = r#"{"attachmentId":"7","requestId":"\u0031","method":"chooseSamplerFiles"}"#;
    assert_eq!(browser_request("7", escaped).unwrap().request_id(), "1");
    for semantic in [
        json!({"attachmentId":"7","requestId":"2"}),
        json!({"version":99,"attachmentId":"7","requestId":"2","method":"unknown","args":null}),
        json!({"version":1,"attachmentId":"7","requestId":"2","method":false}),
        json!({"version":1,"attachmentId":"7","requestId":"2","method":"setParameter","args":[]}),
    ] {
        let wire = semantic.to_string();
        let request = browser_request("7", &wire).unwrap();
        assert_eq!(request.request_id(), "2");
        assert!(
            !request.is_native_selection(),
            "semantic admission remains with the instrument owner"
        );
    }
    for id in [
        json!(null),
        json!(1),
        json!(false),
        json!(""),
        json!("0"),
        json!("00"),
        json!("01"),
        json!("+1"),
        json!("-1"),
        json!(" 1"),
        json!("1 "),
        json!("1.0"),
        json!("١"),
        json!("18446744073709551616"),
        json!("100000000000000000000"),
    ] {
        assert!(
            browser_request("7", &frame(id.clone()).to_string()).is_none(),
            "request ID {id}"
        );
    }
    for wire in [
        "{",
        "null",
        "[]",
        "1",
        "true",
        r#"{"attachmentId":"7"}"#,
        r#"{"attachmentId":7,"requestId":"1"}"#,
        r#"{"attachmentId":"8","requestId":"1"}"#,
    ] {
        assert!(browser_request("7", wire).is_none(), "invalid frame {wire}");
    }
}

#[test]
fn browser_gate_applies_the_global_byte_ceiling_before_owner_negotiated_limits() {
    let mut frame =
        json!({"attachmentId":"7","requestId":"1","method":"getParameterState","args":[]})
            .to_string();
    frame.push_str(&" ".repeat(PATCH_MESSAGE_BYTES_LIMIT - frame.len()));
    assert!(browser_request("7", &frame).is_some());
    frame.push(' ');
    assert!(browser_request("7", &frame).is_none());
}

#[test]
fn native_scripts_keep_full_precision_bootstrap_and_deliver_one_opaque_string_in_javascript() {
    let bootstrap = initialization_script(
        "18446744073709551615",
        "9007199254740993",
        NonZeroU64::new(u64::MAX).unwrap(),
    );
    let frame = "{\"result\":\"\\\"});globalThis.injected=true;//\"}\n\0雪\u{2028}";
    let mut script = bootstrap;
    script.push_str(&delivery_script(frame));
    let program = r#"const vm=require('node:vm'),fs=require('node:fs');
const received=[];const context={window:{__DANDRUM__:{receiveNativeMessage(value){received.push(value);}}}};
vm.runInNewContext(fs.readFileSync(0,'utf8'),context,{timeout:1000});
process.stdout.write(JSON.stringify({bootstrap:context.window.__DANDRUM_NATIVE__,received,injected:context.injected??null}));"#;
    let mut child = Command::new("node")
        .args(["-e", program])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let observed: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        observed,
        json!({"bootstrap":{"protocolVersion":1,"instanceId":"18446744073709551615","attachmentId":"9007199254740993","nextRequestId":"18446744073709551615"},"received":[frame],"injected":null})
    );
}
