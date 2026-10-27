use super::*;

pub(super) fn open_view(
    plugin: &mut PluginInstance<GuiHost>,
    parent: &Parent,
) -> webkit2gtk::WebView {
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
pub(super) fn snapshot(
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
fn feedback(
    plugin: &mut PluginInstance<GuiHost>,
    browser: &webkit2gtk::WebView,
) -> serde_json::Value {
    let reply = invoke(
        plugin,
        browser,
        "window.__DANDRUM__.backend.getNativeFunction('getParameterState')()",
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
struct SecondInstanceBaseline {
    snapshot: serde_json::Value,
    feedback: serde_json::Value,
    saved: Vec<u8>,
    child: u32,
    weak_browser: gtk::glib::WeakRef<gtk::glib::Object>,
    identity: (String, String),
}
impl SecondInstanceBaseline {
    fn capture(
        plugin: &mut PluginInstance<GuiHost>,
        parent: &Parent,
        browser: &webkit2gtk::WebView,
    ) -> Self {
        let snapshot = snapshot(plugin, browser);
        let child = parent.children()[0];
        Self {
            identity: (
                snapshot["connection"]["instanceId"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
                snapshot["connection"]["attachmentId"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            ),
            feedback: feedback(plugin, browser),
            saved: saved_state(plugin),
            weak_browser: weak_browser_in_child(child),
            child,
            snapshot,
        }
    }

    fn check(
        &self,
        plugin: &mut PluginInstance<GuiHost>,
        parent: &Parent,
        browser: &webkit2gtk::WebView,
    ) {
        let current = snapshot(plugin, browser);
        assert_eq!(
            current, self.snapshot,
            "instance B retains its exact session"
        );
        assert_eq!(current["connection"]["instanceId"], self.identity.0);
        assert_eq!(current["connection"]["attachmentId"], self.identity.1);
        assert_eq!(
            feedback(plugin, browser),
            self.feedback,
            "instance B retains its full parameter feedback DTO"
        );
        assert_eq!(
            saved_state(plugin),
            self.saved,
            "instance B retains exact saved bytes"
        );
        assert_eq!(
            parent.children(),
            vec![self.child],
            "instance B view remains attached"
        );
        assert!(
            self.weak_browser.upgrade().is_some(),
            "instance B WebView remains live"
        );

        let mut audio = start_audio(plugin);
        let probes = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_eq!(
                note(&mut audio, 36),
                [-0.5, -0.5],
                "instance B default MIDI 36"
            );
            assert_eq!(
                note(&mut audio, 60),
                [0.0, 0.0],
                "instance B MIDI 60 stays silent"
            );
        }));
        return_audio(plugin, audio);
        if let Err(payload) = probes {
            std::panic::resume_unwind(payload);
        }
    }
}
#[test]
#[ignore = "native structural boundary: inherited X11 and explicitly copied release"]
fn copied_native_mapping_survives_lost_reply_and_view_loss() {
    std::thread::spawn(native_structural_scenario)
        .join()
        .expect("native structural host completed");
}

fn native_structural_scenario() {
    let entry = copied_entry();
    let parent = Parent::new();
    let parent_b = Parent::new();
    let mut plugin = instantiate(&entry);
    let mut plugin_b = instantiate(&entry);
    let mut browser_a = None;
    let mut browser_b = None;
    let mut audio_a = None;
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        browser_a = Some(open_view(&mut plugin, &parent));
        browser_b = Some(open_view(&mut plugin_b, &parent_b));
        let initial = snapshot(&mut plugin, browser_a.as_ref().unwrap());
        let baseline_b =
            SecondInstanceBaseline::capture(&mut plugin_b, &parent_b, browser_b.as_ref().unwrap());
        let initial_b = &baseline_b.snapshot;
        assert_eq!(initial_b["document"]["maps"][0]["zones"][0]["keyLow"], 36);
        assert_eq!(initial_b["document"]["maps"][0]["zones"][0]["keyHigh"], 36);
        assert_eq!(
            initial_b["connection"]["history"],
            serde_json::json!({"undoCount":0,"redoCount":0})
        );
        assert_eq!(
            baseline_b.feedback["parameters"]
                .as_array()
                .unwrap()
                .iter()
                .find(|parameter| parameter["id"] == "drums.level")
                .unwrap(),
            &serde_json::json!({"id":"drums.level","name":"drums.level","value":0.25})
        );
        assert_ne!(
            (
                initial["connection"]["instanceId"].as_str().unwrap(),
                initial["connection"]["attachmentId"].as_str().unwrap()
            ),
            (
                baseline_b.identity.0.as_str(),
                baseline_b.identity.1.as_str()
            ),
            "instance authorities are distinct as composite tuples"
        );
        baseline_b.check(&mut plugin_b, &parent_b, browser_b.as_ref().unwrap());
        let saved = saved_state(&mut plugin);
        audio_a = Some(start_audio(&mut plugin));
        assert_eq!(note(audio_a.as_mut().unwrap(), 36), [-0.5, -0.5]);
        assert_eq!(note(audio_a.as_mut().unwrap(), 60), [0.0, 0.0]);
        let state = plugin
            .plugin_handle()
            .get_extension::<clack_extensions::state::PluginState>()
            .unwrap();
        state
            .load(&plugin.plugin_handle(), &mut saved.as_slice())
            .unwrap();
        let restored = snapshot(&mut plugin, browser_a.as_ref().unwrap());
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
            browser_a.as_ref().unwrap(),
            r#"(()=>{window.__G3_REPLY__=null;const surface=window.__DANDRUM__,receive=surface.receiveNativeMessage;surface.receiveNativeMessage=message=>{const frame=JSON.parse(message);if(frame.result?.job_id==='7001'||frame.error){window.__G3_REPLY__=frame;return 'withheld';}return receive(message);};const c=window.__DANDRUM_NATIVE__;surface.backend.getNativeFunction('setInstrumentZoneKeyRange')(c.instanceId,c.attachmentId,'7001','1','kit','kick',60,60);return 'sent';})()"#,
        );
        let deadline = Instant::now() + Duration::from_secs(15);
        let admission = loop {
            let value = javascript(
                &mut plugin,
                browser_a.as_ref().unwrap(),
                "JSON.stringify(window.__G3_REPLY__)",
            );
            if value != "null" {
                break serde_json::from_str::<serde_json::Value>(&value).unwrap();
            }
            assert!(
                Instant::now() < deadline,
                "native mapping admission reply absent"
            );
        };
        let before_destroy = snapshot(&mut plugin, browser_a.as_ref().unwrap());
        assert_eq!(
            before_destroy["connection"]["revision"], "1",
            "mapping cannot publish while real audio owns an unconsumed restore"
        );
        assert_eq!(
            before_destroy["connection"]["history"],
            serde_json::json!({"undoCount":0,"redoCount":0})
        );
        baseline_b.check(&mut plugin_b, &parent_b, browser_b.as_ref().unwrap());
        let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
        gui.destroy(&plugin.plugin_handle());
        drop(browser_a.take());
        assert!(
            parent.children().is_empty(),
            "mapping continues after complete native child destruction"
        );
        baseline_b.check(&mut plugin_b, &parent_b, browser_b.as_ref().unwrap());
        if admission["status"] != "accepted" {
            return_audio(&mut plugin, audio_a.take().unwrap());
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
            if note(audio_a.as_mut().unwrap(), 60) == [-0.5, -0.5] {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "view-independent GLib service did not publish mapping"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(note(audio_a.as_mut().unwrap(), 36), [0.0, 0.0]);
        assert_eq!(note(audio_a.as_mut().unwrap(), 60), [-0.5, -0.5]);
        return_audio(&mut plugin, audio_a.take().unwrap());
        baseline_b.check(&mut plugin_b, &parent_b, browser_b.as_ref().unwrap());
        browser_a = Some(open_view(&mut plugin, &parent));
        let current = snapshot(&mut plugin, browser_a.as_ref().unwrap());
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
        baseline_b.check(&mut plugin_b, &parent_b, browser_b.as_ref().unwrap());
        javascript(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            "(()=>{window.__G3_CALLS__=[];const backend=window.__DANDRUM__.backend,get=backend.getNativeFunction;backend.getNativeFunction=method=>(...args)=>{window.__G3_CALLS__.push(method);return get(method)(...args);};return 'observing';})()",
        );
        for _ in 0..2 {
            let recovery = invoke(
                &mut plugin,
                browser_a.as_ref().unwrap(),
                "window.__DANDRUM__.backend.getNativeFunction('getInstrumentSessionJobStatus')('7001')",
            );
            if recovery["status"] != "accepted" {
                gui.destroy(&plugin.plugin_handle());
                drop(browser_a.take());
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
                browser_a.as_ref().unwrap(),
                "window.__DANDRUM__.backend.getNativeFunction('getInstrumentSessionJobStatus')('7002')"
            )["code"],
            "commandRejected"
        );
        assert_eq!(
            javascript(
                &mut plugin,
                browser_a.as_ref().unwrap(),
                "String(window.__G3_CALLS__.filter(method=>method==='getInstrumentSessionJobStatus').length)"
            ),
            "3",
            "calibrate the observer with the actual status calls"
        );
        assert_eq!(
            javascript(
                &mut plugin,
                browser_a.as_ref().unwrap(),
                "String(window.__G3_CALLS__.filter(method=>method==='setInstrumentZoneKeyRange').length)"
            ),
            "0",
            "recovery never redispatches through this observed backend getter"
        );
        let stale = invoke(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            "(()=>{const c=window.__DANDRUM_NATIVE__;return window.__DANDRUM__.backend.getNativeFunction('setInstrumentZoneKeyRange')(c.instanceId,c.attachmentId,'7999','0','kit','kick',61,61);})()",
        );
        assert_eq!(stale["code"], "staleRequest");
        assert_eq!(
            javascript(
                &mut plugin,
                browser_a.as_ref().unwrap(),
                "String(window.__G3_CALLS__.filter(method=>method==='setInstrumentZoneKeyRange').length)"
            ),
            "1",
            "known rejected mapping calibrates this path-limited observer"
        );
        assert_eq!(
            snapshot(&mut plugin, browser_a.as_ref().unwrap())["connection"]["history"],
            serde_json::json!({"undoCount":1,"redoCount":0})
        );
        baseline_b.check(&mut plugin_b, &parent_b, browser_b.as_ref().unwrap());
        audio_a = Some(start_audio(&mut plugin));
        assert_eq!(note(audio_a.as_mut().unwrap(), 36), [0.0, 0.0]);
        assert_eq!(note(audio_a.as_mut().unwrap(), 60), [-0.5, -0.5]);
        return_audio(&mut plugin, audio_a.take().unwrap());
        wait_dom(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            "document.querySelector('header button[aria-label=\"Undo Change Key Range (Ctrl+Z)\"]:not(:disabled)')",
        );
        javascript(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            r#"(()=>{window.__G3_UNDO__=null;window.__G3_UNDO_ARGS__=[];const backend=window.__DANDRUM__.backend,get=backend.getNativeFunction;backend.getNativeFunction=method=>(...args)=>{const promise=get(method)(...args);if(method==='undoInstrumentSession'){window.__G3_UNDO_ARGS__.push(args);promise.then(result=>window.__G3_UNDO__={status:'accepted',result},error=>window.__G3_UNDO__={status:'rejected',code:error.code});}return promise;};document.querySelector('header button[aria-label="Undo Change Key Range (Ctrl+Z)"]').click();return 'clicked';})()"#,
        );
        let deadline = Instant::now() + Duration::from_secs(15);
        let undo = loop {
            let value = javascript(
                &mut plugin,
                browser_a.as_ref().unwrap(),
                "JSON.stringify(window.__G3_UNDO__)",
            );
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
            drop(browser_a.take());
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
            browser_a.as_ref().unwrap(),
            "document.querySelector('header button[aria-label=\"Nothing to undo\"]:disabled')",
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&javascript(
                &mut plugin,
                browser_a.as_ref().unwrap(),
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
                browser_a.as_ref().unwrap(),
                "window.__DANDRUM__.backend.getNativeFunction('getInstrumentSessionJobStatus')('7002')",
            );
            assert_eq!(
                outcome["result"],
                serde_json::json!({"request_id":"7002","state":"completed","revision":"3","undoCount":0,"redoCount":1,"error":""})
            );
        }
        let undone = snapshot(&mut plugin, browser_a.as_ref().unwrap());
        assert_eq!(undone["connection"]["revision"], "3");
        assert_eq!(
            undone["connection"]["history"],
            serde_json::json!({"undoCount":0,"redoCount":1})
        );
        baseline_b.check(&mut plugin_b, &parent_b, browser_b.as_ref().unwrap());
        // Exercise real Wry IPC without presenting the legacy frontend as a
        // v2 consumer. A calibrated unsupported call advances the broker while
        // leaving its transport ID unadmitted; the raw frame uses that ID.
        // Subsequent ordinary broker calls therefore remain aligned.
        let negotiation = invoke(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            "window.__DANDRUM__.backend.getNativeFunction('negotiateEditorProtocol')(2)",
        );
        assert_eq!(negotiation["status"], "accepted", "{negotiation}");
        assert_eq!(negotiation["result"]["wireVersion"], 2);
        let before_patch = undone["yaml"].as_str().unwrap();
        let regions = (0..256)
            .map(|index| {
                format!(
                    "        - {{ id: native_{index}, start_frame: {}, end_frame: {} }}\n",
                    index * 32,
                    index * 32 + 16
                )
            })
            .collect::<String>();
        // The current mapping document has been canonically serialized; use
        // the embedded package fixture for the authored region-list layout.
        let candidate = include_str!("../../../../examples/patches/advanced-drum-kit.yaml")
            .replacen("      regions:\n", &format!("      regions:\n{regions}"), 1)
            .replace("key_range: [36, 36]", "key_range: [60, 60]");
        assert!(candidate.len() > 16_384 && candidate.len() < 65_536);
        assert!(
            candidate.contains("id: native_255"),
            "fixture contains the additional real regions"
        );
        dandrum_engine::kernel::document::load_kernel_patch_str(&candidate)
            .expect("meaningful bounded sample-region document");
        javascript(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            r#"(()=>{window.__G5_PATCH_REPLY__=null;window.__G5_PATCH_SLOT__=null;
              const surface=window.__DANDRUM__,receive=surface.receiveNativeMessage;
              surface.receiveNativeMessage=message=>{const frame=JSON.parse(message);
                if(frame.version===2){window.__G5_PATCH_REPLY__=frame;return 'observed';}
                if(frame.error?.code==='capabilityUnavailable')window.__G5_PATCH_SLOT__=frame.requestId;
                return receive(message);};return 'observing';})()"#,
        );
        let unavailable = invoke(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            "window.__DANDRUM__.backend.getNativeFunction('requestSourceWaveform')('drums',0,8,4)",
        );
        assert_eq!(unavailable["code"], "capabilityUnavailable");
        let slot = javascript(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            "String(window.__G5_PATCH_SLOT__)",
        );
        assert!(
            slot.parse::<u64>().is_ok_and(|id| id != 0),
            "bounded unadmitted transport identity: {slot}"
        );
        let script = format!(
            r#"(()=>{{const c=window.__DANDRUM_NATIVE__;
          const frame={{version:2,attachmentId:c.attachmentId,requestId:window.__G5_PATCH_SLOT__,
            method:'prepareInstrumentSessionPatch',args:[c.instanceId,c.attachmentId,'7003','3',{}]}};
          const wire=JSON.stringify(frame);window.__G5_PATCH_BYTES__=new TextEncoder().encode(wire).length;
          window.ipc.postMessage(wire);return 'sent';}})()"#,
            serde_json::to_string(&candidate).unwrap()
        );
        javascript(&mut plugin, browser_a.as_ref().unwrap(), &script);
        let bytes = javascript(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            "String(window.__G5_PATCH_BYTES__)",
        )
        .parse::<usize>()
        .unwrap();
        assert!(
            bytes > 16_384 && bytes <= 524_288,
            "actual UTF-8 wire bytes: {bytes}"
        );
        eprintln!("NATIVE_PATCH negotiated v2, {bytes} wire bytes, prior signed PCM [-0.5,-0.5]");
        let deadline = Instant::now() + Duration::from_secs(5);
        let admitted = loop {
            let raw = javascript(
                &mut plugin,
                browser_a.as_ref().unwrap(),
                "JSON.stringify(window.__G5_PATCH_REPLY__)",
            );
            if raw != "null" {
                break serde_json::from_str::<serde_json::Value>(&raw).unwrap();
            }
            assert!(
                Instant::now() < deadline,
                "negotiated >16KiB patch must reach the actual Wry/Core ingress"
            );
        };
        assert_eq!(admitted["version"], 2);
        assert_eq!(admitted["status"], "accepted", "{admitted}");
        assert_eq!(admitted["result"]["job_id"], "7003");
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let status = invoke(
                &mut plugin,
                browser_a.as_ref().unwrap(),
                "window.__DANDRUM__.backend.getNativeFunction('getInstrumentSessionJobStatus')('7003')",
            );
            assert_eq!(status["status"], "accepted", "{status}");
            match status["result"]["state"].as_str() {
                Some("completed") => {
                    assert_eq!(status["result"]["revision"], "4");
                    break;
                }
                Some("running") => {
                    assert!(Instant::now() < deadline, "bounded native patch worker")
                }
                _ => panic!("native patch failed: {status}"),
            }
        }
        let saved: serde_json::Value = serde_json::from_slice(&saved_state(&mut plugin)).unwrap();
        assert_eq!(
            saved["document"], candidate,
            "actual CLAP state owns the complete published document"
        );
        audio_a = Some(start_audio(&mut plugin));
        assert_eq!(note(audio_a.as_mut().unwrap(), 60), [-0.5, -0.5]);
        assert_eq!(note(audio_a.as_mut().unwrap(), 36), [0.0, 0.0]);
        return_audio(&mut plugin, audio_a.take().unwrap());
        let undo = invoke(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            "(()=>{const c=window.__DANDRUM_NATIVE__;return window.__DANDRUM__.backend.getNativeFunction('undoInstrumentSession')(c.instanceId,c.attachmentId,'7004','4');})()",
        );
        assert_eq!(undo["status"], "accepted", "{undo}");
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let status = invoke(
                &mut plugin,
                browser_a.as_ref().unwrap(),
                "window.__DANDRUM__.backend.getNativeFunction('getInstrumentSessionJobStatus')('7004')",
            );
            assert_eq!(status["status"], "accepted", "{status}");
            match status["result"]["state"].as_str() {
                Some("completed") => {
                    assert_eq!(status["result"]["revision"], "5");
                    break;
                }
                Some("running") => {
                    assert!(Instant::now() < deadline, "bounded native document Undo")
                }
                _ => panic!("native patch Undo failed: {status}"),
            }
        }
        let saved: serde_json::Value = serde_json::from_slice(&saved_state(&mut plugin)).unwrap();
        assert_eq!(saved["document"], before_patch);
        baseline_b.check(&mut plugin_b, &parent_b, browser_b.as_ref().unwrap());
        audio_a = Some(start_audio(&mut plugin));
        assert_eq!(note(audio_a.as_mut().unwrap(), 36), [-0.5, -0.5]);
        assert_eq!(note(audio_a.as_mut().unwrap(), 60), [0.0, 0.0]);
        return_audio(&mut plugin, audio_a.take().unwrap());
        gui.destroy(&plugin.plugin_handle());
        drop(browser_a.take());
        assert!(parent.children().is_empty());
        plugin_b
            .plugin_handle()
            .get_extension::<PluginGui>()
            .unwrap()
            .destroy(&plugin_b.plugin_handle());
        drop(browser_b.take());
        assert!(parent_b.children().is_empty());
    }));

    let mut cleanup_errors = Vec::new();
    if let Some(audio) = audio_a.take()
        && std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            return_audio(&mut plugin, audio)
        }))
        .is_err()
    {
        cleanup_errors.push("A audio return");
    }
    for (plugin, parent, browser, side) in [
        (&mut plugin, &parent, &mut browser_a, "A"),
        (&mut plugin_b, &parent_b, &mut browser_b, "B"),
    ] {
        if (browser.is_some() || !parent.children().is_empty())
            && std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                plugin
                    .plugin_handle()
                    .get_extension::<PluginGui>()
                    .unwrap()
                    .destroy(&plugin.plugin_handle());
                drop(browser.take());
                assert!(parent.children().is_empty());
            }))
            .is_err()
        {
            cleanup_errors.push(if side == "A" { "A GUI" } else { "B GUI" });
        }
    }
    if let Err(payload) = outcome {
        if !cleanup_errors.is_empty() {
            eprintln!("structural native cleanup also failed: {cleanup_errors:?}");
        }
        std::panic::resume_unwind(payload);
    }
    assert!(
        cleanup_errors.is_empty(),
        "native cleanup failures: {cleanup_errors:?}"
    );
}
