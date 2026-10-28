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
        "document.querySelector('[data-testid=trigger-editor]') || document.querySelector('[role=alert]')",
    );
    assert_eq!(
        javascript(
            plugin,
            &browser,
            "document.querySelector('[role=alert]')?.textContent ?? ''"
        ),
        "",
        "native editor initializes from its accepted session"
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
            "document.querySelector('header button[aria-label^=\"Undo \"]:not(:disabled)')",
        );
        javascript(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            r#"(()=>{window.__G3_UNDO__=null;window.__G3_UNDO_ARGS__=[];const backend=window.__DANDRUM__.backend,get=backend.getNativeFunction;backend.getNativeFunction=method=>(...args)=>{const promise=get(method)(...args);if(method==='undoInstrumentSession'){window.__G3_UNDO_ARGS__.push(args);promise.then(result=>window.__G3_UNDO__={status:'accepted',result},error=>window.__G3_UNDO__={status:'rejected',code:error.code});}return promise;};document.querySelector('header button[aria-label^="Undo "]').click();return 'clicked';})()"#,
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
        // Observe raw frames before the broker: rejected oversized ingress is
        // deliberately dropped, so it must not receive even an error reply.
        javascript(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            r#"(()=>{window.__G5_DROPPED_REPLIES__=[];
              const surface=window.__DANDRUM__,receive=surface.receiveNativeMessage;
              surface.receiveNativeMessage=message=>{const frame=JSON.parse(message);
                if(frame.requestId==='900000'){window.__G5_DROPPED_REPLIES__.push(frame);return 'observed';}
                return receive(message);};return 'observing drops';})()"#,
        );
        let rejected_frame = |plugin: &mut PluginInstance<GuiHost>,
                              browser: &webkit2gtk::WebView,
                              version: u32,
                              forged: bool,
                              padding: usize| {
            let script = format!(
                r#"(()=>{{const c=window.__DANDRUM_NATIVE__;
                  const frame={{version:{version},attachmentId:{attachment},requestId:'900000',
                    method:'getSamplerSession',args:[]}};
                  const wire=JSON.stringify(frame)+' '.repeat({padding});
                  window.ipc.postMessage(wire);return String(new TextEncoder().encode(wire).length);}})()"#,
                attachment = if forged {
                    "'18446744073709551615'"
                } else {
                    "c.attachmentId"
                },
            );
            let bytes = javascript(plugin, browser, &script)
                .parse::<usize>()
                .unwrap();
            assert!(bytes > padding);
            // A correlated ordinary request is a main-loop barrier as well as
            // a complete unchanged-session oracle after the raw IPC frame.
            assert_eq!(snapshot(plugin, browser), undone);
            assert_eq!(
                javascript(
                    plugin,
                    browser,
                    "JSON.stringify(window.__G5_DROPPED_REPLIES__)"
                ),
                "[]",
                "oversized v{version} / forged={forged} frame ({bytes} bytes) must be dropped before dispatch"
            );
        };
        rejected_frame(&mut plugin, browser_a.as_ref().unwrap(), 3, false, 20_000);
        // Initialization already negotiated. A supported retry keeps the same
        // profile without changing the accepted session or native request cursor.
        let negotiation = invoke(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            "window.__DANDRUM__.backend.getNativeFunction('negotiateEditorProtocol')(2)",
        );
        assert_eq!(negotiation["status"], "accepted", "{negotiation}");
        assert_eq!(negotiation["result"]["wireVersion"], 2);
        for (version, forged, padding) in
            [(1, false, 20_000), (2, true, 20_000), (2, false, 524_289)]
        {
            rejected_frame(
                &mut plugin,
                browser_a.as_ref().unwrap(),
                version,
                forged,
                padding,
            );
        }
        let before_patch = undone["yaml"].as_str().unwrap();
        let regions = (0..512)
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
            candidate.contains("id: native_511"),
            "fixture contains the additional real regions"
        );
        dandrum_engine::kernel::document::load_kernel_patch_str(&candidate)
            .expect("meaningful bounded sample-region document");
        let script = format!(
            "window.__DANDRUM__.backend.getNativeFunction('prepareInstrumentSessionPatch')(window.__DANDRUM_NATIVE__.instanceId,window.__DANDRUM_NATIVE__.attachmentId,'7003','3',{})",
            serde_json::to_string(&candidate).unwrap()
        );
        let admitted = invoke(&mut plugin, browser_a.as_ref().unwrap(), &script);
        assert_eq!(
            admitted["status"], "accepted",
            "negotiated >16KiB patch must pass through the production browser transport and actual Wry/Core ingress: {admitted}"
        );
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
        let large_snapshot = snapshot(&mut plugin, browser_a.as_ref().unwrap());
        let snapshot_bytes = serde_json::to_vec(&large_snapshot).unwrap().len();
        assert!(
            snapshot_bytes > 65_536,
            "large retained snapshot: {snapshot_bytes} bytes"
        );
        assert_eq!(large_snapshot["yaml"], candidate);
        gui.destroy(&plugin.plugin_handle());
        drop(browser_a.take());
        assert!(parent.children().is_empty());
        browser_a = Some(open_view(&mut plugin, &parent));
        let recovered = snapshot(&mut plugin, browser_a.as_ref().unwrap());
        assert_eq!(recovered["yaml"], candidate);
        assert_eq!(
            recovered["connection"]["instanceId"],
            large_snapshot["connection"]["instanceId"]
        );
        assert_ne!(
            recovered["connection"]["attachmentId"],
            large_snapshot["connection"]["attachmentId"]
        );
        assert_eq!(
            recovered["connection"]["history"],
            large_snapshot["connection"]["history"]
        );
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
        // Start the production React interaction in a fresh attachment to the
        // accepted native state after the preceding direct backend setup.
        gui.destroy(&plugin.plugin_handle());
        drop(browser_a.take());
        assert!(parent.children().is_empty());
        browser_a = Some(open_view(&mut plugin, &parent));
        // A real production React control must serialize a whole document through
        // the persistent worker; it may not fall back to the comparison bridge.
        wait_dom(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            "document.querySelector('header button[aria-label=\"Nothing to undo\"]:disabled')",
        );
        let ui_before = snapshot(&mut plugin, browser_a.as_ref().unwrap());
        let generation = ui_before["document"]["generation"].as_u64().unwrap();
        let value = invoke(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            &format!(
                "window.__DANDRUM__.backend.getNativeFunction('setParameter')('drums.level',0.5,{generation})"
            ),
        );
        assert_eq!(value["status"], "accepted", "{value}");
        audio_a = Some(start_audio(&mut plugin));
        // Admission queues the UI value; a real block must apply it before
        // note() intentionally resets/cancels any unconsumed queued activity.
        let _ = render_audio(
            audio_a.as_mut().unwrap(),
            &EventBuffer::new(),
            &mut Recorder::new(64),
        );
        assert_eq!(note(audio_a.as_mut().unwrap(), 36), [-1.0, -1.0]);
        return_audio(&mut plugin, audio_a.take().unwrap());
        javascript(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            r#"(()=>{window.__UI_DOC_CALLS__=[];const backend=window.__DANDRUM__.backend,get=backend.getNativeFunction;
              backend.getNativeFunction=method=>(...args)=>{if(['prepareSampler','prepareInstrumentSessionPatch','undoInstrumentSession','redoInstrumentSession'].includes(method))window.__UI_DOC_CALLS__.push([method,...args]);return get(method)(...args);};
              [...document.querySelectorAll('[role=tab]')].find(tab=>tab.textContent.trim()==='Voice').click();return 'voice';})()"#,
        );
        wait_dom(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            "document.querySelector('[role=spinbutton][aria-label=\"Voice limit\"][aria-valuenow=\"8\"]')",
        );
        javascript(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            r#"(()=>{document.querySelector('[role=spinbutton][aria-label="Voice limit"]').dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowUp',bubbles:true}));return 'changed';})()"#,
        );
        wait_dom(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            "window.__UI_DOC_CALLS__.length > 0",
        );
        let calls: serde_json::Value = serde_json::from_str(&javascript(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            "JSON.stringify(window.__UI_DOC_CALLS__)",
        ))
        .unwrap();
        assert_eq!(
            calls[0][0], "prepareInstrumentSessionPatch",
            "real React document edits use native history: {calls}"
        );
        wait_dom(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            "document.querySelector('[role=spinbutton][aria-label=\"Voice limit\"][aria-valuenow=\"9\"]')",
        );
        let zone_ids = |state: &serde_json::Value| {
            state["document"]["maps"]
                .as_array()
                .unwrap()
                .iter()
                .map(|map| {
                    (
                        map["id"].clone(),
                        map["zones"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|zone| zone["id"].clone())
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>()
        };
        let edited = snapshot(&mut plugin, browser_a.as_ref().unwrap());
        assert_eq!(
            zone_ids(&edited),
            zone_ids(&ui_before),
            "unrelated React voice edits preserve every native zone identity"
        );
        assert_eq!(edited["connection"]["revision"], "6");
        assert_eq!(
            edited["connection"]["history"],
            serde_json::json!({"undoCount":1,"redoCount":0})
        );
        assert!(edited["yaml"].as_str().unwrap().contains("max_voices: 9"));
        for (action, limit, revision) in [("Undo", 8, "7"), ("Redo", 9, "8")] {
            let selector = format!("header button[aria-label^=\"{action} \"]:not(:disabled)");
            wait_dom(
                &mut plugin,
                browser_a.as_ref().unwrap(),
                &format!(
                    "document.querySelector({})",
                    serde_json::to_string(&selector).unwrap()
                ),
            );
            javascript(
                &mut plugin,
                browser_a.as_ref().unwrap(),
                &format!(
                    "(()=>{{document.querySelector({}).click();return 'clicked';}})()",
                    serde_json::to_string(&selector).unwrap()
                ),
            );
            wait_dom(
                &mut plugin,
                browser_a.as_ref().unwrap(),
                &format!(
                    "document.querySelector('[role=spinbutton][aria-label=\"Voice limit\"][aria-valuenow=\"{limit}\"]')"
                ),
            );
            let current = snapshot(&mut plugin, browser_a.as_ref().unwrap());
            assert_eq!(current["connection"]["revision"], revision);
            assert!(
                current["yaml"]
                    .as_str()
                    .unwrap()
                    .contains(&format!("max_voices: {limit}"))
            );
            let ids = |state: &serde_json::Value| {
                state["document"]["parameters"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|parameter| parameter["id"].clone())
                    .collect::<Vec<_>>()
            };
            assert_eq!(ids(&current), ids(&ui_before));
            assert_eq!(zone_ids(&current), zone_ids(&ui_before));
            let values = feedback(&mut plugin, browser_a.as_ref().unwrap());
            assert_eq!(
                values["parameters"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|parameter| parameter["id"] == "drums.level")
                    .unwrap()["value"],
                0.5
            );
            audio_a = Some(start_audio(&mut plugin));
            assert_eq!(note(audio_a.as_mut().unwrap(), 36), [-1.0, -1.0]);
            assert_eq!(note(audio_a.as_mut().unwrap(), 60), [0.0, 0.0]);
            return_audio(&mut plugin, audio_a.take().unwrap());
            baseline_b.check(&mut plugin_b, &parent_b, browser_b.as_ref().unwrap());
        }
        let calls: serde_json::Value = serde_json::from_str(&javascript(
            &mut plugin,
            browser_a.as_ref().unwrap(),
            "JSON.stringify(window.__UI_DOC_CALLS__)",
        ))
        .unwrap();
        assert_eq!(
            calls
                .as_array()
                .unwrap()
                .iter()
                .map(|call| call[0].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec![
                "prepareInstrumentSessionPatch",
                "undoInstrumentSession",
                "redoInstrumentSession"
            ]
        );
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
