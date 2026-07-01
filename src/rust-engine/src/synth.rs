use std::f32::consts::TAU;

use std::path::Path;

use crate::core::TimedInputEvent;
use crate::graph::Graph;
use crate::graph_processor::RealtimeGraphProcessor;
use crate::patch;
use crate::preparation::{self, PreparationError};
use crate::realtime::RealtimeEvent;
use crate::sample::PreparedSamplerAssets;

#[derive(Debug)]
pub enum LoadPatchError {
    UnsupportedFormat(String),
    Io(std::io::Error),
    Parse(String),
    Validation(String),
    GraphValidation(String),
    Compile(String),
    SamplePreparation(String),
}

pub struct OfflineRender {
    pub sample_rate_hz: u32,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
}

impl std::fmt::Display for LoadPatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadPatchError::UnsupportedFormat(e) => write!(f, "unsupported patch format: {e}"),
            LoadPatchError::Io(e) => write!(f, "I/O error: {e}"),
            LoadPatchError::Parse(e) => write!(f, "parse error: {e}"),
            LoadPatchError::Validation(e) => write!(f, "validation error: {e}"),
            LoadPatchError::GraphValidation(e) => write!(f, "graph validation error: {e}"),
            LoadPatchError::Compile(e) => write!(f, "compile error: {e}"),
            LoadPatchError::SamplePreparation(e) => write!(f, "sample preparation error: {e}"),
        }
    }
}

impl From<patch::PatchLoadError> for LoadPatchError {
    fn from(e: patch::PatchLoadError) -> Self {
        match e {
            patch::PatchLoadError::UnsupportedFormat { path } => {
                LoadPatchError::UnsupportedFormat(path.display().to_string())
            }
            patch::PatchLoadError::ReadFailed { message, .. } => {
                LoadPatchError::Io(std::io::Error::new(std::io::ErrorKind::Other, message))
            }
            patch::PatchLoadError::ParseFailed { message, .. } => LoadPatchError::Parse(message),
        }
    }
}

impl From<patch::PatchValidationError> for LoadPatchError {
    fn from(e: patch::PatchValidationError) -> Self {
        LoadPatchError::Validation(format!("{e}"))
    }
}

impl From<crate::sample::SampleLoadError> for LoadPatchError {
    fn from(e: crate::sample::SampleLoadError) -> Self {
        LoadPatchError::SamplePreparation(format!("{e}"))
    }
}

impl From<PreparationError> for LoadPatchError {
    fn from(error: PreparationError) -> Self {
        match error {
            PreparationError::Load(error) => error.into(),
            PreparationError::Schema(error) => error.into(),
            PreparationError::Graph(error) => Self::GraphValidation(format!("{error}")),
            PreparationError::Assets(error) => error.into(),
            PreparationError::Compile(error) => Self::Compile(format!("{error}")),
        }
    }
}

impl std::error::Error for LoadPatchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LoadPatchError::Io(e) => Some(e),
            _ => None,
        }
    }
}

pub struct DandrumEngine {
    sample_rate: f32,
    prepared_max_block_size: usize,
    fallback: FallbackSynth,
    graph_processor: Option<RealtimeGraphProcessor>,
}

const MAX_VOICES: usize = 16;

#[derive(Clone, Copy)]
struct Voice {
    active: bool,
    note: u8,
    velocity: f32,
    sample_index: usize,
    phases: [f32; 5],
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            active: false,
            note: 60,
            velocity: 0.0,
            sample_index: 0,
            phases: [0.0; 5],
        }
    }
}

pub(crate) struct FallbackSynth {
    voices: [Voice; MAX_VOICES],
    sample_rate: f32,
}

const SECONDS: f32 = 1.25;
const GAIN: f32 = 0.16;
const RATIOS: [f32; 5] = [0.5, 1.0, 1.259_921, 1.498_307, 2.0];
const PANS: [f32; 5] = [-0.65, -0.35, 0.0, 0.35, 0.65];
const DEFAULT_PREPARED_MAX_BLOCK_SIZE: usize = 512;

impl FallbackSynth {
    fn new(sample_rate: f32) -> Self {
        Self {
            voices: [Voice::default(); MAX_VOICES],
            sample_rate,
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
        self.voices = [Voice::default(); MAX_VOICES];
    }

    fn note_on(&mut self, note: u8, velocity: u8) {
        let voice_index = self
            .voices
            .iter()
            .position(|voice| !voice.active)
            .unwrap_or_else(|| oldest_voice_index(&self.voices));

        self.voices[voice_index] = Voice {
            active: true,
            note,
            velocity: (velocity as f32 / 127.0).clamp(0.0, 1.0),
            sample_index: 0,
            phases: [0.0; 5],
        };
    }

    fn note_off(&mut self, note: u8) {
        for voice in &mut self.voices {
            if voice.active && voice.note == note {
                voice.sample_index = voice.sample_index.max((self.sample_rate * 0.85) as usize);
            }
        }
    }

    fn render(&mut self, left: &mut [f32], right: &mut [f32]) -> usize {
        let num_samples = left.len().min(right.len());
        let total_samples = (self.sample_rate * SECONDS) as usize;

        for sample in 0..num_samples {
            let mut l = 0.0;
            let mut r = 0.0;

            for voice in &mut self.voices {
                if !voice.active {
                    continue;
                }

                if voice.sample_index >= total_samples {
                    voice.active = false;
                    continue;
                }

                let env = envelope(voice.sample_index as f32 / self.sample_rate, SECONDS);
                let vibrato =
                    (TAU * 5.1 * voice.sample_index as f32 / self.sample_rate).sin() * 0.004;
                let root_hz = midi_note_to_hz(voice.note);

                for partial in 0..RATIOS.len() {
                    let phase = voice.phases[partial];
                    let saw = (phase / TAU) * 2.0 - 1.0;
                    let sine = phase.sin();
                    let tone =
                        soft_clip(saw * 0.55 + sine * 0.45) * env * GAIN * voice.velocity;
                    let (left_gain, right_gain) = equal_power_pan(PANS[partial]);

                    l += tone * left_gain;
                    r += tone * right_gain;

                    let hz = root_hz * RATIOS[partial] * (1.0 + vibrato);
                    voice.phases[partial] = wrap_phase(phase + TAU * hz / self.sample_rate);
                }

                voice.sample_index += 1;
            }

            left[sample] += soft_clip(l);
            right[sample] += soft_clip(r);
        }

        num_samples
    }

    fn is_finished(&self) -> bool {
        self.voices.iter().all(|voice| !voice.active)
    }

    #[cfg(test)]
    fn voice_sample_index(&self, index: usize) -> usize {
        self.voices[index].sample_index
    }
}

impl DandrumEngine {
    pub fn new() -> Self {
        Self {
            sample_rate: 44_100.0,
            prepared_max_block_size: DEFAULT_PREPARED_MAX_BLOCK_SIZE,
            fallback: FallbackSynth::new(44_100.0),
            graph_processor: None,
        }
    }

    pub fn load_patch_file(&mut self, path: &Path) -> Result<(), LoadPatchError> {
        let prepared = prepare_patch_file(path)?;
        self.load_prepared_instrument(&prepared);
        Ok(())
    }

    pub fn render_patch_file_offline(
        &mut self,
        path: &Path,
        events: Vec<TimedInputEvent>,
    ) -> Result<OfflineRender, LoadPatchError> {
        self.render_patch_file_offline_with_events(path, |_| events)
    }

    pub fn render_patch_file_offline_with_events(
        &mut self,
        path: &Path,
        events: impl FnOnce(&patch::RenderSettings) -> Vec<TimedInputEvent>,
    ) -> Result<OfflineRender, LoadPatchError> {
        let prepared = prepare_patch_file(path)?;
        let events = events(&prepared.patch_doc().render);

        Ok(self.render_prepared_offline(&prepared, events))
    }

    fn render_prepared_offline(
        &mut self,
        prepared: &preparation::PreparedInstrument,
        events: Vec<TimedInputEvent>,
    ) -> OfflineRender {
        self.load_prepared_instrument(prepared);

        let (left, right) = crate::graph_processor::render_offline_with_sampler_assets_polyphonic(
            prepared.graph(),
            &prepared.patch_doc().render,
            events,
            prepared.sampler_assets(),
            &prepared.patch_doc().voice_allocation,
        );

        OfflineRender {
            sample_rate_hz: prepared.patch_doc().render.sample_rate_hz,
            left,
            right,
        }
    }

    pub fn prepare(&mut self, sample_rate: f32) {
        self.prepare_realtime(sample_rate, DEFAULT_PREPARED_MAX_BLOCK_SIZE);
    }

    pub fn prepare_realtime(&mut self, sample_rate: f32, max_block_size: usize) {
        self.sample_rate = sample_rate.max(1.0);
        self.prepared_max_block_size = max_block_size.max(1);
        self.fallback.set_sample_rate(sample_rate);
    }

    pub fn load_patch_with_sampler_assets(
        &mut self,
        patch_doc: &patch::PatchDocument,
        sampler_assets: &PreparedSamplerAssets,
    ) {
        let graph = Graph::from_patch_declarations(patch_doc);
        self.graph_processor = Some(
            RealtimeGraphProcessor::polyphonic_with_sampler_assets_and_max_block_size(
                graph,
                self.sample_rate,
                sampler_assets,
                &patch_doc.voice_allocation,
                self.prepared_max_block_size,
            ),
        );
    }

    fn load_prepared_instrument(&mut self, prepared: &preparation::PreparedInstrument) {
        self.graph_processor = Some(
            RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
                prepared.graph().clone(),
                prepared.compiled_patch().clone(),
                self.sample_rate,
                prepared.sampler_assets(),
                &prepared.patch_doc().voice_allocation,
                self.prepared_max_block_size,
            ),
        );
    }

    pub fn note_on(&mut self, note: u8, velocity: u8) {
        if let Some(gp) = &mut self.graph_processor {
            gp.note_on(note, velocity);
            return;
        }

        self.fallback.note_on(note, velocity);
    }

    pub fn note_off(&mut self, note: u8) {
        if let Some(gp) = &mut self.graph_processor {
            gp.note_off(note);
            return;
        }

        self.fallback.note_off(note);
    }

    pub fn handle_realtime_event(&mut self, event: RealtimeEvent) {
        match event {
            RealtimeEvent::NoteOn { note, velocity } => self.note_on(note, velocity),
            RealtimeEvent::NoteOff { note } => self.note_off(note),
        }
    }

    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) -> usize {
        if let Some(gp) = &mut self.graph_processor {
            return gp.render(left, right);
        }

        self.fallback.render(left, right)
    }

    pub fn is_finished(&self) -> bool {
        if let Some(gp) = &self.graph_processor {
            return gp.is_finished();
        }

        self.fallback.is_finished()
    }
}

fn prepare_patch_file(path: &Path) -> Result<preparation::PreparedInstrument, LoadPatchError> {
    preparation::prepare_instrument_file(path).map_err(LoadPatchError::from)
}

impl Default for DandrumEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn oldest_voice_index(voices: &[Voice; MAX_VOICES]) -> usize {
    voices
        .iter()
        .enumerate()
        .max_by_key(|(_, voice)| voice.sample_index)
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn midi_note_to_hz(note: u8) -> f32 {
    440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0)
}

fn envelope(time: f32, length: f32) -> f32 {
    let attack = 0.025;
    let release = 0.55;
    let decay = (-2.8 * time).exp();
    let fade_in = (time / attack).clamp(0.0, 1.0);
    let fade_out = ((length - time) / release).clamp(0.0, 1.0);

    fade_in * fade_out * decay
}

fn equal_power_pan(pan: f32) -> (f32, f32) {
    let angle = (pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
    (angle.cos(), angle.sin())
}

fn soft_clip(sample: f32) -> f32 {
    sample.tanh()
}

fn wrap_phase(phase: f32) -> f32 {
    if phase >= TAU { phase - TAU } else { phase }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::LoadedSample;
    use std::collections::BTreeMap;
    use std::error::Error;

    #[test]
    fn new_engine_starts_finished_until_a_note_is_triggered() {
        let mut engine = DandrumEngine::new();

        assert!(engine.is_finished());

        engine.note_on(60, 100);

        assert!(!engine.is_finished());
    }

    #[test]
    fn prepare_resets_active_voices_and_clamps_sample_rate() {
        let mut engine = DandrumEngine::new();
        engine.note_on(60, 100);

        engine.prepare(0.0);

        assert!(engine.is_finished());
    }

    #[test]
    fn render_adds_audio_for_active_voice() {
        let mut engine = DandrumEngine::new();
        let mut left = vec![0.0; 128];
        let mut right = vec![0.0; 128];

        engine.note_on(60, 127);
        let rendered = engine.render(&mut left, &mut right);

        assert_eq!(rendered, 128);
        assert!(left.iter().any(|sample| *sample != 0.0));
        assert!(right.iter().any(|sample| *sample != 0.0));
    }

    #[test]
    fn fallback_render_returns_rendered_sample_count() {
        let mut engine = DandrumEngine::new();
        let mut left = vec![0.0; 16];
        let mut right = vec![0.0; 16];

        assert_eq!(engine.render(&mut left, &mut right), 16);
    }

    #[test]
    fn fallback_first_samples_match_characterized_sound() {
        let mut engine = DandrumEngine::new();
        engine.note_on(60, 127);
        let mut left = vec![0.0; 16];
        let mut right = vec![0.0; 16];

        engine.render(&mut left, &mut right);

        assert_samples_close(
            &left,
            &[
                0.0,
                -0.0002307975,
                -0.0004429967,
                -0.0006359607,
                -0.0008091679,
                -0.0009622268,
                -0.0010948868,
                -0.0012070457,
                -0.0012987542,
                -0.0013702152,
                -0.0014217774,
                -0.0014539299,
                -0.0014672908,
                -0.0014625945,
                -0.0014406773,
                -0.0014024646,
            ],
        );
        assert_samples_close(
            &right,
            &[
                0.0,
                -0.00022759906,
                -0.00042988738,
                -0.00060583773,
                -0.00075466523,
                -0.0008758594,
                -0.0009692067,
                -0.001034804,
                -0.0010730614,
                -0.0010846917,
                -0.001070695,
                -0.0010323299,
                -0.00097108335,
                -0.00088863453,
                -0.00078681804,
                -0.00066758815,
            ],
        );
    }

    #[test]
    fn fallback_later_samples_match_characterized_vibrato() {
        let mut engine = DandrumEngine::new();
        engine.note_on(60, 127);
        let mut scratch_left = vec![0.0; 2048];
        let mut scratch_right = vec![0.0; 2048];
        engine.render(&mut scratch_left, &mut scratch_right);
        let mut left = vec![0.0; 8];
        let mut right = vec![0.0; 8];

        engine.render(&mut left, &mut right);

        assert_samples_close(
            &left,
            &[
                0.01752685,
                0.020387465,
                0.022927118,
                0.025151936,
                0.027069096,
                0.028686723,
                0.030013913,
                0.031060744,
            ],
        );
        assert_samples_close(
            &right,
            &[
                0.06240611,
                0.0631915,
                0.0634456,
                0.063187405,
                0.062438093,
                0.06122078,
                0.05956074,
                0.057485342,
            ],
        );
    }

    #[test]
    fn note_off_moves_matching_voice_toward_release() {
        let mut engine = DandrumEngine::new();
        engine.note_on(60, 127);

        engine.note_off(60);

        assert_eq!(
            engine.fallback.voice_sample_index(0),
            (engine.sample_rate * 0.85) as usize
        );
    }

    #[test]
    fn note_off_ignores_non_matching_fallback_voice() {
        let mut engine = DandrumEngine::new();
        engine.note_on(60, 127);

        engine.note_off(61);

        assert_eq!(engine.fallback.voice_sample_index(0), 0);
    }

    #[test]
    fn oldest_fallback_voice_is_stolen_when_all_slots_are_active() {
        let mut engine = DandrumEngine::new();
        let mut left = vec![0.0; 8];
        let mut right = vec![0.0; 8];
        for note in 0..MAX_VOICES {
            engine.note_on(40 + note as u8, 127);
        }
        engine.render(&mut left, &mut right);

        engine.note_on(80, 127);

        assert_eq!(engine.fallback.voices[MAX_VOICES - 1].note, 80);
        assert_eq!(engine.fallback.voice_sample_index(MAX_VOICES - 1), 0);
    }

    #[test]
    fn render_uses_shorter_output_buffer_length() {
        let mut engine = DandrumEngine::new();
        let mut left = vec![0.0; 64];
        let mut right = vec![0.0; 32];

        engine.note_on(60, 127);
        let rendered = engine.render(&mut left, &mut right);

        assert_eq!(rendered, 32);
    }

    #[test]
    fn loaded_sampler_patch_renders_prepared_sample_assets_realtime() {
        let patch = patch::load_patch_str(
            r#"
metadata:
  name: Realtime Sampler
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 128
assets:
  - id: hit
    kind: sample
    path: hit.wav
modules:
  - id: midi
    type: midi_input
  - id: sampler
    type: sampler
    parameters:
      asset: hit
  - id: out
    type: audio_output
connections:
  - from: midi.events
    to: sampler.trigger
  - from: sampler.audio
    to: out.left
  - from: sampler.audio
    to: out.right
"#,
        )
        .expect("patch should parse");
        let assets = PreparedSamplerAssets::from_samples_by_module(BTreeMap::from([(
            "sampler".to_string(),
            LoadedSample::new(48_000, vec![0.25, 0.5, 0.75]),
        )]));
        let mut engine = DandrumEngine::new();
        engine.prepare(48_000.0);
        engine.load_patch_with_sampler_assets(&patch, &assets);
        engine.note_on(60, 100);
        let mut left = vec![0.0; 4];
        let mut right = vec![0.0; 4];

        let rendered = engine.render(&mut left, &mut right);

        assert_eq!(rendered, 4);
        assert_eq!(left, vec![0.25, 0.5, 0.75, 0.0]);
        assert_eq!(right, vec![0.25, 0.5, 0.75, 0.0]);
    }

    #[test]
    fn failed_patch_file_load_preserves_existing_graph_runtime() {
        const BAD_PATCH_FILE_NAME: &str = "dandrum_test_bad_safe_rust_patch.yaml";
        const BAD_PATCH_YAML: &str =
            "metadata:\n  name: Bad\nrender:\n  sample_rate_hz: 48000\n  block_size_frames: 64\n  duration_frames: 128\nmodules: []";

        let patch = patch::load_patch_str(
            r#"
metadata:
  name: Existing Sampler Runtime
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 128
assets:
  - id: hit
    kind: sample
    path: hit.wav
modules:
  - id: midi
    type: midi_input
  - id: sampler
    type: sampler
    parameters:
      asset: hit
  - id: out
    type: audio_output
connections:
  - from: midi.events
    to: sampler.trigger
  - from: sampler.audio
    to: out.left
  - from: sampler.audio
    to: out.right
"#,
        )
        .expect("patch should parse");
        let assets = PreparedSamplerAssets::from_samples_by_module(BTreeMap::from([(
            "sampler".to_string(),
            LoadedSample::new(48_000, vec![0.25, 0.5, 0.75]),
        )]));
        let mut engine = DandrumEngine::new();
        engine.prepare(48_000.0);
        engine.load_patch_with_sampler_assets(&patch, &assets);

        let bad_patch_path = std::env::temp_dir().join(BAD_PATCH_FILE_NAME);
        std::fs::write(&bad_patch_path, BAD_PATCH_YAML).expect("bad patch should be written");

        let error = engine
            .load_patch_file(&bad_patch_path)
            .expect_err("invalid patch should fail to load");

        assert!(matches!(error, LoadPatchError::Validation(_)));

        engine.note_on(60, 100);
        let mut left = vec![0.0; 4];
        let mut right = vec![0.0; 4];
        let rendered = engine.render(&mut left, &mut right);

        assert_eq!(rendered, 4);
        assert_eq!(left, vec![0.25, 0.5, 0.75, 0.0]);
        assert_eq!(right, vec![0.25, 0.5, 0.75, 0.0]);

        std::fs::remove_file(bad_patch_path).ok();
    }

    #[test]
    fn fallback_render_is_deterministic_for_same_input() {
        let mut first = DandrumEngine::new();
        let mut second = DandrumEngine::new();
        first.note_on(60, 100);
        second.note_on(60, 100);
        let mut first_left = vec![0.0; 128];
        let mut first_right = vec![0.0; 128];
        let mut second_left = vec![0.0; 128];
        let mut second_right = vec![0.0; 128];

        assert_eq!(first.render(&mut first_left, &mut first_right), 128);
        assert_eq!(second.render(&mut second_left, &mut second_right), 128);

        assert_eq!(first_left, second_left);
        assert_eq!(first_right, second_right);
    }

    #[test]
    fn different_notes_produce_different_fallback_audio() {
        let mut low = DandrumEngine::new();
        let mut high = DandrumEngine::new();
        low.note_on(48, 127);
        high.note_on(72, 127);
        let mut low_left = vec![0.0; 256];
        let mut low_right = vec![0.0; 256];
        let mut high_left = vec![0.0; 256];
        let mut high_right = vec![0.0; 256];

        low.render(&mut low_left, &mut low_right);
        high.render(&mut high_left, &mut high_right);

        assert_ne!(low_left, high_left);
        assert_ne!(low_right, high_right);
    }

    #[test]
    fn fallback_output_is_stereo_panned() {
        let mut engine = DandrumEngine::new();
        engine.note_on(60, 127);
        let mut left = vec![0.0; 256];
        let mut right = vec![0.0; 256];

        engine.render(&mut left, &mut right);

        assert_ne!(left, right);
    }

    #[test]
    fn higher_velocity_produces_louder_fallback_output() {
        let mut soft = DandrumEngine::new();
        let mut loud = DandrumEngine::new();
        soft.note_on(60, 1);
        loud.note_on(60, 127);
        let mut soft_left = vec![0.0; 128];
        let mut soft_right = vec![0.0; 128];
        let mut loud_left = vec![0.0; 128];
        let mut loud_right = vec![0.0; 128];

        soft.render(&mut soft_left, &mut soft_right);
        loud.render(&mut loud_left, &mut loud_right);

        let soft_peak = soft_left.iter().cloned().fold(0.0f32, f32::max);
        let loud_peak = loud_left.iter().cloned().fold(0.0f32, f32::max);
        assert!(
            loud_peak > soft_peak,
            "higher velocity should produce louder output: soft={soft_peak}, loud={loud_peak}"
        );
    }

    #[test]
    fn fallback_render_without_notes_produces_silence() {
        let mut engine = DandrumEngine::new();
        let mut left = vec![0.0; 128];
        let mut right = vec![0.0; 128];

        engine.render(&mut left, &mut right);

        assert!(left.iter().all(|s| *s == 0.0));
        assert!(right.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn sustained_fallback_voice_uses_configured_duration() {
        let mut engine = DandrumEngine::new();
        engine.note_on(60, 100);
        let mut left = vec![0.0; 48_000];
        let mut right = vec![0.0; 48_000];

        engine.render(&mut left, &mut right);

        assert!(!engine.is_finished());
    }

    #[test]
    fn note_off_eventually_causes_fallback_silence() {
        let mut engine = DandrumEngine::new();
        engine.note_on(60, 100);
        engine.note_off(60);

        let mut left = vec![0.0; 128];
        let mut right = vec![0.0; 128];
        engine.render(&mut left, &mut right);

        assert!(!engine.is_finished(), "note_off starts release tail");

        for _ in 0..1_000 {
            engine.render(&mut left, &mut right);
            if engine.is_finished() {
                return;
            }
        }

        panic!("fallback synth should finish after release tail");
    }

    #[test]
    fn fallback_helpers_preserve_expected_math() {
        assert!((midi_note_to_hz(69) - 440.0).abs() < 0.001);
        assert!((midi_note_to_hz(81) - 880.0).abs() < 0.001);
        assert!(envelope(0.0, SECONDS) == 0.0);
        assert!((envelope(0.025, SECONDS) - (-2.8_f32 * 0.025).exp()).abs() < 0.001);
        assert_eq!(envelope(SECONDS, SECONDS), 0.0);
        assert!(envelope(0.05, SECONDS) > envelope(0.9, SECONDS));
        assert_eq!(wrap_phase(TAU + 0.25), 0.25);

        let (hard_left, hard_right) = equal_power_pan(-1.0);
        let (center_left, center_right) = equal_power_pan(0.0);
        let (far_left, far_right) = equal_power_pan(1.0);
        assert!(hard_left > hard_right);
        assert!((center_left - center_right).abs() < 0.001);
        assert!(far_right > far_left);
    }

    #[test]
    fn load_patch_error_display_and_source_are_specific() {
        let unsupported = LoadPatchError::UnsupportedFormat("patch.json".to_string());
        assert_eq!(
            unsupported.to_string(),
            "unsupported patch format: patch.json"
        );
        assert!(unsupported.source().is_none());

        let io_error = LoadPatchError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "missing patch",
        ));
        assert_eq!(io_error.to_string(), "I/O error: missing patch");
        assert!(io_error.source().is_some());

        let parse = LoadPatchError::Parse("bad yaml".to_string());
        assert_eq!(parse.to_string(), "parse error: bad yaml");
        assert!(parse.source().is_none());
    }

    #[test]
    fn queued_realtime_note_event_renders_like_direct_note_call() {
        let mut direct = DandrumEngine::new();
        let mut queued = DandrumEngine::new();
        let mut queue = crate::realtime::RealtimeEventQueue::with_capacity(4);

        direct.note_on(60, 100);
        assert_eq!(
            queue.submit(crate::realtime::RealtimeEvent::NoteOn {
                note: 60,
                velocity: 100
            }),
            crate::realtime::RealtimeEventSubmitStatus::Accepted
        );
        for event in queue.drain() {
            queued.handle_realtime_event(event);
        }

        let mut direct_left = vec![0.0; 64];
        let mut direct_right = vec![0.0; 64];
        let mut queued_left = vec![0.0; 64];
        let mut queued_right = vec![0.0; 64];

        assert_eq!(direct.render(&mut direct_left, &mut direct_right), 64);
        assert_eq!(queued.render(&mut queued_left, &mut queued_right), 64);
        assert_eq!(queued_left, direct_left);
        assert_eq!(queued_right, direct_right);
    }

    fn assert_samples_close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.iter().zip(expected) {
            assert!((actual - expected).abs() < 0.000_000_1);
        }
    }

}
