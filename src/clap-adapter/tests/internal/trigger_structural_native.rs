use super::*;

fn open_view(plugin: &mut PluginInstance<GuiHost>, parent: &Parent) -> webkit2gtk::WebView {
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
        plugin,
        &browser,
        "document.querySelector('[data-testid=trigger-editor]')",
    );
    browser
}
fn snapshot(
    plugin: &mut PluginInstance<GuiHost>,
    browser: &webkit2gtk::WebView,
) -> serde_json::Value {
    let reply = invoke(
        plugin,
        browser,
        "window.__DANDRUM__.backend.getNativeFunction('getSamplerSession')()",
    );
    assert_eq!(reply["status"], "accepted");
    reply["result"].clone()
}
fn note(audio: &mut StartedPluginAudioProcessor<GuiHost>, key: u8) -> [f32; 2] {
    std::thread::scope(|scope| scope.spawn(|| audio.reset()).join().unwrap());
    let mut events = EventBuffer::new();
    events.push(&MidiEvent::new(0, 0, [0x90, key, 100]));
    let pcm = render_audio(audio, &events, &mut Recorder::new(64));
    [pcm[0][0], pcm[1][0]]
}
#[test]
#[ignore = "native structural boundary: inherited X11 and explicitly copied release"]
fn copied_native_mapping_survives_lost_reply_and_view_loss() {
    std::thread::spawn(native_structural_scenario)
        .join()
        .expect("native structural host completed");
}

fn native_structural_scenario() {
    let parent = Parent::new();
    let mut plugin = copied_plugin();
    let browser = open_view(&mut plugin, &parent);
    let initial = snapshot(&mut plugin, &browser);
    let saved = saved_state(&mut plugin);
    let mut audio = start_audio(&mut plugin);
    assert_eq!(note(&mut audio, 36), [-0.5, -0.5]);
    assert_eq!(note(&mut audio, 60), [0.0, 0.0]);
    let state = plugin
        .plugin_handle()
        .get_extension::<clack_extensions::state::PluginState>()
        .unwrap();
    state
        .load(&plugin.plugin_handle(), &mut saved.as_slice())
        .unwrap();
    let restored = snapshot(&mut plugin, &browser);
    assert_eq!(restored["connection"]["revision"], "1");
    assert_eq!(
        restored["connection"]["history"],
        serde_json::json!({"undoCount":0,"redoCount":0})
    );
    // The active audio endpoint has not consumed this restored runtime.
    // Its capacity-one pending slot prevents any mapping publication until
    // an actual block below, independently of helper GLib dispatch.

    // Observe the real reply but deliberately withhold it from the broker.
    // The pending Promise is discarded with this actual WebView.
    javascript(
        &mut plugin,
        &browser,
        r#"(()=>{window.__G3_REPLY__=null;const surface=window.__DANDRUM__,receive=surface.receiveNativeMessage;surface.receiveNativeMessage=message=>{const frame=JSON.parse(message);if(frame.result?.job_id==='7001'||frame.error){window.__G3_REPLY__=frame;return 'withheld';}return receive(message);};const c=window.__DANDRUM_NATIVE__;surface.backend.getNativeFunction('setInstrumentZoneKeyRange')(c.instanceId,c.attachmentId,'7001','1','kit','kick',60,60);return 'sent';})()"#,
    );
    let deadline = Instant::now() + Duration::from_secs(15);
    let admission = loop {
        let value = javascript(&mut plugin, &browser, "JSON.stringify(window.__G3_REPLY__)");
        if value != "null" {
            break serde_json::from_str::<serde_json::Value>(&value).unwrap();
        }
        assert!(
            Instant::now() < deadline,
            "native mapping admission reply absent"
        );
    };
    let before_destroy = snapshot(&mut plugin, &browser);
    assert_eq!(
        before_destroy["connection"]["revision"], "1",
        "mapping cannot publish while real audio owns an unconsumed restore"
    );
    assert_eq!(
        before_destroy["connection"]["history"],
        serde_json::json!({"undoCount":0,"redoCount":0})
    );
    let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
    gui.destroy(&plugin.plugin_handle());
    drop(browser);
    assert!(
        parent.children().is_empty(),
        "mapping continues after complete native child destruction"
    );
    if admission["status"] != "accepted" {
        return_audio(&mut plugin, audio);
        assert_eq!(
            admission["status"], "accepted",
            "G3 mapping must reach persistent native worker: {admission}"
        );
        unreachable!();
    }
    assert_eq!(admission["result"]["job_id"], "7001");
    // Only the real host GLib context is dispatched: no CLAP timer or main
    // callback, no browser, and no plugin-private service/testing entrypoint.
    let context = gtk::glib::MainContext::default();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        for _ in 0..64 {
            if !context.pending() {
                break;
            }
            context.iteration(false);
        }
        if note(&mut audio, 60) == [-0.5, -0.5] {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "view-independent GLib service did not publish mapping"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(note(&mut audio, 36), [0.0, 0.0]);
    assert_eq!(note(&mut audio, 60), [-0.5, -0.5]);
    return_audio(&mut plugin, audio);
    let browser = open_view(&mut plugin, &parent);
    let current = snapshot(&mut plugin, &browser);
    assert_eq!(
        current["connection"]["instanceId"],
        initial["connection"]["instanceId"]
    );
    assert_ne!(
        current["connection"]["attachmentId"],
        initial["connection"]["attachmentId"]
    );
    assert_eq!(current["connection"]["revision"], "2");
    assert_eq!(
        current["connection"]["history"],
        serde_json::json!({"undoCount":1,"redoCount":0})
    );
    javascript(
        &mut plugin,
        &browser,
        "(()=>{window.__G3_CALLS__=[];const backend=window.__DANDRUM__.backend,get=backend.getNativeFunction;backend.getNativeFunction=method=>(...args)=>{window.__G3_CALLS__.push(method);return get(method)(...args);};return 'observing';})()",
    );
    for _ in 0..2 {
        let recovery = invoke(
            &mut plugin,
            &browser,
            "window.__DANDRUM__.backend.getNativeFunction('getInstrumentSessionJobStatus')('7001')",
        );
        if recovery["status"] != "accepted" {
            gui.destroy(&plugin.plugin_handle());
            drop(browser);
            assert_eq!(
                recovery["status"], "accepted",
                "G3 fresh authenticated editor must recover retained native outcome: {recovery}"
            );
            unreachable!();
        }
        assert_eq!(
            recovery["result"],
            serde_json::json!({"request_id":"7001","state":"completed","revision":"2","undoCount":1,"redoCount":0,"error":""})
        );
    }
    assert_eq!(
        invoke(
            &mut plugin,
            &browser,
            "window.__DANDRUM__.backend.getNativeFunction('getInstrumentSessionJobStatus')('7002')"
        )["code"],
        "commandRejected"
    );
    assert_eq!(
        javascript(
            &mut plugin,
            &browser,
            "String(window.__G3_CALLS__.filter(method=>method==='getInstrumentSessionJobStatus').length)"
        ),
        "3",
        "calibrate the observer with the actual status calls"
    );
    assert_eq!(
        javascript(
            &mut plugin,
            &browser,
            "String(window.__G3_CALLS__.filter(method=>method==='setInstrumentZoneKeyRange').length)"
        ),
        "0",
        "recovery never redispatches through this observed backend getter"
    );
    let stale = invoke(
        &mut plugin,
        &browser,
        "(()=>{const c=window.__DANDRUM_NATIVE__;return window.__DANDRUM__.backend.getNativeFunction('setInstrumentZoneKeyRange')(c.instanceId,c.attachmentId,'7999','0','kit','kick',61,61);})()",
    );
    assert_eq!(stale["code"], "staleRequest");
    assert_eq!(
        javascript(
            &mut plugin,
            &browser,
            "String(window.__G3_CALLS__.filter(method=>method==='setInstrumentZoneKeyRange').length)"
        ),
        "1",
        "known rejected mapping calibrates this path-limited observer"
    );
    assert_eq!(
        snapshot(&mut plugin, &browser)["connection"]["history"],
        serde_json::json!({"undoCount":1,"redoCount":0})
    );
    let mut audio = start_audio(&mut plugin);
    assert_eq!(note(&mut audio, 36), [0.0, 0.0]);
    assert_eq!(note(&mut audio, 60), [-0.5, -0.5]);
    return_audio(&mut plugin, audio);
    wait_dom(
        &mut plugin,
        &browser,
        "document.querySelector('header button[aria-label=\"Undo Change Key Range (Ctrl+Z)\"]:not(:disabled)')",
    );
    javascript(
        &mut plugin,
        &browser,
        r#"(()=>{window.__G3_UNDO__=null;window.__G3_UNDO_ARGS__=[];const backend=window.__DANDRUM__.backend,get=backend.getNativeFunction;backend.getNativeFunction=method=>(...args)=>{const promise=get(method)(...args);if(method==='undoInstrumentSession'){window.__G3_UNDO_ARGS__.push(args);promise.then(result=>window.__G3_UNDO__={status:'accepted',result},error=>window.__G3_UNDO__={status:'rejected',code:error.code});}return promise;};document.querySelector('header button[aria-label="Undo Change Key Range (Ctrl+Z)"]').click();return 'clicked';})()"#,
    );
    let deadline = Instant::now() + Duration::from_secs(15);
    let undo = loop {
        let value = javascript(&mut plugin, &browser, "JSON.stringify(window.__G3_UNDO__)");
        if value != "null" {
            break serde_json::from_str::<serde_json::Value>(&value).unwrap();
        }
        assert!(
            Instant::now() < deadline,
            "production header did not invoke native structural Undo"
        );
    };
    if undo["status"] != "accepted" {
        gui.destroy(&plugin.plugin_handle());
        drop(browser);
        assert_eq!(
            undo["status"], "accepted",
            "G3 production header must reach native structural Undo: {undo}"
        );
        unreachable!();
    }
    assert_eq!(
        undo["result"],
        serde_json::json!({"status":"accepted","job_id":"7002"})
    );
    wait_dom(
        &mut plugin,
        &browser,
        "document.querySelector('header button[aria-label=\"Nothing to undo\"]:disabled')",
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&javascript(
            &mut plugin,
            &browser,
            "JSON.stringify(window.__G3_UNDO_ARGS__)"
        ))
        .unwrap(),
        serde_json::json!([[
            current["connection"]["instanceId"],
            current["connection"]["attachmentId"],
            "7002",
            "2"
        ]])
    );
    for _ in 0..2 {
        let outcome = invoke(
            &mut plugin,
            &browser,
            "window.__DANDRUM__.backend.getNativeFunction('getInstrumentSessionJobStatus')('7002')",
        );
        assert_eq!(
            outcome["result"],
            serde_json::json!({"request_id":"7002","state":"completed","revision":"3","undoCount":0,"redoCount":1,"error":""})
        );
    }
    let undone = snapshot(&mut plugin, &browser);
    assert_eq!(undone["connection"]["revision"], "3");
    assert_eq!(
        undone["connection"]["history"],
        serde_json::json!({"undoCount":0,"redoCount":1})
    );
    let mut audio = start_audio(&mut plugin);
    assert_eq!(note(&mut audio, 36), [-0.5, -0.5]);
    assert_eq!(note(&mut audio, 60), [0.0, 0.0]);
    return_audio(&mut plugin, audio);
    gui.destroy(&plugin.plugin_handle());
    drop(browser);
    assert!(parent.children().is_empty());
}
