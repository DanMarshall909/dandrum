use super::*;
use crate::test_support as common;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};
use x11rb::{
    COPY_DEPTH_FROM_PARENT,
    connection::Connection,
    protocol::xproto::{ConnectionExt, CreateWindowAux, MapState, WindowClass},
};

#[path = "local_policy.rs"]
mod local_policy;
#[path = "production.rs"]
mod production;
#[path = "resource_policy.rs"]
mod resource_policy;

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
            b"<!doctype html><button id='counter'>0</button><script src='/app.js'></script>",
        );
        package.write("app.js", b"let counter=0;const button=document.getElementById('counter');button.addEventListener('click',()=>{button.textContent=String(++counter);window.ipc.postMessage('content|'+counter)});button.addEventListener('inspect-state',()=>window.ipc.postMessage('state|'+counter));fetch('/absent').then(r => window.ipc.postMessage(r.status === 404 ? 'local-owner-ready' : 'bad-resource-status'))");
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
        exercise_visibility_and_fixed_size(&owner, &x, parent, child, baseline, cycle, &messages);
        let accepted_messages = messages.borrow().clone();
        let destroyed = Rc::new(Cell::new(0));
        let unrealized = Rc::new(Cell::new(0));
        let disposed = Rc::new(Cell::new(0));
        let count = destroyed.clone();
        owner
            ._window
            .0
            .connect_destroy(move |_| count.set(count.get() + 1));
        let count = unrealized.clone();
        owner
            ._window
            .0
            .connect_unrealize(move |_| count.set(count.get() + 1));
        let count = disposed.clone();
        let _weak_disposal_notification = owner
            ._window
            .0
            .add_weak_ref_notify_local(move || count.set(count.get() + 1));
        let weak_view = owner.view.webview().downgrade();
        let before = x.get_geometry(parent).unwrap().reply().unwrap();
        assert_eq!((before.width, before.height), (1400, 900));
        let drop_started = Instant::now();
        drop(owner);
        assert!(
            weak.upgrade().is_none(),
            "IPC callback releases before any host tick"
        );
        assert!(
            weak_view.upgrade().is_none(),
            "browser disposes before any host tick"
        );
        // No GTK servicing, wait, or test-side display synchronization before
        // the independent server observes this exact previously owned XID.
        let child_reply = x.get_window_attributes(child).unwrap().reply();
        let children = x.query_tree(parent).unwrap().reply().unwrap().children;
        let parent_geometry = x.get_geometry(parent).unwrap().reply().unwrap();
        let parent_state = x
            .get_window_attributes(parent)
            .unwrap()
            .reply()
            .unwrap()
            .map_state;
        eprintln!(
            "immediate drop cycle={cycle}, child={child}, reply={child_reply:?}, children={children:?}, destroy={}, unrealize={}, weak-disposal={}, elapsed={:?}",
            destroyed.get(),
            unrealized.get(),
            disposed.get(),
            drop_started.elapsed()
        );
        assert!(
            matches!(child_reply, Err(x11rb::errors::ReplyError::X11Error(ref error)) if error.error_kind == x11rb::protocol::ErrorKind::Window && error.error_code == 3 && error.bad_value == child),
            "owned XID must already be BadWindow when drop returns"
        );
        assert!(children.is_empty());
        assert_eq!(
            (
                parent_geometry.x,
                parent_geometry.y,
                parent_geometry.width,
                parent_geometry.height
            ),
            (before.x, before.y, 1400, 900)
        );
        assert_eq!(parent_state, MapState::VIEWABLE);
        assert_eq!(
            (destroyed.get(), unrealized.get(), disposed.get()),
            (1, 1, 1)
        );
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
            &accepted_messages,
            "no late callback after drop"
        );
    }
    initialized_child_consumes_native_delivery_before_releasing_its_callback(parent);
    acquisition_errors_leave_no_owned_window(&x, parent, root, baseline);
    local_policy::external_fetch_is_blocked_by_enforcing_csp(parent);
    local_policy::top_level_document_stays_at_fixed_entry(parent);
    production::unmodified_bundle_renders_silent_preview_with_all_eight_fonts(parent);
    resource_policy::external_resources_are_blocked_by_enforcing_csp(parent);
    parent_can_be_destroyed_immediately_after_editor(&x, root);
    x.destroy_window(parent).unwrap().check().unwrap();
}

fn initialized_child_consumes_native_delivery_before_releasing_its_callback(parent: u32) {
    let package = common::Package::new();
    for file in common::FILES {
        package.write(file, b"fixture");
    }
    package.write(
        "index.html",
        b"<!doctype html><output id='reply'></output><script src='/app.js'></script>",
    );
    package.write("app.js", br#"const bootstrap=window.__nativeHandshake;
window.ipc.postMessage('boot|'+bootstrap?.attachmentId+'|'+bootstrap?.label);
window.receiveNativeHandshake=reply=>{
  document.getElementById('reply').textContent=reply.value;
  window.ipc.postMessage('reply|'+reply.requestId+'|'+document.getElementById('reply').textContent+'|'+bootstrap.attachmentId);
};"#);
    let messages = Rc::new(RefCell::new(Vec::new()));
    let sink = messages.clone();
    let callback_lifetime = Rc::new(());
    let weak = Rc::downgrade(&callback_lifetime);
    let owner = NativeEditor::create_initialized(
        parent,
        EditorAssets::open(package.root()).unwrap(),
        r#"window.__nativeHandshake={attachmentId:"9007199254740993",label:"host π"};"#,
        move |message| {
            let _keep_alive = &callback_lifetime;
            sink.borrow_mut().push(message);
        },
    )
    .expect("initialized child accepts the real X11 parent");
    let deadline = Instant::now() + Duration::from_secs(15);
    while messages.borrow().is_empty() && Instant::now() < deadline {
        NativeEditor::service().unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        &*messages.borrow(),
        &["boot|9007199254740993|host π"],
        "first packaged application script sees the native initializer"
    );
    owner
        .evaluate_script(
            r#"window.receiveNativeHandshake({requestId:"18446744073709551615",value:"-0.5"});"#,
        )
        .expect("main-thread native delivery is queued");
    while messages.borrow().len() < 2 && Instant::now() < deadline {
        NativeEditor::service().unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        &*messages.borrow(),
        &[
            "boot|9007199254740993|host π",
            "reply|18446744073709551615|-0.5|9007199254740993",
        ],
        "actual JavaScript consumption changes the DOM and acknowledges through IPC"
    );
    drop(owner);
    assert!(
        weak.upgrade().is_none(),
        "initialized callback releases synchronously"
    );
    for _ in 0..8 {
        NativeEditor::service().unwrap();
    }
    assert_eq!(
        messages.borrow().len(),
        2,
        "no callback after owner destruction"
    );
}

fn exercise_visibility_and_fixed_size(
    owner: &NativeEditor,
    x: &RustConnection,
    parent: u32,
    child: u32,
    baseline: usize,
    cycle: usize,
    messages: &Rc<RefCell<Vec<String>>>,
) {
    let view = owner.view.webview();
    let host = x.get_geometry(parent).unwrap().reply().unwrap();
    let host_rect = (host.x, host.y, host.width, host.height);
    eprintln!(
        "visibility cycle={cycle}, parent={parent}, child={child}, GTK scale={}",
        owner._window.0.scale_factor()
    );
    let assert_state = |state| {
        assert_eq!(
            x.query_tree(parent).unwrap().reply().unwrap().children,
            [child]
        );
        assert_eq!(x.query_tree(child).unwrap().reply().unwrap().parent, parent);
        let geometry = x.get_geometry(child).unwrap().reply().unwrap();
        let attributes = x.get_window_attributes(child).unwrap().reply().unwrap();
        eprintln!(
            "native presentation: child={child}, parent={parent}, geometry={:?}, map={:?}",
            (geometry.x, geometry.y, geometry.width, geometry.height),
            attributes.map_state
        );
        assert_eq!(
            (geometry.x, geometry.y, geometry.width, geometry.height),
            (0, 0, 1200, 800)
        );
        assert_eq!(attributes.map_state, state);
        let host = x.get_geometry(parent).unwrap().reply().unwrap();
        assert_eq!((host.x, host.y, host.width, host.height), host_rect);
        assert_eq!(
            x.get_window_attributes(parent)
                .unwrap()
                .reply()
                .unwrap()
                .map_state,
            MapState::VIEWABLE
        );
        assert_eq!(gtk::Window::list_toplevels().len(), baseline + 1);
        assert_eq!(owner.view.webview(), view, "same WebKit object throughout");
        assert_eq!(
            view.uri().as_deref(),
            Some("dandrum://localhost/index.html")
        );
    };
    let host_ticks = || {
        for _ in 0..4 {
            NativeEditor::service().unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
        owner._window.0.display().sync();
    };
    let mut expected = vec!["local-owner-ready".to_string()];
    assert_state(MapState::UNMAPPED);
    owner.set_visible(true).unwrap();
    host_ticks();
    assert_state(MapState::VIEWABLE);
    expect_page_message(
        owner,
        messages,
        &mut expected,
        "document.getElementById('counter').click()",
        "content|1",
    );
    for visible in [true, false] {
        owner.set_visible(visible).unwrap();
        host_ticks();
        let state = if visible {
            MapState::VIEWABLE
        } else {
            MapState::UNMAPPED
        };
        assert_state(state);
        owner.set_size(1200, 800).unwrap();
        host_ticks();
        assert_state(state);
        for (width, height) in [
            (1199, 800),
            (1201, 800),
            (1200, 799),
            (1200, 801),
            (0, 800),
            (1200, 0),
            (u32::MAX, 800),
            (1200, u32::MAX),
            (65536 + 1200, 800),
            (1200, 65536 + 800),
        ] {
            let result = owner.set_size(width, height);
            eprintln!("fixed-size request {width}x{height}, shown={visible}, result={result:?}");
            assert!(
                matches!(result, Err(NativeError::UnsupportedSize)),
                "unsupported full-width request {width}x{height}"
            );
            host_ticks();
            assert_state(state);
            expect_page_message(
                owner,
                messages,
                &mut expected,
                "document.getElementById('counter').dispatchEvent(new Event('inspect-state'))",
                "state|1",
            );
        }
    }
    owner.set_visible(false).unwrap();
    host_ticks();
    assert_state(MapState::UNMAPPED);
    owner.set_visible(true).unwrap();
    host_ticks();
    assert_state(MapState::VIEWABLE);
    expect_page_message(
        owner,
        messages,
        &mut expected,
        "document.getElementById('counter').click()",
        "content|2",
    );
    // Exercise immediate disposal once while shown and once while hidden.
    if cycle == 1 {
        owner.set_visible(false).unwrap();
        host_ticks();
        assert_state(MapState::UNMAPPED);
    }
    eprintln!(
        "presentation page receipts cycle={cycle}: {:?}",
        messages.borrow()
    );
}

fn expect_page_message(
    owner: &NativeEditor,
    messages: &Rc<RefCell<Vec<String>>>,
    expected: &mut Vec<String>,
    script: &str,
    receipt: &str,
) {
    owner.view.evaluate_script(script).unwrap();
    expected.push(receipt.to_string());
    let deadline = Instant::now() + Duration::from_secs(15);
    while messages.borrow().len() < expected.len() && Instant::now() < deadline {
        NativeEditor::service().unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        &*messages.borrow(),
        expected,
        "ordinary page state survives presentation changes, with exactly one startup message"
    );
}

fn parent_can_be_destroyed_immediately_after_editor(x: &RustConnection, root: u32) {
    let parent = x.generate_id().unwrap();
    x.create_window(
        COPY_DEPTH_FROM_PARENT,
        parent,
        root,
        30,
        40,
        1400,
        900,
        0,
        WindowClass::INPUT_OUTPUT,
        0,
        &CreateWindowAux::new().override_redirect(1),
    )
    .unwrap()
    .check()
    .unwrap();
    x.map_window(parent).unwrap().check().unwrap();
    let roots = x.query_tree(root).unwrap().reply().unwrap().children;
    let package = common::Package::new();
    for file in common::FILES {
        package.write(file, b"fixture");
    }
    package.write(
        "index.html",
        b"<!doctype html><script src='/app.js'></script>",
    );
    package.write("app.js", b"window.ipc.postMessage('parent-ready');setTimeout(()=>window.ipc.postMessage('late-parent-callback'),100)");
    let messages = Rc::new(RefCell::new(Vec::new()));
    let sink = messages.clone();
    let callback = Rc::new(());
    let weak_callback = Rc::downgrade(&callback);
    let owner = NativeEditor::create(
        parent,
        EditorAssets::open(package.root()).unwrap(),
        move |message| {
            let _owned = &callback;
            sink.borrow_mut().push(message);
        },
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    while messages.borrow().is_empty() && Instant::now() < deadline {
        NativeEditor::service().unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(&*messages.borrow(), &["parent-ready"]);
    let children = x.query_tree(parent).unwrap().reply().unwrap().children;
    assert_eq!(children.len(), 1);
    let child = children[0];
    let weak_view = owner.view.webview().downgrade();
    drop(owner);
    assert!(
        weak_callback.upgrade().is_none(),
        "IPC callback releases before host parent destruction"
    );
    assert!(
        weak_view.upgrade().is_none(),
        "browser disposes before host parent destruction"
    );
    let reply = x.get_window_attributes(child).unwrap().reply();
    eprintln!(
        "immediate host-parent destruction: parent={parent}, child={child}, pre-parent-destroy reply={reply:?}"
    );
    assert!(
        matches!(reply, Err(x11rb::errors::ReplyError::X11Error(ref error)) if error.error_code == 3 && error.bad_value == child)
    );
    assert!(
        x.query_tree(parent)
            .unwrap()
            .reply()
            .unwrap()
            .children
            .is_empty()
    );
    // The host destroys its own parent with no GTK service tick in between.
    x.destroy_window(parent).unwrap().check().unwrap();
    for _ in 0..32 {
        NativeEditor::service().unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(weak_view.upgrade().is_none());
    assert!(weak_callback.upgrade().is_none());
    assert_eq!(&*messages.borrow(), &["parent-ready"]);
    assert!(
        x.query_tree(root)
            .unwrap()
            .reply()
            .unwrap()
            .children
            .iter()
            .all(|child| *child != parent && roots.contains(child))
    );
}

// Observe the real resource callback's release while its private container is
// still owned. Store identities only; no widget alias survives this callback.
struct CaptureWindowOnDrop {
    before: Vec<u32>,
    captured: std::sync::Arc<std::sync::Mutex<Vec<u32>>>,
}
fn gtk_window_ids() -> Vec<u32> {
    gtk::Window::list_toplevels()
        .iter()
        .filter_map(|window| window.window())
        .filter_map(|window| window.downcast::<gdkx11::X11Window>().ok())
        .map(|window| window.xid() as u32)
        .collect()
}
impl Drop for CaptureWindowOnDrop {
    fn drop(&mut self) {
        if gtk::is_initialized_main_thread() {
            self.captured.lock().unwrap().extend(
                gtk_window_ids()
                    .into_iter()
                    .filter(|id| !self.before.contains(id)),
            );
        }
    }
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
    let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let capture = CaptureWindowOnDrop {
        before: gtk_window_ids(),
        captured: captured.clone(),
    };
    assert!(matches!(
        NativeEditor::build_child(
            doomed,
            &checked,
            WebViewBuilder::new().with_custom_protocol("rollback".into(), move |_, _| {
                let _owned = &resource;
                let _identity = &capture;
                wry::http::Response::new(Cow::Borrowed(b"rollback" as &[u8]))
            })
        ),
        Err(NativeError::X11(_))
    ));
    let captured = captured.lock().unwrap();
    assert_eq!(
        captured.len(),
        1,
        "post-WebKit rollback container captured at resource release"
    );
    let reply = x.get_window_attributes(captured[0]).unwrap().reply();
    eprintln!(
        "immediate post-WebKit rollback: child={}, reply={reply:?}",
        captured[0]
    );
    assert!(
        matches!(reply, Err(x11rb::errors::ReplyError::X11Error(ref error)) if error.error_code == 3 && error.bad_value == captured[0])
    );
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
    let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let capture = CaptureWindowOnDrop {
        before: gtk_window_ids(),
        captured: captured.clone(),
    };
    let failed = NativeEditor::build_child(
        parent,
        &checked,
        WebViewBuilder::new_with_web_context(&mut context)
            .with_custom_protocol("capture".into(), move |_, _| {
                let _identity = &capture;
                wry::http::Response::new(Cow::Borrowed(b"never served" as &[u8]))
            })
            .with_custom_protocol("calibration".into(), move |_, _| {
                let _owned = &probe;
                wry::http::Response::new(Cow::Borrowed(b"never served" as &[u8]))
            }),
    );
    assert!(
        matches!(failed, Err(NativeError::WebView(wry::Error::DuplicateCustomProtocol(ref scheme)))
            if scheme == "calibration"),
        "the calibration reaches Wry's actual builder duplicate error: {:?}",
        failed.as_ref().err()
    );
    let captured = captured.lock().unwrap();
    assert_eq!(
        captured.len(),
        1,
        "builder-preflight rollback container captured at resource release"
    );
    let reply = x.get_window_attributes(captured[0]).unwrap().reply();
    eprintln!(
        "immediate builder-preflight rollback: child={}, reply={reply:?}",
        captured[0]
    );
    assert!(
        matches!(reply, Err(x11rb::errors::ReplyError::X11Error(ref error)) if error.error_code == 3 && error.bad_value == captured[0])
    );
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
    let cleanup_started = Instant::now();
    drop(first);
    for child in &children {
        let reply = x.get_window_attributes(*child).unwrap().reply();
        assert!(
            matches!(reply, Err(x11rb::errors::ReplyError::X11Error(ref error)) if error.error_code == 3 && error.bad_value == *child)
        );
    }
    assert!(
        x.query_tree(parent)
            .unwrap()
            .reply()
            .unwrap()
            .children
            .is_empty()
    );
    drop(context);
    let mut ticks = Vec::new();
    for _ in 0..8 {
        ticks.push(NativeEditor::service().unwrap());
    }
    let remaining = x.query_tree(parent).unwrap().reply().unwrap().children;
    eprintln!(
        "acquisition final cleanup: parent={parent}, children={remaining:?}, ticks={ticks:?}, elapsed={:?}",
        cleanup_started.elapsed()
    );
    assert_eq!(gtk::Window::list_toplevels().len(), baseline);
    assert!(remaining.is_empty());
}
