use std::f32::consts::TAU;

#[repr(C)]
pub struct DandrumEngine {
    sample_rate: f32,
    sample_index: usize,
    total_samples: usize,
    phases: [f32; 5],
}

const SECONDS: f32 = 1.25;
const GAIN: f32 = 0.16;
const FREQUENCIES: [f32; 5] = [110.0, 220.0, 277.18, 329.63, 440.0];
const PANS: [f32; 5] = [-0.65, -0.35, 0.0, 0.35, 0.65];

#[unsafe(no_mangle)]
pub extern "C" fn dandrum_engine_create() -> *mut DandrumEngine {
    Box::into_raw(Box::new(DandrumEngine {
        sample_rate: 44_100.0,
        sample_index: 0,
        total_samples: (44_100.0 * SECONDS) as usize,
        phases: [0.0; 5],
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
    engine.sample_index = 0;
    engine.total_samples = (engine.sample_rate * SECONDS) as usize;
    engine.phases = [0.0; 5];
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
    let mut rendered = 0;

    for sample in 0..num_samples {
        if engine.sample_index >= engine.total_samples {
            break;
        }

        let env = envelope(engine.sample_index as f32 / engine.sample_rate, SECONDS);
        let vibrato = (TAU * 5.1 * engine.sample_index as f32 / engine.sample_rate).sin() * 0.004;

        let mut l = 0.0;
        let mut r = 0.0;

        for voice in 0..FREQUENCIES.len() {
            let phase = engine.phases[voice];
            let saw = (phase / TAU) * 2.0 - 1.0;
            let sine = phase.sin();
            let tone = soft_clip(saw * 0.55 + sine * 0.45) * env * GAIN;
            let (left_gain, right_gain) = equal_power_pan(PANS[voice]);

            l += tone * left_gain;
            r += tone * right_gain;

            let hz = FREQUENCIES[voice] * (1.0 + vibrato);
            engine.phases[voice] = wrap_phase(phase + TAU * hz / engine.sample_rate);
        }

        left[sample] += soft_clip(l);
        right[sample] += soft_clip(r);
        engine.sample_index += 1;
        rendered += 1;
    }

    rendered
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_is_finished(engine: *const DandrumEngine) -> bool {
    let Some(engine) = (unsafe { engine.as_ref() }) else {
        return true;
    };

    engine.sample_index >= engine.total_samples
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
