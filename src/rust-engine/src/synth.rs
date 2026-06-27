use std::f32::consts::TAU;

pub struct DandrumEngine {
    sample_rate: f32,
    voices: [Voice; MAX_VOICES],
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

impl DandrumEngine {
    pub fn new() -> Self {
        Self {
            sample_rate: 44_100.0,
            voices: [Voice::default(); MAX_VOICES],
        }
    }

    pub fn prepare(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
        self.voices = [Voice::default(); MAX_VOICES];
    }

    pub fn note_on(&mut self, note: u8, velocity: u8) {
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
        for voice in &mut self.voices {
            if voice.active && voice.note == note {
                voice.sample_index = voice.sample_index.max((self.sample_rate * 0.85) as usize);
            }
        }
    }

    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) -> usize {
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
}
