use super::*;
use crate::test_support as common;
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};
use x11rb::{
    COPY_DEPTH_FROM_PARENT,
    connection::Connection,
    protocol::xproto::{ConnectionExt, CreateWindowAux, MapState, WindowClass},
};

#[test]
#[ignore = "requires real Linux X11, GTK3 and WebKitGTK; native boundary lane"]
fn real_child_loads_local_content_and_drops_without_owning_the_host_parent() {
    gtk::init().expect("GTK on the host UI thread");
    let (x, screen) = x11rb::connect(None).unwrap();
    let root = x.setup().roots[screen].root;
    let parent = x.generate_id().unwrap();
    x.create_window(
        COPY_DEPTH_FROM_PARENT,
        parent,
        root,
        0,
        0,
        1400,
        900,
        0,
        WindowClass::INPUT_OUTPUT,
        0,
        &CreateWindowAux::new(),
    )
    .unwrap()
    .check()
    .unwrap();
    x.map_window(parent).unwrap().check().unwrap();
    let baseline = gtk::Window::list_toplevels().len();
    for cycle in 0..2 {
        let package = common::Package::new();
        for file in common::FILES {
            package.write(file, b"fixture");
        }
        package.write(
            "index.html",
            b"<!doctype html><script src='/app.js'></script>",
        );
        package.write("app.js", b"fetch('/absent').then(r => window.ipc.postMessage(r.status === 404 ? 'local-owner-ready' : 'bad-resource-status'))");
        let messages = Rc::new(RefCell::new(Vec::new()));
        let callback_lifetime = Rc::new(());
        let weak = Rc::downgrade(&callback_lifetime);
        let sink = messages.clone();
        let owner = NativeEditor::create(
            parent,
            EditorAssets::open(package.root()).unwrap(),
            move |message| {
                let _keep_alive = &callback_lifetime;
                sink.borrow_mut().push(message);
            },
        )
        .expect("native owner accepts a real host parent");
        let children = x.query_tree(parent).unwrap().reply().unwrap().children;
        assert_eq!(
            children.len(),
            1,
            "one direct editor child in cycle {cycle}"
        );
        let child = children[0];
        let geometry = x.get_geometry(child).unwrap().reply().unwrap();
        assert_eq!(
            (geometry.x, geometry.y, geometry.width, geometry.height),
            (0, 0, 1200, 800)
        );
        assert_eq!(
            x.get_window_attributes(child)
                .unwrap()
                .reply()
                .unwrap()
                .map_state,
            MapState::UNMAPPED
        );
        let deadline = Instant::now() + Duration::from_secs(15);
        while messages.borrow().is_empty() && Instant::now() < deadline {
            assert!(
                NativeEditor::service().unwrap() <= 64,
                "one finite host tick"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(&*messages.borrow(), &["local-owner-ready"]);
        let weak_view = owner.view.webview().downgrade();
        drop(owner);
        for _ in 0..8 {
            NativeEditor::service().unwrap();
        }
        assert!(
            x.query_tree(parent)
                .unwrap()
                .reply()
                .unwrap()
                .children
                .is_empty(),
            "owner drop releases its child"
        );
        assert_eq!(
            gtk::Window::list_toplevels().len(),
            baseline,
            "no retained GTK toplevel"
        );
        assert!(
            weak.upgrade().is_none(),
            "view-owned IPC closure is released"
        );
        assert!(
            weak_view.upgrade().is_none(),
            "WebKit view itself is released"
        );
        assert_eq!(
            &*messages.borrow(),
            &["local-owner-ready"],
            "no late callback after drop"
        );
    }
    acquisition_errors_leave_no_owned_window(&x, parent, root, baseline);
    x.destroy_window(parent).unwrap().check().unwrap();
}

fn acquisition_errors_leave_no_owned_window(
    x: &RustConnection,
    parent: u32,
    root: u32,
    baseline: usize,
) {
    // The source remains ready, but stops itself on call 65 if the owner's budget
    // regresses. That makes the independent count fail promptly instead of hanging.
    let calls = Rc::new(std::cell::Cell::new(0));
    let observed = calls.clone();
    let source = gtk::glib::idle_add_local(move || {
        observed.set(observed.get() + 1);
        if observed.get() == 65 {
            gtk::glib::ControlFlow::Break
        } else {
            gtk::glib::ControlFlow::Continue
        }
    });
    assert_eq!(
        NativeEditor::service().unwrap(),
        64,
        "host tick has a fixed iteration budget even with a ready source"
    );
    assert!(calls.get() > 0 && calls.get() <= 64);
    source.remove();
    assert!(matches!(
        std::thread::spawn(NativeEditor::service).join().unwrap(),
        Err(NativeError::WrongThread)
    ));
    let package = common::Package::new();
    for file in common::FILES {
        package.write(file, b"fixture");
    }
    let wrong_thread_assets = EditorAssets::open(package.root()).unwrap();
    assert!(
        std::thread::spawn(move || matches!(
            NativeEditor::create(parent, wrong_thread_assets, |_| {}),
            Err(NativeError::WrongThread)
        ))
        .join()
        .unwrap(),
        "creation rejects a non-UI thread before GTK access"
    );
    for invalid in [0, x.generate_id().unwrap()] {
        assert!(matches!(
            NativeEditor::create(invalid, EditorAssets::open(package.root()).unwrap(), |_| {}),
            Err(NativeError::InvalidParent)
        ));
        assert!(
            x.query_tree(parent)
                .unwrap()
                .reply()
                .unwrap()
                .children
                .is_empty()
        );
        assert_eq!(gtk::Window::list_toplevels().len(), baseline);
    }
    let input_only = x.generate_id().unwrap();
    x.create_window(
        0,
        input_only,
        root,
        0,
        0,
        1,
        1,
        0,
        WindowClass::INPUT_ONLY,
        0,
        &CreateWindowAux::new(),
    )
    .unwrap()
    .check()
    .unwrap();
    assert!(matches!(
        NativeEditor::create(
            input_only,
            EditorAssets::open(package.root()).unwrap(),
            |_| {}
        ),
        Err(NativeError::InvalidParent)
    ));
    x.destroy_window(input_only).unwrap().check().unwrap();

    // Split validation/acquisition models a parent disappearing during construction.
    // The error comes from the actual X server; there is no fake native backend.
    let doomed = x.generate_id().unwrap();
    x.create_window(
        COPY_DEPTH_FROM_PARENT,
        doomed,
        root,
        0,
        0,
        10,
        10,
        0,
        WindowClass::INPUT_OUTPUT,
        0,
        &CreateWindowAux::new(),
    )
    .unwrap()
    .check()
    .unwrap();
    let checked = NativeEditor::parent_connection(doomed).unwrap();
    x.destroy_window(doomed).unwrap().check().unwrap();
    let root_children = x.query_tree(root).unwrap().reply().unwrap().children;
    let resource = std::sync::Arc::new(());
    let weak_resource = std::sync::Arc::downgrade(&resource);
    assert!(matches!(
        NativeEditor::build_child(
            doomed,
            &checked,
            WebViewBuilder::new().with_custom_protocol("rollback".into(), move |_, _| {
                let _owned = &resource;
                wry::http::Response::new(Cow::Borrowed(b"rollback" as &[u8]))
            })
        ),
        Err(NativeError::X11(_))
    ));
    for _ in 0..8 {
        NativeEditor::service().unwrap();
    }
    assert!(
        x.query_tree(root)
            .unwrap()
            .reply()
            .unwrap()
            .children
            .iter()
            .all(|child| root_children.contains(child)),
        "failed reparent leaves no added root child"
    );
    assert_eq!(gtk::Window::list_toplevels().len(), baseline);
    assert!(
        weak_resource.upgrade().is_none(),
        "post-WebKit reparent failure releases the registered resource callback"
    );

    // Wry rejects a duplicate in its builder before allocating WebKit. The
    // owner has already allocated its GTK container and must still release it.
    // This calibration context is private; production uses a fresh context.
    let mut context = wry::WebContext::default();
    let first = NativeEditor::build_child(
        parent,
        &checked,
        WebViewBuilder::new_with_web_context(&mut context)
            .with_custom_protocol("calibration".into(), |_, _| {
                wry::http::Response::new(Cow::Borrowed(b"first" as &[u8]))
            }),
    )
    .unwrap();
    let children = x.query_tree(parent).unwrap().reply().unwrap().children;
    let probe = std::sync::Arc::new(());
    let weak = std::sync::Arc::downgrade(&probe);
    let failed = NativeEditor::build_child(
        parent,
        &checked,
        WebViewBuilder::new_with_web_context(&mut context).with_custom_protocol(
            "calibration".into(),
            move |_, _| {
                let _owned = &probe;
                wry::http::Response::new(Cow::Borrowed(b"never served" as &[u8]))
            },
        ),
    );
    assert!(
        matches!(failed, Err(NativeError::WebView(wry::Error::DuplicateCustomProtocol(ref scheme)))
            if scheme == "calibration"),
        "the calibration reaches Wry's actual builder duplicate error: {:?}",
        failed.as_ref().err()
    );
    for _ in 0..8 {
        NativeEditor::service().unwrap();
    }
    assert_eq!(
        x.query_tree(parent).unwrap().reply().unwrap().children,
        children
    );
    assert_eq!(
        gtk::Window::list_toplevels().len(),
        baseline + 1,
        "only the deliberately retained first owner"
    );
    assert!(
        weak.upgrade().is_none(),
        "rejected builder releases its unregistered resource callback"
    );
    drop(first);
    drop(context);
    for _ in 0..8 {
        NativeEditor::service().unwrap();
    }
    assert_eq!(gtk::Window::list_toplevels().len(), baseline);
    assert!(
        x.query_tree(parent)
            .unwrap()
            .reply()
            .unwrap()
            .children
            .is_empty()
    );
}
