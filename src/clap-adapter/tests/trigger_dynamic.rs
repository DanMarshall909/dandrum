//! The real Trigger product is tested only through an explicit copied artifact.
#![cfg(feature = "trigger")]
use clack_host::{entry::PluginEntry, factory::plugin::PluginFactory};
use std::path::PathBuf;
fn artifact() -> PathBuf {
    std::env::var_os("DANDRUM_TRIGGER_CLAP_PLUGIN_PATH")
        .map(PathBuf::from)
        .expect("set DANDRUM_TRIGGER_CLAP_PLUGIN_PATH to the copied Trigger product")
}
#[test]
fn optional_product_discovers_stable_trigger_and_retains_fixture_identities() {
    // SAFETY: the explicit build artifact is trusted and the entry outlives its factory.
    let entry = unsafe { PluginEntry::load(artifact()) }.unwrap();
    let factory = entry.get_factory::<PluginFactory>().unwrap();
    assert_eq!(
        factory.plugin_count(),
        3,
        "Trigger is the third opt-in product"
    );
    let ids: Vec<_> = factory
        .plugin_descriptors()
        .map(|d| d.id().unwrap().to_bytes().to_vec())
        .collect();
    assert_eq!(
        ids,
        [
            b"org.dandrum.experimental.tb303".to_vec(),
            b"org.dandrum.experimental.signed-gain".to_vec(),
            b"org.dandrum.trigger".to_vec()
        ]
    );
    let descriptor = factory.plugin_descriptors().nth(2).unwrap();
    assert_eq!(descriptor.name().unwrap().to_bytes(), b"Dandrum Trigger");
    assert_eq!(descriptor.vendor().unwrap().to_bytes(), b"Dandrum");
    assert_eq!(descriptor.version().unwrap().to_bytes(), b"0.1.0");
    let features: Vec<_> = descriptor.features().map(|f| f.to_bytes()).collect();
    assert_eq!(features, [b"instrument".as_slice(), b"sampler", b"stereo"]);
}

use clack_extensions::params::{ParamInfoBuffer, PluginParams};
use clack_host::prelude::*;
struct TestHost;
#[derive(Default)]
struct HostShared {
    callbacks: std::sync::atomic::AtomicUsize,
    logs: std::sync::atomic::AtomicU8,
    omit_extensions: bool,
}
impl SharedHandler<'_> for HostShared {
    fn request_restart(&self) {}
    fn request_process(&self) {}
    fn request_callback(&self) {
        self.callbacks
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}
impl clack_extensions::log::HostLogImpl for HostShared {
    fn log(&self, severity: clack_extensions::log::LogSeverity, message: &str) {
        let code = match message {
            "DANDRUM_TRIGGER_INVALID_PROCESS" => 1,
            "DANDRUM_TRIGGER_INVALID_FLUSH" => 2,
            _ => 8,
        };
        let code = if code != 8 && severity != clack_extensions::log::LogSeverity::Warning {
            64
        } else {
            code
        };
        self.logs
            .fetch_or(code, std::sync::atomic::Ordering::Relaxed);
    }
}
#[derive(Default)]
struct HostMain {
    rescans: std::cell::Cell<usize>,
}
impl MainThreadHandler<'_> for HostMain {}
impl clack_extensions::params::HostParamsImplShared for HostShared {
    fn request_flush(&self) {}
}
impl clack_extensions::params::HostParamsImplMainThread for HostMain {
    fn rescan(&self, flags: clack_extensions::params::ParamRescanFlags) {
        assert_eq!(flags, clack_extensions::params::ParamRescanFlags::VALUES);
        self.rescans.set(self.rescans.get() + 1);
    }
    fn clear(&self, _: ClapId, _: clack_extensions::params::ParamClearFlags) {
        panic!("identities stay fixed");
    }
}
impl HostHandlers for TestHost {
    type Shared<'a> = HostShared;
    type MainThread<'a> = HostMain;
    type AudioProcessor<'a> = ();
    fn declare_extensions(
        builder: &mut clack_host::host::HostExtensions<Self>,
        shared: &HostShared,
    ) {
        if shared.omit_extensions {
            return;
        }
        builder
            .register::<clack_extensions::log::HostLog>()
            .register::<clack_extensions::params::HostParams>();
    }
}
fn load_at(
    path: &std::path::Path,
) -> Result<PluginInstance<TestHost>, clack_host::plugin::PluginInstanceError> {
    load_with_options(path, false)
}
fn load_with_options(
    path: &std::path::Path,
    omit_extensions: bool,
) -> Result<PluginInstance<TestHost>, clack_host::plugin::PluginInstanceError> {
    // SAFETY: locally-built trusted artifact; PluginInstance retains its entry.
    let entry = {
        use std::os::unix::ffi::OsStrExt;
        let entry_path = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        // The pinned host convenience loader uses to_string_lossy; the explicit
        // provider preserves the OS-byte path required by the actual CLAP entry.
        let library = unsafe { clack_host::entry::LibraryEntry::load_from_path(path) }.unwrap();
        unsafe { PluginEntry::load_from(library, &entry_path) }.unwrap()
    };
    PluginInstance::new(
        move |_| HostShared {
            omit_extensions,
            ..Default::default()
        },
        |_| HostMain::default(),
        &entry,
        c"org.dandrum.trigger",
        &HostInfo::new("Trigger proof", "Dandrum", "", "0.1").unwrap(),
    )
}
#[test]
fn trigger_initializes_from_only_sibling_resources_and_exposes_frozen_physical_parameters() {
    let mut plugin = load_at(&artifact()).expect("real Trigger initializes using its copied kit");
    let handle = plugin.plugin_handle();
    let params = handle.get_extension::<PluginParams>().unwrap();
    assert_eq!(params.count(&handle), 23);
    let expected = [
        ("drums.pitch_ratio", 0.125, 8.0, 1.0),
        ("drums.start_offset", 0.0, 1.0, 0.0),
        ("drums.level", 0.0, 4.0, 1.0),
        ("drums.pan", -1.0, 1.0, 0.0),
        ("drums.variation", 0.0, 1.0, 0.0),
        ("drums.kick.pitch_ratio", 0.125, 8.0, 1.0),
        ("drums.kick.start_offset", 0.0, 1.0, 0.0),
        ("drums.kick.level", 0.0, 4.0, 1.0),
        ("drums.kick.pan", -1.0, 1.0, 0.0),
        ("drums.snare.pitch_ratio", 0.125, 8.0, 1.0),
        ("drums.snare.start_offset", 0.0, 1.0, 0.0),
        ("drums.snare.level", 0.0, 4.0, 1.0),
        ("drums.snare.pan", -1.0, 1.0, 0.0),
        ("drums.snare.variation", 0.0, 1.0, 0.0),
        ("drums.closed_hat.pitch_ratio", 0.125, 8.0, 1.0),
        ("drums.closed_hat.start_offset", 0.0, 1.0, 0.0),
        ("drums.closed_hat.level", 0.0, 4.0, 1.0),
        ("drums.closed_hat.pan", -1.0, 1.0, 0.0),
        ("drums.open_hat.pitch_ratio", 0.125, 8.0, 1.0),
        ("drums.open_hat.start_offset", 0.0, 1.0, 0.0),
        ("drums.open_hat.level", 0.0, 4.0, 1.0),
        ("drums.open_hat.pan", -1.0, 1.0, 0.0),
        ("drums.open_hat.variation", 0.0, 1.0, 0.0),
    ];
    for (index, (name, min, max, default)) in expected.iter().enumerate() {
        let mut buffer = ParamInfoBuffer::new();
        let info = params.get_info(&handle, index as u32, &mut buffer).unwrap();
        assert_eq!(info.id, Some(ClapId::new(index as u32 + 1)));
        assert_eq!(info.name, name.as_bytes());
        assert_eq!(
            (info.min_value, info.max_value, info.default_value),
            (*min, *max, *default)
        );
        assert_eq!(
            params.get_value(&handle, ClapId::new(index as u32 + 1)),
            Some(*default)
        );
        assert_eq!(
            info.flags,
            clack_extensions::params::ParamInfoFlags::IS_AUTOMATABLE
        );
    }
    assert_eq!(params.get_value(&handle, ClapId::new(0)), None);
    assert_eq!(params.get_value(&handle, ClapId::new(24)), None);
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("Missing.clap");
    std::fs::copy(artifact(), &missing).unwrap();
    assert!(
        load_at(&missing).is_err(),
        "no checkout/cwd resource fallback"
    );
}

use clack_host::events::event_types::{MidiEvent, ParamValueEvent};
use clack_host::process::StartedPluginAudioProcessor;
fn activate(
    plugin: &mut PluginInstance<TestHost>,
    rate: f64,
    maximum: u32,
) -> StartedPluginAudioProcessor<TestHost> {
    let stopped = plugin
        .activate(
            |_, _| (),
            PluginAudioConfiguration {
                sample_rate: rate,
                min_frames_count: 1,
                max_frames_count: maximum,
            },
        )
        .unwrap();
    std::thread::spawn(move || stopped.start_processing().unwrap())
        .join()
        .unwrap()
}
fn deactivate(
    plugin: &mut PluginInstance<TestHost>,
    processor: StartedPluginAudioProcessor<TestHost>,
) {
    plugin.deactivate(
        std::thread::spawn(move || processor.stop_processing())
            .join()
            .unwrap(),
    );
}
fn block<E: clack_host::events::io::InputEventBuffer + Sync>(
    processor: &mut StartedPluginAudioProcessor<TestHost>,
    frames: usize,
    events: &E,
    reset: bool,
) -> [Vec<f32>; 2] {
    let mut output = [vec![123.0; frames], vec![123.0; frames]];
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
                let mut output_ports = ports.with_output_buffers([AudioPortBuffer {
                    latency: 0,
                    channels: AudioPortBufferType::f32_output_only(
                        output.iter_mut().map(|x| x.as_mut_slice()),
                    ),
                }]);
                let mut out_events = EventBuffer::new();
                assert_eq!(
                    processor
                        .process(
                            &input,
                            &mut output_ports,
                            &InputEvents::from_buffer(events),
                            &mut out_events.as_output(),
                            None,
                            None
                        )
                        .unwrap(),
                    clack_host::process::ProcessStatus::Continue
                );
            })
            .join()
            .unwrap()
    });
    output
}
fn note(offset: u32, note: u8) -> EventBuffer {
    let mut events = EventBuffer::new();
    events.push(&MidiEvent::new(offset, 0, [0x90, note, 100]));
    events
}
#[test]
fn real_sampler_ports_and_offset_midi_parameter_values_render_literal_signed_pcm() {
    use clack_extensions::{
        audio_ports::{AudioPortInfoBuffer, PluginAudioPorts},
        note_ports::{NoteDialect, NoteDialects, NotePortInfoBuffer, PluginNotePorts},
    };
    let mut plugin = load_at(&artifact()).unwrap();
    let handle = plugin.plugin_handle();
    let audio = handle
        .get_extension::<PluginAudioPorts>()
        .expect("Trigger advertises its actual stereo output");
    assert_eq!(
        (audio.count(&handle, true), audio.count(&handle, false)),
        (0, 1)
    );
    let mut buffer = AudioPortInfoBuffer::new();
    let port = audio.get(&handle, 0, false, &mut buffer).unwrap();
    assert_eq!(port.channel_count, 2);
    assert!(
        !port
            .flags
            .contains(clack_extensions::audio_ports::AudioPortFlags::SUPPORTS_64BITS)
    );
    for (index, input) in [(1, false), (0, true)] {
        assert!(audio.get(&handle, index, input, &mut buffer).is_none());
    }
    let notes = handle.get_extension::<PluginNotePorts>().unwrap();
    assert_eq!(
        (notes.count(&handle, true), notes.count(&handle, false)),
        (1, 0)
    );
    let mut buffer = NotePortInfoBuffer::new();
    let port = notes.get(&handle, 0, true, &mut buffer).unwrap();
    assert_eq!(port.supported_dialects, NoteDialects::MIDI);
    assert_eq!(port.preferred_dialect, Some(NoteDialect::Midi));
    for (index, input) in [(1, true), (0, false)] {
        assert!(notes.get(&handle, index, input, &mut buffer).is_none());
    }
    let mut processor = activate(&mut plugin, 48000.0, 16);
    let mut events = note(3, 36);
    events.push(&ParamValueEvent::new(
        5,
        ClapId::new(3),
        Pckn::match_all(),
        0.5,
    ));
    let output = block(&mut processor, 16, &events, false);
    for channel in output {
        assert_eq!(&channel[..3], &[0.0; 3]);
        assert_eq!(
            &channel[3..7],
            &[-0.5, -358.0 / 32768.0, 1554.0 / 65536.0, 4137.0 / 65536.0]
        );
    }
    assert_eq!(
        plugin
            .plugin_handle()
            .get_extension::<PluginParams>()
            .unwrap()
            .get_value(&plugin.plugin_handle(), ClapId::new(3)),
        Some(0.5)
    );
    assert_eq!(
        block(&mut processor, 16, &note(3, 60), true),
        [vec![0.0; 16], vec![0.0; 16]]
    );
    deactivate(&mut plugin, processor);
}

#[test]
fn callback_rejects_complete_invalid_batches_without_parameter_or_dsp_progress() {
    let mut plugin = load_at(&artifact()).unwrap();
    let mut processor = activate(&mut plugin, 48000.0, 16);
    for bad in [
        ParamValueEvent::new(1, ClapId::new(24), Pckn::match_all(), 0.25),
        ParamValueEvent::new(1, ClapId::new(3), Pckn::match_all(), f64::NAN),
        ParamValueEvent::new(1, ClapId::new(3), Pckn::match_all(), 4.0001),
        ParamValueEvent::new(1, ClapId::new(3), Pckn::from_raw(0, -1, -1, -1), 0.25),
        ParamValueEvent::new(16, ClapId::new(3), Pckn::match_all(), 0.25),
    ] {
        block(&mut processor, 16, &note(0, 36), true);
        let mut events = EventBuffer::new();
        events.push(&ParamValueEvent::new(
            0,
            ClapId::new(3),
            Pckn::match_all(),
            0.5,
        ));
        events.push(&bad);
        assert_eq!(
            block(&mut processor, 16, &events, false),
            [vec![0.0; 16], vec![0.0; 16]]
        );
        let handle = plugin.plugin_handle();
        assert_eq!(
            handle
                .get_extension::<PluginParams>()
                .unwrap()
                .get_value(&handle, ClapId::new(3)),
            Some(1.0)
        );
        let continuation = block(&mut processor, 16, &EventBuffer::new(), false);
        for plane in continuation {
            assert_eq!(
                &plane[..4],
                &[
                    11869.0 / 32768.0,
                    7042.0 / 32768.0,
                    11000.0 / 32768.0,
                    9960.0 / 32768.0
                ]
            );
        }
    }
    for (offset, port, data) in [
        (0, 1, [0x90, 36, 100]),
        (0, 0, [0x90, 128, 100]),
        (0, 0, [0x90, 36, 128]),
        (16, 0, [0x90, 36, 100]),
    ] {
        let mut events = EventBuffer::new();
        events.push(&MidiEvent::new(offset, port, data));
        assert_eq!(
            block(&mut processor, 16, &events, true),
            [vec![0.0; 16], vec![0.0; 16]]
        );
        assert_eq!(
            block(&mut processor, 16, &EventBuffer::new(), false),
            [vec![0.0; 16], vec![0.0; 16]]
        );
    }
    let mut descending = EventBuffer::new();
    descending.push(&MidiEvent::new(3, 0, [0x90, 36, 100]));
    descending.push(&MidiEvent::new(2, 0, [0x90, 36, 100]));
    assert_eq!(
        block(&mut processor, 16, &descending, true),
        [vec![0.0; 16], vec![0.0; 16]]
    );
    let mut many = EventBuffer::new();
    for _ in 0..65 {
        many.push(&MidiEvent::new(0, 0, [0x90, 36, 100]));
    }
    assert_eq!(
        block(&mut processor, 16, &many, true),
        [vec![0.0; 16], vec![0.0; 16]]
    );
    // Unrelated MIDI and unsupported wellformed core messages consume the same finite budget.
    let mut ignored = EventBuffer::new();
    ignored.push(&MidiEvent::new(0, 7, [0xB0, 255, 255]));
    ignored.push(&clack_host::events::event_types::ParamModEvent::new(
        1,
        ClapId::new(3),
        Pckn::match_all(),
        100.0,
    ));
    ignored.push(&MidiEvent::new(3, 0, [0x90, 36, 100]));
    let output = block(&mut processor, 16, &ignored, true);
    assert_eq!((output[0][3], output[1][3]), (-0.5, -0.5));
    deactivate(&mut plugin, processor);
}

#[test]
fn rejected_audio_requests_main_service_and_reports_static_diagnostic_only_on_main() {
    let mut plugin = load_at(&artifact()).unwrap();
    let mut processor = activate(&mut plugin, 48000.0, 16);
    let mut bad = EventBuffer::new();
    bad.push(&ParamValueEvent::new(
        0,
        ClapId::new(24),
        Pckn::match_all(),
        0.5,
    ));
    block(&mut processor, 16, &bad, false);
    plugin.access_shared_handler(|h| {
        assert_eq!(h.callbacks.load(std::sync::atomic::Ordering::Relaxed), 1);
        assert_eq!(h.logs.load(std::sync::atomic::Ordering::Relaxed), 0);
    });
    plugin.call_on_main_thread_callback();
    plugin.access_shared_handler(|h| {
        assert_eq!(h.logs.load(std::sync::atomic::Ordering::Relaxed), 1)
    });
    plugin.call_on_main_thread_callback();
    deactivate(&mut plugin, processor);
}

fn flush_inactive(plugin: &mut PluginInstance<TestHost>, events: &EventBuffer) {
    let params = plugin
        .plugin_handle()
        .get_extension::<PluginParams>()
        .unwrap();
    params.flush(
        &mut plugin.inactive_plugin_handle().unwrap(),
        &events.as_input(),
        &mut EventBuffer::new().as_output(),
    );
}
fn flush_active(processor: &mut StartedPluginAudioProcessor<TestHost>, events: &EventBuffer) {
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let mut handle = processor.plugin_handle();
                let params = handle.get_extension::<PluginParams>().unwrap();
                params.flush_active(
                    &mut handle,
                    &events.as_input(),
                    &mut EventBuffer::new().as_output(),
                );
            })
            .join()
            .unwrap()
    });
}
fn parameter(plugin: &mut PluginInstance<TestHost>, id: u32) -> Option<f64> {
    let handle = plugin.plugin_handle();
    handle
        .get_extension::<PluginParams>()
        .unwrap()
        .get_value(&handle, ClapId::new(id))
}
#[test]
fn active_and_inactive_flush_apply_exact_global_values_without_advancing_audio_and_reject_atomically()
 {
    let mut plugin = load_at(&artifact()).unwrap();
    let exact = 0.312_345_678_901_234_5;
    let mut values = EventBuffer::new();
    values.push(&ParamValueEvent::new(
        u32::MAX,
        ClapId::new(3),
        Pckn::match_all(),
        0.25,
    ));
    values.push(&MidiEvent::new(0, 99, [0x90, 255, 255]));
    values.push(&ParamValueEvent::new(
        1,
        ClapId::new(3),
        Pckn::match_all(),
        exact,
    ));
    flush_inactive(&mut plugin, &values);
    assert_eq!(
        parameter(&mut plugin, 3),
        Some(exact),
        "flush ignores timestamps and unrelated MIDI, exposing applied exact physical bits"
    );
    let mut bad = EventBuffer::new();
    bad.push(&ParamValueEvent::new(
        0,
        ClapId::new(3),
        Pckn::match_all(),
        0.75,
    ));
    bad.push(&ParamValueEvent::new(
        1,
        ClapId::new(24),
        Pckn::match_all(),
        0.5,
    ));
    flush_inactive(&mut plugin, &bad);
    assert_eq!(parameter(&mut plugin, 3), Some(exact));
    let mut processor = activate(&mut plugin, 48000.0, 16);
    let mut value = EventBuffer::new();
    value.push(&ParamValueEvent::new(
        1000,
        ClapId::new(3),
        Pckn::match_all(),
        0.5,
    ));
    flush_active(&mut processor, &value);
    assert_eq!(parameter(&mut plugin, 3), Some(0.5));
    let output = block(&mut processor, 16, &note(3, 36), false);
    for channel in output {
        assert_eq!(
            &channel[3..7],
            &[-0.25, -358.0 / 65536.0, 1554.0 / 65536.0, 4137.0 / 65536.0]
        );
    }
    flush_active(&mut processor, &bad);
    assert_eq!(parameter(&mut plugin, 3), Some(0.5));
    let output = block(&mut processor, 16, &EventBuffer::new(), false);
    assert_eq!(
        output[0][3],
        11869.0 / 65536.0,
        "flush does not advance sample position"
    );
    plugin.call_on_main_thread_callback();
    plugin.access_shared_handler(|h| {
        assert_eq!(h.logs.load(std::sync::atomic::Ordering::Relaxed), 2)
    });
    deactivate(&mut plugin, processor);
}

fn save(plugin: &mut PluginInstance<TestHost>) -> Vec<u8> {
    let handle = plugin.plugin_handle();
    let state = handle
        .get_extension::<clack_extensions::state::PluginState>()
        .expect("Trigger advertises replacement v2 state");
    let mut bytes = Vec::new();
    state.save(&handle, &mut bytes).unwrap();
    bytes
}
fn load_state(plugin: &mut PluginInstance<TestHost>, bytes: &[u8]) -> bool {
    let handle = plugin.plugin_handle();
    handle
        .get_extension::<clack_extensions::state::PluginState>()
        .unwrap()
        .load(&handle, &mut &*bytes)
        .is_ok()
}
#[test]
fn state_restores_fresh_mapping_and_physical_values_and_invalid_tail_preserves_running_audio() {
    let mut original = load_at(&artifact()).unwrap();
    let initial = save(&mut original);
    let mut changed: serde_json::Value = serde_json::from_slice(&initial).unwrap();
    assert_eq!(changed["version"], 2);
    assert_eq!(changed["product"], "org.dandrum.trigger");
    assert_eq!(changed["resource"], "dandrum.trigger.advanced-drums.v1");
    changed["document"] = changed["document"]
        .as_str()
        .unwrap()
        .replace("key_range: [36, 36]", "key_range: [60, 60]")
        .into();
    changed["parameters"][2]["value"] = 0.5.into();
    let bytes = serde_json::to_vec(&changed).unwrap();
    assert!(load_state(&mut original, &bytes));
    let saved = save(&mut original);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&saved).unwrap(),
        changed
    );
    drop(original);
    let mut fresh = load_at(&artifact()).unwrap();
    assert!(load_state(&mut fresh, &saved));
    assert_eq!(parameter(&mut fresh, 3), Some(0.5));
    fresh.access_handler(|h| assert_eq!(h.rescans.get(), 1));
    let mut processor = activate(&mut fresh, 48000.0, 16);
    assert_eq!(
        block(&mut processor, 16, &note(3, 36), true),
        [vec![0.0; 16], vec![0.0; 16]]
    );
    let output = block(&mut processor, 16, &note(3, 60), true);
    for channel in output {
        assert_eq!(
            &channel[3..7],
            &[-0.25, -358.0 / 65536.0, 1554.0 / 65536.0, 4137.0 / 65536.0]
        );
    }
    let mut invalid = Vec::new();
    for (key, value) in [
        ("version", serde_json::json!(1)),
        ("product", serde_json::json!("foreign")),
        ("resource", serde_json::json!("unknown")),
    ] {
        let mut bad = changed.clone();
        bad[key] = value;
        invalid.push(serde_json::to_vec(&bad).unwrap());
    }
    let mut tail = changed.clone();
    tail["parameters"][0]["value"] = 2.0.into();
    tail["parameters"][22]["value"] = 2.0.into();
    invalid.push(serde_json::to_vec(&tail).unwrap());
    let mut duplicate = changed.clone();
    duplicate["parameters"][22] = duplicate["parameters"][0].clone();
    invalid.push(serde_json::to_vec(&duplicate).unwrap());
    invalid.extend([
        b"{".to_vec(),
        vec![b' '; 65537],
        saved[..saved.len() / 2].to_vec(),
    ]);
    for bad in invalid {
        assert!(!load_state(&mut fresh, &bad));
        assert_eq!(save(&mut fresh), saved);
        assert_eq!(parameter(&mut fresh, 3), Some(0.5));
        let output = block(&mut processor, 16, &note(3, 60), true);
        assert_eq!((output[0][3], output[1][3]), (-0.25, -0.25));
    }
    assert!(load_state(&mut fresh, &initial));
    let output = block(&mut processor, 16, &note(3, 36), true);
    assert_eq!((output[0][3], output[1][3]), (-0.5, -0.5));
    deactivate(&mut fresh, processor);
}

#[test]
fn state_streams_support_short_io_reentrant_value_reads_and_fail_without_partial_publication() {
    use std::io::{Read, Write};
    struct Writer<F> {
        bytes: Vec<u8>,
        check: F,
        fail: bool,
    }
    impl<F: FnMut()> Write for Writer<F> {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            (self.check)();
            if self.fail {
                return Err(std::io::ErrorKind::Other.into());
            }
            let n = data.len().min(7);
            self.bytes.extend_from_slice(&data[..n]);
            Ok(n)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    struct Reader<'a, F> {
        bytes: &'a [u8],
        check: F,
        fail: bool,
    }
    impl<F: FnMut()> Read for Reader<'_, F> {
        fn read(&mut self, target: &mut [u8]) -> std::io::Result<usize> {
            (self.check)();
            if self.fail {
                return Err(std::io::ErrorKind::Other.into());
            }
            let n = target.len().min(self.bytes.len()).min(5);
            target[..n].copy_from_slice(&self.bytes[..n]);
            self.bytes = &self.bytes[n..];
            Ok(n)
        }
    }
    let mut plugin = load_at(&artifact()).unwrap();
    let baseline = save(&mut plugin);
    {
        let handle = plugin.plugin_handle();
        let params = handle.get_extension::<PluginParams>().unwrap();
        let state = handle
            .get_extension::<clack_extensions::state::PluginState>()
            .unwrap();
        let check = || {
            assert_eq!(
                params.get_value(&handle, ClapId::new(3)),
                Some(1.0),
                "foreign stream callback may reenter main reads"
            )
        };
        let mut writer = Writer {
            bytes: Vec::new(),
            check,
            fail: false,
        };
        state.save(&handle, &mut writer).unwrap();
        assert_eq!(writer.bytes, baseline);
        let mut reader = Reader {
            bytes: baseline.as_slice(),
            check,
            fail: false,
        };
        state.load(&handle, &mut reader).unwrap();
        let mut writer = Writer {
            bytes: Vec::new(),
            check,
            fail: true,
        };
        assert!(state.save(&handle, &mut writer).is_err());
        let mut reader = Reader {
            bytes: baseline.as_slice(),
            check,
            fail: true,
        };
        assert!(state.load(&handle, &mut reader).is_err());
    }
    assert_eq!(save(&mut plugin), baseline);
}

fn copied_product() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let plugin = directory.path().join("DandrumTrigger.clap");
    std::fs::copy(artifact(), &plugin).unwrap();
    let source = artifact()
        .parent()
        .unwrap()
        .join("DandrumTrigger.resources");
    let target = directory.path().join("DandrumTrigger.resources");
    std::fs::create_dir_all(target.join("instrument/assets")).unwrap();
    for relative in [
        "instrument/advanced-drum-kit.yaml",
        "instrument/assets/advanced-drums.wav",
    ] {
        std::fs::copy(source.join(relative), target.join(relative)).unwrap();
    }
    (directory, plugin)
}
#[test]
fn persistent_endpoints_survive_failed_and_equal_reactivation_then_reconfigure_independently() {
    let (directory, path) = copied_product();
    let mut first = load_at(&path).unwrap();
    let mut second = load_at(&artifact()).unwrap();
    let mut a = activate(&mut first, 48000.0, 16);
    let mut b = activate(&mut second, 48000.0, 16);
    block(&mut a, 16, &note(0, 36), false);
    block(&mut b, 16, &note(0, 36), false);
    deactivate(&mut first, a);
    let accepted = save(&mut first);
    for (rate, min, max) in [
        (f64::NAN, 1, 16),
        (f64::INFINITY, 1, 16),
        (0.0, 1, 16),
        (-1.0, 1, 16),
        (0.5, 1, 16),
        (f64::from(u32::MAX) + 1.0, 1, 16),
        (48000.0, 0, 16),
        (48000.0, 17, 16),
        (48000.0, 1, 16385),
    ] {
        let handle = first.plugin_handle();
        // SAFETY: this initialized, inactive plugin remains alive. Deliberately
        // invalid configurations exercise the C ABI (the typed host panics on min=0).
        assert!(!unsafe {
            (handle.as_raw().activate.unwrap())(handle.as_raw_ptr(), rate, min, max)
        });
        assert_eq!(save(&mut first), accepted);
    }
    let sample = directory
        .path()
        .join("DandrumTrigger.resources/instrument/assets/advanced-drums.wav");
    let retained = std::fs::read(&sample).unwrap();
    std::fs::remove_file(&sample).unwrap();
    assert!(
        first
            .activate(
                |_, _| (),
                PluginAudioConfiguration {
                    sample_rate: 44100.0,
                    min_frames_count: 1,
                    max_frames_count: 32
                }
            )
            .is_err()
    );
    let mut a = activate(&mut first, 48000.0, 16);
    for plane in block(&mut a, 16, &EventBuffer::new(), false) {
        assert_eq!(
            &plane[..4],
            &[
                11869.0 / 32768.0,
                7042.0 / 32768.0,
                11000.0 / 32768.0,
                9960.0 / 32768.0
            ],
            "same returned runtime continues after preparation failure and missing resource"
        );
    }
    deactivate(&mut first, a);
    std::fs::write(sample, retained).unwrap();
    let mut a = activate(&mut first, 44100.0, 32);
    assert_eq!(
        block(&mut a, 16, &EventBuffer::new(), false),
        [vec![0.0; 16], vec![0.0; 16]],
        "changed configuration replaces voices off audio"
    );
    let output = block(&mut a, 16, &note(3, 36), true);
    let expected = [
        -0.5,
        (-358.0 * 134.0 + 1554.0 * 13.0) / (147.0 * 32768.0),
        (1554.0 * 121.0 + 4137.0 * 26.0) / (147.0 * 32768.0),
    ];
    for plane in output {
        assert_eq!(&plane[..3], &[0.0; 3]);
        for (actual, expected) in plane[3..].iter().zip(expected) {
            assert!(
                (*actual - expected).abs() < 1e-6,
                "44.1k interpolation actual{actual} expected{expected}"
            );
        }
    }
    assert_eq!(save(&mut first), accepted);
    for plane in block(&mut b, 16, &EventBuffer::new(), false) {
        assert_eq!(
            &plane[..4],
            &[
                11869.0 / 32768.0,
                7042.0 / 32768.0,
                11000.0 / 32768.0,
                9960.0 / 32768.0
            ]
        );
    }
    assert_eq!(
        block(&mut a, 16, &EventBuffer::new(), true),
        [vec![0.0; 16], vec![0.0; 16]]
    );
    deactivate(&mut first, a);
    deactivate(&mut second, b);
}
#[test]
fn physical_text_conversion_and_unknown_metadata_indices_are_explicit() {
    let mut plugin = load_at(&artifact()).unwrap();
    let before = save(&mut plugin);
    let handle = plugin.plugin_handle();
    let params = handle.get_extension::<PluginParams>().unwrap();
    let mut text = [0; 32];
    assert_eq!(
        params
            .value_to_text(&handle, ClapId::new(3), 0.3125, &mut text)
            .unwrap(),
        b"0.3125"
    );
    assert!(
        params
            .value_to_text(&handle, ClapId::new(24), 0.3, &mut text)
            .is_err()
    );
    let mut too_short = [0; 1];
    assert!(
        params
            .value_to_text(&handle, ClapId::new(3), 0.3125, &mut too_short)
            .is_err()
    );
    for (id, value, expected) in [
        (3, c"0.3125", Some(0.3125)),
        (0, c"0.3125", None),
        (24, c"0.3125", None),
        (3, c"bad", None),
        (3, c"\xff", None),
        (3, c"nan", None),
        (3, c"5", None),
    ] {
        assert_eq!(
            params.text_to_value(&handle, ClapId::new(id), value),
            expected
        );
    }
    assert!(
        params
            .get_info(&handle, 23, &mut ParamInfoBuffer::new())
            .is_none()
    );
    assert_eq!(parameter(&mut plugin, 3), Some(1.0));
    assert_eq!(save(&mut plugin), before);
}

struct CountedEvents {
    count: u32,
    missing: bool,
    event: ParamValueEvent,
    reads: std::sync::atomic::AtomicUsize,
}
impl clack_host::events::io::InputEventBuffer for CountedEvents {
    fn len(&self) -> u32 {
        self.count
    }
    fn get(&self, index: u32) -> Option<&clack_host::events::UnknownEvent> {
        self.reads
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if self.missing || index >= self.count {
            None
        } else {
            Some(self.event.as_ref())
        }
    }
}
#[test]
fn missing_and_oversized_lists_and_truncated_supported_payloads_are_bounded_and_atomic() {
    use clack_host::events::{Event, EventHeader, UnknownEvent};
    struct Truncated {
        header: EventHeader,
    }
    impl clack_host::events::io::InputEventBuffer for Truncated {
        fn len(&self) -> u32 {
            1
        }
        fn get(&self, index: u32) -> Option<&UnknownEvent> {
            if index != 0 {
                return None;
            } // SAFETY: exactly the owned header is exposed; no payload is advertised.
            Some(unsafe { UnknownEvent::from_raw(self.header.as_raw()) })
        }
    }
    let mut plugin = load_at(&artifact()).unwrap();
    let mut processor = activate(&mut plugin, 48000.0, 16);
    for (count, missing, reads) in [(u32::MAX, false, 0), (4, true, 1)] {
        let input = CountedEvents {
            count,
            missing,
            event: ParamValueEvent::new(0, ClapId::new(3), Pckn::match_all(), 0.25),
            reads: Default::default(),
        };
        assert_eq!(
            block(&mut processor, 16, &input, false),
            [vec![0.0; 16], vec![0.0; 16]]
        );
        assert_eq!(
            input.reads.load(std::sync::atomic::Ordering::Relaxed),
            reads
        );
    }
    for type_id in [ParamValueEvent::TYPE_ID, MidiEvent::TYPE_ID] {
        let event = ParamValueEvent::new(0, ClapId::new(3), Pckn::match_all(), 0.25);
        let mut raw = *event.header().as_raw();
        raw.size = std::mem::size_of::<EventHeader>() as u32;
        raw.type_ = type_id;
        let input = Truncated {
            header: *EventHeader::from_raw(&raw),
        };
        assert_eq!(
            block(&mut processor, 16, &input, false),
            [vec![0.0; 16], vec![0.0; 16]]
        );
    }
    assert_eq!(parameter(&mut plugin, 3), Some(1.0));
    let output = block(&mut processor, 16, &note(3, 36), false);
    assert_eq!((output[0][3], output[1][3]), (-0.5, -0.5));
    deactivate(&mut plugin, processor);
}
#[test]
fn rejected_output_layouts_silence_all_writable_planes_and_preserve_atomic_values() {
    for (frames, channels, representation, input_bus) in [
        (16, 1, 0, false),
        (16, 3, 0, false),
        (16, 2, 1, false),
        (16, 2, 2, false),
        (16, 2, 3, false),
        (16, 2, 0, true),
        (17, 2, 0, false),
        (0, 2, 0, false),
    ] {
        let mut plugin = load_at(&artifact()).unwrap();
        let mut processor = activate(&mut plugin, 48000.0, 16);
        let mut planes = [
            vec![123.0f32; frames],
            vec![123.0; frames],
            vec![123.0; frames],
        ];
        let mut doubles = [
            vec![456.0f64; frames],
            vec![456.0; frames],
            vec![456.0; frames],
        ];
        std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let mut inputs = AudioPorts::with_capacity(1, 1);
                    let mut input_plane = vec![0.0f32; frames];
                    let ports = if input_bus {
                        vec![AudioPortBuffer {
                            latency: 0,
                            channels: AudioPortBufferType::f32_input_only([
                                InputChannel::variable(&mut input_plane),
                            ]),
                        }]
                    } else {
                        vec![]
                    };
                    let input = inputs.with_input_buffers(ports);
                    let mut outputs = AudioPorts::with_capacity(3, 1);
                    let mut output = outputs.with_output_buffers([AudioPortBuffer {
                        latency: 0,
                        channels: AudioPortBufferType::f32_output_only(
                            planes.iter_mut().map(|x| x.as_mut_slice()),
                        ),
                    }]);
                    let mut pointers = doubles.each_mut().map(|p| p.as_mut_ptr());
                    let raw = &mut output.as_raw_buffers()[0];
                    raw.channel_count = channels;
                    if representation == 1 || representation == 3 {
                        raw.data32 = std::ptr::null_mut();
                    }
                    if representation == 1 || representation == 2 {
                        raw.data64 = pointers.as_mut_ptr();
                    }
                    let mut events = EventBuffer::new();
                    events.push(&ParamValueEvent::new(
                        0,
                        ClapId::new(3),
                        Pckn::match_all(),
                        0.25,
                    ));
                    assert_eq!(
                        processor
                            .process(
                                &input,
                                &mut output,
                                &events.as_input(),
                                &mut EventBuffer::new().as_output(),
                                None,
                                None
                            )
                            .unwrap(),
                        clack_host::process::ProcessStatus::Continue
                    );
                })
                .join()
                .unwrap()
        });
        if representation != 1 && representation != 3 {
            for plane in planes.iter().take(channels as usize) {
                assert!(plane.iter().all(|x| *x == 0.0));
            }
        }
        if representation == 1 || representation == 2 {
            for plane in doubles.iter().take(channels as usize) {
                assert!(plane.iter().all(|x| *x == 0.0));
            }
        }
        assert_eq!(parameter(&mut plugin, 3), Some(1.0));
        deactivate(&mut plugin, processor);
    }
}
#[cfg(unix)]
#[test]
fn entry_preserves_non_utf8_plugin_path_and_fixture_descriptors_survive_missing_trigger_resources()
{
    use std::os::unix::ffi::OsStringExt;
    let (directory, path) = copied_product();
    let renamed = directory
        .path()
        .join(std::ffi::OsString::from_vec(b"Trigger-\xff.clap".to_vec()));
    std::fs::rename(path, &renamed).unwrap();
    let mut plugin =
        load_at(&renamed).expect("Entry retains Linux OSbytes rather than requiring UTF8");
    let mut processor = activate(&mut plugin, 48000.0, 16);
    let output = block(&mut processor, 16, &note(3, 36), false);
    assert_eq!(output[0][3], -0.5);
    deactivate(&mut plugin, processor);
    drop(plugin);
    std::fs::remove_file(
        directory
            .path()
            .join("DandrumTrigger.resources/instrument/assets/advanced-drums.wav"),
    )
    .unwrap();
    assert!(load_at(&renamed).is_err());
    // SAFETY: explicitly copied trusted product; returned fixture retains its entry.
    let entry = unsafe { PluginEntry::load(&renamed) }.unwrap();
    let factory = entry.get_factory::<PluginFactory>().unwrap();
    assert_eq!(factory.plugin_count(), 3);
    let fixture = PluginInstance::<TestHost>::new(
        |_| HostShared::default(),
        |_| HostMain::default(),
        &entry,
        c"org.dandrum.experimental.signed-gain",
        &HostInfo::new("Fixture retention", "Dandrum", "", "0.1").unwrap(),
    );
    assert!(fixture.is_ok());
}

#[cfg(target_os = "linux")]
#[test]
fn dynamic_callback_allocator_audit_observes_plugin_and_framework_dso() {
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
    let hashes = std::process::Command::new("sha256sum")
        .args([
            artifact(),
            executable.clone(),
            library.clone(),
            rust_library.clone(),
            interposer.clone(),
            probe.clone(),
        ])
        .output()
        .unwrap();
    assert!(hashes.status.success());
    let launch = format!(
        "LD_PRELOAD={} DANDRUM_ALLOCATOR_AUDIT_PROBE={} {} --exact allocator_audit_child --ignored --nocapture\n{}",
        library.display(),
        rust_library.display(),
        executable.display(),
        String::from_utf8_lossy(&hashes.stdout)
    );
    println!("{launch}");
    let output = std::process::Command::new(&executable)
        .args([
            "--exact",
            "allocator_audit_child",
            "--ignored",
            "--nocapture",
        ])
        .env("LD_PRELOAD", &library)
        .env("DANDRUM_ALLOCATOR_AUDIT_PROBE", &rust_library)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "calibrated DSO audit child: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));
    if let Some(root) = std::env::var_os("DANDRUM_TRIGGER_CLAP_AUDIT_EVIDENCE_DIR") {
        let root = PathBuf::from(root).join(format!("allocator-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        for path in [&library, &rust_library, &interposer, &probe] {
            std::fs::copy(path, root.join(path.file_name().unwrap())).unwrap();
        }
        std::fs::write(root.join("launch.txt"), launch).unwrap();
        std::fs::write(root.join("stdout.log"), &output.stdout).unwrap();
        std::fs::write(root.join("stderr.log"), &output.stderr).unwrap();
    }
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "launched by allocator parent with calibrated LD_PRELOAD; never a standalone skip claim"]
fn allocator_audit_child() {
    type Begin = unsafe extern "C" fn();
    type End = unsafe extern "C" fn(*mut u64, *mut u64);
    struct Audit {
        begin: Begin,
        end: End,
    }
    impl Audit {
        fn measure<T>(&self, callback: impl FnOnce() -> T) -> (T, (u64, u64)) {
            // SAFETY: functions belong to the preloaded test fixture and use thread-local counters.
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
    // SAFETY: this test-only process is launched with our explicit trusted interposer/probe.
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
    let preexisting = unsafe { allocate() };
    assert!(!preexisting.is_null());
    let (fresh, allocation) = audit.measure(|| unsafe { allocate() });
    assert!(!fresh.is_null());
    assert!(
        allocation.0 >= 1,
        "Rust cdylib allocation must reach observer"
    );
    assert_eq!(allocation.1, 0);
    let (_, deallocation) = audit.measure(|| unsafe { deallocate(preexisting) });
    assert_eq!(deallocation.0, 0);
    assert!(
        deallocation.1 >= 1,
        "preexisting Rust cdylib deallocation must reach observer"
    );
    unsafe { deallocate(fresh) };
    let (_, empty) = audit.measure(|| ());
    assert_eq!(empty, (0, 0));
    let mut plugin = load_at(&artifact()).unwrap();
    let mut inactive = plugin.inactive_plugin_handle().unwrap();
    let params = inactive.get_extension::<PluginParams>().unwrap();
    let mut stopped = EventBuffer::new();
    stopped.push(&ParamValueEvent::new(
        u32::MAX,
        ClapId::new(3),
        Pckn::match_all(),
        0.5,
    ));
    let mut stopped_out = EventBuffer::with_capacity(64);
    let (_, stopped_count) = audit.measure(|| {
        params.flush(
            &mut inactive,
            &stopped.as_input(),
            &mut stopped_out.as_output(),
        )
    });
    assert_eq!(stopped_count, (0, 0));
    let mut processor = activate(&mut plugin, 48000.0, 16);
    let mut output = [[123.0f32; 16]; 2];
    let mut inputs = AudioPorts::with_capacity(0, 0);
    let input = inputs.with_input_buffers(std::iter::empty::<
        AudioPortBuffer<[InputChannel<'_, f32>; 0], [InputChannel<'_, f64>; 0]>,
    >());
    let mut ports = AudioPorts::with_capacity(2, 1);
    let mut outputs = ports.with_output_buffers([AudioPortBuffer {
        latency: 0,
        channels: AudioPortBufferType::f32_output_only(output.iter_mut().map(|x| x.as_mut_slice())),
    }]);
    let mut out_events = EventBuffer::with_capacity(64);
    let notes = note(3, 36);
    let mut valid = EventBuffer::new();
    valid.push(&ParamValueEvent::new(
        u32::MAX,
        ClapId::new(3),
        Pckn::match_all(),
        0.5,
    ));
    let mut bad = EventBuffer::new();
    bad.push(&ParamValueEvent::new(
        0,
        ClapId::new(24),
        Pckn::match_all(),
        0.25,
    ));
    let callback_counts = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let mut counts = [(0, 0); 5];
                let (status, count) = audit.measure(|| {
                    processor.process(
                        &input,
                        &mut outputs,
                        &notes.as_input(),
                        &mut out_events.as_output(),
                        None,
                        None,
                    )
                });
                counts[0] = count;
                assert_eq!(
                    status.unwrap(),
                    clack_host::process::ProcessStatus::Continue
                );
                let mut handle = processor.plugin_handle();
                let params = handle.get_extension::<PluginParams>().unwrap();
                let (_, count) = audit.measure(|| {
                    params.flush_active(&mut handle, &valid.as_input(), &mut out_events.as_output())
                });
                counts[1] = count;
                let (_, count) = audit.measure(|| {
                    params.flush_active(&mut handle, &bad.as_input(), &mut out_events.as_output())
                });
                counts[2] = count;
                let (status, count) = audit.measure(|| {
                    processor.process(
                        &input,
                        &mut outputs,
                        &bad.as_input(),
                        &mut out_events.as_output(),
                        None,
                        None,
                    )
                });
                counts[3] = count;
                assert_eq!(
                    status.unwrap(),
                    clack_host::process::ProcessStatus::Continue
                );
                let (_, count) = audit.measure(|| processor.reset());
                counts[4] = count;
                counts
            })
            .join()
            .unwrap()
    });
    assert_eq!(
        callback_counts,
        [(0, 0); 5],
        "actual plugin+Clack callback allocations/deallocations"
    );
    let has_gui = plugin
        .plugin_handle()
        .get_extension::<clack_extensions::gui::PluginGui>()
        .is_some();
    let (_, main_count) = audit.measure(|| plugin.call_on_main_thread_callback());
    if !has_gui {
        assert_eq!(
            main_count,
            (0, 0),
            "headless static diagnostic through preallocated host recorder"
        );
    }
    // GUI control service runs off audio and may allocate; the callback counts
    // above remain zero for both products, and exact diagnostics are checked below.
    println!("main-thread service: gui={has_gui}, allocations/deallocations={main_count:?}");
    plugin.access_shared_handler(|h| {
        assert_eq!(h.logs.load(std::sync::atomic::Ordering::Relaxed), 3)
    });
    let mut changed: serde_json::Value = serde_json::from_slice(&save(&mut plugin)).unwrap();
    changed["document"] = changed["document"]
        .as_str()
        .unwrap()
        .replace("key_range: [36, 36]", "key_range: [60, 60]")
        .into();
    assert!(load_state(
        &mut plugin,
        &serde_json::to_vec(&changed).unwrap()
    ));
    let remapped = note(3, 60);
    let (status, swap_count) = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                audit.measure(|| {
                    processor.process(
                        &input,
                        &mut outputs,
                        &remapped.as_input(),
                        &mut out_events.as_output(),
                        None,
                        None,
                    )
                })
            })
            .join()
            .unwrap()
    });
    assert_eq!(
        status.unwrap(),
        clack_host::process::ProcessStatus::Continue
    );
    assert_eq!(
        swap_count,
        (0, 0),
        "pending runtime adoption must retire without freeing on audio"
    );
    deactivate(&mut plugin, processor);
    println!(
        "calibration Rustcdylib allocate={allocation:?} preexisting-free={deallocation:?}; plugin callbacks process/flush/invalid/reset={callback_counts:?}; stoppedflush={stopped_count:?}; swap={swap_count:?}; main={main_count:?}"
    );
}

#[test]
fn full_sixty_four_event_budget_and_velocity_zero_keep_signed_one_shot_semantics() {
    let mut plugin = load_at(&artifact()).unwrap();
    let mut processor = activate(&mut plugin, 48000.0, 16);
    let mut full = EventBuffer::new();
    for _ in 0..64 {
        full.push(&MidiEvent::new(3, 0, [0x90, 36, 100]));
    }
    let output = block(&mut processor, 16, &full, false);
    for channel in output {
        assert_eq!(&channel[..3], &[0.0; 3]);
        assert_eq!(
            channel[3], -4.0,
            "eight prepared sample voices survive the finite host budget"
        );
    }
    let mut release = note(0, 36);
    release.push(&MidiEvent::new(3, 0, [0x90, 36, 0]));
    let output = block(&mut processor, 16, &release, true);
    for channel in output {
        assert_eq!(
            &channel[..6],
            &[
                -0.5,
                -358.0 / 32768.0,
                1554.0 / 32768.0,
                4137.0 / 32768.0,
                -2136.0 / 32768.0,
                1664.0 / 32768.0
            ],
            "velocity zero releases rather than admitting another onset; one shot retains natural tail"
        );
    }
    deactivate(&mut plugin, processor);
}

#[test]
fn optional_host_extensions_and_noncore_events_are_honest_and_unusable_entry_paths_reject() {
    use clack_host::events::{Event, EventHeader, UnknownEvent};
    let mut plugin = load_with_options(&artifact(), true).unwrap();
    let baseline = save(&mut plugin);
    assert!(load_state(&mut plugin, &baseline));
    plugin.access_handler(|h| assert_eq!(h.rescans.get(), 0));
    let mut processor = activate(&mut plugin, 48000.0, 16);
    let mut invalid = EventBuffer::new();
    invalid.push(&ParamValueEvent::new(
        0,
        ClapId::new(24),
        Pckn::match_all(),
        0.5,
    ));
    assert_eq!(
        block(&mut processor, 16, &invalid, false),
        [vec![0.0; 16], vec![0.0; 16]]
    );
    plugin.call_on_main_thread_callback();
    plugin.access_shared_handler(|h| {
        assert_eq!(h.logs.load(std::sync::atomic::Ordering::Relaxed), 0)
    });
    struct Mixed {
        header: EventHeader,
        note: MidiEvent,
    }
    impl clack_host::events::io::InputEventBuffer for Mixed {
        fn len(&self) -> u32 {
            2
        }
        fn get(&self, index: u32) -> Option<&UnknownEvent> {
            match index {
                0 => Some(unsafe { UnknownEvent::from_raw(self.header.as_raw()) }),
                1 => Some(self.note.as_ref()),
                _ => None,
            }
        }
    }
    let value = ParamValueEvent::new(0, ClapId::new(24), Pckn::match_all(), f64::NAN);
    let mut raw = *value.header().as_raw();
    raw.space_id = 7;
    raw.size = std::mem::size_of::<EventHeader>() as u32;
    let events = Mixed {
        header: *EventHeader::from_raw(&raw),
        note: MidiEvent::new(3, 0, [0x90, 36, 100]),
    };
    let output = block(&mut processor, 16, &events, false);
    assert_eq!((output[0][3], output[1][3]), (-0.5, -0.5));
    deactivate(&mut plugin, processor);
    let (_directory, path) = copied_product();
    // SAFETY: a real copied library is retained; only the Entry init path is deliberately unusable.
    let provider = unsafe { clack_host::entry::LibraryEntry::load_from_path(path) }.unwrap();
    let entry = unsafe { PluginEntry::load_from(provider, c"relative-plugin.clap") }.unwrap();
    assert_eq!(
        entry.get_factory::<PluginFactory>().unwrap().plugin_count(),
        3
    );
    assert!(
        PluginInstance::<TestHost>::new(
            |_| HostShared::default(),
            |_| HostMain::default(),
            &entry,
            c"org.dandrum.trigger",
            &HostInfo::new("Invalid entry path", "Dandrum", "", "0.1").unwrap()
        )
        .is_err()
    );
}

#[test]
fn supported_fractional_clocks_activate_and_keep_exact_signed_sample_timing() {
    let (directory, path) = copied_product();
    let resources = directory.path().join("DandrumTrigger.resources/instrument");
    let document_path = resources.join("advanced-drum-kit.yaml");
    let document = std::fs::read_to_string(&document_path).unwrap();
    let original = "id: kick, start_frame: 0, end_frame: 12000";
    assert!(
        document.contains(original),
        "known real kit region is the fixture control"
    );
    std::fs::write(
        &document_path,
        document.replace(original, "id: kick, start_frame: 0, end_frame: 1024"),
    )
    .unwrap();
    // Retain valid source bounds for the entire real kit and its public surface.
    // Only the kick region is shortened; its distinct ramp is exact PCM16.
    let mut values = vec![0.0; 51_000];
    for (frame, value) in values[..1_024].iter_mut().enumerate() {
        *value = -0.75 + frame as f32 / 4_096.0;
    }
    dandrum_engine::wav::write_wav_stereo_i16(
        std::fs::File::create(resources.join("assets/advanced-drums.wav")).unwrap(),
        48_000,
        &values,
        &values,
    )
    .unwrap();
    let mut plugin = load_at(&path).expect("actual Trigger initializes the valid controlled kit");
    for hz in [48_000.0, 1_234.567_8, 48_000.5, 48_000.000_001] {
        let stopped = plugin
            .activate(
                |_, _| (),
                PluginAudioConfiguration {
                    sample_rate: hz,
                    min_frames_count: 1,
                    max_frames_count: 64,
                },
            )
            .unwrap_or_else(|error| panic!("supported {hz} Hz must activate: {error}"));
        let mut processor = std::thread::spawn(move || stopped.start_processing().unwrap())
            .join()
            .unwrap();
        let mut output = [Vec::with_capacity(1_026), Vec::with_capacity(1_026)];
        while output[0].len() < 1_026 {
            let frames = (1_026 - output[0].len()).min(64);
            let events = if output[0].is_empty() {
                note(0, 36)
            } else {
                EventBuffer::new()
            };
            let rendered = block(&mut processor, frames, &events, false);
            for (target, plane) in output.iter_mut().zip(rendered) {
                target.extend(plane);
            }
        }
        for channel in &output {
            assert_eq!(channel[0], -0.75, "literal signed PCM at {hz} Hz");
            for (frame, &actual) in channel.iter().enumerate() {
                let position = frame as f64 * 48_000.0 / hz;
                let expected = if position < 1_024.0 {
                    (-0.75 + position.min(1_023.0) / 4_096.0) as f32
                } else {
                    0.0
                };
                assert!(
                    (actual - expected).abs() <= 0.000_000_2,
                    "{hz} Hz frame {frame}: {actual} != {expected}"
                );
            }
            let sounding_frames = (1_024.0 * hz / 48_000.0).ceil() as usize;
            assert_ne!(
                channel[sounding_frames - 1],
                0.0,
                "last sounding frame at {hz} Hz"
            );
            assert_eq!(
                channel[sounding_frames], 0.0,
                "first silent frame at {hz} Hz"
            );
        }
        deactivate(&mut plugin, processor);
    }
}
