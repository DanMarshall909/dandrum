//! Native chooser acquisition; selection does not publish an instrument edit.
use super::*;

const SELECTED_WAV: &[u8] = &[
    82, 73, 70, 70, 52, 0, 0, 0, 87, 65, 86, 69, 102, 109, 116, 32, 16, 0, 0, 0, 1, 0, 2, 0, 0,
    125, 0, 0, 0, 244, 1, 0, 4, 0, 16, 0, 100, 97, 116, 97, 16, 0, 0, 0, 0, 160, 0, 64, 0, 32, 0,
    224, 0, 192, 0, 16, 0, 64, 0, 160,
];
// Independently computed from the literal 60-byte WAV, not a decoder round trip.
const CONTENT_SHA256: &str = "2e484972f38876c961b3c23fb4952487dce3e18501c7fb0efd1b8cb7011734d5";

#[test]
#[ignore = "native selection boundary: real X11, copied CLAP/React and GTK chooser; no physical-input claim"]
fn copied_native_selection_retains_real_metadata_across_source_and_editor_loss() {
    std::thread::spawn(|| {
        let parent = Parent::new();
        let mut plugin = copied_plugin();
        let params = plugin
            .plugin_handle()
            .get_extension::<clack_extensions::params::PluginParams>()
            .unwrap();
        assert_eq!(params.count(&plugin.plugin_handle()), 23);

        // A real consumer control precedes the expected missing-chooser RED.
        assert_packaged_audio(&mut plugin);
        eprintln!("SELECTION_CONTROL signed packaged PCM = [-0.5, -0.5]");

        let selected = tempfile::tempdir().unwrap();
        let path = selected.path().join("distinct-selected.wav");
        std::fs::write(&path, SELECTED_WAV).unwrap();
        let browser = structural_native::open_view(&mut plugin, &parent);
        let before = structural_native::snapshot(&mut plugin, &browser);
        let saved = saved_state(&mut plugin);
        let started = invoke(
            &mut plugin,
            &browser,
            "window.__DANDRUM__.backend.getNativeFunction('chooseSamplerFiles')()",
        );
        assert_eq!(
            started["status"], "accepted",
            "native sample chooser must be available through the actual copied CLAP: {started}"
        );
        assert_eq!(started["result"]["phase"], "choosing");
        let selection = started["result"]["selectionId"]
            .as_str()
            .unwrap()
            .to_owned();

        // The host ELF and plugin DSO have separate Rust binding state, while
        // sharing GTK's C library and this actual UI thread.
        gtk::init().expect("host bindings join GTK after the plugin initialized it");

        // GTK public API drives a real chooser; no fake bridge or browser path.
        let deadline = Instant::now() + Duration::from_secs(5);
        let chooser = native_dialog(&mut plugin);
        assert!(chooser.set_filename(&path));
        while chooser.filename().as_deref() != Some(path.as_path()) {
            dispatch_context(&mut plugin);
            assert!(
                Instant::now() < deadline,
                "GTK must select the actual requested file"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        chooser.response(gtk::ResponseType::Accept);
        drop(chooser);

        let status_expression = format!(
            "window.__DANDRUM__.backend.getNativeFunction('getSamplerSelectionStatus')({})",
            serde_json::to_string(&selection).unwrap()
        );
        let ready = loop {
            let result = invoke(&mut plugin, &browser, &status_expression);
            assert_eq!(result["status"], "accepted", "{result}");
            match result["result"]["phase"].as_str() {
                Some("ready") => break result["result"].clone(),
                Some("preparing") => {}
                _ => panic!("selected file did not reach decoded readiness: {result}"),
            }
            assert!(
                Instant::now() < deadline,
                "bounded real selected-file preparation"
            );
            std::thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(ready["selectionId"], selection);
        let files = ready["files"].as_array().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0]["name"], "distinct-selected.wav");
        assert_eq!(files[0]["contentSha256"], CONTENT_SHA256);
        assert_eq!(
            files[0]["resourceId"],
            format!("selected/{CONTENT_SHA256}.wav")
        );
        assert_eq!(files[0]["encodedBytes"], 60);
        assert_eq!(files[0]["sampleRateHz"], 32000);
        assert_eq!(files[0]["channelCount"], 2);
        assert_eq!(files[0]["frameCount"], 4);

        std::fs::write(&path, b"replacement is not a WAV").unwrap();
        assert_eq!(
            invoke(&mut plugin, &browser, &status_expression)["result"],
            ready
        );
        std::fs::remove_file(&path).unwrap();
        let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
        gui.destroy(&plugin.plugin_handle());
        drop(browser);
        assert!(parent.children().is_empty());
        let browser = structural_native::open_view(&mut plugin, &parent);
        let after = structural_native::snapshot(&mut plugin, &browser);
        assert_ne!(
            after["connection"]["attachmentId"],
            before["connection"]["attachmentId"]
        );
        assert_eq!(after["yaml"], before["yaml"]);
        assert_eq!(after["document"], before["document"]);
        assert_eq!(
            invoke(&mut plugin, &browser, &status_expression)["result"],
            ready
        );
        assert_eq!(
            saved_state(&mut plugin),
            saved,
            "selection alone cannot edit accepted state"
        );
        // This choosing work has not entered the worker and belongs to this view.
        let choosing = invoke(
            &mut plugin,
            &browser,
            "window.__DANDRUM__.backend.getNativeFunction('chooseSamplerFiles')()",
        );
        assert_eq!(choosing["status"], "accepted");
        assert_eq!(choosing["result"]["phase"], "choosing");
        let cancelled_id = choosing["result"]["selectionId"].as_str().unwrap();
        let cancelled_expression = format!(
            "window.__DANDRUM__.backend.getNativeFunction('getSamplerSelectionStatus')({})",
            serde_json::to_string(cancelled_id).unwrap()
        );
        let chooser = native_dialog(&mut plugin);
        let dialog_window = chooser.window().expect("real chooser window");
        use gtk::glib::translate::ToGlibPtr;
        unsafe extern "C" {
            fn gdk_x11_window_get_xid(window: *mut gtk::gdk::ffi::GdkWindow) -> std::ffi::c_ulong;
        }
        // SAFETY: this is the live X11-backed GDK window on its GTK UI thread;
        // only the copied numeric ID is used after the plugin destroys it.
        let dialog_xid = unsafe { gdk_x11_window_get_xid(dialog_window.to_glib_none().0) } as u32;
        drop(dialog_window);
        drop(chooser); // No widget methods may follow native destruction.
        gui.destroy(&plugin.plugin_handle());
        drop(browser);
        assert!(parent.children().is_empty());
        assert!(
            parent.x.query_tree(dialog_xid).unwrap().reply().is_err(),
            "the open chooser's actual X11 window is destroyed"
        );
        let browser = structural_native::open_view(&mut plugin, &parent);
        assert_eq!(
            invoke(&mut plugin, &browser, &cancelled_expression)["result"]["phase"],
            "cancelled"
        );
        assert_eq!(
            invoke(&mut plugin, &browser, &status_expression)["result"],
            ready
        );
        assert_eq!(saved_state(&mut plugin), saved);
        assert_packaged_audio(&mut plugin);
        gui.destroy(&plugin.plugin_handle());
        drop(browser);
        assert!(parent.children().is_empty());
    })
    .join()
    .expect("native selected-file host thread completed");
}

fn assert_packaged_audio(plugin: &mut PluginInstance<GuiHost>) {
    let mut audio = start_audio(plugin);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for key in [36, 60] {
            std::thread::scope(|scope| scope.spawn(|| audio.reset()).join().unwrap());
            let mut events = EventBuffer::new();
            events.push(&MidiEvent::new(0, 0, [0x90, key, 100]));
            let pcm = render_audio(&mut audio, &events, &mut Recorder::new(64));
            if key == 36 {
                for channel in pcm {
                    assert_eq!(&channel[..2], &[-0.5, -358.0 / 32768.0]);
                }
            } else {
                assert!(pcm.iter().flatten().all(|&value| value == 0.0));
            }
        }
    }));
    return_audio(plugin, audio);
    result.unwrap();
}

fn native_dialog(plugin: &mut PluginInstance<GuiHost>) -> gtk::FileChooserDialog {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        dispatch_context(plugin);
        let mut dialogs = gtk::Window::list_toplevels()
            .into_iter()
            .filter_map(|window| window.downcast::<gtk::FileChooserDialog>().ok());
        if let Some(dialog) = dialogs.next() {
            assert!(dialogs.next().is_none(), "one instance-owned chooser");
            return dialog;
        }
        assert!(
            Instant::now() < deadline,
            "choosing requires a real GTK chooser"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[ignore = "native production Import boundary: real React/Wry/GTK acquisition; no physical-input claim"]
fn copied_react_import_maps_selected_pcm_once_through_native_history() {
    std::thread::spawn(|| {
        let parent = Parent::new();
        let mut plugin = copied_plugin();
        let mut independent = copied_plugin();
        assert_packaged_audio(&mut plugin);
        assert_packaged_audio(&mut independent);
        let independent_state = saved_state(&mut independent);
        let selected = tempfile::tempdir().unwrap();
        let path = selected.path().join("distinct-selected.wav");
        std::fs::write(&path, SELECTED_WAV).unwrap();
        let mut browser = structural_native::open_view(&mut plugin, &parent);
        let initial = structural_native::snapshot(&mut plugin, &browser);
        let generation = initial["document"]["generation"].as_u64().unwrap();
        let changed = invoke(&mut plugin, &browser, &format!(
            "window.__DANDRUM__.backend.getNativeFunction('setParameter')('drums.level',0.5,{generation})"));
        assert_eq!(changed["status"], "accepted");
        assert_import_audio(&mut plugin, false);
        let before = structural_native::snapshot(&mut plugin, &browser);
        assert_eq!(before["document"]["parameters"].as_array().unwrap().iter()
            .find(|parameter| parameter["id"] == "drums.level").unwrap()["normalisedValue"], 0.5);
        javascript(&mut plugin, &browser, r#"(()=>{
            if(!document.querySelector('button[aria-label="Add source…"]'))document.querySelector('button[aria-label="Instrument tree"]').click();
            return 'tree';
        })()"#);
        wait_dom(&mut plugin, &browser, r#"document.querySelector('button[aria-label="Add source…"]')"#);
        javascript(&mut plugin, &browser, r#"(()=>{document.querySelector('button[aria-label="Add source…"]').click();return 'choose';})()"#);
        gtk::init().expect("host joins GTK after plugin initialization");
        let chooser = native_dialog(&mut plugin);
        assert!(chooser.set_filename(&path));
        let deadline = Instant::now() + Duration::from_secs(5);
        while chooser.filename().as_deref() != Some(path.as_path()) {
            dispatch_context(&mut plugin);
            assert!(Instant::now() < deadline, "real chooser selects the requested WAV");
            std::thread::sleep(Duration::from_millis(5));
        }
        chooser.response(gtk::ResponseType::Accept);
        drop(chooser);
        let imported = loop {
            let current = structural_native::snapshot(&mut plugin, &browser);
            if current["yaml"].as_str().unwrap().contains(CONTENT_SHA256) {
                break current;
            }
            assert!(Instant::now() < deadline,
                "React Import must consume the real selected outcome and publish the admitted content; history={:?}, selections={:?}",
                current["connection"]["history"], current["sampleSelections"]);
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(imported["connection"]["history"], serde_json::json!({"undoCount":1,"redoCount":0}));
        assert_eq!(imported["document"]["parameters"], before["document"]["parameters"]);
        assert_eq!(imported["document"]["sources"].as_array().unwrap().len(), 2);
        let source = imported["document"]["sources"].as_array().unwrap().iter().find(|source| source["id"] != "drums").unwrap();
        assert_eq!(source["sampleRateHz"], 32000);
        assert_eq!(source["channelCount"], 2);
        assert_eq!(source["frameCount"], "4");
        std::fs::remove_file(&path).unwrap();
        assert_import_audio(&mut plugin, true);
        let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
        gui.destroy(&plugin.plugin_handle());
        drop(browser);
        assert!(parent.children().is_empty());
        browser = structural_native::open_view(&mut plugin, &parent);
        let recreated = structural_native::snapshot(&mut plugin, &browser);
        assert_ne!(recreated["connection"]["attachmentId"], imported["connection"]["attachmentId"]);
        assert_eq!(recreated["yaml"], imported["yaml"]);
        assert_eq!(recreated["document"], imported["document"]);
        for (action, wanted, revision, undo, redo) in [("Undo", false, "2", 0, 1), ("Redo", true, "3", 1, 0)] {
            let selector = format!("header button[aria-label^=\"{action} \"]:not(:disabled)");
            let selector = serde_json::to_string(&selector).unwrap();
            wait_dom(&mut plugin, &browser, &format!("document.querySelector({selector})"));
            javascript(&mut plugin, &browser, &format!("document.querySelector({selector}).click() || 'clicked'"));
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                let current = structural_native::snapshot(&mut plugin, &browser);
                if current["connection"]["revision"] == revision {
                    assert_eq!(current["yaml"].as_str().unwrap().contains(CONTENT_SHA256), wanted);
                    assert_eq!(current["connection"]["history"], serde_json::json!({"undoCount":undo,"redoCount":redo}));
                    assert_eq!(current["document"]["parameters"], before["document"]["parameters"]);
                    break;
                }
                assert!(Instant::now() < deadline, "native UI history completes once");
                std::thread::sleep(Duration::from_millis(10));
            }
            assert_import_audio(&mut plugin, wanted);
            assert_eq!(saved_state(&mut independent), independent_state);
            assert_packaged_audio(&mut independent);
        }
        gui.destroy(&plugin.plugin_handle());
        drop(browser);
        assert!(parent.children().is_empty());
    }).join().expect("production React selected import host completed");
}

fn assert_import_audio(plugin: &mut PluginInstance<GuiHost>, selected: bool) {
    let mut audio = start_audio(plugin);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // Apply queued gain before reset deliberately cancels unconsumed activity.
        let _ = render_audio(&mut audio, &EventBuffer::new(), &mut Recorder::new(64));
        // Existing stereo playback mix: (-0.75 + 0.5) / 2, times physical gain2.
        for (key, expected) in [
            (48, if selected { [-0.25, -0.25] } else { [0.0, 0.0] }),
            (36, [-1.0, -1.0]),
            (60, [0.0, 0.0]),
        ] {
            std::thread::scope(|scope| scope.spawn(|| audio.reset()).join().unwrap());
            let mut events = EventBuffer::new();
            events.push(&MidiEvent::new(0, 0, [0x90, key, 127]));
            let pcm = render_audio(&mut audio, &events, &mut Recorder::new(64));
            assert_eq!(
                [pcm[0][0], pcm[1][0]],
                expected,
                "known signed native mapping at MIDI{key}"
            );
        }
    }));
    return_audio(plugin, audio);
    result.unwrap();
}
