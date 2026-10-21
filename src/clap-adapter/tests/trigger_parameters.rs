//! Slow product-boundary tests use an explicitly copied, trusted release plugin.
#![cfg(all(feature = "trigger-editor", target_os = "linux"))]

use clack_extensions::gui::{
    GuiApiType, GuiConfiguration, GuiSize, HostGui, HostGuiImpl, PluginGui, Window,
};
use clack_host::entry::PluginEntry;
use clack_host::prelude::*;
use std::path::PathBuf;

struct GuiHost;
struct Shared {
    callback: std::sync::atomic::AtomicBool,
    flush_requests: std::sync::atomic::AtomicUsize,
    timer_available: bool,
}
#[derive(Default)]
struct TimerLedger {
    registrations: Vec<(u32, clack_extensions::timer::TimerId)>,
    live: Vec<clack_extensions::timer::TimerId>,
    removals: Vec<clack_extensions::timer::TimerId>,
    refuse_registration: bool,
    refuse_removals: usize,
    on_register: Option<Box<dyn FnOnce()>>,
}
struct HostMain(Rc<RefCell<TimerLedger>>);
impl MainThreadHandler<'_> for HostMain {}
impl clack_extensions::timer::HostTimerImpl for HostMain {
    fn register_timer(
        &self,
        period_ms: u32,
    ) -> Result<clack_extensions::timer::TimerId, HostError> {
        let id = {
            let mut ledger = self.0.borrow_mut();
            if ledger.refuse_registration {
                return Err(HostError::Message("test host refuses timer"));
            }
            let id = clack_extensions::timer::TimerId(71 + ledger.registrations.len() as u32);
            ledger.registrations.push((period_ms, id));
            ledger.live.push(id);
            id
        };
        let on_register = self.0.borrow_mut().on_register.take();
        if let Some(callback) = on_register {
            callback();
        }
        Ok(id)
    }

    fn unregister_timer(&self, id: clack_extensions::timer::TimerId) -> Result<(), HostError> {
        let mut ledger = self.0.borrow_mut();
        ledger.removals.push(id);
        if ledger.refuse_removals != 0 {
            ledger.refuse_removals -= 1;
            return Err(HostError::Message("test host refuses removal"));
        }
        let index = ledger
            .live
            .iter()
            .position(|live| *live == id)
            .ok_or(HostError::Message("unknown host timer"))?;
        ledger.live.remove(index);
        Ok(())
    }
}
impl clack_extensions::params::HostParamsImplShared for Shared {
    fn request_flush(&self) {
        self.flush_requests
            .fetch_add(1, std::sync::atomic::Ordering::Release);
    }
}
impl clack_extensions::params::HostParamsImplMainThread for HostMain {
    fn rescan(&self, _: clack_extensions::params::ParamRescanFlags) {}
    fn clear(&self, _: ClapId, _: clack_extensions::params::ParamClearFlags) {
        panic!("test surface identities stay fixed");
    }
}

impl SharedHandler<'_> for Shared {
    fn request_restart(&self) {}
    fn request_process(&self) {}
    fn request_callback(&self) {
        self.callback
            .store(true, std::sync::atomic::Ordering::Release);
    }
}

impl HostGuiImpl for Shared {
    fn resize_hints_changed(&self) {}
    fn request_resize(&self, _: GuiSize) -> Result<(), HostError> {
        Err(HostError::Message("the host owns the fixed parent size"))
    }
    fn request_show(&self) -> Result<(), HostError> {
        Err(HostError::Message("the host controls visibility"))
    }
    fn request_hide(&self) -> Result<(), HostError> {
        Err(HostError::Message("the host controls visibility"))
    }
    fn closed(&self, _: bool) {}
}

impl HostHandlers for GuiHost {
    type Shared<'a> = Shared;
    type MainThread<'a> = HostMain;
    type AudioProcessor<'a> = ();

    fn declare_extensions(builder: &mut clack_host::host::HostExtensions<Self>, shared: &Shared) {
        builder
            .register::<HostGui>()
            .register::<clack_extensions::params::HostParams>();
        if shared.timer_available {
            builder.register::<clack_extensions::timer::HostTimer>();
        }
    }
}

fn artifact() -> PathBuf {
    std::env::var_os("DANDRUM_TRIGGER_GUI_PLUGIN_PATH")
        .map(PathBuf::from)
        .expect("set DANDRUM_TRIGGER_GUI_PLUGIN_PATH to the copied release product")
}
fn copied_entry() -> PluginEntry {
    use std::os::unix::ffi::OsStrExt;
    let path = artifact();
    let declared = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    // The pinned convenience loader uses to_string_lossy. Reuse the maintained
    // headless host fixture's explicit provider to preserve the actual Entry bytes.
    // SAFETY: trusted copied product; the returned Entry owns this provider.
    let provider = unsafe { clack_host::entry::LibraryEntry::load_from_path(&path) }.unwrap();
    unsafe { PluginEntry::load_from(provider, &declared) }.unwrap()
}
fn instantiate(entry: &PluginEntry) -> PluginInstance<GuiHost> {
    instantiate_options(entry, true, Rc::new(RefCell::new(TimerLedger::default())))
}
fn instantiate_options(
    entry: &PluginEntry,
    timer_available: bool,
    ledger: Rc<RefCell<TimerLedger>>,
) -> PluginInstance<GuiHost> {
    PluginInstance::new(
        move |_| Shared {
            callback: std::sync::atomic::AtomicBool::new(false),
            flush_requests: std::sync::atomic::AtomicUsize::new(0),
            timer_available,
        },
        move |_| HostMain(ledger),
        entry,
        c"org.dandrum.trigger",
        &HostInfo::new("Trigger GUI proof", "Dandrum", "", "0.1").unwrap(),
    )
    .expect("the copied Trigger initializes from its sibling resources")
}

fn copied_plugin() -> PluginInstance<GuiHost> {
    instantiate(&copied_entry())
}

use std::time::{Duration, Instant};
use x11rb::{
    COPY_DEPTH_FROM_PARENT,
    connection::Connection,
    protocol::xproto::{ConnectionExt, CreateWindowAux, WindowClass},
    rust_connection::RustConnection,
};

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

use gtk::prelude::*;
use std::{cell::RefCell, rc::Rc};
use webkit2gtk::WebViewExt;

fn dispatch_context(plugin: &mut PluginInstance<GuiHost>) {
    let context = gtk::glib::MainContext::default();
    for _ in 0..64 {
        if !context.pending() {
            break;
        }
        context.iteration(false);
    }
    if plugin.access_shared_handler(|s| s.callback.swap(false, std::sync::atomic::Ordering::AcqRel))
    {
        plugin.call_on_main_thread_callback();
    }
}
fn javascript(
    plugin: &mut PluginInstance<GuiHost>,
    browser: &webkit2gtk::WebView,
    script: &str,
) -> String {
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
        dispatch_context(plugin);
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
#[test]
#[ignore = "native parameter boundary: real inherited X11, copied release and host main loop"]
fn copied_native_parameter_state_is_an_authenticated_numeric_dto() {
    std::thread::spawn(|| {
        let parent = Parent::new();
        let mut plugin = copied_plugin();
        let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
        gui.create(&plugin.plugin_handle(), GuiConfiguration { api_type: GuiApiType::X11, is_floating: false }).unwrap();
        unsafe { gui.set_parent(&plugin.plugin_handle(), Window::from_x11_handle(parent.id.into())) }.unwrap();
        gui.show(&plugin.plugin_handle()).unwrap();
        let object = weak_browser_in_child(parent.children()[0]).upgrade().unwrap();
        let browser = object.downcast::<webkit2gtk::WebView>().expect("actual production WebKitWebView");
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let ready = javascript(&mut plugin, &browser, "String(Boolean(document.querySelector('[data-testid=trigger-editor]')))" );
            if ready == "true" { break; }
            assert!(Instant::now() < deadline, "production React did not mount");
            std::thread::sleep(Duration::from_millis(10));
        }
        javascript(&mut plugin, &browser, r#"window.__G2_RESULT__=null;window.__DANDRUM__.backend.getNativeFunction('getParameterState')().then(result=>window.__G2_RESULT__={status:'accepted',result},error=>window.__G2_RESULT__={status:'rejected',code:error.code});'started'"#);
        let result = loop {
            let value = javascript(&mut plugin, &browser, "JSON.stringify(window.__G2_RESULT__)");
            if value != "null" { break serde_json::from_str::<serde_json::Value>(&value).unwrap(); }
            assert!(Instant::now() < deadline, "actual native getParameterState Promise did not settle");
        };
        eprintln!("actual authenticated native parameter reply: {result}");
        // Teardown before assertions so a behavior RED still exercises owner cleanup.
        gui.destroy(&plugin.plugin_handle());
        drop(browser);
        assert_eq!(result["status"], "accepted", "G2: real parameter read must be dispatched; actual={result}");
        assert_eq!(result["result"]["generation"], 1);
        assert_eq!(result["result"]["sequence"], 0);
        let values = result["result"]["parameters"].as_array().unwrap();
        assert_eq!(values.len(), 23);
        let level = values.iter().find(|p| p["id"] == "drums.level").unwrap();
        assert_eq!(level, &serde_json::json!({"id":"drums.level","name":"drums.level","value":0.25}));
    }).join().expect("parameter host thread completed");
}

use clack_extensions::params::PluginParams;
use clack_host::events::{
    UnknownEvent,
    event_types::{ParamGestureBeginEvent, ParamGestureEndEvent, ParamValueEvent},
    io::{OutputEventBuffer, TryPushError},
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Recorded {
    Begin(u32),
    Value(u32, f64),
    End(u32),
}
struct Recorder {
    accepted: [Option<Recorded>; 256],
    len: usize,
    attempts: usize,
    remaining: usize,
}
impl Recorder {
    fn new(remaining: usize) -> Self {
        Self {
            accepted: [None; 256],
            len: 0,
            attempts: 0,
            remaining,
        }
    }
}
impl OutputEventBuffer for Recorder {
    fn try_push(&mut self, event: &UnknownEvent) -> Result<(), TryPushError> {
        self.attempts += 1;
        if self.remaining == 0 || self.attempts > 64 || self.len == 256 {
            return Err(TryPushError::new());
        }
        assert_eq!(
            event.header().time(),
            0,
            "editor automation uses zero-offset output"
        );
        let recorded = if let Some(event) = event.as_event::<ParamGestureBeginEvent>() {
            Recorded::Begin(event.param_id().unwrap().get())
        } else if let Some(event) = event.as_event::<ParamValueEvent>() {
            assert!(event.pckn().matches_all());
            Recorded::Value(event.param_id().unwrap().get(), event.value())
        } else if let Some(event) = event.as_event::<ParamGestureEndEvent>() {
            Recorded::End(event.param_id().unwrap().get())
        } else {
            return Err(TryPushError::new());
        };
        self.accepted[self.len] = Some(recorded);
        self.len += 1;
        self.remaining -= 1;
        Ok(())
    }
}
fn invoke(
    plugin: &mut PluginInstance<GuiHost>,
    browser: &webkit2gtk::WebView,
    expression: &str,
) -> serde_json::Value {
    javascript(
        plugin,
        browser,
        &format!(
            "window.__G2_RESULT__=null;({expression}).then(result=>window.__G2_RESULT__={{status:'accepted',result}},error=>window.__G2_RESULT__={{status:'rejected',code:error.code}});'started'"
        ),
    );
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let value = javascript(plugin, browser, "JSON.stringify(window.__G2_RESULT__)");
        if value != "null" {
            return serde_json::from_str(&value).unwrap();
        }
        assert!(
            Instant::now() < deadline,
            "native command Promise did not settle"
        );
    }
}
#[test]
#[ignore = "native parameter boundary: real inherited X11, copied release and host main loop"]
fn copied_native_parameter_flush_delivers_host_accepted_gestures() {
    std::thread::spawn(|| {
        let parent = Parent::new();
        let mut plugin = copied_plugin();
        let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
        gui.create(
            &plugin.plugin_handle(),
            GuiConfiguration {
                api_type: GuiApiType::X11,
                is_floating: false,
            },
        )
        .unwrap();
        unsafe {
            gui.set_parent(
                &plugin.plugin_handle(),
                Window::from_x11_handle(parent.id.into()),
            )
        }
        .unwrap();
        gui.show(&plugin.plugin_handle()).unwrap();
        let browser = weak_browser_in_child(parent.children()[0])
            .upgrade()
            .unwrap()
            .downcast::<webkit2gtk::WebView>()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(15);
        while javascript(
            &mut plugin,
            &browser,
            "String(Boolean(document.querySelector('[data-testid=trigger-editor]')))",
        ) != "true"
        {
            assert!(Instant::now() < deadline, "production React did not mount");
        }
        let reply = invoke(
            &mut plugin,
            &browser,
            "window.__DANDRUM__.backend.getNativeFunction('setParameter')('drums.level',0.125,1)",
        );
        assert_eq!(reply["status"], "accepted");
        assert_eq!(
            reply["result"],
            serde_json::json!({"status":"accepted","generation":1,"sequence":1})
        );
        let params = plugin
            .plugin_handle()
            .get_extension::<PluginParams>()
            .unwrap();
        let mut output = Recorder::new(64);
        params.flush(
            &mut plugin.inactive_plugin_handle().unwrap(),
            &InputEvents::empty(),
            &mut OutputEvents::from_buffer(&mut output),
        );
        let physical = params.get_value(&plugin.plugin_handle(), ClapId::new(3));
        gui.destroy(&plugin.plugin_handle());
        drop(browser);
        assert_eq!(
            physical,
            Some(0.5),
            "actual stopped flush settles physical control"
        );
        assert_eq!(
            &output.accepted[..output.len],
            &[
                Some(Recorded::Begin(3)),
                Some(Recorded::Value(3, 0.5)),
                Some(Recorded::End(3))
            ],
            "G2: actual CLAP output accepts ordered automation"
        );
    })
    .join()
    .expect("parameter output host thread completed");
}

use clack_host::{
    events::{Pckn, event_types::MidiEvent},
    process::StartedPluginAudioProcessor,
};
fn start_audio(plugin: &mut PluginInstance<GuiHost>) -> StartedPluginAudioProcessor<GuiHost> {
    let stopped = plugin
        .activate(
            |_, _| (),
            PluginAudioConfiguration {
                sample_rate: 48000.0,
                min_frames_count: 1,
                max_frames_count: 16,
            },
        )
        .unwrap();
    std::thread::spawn(move || stopped.start_processing().unwrap())
        .join()
        .unwrap()
}
fn return_audio(plugin: &mut PluginInstance<GuiHost>, audio: StartedPluginAudioProcessor<GuiHost>) {
    plugin.deactivate(
        std::thread::spawn(move || audio.stop_processing())
            .join()
            .unwrap(),
    );
}
fn flush_audio(
    audio: &mut StartedPluginAudioProcessor<GuiHost>,
    input: &EventBuffer,
    output: &mut Recorder,
) {
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let mut handle = audio.plugin_handle();
                let params = handle.get_extension::<PluginParams>().unwrap();
                params.flush_active(
                    &mut handle,
                    &input.as_input(),
                    &mut OutputEvents::from_buffer(output),
                );
            })
            .join()
            .unwrap()
    });
}
use clack_host::events::io::EventBuffer;
#[test]
#[ignore = "native parameter boundary: real inherited X11, copied release and host main loop"]
fn copied_native_automation_survives_rejected_output_activation_failures_and_audio_return() {
    std::thread::spawn(||{
        let parent=Parent::new();let mut plugin=copied_plugin();
        let gui=plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
        gui.create(&plugin.plugin_handle(),GuiConfiguration{api_type:GuiApiType::X11,is_floating:false}).unwrap();
        unsafe{gui.set_parent(&plugin.plugin_handle(),Window::from_x11_handle(parent.id.into()))}.unwrap();gui.show(&plugin.plugin_handle()).unwrap();
        let browser=weak_browser_in_child(parent.children()[0]).upgrade().unwrap().downcast::<webkit2gtk::WebView>().unwrap();
        let deadline=Instant::now()+Duration::from_secs(15);
        while javascript(&mut plugin,&browser,"String(Boolean(document.querySelector('[data-testid=trigger-editor]')))")!="true" {assert!(Instant::now()<deadline);}
        assert_eq!(invoke(&mut plugin,&browser,"window.__DANDRUM__.backend.getNativeFunction('setParameter')('drums.level',0.375,1)")["status"],"accepted");
        plugin.call_on_main_thread_callback(); // Legal main-side staging, no fake callback or private control seam.
        let params=plugin.plugin_handle().get_extension::<PluginParams>().unwrap();
        assert!(plugin.activate(|_,_|(),PluginAudioConfiguration{sample_rate:f64::NAN,min_frames_count:1,max_frames_count:16}).is_err());
        // Reach the fallible preparation path after RuntimeAudio is taken; restore
        // the private copied resource before inspecting the result.
        let sample=artifact().parent().unwrap().join("DandrumTrigger.resources").join(dandrum_trigger_host::ADVANCED_DRUM_KIT_SAMPLE);
        let retained=sample.with_extension("wav.g2-retained");std::fs::rename(&sample,&retained).unwrap();
        let failed=plugin.activate(|_,_|(),PluginAudioConfiguration{sample_rate:44100.0,min_frames_count:1,max_frames_count:16});
        std::fs::rename(&retained,&sample).unwrap();
        if let Ok(stopped)=failed {plugin.deactivate(stopped);panic!("missing resource must reject changed-rate activation");}
        let mut audio=start_audio(&mut plugin);
        let mut malformed=EventBuffer::new();malformed.push(&ParamValueEvent::new(0,ClapId::new(24),Pckn::match_all(),0.5));
        let mut invalid_output=Recorder::new(64);flush_audio(&mut audio,&malformed,&mut invalid_output);
        assert_eq!(invalid_output.attempts,0,"invalid input cannot partially publish queued output");assert_eq!(params.get_value(&plugin.plugin_handle(),ClapId::new(3)),Some(1.0));
        let mut first=Recorder::new(1);flush_audio(&mut audio,&EventBuffer::new(),&mut first);
        assert_eq!(&first.accepted[..first.len],&[Some(Recorded::Begin(3))]);assert_eq!(first.attempts,2,"rejected value stops callback immediately");
        return_audio(&mut plugin,audio);
        // Reactivation retains both the rejected head and the uncollected Begin credit.
        let mut audio=start_audio(&mut plugin);let mut rest=Recorder::new(64);flush_audio(&mut audio,&EventBuffer::new(),&mut rest);
        assert_eq!(&rest.accepted[..rest.len],&[Some(Recorded::Value(3,1.5)),Some(Recorded::End(3))]);
        return_audio(&mut plugin,audio);plugin.call_on_main_thread_callback();
        assert_eq!(params.get_value(&plugin.plugin_handle(),ClapId::new(3)),Some(1.5));
        assert_eq!(invoke(&mut plugin,&browser,"window.__DANDRUM__.backend.getNativeFunction('setParameter')('drums.level',0.125,1)")["result"]["sequence"],2);
        plugin.call_on_main_thread_callback();let mut audio=start_audio(&mut plugin);
        let mut notes=EventBuffer::new();notes.push(&MidiEvent::new(3,0,[0x90,36,100]));
        let mut output=Recorder::new(64);let mut pcm=[[123.0f32;16];2];
        std::thread::scope(|scope|scope.spawn(||{
            let mut input_ports=AudioPorts::with_capacity(0,0);let input=input_ports.with_input_buffers(std::iter::empty::<AudioPortBuffer<[InputChannel<'_,f32>;0],[InputChannel<'_,f64>;0]>>());
            let mut output_ports=AudioPorts::with_capacity(2,1);let mut buffers=output_ports.with_output_buffers([AudioPortBuffer{latency:0,channels:AudioPortBufferType::f32_output_only(pcm.iter_mut().map(|channel|channel.as_mut_slice()))}]);
            audio.process(&input,&mut buffers,&notes.as_input(),&mut OutputEvents::from_buffer(&mut output),None,None).unwrap();
        }).join().unwrap());
        return_audio(&mut plugin,audio);gui.destroy(&plugin.plugin_handle());drop(browser);
        assert_eq!(&output.accepted[..output.len],&[Some(Recorded::Begin(3)),Some(Recorded::Value(3,0.5)),Some(Recorded::End(3))]);
        for channel in pcm {assert_eq!(&channel[..3],&[0.0;3]);assert_eq!(&channel[3..5],&[-0.25,-179.0/32768.0]);}
    }).join().expect("native automation ownership host completed");
}

fn saved_state(plugin: &mut PluginInstance<GuiHost>) -> Vec<u8> {
    let mut bytes = Vec::new();
    let handle = plugin.plugin_handle();
    handle
        .get_extension::<clack_extensions::state::PluginState>()
        .unwrap()
        .save(&handle, &mut bytes)
        .unwrap();
    bytes
}
fn seed_half_level(plugin: &mut PluginInstance<GuiHost>) -> Vec<u8> {
    let params = plugin
        .plugin_handle()
        .get_extension::<PluginParams>()
        .unwrap();
    let mut input = EventBuffer::new();
    input.push(&ParamValueEvent::new(
        u32::MAX,
        ClapId::new(3),
        Pckn::match_all(),
        0.5,
    ));
    let mut output = Recorder::new(64);
    params.flush(
        &mut plugin.inactive_plugin_handle().unwrap(),
        &input.as_input(),
        &mut OutputEvents::from_buffer(&mut output),
    );
    assert_eq!(
        output.attempts, 0,
        "host input must not echo as editor automation"
    );
    assert_eq!(
        params.get_value(&plugin.plugin_handle(), ClapId::new(3)),
        Some(0.5)
    );
    saved_state(plugin)
}
fn render_audio(
    audio: &mut StartedPluginAudioProcessor<GuiHost>,
    events: &EventBuffer,
    output: &mut Recorder,
) -> [[f32; 16]; 2] {
    let mut pcm = [[123.0f32; 16]; 2];
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let mut inputs = AudioPorts::with_capacity(0, 0);
                let input = inputs.with_input_buffers(std::iter::empty::<
                    AudioPortBuffer<[InputChannel<'_, f32>; 0], [InputChannel<'_, f64>; 0]>,
                >());
                let mut outputs = AudioPorts::with_capacity(2, 1);
                let mut buffers = outputs.with_output_buffers([AudioPortBuffer {
                    latency: 0,
                    channels: AudioPortBufferType::f32_output_only(
                        pcm.iter_mut().map(|channel| channel.as_mut_slice()),
                    ),
                }]);
                audio
                    .process(
                        &input,
                        &mut buffers,
                        &events.as_input(),
                        &mut OutputEvents::from_buffer(output),
                        None,
                        None,
                    )
                    .unwrap();
            })
            .join()
            .unwrap()
    });
    pcm
}
fn assert_preserved_half_level(plugin: &mut PluginInstance<GuiHost>, before: &[u8]) {
    assert_eq!(saved_state(plugin), before);
    let params = plugin
        .plugin_handle()
        .get_extension::<PluginParams>()
        .unwrap();
    assert_eq!(
        params.get_value(&plugin.plugin_handle(), ClapId::new(3)),
        Some(0.5)
    );
    let mut audio = start_audio(plugin);
    let mut events = EventBuffer::new();
    events.push(&MidiEvent::new(3, 0, [0x90, 36, 100]));
    let mut output = Recorder::new(64);
    let pcm = render_audio(&mut audio, &events, &mut output);
    return_audio(plugin, audio);
    for channel in pcm {
        assert_eq!(&channel[..3], &[0.0; 3]);
        assert_eq!(&channel[3..5], &[-0.25, -179.0 / 32768.0]);
    }
    assert_eq!(output.attempts, 0);
    assert_eq!(saved_state(plugin), before);
}
fn require_timer(available: bool, refused: bool) {
    let ledger = Rc::new(RefCell::new(TimerLedger {
        refuse_registration: refused,
        ..Default::default()
    }));
    let mut plugin = instantiate_options(&copied_entry(), available, Rc::clone(&ledger));
    let before = seed_half_level(&mut plugin);
    let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
    let result = gui.create(
        &plugin.plugin_handle(),
        GuiConfiguration {
            api_type: GuiApiType::X11,
            is_floating: false,
        },
    );
    gui.destroy(&plugin.plugin_handle());
    assert_preserved_half_level(&mut plugin, &before);
    drop(plugin);
    assert!(
        result.is_err(),
        "G2: interactive create requires an available, successful HostTimer registration"
    );
    assert!(ledger.borrow().live.is_empty());
}
#[test]
#[ignore = "native parameter fixture: explicit copied release"]
fn copied_native_gui_requires_host_timer() {
    require_timer(false, false);
}
#[test]
#[ignore = "native parameter fixture: explicit copied release"]
fn copied_native_gui_rejects_refused_host_timer() {
    require_timer(true, true);
}

fn wait_dom(plugin: &mut PluginInstance<GuiHost>, browser: &webkit2gtk::WebView, expression: &str) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while javascript(plugin, browser, &format!("String(Boolean({expression}))")) != "true" {
        assert!(
            Instant::now() < deadline,
            "DOM condition did not become true: {expression}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn type_level(plugin: &mut PluginInstance<GuiHost>, browser: &webkit2gtk::WebView, physical: &str) {
    javascript(
        plugin,
        browser,
        "(()=>{const tab=Array.from(document.querySelectorAll('[role=tab]')).find(tab=>tab.textContent.trim()==='Voice');if(!tab)throw new Error('Voice tab absent');tab.click();return 'clicked';})()",
    );
    wait_dom(
        plugin,
        browser,
        "document.querySelector('[data-knob=\"drums.level\"] [role=slider]')",
    );
    javascript(
        plugin,
        browser,
        "(()=>{const slider=document.querySelector('[data-knob=\"drums.level\"] [role=slider]');slider.focus();slider.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}));return 'editing';})()",
    );
    wait_dom(
        plugin,
        browser,
        "document.querySelector('[data-knob=\"drums.level\"] input')",
    );
    javascript(
        plugin,
        browser,
        &format!(
            "(()=>{{const input=document.querySelector('[data-knob=\"drums.level\"] input');Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(input,{physical:?});input.dispatchEvent(new Event('input',{{bubbles:true}}));return input.value;}})()"
        ),
    );
    javascript(
        plugin,
        browser,
        "(()=>{const input=document.querySelector('[data-knob=\"drums.level\"] input');input.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}));return 'committed';})()",
    );
    wait_dom(
        plugin,
        browser,
        "!document.querySelector('[data-knob=\"drums.level\"] input')",
    );
}
#[test]
#[ignore = "native parameter boundary: real inherited X11, copied release and host main loop"]
fn copied_native_host_timer_delivers_a_later_rendered_parameter_edit() {
    std::thread::spawn(||{
        let parent=Parent::new();let ledger=Rc::new(RefCell::new(TimerLedger::default()));
        let mut plugin=instantiate_options(&copied_entry(),true,Rc::clone(&ledger));
        let gui=plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
        gui.create(&plugin.plugin_handle(),GuiConfiguration{api_type:GuiApiType::X11,is_floating:false}).unwrap();
        unsafe{gui.set_parent(&plugin.plugin_handle(),Window::from_x11_handle(parent.id.into()))}.unwrap();gui.show(&plugin.plugin_handle()).unwrap();
        let browser=weak_browser_in_child(parent.children()[0]).upgrade().unwrap().downcast::<webkit2gtk::WebView>().unwrap();
        wait_dom(&mut plugin,&browser,"document.querySelector('[data-testid=trigger-editor]')");
        type_level(&mut plugin,&browser,"0.5");
        wait_dom(&mut plugin,&browser,"document.querySelector('[data-knob=\"drums.level\"] [role=slider]').getAttribute('aria-valuenow')==='0.125'");
        let params=plugin.plugin_handle().get_extension::<PluginParams>().unwrap();
        assert_eq!(params.get_value(&plugin.plugin_handle(),ClapId::new(3)),Some(1.0),"withheld timer/flush does not settle scalar");
        assert_eq!(plugin.access_shared_handler(|host|host.flush_requests.load(std::sync::atomic::Ordering::Acquire)),0,"DOM/GLib callbacks cannot call a borrowed host");
        let registered=ledger.borrow().live.clone();
        assert_eq!(registered.len(),1,"G2: real supported create must own one registered host timer after later DOM edit");
        let timer=plugin.plugin_handle().get_extension::<clack_extensions::timer::PluginTimer>().expect("G2: real PluginTimer callback");
        timer.on_timer(&plugin.plugin_handle(),clack_extensions::timer::TimerId(999));
        assert_eq!(plugin.access_shared_handler(|host|host.flush_requests.load(std::sync::atomic::Ordering::Acquire)),0,"unknown ticks do not request a flush");
        timer.on_timer(&plugin.plugin_handle(),registered[0]);
        assert!(plugin.access_shared_handler(|host|host.flush_requests.load(std::sync::atomic::Ordering::Acquire))>0,"actual matching tick requests a stopped flush");
        // Refuse the flush request first: a notification is not acceptance.
        assert_eq!(params.get_value(&plugin.plugin_handle(),ClapId::new(3)),Some(1.0));
        let mut output=Recorder::new(64);
        params.flush(&mut plugin.inactive_plugin_handle().unwrap(),&InputEvents::empty(),&mut OutputEvents::from_buffer(&mut output));
        assert_eq!(&output.accepted[..output.len],&[Some(Recorded::Begin(3)),Some(Recorded::Value(3,0.5)),Some(Recorded::End(3))]);
        assert_eq!(params.get_value(&plugin.plugin_handle(),ClapId::new(3)),Some(0.5));
        let notified=plugin.access_shared_handler(|host|host.flush_requests.load(std::sync::atomic::Ordering::Acquire));
        timer.on_timer(&plugin.plugin_handle(),registered[0]);timer.on_timer(&plugin.plugin_handle(),registered[0]);
        assert_eq!(plugin.access_shared_handler(|host|host.flush_requests.load(std::sync::atomic::Ordering::Acquire)),notified,"collected empty mirror does not request redundant flushes");
        gui.hide(&plugin.plugin_handle()).unwrap();assert_eq!(ledger.borrow().live,registered);
        gui.destroy(&plugin.plugin_handle());drop(browser);assert_eq!(ledger.borrow().live,registered,"destroying view retains instrument timer");
        drop(plugin);assert!(ledger.borrow().live.is_empty());assert_eq!(ledger.borrow().removals,registered,"final owner removes once");
    }).join().expect("real timer parameter host completed");
}

#[test]
#[ignore = "native parameter fixture: explicit copied release; synchronous standard CLAP reentrance"]
fn copied_native_timer_rollback_retains_disabled_cleanup_and_retries() {
    let ledger = Rc::new(RefCell::new(TimerLedger {
        refuse_removals: 1,
        ..Default::default()
    }));
    let mut plugin = instantiate_options(&copied_entry(), true, Rc::clone(&ledger));
    let before = seed_half_level(&mut plugin);
    let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
    let nested_accepted = Rc::new(std::cell::Cell::new(None));
    let observed = Rc::clone(&nested_accepted);
    let raw = plugin.plugin_handle().as_raw_ptr();
    // SAFETY: the live plugin owns its standard GUI extension. This pointer is
    // used only synchronously inside the next create/register call; the host
    // clears the one-shot callback immediately after that call, including failure.
    let extension =
        unsafe { ((*raw).get_extension.unwrap())(raw, clap_sys::ext::gui::CLAP_EXT_GUI.as_ptr()) }
            as *const clap_sys::ext::gui::clap_plugin_gui;
    assert!(!extension.is_null());
    let create = unsafe { (*extension).create.unwrap() };
    let destroy = unsafe { (*extension).destroy.unwrap() };
    ledger.borrow_mut().on_register = Some(Box::new(move || unsafe {
        observed.set(Some(create(
            raw,
            clap_sys::ext::gui::CLAP_WINDOW_API_X11.as_ptr(),
            false,
        )));
        destroy(raw);
    }));
    let embedded = GuiConfiguration {
        api_type: GuiApiType::X11,
        is_floating: false,
    };
    let rolled_back = gui.create(&plugin.plugin_handle(), embedded);
    drop(ledger.borrow_mut().on_register.take()); // No stored raw pointer survives the synchronous registration scope.
    assert!(
        rolled_back.is_err(),
        "reentrant destroy invalidates the provisional create"
    );
    assert_eq!(
        nested_accepted.get(),
        Some(false),
        "registering prevents reentrant duplicate create"
    );
    assert_eq!(gui.get_size(&plugin.plugin_handle()), None);
    assert_eq!(
        ledger.borrow().live,
        [clack_extensions::timer::TimerId(71)],
        "failed unregister retains actual host token"
    );
    assert_eq!(
        ledger.borrow().removals,
        [clack_extensions::timer::TimerId(71)]
    );
    gui.create(&plugin.plugin_handle(), embedded).unwrap();
    assert!(gui.create(&plugin.plugin_handle(), embedded).is_err());
    gui.destroy(&plugin.plugin_handle());
    gui.create(&plugin.plugin_handle(), embedded).unwrap();
    gui.destroy(&plugin.plugin_handle());
    assert_eq!(
        ledger.borrow().registrations,
        [
            (16, clack_extensions::timer::TimerId(71)),
            (16, clack_extensions::timer::TimerId(72))
        ]
    );
    assert_eq!(ledger.borrow().live, [clack_extensions::timer::TimerId(72)]);
    assert_preserved_half_level(&mut plugin, &before);
    drop(plugin);
    assert!(ledger.borrow().live.is_empty());
    assert_eq!(
        ledger.borrow().removals,
        [
            clack_extensions::timer::TimerId(71),
            clack_extensions::timer::TimerId(71),
            clack_extensions::timer::TimerId(72)
        ]
    );
}

#[test]
#[ignore = "native parameter boundary: real inherited X11, copied release and host main loop"]
fn copied_native_view_independent_source_collects_accepted_automation_credits() {
    std::thread::spawn(|| {
        let parent = Parent::new();
        let mut plugin = copied_plugin();
        let before = saved_state(&mut plugin);
        let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
        gui.create(&plugin.plugin_handle(), GuiConfiguration { api_type: GuiApiType::X11, is_floating: false }).unwrap();
        unsafe { gui.set_parent(&plugin.plugin_handle(), Window::from_x11_handle(parent.id.into())) }.unwrap();
        gui.show(&plugin.plugin_handle()).unwrap();
        let browser = weak_browser_in_child(parent.children()[0]).upgrade().unwrap().downcast::<webkit2gtk::WebView>().unwrap();
        wait_dom(&mut plugin, &browser, "document.querySelector('[data-testid=trigger-editor]')");
        type_level(&mut plugin, &browser, "0.5");
        wait_dom(&mut plugin, &browser, "document.querySelector('[data-knob=\"drums.level\"] [role=slider]').getAttribute('aria-valuenow')==='0.125'");
        gui.destroy(&plugin.plugin_handle());
        drop(browser);
        // Destroy stages the edit, but host acceptance happens only after the view is gone.
        let params = plugin.plugin_handle().get_extension::<PluginParams>().unwrap();
        let mut output = Recorder::new(64);
        params.flush(&mut plugin.inactive_plugin_handle().unwrap(), &InputEvents::empty(), &mut OutputEvents::from_buffer(&mut output));
        assert_eq!(&output.accepted[..output.len], &[Some(Recorded::Begin(3)), Some(Recorded::Value(3, 0.5)), Some(Recorded::End(3))]);
        let handle = plugin.plugin_handle();
        let state = handle.get_extension::<clack_extensions::state::PluginState>().unwrap();
        assert!(state.load(&handle, &mut before.as_slice()).is_err(), "accepted but uncollected credits still own the automation ledger");
        // Deliberately never dispatch HostTimer or on_main_thread after acceptance.
        let context = gtk::glib::MainContext::default();
        let deadline = Instant::now() + Duration::from_millis(100);
        while Instant::now() < deadline {
            for _ in 0..64 {
                if !context.pending() { break; }
                context.iteration(false);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let restored = state.load(&handle, &mut before.as_slice());
        drop(plugin);
        assert!(restored.is_ok(), "G2: view-independent GLib service must collect accepted credits without host timer/main callbacks: {restored:?}");
    }).join().expect("view-independent service host completed");
}

#[test]
#[ignore = "native parameter boundary: real inherited X11, copied release and host main loop"]
fn copied_native_host_feedback_updates_rendered_control_without_polling_or_undo_pollution() {
    std::thread::spawn(|| {
        let parent = Parent::new();
        let mut plugin = copied_plugin();
        let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
        gui.create(&plugin.plugin_handle(), GuiConfiguration { api_type: GuiApiType::X11, is_floating: false }).unwrap();
        unsafe { gui.set_parent(&plugin.plugin_handle(), Window::from_x11_handle(parent.id.into())) }.unwrap();
        gui.show(&plugin.plugin_handle()).unwrap();
        let browser = weak_browser_in_child(parent.children()[0]).upgrade().unwrap().downcast::<webkit2gtk::WebView>().unwrap();
        wait_dom(&mut plugin, &browser, "document.querySelector('[data-testid=trigger-editor]')");
        type_level(&mut plugin, &browser, "0.5");
        wait_dom(&mut plugin, &browser, "document.querySelector('[data-knob=\"drums.level\"] [role=slider]').getAttribute('aria-valuenow')==='0.125'");
        let params = plugin.plugin_handle().get_extension::<PluginParams>().unwrap();
        let mut edit = Recorder::new(64);
        params.flush(&mut plugin.inactive_plugin_handle().unwrap(), &InputEvents::empty(), &mut OutputEvents::from_buffer(&mut edit));
        assert_eq!(&edit.accepted[..edit.len], &[Some(Recorded::Begin(3)), Some(Recorded::Value(3, 0.5)), Some(Recorded::End(3))]);
        let state = invoke(&mut plugin, &browser, "window.__DANDRUM__.backend.getNativeFunction('getParameterState')()");
        assert_eq!(state["result"]["sequence"], 1);
        let history = invoke(&mut plugin, &browser, "window.__DANDRUM__.backend.getNativeFunction('getSamplerSession')()")["result"]["connection"]["history"].clone();
        assert_eq!(history, serde_json::json!({"undoCount":0,"redoCount":0}));
        wait_dom(&mut plugin, &browser, "document.querySelector('header button[aria-label=\"Undo Change drums.level (Ctrl+Z)\"]:not(:disabled)')");
        // Observe existing transport entrypoints without replacing production assets or consumers.
        javascript(&mut plugin, &browser, r#"(()=>{window.__G2_EVENTS__=[];window.__G2_CALLS__=[];const surface=window.__DANDRUM__,receive=surface.receiveNativeMessage,get=surface.backend.getNativeFunction;surface.receiveNativeMessage=message=>{const result=receive(message),frame=JSON.parse(message);if(frame.event==='parameterStateChanged')window.__G2_EVENTS__.push({...frame,consumption:result});return result;};surface.backend.getNativeFunction=method=>(...args)=>{window.__G2_CALLS__.push(method);return get(method)(...args);};return 'observing';})()"#);
        let mut audio = start_audio(&mut plugin);
        let mut events = EventBuffer::new();
        events.push(&MidiEvent::new(0, 0, [0x90, 36, 100]));
        events.push(&ParamValueEvent::new(1, ClapId::new(3), Pckn::match_all(), 2.0));
        let mut echo = Recorder::new(64);
        let pcm = render_audio(&mut audio, &events, &mut echo);
        return_audio(&mut plugin, audio);
        for channel in pcm { assert_eq!(&channel[..2], &[-0.25, -716.0 / 32768.0], "host value must apply at frame1, with old level audible at frame0"); }
        assert_eq!(echo.attempts, 0, "host changes must not echo as editor automation");
        assert_eq!(params.get_value(&plugin.plugin_handle(), ClapId::new(3)), Some(2.0));
        let deadline = Instant::now() + Duration::from_millis(100);
        while Instant::now() < deadline { dispatch_context(&mut plugin); std::thread::sleep(Duration::from_millis(2)); }
        let displayed = javascript(&mut plugin, &browser, "document.querySelector('[data-knob=\"drums.level\"] [role=slider]').getAttribute('aria-valuenow')");
        if displayed != "0.5" {
            gui.destroy(&plugin.plugin_handle());
            drop(browser);
            assert_eq!(displayed, "0.5", "G2: real host feedback must update the actual rendered control through an authenticated event");
            unreachable!();
        }
        assert_eq!(javascript(&mut plugin, &browser, "document.querySelector('[data-knob=\"drums.level\"] [role=slider]').getAttribute('aria-valuetext')"), "2");
        let frames: serde_json::Value = serde_json::from_str(&javascript(&mut plugin, &browser, "JSON.stringify(window.__G2_EVENTS__)" )).unwrap();
        let frame = frames.as_array().unwrap().iter().find(|frame| frame["payload"]["frame_offset"] == 1).expect("actual offset1 host feedback event");
        assert_eq!(frame["consumption"], "event");
        assert_eq!(frame["version"], 1);
        assert_eq!(frame["payload"]["generation"], 1);
        assert_eq!(frame["payload"]["sequence"], 1, "host automation may share the settled UI watermark");
        assert!(frame["sequence"].as_str().unwrap().parse::<u64>().unwrap() > 0);
        assert_eq!(frame["payload"]["parameters"], serde_json::json!([{"id":"drums.level","name":"drums.level","value":0.5}]));
        assert_eq!(javascript(&mut plugin, &browser, "String(window.__G2_EVENTS__.every(frame=>frame.instanceId===window.__DANDRUM_NATIVE__.instanceId&&frame.attachmentId===window.__DANDRUM_NATIVE__.attachmentId))"), "true");
        assert_eq!(javascript(&mut plugin, &browser, "String(window.__G2_CALLS__.filter(name=>name==='getParameterState'||name==='setParameter').length)"), "0", "host feedback must neither poll reads nor submit a UI edit");
        assert_eq!(javascript(&mut plugin, &browser, "String(Boolean(document.querySelector('header button[aria-label=\"Undo Change drums.level (Ctrl+Z)\"]:not(:disabled)')))"), "true");
        javascript(&mut plugin, &browser, "document.querySelector('header button[aria-label=\"Undo Change drums.level (Ctrl+Z)\"]').click();'undo clicked'");
        wait_dom(&mut plugin, &browser, "document.querySelector('[data-knob=\"drums.level\"] [role=slider]').getAttribute('aria-valuenow')==='0.25'");
        assert_eq!(javascript(&mut plugin, &browser, "String(window.__G2_CALLS__.filter(name=>name==='setParameter').length)"), "1", "calibration: actual header Undo uses the observed NativeEngine parameter call path");
        let mut undo = Recorder::new(64);
        params.flush(&mut plugin.inactive_plugin_handle().unwrap(), &InputEvents::empty(), &mut OutputEvents::from_buffer(&mut undo));
        assert_eq!(&undo.accepted[..undo.len], &[Some(Recorded::Begin(3)), Some(Recorded::Value(3, 1.0)), Some(Recorded::End(3))]);
        assert_eq!(params.get_value(&plugin.plugin_handle(), ClapId::new(3)), Some(1.0));
        let reads_before = javascript(&mut plugin, &browser, "String(window.__G2_CALLS__.filter(name=>name==='getParameterState').length)").parse::<usize>().unwrap();
        let state = invoke(&mut plugin, &browser, "window.__DANDRUM__.backend.getNativeFunction('getParameterState')()");
        assert_eq!(javascript(&mut plugin, &browser, "String(window.__G2_CALLS__.filter(name=>name==='getParameterState').length)").parse::<usize>().unwrap(), reads_before + 1, "calibration: known explicit read reaches the same observed getter");
        assert_eq!(state["result"]["sequence"], 2);
        assert_eq!(invoke(&mut plugin, &browser, "window.__DANDRUM__.backend.getNativeFunction('getSamplerSession')()")["result"]["connection"]["history"], history, "parameter undo leaves native structural history unchanged");
        wait_dom(&mut plugin, &browser, "document.querySelector('header button[aria-label=\"Nothing to undo\"]:disabled')");
        let mut audio = start_audio(&mut plugin);
        std::thread::scope(|scope| scope.spawn(|| audio.reset()).join().unwrap());
        let mut note = EventBuffer::new(); note.push(&MidiEvent::new(3, 0, [0x90, 36, 100]));
        let mut output = Recorder::new(64);
        let pcm = render_audio(&mut audio, &note, &mut output);
        return_audio(&mut plugin, audio);
        for channel in pcm { assert_eq!(&channel[..3], &[0.0;3]); assert_eq!(&channel[3..5], &[-0.5, -358.0 / 32768.0]); }
        assert_eq!(output.attempts, 0);
        gui.destroy(&plugin.plugin_handle()); drop(browser);
    }).join().expect("host feedback and parameter undo host completed");
}

#[test]
#[ignore = "native parameter boundary: real inherited X11, copied release and host main loop"]
fn copied_native_invalid_stopped_flush_preserves_uncollected_automation_ownership() {
    std::thread::spawn(|| {
        let parent = Parent::new();
        let mut plugin = copied_plugin();
        let initial = saved_state(&mut plugin);
        let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
        gui.create(&plugin.plugin_handle(), GuiConfiguration { api_type: GuiApiType::X11, is_floating: false }).unwrap();
        unsafe { gui.set_parent(&plugin.plugin_handle(), Window::from_x11_handle(parent.id.into())) }.unwrap();
        gui.show(&plugin.plugin_handle()).unwrap();
        let browser = weak_browser_in_child(parent.children()[0]).upgrade().unwrap().downcast::<webkit2gtk::WebView>().unwrap();
        wait_dom(&mut plugin, &browser, "document.querySelector('[data-testid=trigger-editor]')");
        type_level(&mut plugin, &browser, "0.5");
        wait_dom(&mut plugin, &browser, "document.querySelector('[data-knob=\"drums.level\"] [role=slider]').getAttribute('aria-valuenow')==='0.125'");
        gui.destroy(&plugin.plugin_handle()); drop(browser);
        let params = plugin.plugin_handle().get_extension::<PluginParams>().unwrap();
        let mut accepted = Recorder::new(64);
        params.flush(&mut plugin.inactive_plugin_handle().unwrap(), &InputEvents::empty(), &mut OutputEvents::from_buffer(&mut accepted));
        assert_eq!(&accepted.accepted[..accepted.len], &[Some(Recorded::Begin(3)), Some(Recorded::Value(3, 0.5)), Some(Recorded::End(3))]);
        let settled = saved_state(&mut plugin);
        let handle = plugin.plugin_handle();
        let state = handle.get_extension::<clack_extensions::state::PluginState>().unwrap();
        assert!(state.load(&handle, &mut initial.as_slice()).is_err());
        // No GLib, timer or requested main callback is dispatched across this boundary.
        let mut invalid = EventBuffer::new();
        invalid.push(&ParamValueEvent::new(0, ClapId::new(24), Pckn::match_all(), 0.25));
        let mut rejected = Recorder::new(64);
        params.flush(&mut plugin.inactive_plugin_handle().unwrap(), &invalid.as_input(), &mut OutputEvents::from_buffer(&mut rejected));
        assert_eq!(rejected.attempts, 0);
        assert!(state.load(&plugin.plugin_handle(), &mut initial.as_slice()).is_err(), "G2: rejected complete input validation must not collect prior accepted credits or release automation ownership");
        assert_eq!(params.get_value(&plugin.plugin_handle(), ClapId::new(3)), Some(0.5));
        assert_eq!(saved_state(&mut plugin), settled);
        let mut valid = Recorder::new(64);
        params.flush(&mut plugin.inactive_plugin_handle().unwrap(), &InputEvents::empty(), &mut OutputEvents::from_buffer(&mut valid));
        assert_eq!(valid.attempts, 0);
        assert!(state.load(&plugin.plugin_handle(), &mut initial.as_slice()).is_ok(), "later valid stopped service collects exact accepted credits");
        assert_eq!(params.get_value(&plugin.plugin_handle(), ClapId::new(3)), Some(1.0));
    }).join().expect("invalid stopped ownership host completed");
}

#[test]
#[ignore = "native callback audit: trusted copied product, real inherited X11 and explicit calibrated child"]
fn copied_native_parameter_callbacks_allocate_and_destroy_nothing_while_gui_services() {
    let directory = tempfile::tempdir().unwrap();
    let interposer = directory.path().join("allocator_interpose.c");
    let probe = directory.path().join("allocator_probe.rs");
    let library = directory.path().join("allocator_interpose.so");
    let rust_library = directory.path().join("allocator_probe.so");
    std::fs::write(&interposer, include_str!("fixtures/allocator_interpose.c")).unwrap();
    std::fs::write(&probe, include_str!("fixtures/allocator_probe.rs")).unwrap();
    let output = std::process::Command::new("cc")
        .args([
            "-shared", "-fPIC", "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror",
        ])
        .arg(&interposer)
        .arg("-o")
        .arg(&library)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "interposer compile: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = std::process::Command::new("rustc")
        .args(["--edition=2024", "--crate-type=cdylib", "-Copt-level=1"])
        .arg(&probe)
        .arg("-o")
        .arg(&rust_library)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Rust probe compile: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let executable = std::env::current_exe().unwrap();
    let name = "native_parameter_callback_allocator_audit_child";
    let listing = std::process::Command::new(&executable)
        .arg("--list")
        .output()
        .unwrap();
    assert!(listing.status.success());
    assert!(
        String::from_utf8_lossy(&listing.stdout)
            .lines()
            .any(|line| line == format!("{name}: test"))
    );
    let hashes = std::process::Command::new("sha256sum")
        .args([
            artifact(),
            executable.clone(),
            library.clone(),
            rust_library.clone(),
            interposer,
            probe,
        ])
        .output()
        .unwrap();
    assert!(hashes.status.success());
    println!(
        "Audit fixture compilers: cc -shared -fPIC -std=c11 -O2 -Wall -Wextra -Werror; rustc --edition=2024 --crate-type=cdylib -Copt-level=1"
    );
    println!(
        "Explicit audit child: LD_PRELOAD={} DANDRUM_ALLOCATOR_AUDIT_PROBE={} {} --exact {name} --ignored --show-output --test-threads=1\n{}",
        library.display(),
        rust_library.display(),
        executable.display(),
        String::from_utf8_lossy(&hashes.stdout)
    );
    let output = std::process::Command::new(&executable)
        .args([
            "--exact",
            name,
            "--ignored",
            "--show-output",
            "--test-threads=1",
        ])
        .env("LD_PRELOAD", &library)
        .env("DANDRUM_ALLOCATOR_AUDIT_PROBE", &rust_library)
        .env_remove("RUST_MIN_STACK")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{stdout}");
    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    assert!(
        output.status.success(),
        "calibrated actual plugin DSO callback audit child failed"
    );
    assert!(
        stdout.contains("running 1 test")
            && stdout.contains(&format!("test {name} ... ok"))
            && stdout.contains("test result: ok. 1 passed; 0 failed;"),
        "child must execute its exact one test"
    );
}

#[test]
#[ignore = "only the calibrated native callback audit parent may launch this preloaded child"]
fn native_parameter_callback_allocator_audit_child() {
    std::thread::spawn(parameter_callback_allocator_audit_on_host_thread)
        .join()
        .expect("calibrated GUI/audio audit host completed");
}

fn parameter_callback_allocator_audit_on_host_thread() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };
    type Begin = unsafe extern "C" fn();
    type End = unsafe extern "C" fn(*mut u64, *mut u64);
    #[derive(Clone, Copy)]
    struct Audit {
        begin: Begin,
        end: End,
    }
    impl Audit {
        fn measure<T>(&self, callback: impl FnOnce() -> T) -> (T, (u64, u64)) {
            unsafe {
                (self.begin)();
            }
            let result = callback();
            let (mut allocations, mut deallocations) = (0, 0);
            unsafe {
                (self.end)(&mut allocations, &mut deallocations);
            }
            (result, (allocations, deallocations))
        }
    }
    // The trusted parent supplies these test-only observer symbols and probe.
    let global = libloading::os::unix::Library::this();
    let audit = unsafe {
        Audit {
            begin: *global.get::<Begin>(b"dandrum_audit_begin\0").unwrap(),
            end: *global.get::<End>(b"dandrum_audit_end\0").unwrap(),
        }
    };
    let probe = unsafe {
        libloading::Library::new(std::env::var_os("DANDRUM_ALLOCATOR_AUDIT_PROBE").unwrap())
    }
    .unwrap();
    let allocate = unsafe {
        *probe
            .get::<unsafe extern "C" fn() -> *mut u8>(b"probe_allocate\0")
            .unwrap()
    };
    let deallocate = unsafe {
        *probe
            .get::<unsafe extern "C" fn(*mut u8)>(b"probe_deallocate\0")
            .unwrap()
    };
    let old = unsafe { allocate() };
    assert!(!old.is_null());
    let (fresh, allocation) = audit.measure(|| unsafe { allocate() });
    assert!(!fresh.is_null());
    assert!(allocation.0 > 0);
    assert_eq!(allocation.1, 0);
    let (_, destruction) = audit.measure(|| unsafe { deallocate(old) });
    assert_eq!(destruction.0, 0);
    assert!(destruction.1 > 0);
    unsafe {
        deallocate(fresh);
    }
    assert_eq!(audit.measure(|| ()).1, (0, 0));
    let parent = Parent::new();
    let mut plugin = copied_plugin();
    let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
    gui.create(
        &plugin.plugin_handle(),
        GuiConfiguration {
            api_type: GuiApiType::X11,
            is_floating: false,
        },
    )
    .unwrap();
    unsafe {
        gui.set_parent(
            &plugin.plugin_handle(),
            Window::from_x11_handle(parent.id.into()),
        )
    }
    .unwrap();
    gui.show(&plugin.plugin_handle()).unwrap();
    let browser = weak_browser_in_child(parent.children()[0])
        .upgrade()
        .unwrap()
        .downcast::<webkit2gtk::WebView>()
        .unwrap();
    wait_dom(
        &mut plugin,
        &browser,
        "document.querySelector('[data-testid=trigger-editor]')",
    );
    type_level(&mut plugin, &browser, "0.5");
    wait_dom(
        &mut plugin,
        &browser,
        "document.querySelector('[data-knob=\"drums.level\"] [role=slider]').getAttribute('aria-valuenow')==='0.125'",
    );
    let timer = plugin
        .plugin_handle()
        .get_extension::<clack_extensions::timer::PluginTimer>()
        .unwrap();
    timer.on_timer(
        &plugin.plugin_handle(),
        clack_extensions::timer::TimerId(71),
    );
    let mut audio = start_audio(&mut plugin);
    let ready = Arc::new(AtomicBool::new(false));
    let go = Arc::new(AtomicBool::new(false));
    let stop = Arc::new(AtomicBool::new(false));
    let servicing = Arc::new(AtomicBool::new(false));
    let overlap = Arc::new(AtomicUsize::new(0));
    let callbacks = Arc::new(AtomicUsize::new(0));
    struct Stop(Arc<AtomicBool>);
    impl Drop for Stop {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }
    let (audio, recorded, flush_count, process_count, iterations, first_pcm, browser_progress) =
        std::thread::scope(|scope| {
            let _stop_on_exit = Stop(Arc::clone(&stop));
            let (
                ready_audio,
                go_audio,
                stop_audio,
                servicing_audio,
                overlap_audio,
                callbacks_audio,
            ) = (
                Arc::clone(&ready),
                Arc::clone(&go),
                Arc::clone(&stop),
                Arc::clone(&servicing),
                Arc::clone(&overlap),
                Arc::clone(&callbacks),
            );
            let worker = scope.spawn(move || {
                let mut recorded = Recorder::new(1);
                let mut handle = audio.plugin_handle();
                let params = handle.get_extension::<PluginParams>().unwrap();
                let (_, flush_count) = audit.measure(|| {
                    params.flush_active(
                        &mut handle,
                        &InputEvents::empty(),
                        &mut OutputEvents::from_buffer(&mut recorded),
                    )
                });
                assert_eq!(recorded.len, 1);
                assert_eq!(recorded.attempts, 2);
                recorded.remaining = 64;
                ready_audio.store(true, Ordering::Release);
                while !go_audio.load(Ordering::Acquire) && !stop_audio.load(Ordering::Acquire) {
                    std::thread::yield_now();
                }
                let mut inputs = AudioPorts::with_capacity(0, 0);
                let input = inputs.with_input_buffers(std::iter::empty::<
                    AudioPortBuffer<[InputChannel<'_, f32>; 0], [InputChannel<'_, f64>; 0]>,
                >());
                let mut ports = AudioPorts::with_capacity(2, 1);
                let mut pcm = [[123.0f32; 16]; 2];
                let mut first_pcm = pcm;
                let mut note = EventBuffer::new();
                note.push(&MidiEvent::new(3, 0, [0x90, 36, 100]));
                let mut iterations = 0;
                let mut process_count = (0, 0);
                while !stop_audio.load(Ordering::Acquire) && iterations < 100_000 {
                    let input_events = if iterations == 0 {
                        note.as_input()
                    } else {
                        InputEvents::empty()
                    };
                    recorded.attempts = 0;
                    let during_service = servicing_audio.load(Ordering::Acquire);
                    let (status, count) = {
                        let mut output = ports.with_output_buffers([AudioPortBuffer {
                            latency: 0,
                            channels: AudioPortBufferType::f32_output_only(
                                pcm.iter_mut().map(|channel| channel.as_mut_slice()),
                            ),
                        }]);
                        audit.measure(|| {
                            audio.process(
                                &input,
                                &mut output,
                                &input_events,
                                &mut OutputEvents::from_buffer(&mut recorded),
                                None,
                                None,
                            )
                        })
                    };
                    assert_eq!(
                        status.unwrap(),
                        clack_host::process::ProcessStatus::Continue
                    );
                    process_count.0 += count.0;
                    process_count.1 += count.1;
                    if during_service && servicing_audio.load(Ordering::Acquire) {
                        overlap_audio.fetch_add(1, Ordering::Release);
                    }
                    if iterations == 0 {
                        first_pcm = pcm;
                    }
                    iterations += 1;
                    callbacks_audio.fetch_add(1, Ordering::Release);
                }
                (
                    audio,
                    recorded,
                    flush_count,
                    process_count,
                    iterations,
                    first_pcm,
                )
            });
            let deadline = Instant::now() + Duration::from_secs(3);
            while !ready.load(Ordering::Acquire) {
                assert!(
                    Instant::now() < deadline,
                    "audio audit did not reach ready boundary"
                );
                std::thread::sleep(Duration::from_millis(1));
            }
            let (go_main, servicing_main, overlap_main) = (
                Arc::clone(&go),
                Arc::clone(&servicing),
                Arc::clone(&overlap),
            );
            gtk::glib::idle_add_local_once(move || {
                servicing_main.store(true, Ordering::Release);
                go_main.store(true, Ordering::Release);
                let deadline = Instant::now() + Duration::from_secs(3);
                while overlap_main.load(Ordering::Acquire) == 0 && Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(1));
                }
                servicing_main.store(false, Ordering::Release);
            });
            let displayed = javascript(
                &mut plugin,
                &browser,
                "document.querySelector('[data-knob=\"drums.level\"] [role=slider]').getAttribute('aria-valuenow')",
            );
            assert_eq!(displayed, "0.125");
            let before = callbacks.load(Ordering::Acquire);
            let deadline = Instant::now() + Duration::from_millis(30);
            while Instant::now() < deadline {
                dispatch_context(&mut plugin);
                std::thread::sleep(Duration::from_millis(1));
            }
            let after = callbacks.load(Ordering::Acquire);
            stop.store(true, Ordering::Release);
            let (audio, recorded, flush_count, process_count, iterations, first_pcm) =
                worker.join().unwrap();
            (
                audio,
                recorded,
                flush_count,
                process_count,
                iterations,
                first_pcm,
                after - before,
            )
        });
    return_audio(&mut plugin, audio);
    assert_eq!(
        flush_count,
        (0, 0),
        "actual plugin+Clack active flush with accepted Begin and rejected Value head"
    );
    assert_eq!(
        process_count,
        (0, 0),
        "actual plugin+Clack process delivery and empty callbacks during MainContext service"
    );
    assert!(iterations > 0 && browser_progress > 0 && overlap.load(Ordering::Acquire) > 0);
    assert_eq!(
        &recorded.accepted[..recorded.len],
        &[
            Some(Recorded::Begin(3)),
            Some(Recorded::Value(3, 0.5)),
            Some(Recorded::End(3))
        ]
    );
    for channel in first_pcm {
        assert_eq!(&channel[..3], &[0.0; 3]);
        assert_eq!(&channel[3..5], &[-0.25, -179.0 / 32768.0]);
    }
    gui.destroy(&plugin.plugin_handle());
    drop(browser);
    println!(
        "Rust cdylib calibration allocate={allocation:?} preexisting-destruction={destruction:?}; plugin+Clack activeflush={flush_count:?}, process={process_count:?}, callbacks={iterations}, whole callback/service overlaps={}, callbacks during live browser/context progress={browser_progress}",
        overlap.load(Ordering::Acquire)
    );
}

#[path = "internal/trigger_structural_native.rs"]
mod structural_native;
