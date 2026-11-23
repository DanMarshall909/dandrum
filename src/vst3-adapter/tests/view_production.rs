#![cfg(all(
    target_os = "linux",
    target_arch = "x86_64",
    feature = "trigger-editor"
))]
//! Public GTK/WebKit boundary lane; this executable does not prove loader residency.
mod common;
use common::{audio::*, runloop, *};
use gtk::prelude::*;
use std::{
    cell::RefCell,
    path::Path,
    rc::Rc,
    time::{Duration, Instant},
};
use vst3::Steinberg::{Vst::*, *};
use webkit2gtk::WebViewExt;
use x11rb::{
    COPY_DEPTH_FROM_PARENT,
    connection::Connection,
    protocol::xproto::{ConnectionExt, CreateWindowAux, WindowClass},
    rust_connection::RustConnection,
};

#[derive(Default)]
struct DeliveryFaults {
    drop_selection_ack: std::cell::Cell<bool>,
    results: std::cell::Cell<usize>,
}
struct Proxy {
    target: vst3::ComPtr<IConnectionPoint>,
    faults: Rc<DeliveryFaults>,
}
impl vst3::Class for Proxy {
    type Interfaces = (IConnectionPoint,);
}
impl IConnectionPointTrait for Proxy {
    unsafe fn connect(&self, _: *mut IConnectionPoint) -> tresult {
        kNotImplemented
    }
    unsafe fn disconnect(&self, _: *mut IConnectionPoint) -> tresult {
        kNotImplemented
    }
    unsafe fn notify(&self, message: *mut IMessage) -> tresult {
        let packet = unsafe { common::message::decode(message) };
        if packet["body"]["type"] == "nativeSelectionAck" && self.faults.drop_selection_ack.get() {
            return kResultOk;
        }
        if packet["body"]["type"] == "nativeSelection" {
            self.faults.results.set(self.faults.results.get() + 1);
            assert!(
                self.faults.results.get() < 4096,
                "bounded selected-result retries"
            );
        }
        unsafe { self.target.notify(message) }
    }
}

struct Parent {
    x: RustConnection,
    id: u32,
}
impl Parent {
    fn new() -> Self {
        let (x, screen) = x11rb::connect(None).expect("inherited real X11 display");
        let id = x.generate_id().unwrap();
        x.create_window(
            COPY_DEPTH_FROM_PARENT,
            id,
            x.setup().roots[screen].root,
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
        x.map_window(id).unwrap().check().unwrap();
        Self { x, id }
    }
    fn children(&self) -> Vec<u32> {
        self.x
            .query_tree(self.id)
            .unwrap()
            .reply()
            .unwrap()
            .children
    }
}
impl Drop for Parent {
    fn drop(&mut self) {
        self.x.destroy_window(self.id).unwrap().check().unwrap();
    }
}

fn weak_browser_in_child(child: u32) -> gtk::glib::WeakRef<gtk::glib::Object> {
    use gtk::glib::{prelude::ObjectExt, translate::from_glib_none};
    unsafe extern "C" {
        fn gdk_x11_window_get_xid(window: *mut gtk::gdk::ffi::GdkWindow) -> std::ffi::c_ulong;
    }
    fn visit(
        widget: *mut gtk::ffi::GtkWidget,
        depth: usize,
        found: &mut Vec<gtk::glib::WeakRef<gtk::glib::Object>>,
    ) {
        assert!(depth < 32, "finite native widget tree");
        // GTK owns the live tree; inspection does not dispatch or retain a strong object.
        let object: gtk::glib::Object =
            unsafe { from_glib_none(widget as *mut gtk::glib::gobject_ffi::GObject) };
        if object.type_().name() == "WebKitWebView" {
            found.push(object.downgrade());
        }
        if unsafe {
            gtk::glib::gobject_ffi::g_type_check_instance_is_a(
                widget as *mut gtk::glib::gobject_ffi::GTypeInstance,
                gtk::ffi::gtk_container_get_type(),
            )
        } != 0
        {
            let children = unsafe {
                gtk::ffi::gtk_container_get_children(widget as *mut gtk::ffi::GtkContainer)
            };
            let mut node = children;
            while !node.is_null() {
                unsafe {
                    visit((*node).data as *mut gtk::ffi::GtkWidget, depth + 1, found);
                    node = (*node).next;
                }
            }
            unsafe {
                gtk::glib::ffi::g_list_free(children);
            }
        }
    }
    let windows = unsafe { gtk::ffi::gtk_window_list_toplevels() };
    let mut node = windows;
    let mut found = Vec::new();
    while !node.is_null() {
        let widget = unsafe { (*node).data as *mut gtk::ffi::GtkWidget };
        let window = unsafe { gtk::ffi::gtk_widget_get_window(widget) };
        if !window.is_null() && unsafe { gdk_x11_window_get_xid(window) } == u64::from(child) {
            visit(widget, 0, &mut found);
        }
        node = unsafe { (*node).next };
    }
    unsafe {
        gtk::glib::ffi::g_list_free(windows);
    }
    assert_eq!(
        found.len(),
        1,
        "exact actual child contains one runtime-typed WebKitWebView"
    );
    found.pop().unwrap()
}

fn dispatch_context(clock: &runloop::LoopState) {
    clock.tick();
    let context = gtk::glib::MainContext::default();
    for _ in 0..64 {
        if !context.pending() {
            break;
        }
        context.iteration(false);
    }
}
fn copy_directory(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            copy_directory(&entry.path(), &target.join(entry.file_name()));
        } else {
            assert!(kind.is_file(), "package contains only regular assets");
            std::fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
        }
    }
}
fn open_view(
    controller: &vst3::ComPtr<IEditController>,
    parent: &Parent,
    clock: &runloop::LoopState,
) -> (vst3::ComPtr<IPlugView>, webkit2gtk::WebView) {
    let view =
        unsafe { vst3::ComPtr::from_raw(controller.createView(c"editor".as_ptr())) }.unwrap();
    assert_eq!(
        unsafe { view.attached(parent.id as usize as *mut _, kPlatformTypeX11EmbedWindowID) },
        kResultOk
    );
    gtk::init().expect("host joins the plugin's GTK UI thread");
    let browser = weak_browser_in_child(parent.children()[0])
        .upgrade()
        .unwrap()
        .downcast::<webkit2gtk::WebView>()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(12);
    while javascript(
        clock,
        &browser,
        "String(Boolean(document.querySelector('[data-testid=trigger-editor]')))",
    ) != "true"
    {
        assert!(
            Instant::now() < deadline,
            "actual packaged React editor initializes"
        );
    }
    assert_eq!(
        javascript(clock, &browser, "window.innerWidth+'x'+window.innerHeight"),
        "1200x800"
    );
    (view, browser)
}
fn native_dialog(clock: &runloop::LoopState) -> gtk::FileChooserDialog {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        dispatch_context(clock);
        let mut dialogs = gtk::Window::list_toplevels()
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::FileChooserDialog>().ok());
        if let Some(dialog) = dialogs.next() {
            assert!(dialogs.next().is_none());
            return dialog;
        }
        assert!(
            Instant::now() < deadline,
            "one actual GTK chooser must open"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn javascript(clock: &runloop::LoopState, browser: &webkit2gtk::WebView, script: &str) -> String {
    let result = Rc::new(RefCell::new(None));
    let received = Rc::clone(&result);
    browser.run_javascript(script, None::<&gtk::gio::Cancellable>, move |value| {
        *received.borrow_mut() = Some(
            value
                .map(|v| v.js_value().expect("JavaScript result value").to_string())
                .map_err(|e| e.to_string()),
        );
    });
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        dispatch_context(clock);
        if let Some(value) = result.borrow_mut().take() {
            return value.expect("public WebKit evaluation succeeds");
        }
        assert!(
            Instant::now() < deadline,
            "public WebKit evaluation did not complete"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn invoke(
    clock: &runloop::LoopState,
    browser: &webkit2gtk::WebView,
    expression: &str,
) -> serde_json::Value {
    javascript(
        clock,
        browser,
        &format!(
            "window.__G2_RESULT__=null;({expression}).then(result=>window.__G2_RESULT__={{status:'accepted',result}},error=>window.__G2_RESULT__={{status:'rejected',code:error.code}});'started'"
        ),
    );
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let value = javascript(clock, browser, "JSON.stringify(window.__G2_RESULT__)");
        if value != "null" {
            return serde_json::from_str(&value).unwrap();
        }
        assert!(
            Instant::now() < deadline,
            "native command Promise did not settle"
        );
    }
}
const SELECTED_WAV: &[u8] = &[
    82, 73, 70, 70, 52, 0, 0, 0, 87, 65, 86, 69, 102, 109, 116, 32, 16, 0, 0, 0, 1, 0, 2, 0, 0,
    125, 0, 0, 0, 244, 1, 0, 4, 0, 16, 0, 100, 97, 116, 97, 16, 0, 0, 0, 0, 160, 0, 64, 0, 32, 0,
    224, 0, 192, 0, 16, 0, 64, 0, 160,
];
// Independently computed from the literal 60-byte WAV, not a decoder round trip.
const CONTENT_SHA256: &str = "2e484972f38876c961b3c23fb4952487dce3e18501c7fb0efd1b8cb7011734d5";

#[test]
#[ignore = "real production React/Wry/GTK and copied VST3 boundary; explicit packaged assets/display"]
fn production_native_selection_retains_decoded_content_after_source_and_view_loss() {
    let bundle = Bundle::new();
    let assets = std::env::var_os("DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR")
        .expect("explicit complete packaged native editor");
    copy_directory(Path::new(&assets), &bundle.resources.join("editor"));
    let parent = Parent::new();
    let module = bundle.load();
    let (clock, factory, _host) = runloop::host(&module);
    let processor = module.class_from::<IComponent>(&factory, &PROCESSOR);
    let controller = module.class_from::<IEditController>(&factory, &CONTROLLER);
    assert_eq!(
        unsafe { processor.initialize(std::ptr::null_mut()) },
        kResultOk
    );
    assert_eq!(
        unsafe { controller.initialize(std::ptr::null_mut()) },
        kResultOk
    );
    let p = processor.cast::<IConnectionPoint>().unwrap();
    let c = controller.cast::<IConnectionPoint>().unwrap();
    let faults = Rc::new(DeliveryFaults::default());
    faults.drop_selection_ack.set(true);
    let to_c = vst3::ComWrapper::new(Proxy {
        target: c.clone(),
        faults: faults.clone(),
    })
    .to_com_ptr::<IConnectionPoint>()
    .unwrap();
    let to_p = vst3::ComWrapper::new(Proxy {
        target: p.clone(),
        faults: faults.clone(),
    })
    .to_com_ptr::<IConnectionPoint>()
    .unwrap();
    assert_eq!(unsafe { p.connect(to_c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { c.connect(to_p.as_ptr()) }, kResultOk);
    let audio = processor.cast::<IAudioProcessor>().unwrap();
    start(&processor, &audio, 48000.0, 16);
    let notes = events(vec![note_on(5, 36, 1.0)]);
    let mut block = Block::new(16);
    let mut data = block.data();
    data.inputEvents = notes.as_ptr();
    assert_eq!(unsafe { audio.process(&mut data) }, kResultOk);
    assert_eq!((block.left[5], block.right[5]), (-0.5, -0.5));
    assert_eq!(unsafe { audio.setProcessing(0) }, kResultOk);
    let (view, browser) = open_view(&controller, &parent, &clock);
    let selected = tempfile::tempdir().unwrap();
    let path = selected.path().join("distinct-selected.wav");
    std::fs::write(&path, SELECTED_WAV).unwrap();
    let before = invoke(
        &clock,
        &browser,
        "window.__DANDRUM__.backend.getNativeFunction('getSamplerSession')()",
    );
    assert_eq!(before["status"], "accepted");
    let started = invoke(
        &clock,
        &browser,
        "window.__DANDRUM__.backend.getNativeFunction('chooseSamplerFiles')()",
    );
    assert_eq!(
        started["status"], "accepted",
        "production browser must admit native chooser: {started}"
    );
    assert_eq!(started["result"]["phase"], "choosing");
    let id = started["result"]["selectionId"].as_str().unwrap();
    let chooser = native_dialog(&clock);
    assert!(chooser.set_filename(&path));
    let deadline = Instant::now() + Duration::from_secs(8);
    while chooser.filename().as_deref() != Some(path.as_path()) {
        dispatch_context(&clock);
        assert!(Instant::now() < deadline);
    }
    chooser.response(gtk::ResponseType::Accept);
    drop(chooser);
    let expression = format!(
        "window.__DANDRUM__.backend.getNativeFunction('getSamplerSelectionStatus')({})",
        serde_json::to_string(id).unwrap()
    );
    let ready = loop {
        let status = invoke(&clock, &browser, &expression);
        assert_eq!(status["status"], "accepted");
        if status["result"]["phase"] == "ready" {
            break status["result"].clone();
        }
        assert_eq!(status["result"]["phase"], "preparing", "{status}");
        assert!(Instant::now() < deadline, "native decode must complete");
    };
    let file = &ready["files"][0];
    assert_eq!(ready["files"].as_array().unwrap().len(), 1);
    assert_eq!(file["contentSha256"], CONTENT_SHA256);
    assert_eq!(file["encodedBytes"], 60);
    assert_eq!(file["sampleRateHz"], 32000);
    assert_eq!(file["channelCount"], 2);
    assert_eq!(file["frameCount"], 4);
    selected.close().unwrap();
    for _ in 0..3 {
        clock.tick();
    }
    assert!(
        faults.results.get() > 1,
        "lost selection acknowledgements must retry the retained native result"
    );
    assert_eq!(
        invoke(&clock, &browser, &expression)["result"],
        ready,
        "retries cannot reopen a deleted source or change its outcome"
    );
    faults.drop_selection_ack.set(false);
    for _ in 0..3 {
        clock.tick();
    }
    let settled = faults.results.get();
    for _ in 0..3 {
        clock.tick();
    }
    assert_eq!(
        faults.results.get(),
        settled,
        "application acknowledgement retires native result retries"
    );
    assert_eq!(unsafe { view.removed() }, kResultOk);
    drop(browser);
    drop(view);
    assert!(parent.children().is_empty());
    let (view, browser) = open_view(&controller, &parent, &clock);
    assert_eq!(invoke(&clock, &browser, &expression)["result"], ready);
    let after = invoke(
        &clock,
        &browser,
        "window.__DANDRUM__.backend.getNativeFunction('getSamplerSession')()",
    );
    assert_eq!(
        after["result"]["yaml"], before["result"]["yaml"],
        "selection alone cannot change accepted instrument/history"
    );
    assert_eq!(
        after["result"]["connection"]["history"],
        before["result"]["connection"]["history"]
    );
    let cancelled = invoke(
        &clock,
        &browser,
        "window.__DANDRUM__.backend.getNativeFunction('chooseSamplerFiles')()",
    );
    assert_eq!(cancelled["status"], "accepted");
    let cancelled_expression = format!(
        "window.__DANDRUM__.backend.getNativeFunction('getSamplerSelectionStatus')({})",
        cancelled["result"]["selectionId"]
    );
    let chooser = native_dialog(&clock);
    chooser.response(gtk::ResponseType::Cancel);
    drop(chooser);
    assert_eq!(
        invoke(&clock, &browser, &cancelled_expression)["result"]["phase"],
        "cancelled"
    );
    let abandoned = invoke(
        &clock,
        &browser,
        "window.__DANDRUM__.backend.getNativeFunction('chooseSamplerFiles')()",
    );
    assert_eq!(abandoned["status"], "accepted");
    let abandoned_expression = format!(
        "window.__DANDRUM__.backend.getNativeFunction('getSamplerSelectionStatus')({})",
        abandoned["result"]["selectionId"]
    );
    let chooser = native_dialog(&clock);
    use gtk::glib::translate::ToGlibPtr;
    unsafe extern "C" {
        fn gdk_x11_window_get_xid(window: *mut gtk::gdk::ffi::GdkWindow) -> std::ffi::c_ulong;
    }
    let window = chooser.window().unwrap();
    let dialog_xid = unsafe { gdk_x11_window_get_xid(window.to_glib_none().0) } as u32;
    drop(window);
    drop(chooser);
    assert_eq!(unsafe { view.removed() }, kResultOk);
    drop(browser);
    drop(view);
    assert!(
        parent.x.query_tree(dialog_xid).unwrap().reply().is_err(),
        "removing the view destroys its real open chooser"
    );
    let (view, browser) = open_view(&controller, &parent, &clock);
    assert_eq!(
        invoke(&clock, &browser, &abandoned_expression)["result"]["phase"],
        "cancelled"
    );
    let pending = invoke(
        &clock,
        &browser,
        "window.__DANDRUM__.backend.getNativeFunction('chooseSamplerFiles')()",
    );
    assert_eq!(pending["status"], "accepted");
    let pending_expression = format!(
        "window.__DANDRUM__.backend.getNativeFunction('getSamplerSelectionStatus')({})",
        pending["result"]["selectionId"]
    );
    let pending_source = tempfile::tempdir().unwrap();
    let pending_path = pending_source.path().join("admitted.wav");
    std::fs::write(&pending_path, SELECTED_WAV).unwrap();
    let chooser = native_dialog(&clock);
    assert!(chooser.set_filename(&pending_path));
    let deadline = Instant::now() + Duration::from_secs(5);
    while chooser.filename().as_deref() != Some(pending_path.as_path()) {
        dispatch_context(&clock);
        assert!(Instant::now() < deadline);
    }
    chooser.response(gtk::ResponseType::Accept);
    drop(chooser);
    let before_admission = faults.results.get();
    clock.tick();
    assert!(
        faults.results.get() > before_admission,
        "native paths enter processor admission before view loss"
    );
    assert_eq!(unsafe { view.removed() }, kResultOk);
    drop(browser);
    drop(view);
    pending_source.close().unwrap();
    for _ in 0..8 {
        clock.tick();
        std::thread::sleep(Duration::from_millis(2));
    }
    let (view, browser) = open_view(&controller, &parent, &clock);
    let completed = invoke(&clock, &browser, &pending_expression);
    assert_eq!(
        completed["result"]["phase"], "ready",
        "admitted decoding finishes while stopped and editor-absent"
    );
    assert_eq!(
        completed["result"]["files"][0]["contentSha256"],
        CONTENT_SHA256
    );
    assert_eq!(unsafe { view.removed() }, kResultOk);
    drop(browser);
    drop(view);
    assert_eq!(unsafe { controller.terminate() }, kResultOk);
    assert_eq!(unsafe { processor.terminate() }, kResultOk);
    assert!(clock.handlers.borrow().is_empty());
}
