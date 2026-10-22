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
