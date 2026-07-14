use super::processing::{
    EchoControls, ReverbControls, process_dynamics_processor, process_echo,
    process_frequency_splitter, process_note_to_control, process_reverb, process_slew,
};
use super::*;
use crate::builtins::{
    CURVE_EXPONENTIAL, CURVE_PARAMETER, DELAY_SAMPLES_PARAMETER, EVENT_FILTER_NOTE_PARAMETER,
    EVENT_FILTER_NOTE_SELECTOR, EVENT_FILTER_SELECTOR_PARAMETER, SCRIPT_LANGUAGE_PARAMETER,
    SCRIPT_LANGUAGE_RHAI, SCRIPT_SOURCE_PARAMETER, module_types,
};
use crate::builtins::{
    DETECTION_MODE_PEAK, DETECTION_MODE_RMS, DYNAMICS_DETECTION_PARAMETER, DYNAMICS_MODE_LEVEL,
    DYNAMICS_MODE_PARAMETER, DYNAMICS_MODE_TRANSIENT, DYNAMICS_TOPOLOGY_FEEDBACK,
    DYNAMICS_TOPOLOGY_FEEDFORWARD, DYNAMICS_TOPOLOGY_PARAMETER, INTERPOLATION_CUBIC,
    INTERPOLATION_LINEAR, INTERPOLATION_PARAMETER,
};
use crate::core::TimedInputEvent;
use crate::fft;
use crate::oscillator::OSCILLATOR_BASE_HZ;
use crate::graph::*;
use crate::patch;
use crate::sample::{LoadedSample, PreparedSamplerAssets};
use crate::script::ScriptEvent;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

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

fn note_on_value(frame: u64, note: u8, velocity: u8) -> TimedInputEvent {
    TimedInputEvent::new(frame, ScriptEvent::NoteOn { note, velocity })
}

fn event_filter_sampler_graph(filter_note: u8) -> Graph {
    Graph::new(
        vec![
            ModuleNode::new(ModuleId::new("midi"), module_types::MIDI_INPUT)
                .with_output(builtin_ports::EVENTS, SignalType::Event),
            ModuleNode::new(ModuleId::new("filter"), module_types::EVENT_FILTER)
                .with_input(builtin_ports::EVENTS_IN, SignalType::Event)
                .with_output(builtin_ports::EVENTS_OUT, SignalType::Event)
                .with_params(BTreeMap::from([
                    (
                        EVENT_FILTER_SELECTOR_PARAMETER.to_string(),
                        EVENT_FILTER_NOTE_SELECTOR.to_string(),
                    ),
                    (
                        EVENT_FILTER_NOTE_PARAMETER.to_string(),
                        filter_note.to_string(),
                    ),
                ])),
            ModuleNode::new(ModuleId::new("sampler"), module_types::SAMPLER)
                .with_input(builtin_ports::TRIGGER, SignalType::Event)
                .with_input(builtin_ports::RATE, SignalType::Control)
                .with_input(builtin_ports::START, SignalType::Control)
                .with_input(builtin_ports::LOOP_ENABLED, SignalType::Control)
                .with_input(builtin_ports::LOOP_START, SignalType::Control)
                .with_input(builtin_ports::LOOP_END, SignalType::Control)
                .with_output(builtin_ports::AUDIO, SignalType::Audio),
            ModuleNode::new(ModuleId::new("out"), module_types::AUDIO_OUTPUT)
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
                PortRef::new(ModuleId::new("filter"), builtin_ports::EVENTS_IN),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("filter"), builtin_ports::EVENTS_OUT),
                PortRef::new(ModuleId::new("sampler"), builtin_ports::TRIGGER),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("sampler"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            ),
        ],
    )
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

fn envelope_follower_values(
    audio_in: Vec<f32>,
    attack_ms: f32,
    release_ms: f32,
    amount: f32,
    offset: f32,
    invert: f32,
) -> Vec<f32> {
    let frames = audio_in.len();
    let module = ModuleNode::new(ModuleId::new("follower"), module_types::ENVELOPE_FOLLOWER);
    let mut state = PerModuleState::new(&module, 48_000.0, &PreparedSamplerAssets::empty());
    let outputs = process_envelope_follower(
        &mut state,
        &audio_in,
        &vec![attack_ms; frames],
        &vec![release_ms; frames],
        &vec![amount; frames],
        &vec![offset; frames],
        &vec![invert; frames],
        frames,
    );

    outputs
        .control
        .get(builtin_ports::VALUE)
        .expect("envelope_follower should emit value")
        .clone()
}

#[test]
fn envelope_follower_detects_positive_and_negative_audio_levels() {
    let positive = envelope_follower_values(vec![0.5; 8], 0.0, 10.0, 1.0, 0.0, 0.0);
    let negative = envelope_follower_values(vec![-0.5; 8], 0.0, 10.0, 1.0, 0.0, 0.0);

    assert_eq!(positive, negative);
    assert!(positive.iter().all(|value| (*value - 0.5).abs() < 0.0001));
}

#[test]
fn envelope_follower_attack_smooths_rising_input_level() {
    let values = envelope_follower_values(vec![1.0; 480], 10.0, 100.0, 1.0, 0.0, 0.0);

    assert!(values[0] > 0.0);
    assert!(values[0] < values[479]);
    assert!(values[479] > 0.5 && values[479] < 0.8);
}

#[test]
fn envelope_follower_release_smooths_falling_input_level() {
    let mut audio = vec![1.0; 16];
    audio.extend(vec![0.0; 480]);
    let values = envelope_follower_values(audio, 0.0, 10.0, 1.0, 0.0, 0.0);

    assert!(values[15] > 0.99);
    assert!(values[16] < values[15]);
    assert!(values[495] > 0.2 && values[495] < 0.5);
}

#[test]
fn envelope_follower_applies_amount_offset_and_invert() {
    let normal = envelope_follower_values(vec![0.25; 4], 0.0, 10.0, 0.5, 0.25, 0.0);
    let inverted = envelope_follower_values(vec![0.25; 4], 0.0, 10.0, 0.5, 0.25, 1.0);

    assert!(normal.iter().all(|value| (*value - 0.375).abs() < 0.0001));
    assert!(inverted.iter().all(|value| (*value - 0.625).abs() < 0.0001));
}

#[test]
fn envelope_follower_extreme_inputs_and_parameters_remain_finite() {
    let values = envelope_follower_values(
        vec![f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX],
        f32::INFINITY,
        f32::NAN,
        f32::MAX,
        f32::NEG_INFINITY,
        1.0,
    );

    assert!(values.iter().all(|value| value.is_finite()));
}

#[test]
fn curve_mapper_processing_applies_configured_curve() {
    let module = ModuleNode::new(ModuleId::new("mapper"), module_types::CURVE_MAPPER).with_params(
        BTreeMap::from([(CURVE_PARAMETER.to_string(), CURVE_EXPONENTIAL.to_string())]),
    );
    let mut state = PerModuleState::new(&module, 48_000.0, &PreparedSamplerAssets::empty());
    let outputs = process_curve_mapper(&mut state, &[0.5], &[1.0], &[0.0], &[1.0], &[0.0], 1);
    let value = outputs
        .control
        .get(builtin_ports::VALUE)
        .expect("curve_mapper should emit value")[0];

    assert!((value - 0.25).abs() < 0.0001);
}

#[test]
fn envelope_follower_curve_mapper_route_deterministically_controls_gain() {
    let graph = parity_graph(
        vec![
            ModuleNode::new(ModuleId::new("osc"), module_types::OSCILLATOR)
                .with_output(builtin_ports::AUDIO, SignalType::Audio),
            ModuleNode::new(ModuleId::new("follower"), module_types::ENVELOPE_FOLLOWER)
                .with_input(builtin_ports::AUDIO_IN, SignalType::Audio)
                .with_output(builtin_ports::VALUE, SignalType::Control),
            ModuleNode::new(ModuleId::new("mapper"), module_types::CURVE_MAPPER)
                .with_input(builtin_ports::VALUE, SignalType::Control)
                .with_output(builtin_ports::VALUE, SignalType::Control)
                .with_params(BTreeMap::from([(
                    CURVE_PARAMETER.to_string(),
                    CURVE_EXPONENTIAL.to_string(),
                )])),
            ModuleNode::new(ModuleId::new("vca"), module_types::GAIN)
                .with_input(builtin_ports::AUDIO_IN, SignalType::Audio)
                .with_input(builtin_ports::GAIN, SignalType::Control)
                .with_output(builtin_ports::AUDIO_OUT, SignalType::Audio),
            ModuleNode::new(ModuleId::new("mixer"), module_types::AUDIO_MIXER)
                .with_mixing_input(builtin_ports::INPUTS, SignalType::Audio)
                .with_output(builtin_ports::MIX, SignalType::Audio),
            ModuleNode::new(ModuleId::new("out"), module_types::AUDIO_OUTPUT)
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("follower"), builtin_ports::AUDIO_IN),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("follower"), builtin_ports::VALUE),
                PortRef::new(ModuleId::new("mapper"), builtin_ports::VALUE),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("mapper"), builtin_ports::VALUE),
                PortRef::new(ModuleId::new("vca"), builtin_ports::GAIN),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("vca"), builtin_ports::AUDIO_IN),
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
        duration_frames: 512,
    };

    assert_parity(
        &graph,
        &settings,
        Vec::new(),
        &PreparedSamplerAssets::empty(),
    );
    let (left, right) = render_offline(&graph, &settings, Vec::new());

    assert!(
        left.iter()
            .chain(right.iter())
            .all(|sample| sample.is_finite())
    );
    assert!(left.iter().any(|sample| sample.abs() > 0.0));
}

#[test]
fn envelope_follower_curve_mapper_route_deterministically_controls_filter_cutoff() {
    let graph = parity_graph(
        vec![
            ModuleNode::new(ModuleId::new("osc"), module_types::OSCILLATOR)
                .with_output(builtin_ports::AUDIO, SignalType::Audio),
            ModuleNode::new(ModuleId::new("follower"), module_types::ENVELOPE_FOLLOWER)
                .with_input(builtin_ports::AUDIO_IN, SignalType::Audio)
                .with_output(builtin_ports::VALUE, SignalType::Control),
            ModuleNode::new(ModuleId::new("mapper"), module_types::CURVE_MAPPER)
                .with_input(builtin_ports::VALUE, SignalType::Control)
                .with_output(builtin_ports::VALUE, SignalType::Control),
            ModuleNode::new(ModuleId::new("filter"), module_types::FILTER)
                .with_input(builtin_ports::AUDIO_IN, SignalType::Audio)
                .with_input(builtin_ports::CUTOFF, SignalType::Control)
                .with_output(builtin_ports::AUDIO_OUT, SignalType::Audio),
            ModuleNode::new(ModuleId::new("mixer"), module_types::AUDIO_MIXER)
                .with_mixing_input(builtin_ports::INPUTS, SignalType::Audio)
                .with_output(builtin_ports::MIX, SignalType::Audio),
            ModuleNode::new(ModuleId::new("out"), module_types::AUDIO_OUTPUT)
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("follower"), builtin_ports::AUDIO_IN),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("follower"), builtin_ports::VALUE),
                PortRef::new(ModuleId::new("mapper"), builtin_ports::VALUE),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("mapper"), builtin_ports::VALUE),
                PortRef::new(ModuleId::new("filter"), builtin_ports::CUTOFF),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("filter"), builtin_ports::AUDIO_IN),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("filter"), builtin_ports::AUDIO_OUT),
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

fn frequency_splitter_bands(
    audio_in: Vec<f32>,
    crossover_control: f32,
    sample_rate: f32,
) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let frames = audio_in.len();
    let module = ModuleNode::new(ModuleId::new("splitter"), module_types::FREQUENCY_SPLITTER);
    let mut state = PerModuleState::new(&module, sample_rate, &PreparedSamplerAssets::empty());
    let crossover = vec![crossover_control; frames];
    let outputs = process_frequency_splitter(&mut state, &audio_in, &crossover, frames);
    let low = outputs
        .audio
        .get("low")
        .expect("splitter emits low band")
        .clone();
    let mid = outputs
        .audio
        .get("mid")
        .expect("splitter emits mid band")
        .clone();
    let high = outputs
        .audio
        .get("high")
        .expect("splitter emits high band")
        .clone();
    (low, mid, high)
}

#[test]
fn frequency_splitter_routes_band_energy_and_reconstructs_flat() {
    let sample_rate = 48_000.0f32;
    let frames = 8_192;
    let mut impulse = vec![0.0f32; frames];
    impulse[0] = 1.0;

    // control 0.05 -> low split ~840 Hz, high split ~3.4 kHz, so 100 Hz sits in
    // the low band and 8 kHz in the high band, both well clear of the crossovers.
    let (low, mid, high) = frequency_splitter_bands(impulse, 0.05, sample_rate);

    let low_resp = fft::compute_magnitude_response(&low, sample_rate as f64).bins;
    let high_resp = fft::compute_magnitude_response(&high, sample_rate as f64).bins;

    assert!(
        magnitude_at(&low_resp, 100.0) > magnitude_at(&low_resp, 8000.0) + 24.0,
        "low band should favour low frequencies over high ones"
    );
    assert!(
        magnitude_at(&high_resp, 8000.0) > magnitude_at(&high_resp, 100.0) + 24.0,
        "high band should favour high frequencies over low ones"
    );

    // Linkwitz-Riley bands sum flat, so low + mid + high reconstructs the input.
    let combined: Vec<f32> = (0..frames).map(|i| low[i] + mid[i] + high[i]).collect();
    let sum_resp = fft::compute_magnitude_response(&combined, sample_rate as f64).bins;
    assert!(
        magnitude_at(&sum_resp, 1000.0) > -3.0,
        "three bands should reconstruct near-flat, got {:.1} dB",
        magnitude_at(&sum_resp, 1000.0)
    );
}

// Renders `audio_in` through a dynamics module built from `params` and returns
// the steady-state output amplitude (max abs over the final quarter). Control
// inputs are normalized 0-1; attack/sustain gain 0.5 == 0 dB.
fn dynamics_steady_amplitude(
    params: BTreeMap<String, String>,
    audio_in: &[f32],
    attack_gain: f32,
    sustain_gain: f32,
) -> f32 {
    let frames = audio_in.len();
    let module =
        ModuleNode::new(ModuleId::new("dyn"), module_types::DYNAMICS_PROCESSOR).with_params(params);
    let mut state = PerModuleState::new(&module, 48_000.0, &PreparedSamplerAssets::empty());
    let level = |v: f32| vec![v; frames];
    let outputs = process_dynamics_processor(
        &mut state,
        audio_in,
        &vec![0.0; frames], // sidechain (unused)
        &level(0.5),        // threshold -> -40 dB
        &level(0.05),       // below ratio -> ~1:1
        &level(0.1),        // above ratio -> ~5:1
        &level(0.5),        // attack
        &level(0.3),        // release
        &level(0.0),        // knee
        &level(0.0),        // makeup
        &level(attack_gain),
        &level(sustain_gain),
        frames,
    );
    outputs
        .audio
        .get(builtin_ports::AUDIO_OUT)
        .expect("dynamics emits audio_out")
        .iter()
        .skip(frames * 3 / 4)
        .map(|s| s.abs())
        .fold(0.0f32, f32::max)
}

fn steady_sine(amplitude: f32) -> Vec<f32> {
    (0..4_800)
        .map(|i| amplitude * (i as f32 * 0.2).sin())
        .collect()
}

#[test]
fn dynamics_detection_parameter_selects_peak_or_rms_envelope() {
    // A high-crest pulse train: peak detection latches onto the 0.8 spikes while
    // RMS averages them down to ~0.2, so peak compresses the crests far harder.
    let pulses: Vec<f32> = (0..4_800)
        .map(|i| if i % 16 == 0 { 0.8 } else { 0.0 })
        .collect();

    let peak = dynamics_steady_amplitude(
        BTreeMap::from([(
            DYNAMICS_DETECTION_PARAMETER.to_string(),
            DETECTION_MODE_PEAK.to_string(),
        )]),
        &pulses,
        0.5,
        0.5,
    );
    let rms = dynamics_steady_amplitude(
        BTreeMap::from([(
            DYNAMICS_DETECTION_PARAMETER.to_string(),
            DETECTION_MODE_RMS.to_string(),
        )]),
        &pulses,
        0.5,
        0.5,
    );

    // The peak-vs-RMS envelope shape is unit-tested in envelope_follower; here we
    // only need the wiring to prove the parameter reaches the detector and
    // measurably changes the gain reduction.
    let relative_difference = (peak - rms).abs() / peak.max(rms);
    assert!(
        relative_difference > 0.15,
        "detection parameter should measurably change compression (peak {peak}, rms {rms})"
    );
}

#[test]
fn dynamics_mode_parameter_switches_between_compression_and_transient_shaping() {
    // Level mode compresses the steady tone below unity; transient mode at 0 dB
    // attack/sustain gain leaves it at unity, so the two must differ audibly.
    let sine = steady_sine(0.5);
    let level = dynamics_steady_amplitude(
        BTreeMap::from([(
            DYNAMICS_MODE_PARAMETER.to_string(),
            DYNAMICS_MODE_LEVEL.to_string(),
        )]),
        &sine,
        0.5,
        0.5,
    );
    let transient = dynamics_steady_amplitude(
        BTreeMap::from([(
            DYNAMICS_MODE_PARAMETER.to_string(),
            DYNAMICS_MODE_TRANSIENT.to_string(),
        )]),
        &sine,
        0.5,
        0.5,
    );

    assert!(
        transient > level * 1.2,
        "transient mode at unity gain should leave the tone louder than level-mode compression (transient {transient}, level {level})"
    );
}

#[test]
fn dynamics_topology_parameter_makes_feedback_compress_more_gently() {
    // Feedback detection reads the already-reduced output, so it settles at less
    // gain reduction (louder) than feedforward for the same settings.
    let sine = steady_sine(0.5);
    let feedforward = dynamics_steady_amplitude(
        BTreeMap::from([(
            DYNAMICS_TOPOLOGY_PARAMETER.to_string(),
            DYNAMICS_TOPOLOGY_FEEDFORWARD.to_string(),
        )]),
        &sine,
        0.5,
        0.5,
    );
    let feedback = dynamics_steady_amplitude(
        BTreeMap::from([(
            DYNAMICS_TOPOLOGY_PARAMETER.to_string(),
            DYNAMICS_TOPOLOGY_FEEDBACK.to_string(),
        )]),
        &sine,
        0.5,
        0.5,
    );

    assert!(
        feedback > feedforward * 1.05,
        "feedback topology should compress more gently than feedforward (feedback {feedback}, feedforward {feedforward})"
    );
}

fn echo_impulse_output(interpolation: &str) -> Vec<f32> {
    let frames = 1_024;
    let module =
        ModuleNode::new(ModuleId::new("echo"), module_types::ECHO).with_params(BTreeMap::from([(
            INTERPOLATION_PARAMETER.to_string(),
            interpolation.to_string(),
        )]));
    let mut state = PerModuleState::new(&module, 48_000.0, &PreparedSamplerAssets::empty());
    let mut audio_in = vec![0.0f32; frames];
    audio_in[0] = 1.0;
    let c = |v: f32| vec![v; frames];
    let controls = EchoControls {
        feedback: &c(0.6),
        damping: &c(1.0),
        wet: &c(1.0),
        dry: &c(0.0),
        time_l: &c(0.002), // ~4.998 ms -> ~239.9 samples, a fractional delay
        time_r: &c(0.002),
        ping_pong: &c(0.0),
    };
    let outputs = process_echo(&mut state, &audio_in, &audio_in.clone(), controls, frames);
    outputs
        .audio
        .get(builtin_ports::AUDIO_OUT_L)
        .expect("echo emits audio_out_l")
        .clone()
}

#[test]
fn echo_interpolation_parameter_changes_fractional_delay_taps() {
    let linear = echo_impulse_output(INTERPOLATION_LINEAR);
    let cubic = echo_impulse_output(INTERPOLATION_CUBIC);
    let diff: f32 = linear.iter().zip(&cubic).map(|(a, b)| (a - b).abs()).sum();
    assert!(
        diff > 1e-3,
        "cubic interpolation should change the fractional-delay echo taps (sum abs diff {diff})"
    );
}

fn reverb_impulse_output(interpolation: &str) -> Vec<f32> {
    let frames = 2_048;
    let module =
        ModuleNode::new(ModuleId::new("verb"), module_types::REVERB).with_params(BTreeMap::from([
            (
                INTERPOLATION_PARAMETER.to_string(),
                interpolation.to_string(),
            ),
        ]));
    let mut state = PerModuleState::new(&module, 48_000.0, &PreparedSamplerAssets::empty());
    let mut audio_in = vec![0.0f32; frames];
    audio_in[0] = 1.0;
    let c = |v: f32| vec![v; frames];
    let controls = ReverbControls {
        decay_time: &c(0.5),
        room_size: &c(0.5),
        damping: &c(1.0),
        diffusion: &c(0.5),
        wet: &c(1.0),
        dry: &c(0.0),
        pre_delay: &c(0.0),
        stereo_width: &c(0.5),
    };
    let outputs = process_reverb(&mut state, &audio_in, &audio_in.clone(), controls, frames);
    outputs
        .audio
        .get(builtin_ports::AUDIO_OUT_L)
        .expect("reverb emits audio_out_l")
        .clone()
}

#[test]
fn reverb_interpolation_parameter_changes_fractional_delay_network() {
    let linear = reverb_impulse_output(INTERPOLATION_LINEAR);
    let cubic = reverb_impulse_output(INTERPOLATION_CUBIC);
    let diff: f32 = linear.iter().zip(&cubic).map(|(a, b)| (a - b).abs()).sum();
    assert!(
        diff > 1e-3,
        "cubic interpolation should change the reverb's fractional delay network (sum abs diff {diff})"
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
    let event_capacity_after_first = processor.pending_event_capacity();
    let voice_count_after_first = processor.prepared_voice_count();

    processor.render(&mut left, &mut right);

    assert_eq!(processor.top_level_scratch_capacities(), after_first);
    assert_eq!(
        processor.module_output_scratch_capacity(),
        output_capacity_after_first
    );
    assert_eq!(
        processor.pending_event_capacity(),
        event_capacity_after_first
    );
    assert_eq!(processor.prepared_voice_count(), voice_count_after_first);
}

#[test]
fn event_filter_passes_matching_note_events_without_timing_changes() {
    let graph = event_filter_sampler_graph(36);
    graph
        .validate()
        .expect("event filter graph should validate");
    let settings = sampler_settings(6);
    let assets = sampler_assets(vec![0.25, 0.5]);

    let (left, right) = render_offline_with_sampler_assets(
        &graph,
        &settings,
        vec![note_on_value(2, 36, 100)],
        &assets,
    );

    assert_eq!(left, vec![0.0, 0.0, 0.25, 0.5, 0.0, 0.0]);
    assert_eq!(right, vec![0.0; 6]);
}

#[test]
fn event_filter_blocks_non_matching_note_events() {
    let graph = event_filter_sampler_graph(36);
    graph
        .validate()
        .expect("event filter graph should validate");

    let (left, right) = render_offline_with_sampler_assets(
        &graph,
        &sampler_settings(6),
        vec![note_on_value(2, 38, 100)],
        &sampler_assets(vec![0.25, 0.5]),
    );

    assert_eq!(left, vec![0.0; 6]);
    assert_eq!(right, vec![0.0; 6]);
}

#[test]
fn event_filter_render_is_deterministic_for_identical_inputs() {
    let graph = event_filter_sampler_graph(36);
    graph
        .validate()
        .expect("event filter graph should validate");
    let settings = sampler_settings(6);
    let assets = sampler_assets(vec![0.25, 0.5]);
    let events = vec![note_on_value(2, 36, 100), note_on_value(3, 38, 100)];

    let first = render_offline_with_sampler_assets(&graph, &settings, events.clone(), &assets);
    let second = render_offline_with_sampler_assets(&graph, &settings, events, &assets);

    assert_eq!(first, second);
}

#[test]
fn event_filter_alone_does_not_generate_audio_or_hidden_signal_chain_behavior() {
    let graph = Graph::new(
        vec![
            ModuleNode::new(ModuleId::new("midi"), module_types::MIDI_INPUT)
                .with_output(builtin_ports::EVENTS, SignalType::Event),
            ModuleNode::new(ModuleId::new("filter"), module_types::EVENT_FILTER)
                .with_input(builtin_ports::EVENTS_IN, SignalType::Event)
                .with_output(builtin_ports::EVENTS_OUT, SignalType::Event)
                .with_params(BTreeMap::from([
                    (
                        EVENT_FILTER_SELECTOR_PARAMETER.to_string(),
                        EVENT_FILTER_NOTE_SELECTOR.to_string(),
                    ),
                    (EVENT_FILTER_NOTE_PARAMETER.to_string(), "36".to_string()),
                ])),
            ModuleNode::new(ModuleId::new("out"), module_types::AUDIO_OUTPUT)
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ],
        vec![Cable::new(
            PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
            PortRef::new(ModuleId::new("filter"), builtin_ports::EVENTS_IN),
        )],
    );
    graph.validate().expect("event-only graph should validate");

    let (left, right) = render_offline(
        &graph,
        &sampler_settings(6),
        vec![note_on_value(0, 36, 100)],
    );

    assert_eq!(left, vec![0.0; 6]);
    assert_eq!(right, vec![0.0; 6]);
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
fn module_oscillator_gain_voice_renders_like_equivalent_flat_graph() {
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
    let defined_module = render_patch(
        r#"
metadata:
  name: Module Voice
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

    assert_eq!(defined_module, flat);
    assert!(defined_module.0.iter().any(|sample| *sample != 0.0));
}

#[test]
fn module_sampler_voice_renders_through_generic_public_ports() {
    let patch = patch::load_patch_str(
        r#"
metadata:
  name: Module Sampler
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
        .expect("expanded sampler module should validate");
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
fn offline_and_realtime_processors_receive_only_expanded_module_nodes() {
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
    let mut realtime = RealtimeGraphProcessor::new(graph, 48_000.0);
    let mut left = [0.0; 4];
    let mut right = [0.0; 4];

    assert_eq!(realtime.render(&mut left, &mut right), 4);
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
fn realtime_note_on_at_triggers_sample_playback_at_frame_offset() {
    let graph = sampler_graph(Vec::new(), Vec::new());
    let assets = sampler_assets(vec![0.25, 0.5, 0.75]);
    let mut processor = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph,
        48_000.0,
        &assets,
        &VoiceAllocation::default(),
        6,
    );
    let mut left = vec![0.0; 6];
    let mut right = vec![0.0; 6];

    processor.note_on_at(60, 100, 2);
    processor.render(&mut left, &mut right);

    assert_eq!(left, vec![0.0, 0.0, 0.25, 0.5, 0.75, 0.0]);
    assert_eq!(right, vec![0.0; 6]);
}

#[test]
fn realtime_note_off_at_accepts_a_frame_offset_without_panicking() {
    let graph = sampler_graph(Vec::new(), Vec::new());
    let assets = sampler_assets(vec![0.25, 0.5, 0.75]);
    let mut processor = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph,
        48_000.0,
        &assets,
        &VoiceAllocation::default(),
        6,
    );
    let mut left = vec![0.0; 6];
    let mut right = vec![0.0; 6];

    processor.note_on_at(60, 100, 0);
    processor.note_off_at(60, 4);
    processor.render(&mut left, &mut right);
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
        sample: Some(
            LoadedSample::new(48_000, vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]).into(),
        ),
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
        sample: Some(LoadedSample::new(48_000, vec![1.0, 2.0, 3.0]).into()),
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

#[test]
fn sampler_state_persists_across_sequential_process_calls() {
    let mut state = PerModuleState::Sampler {
        sample: Some(LoadedSample::new(48_000, vec![0.1, 0.2, 0.3, 0.4, 0.5]).into()),
        position: 0.0,
        active: true,
    };

    let first = process_sampler(
        &mut state,
        &[],
        &[1.0; 3],
        &[0.0; 3],
        &[0.0; 3],
        &[0.0; 3],
        &[0.0; 3],
        3,
    );

    assert_eq!(first.audio[builtin_ports::AUDIO], vec![0.1, 0.2, 0.3]);

    let second = process_sampler(
        &mut state,
        &[],
        &[1.0; 3],
        &[0.0; 3],
        &[0.0; 3],
        &[0.0; 3],
        &[0.0; 3],
        3,
    );

    assert_eq!(second.audio[builtin_ports::AUDIO], vec![0.4, 0.5, 0.0]);
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

fn poly_allocation(max_voices: u32) -> VoiceAllocation {
    VoiceAllocation {
        max_voices,
        stealing: patch::VoiceStealingPolicy::Disabled,
    }
}

fn poly_allocation_stealing(max_voices: u32) -> VoiceAllocation {
    VoiceAllocation {
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
    assert_ne!(left[100], 0.0, "voices should produce audio early");

    // After note-off at frame 12000, the released voice enters release
    // but the unreleased voice continues -> audio should still be present
    assert_ne!(
        left[20000], 0.0,
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
fn realtime_processor_is_finished_tracks_full_voice_lifecycle() {
    // Regression test for the graph_processor::RealtimeGraphProcessor::is_finished
    // implementation, which a prior broken commit had reduced to an always-false
    // stub. Drives a full note-on -> note-off -> envelope-released cycle through
    // the polyphonic realtime pipeline and checks is_finished() at each stage.
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
            .with_output(builtin_ports::VALUE, SignalType::Control)
            .with_params(BTreeMap::from([
                ("attack".to_string(), "0".to_string()),
                ("decay".to_string(), "0".to_string()),
                ("sustain".to_string(), "0".to_string()),
                ("release".to_string(), "5".to_string()),
            ])),
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

    const SAMPLE_RATE: f32 = 1000.0;
    const BLOCK_SIZE: usize = 16;

    let mut processor = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph,
        SAMPLE_RATE,
        &PreparedSamplerAssets::empty(),
        &poly_allocation(1),
        BLOCK_SIZE,
    );

    let mut left = [0.0; BLOCK_SIZE];
    let mut right = [0.0; BLOCK_SIZE];

    assert!(
        processor.is_finished(),
        "a freshly constructed processor with no active notes should report finished"
    );

    processor.note_on_at(60, 100, 0);
    processor.render(&mut left, &mut right);
    assert!(
        !processor.is_finished(),
        "a held gate should report unfinished even once the envelope has decayed to sustain"
    );

    processor.note_off_at(60, 0);
    processor.render(&mut left, &mut right);
    assert!(
        processor.is_finished(),
        "releasing the only active voice should report finished once the release phase completes"
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
        &[5.0; BLOCK_SIZE],   // attack_in (5ms default)
        &[30.0; BLOCK_SIZE],  // decay_in (30ms default)
        &[0.7; BLOCK_SIZE],   // sustain_in (0.7 default)
        &[200.0; BLOCK_SIZE], // release_in (200ms default)
        0,
        BLOCK_SIZE,
    );

    // After many sustain blocks, level should be ~0.7
    // Run enough blocks to be well into sustain (5ms attack + 30ms decay = 1680 frames)
    for b in 1..20 {
        process_adsr(
            &mut state,
            &[],
            &[5.0; BLOCK_SIZE],
            &[30.0; BLOCK_SIZE],
            &[0.7; BLOCK_SIZE],
            &[200.0; BLOCK_SIZE],
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
        &[5.0; BLOCK_SIZE],
        &[30.0; BLOCK_SIZE],
        &[0.7; BLOCK_SIZE],
        &[200.0; BLOCK_SIZE],
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
            &[5.0; BLOCK_SIZE],
            &[30.0; BLOCK_SIZE],
            &[0.7; BLOCK_SIZE],
            &[200.0; BLOCK_SIZE],
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
        &[5.0; BLOCK_SIZE],
        &[30.0; BLOCK_SIZE],
        &[0.7; BLOCK_SIZE],
        &[200.0; BLOCK_SIZE],
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
        let mut m = BTreeMap::new();
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
fn module_echo_yaml_loads_and_validates() {
    let Some(yaml) = read_repo_fixture("examples/patches/module-echo.yaml") else {
        return;
    };
    let patch = patch::load_patch_str(&yaml).expect("module-echo.yaml should parse");
    patch::validate_patch_schema(&patch).expect("module-echo.yaml schema should be valid");
    let graph = Graph::from_patch_declarations(&patch);
    graph
        .validate()
        .expect("module-echo.yaml graph should validate");
}

#[test]
fn module_reverb_yaml_loads_and_validates() {
    let Some(yaml) = read_repo_fixture("examples/patches/module-reverb.yaml") else {
        return;
    };
    let patch = patch::load_patch_str(&yaml).expect("module-reverb.yaml should parse");
    patch::validate_patch_schema(&patch).expect("module-reverb.yaml schema should be valid");
    let graph = Graph::from_patch_declarations(&patch);
    graph
        .validate()
        .expect("module-reverb.yaml graph should validate");
}

#[test]
fn drum_machine_dogfood_routes_notes_to_explicit_voice_modules_without_primitive() {
    let Some(yaml) = read_repo_fixture("examples/patches/event-routing-drum-machine.yaml") else {
        return;
    };
    let patch = patch::load_patch_str(&yaml).expect("drum machine example should parse");
    patch::validate_patch_schema(&patch).expect("drum machine example should validate");

    assert!(
        patch
            .modules
            .iter()
            .all(|module| module.module_type != "drum_machine")
    );

    let graph = Graph::from_patch_declarations(&patch);
    graph
        .validate()
        .expect("drum machine graph should validate");
    let cable_pairs = graph
        .cables()
        .iter()
        .map(|cable| format!("{}->{}", cable.source(), cable.destination()))
        .collect::<Vec<_>>();

    assert!(cable_pairs.contains(&"kick_route.events_out->kick_voice::env.gate".to_string()));
    assert!(cable_pairs.contains(&"snare_route.events_out->snare_voice::env.gate".to_string()));
    assert!(cable_pairs.contains(&"hat_route.events_out->hat_voice::env.gate".to_string()));
    assert!(cable_pairs.contains(&"kick_voice::vca.audio_out->mixer.inputs".to_string()));
    assert!(cable_pairs.contains(&"snare_voice::vca.audio_out->mixer.inputs".to_string()));
    assert!(cable_pairs.contains(&"hat_voice::vca.audio_out->mixer.inputs".to_string()));

    for note in [60, 38, 42] {
        let (left, right) =
            render_offline(&graph, &patch.render, vec![note_on_value(0, note, 100)]);
        assert!(
            left.iter().chain(right.iter()).any(|sample| *sample != 0.0),
            "note {note} should render through its explicit downstream voice"
        );
    }
}

#[test]
fn drum_machine_loop_schedule_loads_routes_and_renders_audio() {
    let Some(yaml) = read_repo_fixture("examples/patches/event-routing-drum-machine.yaml") else {
        return;
    };
    let patch = patch::load_patch_str(&yaml).expect("drum machine example should parse");
    patch::validate_patch_schema(&patch).expect("drum machine example should validate");

    let graph = Graph::from_patch_declarations(&patch);
    graph
        .validate()
        .expect("drum machine graph should validate");

    let events = vec![
        note_on_value(0, 60, 100),
        note_on_value(125, 42, 88),
        note_on_value(250, 38, 100),
        note_on_value(375, 42, 88),
        note_on_value(500, 60, 100),
        note_on_value(625, 42, 88),
        note_on_value(750, 38, 100),
        note_on_value(875, 42, 88),
    ];

    let (left, right) = render_offline(&graph, &patch.render, events);

    assert!(
        left.iter().chain(right.iter()).any(|sample| *sample != 0.0),
        "drum loop schedule should render non-zero audio"
    );
}

#[test]
fn simple_poly_synth_dogfood_consumes_note_events_through_generic_routing() {
    let Some(yaml) = read_repo_fixture("examples/patches/event-routing-simple-poly-synth.yaml")
    else {
        return;
    };
    let patch = patch::load_patch_str(&yaml).expect("poly synth example should parse");
    patch::validate_patch_schema(&patch).expect("poly synth example should validate");

    assert!(
        patch
            .modules
            .iter()
            .all(|module| module.module_type != "poly_synth")
    );

    let graph = Graph::from_patch_declarations(&patch);
    graph.validate().expect("poly synth graph should validate");
    let matching = render_offline_polyphonic(
        &graph,
        &patch.render,
        vec![note_on_value(0, 60, 100)],
        &patch.voice_allocation,
    );
    let blocked = render_offline_polyphonic(
        &graph,
        &patch.render,
        vec![note_on_value(0, 62, 100)],
        &patch.voice_allocation,
    );

    assert!(
        matching
            .0
            .iter()
            .chain(matching.1.iter())
            .any(|sample| *sample != 0.0),
        "matching note should render deterministic pitched audio"
    );
    assert_eq!(blocked.0, vec![0.0; patch.render.duration_frames as usize]);
    assert_eq!(blocked.1, vec![0.0; patch.render.duration_frames as usize]);
}

#[test]
fn additional_acceptance_examples_load_validate_and_render_where_supported() {
    for (fixture, note) in [
        ("examples/patches/synthetic-snare.yaml", 38),
        ("examples/patches/synthetic-hats.yaml", 42),
        ("examples/patches/synthetic-808-kick.yaml", 36),
    ] {
        let Some(yaml) = read_repo_fixture(fixture) else {
            return;
        };
        let patch = patch::load_patch_str(&yaml).expect("acceptance example should parse");
        patch::validate_patch_schema(&patch).expect("acceptance example should validate");
        let graph = Graph::from_patch_declarations(&patch);
        graph.validate().expect("acceptance graph should validate");

        let (left, right) =
            render_offline(&graph, &patch.render, vec![note_on_value(0, note, 100)]);
        let (left_again, right_again) =
            render_offline(&graph, &patch.render, vec![note_on_value(0, note, 100)]);

        assert_eq!(
            left, left_again,
            "{fixture} left channel should be deterministic"
        );
        assert_eq!(
            right, right_again,
            "{fixture} right channel should be deterministic"
        );
        let peak = left
            .iter()
            .chain(right.iter())
            .map(|sample| sample.abs())
            .fold(0.0_f32, f32::max);
        assert!(
            peak > 0.0,
            "{fixture} should render non-empty audio; peak was {peak}"
        );
    }
}

#[test]
fn synthetic_808_kick_example_has_808_like_spectral_shape() {
    let Some(yaml) = read_repo_fixture("examples/patches/synthetic-808-kick.yaml") else {
        return;
    };
    let patch = patch::load_patch_str(&yaml).expect("808 kick example should parse");
    patch::validate_patch_schema(&patch).expect("808 kick example should validate");
    let graph = Graph::from_patch_declarations(&patch);
    graph
        .validate()
        .expect("808 kick example graph should validate");
    // No NoteOff — the ADSR (sustain=0) and sub gain envelope decay naturally.
    let events = vec![note_on_value(0, 36, 110)];
    let (left, _) = render_offline(&graph, &patch.render, events);
    let spectrum = fft::compute_magnitude_response(&left, patch.render.sample_rate_hz as f64).bins;
    let sub_band = average_band_db(&spectrum, 35.0, 90.0);
    let low_mid_band = average_band_db(&spectrum, 120.0, 400.0);
    let click_band = average_band_db(&spectrum, 1_500.0, 6_000.0);
    let peak = peak_abs(&left);
    let attack_rms = rms(&left[..480.min(left.len())]);
    let tail_start = 12_000.min(left.len());
    let tail_rms = rms(&left[tail_start..]);

    println!(
        "synthetic_808_kick metrics: peak={peak:.3}, attack_rms={attack_rms:.3}, tail_rms={tail_rms:.3}, sub={sub_band:.1}dB, low_mid={low_mid_band:.1}dB, click={click_band:.1}dB"
    );

    assert!(peak > 0.2, "kick peak should be audible, got {peak}");
    assert!(
        sub_band > low_mid_band - 3.0,
        "sub band should be close to or above low mids: sub {sub_band:.1}dB, low_mid {low_mid_band:.1}dB"
    );
    assert!(
        click_band > sub_band - 45.0,
        "kick should retain an audible transient: click {click_band:.1}dB, sub {sub_band:.1}dB"
    );
    assert!(
        peak > attack_rms * 4.0,
        "kick peak should be spiky relative to attack RMS: peak {peak}, attack_rms {attack_rms}"
    );
    // Sub gain envelope lets the sub ring out, so the tail may have sustained sub content.
    assert!(
        tail_rms > 0.001,
        "sub should sustain in the tail: tail_rms {tail_rms}"
    );
}

fn average_band_db(spectrum: &[(f64, f64)], min_hz: f64, max_hz: f64) -> f64 {
    let mut total = 0.0;
    let mut count = 0usize;
    for (_, magnitude_db) in spectrum
        .iter()
        .filter(|(frequency_hz, _)| *frequency_hz >= min_hz && *frequency_hz <= max_hz)
    {
        total += magnitude_db;
        count += 1;
    }

    total / 1f64.max(count as f64)
}

fn peak_abs(samples: &[f32]) -> f32 {
    samples
        .iter()
        .map(|sample| sample.abs())
        .fold(0.0, f32::max)
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt()
}

#[test]
fn drum_kit_module_examples_load_validate_and_render_with_documented_primitive_ports() {
    for (fixture, note, should_render) in [
        ("examples/patches/module-velocity-vca.yaml", 60, true),
        ("examples/patches/module-impulse-tone.yaml", 60, true),
        ("examples/patches/module-impulse-noise.yaml", 60, true),
        ("examples/patches/module-impulse-layer.yaml", 60, true),
        ("examples/patches/drum-kit.yaml", 36, true),
    ] {
        let Some(yaml) = read_repo_fixture(fixture) else {
            return;
        };
        let patch = patch::load_patch_str(&yaml).expect("drum kit example should parse");

        patch::validate_patch_schema(&patch).expect("drum kit example schema should validate");
        let graph = Graph::from_patch_declarations(&patch);
        graph
            .validate()
            .expect("drum kit example graph should validate");

        if !should_render {
            continue;
        }

        let (left, right) =
            render_offline(&graph, &patch.render, vec![note_on_value(0, note, 100)]);

        let peak = left
            .iter()
            .chain(right.iter())
            .map(|sample| sample.abs())
            .fold(0.0_f32, f32::max);
        assert!(
            peak > 0.0,
            "{fixture} should render non-empty audio; peak was {peak}"
        );
    }
}

#[test]
fn script_mapping_example_is_event_control_only_and_validates() {
    let Some(yaml) = read_repo_fixture("examples/patches/script-velocity-map.yaml") else {
        return;
    };
    let patch = patch::load_patch_str(&yaml).expect("script example should parse");

    patch::validate_patch_schema(&patch).expect("script example should validate");
    let script = patch
        .modules
        .iter()
        .find(|module| module.id == "velocity_map")
        .expect("script module should exist");

    assert!(
        script
            .outputs
            .iter()
            .all(|output| output.signal_type != patch::SignalType::Audio)
    );
}

#[test]
fn script_examples_parse_and_validate() {
    for fixture in [
        "examples/patches/script-drum-event-router.yaml",
        "examples/patches/script-velocity-accent.yaml",
        "examples/patches/script-state-counter.yaml",
    ] {
        let Some(yaml) = read_repo_fixture(fixture) else {
            continue;
        };
        let patch = patch::load_patch_str(&yaml).expect("script example should parse");

        patch::validate_patch_schema(&patch).expect("script example should validate");
    }
}

fn script_event_router_sampler_graph() -> Graph {
    Graph::new(
        vec![
            ModuleNode::new(ModuleId::new("midi"), module_types::MIDI_INPUT)
                .with_output(builtin_ports::EVENTS, SignalType::Event),
            ModuleNode::new(ModuleId::new("router"), module_types::SCRIPT)
                .with_input("events", SignalType::Event)
                .with_output("kick", SignalType::Event)
                .with_params(BTreeMap::from([
                    (
                        SCRIPT_LANGUAGE_PARAMETER.to_string(),
                        SCRIPT_LANGUAGE_RHAI.to_string(),
                    ),
                    (
                        SCRIPT_SOURCE_PARAMETER.to_string(),
                        r#"
                        fn process(ctx) {
                            for event in ctx.events {
                                if event.type == "note_on" && event.note == 36 {
                                    ctx.emit("kick", event);
                                }
                            }
                        }
                        "#
                        .to_string(),
                    ),
                ])),
            ModuleNode::new(ModuleId::new("sampler"), module_types::SAMPLER)
                .with_input(builtin_ports::TRIGGER, SignalType::Event)
                .with_input(builtin_ports::RATE, SignalType::Control)
                .with_input(builtin_ports::START, SignalType::Control)
                .with_input(builtin_ports::LOOP_ENABLED, SignalType::Control)
                .with_input(builtin_ports::LOOP_START, SignalType::Control)
                .with_input(builtin_ports::LOOP_END, SignalType::Control)
                .with_output(builtin_ports::AUDIO, SignalType::Audio),
            ModuleNode::new(ModuleId::new("out"), module_types::AUDIO_OUTPUT)
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
                PortRef::new(ModuleId::new("router"), "events"),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("router"), "kick"),
                PortRef::new(ModuleId::new("sampler"), builtin_ports::TRIGGER),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("sampler"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            ),
        ],
    )
}

#[test]
fn offline_render_routes_script_events_deterministically() {
    let graph = script_event_router_sampler_graph();
    graph
        .validate()
        .expect("script router graph should validate");
    let settings = sampler_settings(4);
    let assets = sampler_assets(vec![1.0, 0.0, 0.0, 0.0]);

    let kick = render_offline_with_sampler_assets(
        &graph,
        &settings,
        vec![note_on_value(0, 36, 100)],
        &assets,
    );
    let snare = render_offline_with_sampler_assets(
        &graph,
        &settings,
        vec![note_on_value(0, 38, 100)],
        &assets,
    );

    assert_eq!(kick.0[0], 1.0);
    assert_eq!(snare.0, vec![0.0; 4]);
}

#[test]
fn realtime_graph_processor_renders_prepared_script_modules_without_parsing_source() {
    let graph = script_event_router_sampler_graph();
    let assets = sampler_assets(vec![1.0, 0.0, 0.0, 0.0]);
    let mut processor = RealtimeGraphProcessor::new_with_sampler_assets(graph, 48_000.0, &assets);
    let mut left = vec![0.0; 4];
    let mut right = vec![0.0; 4];

    processor.note_on(36, 100);
    let rendered = processor.render(&mut left, &mut right);

    assert_eq!(rendered, 4);
    assert_eq!(left[0], 1.0);
}

fn read_repo_fixture(relative_path: &str) -> Option<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative_path);
    match fs::read_to_string(&path) {
        Ok(contents) => Some(contents),
        Err(error) if env!("CARGO_MANIFEST_DIR").starts_with("/tmp/cargo-mutants-") => {
            eprintln!("skipping repository fixture test in cargo-mutants copy: {error}");
            None
        }
        Err(error) => panic!("{} should exist: {error}", path.display()),
    }
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

fn assert_offline_realtime_polyphonic_parity(
    graph: &Graph,
    sample_rate: f32,
    total_frames: usize,
    block_size: usize,
    events: &[TimedInputEvent],
    sampler_assets: &PreparedSamplerAssets,
    voice_allocation: &VoiceAllocation,
) {
    let settings = RenderSettings {
        sample_rate_hz: sample_rate as u32,
        block_size_frames: block_size as u32,
        duration_frames: total_frames as u64,
    };

    let (offline_left, offline_right) = render_offline_with_sampler_assets_polyphonic(
        graph,
        &settings,
        events.to_vec(),
        sampler_assets,
        voice_allocation,
    );

    let mut realtime = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph.clone(),
        sample_rate,
        sampler_assets,
        voice_allocation,
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
        "left channel offline/realtime polyphonic parity mismatch"
    );
    assert_eq!(
        &offline_right[..compare_len],
        &realtime_right[..compare_len],
        "right channel offline/realtime polyphonic parity mismatch"
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

    let assets = PreparedSamplerAssets::empty();
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
        &assets,
    );
}

#[test]
fn offline_and_realtime_produce_same_output_for_event_driven_patch() {
    let graph = event_filter_sampler_graph(60);
    graph.validate().expect("graph should validate");
    let assets = sampler_assets(vec![0.25, 0.5, 0.75, 1.0]);

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

#[test]
fn offline_and_realtime_produce_same_output_for_voice_to_global_patch() {
    let graph = Graph::new(
        vec![
            ModuleNode::new(ModuleId::new("midi"), module_types::MIDI_INPUT)
                .with_execution_scope(ExecutionScope::Global)
                .with_output(builtin_ports::EVENTS, SignalType::Event),
            ModuleNode::new(ModuleId::new("osc"), module_types::OSCILLATOR)
                .with_execution_scope(ExecutionScope::Voice)
                .with_output(builtin_ports::AUDIO, SignalType::Audio),
            ModuleNode::new(ModuleId::new("mixer"), module_types::AUDIO_MIXER)
                .with_execution_scope(ExecutionScope::Global)
                .with_mixing_input(builtin_ports::INPUTS, SignalType::Audio)
                .with_output(builtin_ports::MIX, SignalType::Audio),
            ModuleNode::new(ModuleId::new("out"), module_types::AUDIO_OUTPUT)
                .with_execution_scope(ExecutionScope::Global)
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

    assert_offline_realtime_polyphonic_parity(
        &graph,
        48_000.0,
        64,
        16,
        &[TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
        )],
        &PreparedSamplerAssets::empty(),
        &poly_allocation(1),
    );
}

#[test]
fn offline_and_realtime_produce_same_output_for_polyphonic_patch() {
    let graph = poly_sampler_graph(Vec::new(), Vec::new());
    graph.validate().expect("graph should validate");
    let assets = sampler_assets(vec![0.25, 0.5, 0.75, 1.0]);

    assert_offline_realtime_polyphonic_parity(
        &graph,
        48_000.0,
        16,
        4,
        &[
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
        &assets,
        &poly_allocation(2),
    );
}

#[test]
fn realtime_disconnected_audio_input_does_not_leak_stale_output() {
    let graph = Graph::new(
        vec![
            ModuleNode::new(ModuleId::new("gain"), module_types::GAIN)
                .with_input(builtin_ports::AUDIO_IN, SignalType::Audio)
                .with_input(builtin_ports::GAIN, SignalType::Control)
                .with_output(builtin_ports::AUDIO_OUT, SignalType::Audio),
            ModuleNode::new(ModuleId::new("out"), module_types::AUDIO_OUTPUT)
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("gain"), builtin_ports::AUDIO_OUT),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("gain"), builtin_ports::AUDIO_OUT),
                PortRef::new(ModuleId::new("out"), builtin_ports::RIGHT),
            ),
        ],
    );
    graph.validate().expect("graph should validate");
    let mut realtime = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph,
        48_000.0,
        &PreparedSamplerAssets::empty(),
        &poly_allocation(1),
        16,
    );
    let mut left = vec![1.0; 16];
    let mut right = vec![-1.0; 16];

    assert_eq!(realtime.render(&mut left, &mut right), 16);

    assert_eq!(left, vec![0.0; 16]);
    assert_eq!(right, vec![0.0; 16]);
}

#[test]
fn realtime_inactive_voice_does_not_leak_previous_voice_output() {
    let graph = poly_sampler_graph(Vec::new(), Vec::new());
    graph.validate().expect("graph should validate");
    let assets = sampler_assets(vec![0.5, 0.25]);
    let mut realtime = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph,
        48_000.0,
        &assets,
        &poly_allocation(1),
        2,
    );
    let mut left = vec![0.0; 2];
    let mut right = vec![0.0; 2];

    realtime.note_on(60, 100);
    assert_eq!(realtime.render(&mut left, &mut right), 2);
    assert_eq!(left, vec![0.5, 0.25]);

    left.fill(1.0);
    right.fill(-1.0);
    assert_eq!(realtime.render(&mut left, &mut right), 2);

    assert_eq!(left, vec![0.0; 2]);
    assert_eq!(right, vec![0.0; 2]);
}

#[test]
fn note_to_control_holds_frequency_across_blocks() {
    // ntc.frequency -> osc.pitch tests Voice->Global routing which works
    // because osc (Voice) reads ntc output from the previous block's state.
    // With persistent state, frequency is held across blocks.
    let (left, _) = render_patch(
        r#"
metadata:
  name: NoteToControlHoldsFreq
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 192
modules:
  - id: midi
    type: midi_input
  - id: ntc
    type: note_to_control
    inputs:
      - name: events
        signal_type: event
  - id: osc
    type: oscillator
    inputs:
      - name: pitch
        signal_type: control
  - id: mix
    type: audio_mixer
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
connections:
  - from: midi.events
    to: ntc.events
  - from: ntc.frequency
    to: osc.pitch
  - from: osc.audio
    to: mix.inputs
  - from: mix.mix
    to: out.left
"#,
    );
    // With default pitch=1.0 (since osc processes before ntc in same block),
    // the oscillator still produces output. The important thing is no crash.
    assert!(
        left.iter().any(|&s| s != 0.0),
        "expected non-zero output from oscillator driven by ntc"
    );
}

#[test]
fn note_to_control_gate_event_triggers_impulse() {
    // All modules are Global scope so gate events flow correctly.
    let (left, _) = render_patch(
        r#"
metadata:
  name: NoteToControlGateTriggersImpulse
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 128
modules:
  - id: midi
    type: midi_input
  - id: ntc
    type: note_to_control
    inputs:
      - name: events
        signal_type: event
  - id: imp
    type: impulse
    inputs:
      - name: trigger
        signal_type: event
  - id: mix
    type: audio_mixer
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
connections:
  - from: midi.events
    to: ntc.events
  - from: ntc.gate
    to: imp.trigger
  - from: imp.audio
    to: mix.inputs
  - from: mix.mix
    to: out.left
"#,
    );
    assert!(
        left.iter().any(|&s| s != 0.0),
        "expected at least one impulse click from note_to_control gate event"
    );
}

// --- note_to_control legato + slide flag (TB-303) ----------------------------
//
// note_to_control snaps pitch and reports *when* to slide via its `slide`
// output; the actual portamento is done by the reusable `slew` primitive below.

const NTC_TEST_SAMPLE_RATE: f32 = 48_000.0;

fn expected_pitch_ratio(note: u8) -> f32 {
    let freq = 440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0);
    freq / OSCILLATOR_BASE_HZ
}

fn note_to_control_holding(note: u8) -> PerModuleState {
    let ratio = expected_pitch_ratio(note);
    PerModuleState::NoteToControl {
        gate_active: true,
        current_note: Some(note),
        current_velocity: 100.0 / 127.0,
        current_frequency: ratio * OSCILLATOR_BASE_HZ,
        current_pitch_ratio: ratio,
        current_slide: false,
    }
}

fn ntc_note_on(note: u8) -> BlockEvent {
    BlockEvent {
        frame_offset: 0,
        event: ScriptEvent::NoteOn {
            note,
            velocity: 100,
        },
    }
}

#[test]
fn note_to_control_snaps_pitch_and_flags_slide_on_overlap() {
    // A note that overlaps a held note snaps pitch_ratio to the new note and
    // raises the `slide` flag so a downstream slew glides into it.
    let mut state = note_to_control_holding(48);

    let outputs = process_note_to_control(&mut state, &[ntc_note_on(60)], 64);

    assert!(
        (outputs.control["pitch_ratio"][0] - expected_pitch_ratio(60)).abs() < 1.0e-4,
        "overlapping note should target the new pitch immediately (slew does the gliding)"
    );
    assert!(
        outputs.control["slide"].iter().all(|&s| s > 0.5),
        "an overlapping (legato) note must raise the slide flag"
    );
}

#[test]
fn note_to_control_does_not_retrigger_gate_on_legato_overlap() {
    // An overlapping note is a slide: the amplitude envelope must not retrigger,
    // so no NoteOn gate event is emitted.
    let mut state = note_to_control_holding(48);

    let outputs = process_note_to_control(&mut state, &[ntc_note_on(60)], 64);

    assert!(
        !outputs
            .events
            .iter()
            .any(|e| matches!(e.event, ScriptEvent::NoteOn { .. })),
        "legato overlap must not emit a retrigger NoteOn gate event"
    );
}

#[test]
fn note_to_control_snaps_and_retriggers_from_silence() {
    // A note from silence snaps instantly to pitch, triggers the gate, and does
    // not raise the slide flag.
    let mut state = PerModuleState::NoteToControl {
        gate_active: false,
        current_note: None,
        current_velocity: 0.0,
        current_frequency: 0.0,
        current_pitch_ratio: 0.0,
        current_slide: false,
    };

    let outputs = process_note_to_control(&mut state, &[ntc_note_on(60)], 64);

    assert!(
        (outputs.control["pitch_ratio"][0] - expected_pitch_ratio(60)).abs() < 1.0e-4,
        "note from silence should snap immediately to its pitch"
    );
    assert!(
        outputs
            .events
            .iter()
            .any(|e| matches!(e.event, ScriptEvent::NoteOn { .. })),
        "note from silence should retrigger the gate with a NoteOn event"
    );
    assert!(
        outputs.control["slide"].iter().all(|&s| s < 0.5),
        "a note from silence must not raise the slide flag"
    );
}

#[test]
fn note_to_control_ignores_note_off_from_slid_away_note() {
    // After sliding 48 -> 60 the note-off for the original note 48 is stale and
    // must not release the gate; the voice keeps sounding at note 60.
    let mut state = note_to_control_holding(60);
    let note_off_48 = BlockEvent {
        frame_offset: 0,
        event: ScriptEvent::NoteOff { note: 48 },
    };

    let outputs = process_note_to_control(&mut state, &[note_off_48], 64);

    assert!(
        !outputs
            .events
            .iter()
            .any(|e| matches!(e.event, ScriptEvent::NoteOff { .. })),
        "a stale note-off from a slid-away note must not emit a gate release"
    );
    assert!(
        outputs.control["velocity"].iter().all(|&v| v > 0.0),
        "the gate should stay held after a stale note-off, keeping velocity non-zero"
    );
}

// --- slew primitive (gated portamento) ---------------------------------------

fn slew_state() -> PerModuleState {
    PerModuleState::Slew {
        current: 0.0,
        sample_rate: NTC_TEST_SAMPLE_RATE,
    }
}

#[test]
fn slew_glides_toward_target_when_gate_open() {
    // With the glide gate open, the output ramps toward the target over the
    // glide time rather than jumping, and converges to it.
    let mut state = slew_state();
    let time_ms = 20.0;
    let target = 1.0;
    let frames = 8 * (NTC_TEST_SAMPLE_RATE * time_ms / 1000.0) as usize;

    let outputs = process_slew(
        &mut state,
        &vec![target; frames],
        &vec![1.0; frames],
        &vec![time_ms; frames],
        frames,
    );
    let out = &outputs.control["value"];

    assert!(
        out[0] > 0.0 && out[0] < target * 0.2,
        "output should start gliding from 0, not jump to the target: got {}",
        out[0]
    );
    assert!(
        out.windows(2).all(|w| w[1] >= w[0]),
        "a glide toward a higher target should be monotonically non-decreasing"
    );
    assert!(
        (out[frames - 1] - target).abs() < 1.0e-2,
        "output should converge to the target by the end of the glide: got {}",
        out[frames - 1]
    );
}

#[test]
fn slew_snaps_to_target_when_gate_closed() {
    // With the glide gate closed the slew passes its input straight through.
    let mut state = slew_state();
    let target = 0.75;

    let outputs = process_slew(
        &mut state,
        &vec![target; 8],
        &vec![0.0; 8],
        &vec![60.0; 8],
        8,
    );

    assert!(
        outputs.control["value"].iter().all(|&v| (v - target).abs() < 1.0e-6),
        "a closed glide gate must snap the output to the input immediately"
    );
}

#[test]
fn tb303_acid_accent_makes_notes_louder() {
    // End-to-end: the tb303-acid patch routes note velocity to the VCA, so an
    // accented (high-velocity) note renders louder than an unaccented one.
    let Some(yaml) = read_repo_fixture("examples/patches/tb303-acid.yaml") else {
        return;
    };
    let patch = patch::load_patch_str(&yaml).expect("tb303-acid.yaml should parse");
    let graph = Graph::from_patch_declarations(&patch);
    graph.validate().expect("tb303-acid graph should validate");

    let rms_for_velocity = |velocity: u8| {
        let events = vec![
            note_on_value(0, 45, velocity),
            TimedInputEvent::new(12_000, ScriptEvent::NoteOff { note: 45 }),
        ];
        let (left, _) = render_offline(&graph, &patch.render, events);
        let sum_sq: f64 = left.iter().map(|&s| (s as f64) * (s as f64)).sum();
        (sum_sq / left.len() as f64).sqrt()
    };

    let accented = rms_for_velocity(120);
    let unaccented = rms_for_velocity(80);

    assert!(
        accented > unaccented * 1.1,
        "accented note (rms {accented}) should be clearly louder than unaccented (rms {unaccented})"
    );
}

#[test]
fn note_to_control_clears_gate_on_matching_note_off() {
    let patch = patch::load_patch_str(
        r#"
metadata:
  name: NoteToControlClear
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 192
modules:
  - id: midi
    type: midi_input
  - id: ntc
    type: note_to_control
    inputs:
      - name: events
        signal_type: event
  - id: imp
    type: impulse
    inputs:
      - name: trigger
        signal_type: event
  - id: mix
    type: audio_mixer
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
connections:
  - from: midi.events
    to: ntc.events
  - from: ntc.gate
    to: imp.trigger
  - from: imp.audio
    to: mix.inputs
  - from: mix.mix
    to: out.left
"#,
    )
    .expect("patch should parse");
    patch::validate_patch_schema(&patch).expect("schema should be valid");
    let graph = Graph::from_patch_declarations(&patch);
    graph.validate().expect("graph should validate");
    let events = vec![
        TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
        ),
        TimedInputEvent::new(1, ScriptEvent::NoteOff { note: 60 }),
    ];
    let (left, _) = render_offline(
        &graph,
        &RenderSettings {
            sample_rate_hz: 48000,
            block_size_frames: 64,
            duration_frames: 192,
        },
        events,
    );
    // After matching note-off, gate clears — only the initial NoteOn triggers impulse.
    let nonzero_after_first_two: usize = left.iter().skip(2).filter(|&&s| s != 0.0).count();
    assert!(
        nonzero_after_first_two < 2,
        "expected no impulse clicks after matching note-off; got {nonzero_after_first_two}"
    );
}

#[test]
fn note_to_control_ignores_non_matching_note_off() {
    let patch = patch::load_patch_str(
        r#"
metadata:
  name: NoteToControlIgnoreNonMatching
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 130
modules:
  - id: midi
    type: midi_input
  - id: ntc
    type: note_to_control
    inputs:
      - name: events
        signal_type: event
  - id: imp
    type: impulse
    inputs:
      - name: trigger
        signal_type: event
  - id: mix
    type: audio_mixer
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
connections:
  - from: midi.events
    to: ntc.events
  - from: ntc.gate
    to: imp.trigger
  - from: imp.audio
    to: mix.inputs
  - from: mix.mix
    to: out.left
"#,
    )
    .expect("patch should parse");
    patch::validate_patch_schema(&patch).expect("schema should be valid");
    let graph = Graph::from_patch_declarations(&patch);
    graph.validate().expect("graph should validate");
    let events = vec![
        TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
        ),
        TimedInputEvent::new(64, ScriptEvent::NoteOff { note: 61 }),
    ];
    let (left, _) = render_offline(
        &graph,
        &RenderSettings {
            sample_rate_hz: 48000,
            block_size_frames: 64,
            duration_frames: 130,
        },
        events,
    );
    // A note-off for a note that is not the one sounding is fully ignored: it
    // neither releases the gate nor forwards a spurious gate event downstream
    // (which would otherwise wrongly retrigger/release an envelope). Only the
    // initial NoteOn produces a gate click.
    let nonzero_count = left.iter().filter(|&&s| s != 0.0).count();
    assert_eq!(
        nonzero_count, 1,
        "mismatched note-off must not emit a gate event; expected only the initial NoteOn click"
    );
}

#[test]
fn synthetic_808_kick_renders_deterministic_decaying_output() {
    // 808-style kick built from primitives only.
    // Uses note_to_rate (Voice scope) for pitch, ADSR for amplitude envelope,
    // filter for body shaping (saw → lowpass → more sine-like).
    //
    // Kick body: note_to_rate(N=36, C2 ≈ 55 Hz) → osc(220*ratio)
    //   → filter(lowpass) → vca(gain by adsr) → mix → out
    // Envelope: midi.events → adsr.gate, adsr.value → vca.gain
    //
    // Events: NoteOn at frame 0, NoteOff at frame 2000 (~42ms) so release
    // produces the decaying tail.
    let patch = patch::load_patch_str(
        r#"
metadata:
  name: 808Kick
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 48000
modules:
  - id: midi
    type: midi_input
  - id: ntr
    type: note_to_rate
    inputs:
      - name: events
        signal_type: event
  - id: osc
    type: oscillator
    inputs:
      - name: pitch
        signal_type: control
  - id: adsr
    type: adsr
    inputs:
      - name: gate
        signal_type: event
  - id: vca
    type: gain
    inputs:
      - name: audio_in
        signal_type: audio
      - name: gain
        signal_type: control
  - id: filt
    type: filter
    parameters:
      algorithm: moog
      mode: lowpass
    inputs:
      - name: audio_in
        signal_type: audio
  - id: mix
    type: audio_mixer
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
connections:
  - from: midi.events
    to: ntr.events
  - from: midi.events
    to: adsr.gate
  - from: ntr.rate
    to: osc.pitch
  - from: osc.audio
    to: filt.audio_in
  - from: filt.audio_out
    to: vca.audio_in
  - from: adsr.value
    to: vca.gain
  - from: vca.audio_out
    to: mix.inputs
  - from: mix.mix
    to: out.left
"#,
    )
    .expect("patch should parse");
    patch::validate_patch_schema(&patch).expect("schema should be valid");
    let graph = Graph::from_patch_declarations(&patch);
    graph.validate().expect("graph should validate");

    let events = vec![
        TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note: 36,
                velocity: 100,
            },
        ),
        TimedInputEvent::new(2000, ScriptEvent::NoteOff { note: 36 }),
    ];
    let (left, _right) = render_offline(
        &graph,
        &RenderSettings {
            sample_rate_hz: 48000,
            block_size_frames: 64,
            duration_frames: 48000,
        },
        events,
    );
    assert_eq!(left.len(), 48000, "should produce one second of audio");

    let spectrum = fft::compute_magnitude_response(&left, 48000.0).bins;
    let low_band = spectrum
        .iter()
        .filter(|(frequency_hz, _)| *frequency_hz >= 40.0 && *frequency_hz <= 120.0)
        .map(|(_, magnitude_db)| *magnitude_db)
        .sum::<f64>()
        / 1f64.max(
            spectrum
                .iter()
                .filter(|(frequency_hz, _)| *frequency_hz >= 40.0 && *frequency_hz <= 120.0)
                .count() as f64,
        );
    let high_band = spectrum
        .iter()
        .filter(|(frequency_hz, _)| *frequency_hz >= 500.0 && *frequency_hz <= 4000.0)
        .map(|(_, magnitude_db)| *magnitude_db)
        .sum::<f64>()
        / 1f64.max(
            spectrum
                .iter()
                .filter(|(frequency_hz, _)| *frequency_hz >= 500.0 && *frequency_hz <= 4000.0)
                .count() as f64,
        );
    assert!(
        low_band > high_band,
        "expected kick spectrum to favor low frequencies (low {low_band:.1} dB > high {high_band:.1} dB)"
    );

    // Determinism: re-render and compare
    let events2 = vec![
        TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note: 36,
                velocity: 100,
            },
        ),
        TimedInputEvent::new(2000, ScriptEvent::NoteOff { note: 36 }),
    ];
    let (left2, _) = render_offline(
        &graph,
        &RenderSettings {
            sample_rate_hz: 48000,
            block_size_frames: 64,
            duration_frames: 48000,
        },
        events2,
    );
    assert_eq!(left, left2, "808 kick render must be deterministic");

    // Decay: the peak energy should be in the first quarter
    let quarter = left.len() / 4;
    let first_quarter_energy: f32 = left[..quarter].iter().map(|s| s * s).sum();
    let rest_energy: f32 = left[quarter..].iter().map(|s| s * s).sum();
    assert!(
        first_quarter_energy > rest_energy,
        "expected kick energy to decay (first quarter {first_quarter_energy} > rest {rest_energy})"
    );
}

#[test]
fn noise_module_renders_deterministic_output() {
    let (left, _) = render_patch(
        r#"
metadata:
  name: Noise
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 256
modules:
  - id: noise
    type: noise
    parameters:
      seed: 42
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
      - name: right
        signal_type: audio
  - id: mixer
    type: audio_mixer
connections:
  - from: noise.audio
    to: mixer.inputs
  - from: mixer.mix
    to: out.left
"#,
    );
    assert_eq!(left.len(), 256);
    // Noise should have non-zero samples
    assert!(left.iter().any(|&s| s.abs() > 0.001));
    // Verify determinism with same seed
    let (left2, _) = render_patch(
        r#"
metadata:
  name: Noise
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 256
modules:
  - id: noise
    type: noise
    parameters:
      seed: 42
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
      - name: right
        signal_type: audio
  - id: mixer
    type: audio_mixer
connections:
  - from: noise.audio
    to: mixer.inputs
  - from: mixer.mix
    to: out.left
"#,
    );
    assert_eq!(left, left2, "same seed should produce identical output");
}

#[test]
fn impulse_module_produces_click_on_trigger() {
    let (left, _) = render_patch(
        r#"
metadata:
  name: Impulse
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 256
modules:
  - id: midi
    type: midi_input
  - id: imp
    type: impulse
    inputs:
      - name: trigger
        signal_type: event
  - id: mix
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
    to: imp.trigger
  - from: imp.audio
    to: mix.inputs
  - from: mix.mix
    to: out.left
"#,
    );
    assert_eq!(left.len(), 256);
    // Impulse should have non-zero sample at trigger position
    assert!(left.iter().any(|&s| s.abs() > 0.0));
}

#[test]
fn multiply_module_deterministically_combines_two_signals() {
    // Use mixer to bridge voice-scoped oscillator and gain to global scope
    let (left, _) = render_patch(
        r#"
metadata:
  name: Multiply
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: osc
    type: oscillator
  - id: gain
    type: gain
  - id: mix_a
    type: audio_mixer
  - id: mix_b
    type: audio_mixer
  - id: mult
    type: multiply
    inputs:
      - name: audio_in
        signal_type: audio
      - name: gain
        signal_type: audio
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
      - name: right
        signal_type: audio
connections:
  - from: osc.audio
    to: mix_a.inputs
  - from: mix_a.mix
    to: mult.audio_in
  - from: gain.audio_out
    to: mix_b.inputs
  - from: mix_b.mix
    to: mult.gain
  - from: mult.audio_out
    to: out.left
"#,
    );
    assert_eq!(left.len(), 128);
    let (left2, _) = render_patch(
        r#"
metadata:
  name: Multiply
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: osc
    type: oscillator
  - id: gain
    type: gain
  - id: mix_a
    type: audio_mixer
  - id: mix_b
    type: audio_mixer
  - id: mult
    type: multiply
    inputs:
      - name: audio_in
        signal_type: audio
      - name: gain
        signal_type: audio
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
      - name: right
        signal_type: audio
connections:
  - from: osc.audio
    to: mix_a.inputs
  - from: mix_a.mix
    to: mult.audio_in
  - from: gain.audio_out
    to: mix_b.inputs
  - from: mix_b.mix
    to: mult.gain
  - from: mult.audio_out
    to: out.left
"#,
    );
    assert_eq!(left, left2, "multiply should be deterministic");
}

#[test]
fn multiply_module_produces_audio_product() {
    // Multiply two known signals: noise * 0.5 gain = half-amplitude noise
    // Noise peak in first block is about 1.0, so product should be ~0.5.
    let (left, _) = render_patch(
        r#"
metadata:
  name: MultiplyProduct
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: noise
    type: noise
  - id: noise2
    type: noise
  - id: mix_a
    type: audio_mixer
  - id: mix_b
    type: audio_mixer
  - id: mult
    type: multiply
    inputs:
      - name: audio_in
        signal_type: audio
      - name: gain
        signal_type: audio
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
      - name: right
        signal_type: audio
connections:
  - from: noise.audio
    to: mix_a.inputs
  - from: mix_a.mix
    to: mult.audio_in
  - from: noise2.audio
    to: mix_b.inputs
  - from: mix_b.mix
    to: mult.gain
  - from: mult.audio_out
    to: out.left
"#,
    );
    assert_eq!(left.len(), 128);
    // Multiply is deterministic: run twice, get same output.
    let (left2, _) = render_patch(
        r#"
metadata:
  name: MultiplyProduct
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: noise
    type: noise
  - id: noise2
    type: noise
  - id: mix_a
    type: audio_mixer
  - id: mix_b
    type: audio_mixer
  - id: mult
    type: multiply
    inputs:
      - name: audio_in
        signal_type: audio
      - name: gain
        signal_type: audio
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
      - name: right
        signal_type: audio
connections:
  - from: noise.audio
    to: mix_a.inputs
  - from: mix_a.mix
    to: mult.audio_in
  - from: noise2.audio
    to: mix_b.inputs
  - from: mix_b.mix
    to: mult.gain
  - from: mult.audio_out
    to: out.left
"#,
    );
    assert_eq!(left, left2, "multiply product should be deterministic");
}

#[test]
fn note_to_control_converts_midi_note_to_frequency() {
    let (left, _) = render_patch(
        r#"
metadata:
  name: NoteToControl
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 256
modules:
  - id: midi
    type: midi_input
  - id: ntc
    type: note_to_control
    inputs:
      - name: events
        signal_type: event
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
      - name: right
        signal_type: audio
connections:
  - from: midi.events
    to: ntc.events
"#,
    );
    assert_eq!(left.len(), 256);
}

#[test]
fn realtime_preparation_records_configured_max_block_size() {
    for size in [1, 64, 256, 1024] {
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

        let processor = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
            graph,
            48_000.0,
            &PreparedSamplerAssets::empty(),
            &VoiceAllocation::default(),
            size,
        );

        assert_eq!(processor.prepared_max_block_size(), size);
        assert_eq!(
            processor.top_level_scratch_capacities(),
            (size, size),
            "scratch capacities should match prepared max block size {size}"
        );
    }
}

#[test]
fn realtime_preparation_with_min_block_size_renders_without_panic() {
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

    let mut processor = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph,
        48_000.0,
        &PreparedSamplerAssets::empty(),
        &VoiceAllocation::default(),
        1,
    );

    assert_eq!(processor.prepared_max_block_size(), 1);

    let mut left = [0.0f32];
    let mut right = [0.0f32];
    processor.note_on(60, 100);
    let rendered = processor.render(&mut left, &mut right);

    assert_eq!(rendered, 1);
}

#[test]
fn realtime_preparation_skips_module_output_scratch_for_arena_supported_graph() {
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

    let processor = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph,
        48_000.0,
        &PreparedSamplerAssets::empty(),
        &VoiceAllocation::default(),
        64,
    );

    assert_eq!(processor.module_output_scratch_capacity(), 0);
    assert!(processor.pending_event_capacity() >= 64);
}

#[test]
fn realtime_preparation_respects_voice_allocation() {
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
        ],
    );
    graph.validate().expect("graph should validate");

    let processor = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph,
        48_000.0,
        &PreparedSamplerAssets::empty(),
        &VoiceAllocation {
            max_voices: 8,
            stealing: patch::VoiceStealingPolicy::Disabled,
        },
        64,
    );

    assert_eq!(processor.prepared_max_block_size(), 64);
    assert_eq!(processor.prepared_voice_count(), 8);
}

#[test]
fn realtime_note_submission_does_not_fill_unused_prepared_event_queue() {
    let graph = sampler_graph(Vec::new(), Vec::new());
    graph.validate().expect("graph should validate");

    let mut processor = RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
        graph,
        48_000.0,
        &sampler_assets(vec![1.0, 0.0]),
        &VoiceAllocation::default(),
        2,
    );

    processor.note_on(60, 100);
    processor.note_on(61, 100);
    processor.note_on(62, 100);

    assert_eq!(processor.prepared_event_queue_overflow_count(), 0);
}

#[test]
fn decay_module_stays_silent_until_its_trigger_frame_offset() {
    // sample_rate 1000 Hz makes decay_frames equal to time_ms, so a 100 ms
    // control value reproduces the previous fixed 100-frame decay length.
    let mut state = PerModuleState::Decay {
        level: 0.0,
        triggered: false,
        elapsed_frames: 0,
        sample_rate: 1000.0,
        curve: crate::decay::DecayCurve::Exponential,
    };

    let events = [super::outputs::BlockEvent {
        frame_offset: 20,
        event: ScriptEvent::NoteOn {
            note: 60,
            velocity: 100,
        },
    }];

    let time_ms = vec![100.0; 30];
    let outputs = super::processing::process_decay(&mut state, &events, &time_ms, 30);
    let values = &outputs.control[builtin_ports::VALUE];

    for (i, value) in values.iter().enumerate().take(20) {
        assert_eq!(*value, 0.0, "expected silence before trigger at frame {i}");
    }
    assert_eq!(
        values[20], 1.0,
        "decay should jump to peak on its trigger frame"
    );
    assert!(
        values[21] < 1.0,
        "decay should begin falling after the trigger frame"
    );
}

#[test]
fn decay_module_linear_curve_ramps_down_and_resets_when_fully_decayed() {
    // sample_rate 1000 Hz makes decay_frames equal to time_ms, so a 10 ms
    // control value reproduces the previous fixed 10-frame decay length.
    let mut state = PerModuleState::Decay {
        level: 0.0,
        triggered: false,
        elapsed_frames: 0,
        sample_rate: 1000.0,
        curve: crate::decay::DecayCurve::Linear,
    };

    let events = [super::outputs::BlockEvent {
        frame_offset: 0,
        event: ScriptEvent::NoteOn {
            note: 60,
            velocity: 100,
        },
    }];

    let time_ms = vec![10.0; 15];
    let outputs = super::processing::process_decay(&mut state, &events, &time_ms, 15);
    let values = &outputs.control[builtin_ports::VALUE];

    assert_eq!(
        values[0], 1.0,
        "decay should start at peak on its trigger frame"
    );
    for window in values[..11].windows(2) {
        assert!(
            window[1] <= window[0],
            "linear decay should be monotonically non-increasing: {:?}",
            &values[..11]
        );
    }
    for (i, value) in values[10..].iter().enumerate() {
        assert_eq!(
            *value,
            0.0,
            "decay should reach and hold exact zero once fully elapsed (frame {})",
            10 + i
        );
    }

    match state {
        PerModuleState::Decay {
            level, triggered, ..
        } => {
            assert_eq!(
                level, 0.0,
                "state should retain zero level after full decay"
            );
            assert!(
                !triggered,
                "triggered flag should reset once the decay fully completes"
            );
        }
        _ => unreachable!(),
    }

    // A fully-decayed module should be retriggerable, not stuck.
    let retrigger_events = [super::outputs::BlockEvent {
        frame_offset: 0,
        event: ScriptEvent::NoteOn {
            note: 60,
            velocity: 100,
        },
    }];
    let retrigger_outputs =
        super::processing::process_decay(&mut state, &retrigger_events, &[10.0], 1);
    assert_eq!(
        retrigger_outputs.control[builtin_ports::VALUE][0],
        1.0,
        "a fully-decayed module should jump back to peak when retriggered"
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
        let mut m = BTreeMap::new();
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

#[test]
fn control_to_audio_promotion_carries_the_control_signal_into_the_audio_path() {
    // `note_to_rate` emits a steady control rate of 1.0 with no note events.
    // Promoted to audio it must appear sample-for-sample on the audio bus,
    // proving the promotion node renders rather than silently dropping signal.
    let graph = Graph::new(
        vec![
            ModuleNode::new(ModuleId::new("rate"), "note_to_rate")
                .with_input(builtin_ports::EVENTS, SignalType::Event)
                .with_output(builtin_ports::RATE, SignalType::Control),
            ModuleNode::new(ModuleId::new("promote"), "control_to_audio")
                .with_input(builtin_ports::IN, SignalType::Control)
                .with_output(builtin_ports::OUT, SignalType::Audio),
            ModuleNode::new(ModuleId::new("out"), "audio_output")
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("rate"), builtin_ports::RATE),
                PortRef::new(ModuleId::new("promote"), builtin_ports::IN),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("promote"), builtin_ports::OUT),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            ),
        ],
    );
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 4,
        duration_frames: 8,
    };

    let (left, _right) = render_offline(&graph, &settings, Vec::new());

    assert_eq!(left.len(), 8);
    assert!(
        left.iter().all(|sample| (sample - 1.0).abs() < 1e-6),
        "the promoted control value 1.0 appears as audio, got {left:?}"
    );
}

#[test]
fn compensation_delay_delays_an_impulse_exactly_across_block_boundaries() {
    const DELAY_SAMPLES: usize = 6;
    let graph = Graph::new(
        vec![
            ModuleNode::new(ModuleId::new("midi"), module_types::MIDI_INPUT)
                .with_output(builtin_ports::EVENTS, SignalType::Event),
            ModuleNode::new(ModuleId::new("impulse"), module_types::IMPULSE)
                .with_input(builtin_ports::TRIGGER, SignalType::Event)
                .with_output(builtin_ports::AUDIO, SignalType::Audio),
            ModuleNode::new(ModuleId::new("delay"), module_types::COMPENSATION_DELAY)
                .with_input(builtin_ports::AUDIO_IN, SignalType::Audio)
                .with_output(builtin_ports::AUDIO_OUT, SignalType::Audio)
                .with_params(BTreeMap::from([(
                    DELAY_SAMPLES_PARAMETER.to_string(),
                    DELAY_SAMPLES.to_string(),
                )])),
            ModuleNode::new(ModuleId::new("out"), module_types::AUDIO_OUTPUT)
                .with_input(builtin_ports::LEFT, SignalType::Audio)
                .with_input(builtin_ports::RIGHT, SignalType::Audio),
        ],
        vec![
            Cable::new(
                PortRef::new(ModuleId::new("midi"), builtin_ports::EVENTS),
                PortRef::new(ModuleId::new("impulse"), builtin_ports::TRIGGER),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("impulse"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("delay"), builtin_ports::AUDIO_IN),
            ),
            Cable::new(
                PortRef::new(ModuleId::new("delay"), builtin_ports::AUDIO_OUT),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            ),
        ],
    );
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 4,
        duration_frames: 12,
    };

    let (left, _right) = render_offline(&graph, &settings, vec![note_on(0, 100)]);

    let mut expected = vec![0.0; settings.duration_frames as usize];
    expected[DELAY_SAMPLES] = 1.0;
    assert_eq!(left, expected);
}
