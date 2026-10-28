//! Inject a real WebKit IPC-registration collision through public GObject APIs.
use super::*;
use gtk::glib::{self, translate::*};
use std::cell::Cell;
use webkit2gtk::UserContentManagerExt;

#[derive(Default)]
struct Observation {
    matches: usize,
    manager: Option<webkit2gtk::UserContentManager>,
    browser: Option<glib::WeakRef<webkit2gtk::WebView>>,
    registered: bool,
    sentinel_pending: bool,
    baseline_pending: bool,
}

struct HookData {
    observation: Rc<RefCell<Observation>>,
    invalid: Rc<Cell<bool>>,
    messages: Rc<Cell<usize>>,
    destroyed: Rc<Cell<usize>>,
}

fn pending(manager: &webkit2gtk::UserContentManager) -> bool {
    // Detail zero includes generic handlers. The ipc detail misses them on the
    // installed GLib, so calibrate this exact query on the actual Wry manager.
    unsafe {
        let signal = glib::gobject_ffi::g_signal_lookup(
            c"script-message-received".as_ptr(),
            webkit2gtk::UserContentManager::static_type().into_glib(),
        );
        glib::gobject_ffi::g_signal_has_handler_pending(manager.as_ptr().cast(), signal, 0, 1) != 0
    }
}

unsafe extern "C" fn observe_parent(
    _: *mut glib::gobject_ffi::GSignalInvocationHint,
    count: u32,
    values: *const glib::gobject_ffi::GValue,
    data: glib::ffi::gpointer,
) -> glib::ffi::gboolean {
    // The stable box is owned by GObject until synchronous hook removal. No
    // assertion or unwind may cross this foreign callback boundary.
    let data = unsafe { &*(data as *const HookData) };
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if count < 2 || values.is_null() {
            data.invalid.set(true);
            return;
        }
        let object = unsafe { glib::gobject_ffi::g_value_get_object(values) };
        if object.is_null()
            || unsafe {
                glib::gobject_ffi::g_type_check_instance_is_a(
                    object.cast(),
                    webkit2gtk::WebView::static_type().into_glib(),
                )
            } == 0
        {
            return;
        }
        // Cleanup also emits parent-set. Match only insertion, never removal.
        if !unsafe { glib::gobject_ffi::g_value_get_object(values.add(1)) }.is_null()
            || unsafe { gtk::ffi::gtk_widget_get_parent(object.cast()) }.is_null()
        {
            return;
        }
        let Ok(mut observation) = data.observation.try_borrow_mut() else {
            data.invalid.set(true);
            return;
        };
        observation.matches += 1;
        if observation.matches != 1 {
            data.invalid.set(true);
            return;
        }
        let browser: webkit2gtk::WebView = unsafe { from_glib_none(object.cast()) };
        observation.browser = Some(browser.downgrade());
        let Some(manager) = browser.user_content_manager() else {
            data.invalid.set(true);
            return;
        };
        let messages = Rc::clone(&data.messages);
        let sentinel = manager.connect_script_message_received(None, move |_, _| {
            messages.set(messages.get().saturating_add(1));
        });
        observation.registered = manager.register_script_message_handler("ipc");
        observation.sentinel_pending = pending(&manager);
        manager.disconnect(sentinel);
        observation.baseline_pending = pending(&manager);
        observation.manager = Some(manager);
    }));
    if outcome.is_err() {
        data.invalid.set(true);
    }
    1 // Keep the hook until the guard removes it after set_parent returns.
}

unsafe extern "C" fn free_hook(data: glib::ffi::gpointer) {
    let data = unsafe { Box::from_raw(data as *mut HookData) };
    data.destroyed.set(data.destroyed.get().saturating_add(1));
}

struct ParentHook {
    signal: u32,
    id: std::ffi::c_ulong,
}
impl ParentHook {
    fn install(data: HookData) -> Self {
        let signal = unsafe {
            glib::gobject_ffi::g_signal_lookup(
                c"parent-set".as_ptr(),
                gtk::Widget::static_type().into_glib(),
            )
        };
        assert_ne!(signal, 0, "GTK parent-set signal exists");
        let data = Box::into_raw(Box::new(data));
        let id = unsafe {
            glib::gobject_ffi::g_signal_add_emission_hook(
                signal,
                0,
                Some(observe_parent),
                data.cast(),
                Some(free_hook),
            )
        };
        if id == 0 {
            // Registration did not transfer ownership to GObject.
            unsafe {
                drop(Box::from_raw(data));
            }
            panic!("GTK refused the parent-set emission hook");
        }
        Self { signal, id }
    }
}
impl Drop for ParentHook {
    fn drop(&mut self) {
        unsafe {
            glib::gobject_ffi::g_signal_remove_emission_hook(self.signal, self.id);
        }
    }
}

fn assert_retained(plugin: &mut PluginInstance<GuiHost>, saved: &[u8]) {
    assert_eq!(saved_state(plugin), saved);
    let params = plugin
        .plugin_handle()
        .get_extension::<PluginParams>()
        .unwrap();
    assert_eq!(
        params.get_value(&plugin.plugin_handle(), ClapId::new(3)),
        Some(2.0)
    );
    let mut audio = start_audio(plugin);
    std::thread::scope(|scope| scope.spawn(|| audio.reset()).join().unwrap());
    let mut events = EventBuffer::new();
    events.push(&MidiEvent::new(3, 0, [0x90, 36, 100]));
    let mut output = Recorder::new(64);
    let pcm = render_audio(&mut audio, &events, &mut output);
    return_audio(plugin, audio);
    for channel in pcm {
        assert_eq!(&channel[..3], &[0.0; 3]);
        // Original WAV signed PCM [-16384, -358] / 32768, times physical level 2.
        assert_eq!(&channel[3..5], &[-1.0, -716.0 / 32768.0]);
    }
    assert_eq!(output.attempts, 0);
    assert_eq!(saved_state(plugin), saved);
}

#[test]
#[ignore = "native creation failure: inherited X11 and explicitly copied release"]
fn copied_native_ipc_registration_failure_cleans_up_and_retries_preserving_state() {
    std::thread::spawn(creation_failure_scenario)
        .join()
        .expect("creation-failure native host completed");
}

fn creation_failure_scenario() {
    gtk::init().expect("inherited X11 GTK initialization");
    let parent = Parent::new();
    let mut plugin = copied_plugin();
    let params = plugin
        .plugin_handle()
        .get_extension::<PluginParams>()
        .unwrap();
    assert_eq!(params.count(&plugin.plugin_handle()), 23);
    let mut info = clack_extensions::params::ParamInfoBuffer::new();
    let level = params
        .get_info(&plugin.plugin_handle(), 2, &mut info)
        .unwrap();
    assert_eq!((level.id.get(), level.name), (3, b"drums.level".as_slice()));
    assert_eq!(
        (level.min_value, level.max_value, level.default_value),
        (0.0, 4.0, 1.0)
    );
    let mut input = EventBuffer::new();
    input.push(&ParamValueEvent::new(
        0,
        ClapId::new(3),
        Pckn::match_all(),
        2.0,
    ));
    let mut output = Recorder::new(64);
    params.flush(
        &mut plugin.inactive_plugin_handle().unwrap(),
        &input.as_input(),
        &mut OutputEvents::from_buffer(&mut output),
    );
    assert_eq!(output.attempts, 0);
    let saved = saved_state(&mut plugin);
    assert_retained(&mut plugin, &saved);
    let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
    gui.create(
        &plugin.plugin_handle(),
        GuiConfiguration {
            api_type: GuiApiType::X11,
            is_floating: false,
        },
    )
    .unwrap();
    assert!(parent.children().is_empty());
    let baseline_toplevels = gtk::Window::list_toplevels()
        .into_iter()
        .map(|window| window.downgrade())
        .collect::<Vec<_>>();
    let toplevels = baseline_toplevels.len();
    let observation = Rc::new(RefCell::new(Observation::default()));
    let invalid = Rc::new(Cell::new(false));
    let messages = Rc::new(Cell::new(0));
    let destroyed = Rc::new(Cell::new(0));
    let hook = ParentHook::install(HookData {
        observation: Rc::clone(&observation),
        invalid: Rc::clone(&invalid),
        messages: Rc::clone(&messages),
        destroyed: Rc::clone(&destroyed),
    });
    let rejected = unsafe {
        gui.set_parent(
            &plugin.plugin_handle(),
            Window::from_x11_handle(parent.id.into()),
        )
    };
    drop(hook); // Synchronous removal on the same UI thread, before any retry.
    assert_eq!(
        destroyed.get(),
        1,
        "GDestroyNotify frees hook userdata exactly once"
    );
    let observation = Rc::try_unwrap(observation).ok().unwrap().into_inner();
    eprintln!(
        "IPC fixture: matches={} registered={} sentinel_pending={} baseline_pending={} invalid={} rejected={}",
        observation.matches,
        observation.registered,
        observation.sentinel_pending,
        observation.baseline_pending,
        invalid.get(),
        rejected.is_err()
    );
    assert!(
        !invalid.get(),
        "fixture callback was reentrant, malformed or panicked"
    );
    assert_eq!(
        observation.matches, 1,
        "fixture must reach the real Wry manager once"
    );
    assert!(
        observation.registered,
        "fixture must win initial ipc registration"
    );
    assert!(
        observation.sentinel_pending,
        "same-manager positive callback calibration"
    );
    assert!(
        !observation.baseline_pending,
        "same-manager clean baseline before product connects"
    );
    assert!(
        rejected.is_err(),
        "duplicate IPC registration must reject public parenting"
    );
    assert!(
        !pending(observation.manager.as_ref().unwrap()),
        "failed creation releases its IPC callback"
    );
    assert!(
        observation.browser.as_ref().unwrap().upgrade().is_none(),
        "retained manager must not keep failed WebView alive"
    );
    assert_eq!(messages.get(), 0);
    assert!(parent.children().is_empty());
    assert_eq!(gtk::Window::list_toplevels().len(), toplevels);
    drop(observation);
    assert_retained(&mut plugin, &saved);

    unsafe {
        gui.set_parent(
            &plugin.plugin_handle(),
            Window::from_x11_handle(parent.id.into()),
        )
    }
    .unwrap();
    gui.show(&plugin.plugin_handle()).unwrap();
    let children = parent.children();
    assert_eq!(children.len(), 1);
    let geometry = parent.x.get_geometry(children[0]).unwrap().reply().unwrap();
    assert_eq!(
        (geometry.x, geometry.y, geometry.width, geometry.height),
        (0, 0, 1200, 800)
    );
    let weak = weak_browser_in_child(children[0]);
    let browser = weak
        .upgrade()
        .unwrap()
        .downcast::<webkit2gtk::WebView>()
        .unwrap();
    wait_dom(
        &mut plugin,
        &browser,
        "document.querySelector('[data-testid=trigger-editor]')",
    );
    let snapshot = invoke(
        &mut plugin,
        &browser,
        "window.__DANDRUM__.backend.getNativeFunction('getSamplerSession')()",
    );
    assert_eq!(snapshot["status"], "accepted");
    assert_eq!(snapshot["result"]["connection"]["revision"], "0");
    assert_eq!(
        snapshot["result"]["document"]["instrumentId"],
        "dandrum.advanced-drum-kit"
    );
    assert_eq!(
        snapshot["result"]["connection"]["history"],
        serde_json::json!({"undoCount":0,"redoCount":0})
    );
    let parameters = invoke(
        &mut plugin,
        &browser,
        "window.__DANDRUM__.backend.getNativeFunction('getParameterState')()",
    );
    assert_eq!(parameters["status"], "accepted");
    let values = parameters["result"]["parameters"].as_array().unwrap();
    assert_eq!(
        values.iter().find(|p| p["id"] == "drums.level").unwrap()["value"],
        0.5
    );
    assert_retained(&mut plugin, &saved);
    let owner = browser.toplevel().expect("native editor has a GTK owner");
    let owner_weak = owner.downgrade();
    let owner_destroyed = Rc::new(Cell::new(0_usize));
    let counter = Rc::clone(&owner_destroyed);
    owner.connect_destroy(move |_| counter.set(counter.get() + 1));
    let child = children[0];
    drop(owner);
    drop(browser);
    gui.destroy(&plugin.plugin_handle());
    assert!(weak.upgrade().is_none());
    assert!(
        owner_weak.upgrade().is_none(),
        "editor container is finalized"
    );
    assert_eq!(
        owner_destroyed.get(),
        1,
        "editor container is destroyed once"
    );
    assert!(parent.children().is_empty());
    assert!(
        matches!(
            parent.x.get_window_attributes(child).unwrap().reply(),
            Err(x11rb::errors::ReplyError::X11Error(error))
                if error.error_kind == x11rb::protocol::ErrorKind::Window
        ),
        "the editor's original X11 window no longer exists"
    );
    // GTK lazily creates a display-owned tooltip window during the loaded UI.
    // It survives individual views. Reject every other new surviving toplevel
    // while the exact editor owner above must be both destroyed and finalized.
    for window in gtk::Window::list_toplevels() {
        assert!(
            baseline_toplevels
                .iter()
                .any(|weak| weak.upgrade().is_some_and(|old| old == window))
                || window.type_().name() == "GtkTooltipWindow",
            "unexpected surviving toplevel: {} ({})",
            window.type_().name(),
            window.widget_name()
        );
    }
    assert_retained(&mut plugin, &saved);
}
