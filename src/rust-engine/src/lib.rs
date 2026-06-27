use std::f32::consts::TAU;

#[repr(C)]
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

#[unsafe(no_mangle)]
pub extern "C" fn dandrum_engine_create() -> *mut DandrumEngine {
    Box::into_raw(Box::new(DandrumEngine {
        sample_rate: 44_100.0,
        voices: [Voice::default(); MAX_VOICES],
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_destroy(engine: *mut DandrumEngine) {
    if !engine.is_null() {
        drop(unsafe { Box::from_raw(engine) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_prepare(engine: *mut DandrumEngine, sample_rate: f32) {
    let Some(engine) = (unsafe { engine.as_mut() }) else {
        return;
    };

    engine.sample_rate = sample_rate.max(1.0);
    engine.voices = [Voice::default(); MAX_VOICES];
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_note_on(
    engine: *mut DandrumEngine,
    note: u8,
    velocity: u8,
) {
    let Some(engine) = (unsafe { engine.as_mut() }) else {
        return;
    };

    let voice_index = engine
        .voices
        .iter()
        .position(|voice| !voice.active)
        .unwrap_or_else(|| oldest_voice_index(&engine.voices));

    engine.voices[voice_index] = Voice {
        active: true,
        note,
        velocity: (velocity as f32 / 127.0).clamp(0.0, 1.0),
        sample_index: 0,
        phases: [0.0; 5],
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_note_off(engine: *mut DandrumEngine, note: u8) {
    let Some(engine) = (unsafe { engine.as_mut() }) else {
        return;
    };

    for voice in &mut engine.voices {
        if voice.active && voice.note == note {
            voice.sample_index = voice.sample_index.max((engine.sample_rate * 0.85) as usize);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_render(
    engine: *mut DandrumEngine,
    left: *mut f32,
    right: *mut f32,
    num_samples: usize,
) -> usize {
    let Some(engine) = (unsafe { engine.as_mut() }) else {
        return 0;
    };

    if left.is_null() || right.is_null() {
        return 0;
    }

    let left = unsafe { std::slice::from_raw_parts_mut(left, num_samples) };
    let right = unsafe { std::slice::from_raw_parts_mut(right, num_samples) };
    let total_samples = (engine.sample_rate * SECONDS) as usize;

    for sample in 0..num_samples {
        let mut l = 0.0;
        let mut r = 0.0;

        for voice in &mut engine.voices {
            if !voice.active {
                continue;
            }

            if voice.sample_index >= total_samples {
                voice.active = false;
                continue;
            }

            let env = envelope(voice.sample_index as f32 / engine.sample_rate, SECONDS);
            let vibrato = (TAU * 5.1 * voice.sample_index as f32 / engine.sample_rate).sin() * 0.004;
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
                voice.phases[partial] = wrap_phase(phase + TAU * hz / engine.sample_rate);
            }

            voice.sample_index += 1;
        }

        left[sample] += soft_clip(l);
        right[sample] += soft_clip(r);
    }

    num_samples
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_is_finished(engine: *const DandrumEngine) -> bool {
    let Some(engine) = (unsafe { engine.as_ref() }) else {
        return true;
    };

    engine.voices.iter().all(|voice| !voice.active)
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
    if phase >= TAU {
        phase - TAU
    } else {
        phase
    }
}
