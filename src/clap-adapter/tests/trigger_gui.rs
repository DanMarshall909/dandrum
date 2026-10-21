//! Slow product-boundary tests use an explicitly copied, trusted release plugin.
#![cfg(all(feature = "trigger-editor", target_os = "linux"))]

use clack_extensions::{
    gui::{GuiApiType, GuiConfiguration, GuiSize, HostGui, HostGuiImpl, PluginGui, Window},
    params::PluginParams,
};
use clack_host::entry::PluginEntry;
use clack_host::prelude::*;
use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

struct GuiHost;
#[derive(Default)]
struct TimerHost(std::cell::Cell<Option<clack_extensions::timer::TimerId>>);
impl MainThreadHandler<'_> for TimerHost {}
impl clack_extensions::timer::HostTimerImpl for TimerHost {
    fn register_timer(
        &self,
        period_ms: u32,
    ) -> Result<clack_extensions::timer::TimerId, HostError> {
        assert_eq!(period_ms, 16);
        assert!(self.0.get().is_none());
        let id = clack_extensions::timer::TimerId(1);
        self.0.set(Some(id));
        Ok(id)
    }
    fn unregister_timer(&self, id: clack_extensions::timer::TimerId) -> Result<(), HostError> {
        assert_eq!(self.0.take(), Some(id));
        Ok(())
    }
}
struct Shared(std::sync::atomic::AtomicBool);

impl SharedHandler<'_> for Shared {
    fn request_restart(&self) {}
    fn request_process(&self) {}
    fn request_callback(&self) {
        self.0.store(true, std::sync::atomic::Ordering::Release);
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
    type MainThread<'a> = TimerHost;
    type AudioProcessor<'a> = ();

    fn declare_extensions(builder: &mut clack_host::host::HostExtensions<Self>, _: &Shared) {
        builder
            .register::<HostGui>()
            .register::<clack_extensions::timer::HostTimer>();
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
    PluginInstance::new(
        |_| Shared(std::sync::atomic::AtomicBool::new(false)),
        |_| TimerHost::default(),
        entry,
        c"org.dandrum.trigger",
        &HostInfo::new("Trigger GUI proof", "Dandrum", "", "0.1").unwrap(),
    )
    .expect("the copied Trigger initializes from its sibling resources")
}
fn copied_plugin() -> PluginInstance<GuiHost> {
    instantiate(&copied_entry())
}

// Selected Linux /proc observation, independent of plugin's residency implementation.
fn map_lines() -> Vec<Vec<u8>> {
    std::fs::read("/proc/self/maps")
        .unwrap()
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(<[u8]>::to_vec)
        .collect()
}
fn file_mappings(path: &Path) -> Vec<String> {
    let metadata = std::fs::metadata(path).unwrap();
    let dev = metadata.dev();
    let major = ((dev & 0x00000000000fff00) >> 8) | ((dev & 0xfffff00000000000) >> 32);
    let minor = (dev & 0xff) | ((dev & 0x00000ffffff00000) >> 12);
    map_lines()
        .into_iter()
        .filter(|line| {
            // Only the fixed ASCII prefix is parsed; pathname bytes need not be UTF-8.
            let fields: Vec<_> = line
                .split(u8::is_ascii_whitespace)
                .filter(|s| !s.is_empty())
                .take(5)
                .collect();
            let Some((map_major, map_minor)) = fields
                .get(3)
                .and_then(|s| std::str::from_utf8(s).ok())
                .and_then(|s| s.split_once(':'))
            else {
                return false;
            };
            u64::from_str_radix(map_major, 16).ok() == Some(major)
                && u64::from_str_radix(map_minor, 16).ok() == Some(minor)
                && fields
                    .get(4)
                    .and_then(|s| std::str::from_utf8(s).ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    == Some(metadata.ino())
        })
        .map(|line| String::from_utf8_lossy(&line).into_owned())
        .collect()
}
fn webkit_path() -> Option<PathBuf> {
    use std::os::unix::ffi::OsStrExt;
    map_lines().iter().find_map(|line| {
        let field = line
            .split(u8::is_ascii_whitespace)
            .filter(|s| !s.is_empty())
            .nth(5)?;
        let path = Path::new(std::ffi::OsStr::from_bytes(field));
        path.file_name()?
            .as_bytes()
            .starts_with(b"libwebkit2gtk-4.1.so.")
            .then(|| path.to_path_buf())
    })
}
fn log_mappings(phase: &str, path: &Path) {
    let metadata = std::fs::metadata(path).unwrap();
    eprintln!(
        "G1_MAPS phase={phase} pid={} thread={:?} path={path:?} dev={} inode={} ranges={:?}",
        std::process::id(),
        std::thread::current().id(),
        metadata.dev(),
        metadata.ino(),
        file_mappings(path)
    );
}
fn dispatch_host_after_entry_release() {
    let thread = std::thread::current().id();
    let context = gtk::glib::MainContext::default();
    for _ in 0..8 {
        for _ in 0..64 {
            if !context.pending() {
                break;
            }
            context.iteration(false);
        }
    }
    assert_eq!(
        std::thread::current().id(),
        thread,
        "same host thread after Entry release"
    );
}
// Public GTK/GObject introspection of the independently observed X11 child.
// No WebKit symbol/type is linked into the host and no NativeEditor private handle is used.
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

fn rescan_already_loaded(path: &Path) {
    // Linux dlfcn.h RTLD_NOLOAD=0x4. NOW is required, and no GLOBAL/NODELETE flag is used.
    // Some loaders defer the final rescan while owner-thread TLS destructors remain.
    let before = file_mappings(path);
    let opened = unsafe {
        libloading::os::unix::Library::open(Some(path), libloading::os::unix::RTLD_NOW | 0x4)
    };
    match opened {
        Ok(library) => library.close().expect("balanced NOLOAD handle close"),
        Err(error) => assert!(
            before.is_empty(),
            "already-mapped artifact must admit a balanced NOLOAD rescan: {error}; maps={before:?}"
        ),
    }
}

fn saved_state(plugin: &mut PluginInstance<GuiHost>) -> Vec<u8> {
    let handle = plugin.plugin_handle();
    let mut bytes = Vec::new();
    handle
        .get_extension::<clack_extensions::state::PluginState>()
        .unwrap()
        .save(&handle, &mut bytes)
        .unwrap();
    bytes
}

#[test]
#[ignore = "requires an explicitly copied experimental editor release product"]
fn copied_trigger_editor_exposes_the_actual_clap_gui_extension() {
    let mut plugin = copied_plugin();
    let handle = plugin.plugin_handle();
    let params = handle.get_extension::<PluginParams>().unwrap();
    assert_eq!(params.count(&handle), 23, "the real session initialized");
    assert!(
        handle.get_extension::<PluginGui>().is_some(),
        "G1: the opt-in Trigger editor product must expose the actual CLAP GUI extension"
    );
}

use clack_host::{
    events::event_types::{MidiEvent, ParamValueEvent},
    process::StartedPluginAudioProcessor,
};
use std::time::{Duration, Instant};
use x11rb::{
    COPY_DEPTH_FROM_PARENT,
    connection::Connection,
    protocol::xproto::{ConnectionExt, CreateWindowAux, MapState, WindowClass},
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
    fn state(&self, id: u32) -> MapState {
        self.x
            .get_window_attributes(id)
            .unwrap()
            .reply()
            .unwrap()
            .map_state
    }
}
impl Drop for Parent {
    fn drop(&mut self) {
        self.x.destroy_window(self.id).unwrap().check().unwrap();
    }
}
struct CopiedApp {
    path: PathBuf,
    original: Vec<u8>,
}
impl CopiedApp {
    fn observe(&self, instance: u64) {
        let mut bytes = self.original.clone();
        bytes.extend_from_slice(
            format!("\n;window.__G1_EXPECT_INSTANCE__=\"{instance}\";\n").as_bytes(),
        );
        bytes.extend_from_slice(include_bytes!("fixtures/trigger-gui-native-observer.js"));
        std::fs::write(&self.path, bytes).unwrap();
    }
}
impl Drop for CopiedApp {
    fn drop(&mut self) {
        std::fs::write(&self.path, &self.original)
            .expect("restore copied production app even after failed assertion");
    }
}
fn activate(plugin: &mut PluginInstance<GuiHost>) -> StartedPluginAudioProcessor<GuiHost> {
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
fn block(
    processor: &mut StartedPluginAudioProcessor<GuiHost>,
    events: &EventBuffer,
    reset: bool,
) -> [Vec<f32>; 2] {
    let mut output = [vec![123.0; 16], vec![123.0; 16]];
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                if reset {
                    processor.reset();
                }
                let mut inputs = AudioPorts::with_capacity(0, 0);
                let input = inputs.with_input_buffers(std::iter::empty::<
                    AudioPortBuffer<[InputChannel<'_, f32>; 0], [InputChannel<'_, f64>; 0]>,
                >());
                let mut ports = AudioPorts::with_capacity(2, 1);
                let mut buffers = ports.with_output_buffers([AudioPortBuffer {
                    latency: 0,
                    channels: AudioPortBufferType::f32_output_only(
                        output.iter_mut().map(|x| x.as_mut_slice()),
                    ),
                }]);
                assert_eq!(
                    processor
                        .process(
                            &input,
                            &mut buffers,
                            &InputEvents::from_buffer(events),
                            &mut EventBuffer::new().as_output(),
                            None,
                            None
                        )
                        .unwrap(),
                    clack_host::process::ProcessStatus::Continue
                );
            })
            .join()
            .unwrap();
    });
    output
}
fn assert_host_audio(processor: &mut StartedPluginAudioProcessor<GuiHost>) {
    let mut events = EventBuffer::new();
    events.push(&MidiEvent::new(3, 0, [0x90, 36, 100]));
    let output = block(processor, &events, true);
    for channel in output {
        assert_eq!(&channel[..3], &[0.0; 3]);
        assert_eq!(&channel[3..5], &[-0.5, -358.0 / 32768.0]);
    }
}
fn tick(plugin: &mut PluginInstance<GuiHost>) {
    let context = gtk::glib::MainContext::default();
    for _ in 0..64 {
        if !context.pending() {
            break;
        }
        context.iteration(false);
    }
    if plugin.access_shared_handler(|s| s.0.swap(false, std::sync::atomic::Ordering::AcqRel)) {
        plugin.call_on_main_thread_callback();
    }
}
fn observe_ready_note(
    plugin: &mut PluginInstance<GuiHost>,
    processor: &mut StartedPluginAudioProcessor<GuiHost>,
) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        tick(plugin);
        // No host MIDI: only the authenticated browser observer can start this note.
        let output = block(processor, &EventBuffer::new(), false);
        if output[0].iter().any(|v| *v != 0.0) {
            for channel in output {
                assert_eq!(
                    &channel[..2],
                    &[-0.5, -358.0 / 32768.0],
                    "real React/native startup readiness PCM"
                );
            }
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!(
        "G1: actual React/native session did not complete the authenticated readiness note within15s"
    );
}

#[test]
#[ignore = "native boundary lane: real inherited X11/host loop and copied release/resources; outer timeout60s"]
fn copied_native_editor_lifecycle_initializes_the_real_react_session_and_preserves_audio() {
    let path = artifact();
    let webkit = std::thread::spawn(native_lifecycle_on_host_thread)
        .join()
        .expect("owned native host thread completed");
    log_mappings("native-after-owner-thread-join", &path);
    rescan_already_loaded(&path);
    log_mappings("native-after-balanced-loader-rescan", &path);
    log_mappings("webkit-after-balanced-loader-rescan", &webkit);
    assert!(
        !file_mappings(&path).is_empty(),
        "G1: native code residency persists after owner TLS teardown and balanced loader rescan"
    );
    assert!(
        !file_mappings(&webkit).is_empty(),
        "G1: native dependency residency outlives the released host UI thread"
    );
}
fn native_lifecycle_on_host_thread() -> PathBuf {
    let parent_owner = std::cell::OnceCell::new(); // Declared before plugin: its live parent outlives it on failure.
    let path = artifact();
    assert!(
        file_mappings(&path).is_empty(),
        "fresh process starts without a retained plugin loader reference"
    );
    assert!(
        webkit_path().is_none(),
        "host has no prior WebKit loader reference; otherwise this residency oracle needs qualification"
    );
    log_mappings("before-entry", &path);
    let entry = copied_entry();
    let mut plugin = instantiate(&entry);
    let mut processor = activate(&mut plugin);
    assert_host_audio(&mut processor);
    let gui = plugin
        .plugin_handle()
        .get_extension::<PluginGui>()
        .expect("G1: actual CLAP GUI extension");
    let parent = parent_owner.get_or_init(Parent::new);
    let embedded = GuiConfiguration {
        api_type: GuiApiType::X11,
        is_floating: false,
    };
    let floating = GuiConfiguration {
        api_type: GuiApiType::X11,
        is_floating: true,
    };
    let wayland = GuiConfiguration {
        api_type: GuiApiType::WAYLAND,
        is_floating: false,
    };
    let fixed = GuiSize {
        width: 1200,
        height: 800,
    };
    assert!(gui.is_api_supported(&plugin.plugin_handle(), embedded));
    assert!(!gui.is_api_supported(&plugin.plugin_handle(), floating));
    assert!(!gui.is_api_supported(&plugin.plugin_handle(), wayland));
    let preferred = gui.get_preferred_api(&plugin.plugin_handle()).unwrap();
    assert_eq!(preferred.api_type, GuiApiType::X11);
    assert!(!preferred.is_floating);
    assert!(gui.create(&plugin.plugin_handle(), floating).is_err());
    assert!(gui.create(&plugin.plugin_handle(), wayland).is_err());
    assert!(gui.show(&plugin.plugin_handle()).is_err());
    assert!(gui.hide(&plugin.plugin_handle()).is_err());
    // SAFETY: parent remains live until all GUI children and the plugin are dropped.
    assert!(
        unsafe {
            gui.set_parent(
                &plugin.plugin_handle(),
                Window::from_x11_handle(parent.id.into()),
            )
        }
        .is_err()
    );
    gui.destroy(&plugin.plugin_handle());
    gui.destroy(&plugin.plugin_handle());
    assert_host_audio(&mut processor);
    let plugin_path = PathBuf::from(std::env::var_os("DANDRUM_TRIGGER_GUI_PLUGIN_PATH").unwrap());
    let app = plugin_path
        .parent()
        .unwrap()
        .join("DandrumTrigger.resources/editor/app.js");
    let copied_app = CopiedApp {
        original: std::fs::read(&app).expect("copied production app"),
        path: app.clone(),
    };
    copied_app.observe(1);
    for cycle in 0..2 {
        gui.create(&plugin.plugin_handle(), embedded).unwrap();
        assert!(gui.create(&plugin.plugin_handle(), embedded).is_err());
        assert_eq!(gui.get_size(&plugin.plugin_handle()), Some(fixed));
        assert!(!gui.can_resize(&plugin.plugin_handle()));
        assert!(gui.set_scale(&plugin.plugin_handle(), 1.0).is_err());
        assert!(
            gui.set_size(
                &plugin.plugin_handle(),
                GuiSize {
                    width: 600,
                    height: 400
                }
            )
            .is_err()
        );
        gui.set_size(&plugin.plugin_handle(), fixed).unwrap();
        assert!(gui.show(&plugin.plugin_handle()).is_err());
        assert!(gui.hide(&plugin.plugin_handle()).is_err());
        // Exercise raw invalid host inputs deliberately; the plugin must reject without dereferencing them.
        for invalid in [0, u64::from(u32::MAX) + 1] {
            assert!(
                unsafe {
                    gui.set_parent(&plugin.plugin_handle(), Window::from_x11_handle(invalid))
                }
                .is_err()
            );
        }
        if cycle == 0 {
            // Real asset-construction failure after create, followed by retry on the same configuration.
            let retained = app.with_extension("js.g1-retained");
            std::fs::rename(&app, &retained).unwrap();
            let failure = unsafe {
                gui.set_parent(
                    &plugin.plugin_handle(),
                    Window::from_x11_handle(parent.id.into()),
                )
            };
            std::fs::rename(retained, &app).unwrap();
            assert!(failure.is_err());
            assert!(parent.children().is_empty());
            assert_host_audio(&mut processor);
        }
        assert_eq!(
            block(&mut processor, &EventBuffer::new(), true),
            [vec![0.0; 16], vec![0.0; 16]],
            "clear host-held pitch and tails before the browser can admit readiness notes"
        );
        unsafe {
            gui.set_parent(
                &plugin.plugin_handle(),
                Window::from_x11_handle(parent.id.into()),
            )
        }
        .unwrap();
        let children = parent.children();
        assert_eq!(children.len(), 1);
        let child = children[0];
        let geometry = parent.x.get_geometry(child).unwrap().reply().unwrap();
        assert_eq!(
            (geometry.x, geometry.y, geometry.width, geometry.height),
            (0, 0, 1200, 800)
        );
        assert_eq!(parent.state(child), MapState::UNMAPPED);
        assert!(
            unsafe {
                gui.set_parent(
                    &plugin.plugin_handle(),
                    Window::from_x11_handle(parent.id.into()),
                )
            }
            .is_err()
        );
        assert!(
            unsafe {
                gui.set_transient(
                    &plugin.plugin_handle(),
                    Window::from_x11_handle(parent.id.into()),
                )
            }
            .is_err()
        );
        gui.show(&plugin.plugin_handle()).unwrap();
        assert_eq!(parent.state(child), MapState::VIEWABLE);
        let browser = weak_browser_in_child(child);
        observe_ready_note(&mut plugin, &mut processor);
        gui.hide(&plugin.plugin_handle()).unwrap();
        assert_eq!(parent.state(child), MapState::UNMAPPED);
        gui.show(&plugin.plugin_handle()).unwrap();
        assert_eq!(parent.children(), vec![child]);
        gui.set_size(&plugin.plugin_handle(), fixed).unwrap();
        gui.destroy(&plugin.plugin_handle());
        gui.destroy(&plugin.plugin_handle());
        assert!(
            browser.upgrade().is_none(),
            "actual copied-plugin WebView releases before any post-destroy host dispatch"
        );
        assert!(parent.children().is_empty(), "synchronous child cleanup");
        assert!(parent.x.get_geometry(child).unwrap().reply().is_err());
        assert_eq!(parent.state(parent.id), MapState::VIEWABLE);
        assert!(gui.show(&plugin.plugin_handle()).is_err());
        for _ in 0..8 {
            tick(&mut plugin);
        }
        assert!(parent.children().is_empty());
        assert_host_audio(&mut processor);
    }
    // A second factory-created instance in the SAME loaded DSO must bootstrap a distinct wire identity.
    // This makes no cross-DSO/process-global identity claim.
    {
        let second_parent = Parent::new();
        let mut second = instantiate(&entry);
        let mut second_audio = activate(&mut second);
        copied_app.observe(2);
        let second_gui = second.plugin_handle().get_extension::<PluginGui>().unwrap();
        second_gui
            .create(&second.plugin_handle(), embedded)
            .unwrap();
        unsafe {
            second_gui.set_parent(
                &second.plugin_handle(),
                Window::from_x11_handle(second_parent.id.into()),
            )
        }
        .unwrap();
        second_gui.show(&second.plugin_handle()).unwrap();
        let browser = weak_browser_in_child(second_parent.children()[0]);
        observe_ready_note(&mut second, &mut second_audio);
        second_gui.destroy(&second.plugin_handle());
        assert!(browser.upgrade().is_none());
        assert!(second_parent.children().is_empty());
        second.deactivate(
            std::thread::spawn(move || second_audio.stop_processing())
                .join()
                .unwrap(),
        );
        drop(second);
        assert_host_audio(&mut processor);
    }
    // A nondefault old scalar must not survive as the fresh instance's state.
    let mut values = EventBuffer::new();
    values.push(&ParamValueEvent::new(
        0,
        ClapId::new(3),
        Pckn::match_all(),
        0.5,
    ));
    block(&mut processor, &values, true);
    assert_eq!(
        plugin
            .plugin_handle()
            .get_extension::<PluginParams>()
            .unwrap()
            .get_value(&plugin.plugin_handle(), ClapId::new(3)),
        Some(0.5)
    );
    let webkit = webkit_path().expect("native plugin's actual linked WebKit mapping");
    log_mappings("native-before-release", &path);
    log_mappings("webkit-before-release", &webkit);
    plugin.deactivate(
        std::thread::spawn(move || processor.stop_processing())
            .join()
            .unwrap(),
    );
    drop(plugin);
    drop(entry);
    log_mappings("native-after-entry-release", &path);
    log_mappings("webkit-after-entry-release", &webkit);
    assert!(
        !file_mappings(&path).is_empty(),
        "G1: actual native initialization retains the executing plugin code after Entry release"
    );
    assert!(
        !file_mappings(&webkit).is_empty(),
        "G1: linked WebKit remains resident after Entry release"
    );
    dispatch_host_after_entry_release();
    // Persistent module statics, but a genuinely new instrument/session/runtime.
    let entry = copied_entry();
    let mut fresh = instantiate(&entry);
    assert_eq!(
        fresh
            .plugin_handle()
            .get_extension::<PluginParams>()
            .unwrap()
            .get_value(&fresh.plugin_handle(), ClapId::new(3)),
        Some(1.0)
    );
    let mut fresh_audio = activate(&mut fresh);
    assert_host_audio(&mut fresh_audio);
    assert_eq!(
        block(&mut fresh_audio, &EventBuffer::new(), true),
        [vec![0.0; 16], vec![0.0; 16]]
    );
    copied_app.observe(3);
    let fresh_gui = fresh.plugin_handle().get_extension::<PluginGui>().unwrap();
    fresh_gui.create(&fresh.plugin_handle(), embedded).unwrap();
    unsafe {
        fresh_gui.set_parent(
            &fresh.plugin_handle(),
            Window::from_x11_handle(parent.id.into()),
        )
    }
    .unwrap();
    fresh_gui.show(&fresh.plugin_handle()).unwrap();
    let browser = weak_browser_in_child(parent.children()[0]);
    observe_ready_note(&mut fresh, &mut fresh_audio);
    fresh_gui.destroy(&fresh.plugin_handle());
    assert!(browser.upgrade().is_none());
    assert!(parent.children().is_empty());
    fresh.deactivate(
        std::thread::spawn(move || fresh_audio.stop_processing())
            .join()
            .unwrap(),
    );
    drop(fresh);
    drop(entry);
    dispatch_host_after_entry_release();
    assert!(!file_mappings(&path).is_empty());
    assert!(!file_mappings(&webkit).is_empty());
    drop(copied_app);
    webkit
}

#[test]
#[ignore = "exact fresh process: no native initialization, ordinary loader release"]
fn copied_editor_scan_and_configuration_only_do_not_make_code_resident() {
    use clack_host::factory::plugin::PluginFactory;
    let path = artifact();
    let headless_baseline = std::env::var_os("DANDRUM_G1_EXPECT_HEADLESS_BASELINE").as_deref()
        == Some(std::ffi::OsStr::new("1"));
    eprintln!("G1_CONTROL plain_headless_baseline={headless_baseline}");
    assert!(file_mappings(&path).is_empty());
    assert!(webkit_path().is_none());
    {
        let entry = copied_entry();
        {
            let factory = entry.get_factory::<PluginFactory>().unwrap();
            assert_eq!(factory.plugin_count(), 3);
        }
        assert!(!file_mappings(&path).is_empty());
        drop(entry);
    }
    assert!(
        file_mappings(&path).is_empty(),
        "descriptor scan keeps normal unloading"
    );
    assert!(
        webkit_path().is_none(),
        "host has no independent WebKit reference"
    );
    for _ in 0..2 {
        let webkit = std::thread::spawn(|| {
            let path = artifact();
            let entry = copied_entry();
            let mut plugin = instantiate(&entry);
            let mut audio = activate(&mut plugin);
            assert_host_audio(&mut audio);
            let gui = plugin.plugin_handle().get_extension::<PluginGui>();
            let headless_baseline = std::env::var_os("DANDRUM_G1_EXPECT_HEADLESS_BASELINE")
                .as_deref()
                == Some(std::ffi::OsStr::new("1"));
            assert_eq!(
                gui.is_none(),
                headless_baseline,
                "explicit control must match actual artifact's GUI capability"
            );
            if let Some(gui) = gui {
                let configuration = GuiConfiguration {
                    api_type: GuiApiType::X11,
                    is_floating: false,
                };
                assert!(gui.is_api_supported(&plugin.plugin_handle(), configuration));
                gui.create(&plugin.plugin_handle(), configuration).unwrap();
                assert_eq!(
                    gui.get_size(&plugin.plugin_handle()),
                    Some(GuiSize {
                        width: 1200,
                        height: 800
                    })
                );
                let parent = Parent::new();
                let state_before = saved_state(&mut plugin);
                let app = path
                    .parent()
                    .unwrap()
                    .join("DandrumTrigger.resources/editor/app.js");
                let restore_app = CopiedApp {
                    original: std::fs::read(&app).unwrap(),
                    path: app.clone(),
                };
                std::fs::write(&app, b"").unwrap();
                let failure = unsafe {
                    gui.set_parent(
                        &plugin.plugin_handle(),
                        Window::from_x11_handle(parent.id.into()),
                    )
                };
                drop(restore_app);
                assert!(
                    failure.is_err(),
                    "cheap invalid assets reject native initialization"
                );
                assert!(parent.children().is_empty());
                assert_eq!(saved_state(&mut plugin), state_before);
                assert_host_audio(&mut audio);
                assert!(gui.show(&plugin.plugin_handle()).is_err());
                assert!(
                    unsafe { gtk::gdk::ffi::gdk_display_get_default() }.is_null(),
                    "cheap asset rejection precedes GTK and native residency admission"
                );
                gui.destroy(&plugin.plugin_handle());
            } else {
                eprintln!("G1_CONTROL plain baseline: GUI configuration calls excluded");
            }
            plugin.deactivate(
                std::thread::spawn(move || audio.stop_processing())
                    .join()
                    .unwrap(),
            );
            let webkit = webkit_path();
            assert_eq!(
                webkit.is_none(),
                headless_baseline,
                "baseline dependency presence is explicit"
            );
            drop(plugin);
            drop(entry);
            log_mappings("no-native-after-entry-release-on-owner-thread", &path);
            webkit
        })
        .join()
        .expect("owned no-GUI main thread completed");
        log_mappings("no-native-after-owner-thread-join", &path);
        rescan_already_loaded(&path);
        log_mappings("no-native-after-balanced-loader-rescan", &path);
        assert!(
            file_mappings(&path).is_empty(),
            "ordinary no-native loading releases after owner teardown and balanced rescan"
        );
        if let Some(webkit) = webkit {
            assert!(
                file_mappings(&webkit).is_empty(),
                "no-native initialization keeps dependencies unloadable"
            );
        }
    }
}

#[test]
#[ignore = "exact fresh process: real parent, mismatched executing loader identity and OS-byte Entry path"]
fn copied_editor_rejects_wrong_loaded_entry_identity_before_gtk_without_state_or_audio_loss() {
    let path = artifact();
    let decoy_path = std::thread::spawn(wrong_identity_on_host_thread)
        .join()
        .expect("owned identity host thread completed");
    rescan_already_loaded(&path);
    rescan_already_loaded(&decoy_path);
    log_mappings("identity-after-owner-join-and-rescan", &path);
    log_mappings("decoy-after-owner-join-and-rescan", &decoy_path);
    assert!(file_mappings(&path).is_empty());
    assert!(
        file_mappings(&decoy_path).is_empty(),
        "mismatched object must not be promoted before identity check"
    );
    std::fs::remove_file(decoy_path).unwrap();
}
fn wrong_identity_on_host_thread() -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    let path = artifact();
    let parent = Parent::new();
    assert!(file_mappings(&path).is_empty());
    assert!(webkit_path().is_none());
    // This is an actual different loaded object, with a non-UTF8 Entry path.
    // Both files share a valid sibling resource tree, so setup/session open succeeds.
    let decoy_path = path
        .parent()
        .unwrap()
        .join(std::ffi::OsStr::from_bytes(b"trigger-gui-decoy-\xff.clap"));
    std::fs::copy(&path, &decoy_path).unwrap();
    let decoy = unsafe { libloading::Library::new(&decoy_path) }.unwrap();
    let provider = unsafe { clack_host::entry::LibraryEntry::load_from_path(&path) }.unwrap();
    let declared = std::ffi::CString::new(decoy_path.as_os_str().as_bytes()).unwrap();
    let entry = unsafe { PluginEntry::load_from(provider, &declared) }.unwrap();
    let mut plugin = instantiate(&entry);
    let before = saved_state(&mut plugin);
    let mut audio = activate(&mut plugin);
    assert_host_audio(&mut audio);
    let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
    gui.create(
        &plugin.plugin_handle(),
        GuiConfiguration {
            api_type: GuiApiType::X11,
            is_floating: false,
        },
    )
    .unwrap();
    // Independent process-global GDK observation, not the per-Rust-copy gtk initialized flag.
    assert!(unsafe { gtk::gdk::ffi::gdk_display_get_default() }.is_null());
    let result = unsafe {
        gui.set_parent(
            &plugin.plugin_handle(),
            Window::from_x11_handle(parent.id.into()),
        )
    };
    gui.destroy(&plugin.plugin_handle());
    assert!(
        result.is_err(),
        "G1: native initialization must reject an Entry path naming a different loaded object"
    );
    assert!(
        unsafe { gtk::gdk::ffi::gdk_display_get_default() }.is_null(),
        "residency identity failure precedes GTK initialization"
    );
    assert!(parent.children().is_empty());
    assert_eq!(saved_state(&mut plugin), before);
    assert_host_audio(&mut audio);
    assert!(gui.show(&plugin.plugin_handle()).is_err());
    plugin.deactivate(
        std::thread::spawn(move || audio.stop_processing())
            .join()
            .unwrap(),
    );
    drop(plugin);
    drop(entry);
    drop(decoy);
    decoy_path
}

#[test]
#[ignore = "exact fresh process: first real failed native attempt retains code, never application state"]
fn copied_editor_failed_native_parent_attempt_retains_only_code_and_preserves_audio() {
    let path = artifact();
    let webkit = std::thread::spawn(|| {
        let path = artifact();
        assert!(file_mappings(&path).is_empty());
        assert!(webkit_path().is_none());
        let parent = Parent::new();
        let unused = parent.x.generate_id().unwrap();
        let failure = parent.x.get_window_attributes(unused).unwrap().reply();
        assert!(
            matches!(failure, Err(x11rb::errors::ReplyError::X11Error(ref error))
            if error.error_code == 3 && error.bad_value == unused)
        );
        let entry = copied_entry();
        let mut plugin = instantiate(&entry);
        let state = saved_state(&mut plugin);
        let mut audio = activate(&mut plugin);
        assert_host_audio(&mut audio);
        let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
        gui.create(
            &plugin.plugin_handle(),
            GuiConfiguration {
                api_type: GuiApiType::X11,
                is_floating: false,
            },
        )
        .unwrap();
        assert!(unsafe { gtk::gdk::ffi::gdk_display_get_default() }.is_null());
        assert!(
            unsafe {
                gui.set_parent(
                    &plugin.plugin_handle(),
                    Window::from_x11_handle(unused.into()),
                )
            }
            .is_err(),
            "real server rejects this representable but nonexistent parent"
        );
        assert!(
            !unsafe { gtk::gdk::ffi::gdk_display_get_default() }.is_null(),
            "the admitted native initialization reached GTK before server parent refusal"
        );
        assert!(parent.children().is_empty());
        assert_eq!(saved_state(&mut plugin), state);
        assert_host_audio(&mut audio);
        assert!(gui.show(&plugin.plugin_handle()).is_err());
        assert!(gui.hide(&plugin.plugin_handle()).is_err());
        gui.destroy(&plugin.plugin_handle());
        plugin.deactivate(
            std::thread::spawn(move || audio.stop_processing())
                .join()
                .unwrap(),
        );
        let webkit = webkit_path().expect("actual loaded native dependency");
        drop(plugin);
        drop(entry);
        dispatch_host_after_entry_release();
        webkit
    })
    .join()
    .expect("owned failed-native host thread completed");
    log_mappings("failed-native-after-owner-join", &path);
    rescan_already_loaded(&path);
    log_mappings("failed-native-after-balanced-rescan", &path);
    log_mappings("failed-native-webkit-after-balanced-rescan", &webkit);
    assert!(
        !file_mappings(&path).is_empty(),
        "first admitted native attempt establishes residency even when server parent validation fails"
    );
    assert!(!file_mappings(&webkit).is_empty());
}
