use std::collections::HashMap;

use crate::filter::FilterAlgorithm;
use crate::graph::builtin_ports;
use crate::script::ScriptEvent;

use super::ModuleInputProvider;
use super::helpers::{
    audio_output, has_signal, lerp, log_lerp, normalized_end_position, normalized_position,
    set_curve_by_index, stereo_audio_output,
};
use super::outputs::{BlockEvent, ModuleOutputs};
use super::state::PerModuleState;

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

    let has_attack = has_signal(attack_in);
    let has_decay = has_signal(decay_in);
    let has_sustain = has_signal(sustain_in);
    let has_release = has_signal(release_in);

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

        let attack_ms = if has_attack {
            lerp(2.0, 100.0, attack_in[i].clamp(0.0, 1.0))
        } else {
            5.0
        };
        let decay_ms = if has_decay {
            lerp(10.0, 1000.0, decay_in[i].clamp(0.0, 1.0))
        } else {
            30.0
        };
        let sustain = if has_sustain {
            sustain_in[i].clamp(0.0, 1.0)
        } else {
            0.7
        };
        let release_ms = if has_release {
            lerp(10.0, 3000.0, release_in[i].clamp(0.0, 1.0))
        } else {
            200.0
        };

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
    let (sample, position, active) = match state {
        PerModuleState::Sampler {
            sample,
            position,
            active,
        } => (sample.clone(), position, active),
        _ => unreachable!(),
    };

    let mut audio = vec![0.0; frames];
    let Some(sample) = sample else {
        return audio_output(builtin_ports::AUDIO, audio);
    };
    let sample_frames = sample.frames();
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
    frames: usize,
) -> ModuleOutputs {
    let (filter, _sample_rate) = match state {
        PerModuleState::Filter {
            filter,
            sample_rate,
        } => (filter, *sample_rate),
        _ => unreachable!(),
    };

    let mut audio_out = Vec::with_capacity(frames);

    for i in 0..frames {
        let base: f64 = 8000.0 / 20.0;
        let hz = 20.0 * base.powf(cutoff_in[i] as f64);
        filter.set_cutoff(hz);
        filter.set_resonance(0.4);
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
    module_idx: usize,
    input_provider: &impl ModuleInputProvider,
    all_outputs: &HashMap<usize, ModuleOutputs>,
    frames: usize,
) -> ModuleOutputs {
    let (processor, _sample_rate) = match state {
        PerModuleState::Echo {
            processor,
            sample_rate,
        } => (processor, *sample_rate),
        _ => unreachable!(),
    };

    let feedback_in = input_provider.control_input_or_default(
        module_idx,
        builtin_ports::FEEDBACK,
        all_outputs,
        frames,
        0.5,
    );
    let damping_in = input_provider.control_input_or_default(
        module_idx,
        builtin_ports::DAMPING_CUTOFF,
        all_outputs,
        frames,
        0.5,
    );
    let wet_in = input_provider.control_input_or_default(
        module_idx,
        builtin_ports::WET,
        all_outputs,
        frames,
        0.7,
    );
    let dry_in = input_provider.control_input_or_default(
        module_idx,
        builtin_ports::DRY,
        all_outputs,
        frames,
        0.5,
    );
    let time_l_in = input_provider.control_input_or_default(
        module_idx,
        builtin_ports::TIME_LEFT_MS,
        all_outputs,
        frames,
        0.3,
    );
    let time_r_in = input_provider.control_input_or_default(
        module_idx,
        builtin_ports::TIME_RIGHT_MS,
        all_outputs,
        frames,
        0.3,
    );
    let ping_pong_in = input_provider.control_input_or_default(
        module_idx,
        builtin_ports::PING_PONG,
        all_outputs,
        frames,
        0.0,
    );

    let mut out_l = Vec::with_capacity(frames);
    let mut out_r = Vec::with_capacity(frames);

    for i in 0..frames {
        let feedback = feedback_in.get(i).copied().unwrap_or(0.5);
        let damping_norm = damping_in.get(i).copied().unwrap_or(0.5);
        let damping_hz = 20.0 * 1000.0_f32.powf(damping_norm);
        let wet = wet_in.get(i).copied().unwrap_or(0.7);
        let dry = dry_in.get(i).copied().unwrap_or(0.5);
        let time_l = lerp(1.0, 2000.0, time_l_in.get(i).copied().unwrap_or(0.5));
        let time_r = lerp(1.0, 2000.0, time_r_in.get(i).copied().unwrap_or(0.5));
        let ping_pong = ping_pong_in.get(i).copied().unwrap_or(0.0) > 0.5;

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
    module_idx: usize,
    input_provider: &impl ModuleInputProvider,
    all_outputs: &HashMap<usize, ModuleOutputs>,
    frames: usize,
) -> ModuleOutputs {
    let (processor, _sample_rate) = match state {
        PerModuleState::Reverb {
            processor,
            sample_rate,
        } => (processor, *sample_rate),
        _ => unreachable!(),
    };

    let decay_time_in = input_provider.control_input_or_default(
        module_idx,
        builtin_ports::DECAY_TIME,
        all_outputs,
        frames,
        0.35,
    );
    let room_size_in = input_provider.control_input_or_default(
        module_idx,
        builtin_ports::ROOM_SIZE,
        all_outputs,
        frames,
        0.7,
    );
    let damping_in = input_provider.control_input_or_default(
        module_idx,
        builtin_ports::DAMPING,
        all_outputs,
        frames,
        0.3,
    );
    let diffusion_in = input_provider.control_input_or_default(
        module_idx,
        builtin_ports::DIFFUSION,
        all_outputs,
        frames,
        0.5,
    );
    let wet_in = input_provider.control_input_or_default(
        module_idx,
        builtin_ports::WET,
        all_outputs,
        frames,
        0.7,
    );
    let dry_in = input_provider.control_input_or_default(
        module_idx,
        builtin_ports::DRY,
        all_outputs,
        frames,
        0.5,
    );
    let pre_delay_in = input_provider.control_input_or_default(
        module_idx,
        builtin_ports::PRE_DELAY,
        all_outputs,
        frames,
        0.0,
    );
    let stereo_width_in = input_provider.control_input_or_default(
        module_idx,
        builtin_ports::STEREO_WIDTH,
        all_outputs,
        frames,
        0.5,
    );

    let mut out_l = Vec::with_capacity(frames);
    let mut out_r = Vec::with_capacity(frames);

    for i in 0..frames {
        let decay_sec = lerp(0.1, 10.0, decay_time_in.get(i).copied().unwrap_or(0.5));
        let room_size = room_size_in.get(i).copied().unwrap_or(0.5);
        let damping_norm = damping_in.get(i).copied().unwrap_or(0.5);
        let damping_hz = 20.0 * 1000.0_f32.powf(damping_norm);
        let diffusion = diffusion_in.get(i).copied().unwrap_or(0.5);
        let wet = wet_in.get(i).copied().unwrap_or(0.7);
        let dry = dry_in.get(i).copied().unwrap_or(0.5);
        let pre_delay_ms = lerp(0.0, 250.0, pre_delay_in.get(i).copied().unwrap_or(0.0));
        let stereo_width = stereo_width_in.get(i).copied().unwrap_or(0.5);

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
