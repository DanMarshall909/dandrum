use super::*;
use std::collections::BTreeMap;
use webkit2gtk::{URIRequestExt, URIResponseExt, WebResourceExt, WebViewExt};

pub(super) fn unmodified_bundle_renders_silent_preview_with_all_eight_fonts(parent: u32) {
    let bundle = std::path::PathBuf::from(
        std::env::var_os("DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR")
            .expect("native production bundle input must be configured explicitly"),
    );
    let package = common::Package::new();
    let mut expected = BTreeMap::new();
    for path in common::FILES {
        let bytes = std::fs::read(bundle.join(path))
            .unwrap_or_else(|error| panic!("required production asset {path}: {error}"));
        package.write(path, &bytes);
        assert_eq!(
            std::fs::read(package.root().join("editor").join(path)).unwrap(),
            bytes
        );
        if !path.ends_with(".txt") {
            expected.insert(path.to_string(), bytes);
        }
    }
    let messages = Rc::new(RefCell::new(Vec::new()));
    let sink = messages.clone();
    let owner = NativeEditor::create(
        parent,
        EditorAssets::open(package.root()).unwrap(),
        move |message| sink.borrow_mut().push(message),
    )
    .unwrap();
    let view = owner.view.webview();
    let resources = Rc::new(RefCell::new(BTreeMap::new()));
    let failures = Rc::new(RefCell::new(Vec::new()));
    let failed = failures.clone();
    let received = resources.clone();
    let resource_handler = view.connect_resource_load_started(move |_, resource, request| {
        let uri = request.uri().unwrap_or_default().to_string();
        let failed = failed.clone();
        let failed_uri = uri.clone();
        resource.connect_failed(move |_, error| {
            failed.borrow_mut().push(format!("{failed_uri}: {error}"))
        });
        let received = received.clone();
        resource.connect_finished(move |resource| {
            let (status, mime) = resource
                .response()
                .map(|response| {
                    (
                        response.status_code(),
                        response.mime_type().unwrap_or_default().to_string(),
                    )
                })
                .unwrap_or_default();
            let received = received.clone();
            let uri = uri.clone();
            WebResourceExt::data(resource, None::<&gtk::gio::Cancellable>, move |data| {
                received
                    .borrow_mut()
                    .insert(uri, (status, mime, data.map_err(|error| error.to_string())));
            });
        });
    });
    let deadline = Instant::now() + Duration::from_secs(15);
    while (view.uri().as_deref() != Some("dandrum://localhost/index.html") || view.is_loading())
        && Instant::now() < deadline
    {
        NativeEditor::service().unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    owner
        .view
        .evaluate_script(include_str!("../fixtures/production-observer.js"))
        .unwrap();
    while Instant::now() < deadline {
        NativeEditor::service().unwrap();
        if messages
            .borrow()
            .iter()
            .any(|message| message.starts_with("production-error|"))
            || (messages
                .borrow()
                .iter()
                .any(|message| message == "production-ready|silent-preview|eight-fonts")
                && expected.keys().all(|path| {
                    resources
                        .borrow()
                        .contains_key(&format!("dandrum://localhost/{path}"))
                }))
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    eprintln!(
        "production browser receipts: {:?}; consumed resources: {:?}",
        messages.borrow(),
        resources
            .borrow()
            .iter()
            .map(|(uri, (status, mime, bytes))| (uri, status, mime, bytes.as_ref().map(Vec::len)))
            .collect::<Vec<_>>()
    );
    let mut expected_messages = vec![
        "production-preview|silent|mock|1200px|800px|16px|rgb(26, 21, 17)|inline-svg".to_string(),
    ];
    for (family, weights) in [
        ("Barlow", &[500, 600, 700][..]),
        ("Barlow Semi Condensed", &[500, 600, 700][..]),
        ("JetBrains Mono", &[500, 600][..]),
    ] {
        for weight in weights {
            expected_messages.push(format!("production-font|{family}|{weight}|loaded"));
        }
    }
    expected_messages.push("production-ready|silent-preview|eight-fonts".into());
    assert_eq!(
        &*messages.borrow(),
        &expected_messages,
        "actual production rendering and every font face"
    );
    assert!(
        failures.borrow().is_empty(),
        "no failed production resource: {:?}",
        failures.borrow()
    );
    for (path, bytes) in expected {
        let received = resources.borrow();
        let (status, mime, data) = received
            .get(&format!("dandrum://localhost/{path}"))
            .unwrap_or_else(|| panic!("browser resource {path} was observed"));
        assert_eq!(*status, 200);
        let expected_mime = if path.ends_with(".ttf") {
            "font/ttf"
        } else if path.ends_with(".js") {
            "text/javascript"
        } else if path.ends_with(".css") {
            "text/css"
        } else {
            "text/html"
        };
        assert_eq!(mime, expected_mime);
        assert!(
            data.as_ref().is_ok_and(|actual| actual == &bytes),
            "actual WebKit-consumed bytes for {path}: expected {} bytes, received {:?}",
            bytes.len(),
            data.as_ref().map(Vec::len)
        );
    }
    view.disconnect(resource_handler);
    let weak_view = view.downgrade();
    drop(view);
    drop(owner);
    assert!(
        weak_view.upgrade().is_none(),
        "production observer does not retain WebKit"
    );
}
