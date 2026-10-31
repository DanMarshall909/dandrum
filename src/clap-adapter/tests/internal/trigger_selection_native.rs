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
#[ignore = "real X11 installed CLAP paint boundary"]
fn installed_react_editor_paints_and_releases_its_host_child() {
    std::thread::spawn(|| {
        let parent = Parent::new();
        let mut plugin = copied_plugin();
        let browser = structural_native::open_view(&mut plugin, &parent);
        let initial_viewport = javascript(
            &mut plugin,
            &browser,
            "window.innerWidth + 'x' + window.innerHeight",
        );
        let child = parent.children()[0];
        let geometry = parent.x.get_geometry(child).unwrap().reply().unwrap();
        let child_size = (geometry.width, geometry.height);
        let deadline = Instant::now() + Duration::from_secs(5);
        let (painted, sampled, viewport) = loop {
            dispatch_context(&mut plugin);
            let viewport = javascript(
                &mut plugin,
                &browser,
                "window.innerWidth + 'x' + window.innerHeight",
            );
            let sampled: Vec<_> = [(20, 20), (200, 100), (600, 400), (1000, 700)]
                .map(|(x, y)| {
                    parent
                        .x
                        .get_image(
                            x11rb::protocol::xproto::ImageFormat::Z_PIXMAP,
                            child,
                            x,
                            y,
                            1,
                            1,
                            u32::MAX,
                        )
                        .unwrap()
                        .reply()
                        .unwrap()
                        .data
                })
                .into();
            if viewport == "1200x800"
                && sampled
                    .iter()
                    .any(|pixel| pixel.len() >= 3 && pixel[0] != pixel[1])
            {
                break (true, sampled, viewport);
            }
            if Instant::now() >= deadline {
                break (false, sampled, viewport);
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
        gui.destroy(&plugin.plugin_handle());
        drop(browser);
        assert!(
            parent.children().is_empty(),
            "editor child closes immediately"
        );
        assert_eq!(
            child_size,
            (1200, 800),
            "CLAP's advertised physical editor size"
        );
        assert_eq!(
            viewport, "1200x800",
            "complete React layout fits the editor; initial viewport={initial_viewport}"
        );
        assert!(
            painted,
            "installed React editor remained gray; sampled pixels={sampled:?}"
        );
    })
    .join()
    .expect("installed native paint host thread completed");
}

#[test]
#[ignore = "manual desktop acceptance: use real mouse and keyboard input in the installed editor"]
fn installed_react_accepts_physical_sample_selection_and_retains_native_history() {
    std::thread::spawn(|| {
        let selected = tempfile::tempdir().unwrap();
        let path = selected.path().join("physical-selected.wav");
        std::fs::write(&path, SELECTED_WAV).unwrap();
        let parent = Parent::new();
        let mut plugin = copied_plugin();
        let mut independent = copied_plugin();
        let independent_state = saved_state(&mut independent);
        assert_packaged_audio(&mut plugin);
        let mut browser = structural_native::open_view(&mut plugin, &parent);
        let initial = structural_native::snapshot(&mut plugin, &browser);
        eprintln!("MANUAL_READY select {} through Instrument tree > Add source in the visible Trigger window", path.display());

        let deadline = Instant::now() + Duration::from_secs(180);
        let imported = loop {
            let current = structural_native::snapshot(&mut plugin, &browser);
            if current["yaml"].as_str().unwrap().contains(CONTENT_SHA256) {
                break current;
            }
            assert!(Instant::now() < deadline, "physical native selection and import timed out");
            std::thread::sleep(Duration::from_millis(50));
        };
        assert_eq!(imported["connection"]["history"], serde_json::json!({"undoCount":1,"redoCount":0}));
        assert_ne!(imported["connection"]["revision"], initial["connection"]["revision"]);
        selected.close().unwrap(); // This test owns the temporary source directory.
        assert_mapping_audio(&mut plugin, &[(48, [-0.125, -0.125]), (36, [-0.5, -0.5])]);

        let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
        gui.destroy(&plugin.plugin_handle());
        drop(browser);
        assert!(parent.children().is_empty());
        browser = structural_native::open_view(&mut plugin, &parent);
        let recreated = structural_native::snapshot(&mut plugin, &browser);
        assert_ne!(recreated["connection"]["attachmentId"], imported["connection"]["attachmentId"]);
        assert_eq!(recreated["yaml"], imported["yaml"]);

        for (action, revision, undo, redo, selected_audio) in
            [("Undo", "2", 0, 1, false), ("Redo", "3", 1, 0, true)]
        {
            eprintln!("MANUAL_{action} click {action} in the reopened Trigger window");
            let deadline = Instant::now() + Duration::from_secs(90);
            loop {
                let current = structural_native::snapshot(&mut plugin, &browser);
                if current["connection"]["revision"] == revision {
                    assert_eq!(current["connection"]["history"], serde_json::json!({"undoCount":undo,"redoCount":redo}));
                    assert_eq!(current["yaml"].as_str().unwrap().contains(CONTENT_SHA256), selected_audio);
                    break;
                }
                assert!(Instant::now() < deadline, "physical {action} timed out");
                std::thread::sleep(Duration::from_millis(50));
            }
            assert_mapping_audio(&mut plugin, &[(48, if selected_audio { [-0.125, -0.125] } else { [0.0, 0.0] }), (36, [-0.5, -0.5])]);
        }
        assert_eq!(saved_state(&mut independent), independent_state);
        assert_packaged_audio(&mut independent);
        gui.destroy(&plugin.plugin_handle());
        drop(browser);
        assert!(parent.children().is_empty());
    })
    .join()
    .expect("manual native input host thread completed");
}

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
        // Exercise the production keyboard cancellation path against real native leases.
        let before_cancel = structural_native::snapshot(&mut plugin, &browser);
        let first = selected.path().join("cancel-first.wav");
        let second = selected.path().join("cancel-second.wav");
        std::fs::write(&first, SELECTED_WAV).unwrap();
        std::fs::write(&second, SELECTED_WAV).unwrap();
        javascript(&mut plugin, &browser, r#"(()=>{
            if(!document.querySelector('button[aria-label="Add source…"]'))document.querySelector('button[aria-label="Instrument tree"]').click();
            return 'tree';
        })()"#);
        wait_dom(&mut plugin, &browser, r#"document.querySelector('button[aria-label="Add source…"]')"#);
        javascript(&mut plugin, &browser, r#"(()=>{document.querySelector('button[aria-label="Add source…"]').click();return 'choose';})()"#);
        let chooser = native_dialog(&mut plugin);
        assert!(chooser.set_filename(&first));
        let deadline = Instant::now() + Duration::from_secs(5);
        while chooser.filename().as_deref() != Some(first.as_path()) {
            dispatch_context(&mut plugin);
            assert!(Instant::now() < deadline, "first cancellation fixture file selected");
            std::thread::sleep(Duration::from_millis(5));
        }
        while chooser.filenames().len() != 2 {
            dispatch_context(&mut plugin);
            // The chooser may report its pending filename before its folder model
            // is loaded; select its two actual files once that model is ready.
            chooser.select_all();
            assert!(Instant::now() < deadline, "two actual selected files create mapping presentation: {:?}", chooser.filenames());
            std::thread::sleep(Duration::from_millis(5));
        }
        chooser.response(gtk::ResponseType::Accept);
        drop(chooser);
        wait_dom(&mut plugin, &browser, r#"document.querySelector('section[aria-label="Import mapping"]')"#);
        let ready = structural_native::snapshot(&mut plugin, &browser);
        eprintln!("CANCEL_CHOICES before={}, ready={}", before_cancel["sampleSelections"], ready["sampleSelections"]);
        let choices = ready["sampleSelections"].as_array().unwrap().iter()
            .filter(|selection| selection["phase"] == "ready" && !before_cancel["sampleSelections"].as_array().unwrap().iter()
                .any(|previous| previous["selectionId"] == selection["selectionId"]))
            .collect::<Vec<_>>();
        assert_eq!(choices.len(), 1, "one newly acquired mapping choice");
        let choice = choices[0]["selectionId"].clone();
        javascript(&mut plugin, &browser, r#"(()=>{window.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}));return 'escape';})()"#);
        eprintln!("CANCEL_DOM {}", javascript(&mut plugin, &browser, r#"JSON.stringify({mapping:!!document.querySelector('section[aria-label="Import mapping"]'),text:document.body.innerText.slice(-1600)})"#));
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let current = structural_native::snapshot(&mut plugin, &browser);
            let phase = &current["sampleSelections"].as_array().unwrap().iter()
                .find(|selection| selection["selectionId"] == choice).unwrap()["phase"];
            if phase == "released" {
                assert_eq!(current["yaml"], before_cancel["yaml"]);
                assert_eq!(current["connection"]["history"], before_cancel["connection"]["history"]);
                assert_eq!(current["document"]["parameters"], before_cancel["document"]["parameters"]);
                break;
            }
            assert!(Instant::now() < deadline, "real React Escape must release the discarded native selection; phase={phase}");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_import_audio(&mut plugin, true);
        let selected_state = saved_state(&mut plugin);
        let selected_wire = portable_state::metadata(&selected_state);
        assert_eq!(selected_wire["document"], before_cancel["yaml"]);
        assert_eq!(selected_wire["resources"].as_array().unwrap().len(), 2);
        assert!(selected_wire["resources"].as_array().unwrap().iter().any(|resource|
            resource["path"] == format!("selected/{CONTENT_SHA256}.wav")
                && resource["sha256"] == CONTENT_SHA256
                && resource["encoded_bytes"] == SELECTED_WAV.len()));
        assert_eq!(selected_wire["parameters"].as_array().unwrap().len(), 23);
        assert_eq!(selected_wire["parameters"].as_array().unwrap().iter()
            .find(|parameter| parameter["id"] == "drums.level").unwrap()["value"], 2.0);
        gui.destroy(&plugin.plugin_handle());
        drop(browser);
        assert!(parent.children().is_empty());
        drop(plugin);
        selected.close().unwrap();

        // A different DSO inode has separate process statics. Its package is
        // needed only for initialization and disappears before state loading.
        let fresh_root = tempfile::tempdir().unwrap();
        let fresh_path = fresh_root.path().join("DandrumTrigger.clap");
        std::fs::copy(artifact(), &fresh_path).unwrap();
        let source_resources = artifact().parent().unwrap().join("DandrumTrigger.resources");
        let fresh_resources = fresh_root.path().join("DandrumTrigger.resources/instrument");
        std::fs::create_dir_all(fresh_resources.join("assets")).unwrap();
        for relative in ["advanced-drum-kit.yaml", "assets/advanced-drums.wav"] {
            std::fs::copy(source_resources.join("instrument").join(relative),
                fresh_resources.join(relative)).unwrap();
        }
        use std::os::unix::ffi::OsStrExt;
        let declared = std::ffi::CString::new(fresh_path.as_os_str().as_bytes()).unwrap();
        let provider = unsafe { clack_host::entry::LibraryEntry::load_from_path(&fresh_path) }.unwrap();
        let entry = unsafe { PluginEntry::load_from(provider, &declared) }.unwrap();
        let mut fresh = instantiate(&entry);
        std::fs::remove_file(fresh_resources.join("assets/advanced-drums.wav")).unwrap();
        let handle = fresh.plugin_handle();
        handle.get_extension::<clack_extensions::state::PluginState>().unwrap()
            .load(&handle, &mut selected_state.as_slice()).unwrap();
        assert_eq!(saved_state(&mut fresh), selected_state,
            "fresh CLAP retains the logical document, resource identities and named values");
        let params = fresh.plugin_handle().get_extension::<PluginParams>().unwrap();
        for (index, named) in selected_wire["parameters"].as_array().unwrap().iter().enumerate() {
            assert_eq!(params.get_value(&fresh.plugin_handle(), ClapId::new(index as u32 + 1)),
                named["value"].as_f64(), "restored physical value for {}", named["id"]);
        }
        assert_import_audio(&mut fresh, true);
        assert_eq!(saved_state(&mut independent), independent_state);
        assert_packaged_audio(&mut independent);
    }).join().expect("production React selected import host completed");
}

fn assert_import_audio(plugin: &mut PluginInstance<GuiHost>, selected: bool) {
    assert_mapping_audio(
        plugin,
        &[
            (48, if selected { [-0.25, -0.25] } else { [0.0, 0.0] }),
            (36, [-1.0, -1.0]),
            (60, [0.0, 0.0]),
        ],
    );
}
fn assert_mapping_audio(plugin: &mut PluginInstance<GuiHost>, cases: &[(u8, [f32; 2])]) {
    let mut audio = start_audio(plugin);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // Apply queued gain before reset deliberately cancels unconsumed activity.
        let _ = render_audio(&mut audio, &EventBuffer::new(), &mut Recorder::new(64));
        // Existing stereo playback mix: (-0.75 + 0.5) / 2, times physical gain2.
        for &(key, expected) in cases {
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

#[test]
#[ignore = "native Empty/import boundary: actual copied React/Wry/GTK; no physical-input claim"]
fn copied_react_empty_retains_controls_and_imports_after_recreation() {
    std::thread::spawn(|| {
        let parent = Parent::new();
        let mut plugin = copied_plugin();
        let mut independent = copied_plugin();
        assert_packaged_audio(&mut independent);
        let independent_state = saved_state(&mut independent);
        let mut browser = structural_native::open_view(&mut plugin, &parent);
        let initial = structural_native::snapshot(&mut plugin, &browser);
        let generation = initial["document"]["generation"].as_u64().unwrap();
        let value = invoke(&mut plugin, &browser, &format!(
            "window.__DANDRUM__.backend.getNativeFunction('setParameter')('drums.level',0.5,{generation})"));
        assert_eq!(value["status"], "accepted");
        assert_import_audio(&mut plugin, false);
        let before = structural_native::snapshot(&mut plugin, &browser);
        assert_eq!(before["document"]["parameters"].as_array().unwrap().len(), 23);
        javascript(&mut plugin, &browser, r#"(()=>{Array.from(document.querySelectorAll('header button')).find(button=>button.textContent.trim().startsWith('Patch')).click();return 'menu';})()"#);
        wait_dom(&mut plugin, &browser, "document.querySelector('.patch-menu')");
        javascript(&mut plugin, &browser, r#"(()=>{Array.from(document.querySelectorAll('.patch-menu button')).find(button=>button.textContent.trim()==='New empty patch').click();return 'new';})()"#);
        let deadline = Instant::now() + Duration::from_secs(5);
        let empty = loop {
            let current = structural_native::snapshot(&mut plugin, &browser);
            if current["connection"]["revision"] == "1" && current["document"]["maps"].as_array().unwrap().is_empty() {
                break current;
            }
            assert!(Instant::now() < deadline,
                "New empty patch must be a native accepted silent document; revision={}, maps={}, history={}",
                current["connection"]["revision"], current["document"]["maps"], current["connection"]["history"]);
            std::thread::sleep(Duration::from_millis(5));
        };
        assert!(empty["document"]["sources"].as_array().unwrap().is_empty());
        assert_eq!(empty["document"]["parameters"], before["document"]["parameters"]);
        assert_eq!(empty["connection"]["history"], serde_json::json!({"undoCount":1,"redoCount":0}));
        assert_mapping_audio(&mut plugin, &[(36,[0.0,0.0]),(48,[0.0,0.0]),(60,[0.0,0.0])]);
        let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
        gui.destroy(&plugin.plugin_handle());drop(browser);assert!(parent.children().is_empty());
        browser = structural_native::open_view(&mut plugin, &parent);
        let recreated = structural_native::snapshot(&mut plugin, &browser);
        assert_ne!(recreated["connection"]["attachmentId"], empty["connection"]["attachmentId"]);
        assert_eq!(recreated["yaml"], empty["yaml"]);assert_eq!(recreated["document"], empty["document"]);
        assert_mapping_audio(&mut plugin, &[(36,[0.0,0.0]),(48,[0.0,0.0]),(60,[0.0,0.0])]);
        // This new consumer has only the accepted Empty graph, never a cached kit template.
        wait_dom(&mut plugin, &browser, r#"Array.from(document.querySelectorAll('button')).some(button=>button.textContent.trim()==='Browse samples…')"#);
        let selected = tempfile::tempdir().unwrap();let path = selected.path().join("empty-selected.wav");
        std::fs::write(&path, SELECTED_WAV).unwrap();
        javascript(&mut plugin, &browser, r#"(()=>{Array.from(document.querySelectorAll('button')).find(button=>button.textContent.trim()==='Browse samples…').click();return 'choose';})()"#);
        gtk::init().expect("host joins initialized GTK");let chooser = native_dialog(&mut plugin);
        assert!(chooser.set_filename(&path));let deadline = Instant::now() + Duration::from_secs(5);
        while chooser.filename().as_deref() != Some(path.as_path()) {
            dispatch_context(&mut plugin);assert!(Instant::now() < deadline, "real Empty import file selection");
            std::thread::sleep(Duration::from_millis(5));
        }
        chooser.response(gtk::ResponseType::Accept);drop(chooser);
        loop {
            let current = structural_native::snapshot(&mut plugin, &browser);
            if current["connection"]["revision"] == "2" && current["yaml"].as_str().unwrap().contains(CONTENT_SHA256) {
                assert_eq!(current["document"]["parameters"], before["document"]["parameters"]);
                assert_eq!(current["document"]["sources"].as_array().unwrap().len(),1);
                assert_eq!(current["connection"]["history"],serde_json::json!({"undoCount":2,"redoCount":0}));break;
            }
            assert!(Instant::now() < deadline, "recreated Empty editor publishes the selected sample without a kit fallback");
            std::thread::sleep(Duration::from_millis(5));
        }
        std::fs::remove_file(&path).unwrap();
        assert_mapping_audio(&mut plugin, &[(48,[-0.25,-0.25]),(36,[0.0,0.0]),(60,[0.0,0.0])]);
        for (action,revision,selected,undo,redo) in [("Undo","3",false,1,1),("Redo","4",true,2,0)] {
            let selector=serde_json::to_string(&format!("header button[aria-label^=\"{action} \"]:not(:disabled)")).unwrap();
            wait_dom(&mut plugin,&browser,&format!("document.querySelector({selector})"));
            javascript(&mut plugin,&browser,&format!("document.querySelector({selector}).click() || 'clicked'"));
            let deadline=Instant::now()+Duration::from_secs(5);
            loop {
                let current=structural_native::snapshot(&mut plugin,&browser);
                if current["connection"]["revision"]==revision {
                    assert_eq!(current["yaml"].as_str().unwrap().contains(CONTENT_SHA256),selected);
                    assert_eq!(current["document"]["parameters"],before["document"]["parameters"]);
                    assert_eq!(current["connection"]["history"],serde_json::json!({"undoCount":undo,"redoCount":redo}));break;
                }
                assert!(Instant::now()<deadline,"Empty/import history completes once");std::thread::sleep(Duration::from_millis(5));
            }
            assert_mapping_audio(&mut plugin,&[(48,if selected {[-0.25,-0.25]} else {[0.0,0.0]}),(36,[0.0,0.0]),(60,[0.0,0.0])]);
        }
        assert_eq!(saved_state(&mut independent),independent_state);assert_packaged_audio(&mut independent);
        gui.destroy(&plugin.plugin_handle());drop(browser);assert!(parent.children().is_empty());
    }).join().expect("real Empty/import host thread completes");
}

#[test]
#[ignore = "native recovery boundary: copied React/Wry/GTK and real runtime backpressure; no physical-input claim"]
fn copied_react_recovers_selected_import_after_lost_reply_and_recreation() {
    std::thread::spawn(|| {
        let parent = Parent::new();
        let mut plugin = copied_plugin();
        let mut independent = copied_plugin();
        assert_packaged_audio(&mut independent);
        let independent_state = saved_state(&mut independent);
        let selected = tempfile::tempdir().unwrap();
        let path = selected.path().join("recovered-selected.wav");
        std::fs::write(&path, SELECTED_WAV).unwrap();
        let mut browser = None;
        let mut audio = None;
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            browser = Some(structural_native::open_view(&mut plugin, &parent));
            let view = browser.as_ref().unwrap();
            let initial = structural_native::snapshot(&mut plugin, view);
            let generation = initial["document"]["generation"].as_u64().unwrap();
            assert_eq!(invoke(&mut plugin, view, &format!(
                "window.__DANDRUM__.backend.getNativeFunction('setParameter')('drums.level',0.5,{generation})"))["status"], "accepted");
            assert_import_audio(&mut plugin, false);
            audio = Some(start_audio(&mut plugin));
            // Publish an ordinary React edit without processing another audio block.
            // Its real capacity-one runtime slot holds the subsequent import pending.
            javascript(&mut plugin, view, r#"(()=>{[...document.querySelectorAll('[role=tab]')].find(tab=>tab.textContent.trim()==='Voice').click();return 'voice';})()"#);
            wait_dom(&mut plugin, view, "document.querySelector('[role=spinbutton][aria-label=\"Voice limit\"][aria-valuenow=\"8\"]')");
            javascript(&mut plugin, view, r#"(()=>{document.querySelector('[role=spinbutton][aria-label="Voice limit"]').dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowUp',bubbles:true}));return 'changed';})()"#);
            wait_dom(&mut plugin, view, "document.querySelector('header button[aria-label^=\"Undo \"]:not(:disabled)')");
            let barrier = structural_native::snapshot(&mut plugin, view);
            assert_eq!(barrier["connection"]["revision"], "1");
            assert_eq!(barrier["connection"]["history"], serde_json::json!({"undoCount":1,"redoCount":0}));
            assert!(barrier["yaml"].as_str().unwrap().contains("max_voices: 9"));
            javascript(&mut plugin, view, r#"(()=>{
                window.__LOST_IMPORT_REPLY__=null;const surface=window.__DANDRUM__,receive=surface.receiveNativeMessage;
                surface.receiveNativeMessage=message=>{const frame=JSON.parse(message);if(frame.result?.job_id==='2'||frame.error){window.__LOST_IMPORT_REPLY__=frame;return 'withheld';}return receive(message);};
                if(!document.querySelector('button[aria-label="Add source…"]'))document.querySelector('button[aria-label="Instrument tree"]').click();return 'observing';
            })()"#);
            wait_dom(&mut plugin, view, r#"document.querySelector('button[aria-label="Add source…"]')"#);
            javascript(&mut plugin, view, r#"document.querySelector('button[aria-label="Add source…"]').click() || 'choose'"#);
            gtk::init().expect("host joins GTK");
            let chooser = native_dialog(&mut plugin);
            assert!(chooser.set_filename(&path));
            let deadline = Instant::now() + Duration::from_secs(5);
            while chooser.filename().as_deref() != Some(path.as_path()) {
                dispatch_context(&mut plugin);
                assert!(Instant::now() < deadline, "real recovery chooser selects the WAV");
                std::thread::sleep(Duration::from_millis(5));
            }
            chooser.response(gtk::ResponseType::Accept);
            drop(chooser);
            wait_dom(&mut plugin, view, "window.__LOST_IMPORT_REPLY__ !== null");
            let admission: serde_json::Value = serde_json::from_str(&javascript(&mut plugin, view, "JSON.stringify(window.__LOST_IMPORT_REPLY__)")).unwrap();
            assert_eq!(admission["status"], "accepted", "actual selected import admission: {admission}");
            assert_eq!(admission["result"]["job_id"], "2");
            let pending = structural_native::snapshot(&mut plugin, view);
            assert_eq!(pending["connection"]["activeRequestId"], "2");
            assert_eq!(pending["connection"]["revision"], "1");
            std::fs::remove_file(&path).unwrap();
            let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
            gui.destroy(&plugin.plugin_handle());
            drop(browser.take());
            assert!(parent.children().is_empty());
            // Observe a real recreated view while production initialization is pending.
            gui.create(&plugin.plugin_handle(), GuiConfiguration {api_type:GuiApiType::X11,is_floating:false}).unwrap();
            unsafe {gui.set_parent(&plugin.plugin_handle(),Window::from_x11_handle(parent.id.into()))}.unwrap();
            gui.show(&plugin.plugin_handle()).unwrap();
            browser = Some(weak_browser_in_child(parent.children()[0]).upgrade().unwrap().downcast::<webkit2gtk::WebView>().unwrap());
            let view = browser.as_ref().unwrap();
            wait_dom(&mut plugin, view, "window.__DANDRUM__?.backend");
            javascript(&mut plugin, view, r#"(()=>{window.__RECOVERY_CALLS__=[];const backend=window.__DANDRUM__.backend,get=backend.getNativeFunction;backend.getNativeFunction=method=>(...args)=>{window.__RECOVERY_CALLS__.push([method,...args]);return get(method)(...args);};return 'observing';})()"#);
            wait_dom(&mut plugin, view, "window.__RECOVERY_CALLS__.some(([method])=>method==='getInstrumentSessionJobStatus') || document.querySelector('[data-testid=trigger-editor]') || document.querySelector('[role=alert]')");
            assert_eq!(javascript(&mut plugin, view, "String(window.__RECOVERY_CALLS__.some(([method,id])=>method==='getInstrumentSessionJobStatus'&&id==='2'))"), "true", "recreated production editor must discover and follow the existing import without a manual status call");
            assert_eq!(javascript(&mut plugin, view, "String(!!document.querySelector('[data-testid=trigger-editor]'))"), "false", "pending recovery does not expose stale editable state");
            let recovering = structural_native::snapshot(&mut plugin, view);
            assert_ne!(recovering["connection"]["attachmentId"], pending["connection"]["attachmentId"]);
            assert_eq!(recovering["connection"]["activeRequestId"], "2");
            // A real block consumes the earlier runtime; ordinary main-loop service
            // then reclaims it and publishes the already-owned selected import.
            let mut events = EventBuffer::new();
            events.push(&MidiEvent::new(0,0,[0x90,36,127]));
            let pcm = render_audio(audio.as_mut().unwrap(),&events,&mut Recorder::new(64));
            assert_eq!([pcm[0][0],pcm[1][0]],[-1.0,-1.0]);
            wait_dom(&mut plugin, view, "document.querySelector('[data-testid=trigger-editor]') || document.querySelector('[role=alert]')");
            assert_eq!(javascript(&mut plugin, view, "document.querySelector('[role=alert]')?.textContent ?? ''"), "");
            let recovered = structural_native::snapshot(&mut plugin, view);
            assert_eq!(recovered["connection"]["revision"], "2");
            assert_eq!(recovered["connection"]["nextRequestId"], "3");
            assert!(recovered["connection"].get("activeRequestId").is_none());
            assert_eq!(recovered["connection"]["history"],serde_json::json!({"undoCount":2,"redoCount":0}));
            assert_eq!(recovered["document"]["parameters"],barrier["document"]["parameters"]);
            assert!(recovered["yaml"].as_str().unwrap().contains(CONTENT_SHA256));
            assert_eq!(javascript(&mut plugin, view, "String(window.__RECOVERY_CALLS__.some(([method])=>['prepareInstrumentSessionPatch','prepareSampler'].includes(method)))"), "false");
            return_audio(&mut plugin,audio.take().unwrap());
            assert_import_audio(&mut plugin,true);
            for (action,revision,selected,undo,redo) in [("Undo","3",false,1,1),("Redo","4",true,2,0)] {
                let selector=serde_json::to_string(&format!("header button[aria-label^=\"{action} \"]:not(:disabled)")).unwrap();
                wait_dom(&mut plugin,view,&format!("document.querySelector({selector})"));
                javascript(&mut plugin,view,&format!("document.querySelector({selector}).click() || 'clicked'"));
                let deadline=Instant::now()+Duration::from_secs(5);
                loop {
                    let current=structural_native::snapshot(&mut plugin,view);
                    if current["connection"]["revision"]==revision {
                        assert_eq!(current["yaml"].as_str().unwrap().contains(CONTENT_SHA256),selected);
                        assert_eq!(current["connection"]["history"],serde_json::json!({"undoCount":undo,"redoCount":redo}));break;
                    }
                    assert!(Instant::now()<deadline,"recovered native history completes once");std::thread::sleep(Duration::from_millis(5));
                }
                assert_import_audio(&mut plugin,selected);
            }
            assert_eq!(saved_state(&mut independent),independent_state);
            assert_packaged_audio(&mut independent);
        }));
        if let Some(audio) = audio.take() {return_audio(&mut plugin,audio);}
        if browser.is_some() {plugin.plugin_handle().get_extension::<PluginGui>().unwrap().destroy(&plugin.plugin_handle());drop(browser.take());}
        assert!(parent.children().is_empty());
        outcome.unwrap();
    }).join().expect("native selected-import recovery host completes");
}
