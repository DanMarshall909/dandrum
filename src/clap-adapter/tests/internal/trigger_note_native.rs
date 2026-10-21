//! Real copied-product note acceptance; native time and signed composed-sample output.
use super::*;

const SAMPLE_FRAMES: usize = 480_000;
const BLOCK_FRAMES: usize = 256;

// Only copied instrument resources change. The production app and transport stay intact.
struct HeldSample {
    root: PathBuf,
    document: Vec<u8>,
    sample: Vec<u8>,
}
impl HeldSample {
    fn install() -> Self {
        use dandrum_trigger_host::{ADVANCED_DRUM_KIT_DOCUMENT, ADVANCED_DRUM_KIT_SAMPLE};
        let root = artifact()
            .parent()
            .unwrap()
            .join("DandrumTrigger.resources");
        let fixture = Self {
            document: std::fs::read(root.join(ADVANCED_DRUM_KIT_DOCUMENT)).unwrap(),
            sample: std::fs::read(root.join(ADVANCED_DRUM_KIT_SAMPLE)).unwrap(),
            root,
        };
        let mut samples = vec![-0.125; SAMPLE_FRAMES];
        samples[..4].copy_from_slice(&[-0.5, 0.25, -0.25, 0.5]);
        dandrum_engine::wav::write_wav_stereo_i16(
            std::fs::File::create(fixture.root.join(ADVANCED_DRUM_KIT_SAMPLE)).unwrap(),
            48_000,
            &samples,
            &samples,
        )
        .unwrap();
        let document = r#"
instrument: { id: dandrum.advanced-drum-kit, preset_schema_version: 1 }
assets:
  sample_sources:
    - id: drums
      path: assets/advanced-drums.wav
      regions:
        - { id: body, start_frame: 0, end_frame: 480000 }
        - { id: click, start_frame: 0, end_frame: 1 }
  sample_maps:
    - id: kit
      zones:
        - { id: kick, region: drums.click, key_range: [36, 36], velocity_range: [1, 127] }
ports:
  - { name: level, direction: input, signal: control, channels: 1, default: 1, min: 0, max: 4, maps_to: gain.gain }
  - { name: master, direction: output, signal: audio, channels: 2, maps_from: gain.audio_out }
preset_surface:
  parameters:
    - { name: drums.level, maps_to: level }
module_definitions:
  - type: sample_voice
    ports:
      - { name: audio, direction: output, signal: audio, channels: 2, maps_from: player.audio }
    modules:
      - { id: player, type: sample_player, static: { source: drums, region: body, channels: 2, mode: gated }, defaults: { pan: 0 } }
    connections:
      - { from: voice.gate, to: player.trigger }
      - { from: voice.gate, to: player.gate }
      - { from: voice.velocity, to: player.level }
modules:
  - { id: midi, type: midi_input }
  - { id: voices, type: poly, static: { definition: sample_voice, max_voices: 8, allocation: reject-new } }
  - { id: mapped, type: sample_map_player, static: { sample_map: kit, channels: 2, max_voices: 8 } }
  - { id: mixer, type: audio_mixer, static: { channels: 2 } }
  - { id: gain, type: gain, static: { channels: 2 } }
connections:
  - { from: midi.events, to: voices.notes }
  - { from: midi.events, to: mapped.note }
  - { from: voices.audio, to: mixer.inputs }
  - { from: mapped.audio, to: mixer.inputs }
  - { from: mixer.mix, to: gain.audio_in }
"#;
        std::fs::write(fixture.root.join(ADVANCED_DRUM_KIT_DOCUMENT), document).unwrap();
        fixture
    }
}
impl Drop for HeldSample {
    fn drop(&mut self) {
        use dandrum_trigger_host::{ADVANCED_DRUM_KIT_DOCUMENT, ADVANCED_DRUM_KIT_SAMPLE};
        std::fs::write(self.root.join(ADVANCED_DRUM_KIT_DOCUMENT), &self.document).unwrap();
        std::fs::write(self.root.join(ADVANCED_DRUM_KIT_SAMPLE), &self.sample).unwrap();
    }
}

struct HeldAudio {
    audio: StartedPluginAudioProcessor<GuiHost>,
    frames: usize,
    output: Recorder,
}
impl HeldAudio {
    fn start(plugin: &mut PluginInstance<GuiHost>) -> Self {
        let stopped = plugin
            .activate(
                |_, _| (),
                PluginAudioConfiguration {
                    sample_rate: 48_000.0,
                    min_frames_count: 1,
                    max_frames_count: BLOCK_FRAMES as u32,
                },
            )
            .unwrap();
        Self {
            audio: std::thread::spawn(move || stopped.start_processing().unwrap())
                .join()
                .unwrap(),
            frames: 0,
            output: Recorder::new(64),
        }
    }
    fn render(&mut self, note: Option<[u8; 3]>) -> [[f32; BLOCK_FRAMES]; 2] {
        // Independent upper bound, including all blocks since activation: no
        // played voice can reach the sample's natural end during this scenario.
        self.frames += BLOCK_FRAMES;
        assert!(self.frames < SAMPLE_FRAMES / 2, "fixture cannot exhaust");
        let mut pcm = [[123.0; BLOCK_FRAMES]; 2];
        std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let mut events = EventBuffer::new();
                    if let Some(note) = note {
                        events.push(&MidiEvent::new(0, 0, note));
                    }
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
                    self.audio
                        .process(
                            &input,
                            &mut buffers,
                            &events.as_input(),
                            &mut OutputEvents::from_buffer(&mut self.output),
                            None,
                            None,
                        )
                        .unwrap();
                })
                .join()
                .unwrap();
        });
        pcm
    }
    fn until(&mut self, deadline: Instant, expected: f32) {
        // The host's bounded GLib dispatch is the only main-side service here:
        // no PluginTimer or on_main_thread calls, browser calls or reset.
        while Instant::now() < deadline {
            assert_eq!(self.service_and_render(), [[expected; BLOCK_FRAMES]; 2]);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn service_and_render(&mut self) -> [[f32; BLOCK_FRAMES]; 2] {
        let context = gtk::glib::MainContext::default();
        for _ in 0..64 {
            if !context.pending() {
                break;
            }
            context.iteration(false);
        }
        self.render(None)
    }
    fn finish(self, plugin: &mut PluginInstance<GuiHost>) {
        eprintln!(
            "held sample rendered {} of {SAMPLE_FRAMES} frames",
            self.frames
        );
        return_audio(plugin, self.audio);
    }
}

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
    // Observe only note traffic; all calls still reach the real authenticated
    // broker. Direct backend presses do not enter NativeLive's audition map, so
    // its automatic timer has no held note to renew. No timer is replaced.
    javascript(
        plugin,
        &browser,
        r#"(()=>{window.__NOTE_CALLS__=[];const backend=window.__DANDRUM__.backend,get=backend.getNativeFunction;backend.getNativeFunction=method=>{const call=get(method);return (...args)=>{if(['noteOn','noteOff','noteHeartbeat'].includes(method))window.__NOTE_CALLS__.push(method);return call(...args);};};return 'observing';})()"#,
    );
    browser
}
fn command(
    plugin: &mut PluginInstance<GuiHost>,
    browser: &webkit2gtk::WebView,
    method: &str,
    args: &str,
) -> serde_json::Value {
    invoke(
        plugin,
        browser,
        &format!("window.__DANDRUM__.backend.getNativeFunction('{method}')({args})"),
    )
}
fn close_view(plugin: &mut PluginInstance<GuiHost>, browser: webkit2gtk::WebView) {
    plugin
        .plugin_handle()
        .get_extension::<PluginGui>()
        .unwrap()
        .destroy(&plugin.plugin_handle());
    drop(browser);
}

#[test]
#[ignore = "native destruction boundary: inherited X11, copied release and unmodified production app"]
fn copied_native_destroy_releases_held_activity_once_and_reconnects() {
    std::thread::spawn(|| {
        let _fixture = HeldSample::install();
        let parent = Parent::new();
        let mut plugin = copied_plugin();
        // This fixture declares exactly one public alias, unlike the full kit.
        let params = plugin
            .plugin_handle()
            .get_extension::<PluginParams>()
            .unwrap();
        assert_eq!(params.count(&plugin.plugin_handle()), 1);
        let mut info_buffer = clack_extensions::params::ParamInfoBuffer::new();
        let info = params
            .get_info(&plugin.plugin_handle(), 0, &mut info_buffer)
            .unwrap();
        assert_eq!(info.id, ClapId::new(1));
        assert_eq!(info.name, b"drums.level");
        let browser = open_view(&mut plugin, &parent);
        let child = parent.children()[0];
        let weak_browser = weak_browser_in_child(child);
        let attachment = javascript(
            &mut plugin,
            &browser,
            "window.__DANDRUM_NATIVE__.attachmentId",
        );
        let mut audio = HeldAudio::start(&mut plugin);
        let held_since = Instant::now();
        for (method, args) in [
            ("beginGesture", "'drums.level',1"),
            ("setParameter", "'drums.level',0.125,1"),
            ("noteOn", "60,1,1"),
        ] {
            assert_eq!(
                command(&mut plugin, &browser, method, args)["status"],
                "accepted"
            );
        }
        plugin.call_on_main_thread_callback();
        flush_audio(&mut audio.audio, &EventBuffer::new(), &mut audio.output);
        let open_gesture = [Some(Recorded::Begin(1)), Some(Recorded::Value(1, 0.5))];
        assert_eq!(&audio.output.accepted[..audio.output.len], &open_gesture);
        // Literal PCM16 input, multiplied by physical gain 0.5. Normalized
        // drums.level 0.125 spans physical [0,4]; neither audio path is an oracle.
        for channel in audio.render(None) {
            assert_eq!(
                &channel[..4],
                &[-0.25, 8191.0 / 65536.0, -0.125, 16383.0 / 65536.0]
            );
        }
        assert_eq!(audio.render(None), [[-0.0625; BLOCK_FRAMES]; 2]);
        let host_onset = audio.render(Some([0x90, 61, 127]));
        assert_eq!([host_onset[0][0], host_onset[1][0]], [-0.3125; 2]);
        assert_eq!(audio.render(None), [[-0.125; BLOCK_FRAMES]; 2]);
        assert_eq!(&audio.output.accepted[..audio.output.len], &open_gesture);

        // No noteOff or endGesture is sent: complete native destruction owns
        // both cleanup obligations. Release the observer's strong reference too.
        close_view(&mut plugin, browser);
        assert!(parent.children().is_empty());
        assert!(parent.x.get_geometry(child).unwrap().reply().is_err());
        assert!(
            weak_browser.upgrade().is_none(),
            "destroy releases the actual WebView"
        );
        flush_audio(&mut audio.audio, &EventBuffer::new(), &mut audio.output);
        let closed_gesture = [
            Some(Recorded::Begin(1)),
            Some(Recorded::Value(1, 0.5)),
            Some(Recorded::End(1)),
        ];
        assert_eq!(&audio.output.accepted[..audio.output.len], &closed_gesture);
        assert_eq!(
            audio.render(None),
            [[-0.0625; BLOCK_FRAMES]; 2],
            "only the editor note is released"
        );
        assert!(
            held_since.elapsed() < Duration::from_millis(1800),
            "destruction cleanup must precede the independent two-second note expiry"
        );
        let gui = plugin.plugin_handle().get_extension::<PluginGui>().unwrap();
        gui.destroy(&plugin.plugin_handle());
        // Bounded host-loop progress proves no additional accepted automation
        // here; synchronous application-handler release has a separate owner test.
        audio.until(Instant::now() + Duration::from_millis(100), -0.0625);
        plugin.call_on_main_thread_callback();
        flush_audio(&mut audio.audio, &EventBuffer::new(), &mut audio.output);
        assert_eq!(audio.render(Some([0x80, 61, 0])), [[0.0; BLOCK_FRAMES]; 2]);
        assert_eq!(&audio.output.accepted[..audio.output.len], &closed_gesture);
        assert_eq!(audio.output.attempts, 3, "no duplicate End is even offered");
        assert!(parent.children().is_empty());
        assert!(weak_browser.upgrade().is_none());

        let browser = open_view(&mut plugin, &parent);
        let fresh_attachment = javascript(
            &mut plugin,
            &browser,
            "window.__DANDRUM_NATIVE__.attachmentId",
        );
        assert_ne!(fresh_attachment, attachment);
        let session = command(&mut plugin, &browser, "getSamplerSession", "");
        assert_eq!(session["status"], "accepted");
        assert_eq!(
            session["result"]["connection"]["attachmentId"],
            fresh_attachment
        );
        let params = plugin
            .plugin_handle()
            .get_extension::<PluginParams>()
            .unwrap();
        assert_eq!(
            params.get_value(&plugin.plugin_handle(), ClapId::new(1)),
            Some(0.5)
        );
        assert_eq!(
            audio.render(None),
            [[0.0; BLOCK_FRAMES]; 2],
            "reconnect cannot replay the old press"
        );
        assert_eq!(
            command(&mut plugin, &browser, "noteOn", "60,1,1")["status"],
            "accepted"
        );
        let onset = audio.render(None);
        assert_eq!([onset[0][0], onset[1][0]], [-0.25; 2]);
        assert_eq!(audio.render(None), [[-0.0625; BLOCK_FRAMES]; 2]);
        assert_eq!(
            command(&mut plugin, &browser, "noteOff", "60,1")["status"],
            "accepted"
        );
        assert_eq!(audio.render(None), [[0.0; BLOCK_FRAMES]; 2]);
        for (method, args) in [
            ("beginGesture", "'drums.level',1"),
            ("setParameter", "'drums.level',0.375,1"),
            ("endGesture", "'drums.level',1"),
        ] {
            assert_eq!(
                command(&mut plugin, &browser, method, args)["status"],
                "accepted"
            );
        }
        plugin.call_on_main_thread_callback();
        flush_audio(&mut audio.audio, &EventBuffer::new(), &mut audio.output);
        let both_gestures = [
            Some(Recorded::Begin(1)),
            Some(Recorded::Value(1, 0.5)),
            Some(Recorded::End(1)),
            Some(Recorded::Begin(1)),
            Some(Recorded::Value(1, 1.5)),
            Some(Recorded::End(1)),
        ];
        assert_eq!(&audio.output.accepted[..audio.output.len], &both_gestures);
        assert_eq!(
            params.get_value(&plugin.plugin_handle(), ClapId::new(1)),
            Some(1.5)
        );
        close_view(&mut plugin, browser);
        gui.destroy(&plugin.plugin_handle());
        audio.until(Instant::now() + Duration::from_millis(100), 0.0);
        plugin.call_on_main_thread_callback();
        flush_audio(&mut audio.audio, &EventBuffer::new(), &mut audio.output);
        assert_eq!(&audio.output.accepted[..audio.output.len], &both_gestures);
        assert_eq!(audio.output.attempts, 6);
        eprintln!(
            "destroyed attachment {attachment}; fresh attachment {fresh_attachment}; accepted CLAP events {:?}; attempts {}",
            &audio.output.accepted[..audio.output.len], audio.output.attempts
        );
        audio.finish(&mut plugin);
    })
    .join()
    .expect("native held-activity destruction and reconnect completed");
}

#[test]
#[ignore = "native note boundary: inherited X11, copied release and unmodified production app"]
fn copied_native_gated_sample_outlasts_short_fixture_and_releases() {
    std::thread::spawn(|| {
        let _fixture = HeldSample::install();
        let parent = Parent::new();
        let mut plugin = copied_plugin();
        let browser = open_view(&mut plugin, &parent);
        let mut audio = HeldAudio::start(&mut plugin);
        assert_eq!(
            command(&mut plugin, &browser, "noteOn", "60,1,1")["status"],
            "accepted"
        );
        let onset = audio.render(None);
        for channel in onset {
            // PCM16 positive samples use 32767 scaling on write, then /32768 on read.
            assert_eq!(
                &channel[..4],
                &[-0.5, 8191.0 / 32768.0, -0.25, 16383.0 / 32768.0]
            );
        }
        audio.until(Instant::now() + Duration::from_millis(600), -0.125);
        assert!(
            audio.frames > 16_384,
            "held output exceeds the earlier short sample duration"
        );
        assert_eq!(
            command(&mut plugin, &browser, "noteOff", "60,1")["status"],
            "accepted"
        );
        assert_eq!(audio.render(None), [[0.0; BLOCK_FRAMES]; 2]);
        assert_eq!(
            javascript(
                &mut plugin,
                &browser,
                "JSON.stringify(window.__NOTE_CALLS__)"
            ),
            r#"["noteOn","noteOff"]"#
        );
        audio.finish(&mut plugin);
        close_view(&mut plugin, browser);
    })
    .join()
    .expect("native gated sample calibration completed");
}

#[test]
#[ignore = "native note boundary: inherited X11, copied release and unmodified production app"]
fn copied_native_heartbeat_renews_held_note_past_original_deadline() {
    std::thread::spawn(|| {
        let _fixture = HeldSample::install();
        let parent = Parent::new();
        let mut plugin = copied_plugin();
        let browser = open_view(&mut plugin, &parent);
        let mut audio = HeldAudio::start(&mut plugin);
        assert_eq!(
            command(&mut plugin, &browser, "noteOn", "60,1,1")["status"],
            "accepted"
        );
        let pressed = Instant::now();
        assert_eq!(audio.render(None)[0][0], -0.5);
        audio.until(pressed + Duration::from_millis(1100), -0.125);
        let heartbeat = command(&mut plugin, &browser, "noteHeartbeat", "");
        if heartbeat["status"] != "accepted" {
            audio.finish(&mut plugin);
            close_view(&mut plugin, browser);
            assert_eq!(
                heartbeat["status"], "accepted",
                "N1: authenticated heartbeat must renew native note activity: {heartbeat}"
            );
            unreachable!();
        }
        assert!(heartbeat["result"].is_null());
        let renewed = Instant::now();
        audio.until(pressed + Duration::from_millis(2300), -0.125);
        assert!(
            renewed.elapsed() < Duration::from_millis(1800),
            "scheduling must leave renewal margin"
        );
        // Unknown release is an authenticated no-op, but first sweeps native
        // expiry. Without renewal it cancels pitch60 even before the periodic
        // service is joined; it never releases or renews that held pitch itself.
        assert_eq!(
            command(&mut plugin, &browser, "noteOff", "62,1")["status"],
            "accepted"
        );
        assert_eq!(audio.render(None), [[-0.125; BLOCK_FRAMES]; 2]);
        assert_eq!(
            command(&mut plugin, &browser, "noteOff", "60,1")["status"],
            "accepted"
        );
        assert_eq!(audio.render(None), [[0.0; BLOCK_FRAMES]; 2]);
        assert_eq!(
            javascript(
                &mut plugin,
                &browser,
                "JSON.stringify(window.__NOTE_CALLS__)"
            ),
            r#"["noteOn","noteHeartbeat","noteOff","noteOff"]"#
        );
        audio.finish(&mut plugin);
        close_view(&mut plugin, browser);
    })
    .join()
    .expect("native heartbeat renewal completed");
}

#[test]
#[ignore = "native note boundary: inherited X11, copied release and unmodified production app"]
fn copied_native_live_attachment_expires_renewed_note_and_preserves_host_voice() {
    std::thread::spawn(|| {
        let _fixture = HeldSample::install();
        let parent = Parent::new();
        let mut plugin = copied_plugin();
        let browser = open_view(&mut plugin, &parent);
        let child = parent.children();
        let attachment = javascript(&mut plugin, &browser, "window.__DANDRUM_NATIVE__.attachmentId");
        let mut audio = HeldAudio::start(&mut plugin);
        assert_eq!(command(&mut plugin, &browser, "noteOn", "60,1,1")["status"], "accepted");
        let onset = audio.render(None);
        assert_eq!([onset[0][0], onset[1][0]], [-0.5, -0.5]);
        // An ordinary host MIDI note owns a different sustained voice. Expiry
        // must remove only the editor voice, not reset the graph or all notes.
        let host_onset = audio.render(Some([0x90,61,127]));
        assert_eq!([host_onset[0][0],host_onset[1][0]], [-0.625,-0.625]);
        audio.until(Instant::now()+Duration::from_millis(1100), -0.25);
        let renewal_before = Instant::now();
        assert_eq!(command(&mut plugin, &browser, "noteHeartbeat", "")["status"], "accepted");
        let renewal_after = Instant::now();
        assert!(renewal_after.duration_since(renewal_before)<Duration::from_millis(200), "heartbeat reply must leave a known native scheduling margin");
        // Later than the original two-second deadline, yet safely before the
        // renewed deadline: both literal signed tails must still be present.
        audio.until(renewal_after+Duration::from_millis(1700), -0.25);
        let late = audio.render(None);
        assert_eq!(late, [[-0.25; BLOCK_FRAMES]; 2]);
        while Instant::now()<renewal_after+Duration::from_millis(2300) {
            let pcm = audio.service_and_render();
            assert!(pcm==[[-0.25;BLOCK_FRAMES];2] || pcm==[[-0.125;BLOCK_FRAMES];2], "only the editor voice may disappear during the expiry window");
            std::thread::sleep(Duration::from_millis(5));
        }
        // Observe audio BEFORE any subsequent browser or CLAP main callback.
        // Only the existing host-dispatched GLib source could have expired it.
        let after_expiry = audio.render(None);
        let after_host_release = audio.render(Some([0x80,61,0]));
        let still_attached = javascript(&mut plugin, &browser, "window.__DANDRUM_NATIVE__.attachmentId");
        let calls = javascript(&mut plugin, &browser, "JSON.stringify(window.__NOTE_CALLS__)");
        assert_eq!(parent.children(),child);
        assert_eq!(still_attached,attachment);
        let session = command(&mut plugin, &browser, "getSamplerSession", "");
        assert_eq!(session["status"], "accepted", "expiry keeps the native attachment authenticated");
        assert_eq!(session["result"]["connection"]["attachmentId"], attachment);
        assert_eq!(calls,r#"["noteOn","noteHeartbeat"]"#, "no automatic heartbeat, browser release or detachment drives expiry");
        eprintln!("native expiry observed at {:?} after heartbeat; late={}, remaining={}, after host release={}",renewal_after.elapsed(),late[0][0],after_expiry[0][0],after_host_release[0][0]);
        audio.finish(&mut plugin);
        close_view(&mut plugin,browser);
        assert_eq!(after_expiry,[[-0.125;BLOCK_FRAMES];2],"N2: live attachment's editor note must expire through bounded GLib service while host MIDI remains held");
        assert_eq!(after_host_release,[[0.0;BLOCK_FRAMES];2],"host release leaves silence after native editor expiry");
    }).join().expect("native live-attachment expiry completed");
}
