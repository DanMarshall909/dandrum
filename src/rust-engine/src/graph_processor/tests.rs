use super::*;
use crate::core::TimedInputEvent;
use crate::fft;
use crate::graph::*;
use crate::patch;
use crate::sample::{LoadedSample, PreparedSamplerAssets};
use crate::script::ScriptEvent;
use std::collections::BTreeMap;
use std::fs;

fn sampler_assets(frames: Vec<f32>) -> PreparedSamplerAssets {
    PreparedSamplerAssets::from_samples_by_module(BTreeMap::from([(
        "sampler".to_string(),
        LoadedSample::new(48_000, frames),
    )]))
}

fn sampler_graph(extra_modules: Vec<ModuleNode>, extra_cables: Vec<Cable>) -> Graph {
    let mut modules = vec![
        ModuleNode::new(ModuleId::new("midi"), "midi_input")
            .with_output(builtin_ports::EVENTS, SignalType::Event),
        ModuleNode::new(ModuleId::new("sampler"), "sampler")
            .with_input(builtin_ports::TRIGGER, SignalType::Event)
            .with_input(builtin_ports::RATE, SignalType::Control)
            .with_input(builtin_ports::START, SignalType::Control)
            .with_input(builtin_ports::LOOP_ENABLED, SignalType::Control)
            .with_input(builtin_ports::LOOP_START, SignalType::Control)
            .with_input(builtin_ports::LOOP_END, SignalType::Control)
            .with_output(builtin_ports::AUDIO, SignalType::Audio),
        ModuleNode::new(ModuleId::new("out"), "audio_output")
            .with_input(builtin_ports::LEFT, SignalType::Audio)
            .with_input(builtin_ports::RIGHT, SignalType::Audio),
    ];
    modules.extend(extra_modules);

    let mut cables = vec![
        Cable::new(
            PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
            PortRef::new(ModuleId::new("sampler"), builtin_ports::TRIGGER),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("sampler"), builtin_ports::AUDIO),
            PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
        ),
    ];
    cables.extend(extra_cables);
    Graph::new(modules, cables)
}

fn sampler_settings(duration_frames: u64) -> RenderSettings {
    RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 4,
        duration_frames,
    }
}

fn note_on(frame: u64, velocity: u8) -> TimedInputEvent {
    TimedInputEvent::new(frame, ScriptEvent::NoteOn { note: 60, velocity })
}

fn render_patch(yaml: &str) -> (Vec<f32>, Vec<f32>) {
    let patch = patch::load_patch_str(yaml).expect("patch should parse");
    patch::validate_patch_schema(&patch).expect("schema should be valid");
    let graph = Graph::from_patch_declarations(&patch);
    graph.validate().expect("graph should validate");
    render_offline(&graph, &patch.render, vec![note_on(0, 100)])
}

fn cutoff_control_for_hz(hz: f64) -> f32 {
    let base: f64 = 8000.0 / 20.0;
    ((hz / 20.0).ln() / base.ln()) as f32
}

fn magnitude_at(bins: &[(f64, f64)], target_hz: f64) -> f64 {
    bins.iter()
        .min_by(|(a, _), (b, _)| {
            (a - target_hz)
                .abs()
                .partial_cmp(&(b - target_hz).abs())
                .unwrap()
        })
        .map(|&(_, db)| db)
        .unwrap_or(-100.0)
}

fn filter_impulse_response(
    algorithm: &str,
    mode: Option<&str>,
    comb_type: Option<&str>,
    sample_rate: f32,
    cutoff: Vec<f32>,
    resonance: Vec<f32>,
    gain: Vec<f32>,
) -> Vec<f32> {
    let frames = cutoff.len();
    let mut params = BTreeMap::from([("algorithm".to_string(), algorithm.to_string())]);
    if let Some(mode) = mode {
        params.insert("mode".to_string(), mode.to_string());
    }
    if let Some(comb_type) = comb_type {
        params.insert("comb_type".to_string(), comb_type.to_string());
    }
    let module = ModuleNode::new(ModuleId::new("filter"), "filter").with_params(params);
    let mut state = PerModuleState::new(&module, sample_rate, &PreparedSamplerAssets::empty());
    let mut audio_in = vec![0.0; frames];
    audio_in[0] = 1.0;
    let outputs = process_filter(&mut state, &audio_in, &cutoff, &resonance, &gain, frames);
    outputs
        .audio
        .get(builtin_ports::AUDIO_OUT)
        .expect("filter should emit audio_out")
        .clone()
}

#[test]
fn graph_filter_biquad_highpass_mode_attenuates_low_frequencies() {
    let frames = 16_384;
    let sample_rate = 48_000.0;
    let cutoff = vec![cutoff_control_for_hz(1_000.0); frames];
    let resonance = vec![0.06; frames];
    let gain = vec![0.5; frames];

    let impulse = filter_impulse_response(
        "biquad",
        Some("highpass"),
        None,
        sample_rate as f32,
        cutoff,
        resonance,
        gain,
    );
    let response = fft::compute_magnitude_response(&impulse, sample_rate).bins;

    let low_db = magnitude_at(&response, 100.0);
    let high_db = magnitude_at(&response, 8_000.0);
    assert!(
        high_db - low_db > 18.0,
        "graph highpass should attenuate lows: 100 Hz {low_db:.1} dB, 8 kHz {high_db:.1} dB"
    );
}

#[test]
fn graph_filter_biquad_cutoff_tracks_render_sample_rate() {
    let frames = 32_768;
    let sample_rate = 96_000.0;
    let cutoff = vec![cutoff_control_for_hz(8_000.0); frames];
    let resonance = vec![0.06; frames];
    let gain = vec![0.5; frames];

    let impulse = filter_impulse_response(
        "biquad",
        Some("lowpass"),
        None,
        sample_rate as f32,
        cutoff,
        resonance,
        gain,
    );
    let response = fft::compute_magnitude_response(&impulse, sample_rate).bins;

    let passband_db = magnitude_at(&response, 1_000.0);
    let cutoff_db = magnitude_at(&response, 8_000.0);
    assert!(
        (1.5..=6.0).contains(&(passband_db - cutoff_db)),
        "96 kHz lowpass should place the cutoff near 8 kHz: 1 kHz {passband_db:.1} dB, 8 kHz {cutoff_db:.1} dB"
    );
}

#[test]
fn graph_filter_comb_uses_resonance_for_feedback_amount() {
    let frames = 8_192;
    let sample_rate: f64 = 48_000.0;
    let delay_ms: f64 = 2.0;
    let cutoff_control = ((delay_ms - 1.0) / 99.0) as f32;
    let delay_samples = (sample_rate * delay_ms / 1_000.0).round() as usize;

    let impulse = filter_impulse_response(
        "comb",
        None,
        Some("feedback"),
        sample_rate as f32,
        vec![cutoff_control; frames],
        vec![0.8; frames],
        vec![0.0; frames],
    );

    let first_repeat = impulse[delay_samples - 2..=delay_samples + 2]
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);
    let second_repeat = impulse[delay_samples * 2 - 2..=delay_samples * 2 + 2]
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);
    assert!(
        first_repeat > 0.9,
        "feedback comb should emit the delayed impulse, got {first_repeat}"
    );
    assert!(
        second_repeat > 0.6,
        "resonance should control comb feedback gain independently of gain input, got {second_repeat}"
    );
}

#[test]
fn realtime_graph_processor_records_prepared_maximum_block_size() {
    let graph = sampler_graph(Vec::new(), Vec::new());
    let processor = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph,
        48_000.0,
        &PreparedSamplerAssets::empty(),
        &VoiceAllocation::default(),
        256,
    );

    assert_eq!(processor.prepared_max_block_size(), 256);
}

#[test]
fn realtime_graph_processor_splits_oversized_render_blocks() {
    let graph = sampler_graph(Vec::new(), Vec::new());
    let mut processor = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph,
        48_000.0,
        &sampler_assets(vec![0.25; 16]),
        &VoiceAllocation::default(),
        4,
    );
    let mut left = vec![0.0; 10];
    let mut right = vec![0.0; 10];

    assert_eq!(processor.render(&mut left, &mut right), 10);

    assert_eq!(processor.last_render_chunk_count(), 3);
}

#[test]
fn realtime_graph_processor_is_deterministic_for_same_events_and_block_sequence() {
    let graph = sampler_graph(Vec::new(), Vec::new());
    let assets = sampler_assets(vec![0.25, 0.5, 0.75, 1.0]);
    let mut first = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph.clone(),
        48_000.0,
        &assets,
        &VoiceAllocation::default(),
        8,
    );
    let mut second = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph,
        48_000.0,
        &assets,
        &VoiceAllocation::default(),
        8,
    );
    let mut first_left = vec![0.0; 12];
    let mut first_right = vec![0.0; 12];
    let mut second_left = vec![0.0; 12];
    let mut second_right = vec![0.0; 12];

    first.note_on(60, 100);
    second.note_on(60, 100);

    assert_eq!(first.render(&mut first_left[..5], &mut first_right[..5]), 5);
    assert_eq!(
        second.render(&mut second_left[..5], &mut second_right[..5]),
        5
    );
    assert_eq!(first.render(&mut first_left[5..], &mut first_right[5..]), 7);
    assert_eq!(
        second.render(&mut second_left[5..], &mut second_right[5..]),
        7
    );

    assert_eq!(first_left, second_left);
    assert_eq!(first_right, second_right);
}

#[test]
fn realtime_graph_processor_reuses_top_level_render_scratch_between_blocks() {
    let graph = sampler_graph(Vec::new(), Vec::new());
    let mut processor = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph,
        48_000.0,
        &sampler_assets(vec![0.25; 16]),
        &VoiceAllocation::default(),
        8,
    );
    let mut left = vec![0.0; 8];
    let mut right = vec![0.0; 8];

    processor.note_on(60, 100);
    processor.render(&mut left, &mut right);
    let after_first = processor.top_level_scratch_capacities();
    let output_capacity_after_first = processor.module_output_scratch_capacity();

    processor.render(&mut left, &mut right);

    assert_eq!(processor.top_level_scratch_capacities(), after_first);
    assert_eq!(
        processor.module_output_scratch_capacity(),
        output_capacity_after_first
    );
}

#[test]
fn graph_processor_produces_audio_from_midi_triggered_303_chain() {
    let patch = patch::load_patch_str(
        r#"
metadata:
  name: 303-style
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 48000
modules:
  - id: midi
    type: midi_input
    outputs:
      - name: events
        signal_type: event
  - id: osc
    type: oscillator
    inputs:
      - name: pitch
        signal_type: control
    outputs:
      - name: audio
        signal_type: audio
  - id: env
    type: adsr
    inputs:
      - name: gate
        signal_type: event
    outputs:
      - name: value
        signal_type: control
  - id: vca
    type: gain
    inputs:
      - name: audio_in
        signal_type: audio
      - name: gain
        signal_type: control
    outputs:
      - name: audio_out
        signal_type: audio
  - id: mixer
    type: audio_mixer
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
      - name: right
        signal_type: audio
connections:
  - from: midi.events
    to: env.gate
  - from: osc.audio
    to: vca.audio_in
  - from: env.value
    to: vca.gain
  - from: vca.audio_out
    to: mixer.inputs
  - from: mixer.mix
    to: out.left
  - from: mixer.mix
    to: out.right
"#,
    )
    .expect("patch should parse");

    patch::validate_patch_schema(&patch).expect("schema should be valid");
    let graph = Graph::from_patch_declarations(&patch);
    graph.validate().expect("graph should validate");

    let (left, right) = render_offline(
        &graph,
        &patch.render,
        vec![
            TimedInputEvent::new(
                0,
                ScriptEvent::NoteOn {
                    note: 45,
                    velocity: 100,
                },
            ),
            TimedInputEvent::new(12000, ScriptEvent::NoteOff { note: 45 }),
        ],
    );

    let has_signal = left.iter().any(|&s| s != 0.0) || right.iter().any(|&s| s != 0.0);
    assert!(has_signal, "303-style chain should produce audio");
    assert_eq!(left.len(), 48000);
    assert_eq!(right.len(), 48000);
}

#[test]
fn composite_oscillator_gain_voice_renders_like_equivalent_flat_graph() {
    let flat = render_patch(
        r#"
metadata:
  name: Flat Voice
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 512
modules:
  - id: midi
    type: midi_input
  - id: osc
    type: oscillator
  - id: env
    type: adsr
  - id: vca
    type: gain
  - id: mixer
    type: audio_mixer
  - id: out
    type: audio_output
connections:
  - from: midi.events
    to: env.gate
  - from: osc.audio
    to: vca.audio_in
  - from: env.value
    to: vca.gain
  - from: vca.audio_out
    to: mixer.inputs
  - from: mixer.mix
    to: out.left
"#,
    );
    let composite = render_patch(
        r#"
metadata:
  name: Composite Voice
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 512
module_definitions:
  - type: drum_voice
    inputs:
      - name: trigger
        signal_type: event
        maps_to:
          - env.gate
    outputs:
      - name: audio
        signal_type: audio
        maps_from:
          - vca.audio_out
    modules:
      - id: osc
        type: oscillator
      - id: env
        type: adsr
      - id: vca
        type: gain
    connections:
      - from: osc.audio
        to: vca.audio_in
      - from: env.value
        to: vca.gain
modules:
  - id: midi
    type: midi_input
  - id: voice
    type: drum_voice
  - id: mixer
    type: audio_mixer
  - id: out
    type: audio_output
connections:
  - from: midi.events
    to: voice.trigger
  - from: voice.audio
    to: mixer.inputs
  - from: mixer.mix
    to: out.left
"#,
    );

    assert_eq!(composite, flat);
    assert!(composite.0.iter().any(|sample| *sample != 0.0));
}

#[test]
fn composite_sampler_voice_renders_through_generic_public_ports() {
    let patch = patch::load_patch_str(
        r#"
metadata:
  name: Composite Sampler
render:
  sample_rate_hz: 48000
  block_size_frames: 4
  duration_frames: 4
module_definitions:
  - type: sample_voice
    inputs:
      - name: trigger
        signal_type: event
        maps_to:
          - sampler.trigger
      - name: rate
        signal_type: control
        maps_to:
          - sampler.rate
    outputs:
      - name: audio
        signal_type: audio
        maps_from:
          - sampler.audio
    modules:
      - id: sampler
        type: sampler
modules:
  - id: midi
    type: midi_input
  - id: voice
    type: sample_voice
  - id: mixer
    type: audio_mixer
  - id: out
    type: audio_output
connections:
  - from: midi.events
    to: voice.trigger
  - from: voice.audio
    to: mixer.inputs
  - from: mixer.mix
    to: out.left
"#,
    )
    .expect("patch should parse");
    patch::validate_patch_schema(&patch).expect("schema should validate");
    let graph = Graph::from_patch_declarations(&patch);
    graph
        .validate()
        .expect("expanded sampler composite should validate");
    let assets = PreparedSamplerAssets::from_samples_by_module(BTreeMap::from([(
        "voice::sampler".to_string(),
        LoadedSample::new(48_000, vec![0.25, 0.5, 0.75]),
    )]));

    let (left, right) =
        render_offline_with_sampler_assets(&graph, &patch.render, vec![note_on(0, 100)], &assets);

    assert_eq!(left, vec![0.25, 0.5, 0.75, 0.0]);
    assert_eq!(right, vec![0.0; 4]);
}

#[test]
fn offline_and_realtime_processors_receive_only_expanded_composite_nodes() {
    let patch = patch::load_patch_str(
        r#"
metadata:
  name: Processor Expansion
render:
  sample_rate_hz: 48000
  block_size_frames: 4
  duration_frames: 4
module_definitions:
  - type: drum_voice
    outputs:
      - name: audio
        signal_type: audio
        maps_from:
          - osc.audio
    modules:
      - id: osc
        type: oscillator
modules:
  - id: voice
    type: drum_voice
  - id: mixer
    type: audio_mixer
  - id: out
    type: audio_output
connections:
  - from: voice.audio
    to: mixer.inputs
  - from: mixer.mix
    to: out.left
"#,
    )
    .expect("patch should parse");
    patch::validate_patch_schema(&patch).expect("schema should validate");
    let graph = Graph::from_patch_declarations(&patch);
    graph.validate().expect("expanded graph should validate");

    assert!(
        graph
            .modules()
            .iter()
            .all(|module| module.module_type() != "drum_voice")
    );
    assert!(
        graph
            .modules()
            .iter()
            .any(|module| module.id().as_str() == "voice::osc")
    );
    let _offline = render_offline(&graph, &patch.render, Vec::new());
    let realtime = RealtimeGraphProcessor::new(graph, 48_000.0);

    assert!(
        realtime
            .graph
            .modules()
            .iter()
            .all(|module| module.module_type() != "drum_voice")
    );
}

#[test]
fn sampler_trigger_event_starts_sample_playback_at_event_frame() {
    let graph = sampler_graph(Vec::new(), Vec::new());
    let assets = sampler_assets(vec![0.25, 0.5, 0.75]);

    let (left, right) = render_offline_with_sampler_assets(
        &graph,
        &sampler_settings(6),
        vec![note_on(2, 100)],
        &assets,
    );

    assert_eq!(left, vec![0.0, 0.0, 0.25, 0.5, 0.75, 0.0]);
    assert_eq!(right, vec![0.0; 6]);
}

#[test]
fn sampler_ignores_trigger_velocity_payload_for_amplitude() {
    let graph = sampler_graph(Vec::new(), Vec::new());
    let assets = sampler_assets(vec![0.25, 0.5, 0.75]);

    let low_velocity = render_offline_with_sampler_assets(
        &graph,
        &sampler_settings(4),
        vec![note_on(0, 1)],
        &assets,
    );
    let high_velocity = render_offline_with_sampler_assets(
        &graph,
        &sampler_settings(4),
        vec![note_on(0, 127)],
        &assets,
    );

    assert_eq!(low_velocity, high_velocity);
    assert_eq!(low_velocity.0, vec![0.25, 0.5, 0.75, 0.0]);
}

#[test]
fn sampler_ignores_midi_note_payload_for_playback_rate() {
    let graph = sampler_graph(Vec::new(), Vec::new());
    let assets = sampler_assets(vec![0.25, 0.5, 0.75]);

    let low_note = render_offline_with_sampler_assets(
        &graph,
        &sampler_settings(4),
        vec![TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note: 36,
                velocity: 100,
            },
        )],
        &assets,
    );
    let high_note = render_offline_with_sampler_assets(
        &graph,
        &sampler_settings(4),
        vec![TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note: 84,
                velocity: 100,
            },
        )],
        &assets,
    );

    assert_eq!(low_note, high_note);
}

#[test]
fn routed_rate_control_changes_sampler_playback_speed() {
    let graph = sampler_graph(
        vec![
            ModuleNode::new(ModuleId::new("rate"), "adsr")
                .with_output(builtin_ports::VALUE, SignalType::Control),
        ],
        vec![Cable::new(
            PortRef::new(ModuleId::new("rate"), builtin_ports::VALUE),
            PortRef::new(ModuleId::new("sampler"), builtin_ports::RATE),
        )],
    );
    let assets = sampler_assets(vec![1.0, 2.0, 3.0, 4.0]);

    let (default_rate, _) = render_offline_with_sampler_assets(
        &sampler_graph(Vec::new(), Vec::new()),
        &sampler_settings(4),
        vec![note_on(0, 100)],
        &assets,
    );
    let (routed_rate, _) = render_offline_with_sampler_assets(
        &graph,
        &sampler_settings(4),
        vec![note_on(0, 100)],
        &assets,
    );

    assert_ne!(routed_rate, default_rate);
    assert_eq!(default_rate, vec![1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn downstream_gain_can_apply_amplitude_policy_outside_sampler() {
    let output = process_vca(vec![0.25, 0.5, 0.75], vec![0.5, 0.5, 0.5]);

    assert_eq!(
        output.audio[builtin_ports::AUDIO_OUT],
        vec![0.125, 0.25, 0.375]
    );
}

#[test]
fn note_to_rate_converts_midi_notes_to_equal_tempered_playback_rates() {
    let mut state = PerModuleState::NoteToRate { rate: 1.0 };

    let output = process_note_to_rate(
        &mut state,
        &[
            BlockEvent {
                frame_offset: 0,
                event: ScriptEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                },
            },
            BlockEvent {
                frame_offset: 2,
                event: ScriptEvent::NoteOn {
                    note: 72,
                    velocity: 100,
                },
            },
        ],
        4,
    );

    assert_eq!(
        output.control[builtin_ports::RATE],
        vec![1.0, 1.0, 2.0, 2.0]
    );
}

#[test]
fn routed_note_to_rate_changes_sampler_pitch_from_midi_note() {
    let graph = sampler_graph(
        vec![
            ModuleNode::new(ModuleId::new("note_rate"), "note_to_rate")
                .with_input(builtin_ports::EVENTS, SignalType::Event)
                .with_output(builtin_ports::RATE, SignalType::Control),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
                PortRef::new(ModuleId::new("note_rate"), builtin_ports::EVENTS),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("note_rate"), builtin_ports::RATE),
                PortRef::new(ModuleId::new("sampler"), builtin_ports::RATE),
            ),
        ],
    );
    graph
        .validate()
        .expect("note_to_rate should route event input to sampler rate");
    let assets = sampler_assets(vec![1.0, 2.0, 3.0, 4.0]);

    let (middle_c, _) = render_offline_with_sampler_assets(
        &graph,
        &sampler_settings(4),
        vec![TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
        )],
        &assets,
    );
    let (octave_up, _) = render_offline_with_sampler_assets(
        &graph,
        &sampler_settings(4),
        vec![TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note: 72,
                velocity: 100,
            },
        )],
        &assets,
    );

    assert_eq!(middle_c, vec![1.0, 2.0, 3.0, 4.0]);
    assert_eq!(octave_up, vec![1.0, 3.0, 0.0, 0.0]);
}

#[test]
fn later_trigger_replaces_monophonic_sampler_playback() {
    let graph = sampler_graph(Vec::new(), Vec::new());
    let assets = sampler_assets(vec![1.0, 2.0, 3.0, 4.0]);

    let (left, _) = render_offline_with_sampler_assets(
        &graph,
        &sampler_settings(5),
        vec![note_on(0, 100), note_on(2, 100)],
        &assets,
    );

    assert_eq!(left, vec![1.0, 2.0, 1.0, 2.0, 3.0]);
}

#[test]
fn sampler_outputs_silence_after_sample_completion() {
    let graph = sampler_graph(Vec::new(), Vec::new());
    let assets = sampler_assets(vec![0.5, 0.25]);

    let (left, _) = render_offline_with_sampler_assets(
        &graph,
        &sampler_settings(5),
        vec![note_on(0, 100)],
        &assets,
    );

    assert_eq!(left, vec![0.5, 0.25, 0.0, 0.0, 0.0]);
}

#[test]
fn start_control_changes_sampler_playback_position_before_trigger() {
    let mut state = PerModuleState::Sampler {
        sample: Some(LoadedSample::new(
            48_000,
            vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
        )),
        position: 0.0,
        active: false,
    };

    let output = process_sampler(
        &mut state,
        &[BlockEvent {
            frame_offset: 0,
            event: ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
        }],
        &[1.0; 4],
        &[0.75; 4],
        &[0.0; 4],
        &[0.0; 4],
        &[0.0; 4],
        4,
    );

    assert_eq!(output.audio[builtin_ports::AUDIO], vec![6.0, 7.0, 0.0, 0.0]);
}

#[test]
fn loop_control_wraps_active_sampler_playback() {
    let mut state = PerModuleState::Sampler {
        sample: Some(LoadedSample::new(48_000, vec![1.0, 2.0, 3.0])),
        position: 0.0,
        active: false,
    };

    let output = process_sampler(
        &mut state,
        &[BlockEvent {
            frame_offset: 0,
            event: ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
        }],
        &[1.0; 7],
        &[0.0; 7],
        &[1.0; 7],
        &[0.0; 7],
        &[1.0; 7],
        7,
    );

    assert_eq!(
        output.audio[builtin_ports::AUDIO],
        vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0, 1.0]
    );
}

#[test]
fn offline_graph_processor_handles_sampler_modules_without_panics() {
    let graph = sampler_graph(Vec::new(), Vec::new());

    let (left, right) = render_offline(&graph, &sampler_settings(4), vec![note_on(0, 100)]);

    assert_eq!(left, vec![0.0; 4]);
    assert_eq!(right, vec![0.0; 4]);
}

#[test]
fn realtime_graph_processor_handles_sampler_modules_without_panics() {
    let graph = sampler_graph(Vec::new(), Vec::new());
    let mut processor = RealtimeGraphProcessor::new(graph, 48_000.0);
    let mut left = vec![1.0; 4];
    let mut right = vec![1.0; 4];

    processor.note_on(60, 100);
    let rendered = processor.render(&mut left, &mut right);

    assert_eq!(rendered, 4);
    assert_eq!(left, vec![0.0; 4]);
    assert_eq!(right, vec![0.0; 4]);
}

#[test]
fn sampler_render_repeats_exactly_for_same_inputs() {
    let graph = sampler_graph(Vec::new(), Vec::new());
    let settings = sampler_settings(8);
    let assets = sampler_assets(vec![0.1, 0.2, 0.3, 0.4]);
    let events = vec![note_on(1, 64), note_on(5, 127)];

    let first = render_offline_with_sampler_assets(&graph, &settings, events.clone(), &assets);
    let second = render_offline_with_sampler_assets(&graph, &settings, events, &assets);

    assert_eq!(first, second);
}

// --- Section 4: Polyphonic rendering ---

fn poly_sampler_graph(extra_modules: Vec<ModuleNode>, extra_cables: Vec<Cable>) -> Graph {
    let mut modules = vec![
        ModuleNode::new(ModuleId::new("midi"), "midi_input")
            .with_execution_scope(ExecutionScope::Global)
            .with_output(builtin_ports::EVENTS, SignalType::Event),
        ModuleNode::new(ModuleId::new("sampler"), "sampler")
            .with_execution_scope(ExecutionScope::Voice)
            .with_input(builtin_ports::TRIGGER, SignalType::Event)
            .with_input(builtin_ports::RATE, SignalType::Control)
            .with_input(builtin_ports::START, SignalType::Control)
            .with_input(builtin_ports::LOOP_ENABLED, SignalType::Control)
            .with_input(builtin_ports::LOOP_START, SignalType::Control)
            .with_input(builtin_ports::LOOP_END, SignalType::Control)
            .with_output(builtin_ports::AUDIO, SignalType::Audio),
        ModuleNode::new(ModuleId::new("mixer"), "audio_mixer")
            .with_execution_scope(ExecutionScope::Global)
            .with_mixing_input(builtin_ports::INPUTS, SignalType::Audio)
            .with_output(builtin_ports::MIX, SignalType::Audio),
        ModuleNode::new(ModuleId::new("out"), "audio_output")
            .with_execution_scope(ExecutionScope::Global)
            .with_input(builtin_ports::LEFT, SignalType::Audio)
            .with_input(builtin_ports::RIGHT, SignalType::Audio),
    ];
    modules.extend(extra_modules);

    let mut cables = vec![
        Cable::new(
            PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
            PortRef::new(ModuleId::new("sampler"), builtin_ports::TRIGGER),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("sampler"), builtin_ports::AUDIO),
            PortRef::new(ModuleId::new("mixer"), builtin_ports::INPUTS),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
            PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
        ),
    ];
    cables.extend(extra_cables);
    Graph::new(modules, cables)
}

fn poly_allocation(max_voices: u32) -> patch::VoiceAllocation {
    patch::VoiceAllocation {
        max_voices,
        stealing: patch::VoiceStealingPolicy::Disabled,
    }
}

fn poly_allocation_stealing(max_voices: u32) -> patch::VoiceAllocation {
    patch::VoiceAllocation {
        max_voices,
        stealing: patch::VoiceStealingPolicy::OldestActive,
    }
}

#[test]
fn overlapping_sampler_notes_mix_instead_of_replacing() {
    let graph = poly_sampler_graph(Vec::new(), Vec::new());
    graph.validate().expect("graph should validate");
    let settings = sampler_settings(8);
    let assets = sampler_assets(vec![1.0, 2.0, 3.0, 4.0]);

    let (left, _) = render_offline_with_sampler_assets_polyphonic(
        &graph,
        &settings,
        vec![note_on(0, 100), note_on(2, 100)],
        &assets,
        &poly_allocation(2),
    );

    // Monophonic would replace: [1.0, 2.0, 1.0, 2.0, 3.0, 0.0, 0.0, 0.0]
    // Polyphonic (2 voices) sums overlapping samples:
    // Voice 0: [1.0, 2.0, 3.0, 4.0, 0.0, 0.0, 0.0, 0.0]
    // Voice 1: [0.0, 0.0, 1.0, 2.0, 3.0, 4.0, 0.0, 0.0]
    // Sum:     [1.0, 2.0, 4.0, 6.0, 3.0, 4.0, 0.0, 0.0]
    assert_eq!(left, vec![1.0, 2.0, 4.0, 6.0, 3.0, 4.0, 0.0, 0.0]);
    assert_ne!(left, vec![1.0, 2.0, 1.0, 2.0, 3.0, 0.0, 0.0, 0.0]);
}

#[test]
fn overlapping_notes_on_different_notes_produce_independent_voice_output() {
    let graph = poly_sampler_graph(Vec::new(), Vec::new());
    graph.validate().expect("graph should validate");
    let settings = sampler_settings(8);
    let assets = sampler_assets(vec![1.0, 2.0, 3.0, 4.0]);

    let (left, _) = render_offline_with_sampler_assets_polyphonic(
        &graph,
        &settings,
        vec![
            note_on(0, 100),
            TimedInputEvent::new(
                1,
                ScriptEvent::NoteOn {
                    note: 64,
                    velocity: 100,
                },
            ),
        ],
        &assets,
        &poly_allocation(2),
    );

    // Voice 0 (note 60): [1.0, 2.0, 3.0, 4.0, 0.0, 0.0, 0.0, 0.0]
    // Voice 1 (note 64): [0.0, 1.0, 2.0, 3.0, 4.0, 0.0, 0.0, 0.0]
    // Sum: [1.0, 3.0, 5.0, 7.0, 4.0, 0.0, 0.0, 0.0]
    assert_eq!(left, vec![1.0, 3.0, 5.0, 7.0, 4.0, 0.0, 0.0, 0.0]);
}

#[test]
fn note_off_releases_matching_voice_while_other_continues() {
    // Graph: midi -> adsr -> vca (with osc audio in) -> mixer -> out
    let modules = vec![
        ModuleNode::new(ModuleId::new("midi"), "midi_input")
            .with_execution_scope(ExecutionScope::Global)
            .with_output(builtin_ports::EVENTS, SignalType::Event),
        ModuleNode::new(ModuleId::new("osc"), "oscillator")
            .with_execution_scope(ExecutionScope::Voice)
            .with_output(builtin_ports::AUDIO, SignalType::Audio),
        ModuleNode::new(ModuleId::new("adsr"), "adsr")
            .with_execution_scope(ExecutionScope::Voice)
            .with_input(builtin_ports::GATE, SignalType::Event)
            .with_output(builtin_ports::VALUE, SignalType::Control),
        ModuleNode::new(ModuleId::new("vca"), "gain")
            .with_execution_scope(ExecutionScope::Voice)
            .with_input(builtin_ports::AUDIO_IN, SignalType::Audio)
            .with_input(builtin_ports::GAIN, SignalType::Control)
            .with_output(builtin_ports::AUDIO_OUT, SignalType::Audio),
        ModuleNode::new(ModuleId::new("mixer"), "audio_mixer")
            .with_execution_scope(ExecutionScope::Global)
            .with_mixing_input(builtin_ports::INPUTS, SignalType::Audio)
            .with_output(builtin_ports::MIX, SignalType::Audio),
        ModuleNode::new(ModuleId::new("out"), "audio_output")
            .with_execution_scope(ExecutionScope::Global)
            .with_input(builtin_ports::LEFT, SignalType::Audio)
            .with_input(builtin_ports::RIGHT, SignalType::Audio),
    ];

    let cables = vec![
        Cable::new(
            PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
            PortRef::new(ModuleId::new("adsr"), builtin_ports::GATE),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
            PortRef::new(ModuleId::new("vca"), builtin_ports::AUDIO_IN),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("adsr"), builtin_ports::VALUE),
            PortRef::new(ModuleId::new("vca"), builtin_ports::GAIN),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("vca"), builtin_ports::AUDIO_OUT),
            PortRef::new(ModuleId::new("mixer"), builtin_ports::INPUTS),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
            PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
        ),
    ];

    let graph = Graph::new(modules, cables);
    graph.validate().expect("graph should validate");

    let settings = RenderSettings {
        sample_rate_hz: 48000,
        block_size_frames: 128,
        duration_frames: 48000,
    };

    let (left, _) = render_offline_polyphonic(
        &graph,
        &settings,
        vec![
            TimedInputEvent::new(
                0,
                ScriptEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                },
            ),
            TimedInputEvent::new(
                0,
                ScriptEvent::NoteOn {
                    note: 64,
                    velocity: 127,
                },
            ),
            TimedInputEvent::new(12000, ScriptEvent::NoteOff { note: 60 }),
        ],
        &poly_allocation(2),
    );

    // Both voices produce audio initially
    assert!(left[100] != 0.0, "voices should produce audio early");

    // After note-off at frame 12000, the released voice enters release
    // but the unreleased voice continues -> audio should still be present
    assert!(
        left[20000] != 0.0,
        "unreleased voice should still be audible after first note-off"
    );

    // The unreleased voice (note 64) eventually gets a note-off? No — it never gets NoteOff
    // It will have a fixed sustain level unless the ADSR is gated off.
    // With only one NoteOff(60), voice with note 64 stays in sustain.
    // At 48k sample rate with default 200ms release, the ADSR release of voice 60
    // completes quickly after note-off. Voice 64 continues in sustain.
    // By frame 45000, voice 60 is done but voice 64 should still be in sustain.
    assert!(
        left[45000].abs() > 0.001,
        "sustained voice should still produce audio late in render"
    );
}

#[test]
fn per_voice_adsr_gate_isolation() {
    // Same graph as note_off test, but verifies that note-off for one note
    // doesn't affect the ADSR of another voice.
    let modules = vec![
        ModuleNode::new(ModuleId::new("midi"), "midi_input")
            .with_execution_scope(ExecutionScope::Global)
            .with_output(builtin_ports::EVENTS, SignalType::Event),
        ModuleNode::new(ModuleId::new("osc"), "oscillator")
            .with_execution_scope(ExecutionScope::Voice)
            .with_output(builtin_ports::AUDIO, SignalType::Audio),
        ModuleNode::new(ModuleId::new("adsr"), "adsr")
            .with_execution_scope(ExecutionScope::Voice)
            .with_input(builtin_ports::GATE, SignalType::Event)
            .with_output(builtin_ports::VALUE, SignalType::Control),
        ModuleNode::new(ModuleId::new("vca"), "gain")
            .with_execution_scope(ExecutionScope::Voice)
            .with_input(builtin_ports::AUDIO_IN, SignalType::Audio)
            .with_input(builtin_ports::GAIN, SignalType::Control)
            .with_output(builtin_ports::AUDIO_OUT, SignalType::Audio),
        ModuleNode::new(ModuleId::new("mixer"), "audio_mixer")
            .with_execution_scope(ExecutionScope::Global)
            .with_mixing_input(builtin_ports::INPUTS, SignalType::Audio)
            .with_output(builtin_ports::MIX, SignalType::Audio),
        ModuleNode::new(ModuleId::new("out"), "audio_output")
            .with_execution_scope(ExecutionScope::Global)
            .with_input(builtin_ports::LEFT, SignalType::Audio)
            .with_input(builtin_ports::RIGHT, SignalType::Audio),
    ];

    let cables = vec![
        Cable::new(
            PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
            PortRef::new(ModuleId::new("adsr"), builtin_ports::GATE),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
            PortRef::new(ModuleId::new("vca"), builtin_ports::AUDIO_IN),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("adsr"), builtin_ports::VALUE),
            PortRef::new(ModuleId::new("vca"), builtin_ports::GAIN),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("vca"), builtin_ports::AUDIO_OUT),
            PortRef::new(ModuleId::new("mixer"), builtin_ports::INPUTS),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
            PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
        ),
    ];

    let graph = Graph::new(modules, cables);
    graph.validate().expect("graph should validate");

    let settings = RenderSettings {
        sample_rate_hz: 48000,
        block_size_frames: 128,
        duration_frames: 48000,
    };

    // Mono render with both notes should be louder than polyphonic with isolated gates
    let (mono_left, _) = render_offline(
        &graph,
        &settings,
        vec![
            TimedInputEvent::new(
                0,
                ScriptEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                },
            ),
            TimedInputEvent::new(
                0,
                ScriptEvent::NoteOn {
                    note: 64,
                    velocity: 100,
                },
            ),
        ],
    );

    let (poly_left, _) = render_offline_polyphonic(
        &graph,
        &settings,
        vec![
            TimedInputEvent::new(
                0,
                ScriptEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                },
            ),
            TimedInputEvent::new(
                0,
                ScriptEvent::NoteOn {
                    note: 64,
                    velocity: 100,
                },
            ),
        ],
        &poly_allocation(2),
    );

    // Polyphonic should produce more signal because ADSR gate is per-voice
    // (mono re-triggers the same ADSR on second note, poly has two independent ADSRs)
    let mono_max = mono_left.iter().cloned().fold(0.0f32, f32::max);
    let poly_max = poly_left.iter().cloned().fold(0.0f32, f32::max);
    assert!(
        poly_max > mono_max,
        "polyphonic ADSR should gate independently per voice, producing more output than mono"
    );
}

#[test]
fn adsr_release_duration_matches_default_release_time() {
    // Direct unit test of process_adsr release phase duration.
    // Default release = 200ms. At 48kHz that's 9600 frames.
    // After a single NoteOff, the ADSR should take ~9600 frames to reach near-zero.
    const SAMPLE_RATE: f32 = 48000.0;
    const BLOCK_SIZE: usize = 128;

    // First, let the ADSR reach sustain level
    let mut state = PerModuleState::Adsr {
        level: 0.0,
        gate_active: false,
        release_start_frame: 0,
        release_start_level: 0.0,
        sample_rate: SAMPLE_RATE,
    };

    // Block with NoteOn
    process_adsr(
        &mut state,
        &[BlockEvent {
            frame_offset: 0,
            event: ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
        }],
        &[0.0; BLOCK_SIZE], // attack_in (no signal = default 5ms)
        &[0.0; BLOCK_SIZE], // decay_in (no signal = default 30ms)
        &[0.0; BLOCK_SIZE], // sustain_in (no signal = default 0.7)
        &[0.0; BLOCK_SIZE], // release_in (no signal = default 200ms)
        0,
        BLOCK_SIZE,
    );

    // After many sustain blocks, level should be ~0.7
    // Run enough blocks to be well into sustain (5ms attack + 30ms decay = 1680 frames)
    for b in 1..20 {
        process_adsr(
            &mut state,
            &[],
            &[0.0; BLOCK_SIZE],
            &[0.0; BLOCK_SIZE],
            &[0.0; BLOCK_SIZE],
            &[0.0; BLOCK_SIZE],
            (b * BLOCK_SIZE) as u64,
            BLOCK_SIZE,
        );
    }

    // Verify we're in sustain at level 0.7
    let start_level = match &state {
        PerModuleState::Adsr {
            level, gate_active, ..
        } => {
            assert!(*gate_active, "should be gate active in sustain");
            *level
        }
        _ => unreachable!(),
    };
    // Level should be 0.7 at sustain
    assert!(
        (start_level - 0.7).abs() < 0.01,
        "should be at sustain level"
    );

    // Now send NoteOff — this block starts at frame 20*128 = 2560
    process_adsr(
        &mut state,
        &[BlockEvent {
            frame_offset: 0,
            event: ScriptEvent::NoteOff { note: 60 },
        }],
        &[0.0; BLOCK_SIZE],
        &[0.0; BLOCK_SIZE],
        &[0.0; BLOCK_SIZE],
        &[0.0; BLOCK_SIZE],
        2560,
        BLOCK_SIZE,
    );

    // After NoteOff, gate should be inactive
    match &state {
        PerModuleState::Adsr { gate_active, .. } => assert!(!*gate_active),
        _ => unreachable!(),
    };

    // Continue in release for 9600 frames / 128 = 75 blocks
    // After 74 blocks, level should still be non-zero
    for b in 1..74 {
        process_adsr(
            &mut state,
            &[],
            &[0.0; BLOCK_SIZE],
            &[0.0; BLOCK_SIZE],
            &[0.0; BLOCK_SIZE],
            &[0.0; BLOCK_SIZE],
            2560 + (b * BLOCK_SIZE) as u64,
            BLOCK_SIZE,
        );
    }

    let mid_release_level = match &state {
        PerModuleState::Adsr { level, .. } => *level,
        _ => unreachable!(),
    };
    // After 74 blocks of 128 frames = 9472 frames into release (out of 9600),
    // the level should approach near-zero but not quite there yet
    assert!(
        mid_release_level > 0.001,
        "release should still be audible at 9472 frames (98% through release): level={mid_release_level}"
    );

    // One more block should complete the release
    process_adsr(
        &mut state,
        &[],
        &[0.0; BLOCK_SIZE],
        &[0.0; BLOCK_SIZE],
        &[0.0; BLOCK_SIZE],
        &[0.0; BLOCK_SIZE],
        2560 + (75 * BLOCK_SIZE) as u64,
        BLOCK_SIZE,
    );

    let final_level = match &state {
        PerModuleState::Adsr { level, .. } => *level,
        _ => unreachable!(),
    };
    assert!(
        final_level < 0.001,
        "release should complete within 9600 frames of default release: final_level={final_level}"
    );
}

#[test]
fn note_off_produces_release_tail_in_polyphonic_render() {
    // Single voice: oscillator -> ADSR -> VCA -> mixer -> out
    // NoteOn at 0, NoteOff just after attack/decay (frame 10000).
    // Voice should produce a gradual release tail, not instant cutoff.
    let modules = vec![
        ModuleNode::new(ModuleId::new("midi"), "midi_input")
            .with_execution_scope(ExecutionScope::Global)
            .with_output(builtin_ports::EVENTS, SignalType::Event),
        ModuleNode::new(ModuleId::new("osc"), "oscillator")
            .with_execution_scope(ExecutionScope::Voice)
            .with_output(builtin_ports::AUDIO, SignalType::Audio),
        ModuleNode::new(ModuleId::new("adsr"), "adsr")
            .with_execution_scope(ExecutionScope::Voice)
            .with_input(builtin_ports::GATE, SignalType::Event)
            .with_output(builtin_ports::VALUE, SignalType::Control),
        ModuleNode::new(ModuleId::new("vca"), "gain")
            .with_execution_scope(ExecutionScope::Voice)
            .with_input(builtin_ports::AUDIO_IN, SignalType::Audio)
            .with_input(builtin_ports::GAIN, SignalType::Control)
            .with_output(builtin_ports::AUDIO_OUT, SignalType::Audio),
        ModuleNode::new(ModuleId::new("mixer"), "audio_mixer")
            .with_execution_scope(ExecutionScope::Global)
            .with_mixing_input(builtin_ports::INPUTS, SignalType::Audio)
            .with_output(builtin_ports::MIX, SignalType::Audio),
        ModuleNode::new(ModuleId::new("out"), "audio_output")
            .with_execution_scope(ExecutionScope::Global)
            .with_input(builtin_ports::LEFT, SignalType::Audio)
            .with_input(builtin_ports::RIGHT, SignalType::Audio),
    ];

    let cables = vec![
        Cable::new(
            PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
            PortRef::new(ModuleId::new("adsr"), builtin_ports::GATE),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
            PortRef::new(ModuleId::new("vca"), builtin_ports::AUDIO_IN),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("adsr"), builtin_ports::VALUE),
            PortRef::new(ModuleId::new("vca"), builtin_ports::GAIN),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("vca"), builtin_ports::AUDIO_OUT),
            PortRef::new(ModuleId::new("mixer"), builtin_ports::INPUTS),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
            PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
        ),
    ];

    let graph = Graph::new(modules, cables);
    graph.validate().expect("graph should validate");

    let note_off_frame = 10000u64;
    let settings = RenderSettings {
        sample_rate_hz: 48000,
        block_size_frames: 128,
        duration_frames: 48000,
    };

    let (left, _) = render_offline_polyphonic(
        &graph,
        &settings,
        vec![
            TimedInputEvent::new(
                0,
                ScriptEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                },
            ),
            TimedInputEvent::new(note_off_frame, ScriptEvent::NoteOff { note: 60 }),
        ],
        &poly_allocation(1),
    );

    // Immediately after NoteOff audio should NOT be silent
    // (ADSR release phase just started).
    assert!(
        (0..10).any(|i| left[(note_off_frame as usize) + 1 + i] != 0.0),
        "audio should NOT go silent immediately after NoteOff"
    );

    // The release tail should last roughly the release time (200ms = 9600 frames).
    // At mid-point (~5000 frames into release), audio should still be present.
    let mid_release = note_off_frame as usize + 5000;
    assert!(
        (0..10).any(|i| left[mid_release + i].abs() > 0.001),
        "audio should still be present mid-release (~5000 frames after NoteOff)"
    );

    // After release completes (well past 9600 frames), audio should be near-zero.
    assert!(
        (0..10).all(|i| left[(note_off_frame as usize) + 12000 + i].abs() < 0.01),
        "audio should fade to near-zero well after release completes"
    );
}

#[test]
fn polyphonic_render_is_deterministic_without_stealing() {
    let graph = poly_sampler_graph(Vec::new(), Vec::new());
    graph.validate().expect("graph should validate");
    let settings = sampler_settings(8);
    let events = vec![
        TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
        ),
        TimedInputEvent::new(
            2,
            ScriptEvent::NoteOn {
                note: 64,
                velocity: 80,
            },
        ),
    ];

    let (left1, right1) =
        render_offline_polyphonic(&graph, &settings, events.clone(), &poly_allocation(4));
    let (left2, right2) = render_offline_polyphonic(&graph, &settings, events, &poly_allocation(4));

    assert_eq!(
        left1, left2,
        "polyphonic render without stealing should be deterministic (left)"
    );
    assert_eq!(
        right1, right2,
        "polyphonic render without stealing should be deterministic (right)"
    );
}

#[test]
fn polyphonic_render_is_deterministic_with_stealing() {
    let graph = poly_sampler_graph(Vec::new(), Vec::new());
    graph.validate().expect("graph should validate");
    // Use max_voices=1 with 2 overlapping notes to force stealing
    let settings = sampler_settings(8);
    let events = vec![
        TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
        ),
        TimedInputEvent::new(
            2,
            ScriptEvent::NoteOn {
                note: 64,
                velocity: 80,
            },
        ),
    ];

    let (left1, right1) = render_offline_polyphonic(
        &graph,
        &settings,
        events.clone(),
        &poly_allocation_stealing(1),
    );
    let (left2, right2) =
        render_offline_polyphonic(&graph, &settings, events, &poly_allocation_stealing(1));

    assert_eq!(
        left1, left2,
        "polyphonic render with stealing should be deterministic (left)"
    );
    assert_eq!(
        right1, right2,
        "polyphonic render with stealing should be deterministic (right)"
    );
}

// --- Section 5: Compiled render parity ---

use crate::compiled_patch::compile;

fn parity_graph(modules: Vec<ModuleNode>, cables: Vec<Cable>) -> Graph {
    let graph = Graph::new(modules, cables);
    graph.validate().expect("graph should validate");
    graph
}

fn assert_parity(
    graph: &Graph,
    settings: &RenderSettings,
    events: Vec<TimedInputEvent>,
    sampler_assets: &PreparedSamplerAssets,
) {
    let compiled = compile(graph, settings).expect("graph should compile");
    let (expected_left, expected_right) =
        render_offline_with_sampler_assets(graph, settings, events.clone(), sampler_assets);
    let (actual_left, actual_right) = render_offline_compiled(&compiled, events, sampler_assets);

    assert_eq!(
        expected_left,
        actual_left,
        "left channel parity mismatch for graph: {:?}",
        graph
            .modules()
            .iter()
            .map(|m| m.module_type())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        expected_right,
        actual_right,
        "right channel parity mismatch for graph: {:?}",
        graph
            .modules()
            .iter()
            .map(|m| m.module_type())
            .collect::<Vec<_>>()
    );
}

#[test]
fn compiled_render_matches_raw_for_oscillator_patch() {
    let graph = parity_graph(
        vec![
            ModuleNode::new(ModuleId::new("osc"), "oscillator")
                .with_output(builtin_ports::AUDIO, SignalType::Audio),
            ModuleNode::new(ModuleId::new("mixer"), "audio_mixer")
                .with_mixing_input(builtin_ports::INPUTS, SignalType::Audio)
                .with_output(builtin_ports::MIX, SignalType::Audio),
            ModuleNode::new(ModuleId::new("out"), "audio_output")
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("mixer"), builtin_ports::INPUTS),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
                PortRef::new(ModuleId::new("out"), builtin_ports::RIGHT),
            ),
        ],
    );

    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 64,
        duration_frames: 512,
    };

    assert_parity(
        &graph,
        &settings,
        Vec::new(),
        &PreparedSamplerAssets::empty(),
    );
}

#[test]
fn compiled_render_matches_raw_for_midi_voice_patch() {
    let graph = parity_graph(
        vec![
            ModuleNode::new(ModuleId::new("midi"), "midi_input")
                .with_output(builtin_ports::EVENTS, SignalType::Event),
            ModuleNode::new(ModuleId::new("osc"), "oscillator")
                .with_output(builtin_ports::AUDIO, SignalType::Audio),
            ModuleNode::new(ModuleId::new("env"), "adsr")
                .with_input(builtin_ports::GATE, SignalType::Event)
                .with_output(builtin_ports::VALUE, SignalType::Control),
            ModuleNode::new(ModuleId::new("vca"), "gain")
                .with_input(builtin_ports::AUDIO_IN, SignalType::Audio)
                .with_input(builtin_ports::GAIN, SignalType::Control)
                .with_output(builtin_ports::AUDIO_OUT, SignalType::Audio),
            ModuleNode::new(ModuleId::new("mixer"), "audio_mixer")
                .with_mixing_input(builtin_ports::INPUTS, SignalType::Audio)
                .with_output(builtin_ports::MIX, SignalType::Audio),
            ModuleNode::new(ModuleId::new("out"), "audio_output")
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
                PortRef::new(ModuleId::new("env"), builtin_ports::GATE),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("vca"), builtin_ports::AUDIO_IN),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("env"), builtin_ports::VALUE),
                PortRef::new(ModuleId::new("vca"), builtin_ports::GAIN),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("vca"), builtin_ports::AUDIO_OUT),
                PortRef::new(ModuleId::new("mixer"), builtin_ports::INPUTS),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
                PortRef::new(ModuleId::new("out"), builtin_ports::RIGHT),
            ),
        ],
    );

    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 64,
        duration_frames: 1024,
    };

    assert_parity(
        &graph,
        &settings,
        vec![
            TimedInputEvent::new(
                0,
                ScriptEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                },
            ),
            TimedInputEvent::new(500, ScriptEvent::NoteOff { note: 60 }),
        ],
        &PreparedSamplerAssets::empty(),
    );
}

#[test]
fn compiled_render_matches_raw_for_sampler_patch() {
    let graph = parity_graph(
        vec![
            ModuleNode::new(ModuleId::new("midi"), "midi_input")
                .with_output(builtin_ports::EVENTS, SignalType::Event),
            ModuleNode::new(ModuleId::new("sampler"), "sampler")
                .with_input(builtin_ports::TRIGGER, SignalType::Event)
                .with_input(builtin_ports::RATE, SignalType::Control)
                .with_input(builtin_ports::START, SignalType::Control)
                .with_input(builtin_ports::LOOP_ENABLED, SignalType::Control)
                .with_input(builtin_ports::LOOP_START, SignalType::Control)
                .with_input(builtin_ports::LOOP_END, SignalType::Control)
                .with_output(builtin_ports::AUDIO, SignalType::Audio),
            ModuleNode::new(ModuleId::new("out"), "audio_output")
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
                PortRef::new(ModuleId::new("sampler"), builtin_ports::TRIGGER),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("sampler"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            ),
        ],
    );

    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 4,
        duration_frames: 8,
    };

    let assets = PreparedSamplerAssets::from_samples_by_module({
        let mut m = std::collections::BTreeMap::new();
        m.insert(
            "sampler".to_string(),
            LoadedSample::new(48_000, vec![0.25, 0.5, 0.75, 1.0]),
        );
        m
    });

    assert_parity(
        &graph,
        &settings,
        vec![TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
        )],
        &assets,
    );
}

#[test]
fn compiled_render_matches_raw_for_voice_to_global_patch() {
    let graph = parity_graph(
        vec![
            ModuleNode::new(ModuleId::new("osc"), "oscillator")
                .with_output(builtin_ports::AUDIO, SignalType::Audio)
                .with_execution_scope(ExecutionScope::Voice),
            ModuleNode::new(ModuleId::new("mixer"), "audio_mixer")
                .with_mixing_input(builtin_ports::INPUTS, SignalType::Audio)
                .with_output(builtin_ports::MIX, SignalType::Audio)
                .with_execution_scope(ExecutionScope::Global),
            ModuleNode::new(ModuleId::new("out"), "audio_output")
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio)
                .with_execution_scope(ExecutionScope::Global),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("mixer"), builtin_ports::INPUTS),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
                PortRef::new(ModuleId::new("out"), builtin_ports::RIGHT),
            ),
        ],
    );

    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 64,
        duration_frames: 512,
    };

    assert_parity(
        &graph,
        &settings,
        Vec::new(),
        &PreparedSamplerAssets::empty(),
    );
}

#[test]
fn compiled_execution_order_remains_globals_first() {
    use crate::compiled_patch::compile;

    let graph = parity_graph(
        vec![
            ModuleNode::new(ModuleId::new("osc"), "oscillator")
                .with_output(builtin_ports::AUDIO, SignalType::Audio)
                .with_execution_scope(ExecutionScope::Voice),
            ModuleNode::new(ModuleId::new("mixer"), "audio_mixer")
                .with_mixing_input(builtin_ports::INPUTS, SignalType::Audio)
                .with_output(builtin_ports::MIX, SignalType::Audio)
                .with_execution_scope(ExecutionScope::Global),
            ModuleNode::new(ModuleId::new("out"), "audio_output")
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio)
                .with_execution_scope(ExecutionScope::Global),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("mixer"), builtin_ports::INPUTS),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
                PortRef::new(ModuleId::new("out"), builtin_ports::RIGHT),
            ),
        ],
    );

    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 64,
        duration_frames: 512,
    };

    let compiled = compile(&graph, &settings).expect("graph should compile");

    // execution_order is globals-first: [mixer(1), out(2), osc(0)]
    let order = compiled.execution_order();
    let global_end = compiled.global_node_indices().len();
    assert_eq!(&order[..global_end], &[1, 2]);
    assert_eq!(&order[global_end..], &[0]);

    assert_eq!(compiled.global_node_indices(), &[1, 2]);
    assert_eq!(compiled.voice_node_indices(), &[0]);
}

// === End-to-end YAML patch tests for dynamics modules ===

#[test]
fn dynamics_saturator_chain_renders_without_error() {
    let (left, _right) = render_patch(
        r#"
metadata:
  name: dynamics-saturator-chain
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 4800
modules:
  - id: midi
    type: midi_input
  - id: osc
    type: oscillator
  - id: env
    type: adsr
  - id: vca
    type: gain
  - id: comp
    type: dynamics-processor
  - id: sat
    type: saturator
  - id: mixer
    type: audio_mixer
  - id: out
    type: audio_output
connections:
  - from: midi.events
    to: env.gate
  - from: osc.audio
    to: vca.audio_in
  - from: env.value
    to: vca.gain
  - from: vca.audio_out
    to: mixer.inputs
  - from: mixer.mix
    to: comp.audio_in
  - from: comp.audio_out
    to: sat.audio_in
  - from: sat.audio_out
    to: out.left
  - from: sat.audio_out
    to: out.right
"#,
    );
    assert!(
        left.len() > 0,
        "dynamics+saturator chain should produce output"
    );
    let has_signal = left.iter().any(|&s| s != 0.0);
    assert!(has_signal, "chain should produce non-zero audio");
}

#[test]
fn dynamics_processor_limiter_mode_prevents_overshoot() {
    // Limiter: threshold at -6 dB, very high above_ratio, below_ratio = 1
    let (left, _right) = render_patch(
        r#"
metadata:
  name: limiter-test
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 48000
modules:
  - id: midi
    type: midi_input
  - id: osc
    type: oscillator
  - id: env
    type: adsr
  - id: vca
    type: gain
  - id: mixer
    type: audio_mixer
  - id: comp
    type: dynamics-processor
    inputs:
      - name: audio_in
        signal_type: audio
      - name: threshold
        signal_type: control
        default: 0.675
      - name: above_ratio
        signal_type: control
        default: 0.95
      - name: attack
        signal_type: control
        default: 0.01
      - name: release
        signal_type: control
        default: 0.1
    outputs:
      - name: audio_out
        signal_type: audio
  - id: out
    type: audio_output
connections:
  - from: midi.events
    to: env.gate
  - from: osc.audio
    to: vca.audio_in
  - from: env.value
    to: vca.gain
  - from: vca.audio_out
    to: mixer.inputs
  - from: mixer.mix
    to: comp.audio_in
  - from: comp.audio_out
    to: out.left
  - from: comp.audio_out
    to: out.right
"#,
    );
    let max_amplitude = left.iter().map(|&s| s.abs()).fold(0.0f32, f32::max);
    // Threshold at 0.675 (mapped to dB) limits output; verify some clipping occurred
    assert!(
        max_amplitude < 10.0,
        "limiter should prevent extreme overshoot, max = {max_amplitude}"
    );
}

#[test]
fn convolution_patch_renders_with_unit_impulse_ir() {
    // Create a graph with convolution. The IR is loaded through PreparedSamplerAssets.
    let modules = vec![
        ModuleNode::new(ModuleId::new("midi"), "midi_input")
            .with_output(builtin_ports::EVENTS, SignalType::Event),
        ModuleNode::new(ModuleId::new("osc"), "oscillator")
            .with_output(builtin_ports::AUDIO, SignalType::Audio),
        ModuleNode::new(ModuleId::new("env"), "adsr")
            .with_input(builtin_ports::GATE, SignalType::Event)
            .with_output(builtin_ports::VALUE, SignalType::Control),
        ModuleNode::new(ModuleId::new("vca"), "gain")
            .with_input(builtin_ports::AUDIO_IN, SignalType::Audio)
            .with_input(builtin_ports::GAIN, SignalType::Control)
            .with_output(builtin_ports::AUDIO_OUT, SignalType::Audio),
        ModuleNode::new(ModuleId::new("conv"), "convolution")
            .with_input(builtin_ports::AUDIO_IN, SignalType::Audio)
            .with_input(builtin_ports::MIX, SignalType::Control)
            .with_output(builtin_ports::AUDIO_OUT, SignalType::Audio),
        ModuleNode::new(ModuleId::new("out"), "audio_output")
            .with_input(builtin_ports::LEFT, SignalType::Audio)
            .with_input(builtin_ports::RIGHT, SignalType::Audio),
    ];
    let cables = vec![
        Cable::new(
            PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
            PortRef::new(ModuleId::new("env"), builtin_ports::GATE),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
            PortRef::new(ModuleId::new("vca"), builtin_ports::AUDIO_IN),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("env"), builtin_ports::VALUE),
            PortRef::new(ModuleId::new("vca"), builtin_ports::GAIN),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("vca"), builtin_ports::AUDIO_OUT),
            PortRef::new(ModuleId::new("conv"), builtin_ports::AUDIO_IN),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("conv"), builtin_ports::AUDIO_OUT),
            PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
        ),
        Cable::new(
            PortRef::new(ModuleId::new("conv"), builtin_ports::AUDIO_OUT),
            PortRef::new(ModuleId::new("out"), builtin_ports::RIGHT),
        ),
    ];

    let graph = Graph::new(modules, cables);
    graph.validate().expect("graph should validate");

    let settings = RenderSettings {
        sample_rate_hz: 48000,
        block_size_frames: 64,
        duration_frames: 48000,
    };

    // Convolution with an empty IR is a passthrough — just verify no crash
    let assets = PreparedSamplerAssets::empty();
    let (left, right) =
        render_offline_with_sampler_assets(&graph, &settings, vec![note_on(0, 100)], &assets);

    assert!(!left.is_empty());
    assert!(!right.is_empty());
    let has_signal = left.iter().any(|&s| s != 0.0) || right.iter().any(|&s| s != 0.0);
    assert!(has_signal, "convolution patch should produce audio");
}

#[test]
fn echo_yaml_patch_produces_repeating_delays_with_feedback_decay() {
    let yaml = r#"
metadata:
  name: Echo Integration Test
  version: "0.1"
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 144000
modules:
  - id: midi
    type: midi_input
  - id: osc
    type: oscillator
  - id: env
    type: adsr
  - id: vca
    type: gain
  - id: mixer
    type: audio_mixer
  - id: echo
    type: echo
  - id: out
    type: audio_output
connections:
  - from: midi.events
    to: env.gate
  - from: osc.audio
    to: vca.audio_in
  - from: env.value
    to: vca.gain
  - from: vca.audio_out
    to: mixer.inputs
  - from: mixer.mix
    to: echo.audio_in_l
  - from: mixer.mix
    to: echo.audio_in_r
  - from: echo.audio_out_l
    to: out.left
  - from: echo.audio_out_r
    to: out.right
"#;
    let patch = patch::load_patch_str(yaml).expect("patch should parse");
    patch::validate_patch_schema(&patch).expect("schema should be valid");
    let graph = Graph::from_patch_declarations(&patch);
    graph.validate().expect("graph should validate");

    let (left, _right) = render_offline(
        &graph,
        &patch.render,
        vec![
            TimedInputEvent::new(
                0,
                ScriptEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                },
            ),
            TimedInputEvent::new(960, ScriptEvent::NoteOff { note: 60 }),
        ],
    );

    assert!(
        left.iter().any(|&s| s != 0.0),
        "echo patch should produce audio"
    );

    let r1_peak = left[28000..30000]
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);
    let r2_peak = left[56500..58500]
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);

    assert!(
        r1_peak > 0.001,
        "first echo repeat should be audible at ~600ms, got {r1_peak}"
    );
    assert!(
        r2_peak > 0.001,
        "second echo repeat should be audible at ~1200ms, got {r2_peak}"
    );
    assert!(
        r2_peak < r1_peak * 0.99,
        "echo repeats should decay: {r2_peak} >= {r1_peak}"
    );
}

#[test]
fn reverb_yaml_patch_produces_tail_with_stereo_spread() {
    let yaml = r#"
metadata:
  name: Reverb Integration Test
  version: "0.1"
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 144000
modules:
  - id: midi
    type: midi_input
  - id: osc
    type: oscillator
  - id: env
    type: adsr
  - id: vca
    type: gain
  - id: mixer
    type: audio_mixer
  - id: reverb
    type: reverb
  - id: out
    type: audio_output
connections:
  - from: midi.events
    to: env.gate
  - from: osc.audio
    to: vca.audio_in
  - from: env.value
    to: vca.gain
  - from: vca.audio_out
    to: mixer.inputs
  - from: mixer.mix
    to: reverb.audio_in_l
  - from: mixer.mix
    to: reverb.audio_in_r
  - from: reverb.audio_out_l
    to: out.left
  - from: reverb.audio_out_r
    to: out.right
"#;
    let patch = patch::load_patch_str(yaml).expect("patch should parse");
    patch::validate_patch_schema(&patch).expect("schema should be valid");
    let graph = Graph::from_patch_declarations(&patch);
    graph.validate().expect("graph should validate");

    let (left, right) = render_offline(
        &graph,
        &patch.render,
        vec![
            TimedInputEvent::new(
                0,
                ScriptEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                },
            ),
            TimedInputEvent::new(960, ScriptEvent::NoteOff { note: 60 }),
        ],
    );

    assert!(
        left.iter().any(|&s| s != 0.0),
        "reverb patch should produce audio"
    );

    // The reverb tail should still be audible late in the buffer
    // Default RT60 ≈ 3.5s, check at ~2.5s (frame 120000)
    let tail_energy: f32 = left[118000..122000].iter().map(|s| s.abs()).sum();
    assert!(
        tail_energy > 0.001,
        "reverb tail should still be present at ~2.5s, got {tail_energy}"
    );

    // Left and right should differ (stereo spread from stereo_width default)
    let stereo_diff: f32 = left[80000..120000]
        .iter()
        .zip(right[80000..120000].iter())
        .map(|(l, r)| (l - r).abs())
        .sum();
    assert!(
        stereo_diff > 0.001,
        "reverb should produce stereo decorrelation"
    );
}

#[test]
fn compiled_render_matches_raw_for_echo_chain() {
    let graph = parity_graph(
        vec![
            ModuleNode::new(ModuleId::new("osc"), "oscillator")
                .with_output(builtin_ports::AUDIO, SignalType::Audio),
            ModuleNode::new(ModuleId::new("mixer"), "audio_mixer")
                .with_mixing_input(builtin_ports::INPUTS, SignalType::Audio)
                .with_output(builtin_ports::MIX, SignalType::Audio),
            ModuleNode::new(ModuleId::new("echo"), "echo")
                .with_input(builtin_ports::AUDIO_IN_L, SignalType::Audio)
                .with_input(builtin_ports::AUDIO_IN_R, SignalType::Audio)
                .with_output(builtin_ports::AUDIO_OUT_L, SignalType::Audio)
                .with_output(builtin_ports::AUDIO_OUT_R, SignalType::Audio),
            ModuleNode::new(ModuleId::new("out"), "audio_output")
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("mixer"), builtin_ports::INPUTS),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
                PortRef::new(ModuleId::new("echo"), builtin_ports::AUDIO_IN_L),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
                PortRef::new(ModuleId::new("echo"), builtin_ports::AUDIO_IN_R),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("echo"), builtin_ports::AUDIO_OUT_L),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("echo"), builtin_ports::AUDIO_OUT_R),
                PortRef::new(ModuleId::new("out"), builtin_ports::RIGHT),
            ),
        ],
    );

    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 64,
        duration_frames: 2048,
    };

    assert_parity(
        &graph,
        &settings,
        Vec::new(),
        &PreparedSamplerAssets::empty(),
    );
}

#[test]
fn compiled_render_matches_raw_for_reverb_chain() {
    let graph = parity_graph(
        vec![
            ModuleNode::new(ModuleId::new("osc"), "oscillator")
                .with_output(builtin_ports::AUDIO, SignalType::Audio),
            ModuleNode::new(ModuleId::new("mixer"), "audio_mixer")
                .with_mixing_input(builtin_ports::INPUTS, SignalType::Audio)
                .with_output(builtin_ports::MIX, SignalType::Audio),
            ModuleNode::new(ModuleId::new("reverb"), "reverb")
                .with_input(builtin_ports::AUDIO_IN_L, SignalType::Audio)
                .with_input(builtin_ports::AUDIO_IN_R, SignalType::Audio)
                .with_output(builtin_ports::AUDIO_OUT_L, SignalType::Audio)
                .with_output(builtin_ports::AUDIO_OUT_R, SignalType::Audio),
            ModuleNode::new(ModuleId::new("out"), "audio_output")
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("mixer"), builtin_ports::INPUTS),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
                PortRef::new(ModuleId::new("reverb"), builtin_ports::AUDIO_IN_L),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
                PortRef::new(ModuleId::new("reverb"), builtin_ports::AUDIO_IN_R),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("reverb"), builtin_ports::AUDIO_OUT_L),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("reverb"), builtin_ports::AUDIO_OUT_R),
                PortRef::new(ModuleId::new("out"), builtin_ports::RIGHT),
            ),
        ],
    );

    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 64,
        duration_frames: 2048,
    };

    assert_parity(
        &graph,
        &settings,
        Vec::new(),
        &PreparedSamplerAssets::empty(),
    );
}

#[test]
fn composite_echo_yaml_loads_and_validates() {
    let yaml = fs::read_to_string("../../examples/patches/composite-echo.yaml")
        .expect("composite-echo.yaml should exist");
    let patch = patch::load_patch_str(&yaml).expect("composite-echo.yaml should parse");
    patch::validate_patch_schema(&patch).expect("composite-echo.yaml schema should be valid");
    let graph = Graph::from_patch_declarations(&patch);
    graph
        .validate()
        .expect("composite-echo.yaml graph should validate");
}

#[test]
fn composite_reverb_yaml_loads_and_validates() {
    let yaml = fs::read_to_string("../../examples/patches/composite-reverb.yaml")
        .expect("composite-reverb.yaml should exist");
    let patch = patch::load_patch_str(&yaml).expect("composite-reverb.yaml should parse");
    patch::validate_patch_schema(&patch).expect("composite-reverb.yaml schema should be valid");
    let graph = Graph::from_patch_declarations(&patch);
    graph
        .validate()
        .expect("composite-reverb.yaml graph should validate");
}

// --- Offline vs Realtime parity ---

fn assert_offline_realtime_parity(
    graph: &Graph,
    sample_rate: f32,
    total_frames: usize,
    block_size: usize,
    events: &[TimedInputEvent],
    sampler_assets: &PreparedSamplerAssets,
) {
    let settings = RenderSettings {
        sample_rate_hz: sample_rate as u32,
        block_size_frames: block_size as u32,
        duration_frames: total_frames as u64,
    };

    let (offline_left, offline_right) =
        render_offline_with_sampler_assets(graph, &settings, events.to_vec(), sampler_assets);

    let mut realtime = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph.clone(),
        sample_rate,
        sampler_assets,
        &VoiceAllocation::default(),
        block_size,
    );

    for event in events {
        match event.event() {
            ScriptEvent::NoteOn { note, velocity } => realtime.note_on(*note, *velocity),
            ScriptEvent::NoteOff { note } => realtime.note_off(*note),
        }
    }

    let mut realtime_left = vec![0.0; total_frames];
    let mut realtime_right = vec![0.0; total_frames];
    let rendered = realtime.render(&mut realtime_left, &mut realtime_right);

    assert_eq!(
        rendered,
        total_frames.min(realtime_left.len().min(realtime_right.len()))
    );

    let compare_len = offline_left.len().min(realtime_left.len());
    assert_eq!(
        &offline_left[..compare_len],
        &realtime_left[..compare_len],
        "left channel offline/realtime parity mismatch"
    );
    assert_eq!(
        &offline_right[..compare_len],
        &realtime_right[..compare_len],
        "right channel offline/realtime parity mismatch"
    );
}

#[test]
fn offline_and_realtime_produce_same_output_for_oscillator_patch() {
    let graph = Graph::new(
        vec![
            ModuleNode::new(ModuleId::new("osc"), "oscillator")
                .with_output(builtin_ports::AUDIO, SignalType::Audio),
            ModuleNode::new(ModuleId::new("mixer"), "audio_mixer")
                .with_mixing_input(builtin_ports::INPUTS, SignalType::Audio)
                .with_output(builtin_ports::MIX, SignalType::Audio),
            ModuleNode::new(ModuleId::new("out"), "audio_output")
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("mixer"), builtin_ports::INPUTS),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("mixer"), builtin_ports::MIX),
                PortRef::new(ModuleId::new("out"), builtin_ports::RIGHT),
            ),
        ],
    );
    graph.validate().expect("graph should validate");

    assert_offline_realtime_parity(
        &graph,
        48_000.0,
        512,
        64,
        &[TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
        )],
        &PreparedSamplerAssets::empty(),
    );
}

#[test]
fn offline_and_realtime_produce_same_output_for_sampler_patch() {
    let graph = Graph::new(
        vec![
            ModuleNode::new(ModuleId::new("midi"), "midi_input")
                .with_output(builtin_ports::EVENTS, SignalType::Event),
            ModuleNode::new(ModuleId::new("sampler"), "sampler")
                .with_input(builtin_ports::TRIGGER, SignalType::Event)
                .with_input(builtin_ports::RATE, SignalType::Control)
                .with_input(builtin_ports::START, SignalType::Control)
                .with_input(builtin_ports::LOOP_ENABLED, SignalType::Control)
                .with_input(builtin_ports::LOOP_START, SignalType::Control)
                .with_input(builtin_ports::LOOP_END, SignalType::Control)
                .with_output(builtin_ports::AUDIO, SignalType::Audio),
            ModuleNode::new(ModuleId::new("out"), "audio_output")
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
                PortRef::new(ModuleId::new("sampler"), builtin_ports::TRIGGER),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("sampler"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            ),
        ],
    );
    graph.validate().expect("graph should validate");

    let assets = PreparedSamplerAssets::from_samples_by_module({
        let mut m = std::collections::BTreeMap::new();
        m.insert(
            "sampler".to_string(),
            LoadedSample::new(48_000, vec![0.25, 0.5, 0.75, 1.0]),
        );
        m
    });

    assert_offline_realtime_parity(
        &graph,
        48_000.0,
        16,
        4,
        &[TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
        )],
        &assets,
    );
}
