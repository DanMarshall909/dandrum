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

#[derive(Clone, Copy, Debug, PartialEq)]
enum HostEdit {
    Begin(u32),
    Value(u32, f64),
    End(u32),
}
struct Handler(Rc<RefCell<Vec<HostEdit>>>);
impl vst3::Class for Handler {
    type Interfaces = (IComponentHandler,);
}
impl IComponentHandlerTrait for Handler {
    unsafe fn beginEdit(&self, id: u32) -> tresult {
        self.0.borrow_mut().push(HostEdit::Begin(id));
        kResultOk
    }
    unsafe fn performEdit(&self, id: u32, value: f64) -> tresult {
        self.0.borrow_mut().push(HostEdit::Value(id, value));
        kResultOk
    }
    unsafe fn endEdit(&self, id: u32) -> tresult {
        self.0.borrow_mut().push(HostEdit::End(id));
        kResultOk
    }
    unsafe fn restartComponent(&self, _: i32) -> tresult {
        kResultOk
    }
}

struct BitwigHost;
impl vst3::Class for BitwigHost {
    type Interfaces = (IHostApplication,);
}
impl IHostApplicationTrait for BitwigHost {
    unsafe fn getName(&self, output: *mut String128) -> tresult {
        let Some(output) = (unsafe { output.as_mut() }) else {
            return kInvalidArgument;
        };
        output.fill(0);
        for (slot, unit) in output.iter_mut().zip("Bitwig Studio".encode_utf16()) {
            *slot = unit;
        }
        kResultOk
    }
    unsafe fn createInstance(
        &self,
        _: *mut TUID,
        _: *mut TUID,
        _: *mut *mut std::ffi::c_void,
    ) -> tresult {
        kNotImplemented
    }
}
fn saved_state(processor: &vst3::ComPtr<IComponent>) -> Vec<u8> {
    let (state, stream) = common::stream::stream(Vec::new(), 4096);
    assert_eq!(unsafe { processor.getState(stream.as_ptr()) }, kResultOk);
    state.bytes.borrow().clone()
}
fn assert_mapping_audio(audio: &vst3::ComPtr<IAudioProcessor>, selected: bool, gain: f32) {
    for (key, expected) in [
        (48, if selected { -0.125 * gain } else { 0.0 }),
        (36, -0.5 * gain),
        (60, 0.0),
    ] {
        assert_eq!(unsafe { audio.setProcessing(1) }, kResultOk);
        let notes = events(vec![note_on(0, key, 1.0)]);
        let mut block = Block::new(16);
        let mut data = block.data();
        data.inputEvents = notes.as_ptr();
        assert_eq!(unsafe { audio.process(&mut data) }, kResultOk);
        assert_eq!(
            (block.left[0], block.right[0]),
            (expected, expected),
            "literal signed actual native mapping at MIDI{key}"
        );
        assert_eq!(unsafe { audio.setProcessing(0) }, kResultOk);
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
        if !window.is_null() {
            let (x, _) = x11rb::connect(None).unwrap();
            let mut current = unsafe { gdk_x11_window_get_xid(window) } as u32;
            for _ in 0..32 {
                if current == child {
                    visit(widget, 0, &mut found);
                    break;
                }
                let tree = x11rb::protocol::xproto::ConnectionExt::query_tree(&x, current)
                    .unwrap()
                    .reply()
                    .unwrap();
                if tree.parent == tree.root || tree.parent == 0 {
                    break;
                }
                current = tree.parent;
            }
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
    let dot:Vec<u16>=serde_json::from_str(&javascript(clock,&browser,"JSON.stringify((()=>{const dot=Array.from(document.querySelectorAll('header span')).find(el=>{const r=el.getBoundingClientRect();return r.width===6&&r.height===6});if(!dot)throw new Error('header accent absent');const r=dot.getBoundingClientRect();return [Math.floor(r.left+3),Math.floor(r.top+3)]})())")).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        dispatch_context(clock);
        let pixel = parent
            .x
            .get_image(
                x11rb::protocol::xproto::ImageFormat::Z_PIXMAP,
                parent.children()[0],
                dot[0] as i16,
                dot[1] as i16,
                1,
                1,
                u32::MAX,
            )
            .unwrap()
            .reply()
            .unwrap()
            .data;
        if pixel.get(..3) == Some(&[0x4e, 0x8a, 0xe0]) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "actual packaged React paints its literal #E08A4E header accent: {pixel:?}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
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
fn wait_dom(clock: &runloop::LoopState, browser: &webkit2gtk::WebView, expression: &str) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while javascript(clock, browser, &format!("String(Boolean({expression}))")) != "true" {
        assert!(
            Instant::now() < deadline,
            "DOM condition did not become true: {expression}; input state: {}",
            javascript(
                clock,
                browser,
                "JSON.stringify({focused:document.hasFocus(),active:document.activeElement?.outerHTML,level:document.querySelector('[data-knob=\"drums.level\"] input')?.value,trace:window.__VST3_INPUT_TRACE__})"
            )
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn type_level(clock: &runloop::LoopState, browser: &webkit2gtk::WebView, physical: &str) {
    javascript(
        clock,
        browser,
        "(()=>{const tab=Array.from(document.querySelectorAll('[role=tab]')).find(tab=>tab.textContent.trim()==='Voice');if(!tab)throw new Error('Voice tab absent');tab.click();return 'clicked';})()",
    );
    wait_dom(
        clock,
        browser,
        "document.querySelector('[data-knob=\"drums.level\"] [role=slider]')",
    );
    javascript(
        clock,
        browser,
        "(()=>{const slider=document.querySelector('[data-knob=\"drums.level\"] [role=slider]');slider.focus();slider.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}));return 'editing';})()",
    );
    wait_dom(
        clock,
        browser,
        "document.querySelector('[data-knob=\"drums.level\"] input')",
    );
    javascript(
        clock,
        browser,
        &format!(
            "(()=>{{const input=document.querySelector('[data-knob=\"drums.level\"] input');Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(input,{physical:?});input.dispatchEvent(new Event('input',{{bubbles:true}}));return input.value;}})()"
        ),
    );
    javascript(
        clock,
        browser,
        "(()=>{const input=document.querySelector('[data-knob=\"drums.level\"] input');input.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}));return 'committed';})()",
    );
    wait_dom(
        clock,
        browser,
        "!document.querySelector('[data-knob=\"drums.level\"] input')",
    );
}
const SELECTED_WAV: &[u8] = &[
    82, 73, 70, 70, 52, 0, 0, 0, 87, 65, 86, 69, 102, 109, 116, 32, 16, 0, 0, 0, 1, 0, 2, 0, 0,
    125, 0, 0, 0, 244, 1, 0, 4, 0, 16, 0, 100, 97, 116, 97, 16, 0, 0, 0, 0, 160, 0, 64, 0, 32, 0,
    224, 0, 192, 0, 16, 0, 64, 0, 160,
];
// Independently computed from the literal 60-byte WAV, not a decoder round trip.
const CONTENT_SHA256: &str = "2e484972f38876c961b3c23fb4952487dce3e18501c7fb0efd1b8cb7011734d5";

#[derive(Clone, Copy)]
enum CallbackAudit {
    Unarmed,
    Armed {
        begin: unsafe extern "C" fn(),
        end: unsafe extern "C" fn(*mut u64, *mut u64),
    },
}
impl CallbackAudit {
    fn observe<T>(&self, call: impl FnOnce() -> T) -> (T, Option<(u64, u64)>) {
        match self {
            Self::Unarmed => (call(), None),
            Self::Armed { begin, end } => {
                unsafe {
                    begin();
                }
                let result = call();
                let (mut allocated, mut freed) = (0, 0);
                unsafe {
                    end(&mut allocated, &mut freed);
                }
                (result, Some((allocated, freed)))
            }
        }
    }
    fn clean<T>(&self, call: impl FnOnce() -> T) -> T {
        let (result, counts) = self.observe(call);
        if let Some(counts) = counts {
            assert_eq!(
                counts,
                (0, 0),
                "complete native audio callback allocation/free sentinel"
            );
        }
        result
    }
    fn load() -> Self {
        if std::env::var_os("DANDRUM_VST3_NATIVE_AUDIT").is_none() {
            return Self::Unarmed;
        }
        let global = libloading::os::unix::Library::this();
        let audit = unsafe {
            Self::Armed {
                begin: *global.get(b"dandrum_audit_begin\0").unwrap(),
                end: *global.get(b"dandrum_audit_end\0").unwrap(),
            }
        };
        let probe = unsafe {
            libloading::Library::new(
                std::env::var_os("DANDRUM_ALLOCATOR_AUDIT_PROBE")
                    .expect("calibration DSO required"),
            )
        }
        .unwrap();
        let allocate = unsafe {
            *probe
                .get::<unsafe extern "C" fn() -> *mut u8>(b"probe_allocate\0")
                .unwrap()
        };
        let free = unsafe {
            *probe
                .get::<unsafe extern "C" fn(*mut u8)>(b"probe_deallocate\0")
                .unwrap()
        };
        let old = unsafe { allocate() };
        let (fresh, counts) = audit.observe(|| unsafe { allocate() });
        let (a, d) = counts.unwrap();
        assert!(
            a > 0 && d == 0,
            "native process calibration catches real DSO allocation"
        );
        let (_, counts) = audit.observe(|| unsafe { free(old) });
        let (a, d) = counts.unwrap();
        assert!(
            a == 0 && d > 0,
            "native process calibration catches preexisting DSO destruction"
        );
        unsafe {
            free(fresh);
        }
        assert_eq!(audit.observe(|| ()).1, Some((0, 0)));
        audit
    }
}
fn concurrent_native_callbacks(
    clock: &runloop::LoopState,
    browser: &webkit2gtk::WebView,
    audio: &vst3::ComPtr<IAudioProcessor>,
) {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    };
    let selected = tempfile::tempdir().unwrap();
    let path = selected.path().join("active-selected.wav");
    let mut distinct = SELECTED_WAV.to_vec();
    distinct[45] = 0x80;
    std::fs::write(&path, distinct).unwrap();
    let retained_audio = audio.clone();
    let audit = CallbackAudit::load();
    let audio = audio.clone();
    let stopping = Arc::new(AtomicBool::new(false));
    let stop_audio = stopping.clone();
    let changed = Arc::new(AtomicBool::new(false));
    let observed_change = changed.clone();
    let servicing = Arc::new(AtomicBool::new(false));
    let during_service = servicing.clone();
    let (ready, started) = mpsc::channel();
    let (go, begin) = mpsc::channel();
    let (stop, finish) = mpsc::channel();
    struct Stop(Arc<AtomicBool>, mpsc::Sender<()>);
    impl Drop for Stop {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
            let _ = self.1.send(());
        }
    }
    std::thread::scope(|scope| {
        let _stop_on_unwind = Stop(stopping.clone(), stop.clone());
        let worker = scope.spawn(move || {
            let notes = events(vec![note_on(0, 48, 1.0)]);
            let mut block = Block::new(16);
            let mut zero = Block::new(0);
            assert_eq!(audit.clean(|| unsafe { audio.setProcessing(1) }), kResultOk);
            let mut render = || {
                let mut data = block.data();
                data.inputEvents = notes.as_ptr();
                assert_eq!(
                    audit.clean(|| unsafe { audio.process(&mut data) }),
                    kResultOk
                );
                let left = block.left[0];
                let right = block.right[0];
                assert!(
                    left == -0.25 || left == -0.3125,
                    "known signed sample at physical level2 or2.5: {left}"
                );
                assert_eq!(left, right);
                if left == -0.3125 {
                    observed_change.store(true, Ordering::Release);
                }
                assert_eq!((block.left[15], block.right[15]), (0.0, 0.0));
                assert_eq!(
                    audit.clean(|| unsafe { audio.process(&mut zero.data()) }),
                    kResultOk
                );
                left
            };
            assert_eq!(render(), -0.25);
            ready.send(()).unwrap();
            begin.recv_timeout(Duration::from_secs(5)).unwrap();
            let mut overlapping = 0;
            let mut renders = 1;
            while !stop_audio.load(Ordering::Acquire) {
                assert!(
                    renders < 4096,
                    "bounded callbacks during native control workflow"
                );
                if during_service.load(Ordering::Acquire) {
                    overlapping += 1;
                }
                render();
                renders += 1;
                std::thread::sleep(Duration::from_millis(1));
            }
            finish.recv_timeout(Duration::from_secs(5)).unwrap();
            assert_eq!(audit.clean(|| unsafe { audio.setProcessing(0) }), kResultOk);
            (
                overlapping,
                renders,
                matches!(audit, CallbackAudit::Armed { .. }),
            )
        });
        started.recv_timeout(Duration::from_secs(5)).unwrap();
        servicing.store(true, Ordering::Release);
        go.send(()).unwrap();
        type_level(clock, browser, "2.5");
        wait_dom(
            clock,
            browser,
            "document.querySelector('[data-knob=\"drums.level\"] [role=slider]').getAttribute('aria-valuenow')==='0.625'",
        );
        javascript(
            clock,
            browser,
            r#"(()=>{if(!document.querySelector('button[aria-label="Add source…"]'))document.querySelector('button[aria-label="Instrument tree"]').click();return 'tree';})()"#,
        );
        wait_dom(
            clock,
            browser,
            r#"document.querySelector('button[aria-label="Add source…"]')"#,
        );
        javascript(
            clock,
            browser,
            r#"document.querySelector('button[aria-label="Add source…"]').click()||'choose'"#,
        );
        let chooser = native_dialog(clock);
        assert!(chooser.set_filename(&path));
        let deadline = Instant::now() + Duration::from_secs(5);
        while chooser.filename().as_deref() != Some(path.as_path()) {
            dispatch_context(clock);
            assert!(Instant::now() < deadline);
        }
        chooser.response(gtk::ResponseType::Accept);
        drop(chooser);
        let mut progress = 0;
        loop {
            let state = invoke(
                clock,
                browser,
                "window.__DANDRUM__.backend.getNativeFunction('getSamplerSession')()",
            )["result"]
                .clone();
            progress += 1;
            if state["yaml"]
                .as_str()
                .unwrap()
                .contains("9fe86bf31a0b240e95e956ae1bee77067ab24319ca9fbdb6c93bab2a52ec0a03")
            {
                assert_eq!(state["document"]["sources"].as_array().unwrap().len(), 3);
                assert_eq!(
                    state["connection"]["history"],
                    serde_json::json!({"undoCount":2,"redoCount":0})
                );
                break;
            }
            assert!(
                Instant::now() < deadline,
                "real selection/mapping completes during measured audio callbacks"
            );
        }
        while !changed.load(Ordering::Acquire) {
            dispatch_context(clock);
            assert!(
                Instant::now() < deadline,
                "actual audio observes the native public level change"
            );
        }
        selected.close().unwrap();
        stopping.store(true, Ordering::Release);
        servicing.store(false, Ordering::Release);
        stop.send(()).unwrap();
        let (overlapping, renders, armed) = worker.join().unwrap();
        assert!(overlapping > 0 && progress > 0);
        if armed {
            println!(
                "calibrated native DSO sentinel: {} complete start/process/zero/stop callbacks each0 allocations/0 frees; {renders} signed selected-sample renders overlap actual React gestures, GTK selection, mapping and graph replacement",
                2 * renders + 2
            );
        }
    });
    assert_mapping_audio(&retained_audio, true, 2.5);
    assert_eq!(unsafe { retained_audio.setProcessing(1) }, kResultOk);
    let notes = events(vec![note_on(0, 49, 1.0)]);
    let mut block = Block::new(16);
    let mut data = block.data();
    data.inputEvents = notes.as_ptr();
    assert_eq!(unsafe { retained_audio.process(&mut data) }, kResultOk);
    assert_eq!(
        (block.left[0], block.right[0]),
        (-0.625, -0.625),
        "distinct second selected source maps to the next free key49 and survives deletion"
    );
    assert_eq!(unsafe { retained_audio.setProcessing(0) }, kResultOk);
}

fn held_native_note_cleanup() {
    let bundle = Bundle::new();
    let assets = std::env::var_os("DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR").unwrap();
    copy_directory(Path::new(&assets), &bundle.resources.join("editor"));
    let document = bundle.resources.join("instrument/advanced-drum-kit.yaml");
    let definition = r#"module_definitions:
  - type: held_sample_voice
    ports:
      - { name: audio, direction: output, signal: audio, channels: 2, maps_from: gated.audio }
    modules:
      - { id: gated, type: sample_player, static: { source: drums, region: kick, channels: 2, mode: gated } }
    connections:
      - { from: voice.gate, to: gated.trigger }
      - { from: voice.gate, to: gated.gate }
      - { from: voice.velocity, to: gated.level }
modules:
"#;
    let yaml=std::fs::read_to_string(&document).unwrap().replace("maps_from: player.audio","maps_from: mix.mix").replacen("modules:\n",definition,1).replacen("connections:\n  - { from: midi.events, to: player.note }","  - { id: held, type: poly, static: { definition: held_sample_voice, max_voices: 8, allocation: reject-new } }\n  - { id: mix, type: audio_mixer, static: { channels: 2 } }\nconnections:\n  - { from: midi.events, to: player.note }\n  - { from: midi.events, to: held.notes }\n  - { from: player.audio, to: mix.inputs }\n  - { from: held.audio, to: mix.inputs }",1);
    std::fs::write(document, yaml).unwrap();
    let frames = 51000u32;
    let data_bytes = frames * 4;
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&48000u32.to_le_bytes());
    wav.extend_from_slice(&192000u32.to_le_bytes());
    wav.extend_from_slice(&4u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes.to_le_bytes());
    for frame in 0..frames {
        let value: i16 = match frame {
            0 => -16384,
            1 => 8192,
            2 => -8192,
            3 => 16384,
            _ => -4096,
        };
        wav.extend_from_slice(&value.to_le_bytes());
        wav.extend_from_slice(&value.to_le_bytes());
    }
    std::fs::write(
        bundle
            .resources
            .join("instrument/assets/advanced-drums.wav"),
        wav,
    )
    .unwrap();
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
    assert_eq!(unsafe { p.connect(c.as_ptr()) }, kResultOk);
    assert_eq!(unsafe { c.connect(p.as_ptr()) }, kResultOk);
    let audio = processor.cast::<IAudioProcessor>().unwrap();
    start(&processor, &audio, 48000.0, 16);
    let (view, browser) = open_view(&controller, &parent, &clock);
    assert_eq!(
        invoke(
            &clock,
            &browser,
            "window.__DANDRUM__.backend.getNativeFunction('noteOn')(60,1,1)"
        )["status"],
        "accepted"
    );
    let mut first = Block::new(16);
    assert_eq!(unsafe { audio.process(&mut first.data()) }, kResultOk);
    assert_eq!(&first.left[..4], &[-0.5, 0.25, -0.25, 0.5]);
    assert_eq!(&first.right[..4], &[-0.5, 0.25, -0.25, 0.5]);
    let mut held = Block::new(16);
    assert_eq!(unsafe { audio.process(&mut held.data()) }, kResultOk);
    assert_eq!(held.left, [-0.125; 16]);
    assert_eq!(held.right, [-0.125; 16]);
    assert_eq!(unsafe { view.removed() }, kResultOk);
    drop(browser);
    drop(view);
    let mut released = Block::new(16);
    assert_eq!(unsafe { audio.process(&mut released.data()) }, kResultOk);
    assert_eq!(released.left, [0.0; 16]);
    assert_eq!(released.right, [0.0; 16]);
    let (view, browser) = open_view(&controller, &parent, &clock);
    let mut fresh = Block::new(16);
    assert_eq!(unsafe { audio.process(&mut fresh.data()) }, kResultOk);
    assert_eq!(fresh.left, [0.0; 16]);
    assert_eq!(fresh.right, [0.0; 16]);
    assert_eq!(unsafe { view.removed() }, kResultOk);
    drop(browser);
    drop(view);
    assert_eq!(unsafe { audio.setProcessing(0) }, kResultOk);
    assert_eq!(unsafe { controller.terminate() }, kResultOk);
    assert_eq!(unsafe { processor.terminate() }, kResultOk);
    assert!(clock.handlers.borrow().is_empty());
}

#[test]
#[ignore = "production React/Wry/GTK and copied VST3; packaged assets and dedicated X11/XTest display"]
fn production_native_selection_retains_decoded_content_after_source_and_view_loss() {
    if std::env::var_os("DANDRUM_SOURCE_HIDDEN_REQUIRED").is_some() {
        assert!(std::env::var_os("DANDRUM_VST3_INSTALLED_BUNDLE").is_some());
        assert!(
            !Path::new(env!("CARGO_MANIFEST_DIR")).exists(),
            "development sources must actually be hidden"
        );
        let namespace = std::fs::read_link("/proc/self/ns/net").unwrap();
        assert_ne!(
            namespace.as_os_str(),
            std::env::var_os("DANDRUM_PARENT_NET_NAMESPACE").unwrap()
        );
        let route = std::fs::read_to_string("/proc/net/route").unwrap();
        assert_eq!(
            route.lines().count(),
            1,
            "no network route in this namespace"
        );
    }
    let bundle = Bundle::new();
    let assets = std::env::var_os("DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR")
        .expect("explicit complete packaged native editor");
    copy_directory(Path::new(&assets), &bundle.resources.join("editor"));
    let parent = Parent::new();
    let module = bundle.load();
    let (clock, factory, _host) = runloop::host(&module);
    let processor = module.class_from::<IComponent>(&factory, &PROCESSOR);
    let controller = module.class_from::<IEditController>(&factory, &CONTROLLER);
    let host = vst3::ComWrapper::new(BitwigHost)
        .to_com_ptr::<FUnknown>()
        .unwrap();
    assert_eq!(
        unsafe { processor.initialize(std::ptr::null_mut()) },
        kResultOk
    );
    assert_eq!(unsafe { controller.initialize(host.as_ptr()) }, kResultOk);
    let edits = Rc::new(RefCell::new(Vec::new()));
    let handler = vst3::ComWrapper::new(Handler(edits.clone()))
        .to_com_ptr::<IComponentHandler>()
        .unwrap();
    assert_eq!(
        unsafe { controller.setComponentHandler(handler.as_ptr()) },
        kResultOk
    );
    let independent = module.class_from::<IComponent>(&factory, &PROCESSOR);
    assert_eq!(
        unsafe { independent.initialize(std::ptr::null_mut()) },
        kResultOk
    );
    let independent_audio = independent.cast::<IAudioProcessor>().unwrap();
    start(&independent, &independent_audio, 48000.0, 16);
    assert_eq!(unsafe { independent_audio.setProcessing(0) }, kResultOk);
    let independent_before = saved_state(&independent);
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
    wait_dom(
        &clock,
        &browser,
        "document.body.innerText.includes('36 C1')",
    );
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
        assert!(
            matches!(
                status["result"]["phase"].as_str(),
                Some("choosing" | "preparing")
            ),
            "{status}"
        );
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
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let phase = invoke(&clock, &browser, &cancelled_expression)["result"]["phase"].clone();
        if phase == "cancelled" {
            break;
        }
        assert_eq!(
            phase, "choosing",
            "only the pending cancellation may precede its receipt"
        );
        assert!(Instant::now() < deadline, "native cancellation must settle");
        std::thread::sleep(Duration::from_millis(5));
    }
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
    assert!(
        edits.borrow().is_empty(),
        "native selection cannot create host parameter gestures"
    );
    javascript(
        &clock,
        &browser,
        "(()=>{Array.from(document.querySelectorAll('[role=tab]')).find(tab=>tab.textContent.trim()==='Voice').click();return 'voice';})()",
    );
    wait_dom(
        &clock,
        &browser,
        "document.querySelector('[data-knob=\"drums.level\"] [role=slider]')",
    );
    javascript(
        &clock,
        &browser,
        "(()=>{const s=document.querySelector('[data-knob=\"drums.level\"] [role=slider]');s.focus();s.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}));return 'entry';})()",
    );
    wait_dom(
        &clock,
        &browser,
        "document.querySelector('[data-knob=\"drums.level\"] input')",
    );
    javascript(
        &clock,
        &browser,
        "(()=>{window.__VST3_INPUT_TRACE__=[];for(const type of ['pointerdown','pointerup','click','keydown','keyup','beforeinput','input','compositionstart','compositionend'])document.addEventListener(type,event=>{const input=document.querySelector('[data-knob=\"drums.level\"] input');const trace=window.__VST3_INPUT_TRACE__;trace.push({type,key:event.key,ctrl:event.ctrlKey,trusted:event.isTrusted,target:event.target===input,focused:document.hasFocus(),value:input?.value,start:input?.selectionStart,end:input?.selectionEnd});if(trace.length>32)trace.shift();},true);return 'observing';})()",
    );
    parent
        .x
        .set_input_focus(
            x11rb::protocol::xproto::InputFocus::PARENT,
            parent.id,
            x11rb::CURRENT_TIME,
        )
        .unwrap()
        .check()
        .unwrap();
    assert_eq!(unsafe { view.onFocus(1) }, kResultOk);
    let focus = parent.x.get_input_focus().unwrap().reply().unwrap().focus;
    assert_ne!(focus, parent.id, "host focus must reach the native child");
    javascript(
        &clock,
        &browser,
        "(()=>{document.querySelector('[data-knob=\"drums.level\"] input').focus();return 'focus requested';})()",
    );
    let point: Vec<i16> = serde_json::from_str(&javascript(&clock, &browser, "JSON.stringify((()=>{const r=document.querySelector('[data-knob=\"drums.level\"] input').getBoundingClientRect();return [Math.floor(r.left+r.width/2),Math.floor(r.top+r.height/2)]})())")).unwrap();
    let root = parent
        .x
        .get_geometry(parent.id)
        .unwrap()
        .reply()
        .unwrap()
        .root;
    let point = parent
        .x
        .translate_coordinates(parent.id, root, point[0], point[1])
        .unwrap()
        .reply()
        .unwrap();
    parent
        .x
        .set_input_focus(
            x11rb::protocol::xproto::InputFocus::PARENT,
            parent.id,
            x11rb::CURRENT_TIME,
        )
        .unwrap()
        .check()
        .unwrap();
    assert!(
        std::process::Command::new("xdotool")
            .args([
                "mousemove",
                &point.dst_x.to_string(),
                &point.dst_y.to_string(),
                "click",
                "1"
            ])
            .status()
            .unwrap()
            .success()
    );
    // xdotool returning means X11 accepted the click, not that WebKit applied
    // its caret movement. Finish that trusted event before sending selection keys.
    wait_dom(
        &clock,
        &browser,
        "document.activeElement===document.querySelector('[data-knob=\"drums.level\"] input')&&window.__VST3_INPUT_TRACE__.some(event=>event.type==='click'&&event.trusted&&event.target)",
    );
    assert_ne!(
        parent.x.get_input_focus().unwrap().reply().unwrap().focus,
        parent.id,
        "native pointer entry must claim focus even without host onFocus"
    );
    wait_dom(
        &clock,
        &browser,
        "document.querySelector('[data-knob=\"drums.level\"] input')",
    );
    assert!(
        std::process::Command::new("xdotool")
            .args(["key", "ctrl+a"])
            .status()
            .unwrap()
            .success()
    );
    wait_dom(
        &clock,
        &browser,
        "(()=>{const i=document.querySelector('[data-knob=\"drums.level\"] input');const trace=window.__VST3_INPUT_TRACE__;return i&&i.value.length>0&&i.selectionStart===0&&i.selectionEnd===i.value.length&&trace.some(event=>event.type==='keydown'&&event.key==='a'&&event.ctrl&&event.trusted&&event.target)&&trace.some(event=>event.type==='keyup'&&event.key==='a'&&event.trusted&&event.target)})()",
    );
    assert!(
        std::process::Command::new("xdotool")
            .args(["type", "--clearmodifiers", "2"])
            .status()
            .unwrap()
            .success()
    );
    wait_dom(
        &clock,
        &browser,
        "document.querySelector('[data-knob=\"drums.level\"] input')?.value==='2'",
    );
    assert!(
        std::process::Command::new("xdotool")
            .args(["key", "Return"])
            .status()
            .unwrap()
            .success()
    );
    wait_dom(
        &clock,
        &browser,
        "!document.querySelector('[data-knob=\"drums.level\"] input')",
    );
    wait_dom(
        &clock,
        &browser,
        "document.querySelector('[data-knob=\"drums.level\"] [role=slider]').getAttribute('aria-valuenow')==='0.5'",
    );
    for _ in 0..4 {
        clock.tick();
    }
    assert_eq!(
        *edits.borrow(),
        [
            HostEdit::Begin(0x1003),
            HostEdit::End(0x1003),
            HostEdit::Begin(0x1003),
            HostEdit::Value(0x1003, 0.5),
            HostEdit::End(0x1003)
        ],
        "pointer activation balances a no-change gesture; native keys commit one balanced edit"
    );
    let imported_source = tempfile::tempdir().unwrap();
    let imported_path = imported_source.path().join("mapped.wav");
    std::fs::write(&imported_path, SELECTED_WAV).unwrap();
    let before_import = invoke(
        &clock,
        &browser,
        "window.__DANDRUM__.backend.getNativeFunction('getSamplerSession')()",
    )["result"]
        .clone();
    assert_mapping_audio(&audio, false, 2.0);
    javascript(
        &clock,
        &browser,
        r#"(()=>{if(!document.querySelector('button[aria-label="Add source…"]'))document.querySelector('button[aria-label="Instrument tree"]').click();return 'tree';})()"#,
    );
    wait_dom(
        &clock,
        &browser,
        r#"document.querySelector('button[aria-label="Add source…"]')"#,
    );
    javascript(
        &clock,
        &browser,
        r#"document.querySelector('button[aria-label="Add source…"]').click()||'choose'"#,
    );
    let chooser = native_dialog(&clock);
    assert!(chooser.set_filename(&imported_path));
    let deadline = Instant::now() + Duration::from_secs(8);
    while chooser.filename().as_deref() != Some(imported_path.as_path()) {
        dispatch_context(&clock);
        assert!(Instant::now() < deadline);
    }
    chooser.response(gtk::ResponseType::Accept);
    drop(chooser);
    let imported = loop {
        let current = invoke(
            &clock,
            &browser,
            "window.__DANDRUM__.backend.getNativeFunction('getSamplerSession')()",
        )["result"]
            .clone();
        if current["yaml"].as_str().unwrap().contains(CONTENT_SHA256) {
            break current;
        }
        assert!(
            Instant::now() < deadline,
            "actual React Add source publishes selected content: {current}"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(
        imported["connection"]["history"],
        serde_json::json!({"undoCount":1,"redoCount":0})
    );
    assert_eq!(
        imported["document"]["parameters"],
        before_import["document"]["parameters"]
    );
    imported_source.close().unwrap();
    assert_mapping_audio(&audio, true, 2.0);
    assert_eq!(saved_state(&independent), independent_before);
    assert_mapping_audio(&independent_audio, false, 1.0);
    assert_eq!(unsafe { view.removed() }, kResultOk);
    drop(browser);
    drop(view);
    let (view, browser) = open_view(&controller, &parent, &clock);
    assert_eq!(
        invoke(
            &clock,
            &browser,
            "window.__DANDRUM__.backend.getNativeFunction('getSamplerSession')()"
        )["result"]["yaml"],
        imported["yaml"]
    );
    for (action, wanted, revision, undo, redo) in
        [("Undo", false, "2", 0, 1), ("Redo", true, "3", 1, 0)]
    {
        let selector = serde_json::to_string(&format!(
            "header button[aria-label^=\"{action} \"]:not(:disabled)"
        ))
        .unwrap();
        wait_dom(
            &clock,
            &browser,
            &format!("document.querySelector({selector})"),
        );
        javascript(
            &clock,
            &browser,
            &format!("document.querySelector({selector}).click()||'clicked'"),
        );
        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            let state = invoke(
                &clock,
                &browser,
                "window.__DANDRUM__.backend.getNativeFunction('getSamplerSession')()",
            )["result"]
                .clone();
            if state["connection"]["revision"] == revision {
                assert_eq!(
                    state["connection"]["history"],
                    serde_json::json!({"undoCount":undo,"redoCount":redo})
                );
                assert_eq!(
                    state["yaml"].as_str().unwrap().contains(CONTENT_SHA256),
                    wanted
                );
                break;
            }
            assert!(
                Instant::now() < deadline,
                "actual React {action} completes after source deletion"
            );
        }
        assert_mapping_audio(&audio, wanted, 2.0);
        assert_eq!(saved_state(&independent), independent_before);
        assert_mapping_audio(&independent_audio, false, 1.0);
    }
    concurrent_native_callbacks(&clock, &browser, &audio);
    assert_eq!(
        *edits.borrow(),
        [
            HostEdit::Begin(0x1003),
            HostEdit::End(0x1003),
            HostEdit::Begin(0x1003),
            HostEdit::Value(0x1003, 0.5),
            HostEdit::End(0x1003),
            HostEdit::Begin(0x1003),
            HostEdit::Value(0x1003, 0.625),
            HostEdit::End(0x1003)
        ]
    );
    assert_eq!(saved_state(&independent), independent_before);
    assert_mapping_audio(&independent_audio, false, 1.0);
    let active_state = invoke(
        &clock,
        &browser,
        "window.__DANDRUM__.backend.getNativeFunction('getSamplerSession')()",
    )["result"]
        .clone();
    assert_eq!(unsafe { view.removed() }, kResultOk);
    drop(browser);
    drop(view);
    let invalid =
        unsafe { vst3::ComPtr::from_raw(controller.createView(c"editor".as_ptr())) }.unwrap();
    let app = bundle.resources.join("editor/app.js");
    let missing = app.with_extension("missing");
    std::fs::rename(&app, &missing).unwrap();
    assert_eq!(
        unsafe { invalid.attached(parent.id as usize as *mut _, kPlatformTypeX11EmbedWindowID) },
        kResultFalse
    );
    std::fs::rename(&missing, &app).unwrap();
    clock.reject_register.set(true);
    assert_eq!(
        unsafe { invalid.attached(parent.id as usize as *mut _, kPlatformTypeX11EmbedWindowID) },
        kResultFalse
    );
    clock.reject_register.set(false);
    assert!(parent.children().is_empty());
    drop(invalid);
    assert_mapping_audio(&audio, true, 2.5);
    let (view, browser) = open_view(&controller, &parent, &clock);
    let recovered = invoke(
        &clock,
        &browser,
        "window.__DANDRUM__.backend.getNativeFunction('getSamplerSession')()",
    )["result"]
        .clone();
    assert_eq!(recovered["yaml"], active_state["yaml"]);
    assert_eq!(recovered["document"], active_state["document"]);
    assert_eq!(
        recovered["connection"]["history"], active_state["connection"]["history"],
        "failed parenting/service cannot lose accepted native history"
    );
    for (action, selected, revision, undo, redo) in
        [("Undo", false, "5", 1, 1), ("Redo", true, "6", 2, 0)]
    {
        let selector = serde_json::to_string(&format!(
            "header button[aria-label^=\"{action} \"]:not(:disabled)"
        ))
        .unwrap();
        wait_dom(
            &clock,
            &browser,
            &format!("document.querySelector({selector})"),
        );
        javascript(
            &clock,
            &browser,
            &format!("document.querySelector({selector}).click()||'clicked'"),
        );
        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            let state = invoke(
                &clock,
                &browser,
                "window.__DANDRUM__.backend.getNativeFunction('getSamplerSession')()",
            )["result"]
                .clone();
            if state["connection"]["revision"] == revision {
                assert_eq!(
                    state["connection"]["history"],
                    serde_json::json!({"undoCount":undo,"redoCount":redo})
                );
                assert_eq!(
                    state["document"]["parameters"],
                    active_state["document"]["parameters"]
                );
                break;
            }
            assert!(
                Instant::now() < deadline,
                "Undo/Redo of the edit admitted during actual callbacks completes after source and view loss"
            );
        }
        assert_mapping_audio(&audio, true, 2.5);
        assert_eq!(unsafe { audio.setProcessing(1) }, kResultOk);
        let notes = events(vec![note_on(0, 49, 1.0)]);
        let mut block = Block::new(16);
        let mut data = block.data();
        data.inputEvents = notes.as_ptr();
        assert_eq!(unsafe { audio.process(&mut data) }, kResultOk);
        let wanted = if selected { -0.625 } else { 0.0 };
        assert_eq!((block.left[0], block.right[0]), (wanted, wanted));
        assert_eq!(unsafe { audio.setProcessing(0) }, kResultOk);
        assert_eq!(saved_state(&independent), independent_before);
        assert_mapping_audio(&independent_audio, false, 1.0);
    }
    assert_eq!(unsafe { independent.terminate() }, kResultOk);
    assert_eq!(unsafe { view.removed() }, kResultOk);
    drop(browser);
    drop(view);
    assert_eq!(unsafe { controller.terminate() }, kResultOk);
    assert_eq!(unsafe { processor.terminate() }, kResultOk);
    assert!(clock.handlers.borrow().is_empty());
    held_native_note_cleanup();
}
