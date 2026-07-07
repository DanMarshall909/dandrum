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
                    let tone = soft_clip(saw * 0.55 + sine * 0.45) * env * GAIN * voice.velocity;
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

    pub(crate) fn render_prepared_instrument_offline(
        &mut self,
        prepared: &preparation::PreparedInstrument,
        events: Vec<TimedInputEvent>,
    ) -> OfflineRender {
        self.render_prepared_offline(prepared, events)
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

    pub fn set_numeric_parameter_by_target(
        &mut self,
        module_id: &str,
        parameter_name: &str,
        value: f32,
    ) -> bool {
        let Some(graph_processor) = &mut self.graph_processor else {
            return false;
        };

        graph_processor.set_numeric_parameter_by_target(module_id, parameter_name, value)
    }

    pub fn numeric_parameter_value(&self, module_id: &str, parameter_name: &str) -> Option<f32> {
        self.graph_processor
            .as_ref()
            .and_then(|processor| processor.numeric_parameter_value(module_id, parameter_name))
    }

    /// Resolve a (module_id, parameter_name) target to a slot index once, off the
    /// audio thread. The returned index can then be applied every block via
    /// `set_parameter_slot` without any string lookup.
    pub fn parameter_slot_index(&self, module_id: &str, parameter_name: &str) -> Option<usize> {
        self.graph_processor
            .as_ref()
            .and_then(|processor| processor.parameter_slot_index(module_id, parameter_name))
    }

    /// O(1) parameter update by a previously-resolved slot index. Safe to call
    /// from a realtime audio callback.
    pub fn set_parameter_slot(&mut self, slot_index: usize, value: f32) -> bool {
        let Some(gp) = &mut self.graph_processor else {
            return false;
        };

        gp.set_parameter_slot(slot_index, value)
    }

    pub fn note_on(&mut self, note: u8, velocity: u8) {
        self.note_on_at(note, velocity, 0);
    }

    pub fn note_off(&mut self, note: u8) {
        self.note_off_at(note, 0);
    }

    pub fn note_on_at(&mut self, note: u8, velocity: u8, frame_offset: u32) {
        if let Some(gp) = &mut self.graph_processor {
            gp.note_on_at(note, velocity, frame_offset);
            return;
        }

        self.fallback.note_on(note, velocity);
    }

    pub fn note_off_at(&mut self, note: u8, frame_offset: u32) {
        if let Some(gp) = &mut self.graph_processor {
            gp.note_off_at(note, frame_offset);
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
    use crate::patch;
    use crate::sample::LoadedSample;
    use std::collections::BTreeMap;

    #[test]
    fn new_engine_starts_finished_until_a_note_is_triggered() {
        let mut engine = DandrumEngine::new();

        assert!(engine.is_finished());

        engine.note_on(60, 100);

        assert!(!engine.is_finished());
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
    fn numeric_parameter_updates_are_delegated_to_runtime_slots() {
        let patch = patch::load_patch_str(
            r#"
metadata:
  name: Slot Parameter Test
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 128
modules:
  - id: osc
    type: oscillator
    parameters:
      pitch: 1
    outputs:
      - name: audio
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
    to: out.left
  - from: osc.audio
    to: out.right
"#,
        )
        .expect("patch should parse");
        let mut engine = DandrumEngine::new();
        engine.prepare(48_000.0);
        engine.load_patch_with_sampler_assets(&patch, &PreparedSamplerAssets::empty());

        assert_eq!(engine.numeric_parameter_value("osc", "pitch"), Some(1.0));
        assert!(engine.set_numeric_parameter_by_target("osc", "pitch", 2.0));
        assert_eq!(engine.numeric_parameter_value("osc", "pitch"), Some(2.0));
    }
}
