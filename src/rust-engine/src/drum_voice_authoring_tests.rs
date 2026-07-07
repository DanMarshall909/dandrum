//! Acceptance tests for the `add-drum-voice-authoring` change.
//!
//! These prove that the seeded 808/909-style drum instruments load, expose
//! their declared public parameters, and render non-silent audio from a MIDI
//! trigger, and that the primitive gaps closed for drum authoring (oscillator
//! waveform selection and runtime-updatable decay) behave as specified.

use std::fs;
use std::path::PathBuf;

use crate::core::TimedInputEvent;
use crate::patch::{self, validate_patch_schema};
use crate::script::ScriptEvent;
use crate::synth::DandrumEngine;

const SAMPLE_RATE_HZ: f32 = 48_000.0;
const TRIGGER_NOTE: u8 = 40;
const TRIGGER_VELOCITY: u8 = 110;

fn drums_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("examples")
        .join("patches")
        .join("drums")
}

fn drum_patch_paths() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(drums_dir())
        .expect("drums example directory should exist")
        .map(|entry| entry.expect("directory entry should be readable").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "yaml"))
        .collect();
    paths.sort();
    assert!(
        !paths.is_empty(),
        "expected at least one authored drum instrument in {}",
        drums_dir().display()
    );
    paths
}

fn note_on_at_start() -> Vec<TimedInputEvent> {
    vec![TimedInputEvent::new(
        0,
        ScriptEvent::NoteOn {
            note: TRIGGER_NOTE,
            velocity: TRIGGER_VELOCITY,
        },
    )]
}

fn peak(samples: &[f32]) -> f32 {
    samples
        .iter()
        .fold(0.0_f32, |acc, sample| acc.max(sample.abs()))
}

#[test]
fn every_drum_voice_loads_validates_and_declares_public_parameters() {
    for path in drum_patch_paths() {
        let patch = patch::load_patch_file(&path)
            .unwrap_or_else(|error| panic!("{} should parse: {error}", path.display()));

        validate_patch_schema(&patch)
            .unwrap_or_else(|error| panic!("{} should validate: {error}", path.display()));

        assert!(
            patch.instrument.is_some(),
            "{} should declare an instrument identity so presets can target it",
            path.display()
        );
        assert!(
            !patch.preset_surface.parameters.is_empty(),
            "{} should expose at least one public parameter through preset_surface",
            path.display()
        );
    }
}

#[test]
fn every_drum_voice_renders_non_silent_audio_from_a_midi_trigger() {
    for path in drum_patch_paths() {
        let mut engine = DandrumEngine::new();
        engine.prepare(SAMPLE_RATE_HZ);
        let render = engine
            .render_patch_file_offline(&path, note_on_at_start())
            .unwrap_or_else(|error| panic!("{} should render: {error}", path.display()));

        assert!(
            peak(&render.left) > 0.0 && peak(&render.right) > 0.0,
            "{} should render non-silent stereo audio from a MIDI trigger",
            path.display()
        );
    }
}

#[test]
fn sampler_backed_909_metallic_voices_reference_placeholder_assets() {
    // Task 3.2/4.3: 909 hats/crash/ride are sampler-backed, and the repository
    // ships only self-authored placeholder samples, never proprietary content.
    for path in drum_patch_paths() {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let is_metallic = ["hat", "crash", "ride"]
            .iter()
            .any(|voice| name.contains(voice));
        if !is_metallic {
            continue;
        }

        let patch = patch::load_patch_file(&path).expect("metallic voice should parse");
        assert!(
            patch
                .assets
                .iter()
                .any(|asset| { asset.path.contains("assets/drums/909") }),
            "{name} should reference a self-authored placeholder sample under assets/drums/909",
        );
    }
}

fn render_tom_with_runtime_decay(decay_ms: f32) -> Vec<f32> {
    let path = drums_dir().join("drum-808-tom.yaml");
    let mut engine = DandrumEngine::new();
    engine.prepare(SAMPLE_RATE_HZ);
    engine
        .load_patch_file(&path)
        .expect("808 tom should load for runtime parameter test");

    // Change the mapped decay target at runtime. `amp_env.time_ms` is the live
    // control input the tom's `tom.decay_ms` public parameter maps onto.
    assert!(
        engine.set_numeric_parameter_by_target("amp_env", "time_ms", decay_ms),
        "amp_env.time_ms should be an addressable runtime target"
    );

    engine.note_on(TRIGGER_NOTE, TRIGGER_VELOCITY);
    let frames = (SAMPLE_RATE_HZ as usize) / 2;
    let mut left = vec![0.0_f32; frames];
    let mut right = vec![0.0_f32; frames];
    engine.render(&mut left, &mut right);
    left
}

#[test]
fn decay_public_parameter_updates_mapped_target_at_runtime_without_reloading_yaml() {
    // Task 1.3: a decay parameter exposed through preset_surface updates the
    // mapped runtime target without rewriting or reparsing YAML.
    let short = render_tom_with_runtime_decay(60.0);
    let long = render_tom_with_runtime_decay(900.0);

    let tail = |samples: &[f32]| -> f32 {
        let start = samples.len() * 3 / 4;
        samples[start..].iter().map(|sample| sample.abs()).sum()
    };

    assert!(
        tail(&long) > tail(&short) * 2.0,
        "a longer runtime decay should leave more energy in the tail \
         (short={}, long={})",
        tail(&short),
        tail(&long)
    );
}

#[test]
fn oscillator_waveform_selection_changes_rendered_output() {
    // Task 1.2: waveform selection is honoured by the render path, not just the
    // metadata surface. A sine and a saw body at the same pitch differ.
    fn render_waveform(waveform: &str) -> Vec<f32> {
        let yaml = format!(
            r#"
metadata:
  name: Waveform Probe
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 512
modules:
  - id: midi
    type: midi_input
  - id: osc
    type: oscillator
    parameters:
      waveform: {waveform}
      pitch: 1
  - id: mixer
    type: audio_mixer
  - id: out
    type: audio_output
connections:
  - from: osc.audio
    to: mixer.inputs
  - from: mixer.mix
    to: out.left
  - from: mixer.mix
    to: out.right
"#
        );
        let patch = patch::load_patch_str(&yaml).expect("waveform probe patch should parse");
        let graph = crate::graph::Graph::from_patch_declarations(&patch);
        graph
            .validate()
            .expect("waveform probe graph should validate");
        let (left, _) = crate::graph_processor::render_offline(&graph, &patch.render, Vec::new());
        left
    }

    let saw = render_waveform("saw");
    let sine = render_waveform("sine");

    assert_eq!(
        saw.len(),
        sine.len(),
        "both waveforms render the same length"
    );
    assert!(
        saw.iter().zip(&sine).any(|(a, b)| (a - b).abs() > 1e-3),
        "saw and sine oscillators at the same pitch should produce different samples"
    );
    // The default (saw) rises from -1, while sine starts near zero.
    assert_approx_eq!(sine[0], 0.0, 1e-3, "a sine oscillator starts near zero");
}
