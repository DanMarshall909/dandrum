use std::f32::consts::TAU;

use crate::graph::Graph;
use crate::graph_processor::RealtimeGraphProcessor;
use crate::patch;
use crate::realtime::RealtimeEvent;
use crate::sample::PreparedSamplerAssets;

pub struct DandrumEngine {
    sample_rate: f32,
    prepared_max_block_size: usize,
    voices: [Voice; MAX_VOICES],
    graph_processor: Option<RealtimeGraphProcessor>,
}

#[derive(Clone, Copy)]
struct Voice {
    active: bool,
    note: u8,
    velocity: f32,
    sample_index: usize,
    phases: [f32; 5],
}

const MAX_VOICES: usize = 16;
const SECONDS: f32 = 1.25;
const GAIN: f32 = 0.16;
const RATIOS: [f32; 5] = [0.5, 1.0, 1.259_921, 1.498_307, 2.0];
const PANS: [f32; 5] = [-0.65, -0.35, 0.0, 0.35, 0.65];
const DEFAULT_PREPARED_MAX_BLOCK_SIZE: usize = 512;

impl DandrumEngine {
    pub fn new() -> Self {
        Self {
            sample_rate: 44_100.0,
            prepared_max_block_size: DEFAULT_PREPARED_MAX_BLOCK_SIZE,
            voices: [Voice::default(); MAX_VOICES],
            graph_processor: None,
        }
    }

    pub fn prepare(&mut self, sample_rate: f32) {
        self.prepare_realtime(sample_rate, DEFAULT_PREPARED_MAX_BLOCK_SIZE);
    }

    pub fn prepare_realtime(&mut self, sample_rate: f32, max_block_size: usize) {
        self.sample_rate = sample_rate.max(1.0);
        self.prepared_max_block_size = max_block_size.max(1);
        self.voices = [Voice::default(); MAX_VOICES];
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

    pub fn note_on(&mut self, note: u8, velocity: u8) {
        if let Some(gp) = &mut self.graph_processor {
            gp.note_on(note, velocity);
            return;
        }

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

    pub fn note_off(&mut self, note: u8) {
        if let Some(gp) = &mut self.graph_processor {
            gp.note_off(note);
            return;
        }

        for voice in &mut self.voices {
            if voice.active && voice.note == note {
                voice.sample_index = voice.sample_index.max((self.sample_rate * 0.85) as usize);
            }
        }
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

    pub fn is_finished(&self) -> bool {
        if let Some(gp) = &self.graph_processor {
            return gp.is_finished();
        }
        self.voices.iter().all(|voice| !voice.active)
    }
}

impl Default for DandrumEngine {
    fn default() -> Self {
        Self::new()
    }
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
        assert_eq!(engine.sample_rate, 1.0);
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
    fn note_off_moves_matching_voice_toward_release() {
        let mut engine = DandrumEngine::new();
        engine.note_on(60, 127);

        engine.note_off(60);

        assert!(engine.voices[0].sample_index >= (engine.sample_rate * 0.85) as usize);
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
    fn note_off_eventually_causes_fallback_silence() {
        let mut engine = DandrumEngine::new();
        engine.note_on(60, 100);
        engine.note_off(60);

        let mut left = vec![0.0; 128];
        let mut right = vec![0.0; 128];
        engine.render(&mut left, &mut right);

        assert!(!engine.is_finished(), "note_off starts release tail");

        loop {
            engine.render(&mut left, &mut right);
            if engine.is_finished() {
                break;
            }
        }
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
}
