use clack_host::entry::PluginEntry;
use clack_host::factory::plugin::PluginFactory;
use std::path::PathBuf;

fn artifact() -> PathBuf {
    std::env::var_os("DANDRUM_CLAP_PLUGIN_PATH").map(PathBuf::from).unwrap_or_else(||
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../build/engine-only/clap/DandrumHeadlessProof.clap"))
}

#[test]
fn exported_binary_discovers_two_experimental_descriptors() {
    // SAFETY: this test loads only the exact locally-built proof artifact and
    // keeps its entry alive through every borrowed factory/descriptor access.
    let entry = unsafe { PluginEntry::load(artifact()) }.expect("built .clap artifact loads dynamically");
    let factory = entry.get_factory::<PluginFactory>().expect("exported plugin factory must exist");
    assert_eq!(factory.plugin_count(), 2);
    let ids: Vec<_> = factory.plugin_descriptors().map(|d| d.id().unwrap().to_bytes().to_vec()).collect();
    assert_eq!(ids, [b"org.dandrum.experimental.tb303".to_vec(), b"org.dandrum.experimental.signed-gain".to_vec()]);
}

mod common;
use clack_host::prelude::*;
use clack_host::process::StartedPluginAudioProcessor;
use clack_host::events::event_types::{MidiEvent, ParamValueEvent};
use clack_extensions::params::{PluginParams, ParamInfoBuffer};
use clack_extensions::note_ports::{PluginNotePorts, NotePortInfoBuffer, NoteDialects};

struct TestHost;
struct HostShared;
impl SharedHandler<'_> for HostShared {
    fn request_restart(&self) {}
    fn request_process(&self) {}
    fn request_callback(&self) {}
}
impl HostHandlers for TestHost {
    type Shared<'a> = HostShared;
    type MainThread<'a> = ();
    type AudioProcessor<'a> = ();
}

fn load(id: &str) -> (PluginEntry, PluginInstance<TestHost>) {
    // SAFETY: locally-built trusted artifact, retained alongside its instance.
    let entry = unsafe { PluginEntry::load(artifact()) }.unwrap();
    let id = std::ffi::CString::new(id).unwrap();
    let plugin = PluginInstance::<TestHost>::new(|_| HostShared, |_| (), &entry, &id,
        &HostInfo::new("Dandrum dynamic proof host", "Dandrum", "", "0.1").unwrap()).unwrap();
    (entry, plugin)
}

fn block<E: clack_host::events::io::InputEventBuffer + Sync>(processor: &mut StartedPluginAudioProcessor<TestHost>, frames: usize,
         gain_input: bool, events: &E, channels: usize, reset: bool) -> Vec<Vec<f32>> {
    block_with_status(processor, frames, gain_input, events, channels, reset).0
}

fn block_with_status<E: clack_host::events::io::InputEventBuffer + Sync>(processor: &mut StartedPluginAudioProcessor<TestHost>, frames: usize,
         gain_input: bool, events: &E, channels: usize, reset: bool) -> (Vec<Vec<f32>>, clack_host::process::ProcessStatus) {
    let mut inputs = [vec![-0.5_f32; frames], vec![0.25_f32; frames]];
    let mut outputs = vec![vec![123.0_f32; frames]; channels];
    // Sample planes are prepared before entering the render thread. Host-side
    // AudioPorts and event descriptors are then allocated before process(); this
    // is a functional thread-ownership proof, not an allocation audit.
    let status = std::thread::scope(|scope| scope.spawn(|| {
        if reset { processor.reset(); }
        let mut input_ports = AudioPorts::with_capacity(2, 1);
        let mut output_ports = AudioPorts::with_capacity(channels, 1);
        let ports = if gain_input { vec![AudioPortBuffer { latency: 0,
            channels: AudioPortBufferType::f32_input_only(inputs.iter_mut().map(InputChannel::variable)) }] }
            else { vec![] };
        let input = input_ports.with_input_buffers(ports);
        let mut output = output_ports.with_output_buffers([AudioPortBuffer { latency: 0,
            channels: AudioPortBufferType::f32_output_only(outputs.iter_mut().map(|v| v.as_mut_slice())) }]);
        let mut out_events = EventBuffer::new();
        processor.process(&input, &mut output, &InputEvents::from_buffer(events), &mut out_events.as_output(), None, None).unwrap()
    }).join().unwrap());
    (outputs, status)
}

fn activate(plugin: &mut PluginInstance<TestHost>, rate: f64, max_frames: u32) -> StartedPluginAudioProcessor<TestHost> {
    let stopped = plugin.activate(|_, _| (), PluginAudioConfiguration {
        sample_rate: rate, min_frames_count: 1, max_frames_count: max_frames }).unwrap();
    std::thread::spawn(move || stopped.start_processing().unwrap()).join().unwrap()
}

fn stop(processor: StartedPluginAudioProcessor<TestHost>) -> clack_host::process::StoppedPluginAudioProcessor<TestHost> {
    std::thread::spawn(move || processor.stop_processing()).join().unwrap()
}

#[test]
fn signed_gain_defaults_and_offset_order_use_public_root_control() {
    let (_entry, mut plugin) = load("org.dandrum.experimental.signed-gain");
    let handle = plugin.plugin_handle();
    let params = handle.get_extension::<PluginParams>().unwrap();
    assert_eq!(params.count(&handle), 1);
    let mut info = ParamInfoBuffer::new();
    let info = params.get_info(&handle, 0, &mut info).unwrap();
    assert_eq!(info.name, b"fixture.level");
    assert_eq!((info.min_value, info.max_value, info.default_value), (0.0, 1.0, 0.5));
    let mut processor = activate(&mut plugin, 48000.0, 16);
    let output = block(&mut processor, 16, true, &EventBuffer::new(), 2, false);
    assert_eq!(output, vec![vec![-0.25;16], vec![0.125;16]]);
    let mut changes = EventBuffer::new();
    for (offset, value) in [(5, 0.25), (9, 1.0), (9, 0.75)] {
        changes.push(&ParamValueEvent::new(offset, ClapId::new(1), Pckn::match_all(), value));
    }
    let output = block(&mut processor, 16, true, &changes, 2, false);
    for (channel, values) in output.iter().zip([[-0.25,-0.125,-0.375],[0.125,0.0625,0.1875]]) {
        assert_eq!(&channel[..5], &[values[0];5]); assert_eq!(&channel[5..9], &[values[1];4]);
        assert_eq!(&channel[9..], &[values[2];7]);
    }
    assert_eq!(block(&mut processor, 16, true, &EventBuffer::new(),2,false),
        vec![vec![-0.375;16],vec![0.1875;16]]);
    plugin.deactivate(stop(processor));
}

#[test]
fn tb303_midi1_timing_overlap_and_root_alias_match_signed_engine_oracle() {
    let (_entry, mut plugin) = load("org.dandrum.experimental.tb303");
    let handle = plugin.plugin_handle();
    let params = handle.get_extension::<PluginParams>().unwrap();
    assert_eq!(params.count(&handle), 7);
    let notes_ext = handle.get_extension::<PluginNotePorts>().unwrap();
    assert_eq!(notes_ext.count(&handle, true), 1);
    let mut info = NotePortInfoBuffer::new();
    assert_eq!(notes_ext.get(&handle,0,true,&mut info).unwrap().supported_dialects, NoteDialects::MIDI);
    let mut processor = activate(&mut plugin,48000.0,64);
    let notes = [(5,[0x90,60,110]),(24,[0x90,64,90]),(30,[0x80,60,0]),(48,[0x80,64,0])];
    let mut events = EventBuffer::new();
    events.push(&MidiEvent::new(5,0,notes[0].1));
    events.push(&ParamValueEvent::new(16,ClapId::new(1),Pckn::match_all(),0.2));
    for &(time,data) in &notes[1..] { events.push(&MidiEvent::new(time,0,data)); }
    let actual = block(&mut processor,64,false,&events,2,false);
    let expected = common::direct_engine(common::TB303,64,48000,&notes,&[(16,0,0.2)]);
    for (actual,expected) in actual.iter().zip(expected) {
        assert_eq!(&actual[..5], &[0.0;5]);
        for (&a,e) in actual.iter().zip(expected) { assert!((a-e).abs()<=1e-7, "TB303 {a} vs {e}"); }
        assert!((actual[20] + 0.0014245948).abs() <= 1e-8);
    }
    // Shared-offset event ordering is tested against the engine's declared mono semantics.
    for reverse in [false,true] {
        let ordered = if reverse { [(5,[0x90,60,110]),(24,[0x90,64,90]),(24,[0x80,60,0])] }
            else { [(5,[0x90,60,110]),(24,[0x80,60,0]),(24,[0x90,64,90])] };
        let mut events = EventBuffer::new();
        for (time,data) in ordered { events.push(&MidiEvent::new(time,0,data)); }
        let actual=block(&mut processor,64,false,&events,2,true);
        let expected=common::direct_engine(common::TB303,64,48000,&ordered,&[(0,0,0.2)]);
        for (actual,expected) in actual.iter().zip(expected) {
            for (&a,e) in actual.iter().zip(expected) { assert!((a-e).abs()<=1e-7); }
        }
    }
    plugin.deactivate(stop(processor));
}

#[test]
fn independent_instances_reset_and_changed_settings_reactivation_retain_controls() {
    let (_entry,mut first)=load("org.dandrum.experimental.signed-gain");
    let (_other_entry,mut second)=load("org.dandrum.experimental.signed-gain");
    let mut a=activate(&mut first,48000.0,16); let mut b=activate(&mut second,48000.0,16);
    let mut changes=EventBuffer::new();
    changes.push(&ParamValueEvent::new(0,ClapId::new(1),Pckn::match_all(),0.75));
    assert_eq!(block(&mut a,16,true,&changes,2,false)[0],vec![-0.375;16]);
    assert_eq!(block(&mut b,16,true,&EventBuffer::new(),2,false)[0],vec![-0.25;16]);
    assert_eq!(block(&mut a,16,true,&EventBuffer::new(),2,true)[1],vec![0.1875;16]);
    first.deactivate(stop(a));
    let mut a=activate(&mut first,44100.0,32);
    assert_eq!(block(&mut a,32,true,&EventBuffer::new(),2,false)[0],vec![-0.375;32]);
    assert_eq!(block(&mut b,16,true,&EventBuffer::new(),2,false)[1],vec![0.125;16]);
    first.deactivate(stop(a)); second.deactivate(stop(b));
    let (_entry,mut synth)=load("org.dandrum.experimental.tb303");
    let mut synth_processor=activate(&mut synth,48000.0,64);
    let mut notes=EventBuffer::new(); notes.push(&MidiEvent::new(0,0,[0x90,60,110]));
    assert!(block(&mut synth_processor,64,false,&notes,2,false)[0].iter().any(|v| *v<0.0));
    assert!(block(&mut synth_processor,64,false,&EventBuffer::new(),2,true).iter().flatten().all(|v| *v==0.0));
    synth.deactivate(stop(synth_processor));
    let mut synth_processor=activate(&mut synth,96000.0,32);
    assert!(block(&mut synth_processor,32,false,&EventBuffer::new(),2,false).iter().flatten().all(|v| *v==0.0));
    synth.deactivate(stop(synth_processor));
}

#[test]
fn malformed_callback_events_and_layout_silence_without_changing_valid_control_state() {
    let (_entry,mut plugin)=load("org.dandrum.experimental.signed-gain");
    let mut processor=activate(&mut plugin,48000.0,16);
    let cases=[vec![(3,1,f64::NAN)],vec![(3,99,0.5)],vec![(16,1,0.5)],
        vec![(9,1,0.2),(3,1,0.3)],vec![(3,1,2.0)]];
    for case in cases {
        let mut events=EventBuffer::new();
        for (time,id,value) in case { events.push(&ParamValueEvent::new(time,ClapId::new(id),Pckn::match_all(),value)); }
        assert!(block(&mut processor,16,true,&events,2,false).iter().flatten().all(|v| *v==0.0));
        assert_eq!(block(&mut processor,16,true,&EventBuffer::new(),2,false)[0],vec![-0.25;16]);
    }
    assert!(block(&mut processor,16,true,&EventBuffer::new(),1,false).iter().flatten().all(|v| *v==0.0));
    let mut events=EventBuffer::new();
    for _ in 0..1025 { events.push(&ParamValueEvent::new(0,ClapId::new(1),Pckn::match_all(),0.75)); }
    assert!(block(&mut processor,16,true,&events,2,false).iter().flatten().all(|v| *v==0.0));
    plugin.deactivate(stop(processor));
}

#[test]
fn midi1_repeated_key_and_velocity_zero_follow_engine_key_semantics() {
    let (_entry,mut synth)=load("org.dandrum.experimental.tb303");
    let mut processor=activate(&mut synth,48000.0,64);
    // MIDI 1 keys identify notes here; no independent CLAP note-ID ownership is claimed.
    for notes in [
        vec![(5,[0x90,60,110]),(24,[0x90,60,90]),(40,[0x80,60,0])],
        vec![(5,[0x90,60,110]),(24,[0x90,60,0])],
    ] {
        let mut events=EventBuffer::new();
        for &(time,data) in &notes { events.push(&MidiEvent::new(time,0,data)); }
        let actual=block(&mut processor,64,false,&events,2,true);
        let expected=common::direct_engine(common::TB303,64,48000,&notes,&[]);
        for (actual,expected) in actual.iter().zip(expected) {
            for (&a,e) in actual.iter().zip(expected) { assert!((a-e).abs()<=1e-7); }
            assert!(actual[..24].iter().any(|v|*v<0.0));
            assert_eq!(actual[63],0.0);
        }
    }
    synth.deactivate(stop(processor));
}

#[test]
fn two_tb303_instances_keep_independent_held_notes_and_filter_history() {
    let (_entry,mut first)=load("org.dandrum.experimental.tb303");
    let (_other,mut second)=load("org.dandrum.experimental.tb303");
    let mut a=activate(&mut first,48000.0,64); let mut b=activate(&mut second,48000.0,64);
    let mut a_notes=EventBuffer::new(); a_notes.push(&MidiEvent::new(0,0,[0x90,64,90]));
    let mut b_notes=EventBuffer::new(); b_notes.push(&MidiEvent::new(0,0,[0x90,60,110]));
    let expected_a=common::direct_engine(common::TB303,64,48000,&[(0,[0x90,64,90])],&[]);
    let expected_b=common::direct_engine(common::TB303,128,48000,&[(0,[0x90,60,110])],&[]);
    let actual_a=block(&mut a,64,false,&a_notes,2,false);
    let actual_b=block(&mut b,64,false,&b_notes,2,false);
    for channel in 0..2 {
        for (&x,&y) in actual_a[channel].iter().zip(&expected_a[channel]) { assert!((x-y).abs()<=1e-7); }
        for (&x,&y) in actual_b[channel].iter().zip(&expected_b[channel][..64]) { assert!((x-y).abs()<=1e-7); }
    }
    assert!(block(&mut a,64,false,&EventBuffer::new(),2,true).iter().flatten().all(|v|*v==0.0));
    let continued=block(&mut b,64,false,&EventBuffer::new(),2,false);
    for channel in 0..2 {
        for (&x,&y) in continued[channel].iter().zip(&expected_b[channel][64..]) { assert!((x-y).abs()<=1e-7); }
        assert!(continued[channel].iter().any(|v|*v<0.0));
    }
    first.deactivate(stop(a)); second.deactivate(stop(b));
}

#[test]
fn high_sample_rate_reactivation_keeps_signed_gain_and_real_instrument_usable() {
    let (_entry, mut gain) = load("org.dandrum.experimental.signed-gain");
    let mut processor = activate(&mut gain, 384000.0, 16);
    assert_eq!(block(&mut processor, 16, true, &EventBuffer::new(), 2, false),
        vec![vec![-0.25; 16], vec![0.125; 16]]);
    gain.deactivate(stop(processor));
    let (_entry, mut synth) = load("org.dandrum.experimental.tb303");
    let mut processor = activate(&mut synth, 384000.0, 64);
    let mut notes = EventBuffer::new();
    notes.push(&MidiEvent::new(5, 0, [0x90, 60, 110]));
    let actual = block(&mut processor, 64, false, &notes, 2, false);
    let expected = common::direct_engine(common::TB303, 64, 384000, &[(5, [0x90, 60, 110])], &[]);
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(&actual[..5], &[0.0; 5]);
        for (&a, e) in actual.iter().zip(expected) { assert!((a - e).abs() <= 1e-7); }
        assert!(actual.iter().any(|value| *value < 0.0));
    }
    synth.deactivate(stop(processor));
}

#[test]
fn fractional_and_high_host_rates_preserve_signed_output_and_runtime_clock() {
    for rate in [768000.0, 1234.5678, 12345.678, 45678.901, 123456.78] {
        let (_entry, mut gain) = load("org.dandrum.experimental.signed-gain");
        let mut processor = activate(&mut gain, rate, 16);
        assert_eq!(block(&mut processor, 16, true, &EventBuffer::new(), 2, false),
            vec![vec![-0.25; 16], vec![0.125; 16]]);
        gain.deactivate(stop(processor));
        let (_entry, mut synth) = load("org.dandrum.experimental.tb303");
        let mut processor = activate(&mut synth, rate, 64);
        let mut notes = EventBuffer::new();
        notes.push(&MidiEvent::new(5, 0, [0x90, 60, 110]));
        let actual = block(&mut processor, 64, false, &notes, 2, false);
        let expected = common::direct_engine_at_rate(common::TB303, 64, rate, &[(5, [0x90, 60, 110])], &[]);
        for (actual, expected) in actual.iter().zip(expected) {
            assert_eq!(&actual[..5], &[0.0; 5]);
            for (&a, e) in actual.iter().zip(expected) { assert!((a - e).abs() <= 1e-7); }
            assert!(actual.iter().any(|value| *value < 0.0));
        }
        synth.deactivate(stop(processor));
    }
}

#[test]
fn note_batch_over_prepared_capacity_is_rejected_before_admitting_partial_notes() {
    let (_entry, mut synth) = load("org.dandrum.experimental.tb303");
    let mut processor = activate(&mut synth, 48000.0, 16);
    let mut notes = EventBuffer::new();
    for key in 0..129 { notes.push(&MidiEvent::new(0, 0, [0x90, (key % 128) as u8, 110])); }
    assert!(block(&mut processor, 16, false, &notes, 2, false).iter().flatten().all(|value| *value == 0.0));
    assert!(block(&mut processor, 16, false, &EventBuffer::new(), 2, false).iter().flatten().all(|value| *value == 0.0),
        "an oversized note batch must not leave partially admitted held notes");
    let mut valid_note = EventBuffer::new();
    valid_note.push(&MidiEvent::new(0, 0, [0x90, 60, 110]));
    let actual = block(&mut processor, 16, false, &valid_note, 2, false);
    // One valid silent block advanced ordinary DSP before this admitted note.
    let expected = common::direct_engine(common::TB303, 32, 48000, &[(16, [0x90, 60, 110])], &[]);
    for (actual, expected) in actual.iter().zip(expected) {
        for (&a, &e) in actual.iter().zip(&expected[16..]) { assert!((a - e).abs() <= 1e-7); }
    }
    synth.deactivate(stop(processor));
}

struct CountedEvents {
    count: u32,
    missing: bool,
    event: ParamValueEvent,
    reads: std::sync::atomic::AtomicUsize,
}

impl clack_host::events::io::InputEventBuffer for CountedEvents {
    fn len(&self) -> u32 { self.count }
    fn get(&self, index: u32) -> Option<&clack_host::events::UnknownEvent> {
        self.reads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if self.missing || index >= self.count { None } else { Some(self.event.as_ref()) }
    }
}

#[test]
fn malformed_event_lists_are_rejected_without_unbounded_event_reads() {
    let (_entry, mut gain) = load("org.dandrum.experimental.signed-gain");
    let mut processor = activate(&mut gain, 48000.0, 16);
    for (count, missing, expected_reads) in [(2048, false, 0), (4, true, 4)] {
        let events = CountedEvents { count, missing,
            event: ParamValueEvent::new(0, ClapId::new(1), Pckn::match_all(), 0.5),
            reads: std::sync::atomic::AtomicUsize::new(0) };
        let output = block(&mut processor, 16, true, &events, 2, false);
        assert!(output.iter().flatten().all(|value| *value == 0.0));
        assert_eq!(events.reads.load(std::sync::atomic::Ordering::Relaxed), expected_reads);
    }
    assert_eq!(block(&mut processor, 16, true, &EventBuffer::new(), 2, false),
        vec![vec![-0.25; 16], vec![0.125; 16]]);
    gain.deactivate(stop(processor));
}

#[test]
fn rejected_callback_keeps_host_processing_an_existing_held_instrument() {
    let (_entry, mut synth) = load("org.dandrum.experimental.tb303");
    let mut processor = activate(&mut synth, 48000.0, 64);
    let mut notes = EventBuffer::new();
    notes.push(&MidiEvent::new(0, 0, [0x90, 60, 110]));
    block(&mut processor, 64, false, &notes, 2, false);
    let mut invalid = EventBuffer::new();
    invalid.push(&ParamValueEvent::new(0, ClapId::new(1), Pckn::match_all(), f64::NAN));
    let (silent, status) = block_with_status(&mut processor, 64, false, &invalid, 2, false);
    assert!(silent.iter().flatten().all(|value| *value == 0.0));
    assert_eq!(status, clack_host::process::ProcessStatus::Continue,
        "silent rejection must not tell the host that the held instrument can sleep");
    let continued = block(&mut processor, 64, false, &EventBuffer::new(), 2, false);
    assert!(continued.iter().flatten().any(|value| *value != 0.0));
    synth.deactivate(stop(processor));
}
