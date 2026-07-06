use crate::graph::builtin_ports;
use crate::script::{ScriptEvent, ScriptExecutionContext, ScriptProcessInput, ScriptRuntime};
use std::collections::BTreeMap;

use super::helpers::{
    audio_output, lerp, log_lerp, normalized_end_position, normalized_position,
    set_curve_by_index, stereo_audio_output,
};
use super::outputs::{BlockEvent, ModuleOutputs};
use super::state::PerModuleState;
use crate::decay::DecayCurve;

pub(super) struct EchoControls<'a> {
    pub(super) feedback: &'a [f32],
    pub(super) damping: &'a [f32],
    pub(super) wet: &'a [f32],
    pub(super) dry: &'a [f32],
    pub(super) time_l: &'a [f32],
    pub(super) time_r: &'a [f32],
    pub(super) ping_pong: &'a [f32],
}

pub(super) struct ReverbControls<'a> {
    pub(super) decay_time: &'a [f32],
    pub(super) room_size: &'a [f32],
    pub(super) damping: &'a [f32],
    pub(super) diffusion: &'a [f32],
    pub(super) wet: &'a [f32],
    pub(super) dry: &'a [f32],
    pub(super) pre_delay: &'a [f32],
    pub(super) stereo_width: &'a [f32],
}

pub(super) fn process_oscillator(
    state: &mut PerModuleState,
    pitch_ratio: &[f32],
    frames: usize,
) -> ModuleOutputs {
    let (phase, sample_rate) = match state {
        PerModuleState::Oscillator { phase, sample_rate } => (phase, *sample_rate),
        _ => unreachable!(),
    };

    let mut audio = Vec::with_capacity(frames);
    for &ratio in pitch_ratio.iter().take(frames) {
        let base_hz = 220.0;
        let freq = base_hz * ratio;
        let phase_inc = freq / sample_rate;
        audio.push(*phase * 2.0 - 1.0);
        *phase += phase_inc;
        if *phase >= 1.0 {
            *phase -= 1.0;
        }
    }

    audio_output(builtin_ports::AUDIO, audio)
}

pub(super) fn process_adsr(
    state: &mut PerModuleState,
    events_in: &[BlockEvent],
    attack_in: &[f32],
    decay_in: &[f32],
    sustain_in: &[f32],
    release_in: &[f32],
    block_start_frame: u64,
    frames: usize,
) -> ModuleOutputs {
    let (level, gate_active, release_start_frame, release_start_level, sample_rate) = match state {
        PerModuleState::Adsr {
            level,
            gate_active,
            release_start_frame,
            release_start_level,
            sample_rate,
        } => (
            level,
            gate_active,
            release_start_frame,
            release_start_level,
            *sample_rate,
        ),
        _ => unreachable!(),
    };

    for event in events_in {
        match &event.event {
            ScriptEvent::NoteOn { .. } => {
                *gate_active = true;
                *release_start_frame = block_start_frame;
            }
            ScriptEvent::NoteOff { .. } => {
                *gate_active = false;
                *release_start_frame = block_start_frame;
                *release_start_level = *level;
            }
        }
    }

    let mut adsr_value = Vec::with_capacity(frames);

    for i in 0..frames {
        let absolute_frame = block_start_frame + i as u64;

        let attack_ms = adsr_time_ms(attack_in[i], 2.0, 100.0);
        let decay_ms = adsr_time_ms(decay_in[i], 10.0, 1000.0);
        let sustain = sustain_in[i].clamp(0.0, 1.0);
        let release_ms = adsr_time_ms(release_in[i], 10.0, 3000.0);

        let attack_frames = (sample_rate * attack_ms / 1000.0) as u64;
        let decay_frames = (sample_rate * decay_ms / 1000.0) as u64;
        let release_frames = (sample_rate * release_ms / 1000.0) as u64;

        if *gate_active {
            let lifetime = absolute_frame - *release_start_frame;
            if lifetime < attack_frames {
                adsr_value.push((lifetime as f32) / (attack_frames as f32));
            } else if lifetime < attack_frames + decay_frames {
                let decay_progress = (lifetime - attack_frames) as f32 / (decay_frames as f32);
                adsr_value.push(1.0 - (1.0 - sustain) * decay_progress);
            } else {
                adsr_value.push(sustain);
            }
        } else {
            let release_progress =
                (absolute_frame - *release_start_frame) as f32 / (release_frames as f32);
            if release_progress >= 1.0 {
                adsr_value.push(0.0);
            } else {
                adsr_value.push(*release_start_level * (1.0 - release_progress));
            }
        }
    }

    *level = *adsr_value.last().unwrap_or(&0.0);

    let mut outputs = ModuleOutputs::empty();
    outputs
        .control
        .insert(builtin_ports::VALUE.to_string(), adsr_value);
    outputs
}

fn adsr_time_ms(value: f32, min_ms: f32, max_ms: f32) -> f32 {
    if value > 1.0 {
        value.clamp(min_ms, max_ms)
    } else {
        lerp(min_ms, max_ms, value.clamp(0.0, 1.0))
    }
}

pub(super) fn process_vca(audio_in: Vec<f32>, gain_in: Vec<f32>) -> ModuleOutputs {
    let frames = audio_in.len().min(gain_in.len());
    let mut audio = Vec::with_capacity(frames);
    for i in 0..frames {
        audio.push(audio_in[i] * gain_in[i]);
    }

    audio_output(builtin_ports::AUDIO_OUT, audio)
}

pub(super) fn process_sampler(
    state: &mut PerModuleState,
    events_in: &[BlockEvent],
    rate_in: &[f32],
    start_in: &[f32],
    loop_enabled_in: &[f32],
    loop_start_in: &[f32],
    loop_end_in: &[f32],
    frames: usize,
) -> ModuleOutputs {
    let PerModuleState::Sampler {
        sample,
        position,
        active,
    } = state
    else {
        unreachable!()
    };

    let mut audio = vec![0.0; frames];
    let Some(sample_ref) = sample.as_ref() else {
        return audio_output(builtin_ports::AUDIO, audio);
    };
    let sample_frames = sample_ref.frames();
    if sample_frames.is_empty() {
        return audio_output(builtin_ports::AUDIO, audio);
    }

    let mut events = events_in.to_vec();
    events.sort_by_key(|event| event.frame_offset);
    let mut next_event = 0usize;

    for frame in 0..frames {
        while next_event < events.len() && events[next_event].frame_offset as usize == frame {
            if matches!(events[next_event].event, ScriptEvent::NoteOn { .. }) {
                *position = normalized_position(
                    start_in.get(frame).copied().unwrap_or(0.0),
                    sample_frames.len(),
                );
                *active = true;
            }
            next_event += 1;
        }

        if !*active {
            continue;
        }

        let idx = *position as usize;
        if idx >= sample_frames.len() {
            *active = false;
            continue;
        }

        audio[frame] = sample_frames[idx];

        let rate = rate_in.get(frame).copied().unwrap_or(1.0).max(0.0);
        *position += rate;

        if loop_enabled_in.get(frame).copied().unwrap_or(0.0) > 0.5 {
            let loop_start = normalized_position(
                loop_start_in.get(frame).copied().unwrap_or(0.0),
                sample_frames.len(),
            );
            let mut loop_end = normalized_end_position(
                loop_end_in.get(frame).copied().unwrap_or(1.0),
                sample_frames.len(),
            );
            if loop_end <= loop_start {
                loop_end = sample_frames.len() as f32;
            }
            while *position >= loop_end {
                *position = loop_start + (*position - loop_end);
            }
        } else if *position >= sample_frames.len() as f32 {
            *active = false;
        }
    }

    audio_output(builtin_ports::AUDIO, audio)
}

pub(super) fn process_note_to_rate(
    state: &mut PerModuleState,
    events_in: &[BlockEvent],
    frames: usize,
) -> ModuleOutputs {
    let rate = match state {
        PerModuleState::NoteToRate { rate } => rate,
        _ => unreachable!(),
    };
    let mut events = events_in.to_vec();
    events.sort_by_key(|event| event.frame_offset);
    let mut next_event = 0usize;
    let mut output = Vec::with_capacity(frames);

    for frame in 0..frames {
        while next_event < events.len() && events[next_event].frame_offset as usize == frame {
            if let ScriptEvent::NoteOn { note, .. } = events[next_event].event {
                *rate = 2.0f32.powf((note as f32 - 60.0) / 12.0);
            }
            next_event += 1;
        }
        output.push(*rate);
    }

    let mut outputs = ModuleOutputs::empty();
    outputs
        .control
        .insert(builtin_ports::RATE.to_string(), output);
    outputs
}

pub(super) fn process_event_filter(
    state: &mut PerModuleState,
    events_in: &[BlockEvent],
) -> ModuleOutputs {
    let note = match state {
        PerModuleState::EventFilter { note } => *note,
        _ => unreachable!(),
    };

    let mut outputs = ModuleOutputs::empty();
    outputs.events.reserve(events_in.len());
    for event in events_in {
        if event_matches_note(event, note) {
            outputs.events.push(event.clone());
        }
    }
    outputs
}

pub(super) fn process_envelope_follower(
    state: &mut PerModuleState,
    audio_in: &[f32],
    attack_in: &[f32],
    release_in: &[f32],
    amount_in: &[f32],
    offset_in: &[f32],
    invert_in: &[f32],
    frames: usize,
) -> ModuleOutputs {
    let (detector, mode) = match state {
        PerModuleState::EnvelopeFollower { detector, mode } => (detector, *mode),
        _ => unreachable!(),
    };

    detector.set_mode(mode);
    let mut output = Vec::with_capacity(frames);
    for i in 0..frames {
        let attack_ms = attack_in.get(i).copied().unwrap_or(5.0).max(0.0) as f64;
        let release_ms = release_in.get(i).copied().unwrap_or(50.0).max(0.0) as f64;
        detector.set_params(attack_ms, release_ms);

        let envelope = detector.process(audio_in.get(i).copied().unwrap_or(0.0) as f64) as f32;
        let shaped = if invert_in.get(i).copied().unwrap_or(0.0) > 0.5 {
            1.0 - envelope
        } else {
            envelope
        };
        let amount = amount_in.get(i).copied().unwrap_or(1.0);
        let offset = offset_in.get(i).copied().unwrap_or(0.0);
        output.push(finite_or_zero(shaped * amount + offset).clamp(0.0, 1.0));
    }

    let mut outputs = ModuleOutputs::empty();
    outputs
        .control
        .insert(builtin_ports::VALUE.to_string(), output);
    outputs
}

pub(super) fn process_curve_mapper(
    state: &mut PerModuleState,
    value_in: &[f32],
    amount_in: &[f32],
    bias_in: &[f32],
    scale_in: &[f32],
    offset_in: &[f32],
    frames: usize,
) -> ModuleOutputs {
    let mapper = match state {
        PerModuleState::CurveMapper { mapper } => mapper,
        _ => unreachable!(),
    };

    let mut output = Vec::with_capacity(frames);
    for i in 0..frames {
        output.push(mapper.process(
            value_in.get(i).copied().unwrap_or(0.0),
            amount_in.get(i).copied().unwrap_or(1.0),
            bias_in.get(i).copied().unwrap_or(0.0),
            scale_in.get(i).copied().unwrap_or(1.0),
            offset_in.get(i).copied().unwrap_or(0.0),
        ));
    }

    let mut outputs = ModuleOutputs::empty();
    outputs
        .control
        .insert(builtin_ports::VALUE.to_string(), output);
    outputs
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn event_matches_note(event: &BlockEvent, expected_note: Option<u8>) -> bool {
    let Some(expected_note) = expected_note else {
        return true;
    };

    match event.event {
        ScriptEvent::NoteOn { note, .. } | ScriptEvent::NoteOff { note } => note == expected_note,
    }
}

pub(super) fn process_dynamics_processor(
    state: &mut PerModuleState,
    audio_in: &[f32],
    sidechain_in: &[f32],
    threshold_in: &[f32],
    below_ratio_in: &[f32],
    above_ratio_in: &[f32],
    attack_in: &[f32],
    release_in: &[f32],
    knee_in: &[f32],
    makeup_in: &[f32],
    attack_gain_in: &[f32],
    sustain_gain_in: &[f32],
    frames: usize,
) -> ModuleOutputs {
    let (processor, _sample_rate) = match state {
        PerModuleState::DynamicsProcessor {
            processor,
            sample_rate,
        } => (processor, *sample_rate),
        _ => unreachable!(),
    };

    let mut audio_out = Vec::with_capacity(frames);

    for i in 0..frames {
        let threshold_db = lerp(-80.0, 0.0, threshold_in[i]);
        let below_ratio = lerp(0.0, 20.0, below_ratio_in[i]);
        let above_ratio = lerp(1.0, 40.0, above_ratio_in[i]);
        let attack_ms = log_lerp(0.1, 100.0, attack_in[i]);
        let release_ms = log_lerp(10.0, 3000.0, release_in[i]);
        let knee_db = lerp(0.0, 12.0, knee_in[i]);
        let makeup_gain_db = lerp(0.0, 24.0, makeup_in[i]);
        let attack_gain_db = lerp(-24.0, 24.0, attack_gain_in[i]);
        let sustain_gain_db = lerp(-24.0, 24.0, sustain_gain_in[i]);

        processor.set_level_params(
            threshold_db as f64,
            below_ratio as f64,
            above_ratio as f64,
            knee_db as f64,
            makeup_gain_db as f64,
        );
        processor.set_transient_params(attack_gain_db as f64, sustain_gain_db as f64);
        processor.set_time_constants(attack_ms as f64, release_ms as f64);

        let has_sidechain = i < sidechain_in.len() && sidechain_in[i] != 0.0;
        let sc = if has_sidechain {
            Some(sidechain_in[i] as f64)
        } else {
            None
        };

        let out = processor.process(audio_in[i] as f64, sc);
        audio_out.push(out as f32);
    }

    audio_output(builtin_ports::AUDIO_OUT, audio_out)
}

pub(super) fn process_filter(
    state: &mut PerModuleState,
    audio_in: &[f32],
    cutoff_in: &[f32],
    resonance_in: &[f32],
    gain_in: &[f32],
    frames: usize,
) -> ModuleOutputs {
    let (filter, sample_rate) = match state {
        PerModuleState::Filter {
            filter,
            sample_rate,
        } => (filter, *sample_rate),
        _ => unreachable!(),
    };

    let mut audio_out = Vec::with_capacity(frames);

    for i in 0..frames {
        filter.set_cutoff_control(cutoff_in[i], sample_rate);
        filter.set_resonance_control(resonance_in[i]);
        filter.set_gain_db(gain_in[i] as f64 * 48.0 - 24.0);
        audio_out.push(filter.process(audio_in[i]));
    }

    audio_output(builtin_ports::AUDIO_OUT, audio_out)
}

pub(super) fn process_saturator(
    state: &mut PerModuleState,
    audio_in: &[f32],
    drive_in: &[f32],
    bias_in: &[f32],
    curve_select_in: &[f32],
    frames: usize,
) -> ModuleOutputs {
    let processor = match state {
        PerModuleState::Saturator { processor } => processor,
        _ => unreachable!(),
    };

    let mut audio_out = Vec::with_capacity(frames);

    for i in 0..frames {
        let drive_db = lerp(0.0, 48.0, drive_in[i]);
        let bias = lerp(-1.0, 1.0, bias_in[i]);
        let curve_idx = (curve_select_in[i] * 4.0).round().clamp(0.0, 4.0) as usize;

        processor.set_drive_db(drive_db as f64);
        processor.set_bias(bias as f64);
        set_curve_by_index(processor, curve_idx);
        let out = processor.process(audio_in[i] as f64);
        audio_out.push(out as f32);
    }

    audio_output(builtin_ports::AUDIO_OUT, audio_out)
}

pub(super) fn process_convolution(
    state: &mut PerModuleState,
    audio_in: &[f32],
    mix_in: &[f32],
    frames: usize,
) -> ModuleOutputs {
    let processor = match state {
        PerModuleState::Convolution { processor } => processor,
        _ => unreachable!(),
    };

    let mut audio_out = Vec::with_capacity(frames);

    for i in 0..frames {
        let mix = mix_in[i].clamp(0.0, 1.0);
        processor.set_wet(mix);
        audio_out.push(processor.process(audio_in[i]));
    }

    audio_output(builtin_ports::AUDIO_OUT, audio_out)
}

pub(super) fn process_echo(
    state: &mut PerModuleState,
    audio_in_l: &[f32],
    audio_in_r: &[f32],
    controls: EchoControls<'_>,
    frames: usize,
) -> ModuleOutputs {
    let (processor, _sample_rate) = match state {
        PerModuleState::Echo {
            processor,
            sample_rate,
        } => (processor, *sample_rate),
        _ => unreachable!(),
    };

    let mut out_l = Vec::with_capacity(frames);
    let mut out_r = Vec::with_capacity(frames);

    for i in 0..frames {
        let feedback = controls.feedback.get(i).copied().unwrap_or(0.5);
        let damping_norm = controls.damping.get(i).copied().unwrap_or(0.5);
        let damping_hz = 20.0 * 1000.0_f32.powf(damping_norm);
        let wet = controls.wet.get(i).copied().unwrap_or(0.7);
        let dry = controls.dry.get(i).copied().unwrap_or(0.5);
        let time_l = lerp(1.0, 2000.0, controls.time_l.get(i).copied().unwrap_or(0.5));
        let time_r = lerp(1.0, 2000.0, controls.time_r.get(i).copied().unwrap_or(0.5));
        let ping_pong = controls.ping_pong.get(i).copied().unwrap_or(0.0) > 0.5;

        processor.set_feedback(feedback);
        processor.set_damping_cutoff(damping_hz as f64);
        processor.set_wet_dry(wet, dry);
        processor.set_delay_ms(time_l as f64, time_r as f64);
        processor.set_ping_pong(ping_pong);

        let in_l = audio_in_l.get(i).copied().unwrap_or(0.0);
        let in_r = audio_in_r.get(i).copied().unwrap_or(0.0);
        let (l, r) = processor.process(in_l, in_r);
        out_l.push(l);
        out_r.push(r);
    }

    stereo_audio_output(out_l, out_r)
}

pub(super) fn process_reverb(
    state: &mut PerModuleState,
    audio_in_l: &[f32],
    audio_in_r: &[f32],
    controls: ReverbControls<'_>,
    frames: usize,
) -> ModuleOutputs {
    let (processor, _sample_rate) = match state {
        PerModuleState::Reverb {
            processor,
            sample_rate,
        } => (processor, *sample_rate),
        _ => unreachable!(),
    };

    let mut out_l = Vec::with_capacity(frames);
    let mut out_r = Vec::with_capacity(frames);

    for i in 0..frames {
        let decay_sec = lerp(
            0.1,
            10.0,
            controls.decay_time.get(i).copied().unwrap_or(0.5),
        );
        let room_size = controls.room_size.get(i).copied().unwrap_or(0.5);
        let damping_norm = controls.damping.get(i).copied().unwrap_or(0.5);
        let damping_hz = 20.0 * 1000.0_f32.powf(damping_norm);
        let diffusion = controls.diffusion.get(i).copied().unwrap_or(0.5);
        let wet = controls.wet.get(i).copied().unwrap_or(0.7);
        let dry = controls.dry.get(i).copied().unwrap_or(0.5);
        let pre_delay_ms = lerp(
            0.0,
            250.0,
            controls.pre_delay.get(i).copied().unwrap_or(0.0),
        );
        let stereo_width = controls.stereo_width.get(i).copied().unwrap_or(0.5);

        processor.set_decay_time(decay_sec as f64);
        processor.set_room_size(room_size);
        processor.set_damping(damping_hz as f64);
        processor.set_diffusion(diffusion);
        processor.set_wet_dry(wet, dry);
        processor.set_pre_delay(pre_delay_ms as f64);
        processor.set_stereo_width(stereo_width);

        let in_l = audio_in_l.get(i).copied().unwrap_or(0.0);
        let in_r = audio_in_r.get(i).copied().unwrap_or(0.0);
        let (l, r) = processor.process(in_l, in_r);
        out_l.push(l);
        out_r.push(r);
    }

    stereo_audio_output(out_l, out_r)
}

pub(super) fn process_frequency_splitter(
    state: &mut PerModuleState,
    audio_in: &[f32],
    crossover_hz_in: &[f32],
    frames: usize,
) -> ModuleOutputs {
    let (lp1, lp2, sample_rate) = match state {
        PerModuleState::FrequencySplitter {
            first,
            second,
            sample_rate,
        } => (first, second, *sample_rate),
        _ => unreachable!(),
    };

    let mut low = Vec::with_capacity(frames);
    let mut mid = Vec::with_capacity(frames);
    let mut high = Vec::with_capacity(frames);

    for i in 0..frames {
        let hz = (crossover_hz_in[i] as f64 * 16000.0 + 40.0).clamp(40.0, 20000.0);
        let norm = (hz / sample_rate).clamp(0.0, 0.49);
        lp1.set_crossover(norm);
        lp2.set_crossover((norm * 4.0).clamp(0.0, 0.49));

        let (l, rest) = lp1.process(audio_in[i]);
        let (m, h) = lp2.process(rest);
        low.push(l);
        mid.push(m);
        high.push(h);
    }

    let mut outputs = ModuleOutputs::empty();
    outputs.audio.insert("low".to_string(), low);
    outputs.audio.insert("mid".to_string(), mid);
    outputs.audio.insert("high".to_string(), high);
    outputs
}

pub(super) fn process_spectral_processor(
    state: &mut PerModuleState,
    audio_in: &[f32],
    threshold_in: &[f32],
    mix_in: &[f32],
    frames: usize,
) -> ModuleOutputs {
    let processor = match state {
        PerModuleState::SpectralProcessor { processor } => processor,
        _ => unreachable!(),
    };

    let mut audio_out = Vec::with_capacity(frames);

    for i in 0..frames {
        let threshold_db = threshold_in[i] as f64 * 80.0 - 40.0;
        let mix = mix_in[i];
        processor.set_threshold(threshold_db);
        let processed = processor.process(audio_in[i]);
        audio_out.push(processed * mix + audio_in[i] * (1.0 - mix));
    }

    audio_output(builtin_ports::AUDIO_OUT, audio_out)
}

pub(super) fn process_noise(state: &mut PerModuleState, frames: usize) -> ModuleOutputs {
    let rng_state = match state {
        PerModuleState::Noise { state, .. } => state,
        _ => unreachable!(),
    };

    let mut audio = Vec::with_capacity(frames);
    for _ in 0..frames {
        // Simple xorshift32 PRNG seeded deterministically.
        let mut x = *rng_state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        // Normalize to [-1.0, 1.0]
        let sample = (x as f32) / (u32::MAX as f32) * 2.0 - 1.0;
        audio.push(sample);
        *rng_state = x;
    }

    audio_output(builtin_ports::AUDIO, audio)
}

pub(super) fn process_decay(
    state: &mut PerModuleState,
    events: &[BlockEvent],
    frames: usize,
) -> ModuleOutputs {
    let (level, triggered, elapsed_frames, decay_frames, curve) = match state {
        PerModuleState::Decay {
            level,
            triggered,
            elapsed_frames,
            decay_frames,
            curve,
        } => (level, triggered, elapsed_frames, *decay_frames, *curve),
        _ => unreachable!(),
    };

    for event in events {
        if matches!(event.event, ScriptEvent::NoteOn { .. }) {
            *level = 1.0;
            *triggered = true;
            *elapsed_frames = 0;
        }
    }

    let mut values = Vec::with_capacity(frames);
    for _ in 0..frames {
        if *triggered {
            let t = *elapsed_frames as f32 / decay_frames;
            *level = match curve {
                DecayCurve::Linear => (1.0 - t).max(0.0),
                DecayCurve::Exponential => (-4.0 * t).exp(),
            };
            *elapsed_frames += 1;
            if *level <= 0.0 {
                *level = 0.0;
                *triggered = false;
            }
        }
        values.push(*level);
    }

    let mut outputs = ModuleOutputs::empty();
    outputs
        .control
        .insert(builtin_ports::VALUE.to_string(), values);
    outputs
}

pub(super) fn process_impulse(
    _state: &mut PerModuleState,
    events: &[BlockEvent],
    frames: usize,
) -> ModuleOutputs {
    let mut audio = vec![0.0_f32; frames];
    for event in events {
        let f = event.frame_offset as usize;
        if f < frames {
            audio[f] = 1.0;
        }
    }
    audio_output(builtin_ports::AUDIO, audio)
}

pub(super) fn process_multiply(a: Vec<f32>, b: Vec<f32>) -> ModuleOutputs {
    let max = a.len().max(b.len());
    let mut audio = Vec::with_capacity(max);
    for i in 0..max {
        let av = a.get(i).copied().unwrap_or(0.0);
        let bv = b.get(i).copied().unwrap_or(0.0);
        audio.push(av * bv);
    }
    audio_output(builtin_ports::AUDIO_OUT, audio)
}

pub(super) fn process_note_to_control(
    state: &mut PerModuleState,
    events: &[BlockEvent],
    frames: usize,
) -> ModuleOutputs {
    let (gate_active, current_note, current_velocity, current_frequency, current_pitch_ratio) =
        match state {
            PerModuleState::NoteToControl {
                gate_active,
                current_note,
                current_velocity,
                current_frequency,
                current_pitch_ratio,
            } => (
                gate_active,
                current_note,
                current_velocity,
                current_frequency,
                current_pitch_ratio,
            ),
            _ => unreachable!(),
        };

    let mut frequency_out = vec![0.0_f32; frames];
    let mut pitch_ratio_out = vec![0.0_f32; frames];
    let mut velocity_out = vec![0.0_f32; frames];
    let mut gate_events = Vec::new();

    for event in events {
        let f = event.frame_offset as usize;
        match &event.event {
            ScriptEvent::NoteOn { note, velocity } => {
                let freq = midi_note_to_freq(*note);
                let ratio = freq / 220.0;
                let norm_vel = (*velocity as f32) / 127.0;
                *current_note = Some(*note);
                *current_velocity = norm_vel;
                *current_frequency = freq;
                *current_pitch_ratio = ratio;
                if f < frames {
                    frequency_out[f] = freq;
                    pitch_ratio_out[f] = ratio;
                    velocity_out[f] = norm_vel;
                }
                *gate_active = true;
                gate_events.push(BlockEvent {
                    frame_offset: event.frame_offset,
                    event: ScriptEvent::NoteOn {
                        note: *note,
                        velocity: *velocity,
                    },
                });
            }
            ScriptEvent::NoteOff { note } => {
                if current_note.map(|n| n == *note).unwrap_or(false) {
                    *gate_active = false;
                    *current_note = None;
                    *current_velocity = 0.0;
                    *current_frequency = 0.0;
                    *current_pitch_ratio = 0.0;
                }
                gate_events.push(BlockEvent {
                    frame_offset: event.frame_offset,
                    event: ScriptEvent::NoteOff { note: *note },
                });
            }
        }
    }

    if *gate_active {
        for frame in 0..frames {
            frequency_out[frame] = *current_frequency;
            pitch_ratio_out[frame] = *current_pitch_ratio;
            velocity_out[frame] = *current_velocity;
        }
    }

    let mut outputs = ModuleOutputs::empty();
    outputs
        .control
        .insert("frequency".to_string(), frequency_out);
    outputs
        .control
        .insert("pitch_ratio".to_string(), pitch_ratio_out);
    outputs.control.insert("velocity".to_string(), velocity_out);
    outputs.events = gate_events;
    outputs
}

pub(super) fn process_script(
    state: &mut PerModuleState,
    events_in: &[BlockEvent],
    control_inputs: BTreeMap<String, f32>,
    frames: usize,
) -> ModuleOutputs {
    let PerModuleState::Script {
        runtime,
        state: script_state,
        ..
    } = state
    else {
        unreachable!();
    };

    let input = ScriptProcessInput::new(
        events_in.iter().map(|event| event.event.clone()).collect(),
        control_inputs,
        ScriptExecutionContext::new(1_000),
        script_state.clone(),
    );

    let Ok(script_output) = runtime.process(input) else {
        return ModuleOutputs::empty();
    };

    *script_state = script_output.state;

    let mut outputs = ModuleOutputs::empty();
    for (port, events) in script_output.events {
        let block_events: Vec<BlockEvent> = events
            .into_iter()
            .map(|event| BlockEvent {
                frame_offset: 0,
                event,
            })
            .collect();
        outputs.events.extend(block_events.clone());
        outputs.event_ports.insert(port, block_events);
    }

    for (port, value) in script_output.controls {
        outputs.control.insert(port, vec![value; frames]);
    }

    outputs
}

fn midi_note_to_freq(note: u8) -> f32 {
    440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0)
}
