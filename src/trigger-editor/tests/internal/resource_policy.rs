use super::{EditorAssets, NativeEditor, local_policy};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

pub(super) fn external_resources_are_blocked_by_enforcing_csp(parent: u32) {
    let bundle = std::path::PathBuf::from(
        std::env::var_os("DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR")
            .expect("native production bundle input must be configured explicitly"),
    );
    let font = std::fs::read(bundle.join("fonts/91c841fdfa8e7b94.ttf")).unwrap();
    let resources = vec![
        (
            "/external.js",
            "text/javascript",
            b"window.externalResourceMarker='owned-script-executed';".to_vec(),
        ),
        (
            "/external.css",
            "text/css",
            b"#external-control { height:61px !important; }".to_vec(),
        ),
        ("/external.ttf", "font/ttf", font),
        (
            "/external.png",
            "image/png",
            include_bytes!("../fixtures/resource-policy.png").to_vec(),
        ),
    ];
    let endpoint = local_policy::Endpoint::with_resources(resources.clone());
    endpoint.calibrate();
    // Independently check exact bytes and MIME, including the valid font/PNG;
    // these requests are distinct from the subsequent ordinary-page attempts.
    for (route, mime, body) in &resources {
        let response = endpoint.read_response_bytes(route);
        let split = response.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let headers = std::str::from_utf8(&response[..split]).unwrap();
        assert!(headers.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(headers.contains(&format!("Content-Type: {mime}\r\n")));
        assert_eq!(&response[split + 4..], body);
    }
    let calibrated = endpoint.requests.lock().unwrap().clone();
    let package = local_policy::page_package(
        include_str!("../fixtures/resource-policy.mjs")
            .replace("__RESOURCE_POLICY_ENDPOINT__", &endpoint.origin()),
    );
    let messages = Rc::new(RefCell::new(Vec::new()));
    let sink = messages.clone();
    let owner = NativeEditor::create(
        parent,
        EditorAssets::open(package.root()).unwrap(),
        move |message| sink.borrow_mut().push(message),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    while !messages.borrow().iter().any(|m| m == "resource-done") && Instant::now() < deadline {
        NativeEditor::service().unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    let observed = messages.borrow();
    let requests = endpoint.requests.lock().unwrap();
    eprintln!(
        "resource browser receipts: {observed:?}; owned endpoint requests: {requests:?}; independent calibration: {calibrated:?}"
    );
    assert!(
        observed.iter().any(|m| m == "resource-done"),
        "all four ordinary-page attempts complete"
    );
    assert!(observed.iter().any(|m| m == "resource-local|404|43px|37px"));
    // Aggregate every denial row before asserting, so a first failure cannot
    // hide successful external effects or missing enforcement in other classes.
    let mut failures = Vec::new();
    for expected in [
        "resource-script|rejected|absent",
        "resource-style|rejected|7px",
        "resource-font|error|ResourcePolicyExternal|500|false",
        "resource-image|rejected|0|0",
    ] {
        if !observed.iter().any(|message| message == expected) {
            failures.push(format!("missing denial effect {expected}"));
        }
    }
    for (directive, route) in [
        ("script-src-elem", "/external.js"),
        ("style-src-elem", "/external.css"),
        ("font-src", "/external.ttf"),
        ("img-src", "/external.png"),
    ] {
        let expected = format!(
            "resource-policy|{directive}|{directive}|enforce|{}{route}|dandrum://localhost/index.html",
            endpoint.origin()
        );
        if !observed.iter().any(|message| message == &expected) {
            failures.push(format!("missing exact enforcing violation {expected}"));
        }
    }
    if *requests != calibrated {
        failures.push(format!(
            "external browser GETs reached endpoint: {:?}",
            &requests[calibrated.len()..]
        ));
    }
    assert!(
        failures.is_empty(),
        "four-row enforcing resource CSP: {failures:#?}"
    );
    drop(owner);
}
