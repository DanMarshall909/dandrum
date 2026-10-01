use super::event_queue::BoundedEventQueue;
use super::helpers::{normalized_end_position, normalized_position};
use super::outputs::BlockEvent;
use super::process_context::ProcessContext;
use super::state::PerModuleState;
use crate::compiled_patch::{SampleInterpolation, SampleSelectionMode};
use crate::decay::DecayCurve;
use crate::kernel::document::SampleRegion;
use crate::oscillator::OSCILLATOR_BASE_HZ;
use crate::saturator::Saturator;
use crate::script::ScriptEvent;

const STEREO_CHANNELS: usize = 2;

pub(super) fn process_audio_mixer(context: &mut ProcessContext<'_>) {
    for channel in 0..context.output_count() {
        context
            .write_output_from_input(channel, channel, |sample| sample)
            .expect("audio mixer channel buffers should be available in supported arena step");
    }
}

pub(super) fn process_noise(state: &mut PerModuleState, context: &mut ProcessContext<'_>) {
    let rng_states = match state {
        PerModuleState::Noise { states, .. } => states,
        _ => unreachable!(),
    };
    process_noise_states(rng_states, context);
}

pub(super) fn process_noise_states(rng_states: &mut [u32], context: &mut ProcessContext<'_>) {
    for (channel, rng_state) in rng_states.iter_mut().enumerate() {
        for frame in 0..context.frames() {
            let mut x = *rng_state;
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            let sample = (x as f32) / (u32::MAX as f32) * 2.0 - 1.0;
            context
                .set_output_sample(channel, frame, sample)
                .expect("noise output channel should be available in supported arena step");
            *rng_state = x;
        }
    }
}

pub(super) fn process_sampler(
    state: &mut PerModuleState,
    context: &mut ProcessContext<'_>,
    events: &[BlockEvent],
) {
    let PerModuleState::Sampler {
        sample,
        position,
        active,
    } = state
    else {
        unreachable!()
    };
    process_sampler_state(sample, position, active, context, events);
}

pub(super) fn process_sampler_state(
    sample: &Option<crate::compiled_patch::SampleResourceHandle>,
    position: &mut f32,
    active: &mut bool,
    context: &mut ProcessContext<'_>,
    events: &[BlockEvent],
) {
    let frames = sample.as_ref().map_or(&[][..], |sample| sample.frames());
    if frames.is_empty() {
        for frame in 0..context.frames() {
            for channel in 0..context.output_count() {
                context
                    .set_output_sample(channel, frame, 0.0)
                    .expect("sampler output channel should be available in supported arena step");
            }
        }
        return;
    }

    for frame in 0..context.frames() {
        for event in events
            .iter()
            .filter(|event| event.frame_offset as usize == frame)
        {
            if matches!(event.event, ScriptEvent::NoteOn { .. }) {
                *position = normalized_position(context.input_sample(1, frame, 0.0), frames.len());
                *active = true;
            }
        }

        let mut output = 0.0;
        if *active {
            let index = *position as usize;
            if index >= frames.len() {
                *active = false;
            } else {
                output = frames[index];
                let rate = context.input_sample(0, frame, 1.0).max(0.0);
                *position += rate;
                if context.input_sample(2, frame, 0.0) > 0.5 {
                    let loop_start =
                        normalized_position(context.input_sample(3, frame, 0.0), frames.len());
                    let mut loop_end =
                        normalized_end_position(context.input_sample(4, frame, 1.0), frames.len());
                    if loop_end <= loop_start {
                        loop_end = frames.len() as f32;
                    }
                    while *position >= loop_end {
                        *position = loop_start + (*position - loop_end);
                    }
                } else if *position >= frames.len() as f32 {
                    *active = false;
                }
            }
        }
        for channel in 0..context.output_count() {
            context
                .set_output_sample(channel, frame, output)
                .expect("sampler output channel should be available in supported arena step");
        }
    }
}

pub(super) fn process_sample_player_state(
    sample: &Option<crate::compiled_patch::SampleResourceHandle>,
    region: &SampleRegion,
    mode: &str,
    interpolation: SampleInterpolation,
    position: &mut f64,
    active: &mut bool,
    context: &mut ProcessContext<'_>,
    events: &[BlockEvent],
    gate_events: &[BlockEvent],
) {
    let frames = sample.as_ref().map_or(&[][..], |sample| sample.frames());
    let sample_rate = sample
        .as_ref()
        .map_or(0.0, |sample| f64::from(sample.sample_rate_hz()));
    let fade_in_frames = (region.fade_in_ms.unwrap_or(0.0) * sample_rate / 1000.0).round() as usize;
    let fade_out_frames =
        (region.fade_out_ms.unwrap_or(0.0) * sample_rate / 1000.0).round() as usize;
    let loop_settings = if mode == "looped" {
        region.loop_settings.as_ref()
    } else {
        None
    };
    let crossfade_frames = loop_settings.map_or(0, |loop_settings| {
        let rate = sample.as_ref().map_or(0, |sample| sample.sample_rate_hz());
        let requested = loop_settings.crossfade_ms.unwrap_or(0.0) * f64::from(rate) / 1000.0;
        (requested.round() as usize)
            .min(((loop_settings.end_frame - loop_settings.start_frame) / 2) as usize)
    });
    for frame in 0..context.frames() {
        if events.iter().any(|event| {
            event.frame_offset as usize == frame
                && matches!(event.event, ScriptEvent::NoteOn { .. })
        }) {
            *position = if region.reverse {
                (region.end_frame - 1) as f64
            } else {
                region.start_frame as f64
            };
            *active = true;
        }
        if mode != "one_shot"
            && gate_events.iter().any(|event| {
                event.frame_offset as usize == frame
                    && matches!(event.event, ScriptEvent::NoteOff { .. })
            })
        {
            *active = false;
        }
        let output = if *active {
            let index = *position as usize;
            if *position >= region.start_frame as f64
                && *position < region.end_frame as f64
                && index < frames.len()
            {
                let mut value = if let Some(loop_settings) = loop_settings {
                    let fade_start = loop_settings.end_frame as usize - crossfade_frames;
                    let in_crossfade = crossfade_frames > 0
                        && if region.reverse {
                            *position < (loop_settings.start_frame + crossfade_frames as u64) as f64
                        } else {
                            index >= fade_start
                        };
                    if in_crossfade {
                        let fade_position = if region.reverse {
                            (loop_settings.start_frame + crossfade_frames as u64 - 1) as f64
                                - *position
                        } else {
                            *position - fade_start as f64
                        };
                        let head_position = if region.reverse {
                            (loop_settings.end_frame - 1) as f64 - fade_position
                        } else {
                            loop_settings.start_frame as f64 + fade_position
                        };
                        let head =
                            sample_interpolated(frames, head_position, region, interpolation);
                        let tail = sample_interpolated(frames, *position, region, interpolation);
                        let weight =
                            ((fade_position + 1.0) / crossfade_frames as f64).min(1.0) as f32;
                        tail * (1.0 - weight) + head * weight
                    } else {
                        sample_interpolated(frames, *position, region, interpolation)
                    }
                } else {
                    sample_interpolated(frames, *position, region, interpolation)
                };
                let (from_start, to_end) = if region.reverse {
                    (
                        (region.end_frame - 1) as f64 - *position,
                        *position - region.start_frame as f64,
                    )
                } else {
                    (
                        *position - region.start_frame as f64,
                        (region.end_frame - 1) as f64 - *position,
                    )
                };
                if fade_in_frames > 0 {
                    let span = fade_in_frames.saturating_sub(1) as f64;
                    value *= if span > 0.0 {
                        (from_start / span).clamp(0.0, 1.0) as f32
                    } else {
                        0.0
                    };
                }
                if fade_out_frames > 0 {
                    let span = fade_out_frames.saturating_sub(1) as f64;
                    value *= if span > 0.0 {
                        (to_end / span).clamp(0.0, 1.0) as f32
                    } else {
                        0.0
                    };
                }
                let ratio = context.input_sample(0, frame, 1.0);
                let ratio = if ratio.is_finite() {
                    ratio.clamp(0.125, 8.0)
                } else {
                    1.0
                };
                *position += if region.reverse {
                    -f64::from(ratio)
                } else {
                    f64::from(ratio)
                };
                if let Some(loop_settings) = loop_settings {
                    if region.reverse && *position < loop_settings.start_frame as f64 {
                        let wrap_length = (loop_settings.end_frame
                            - loop_settings.start_frame
                            - crossfade_frames as u64)
                            as f64;
                        *position = loop_settings.start_frame as f64
                            + (*position - loop_settings.start_frame as f64)
                                .rem_euclid(wrap_length);
                    } else if !region.reverse && *position >= loop_settings.end_frame as f64 {
                        let wrap_start =
                            (loop_settings.start_frame + crossfade_frames as u64) as f64;
                        let wrap_length = loop_settings.end_frame as f64 - wrap_start;
                        *position = wrap_start
                            + (*position - loop_settings.end_frame as f64).rem_euclid(wrap_length);
                    }
                } else if (region.reverse && *position < region.start_frame as f64)
                    || (!region.reverse && *position >= region.end_frame as f64)
                {
                    *active = false;
                }
                value
            } else {
                *active = false;
                0.0
            }
        } else {
            0.0
        };
        for channel in 0..context.output_count() {
            context
                .set_output_sample(channel, frame, output)
                .expect("sample player output channel is prepared");
        }
    }
}

fn sample_interpolated(
    frames: &[f32],
    position: f64,
    region: &SampleRegion,
    interpolation: SampleInterpolation,
) -> f32 {
    let first = region.start_frame as usize;
    let last = region.end_frame as usize - 1;
    if interpolation == SampleInterpolation::Nearest {
        return frames[(position.round() as usize).clamp(first, last)];
    }
    let base = (position.floor() as usize).clamp(first, last);
    let fraction = (position - base as f64) as f32;
    let a = frames[base];
    let b = frames[(base + 1).min(last)];
    if interpolation == SampleInterpolation::Linear {
        return a + fraction * (b - a);
    }
    let before = frames[base.saturating_sub(1).max(first)];
    let after = frames[(base + 2).min(last)];
    let f2 = fraction * fraction;
    let f3 = f2 * fraction;
    0.5 * ((3.0 * (a - b) + after - before) * f3
        + (2.0 * before + 4.0 * b - 5.0 * a - after) * f2
        + (b - before) * fraction
        + 2.0 * a)
}

pub(super) fn process_sample_map_player_state(
    zones: &[crate::compiled_patch::CompiledSampleZone],
    selection_mode: SampleSelectionMode,
    round_robin_counters: &mut [usize],
    selected_zone: &mut Option<usize>,
    position: &mut f64,
    active: &mut bool,
    context: &mut ProcessContext<'_>,
    events: &[BlockEvent],
) {
    for frame in 0..context.frames() {
        for event in events {
            if event.frame_offset as usize != frame {
                continue;
            }
            if let ScriptEvent::NoteOn { note, velocity } = event.event {
                let matches = |zone: &crate::compiled_patch::CompiledSampleZone| {
                    note >= zone.key_range[0]
                        && note <= zone.key_range[1]
                        && velocity >= zone.velocity_range[0]
                        && velocity <= zone.velocity_range[1]
                };
                let first = zones.iter().position(matches);
                let chosen = first.map(|first| {
                    if selection_mode == SampleSelectionMode::RoundRobin {
                        if let Some(group) = zones[first].round_robin_group {
                            let count = zones
                                .iter()
                                .filter(|zone| {
                                    matches(zone) && zone.round_robin_group == Some(group)
                                })
                                .count();
                            let turn = round_robin_counters[group] % count;
                            round_robin_counters[group] =
                                round_robin_counters[group].wrapping_add(1);
                            return zones
                                .iter()
                                .enumerate()
                                .filter(|(_, zone)| {
                                    matches(zone) && zone.round_robin_group == Some(group)
                                })
                                .nth(turn)
                                .expect("matching round-robin group has a selected zone")
                                .0;
                        }
                    }
                    first
                });
                if let Some(index) = chosen {
                    *selected_zone = Some(index);
                    let region = &zones[index].region;
                    *position = if region.reverse {
                        (region.end_frame - 1) as f64
                    } else {
                        region.start_frame as f64
                    };
                    *active = true;
                }
            }
        }
        let output = if *active {
            let zone = &zones[selected_zone.expect("active map player has a selected zone")];
            let region = &zone.region;
            if *position >= region.start_frame as f64 && *position < region.end_frame as f64 {
                let sample = sample_interpolated(
                    zone.sample.frames(),
                    *position,
                    region,
                    SampleInterpolation::Linear,
                );
                let ratio = context.input_sample(0, frame, 1.0);
                let ratio = if ratio.is_finite() {
                    ratio.clamp(0.125, 8.0)
                } else {
                    1.0
                };
                *position += if region.reverse {
                    -f64::from(ratio)
                } else {
                    f64::from(ratio)
                };
                if *position < region.start_frame as f64 || *position >= region.end_frame as f64 {
                    *active = false;
                }
                sample
            } else {
                *active = false;
                0.0
            }
        } else {
            0.0
        };
        for channel in 0..context.output_count() {
            context
                .set_output_sample(channel, frame, output)
                .expect("sample map output channel is prepared");
        }
    }
}

pub(super) fn process_sample_map_player(
    state: &mut PerModuleState,
    context: &mut ProcessContext<'_>,
    events: &[BlockEvent],
) {
    let PerModuleState::SampleMapPlayer {
        zones,
        selection_mode,
        round_robin_counters,
        selected_zone,
        position,
        active,
    } = state
    else {
        unreachable!()
    };
    process_sample_map_player_state(
        zones,
        *selection_mode,
        round_robin_counters,
        selected_zone,
        position,
        active,
        context,
        events,
    );
}

pub(super) fn process_sample_player(
    state: &mut PerModuleState,
    context: &mut ProcessContext<'_>,
    events: &[BlockEvent],
    gate_events: &[BlockEvent],
) {
    let PerModuleState::SamplePlayer {
        sample,
        region,
        mode,
        interpolation,
        position,
        active,
    } = state
    else {
        unreachable!()
    };
    process_sample_player_state(
        sample,
        region,
        mode,
        *interpolation,
        position,
        active,
        context,
        events,
        gate_events,
    );
}

pub(super) fn process_note_to_rate(
    state: &mut PerModuleState,
    context: &mut ProcessContext<'_>,
    events: &[BlockEvent],
) {
    let PerModuleState::NoteToRate { rate } = state else {
        unreachable!()
    };
    process_note_to_rate_state(rate, context, events);
}

pub(super) fn process_note_to_rate_state(
    rate: &mut f32,
    context: &mut ProcessContext<'_>,
    events: &[BlockEvent],
) {
    for frame in 0..context.frames() {
        for event in events {
            if event.frame_offset as usize == frame {
                if let ScriptEvent::NoteOn { note, .. } = &event.event {
                    *rate = 2.0f32.powf((f32::from(*note) - 60.0) / 12.0);
                }
            }
        }
        context
            .set_output_sample(0, frame, *rate)
            .expect("note-to-rate output is present in a supported arena step");
    }
}

pub(super) fn process_note_to_control(
    state: &mut PerModuleState,
    context: &mut ProcessContext<'_>,
    events: &[BlockEvent],
    gate_output: &mut BoundedEventQueue,
) {
    let PerModuleState::NoteToControl {
        gate_active,
        current_note,
        current_velocity,
        current_frequency,
        current_pitch_ratio,
        current_slide,
    } = state
    else {
        unreachable!()
    };
    process_note_to_control_state(
        gate_active,
        current_note,
        current_velocity,
        current_frequency,
        current_pitch_ratio,
        current_slide,
        context,
        events,
        gate_output,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn process_note_to_control_state(
    gate_active: &mut bool,
    current_note: &mut Option<u8>,
    current_velocity: &mut f32,
    current_frequency: &mut f32,
    current_pitch_ratio: &mut f32,
    current_slide: &mut bool,
    context: &mut ProcessContext<'_>,
    events: &[BlockEvent],
    gate_output: &mut BoundedEventQueue,
) {
    for frame in 0..context.frames() {
        for event in events {
            if event.frame_offset as usize != frame {
                continue;
            }
            match &event.event {
                ScriptEvent::NoteOn { note, velocity } => {
                    let frequency = super::processing::midi_note_to_freq(*note);
                    *current_slide = *gate_active;
                    if !*gate_active {
                        *gate_active = true;
                        let _ = gate_output.push_at(event.event.clone(), event.frame_offset);
                    }
                    *current_note = Some(*note);
                    *current_velocity = f32::from(*velocity) / 127.0;
                    *current_frequency = frequency;
                    *current_pitch_ratio = frequency / OSCILLATOR_BASE_HZ;
                }
                ScriptEvent::NoteOff { note } if *current_note == Some(*note) => {
                    *gate_active = false;
                    *current_note = None;
                    *current_velocity = 0.0;
                    *current_slide = false;
                    let _ = gate_output.push_at(event.event.clone(), event.frame_offset);
                }
                ScriptEvent::NoteOff { .. } => {}
            }
        }
        for (channel, value) in [
            *current_frequency,
            *current_pitch_ratio,
            *current_velocity,
            if *current_slide { 1.0 } else { 0.0 },
        ]
        .into_iter()
        .enumerate()
        {
            context
                .set_output_sample(channel, frame, if *gate_active { value } else { 0.0 })
                .expect("note_to_control output buffer is present in a supported arena step");
        }
    }
}

pub(super) fn process_impulse(context: &mut ProcessContext<'_>, events: &[BlockEvent]) {
    for frame in 0..context.frames() {
        let triggered = events
            .iter()
            .any(|event| event.frame_offset as usize == frame);
        for channel in 0..context.output_count() {
            context
                .set_output_sample(channel, frame, if triggered { 1.0 } else { 0.0 })
                .expect("impulse output is present in a supported arena step");
        }
    }
}

pub(super) fn process_decay(
    state: &mut PerModuleState,
    context: &mut ProcessContext<'_>,
    events: &[BlockEvent],
) {
    let PerModuleState::Decay {
        level,
        triggered,
        elapsed_frames,
        sample_rate,
        curve,
    } = state
    else {
        unreachable!()
    };
    process_decay_state(
        level,
        triggered,
        elapsed_frames,
        *sample_rate,
        *curve,
        context,
        events,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn process_decay_state(
    level: &mut f32,
    triggered: &mut bool,
    elapsed_frames: &mut u64,
    sample_rate: f32,
    curve: DecayCurve,
    context: &mut ProcessContext<'_>,
    events: &[BlockEvent],
) {
    for frame in 0..context.frames() {
        for event in events {
            if event.frame_offset as usize == frame
                && matches!(event.event, ScriptEvent::NoteOn { .. })
            {
                *level = 1.0;
                *triggered = true;
                *elapsed_frames = 0;
            }
        }
        if *triggered {
            let time_ms = context.input_sample(0, frame, 100.0);
            let decay_frames = (sample_rate * time_ms / 1000.0).max(1.0);
            let t = *elapsed_frames as f32 / decay_frames;
            *level = match &curve {
                DecayCurve::Linear => (1.0 - t).max(0.0),
                DecayCurve::Exponential => (-4.0 * t).exp(),
            };
            *elapsed_frames += 1;
            if *level <= 0.0 {
                *level = 0.0;
                *triggered = false;
            }
        }
        context
            .set_output_sample(0, frame, *level)
            .expect("decay output is present in a supported arena step");
    }
}

pub(super) fn process_spectral_processor(
    state: &mut PerModuleState,
    context: &mut ProcessContext<'_>,
) {
    let PerModuleState::SpectralProcessor { processor } = state else {
        unreachable!()
    };
    process_spectral_processor_state(processor, context);
}

pub(super) fn process_spectral_processor_state(
    processor: &mut crate::spectral::SpectralProcessor,
    context: &mut ProcessContext<'_>,
) {
    for frame in 0..context.frames() {
        let audio = context.input_sample(0, frame, 0.0);
        let threshold_db = context.input_sample(1, frame, 0.0) as f64 * 80.0 - 40.0;
        let mix = context.input_sample(2, frame, 1.0);
        processor.set_threshold(threshold_db);
        let processed = processor.process(audio);
        context
            .set_output_sample(0, frame, processed * mix + audio * (1.0 - mix))
            .expect("spectral output is present in a supported arena step");
    }
}

#[cfg(test)]
mod sampler_tests {
    use super::super::audio_arena::AudioArena;
    use super::super::render_plan::{AudioBufferPlan, BufferId};
    use super::*;

    #[test]
    fn sampler_without_decoded_frames_clears_its_output_span() {
        let mut arena = AudioArena::new(AudioBufferPlan {
            buffer_count: 6,
            max_block_frames: 4,
            max_voices: 1,
        });
        arena.fill(BufferId(5), 4, 0.75);
        let inputs = [
            BufferId(0),
            BufferId(1),
            BufferId(2),
            BufferId(3),
            BufferId(4),
        ];
        let outputs = [BufferId(5)];
        let mut context = ProcessContext::new(&mut arena, &inputs, &outputs, 4);
        let mut state = PerModuleState::Sampler {
            sample: None,
            position: 0.0,
            active: false,
        };

        process_sampler(&mut state, &mut context, &[]);

        assert_eq!(arena.sample(BufferId(5), 0), 0.0);
        assert_eq!(arena.sample(BufferId(5), 3), 0.0);
    }
}

pub(super) fn process_oscillator(state: &mut PerModuleState, context: &mut ProcessContext<'_>) {
    let (phase, sample_rate, waveform) = match state {
        PerModuleState::Oscillator {
            phase,
            sample_rate,
            waveform,
        } => (phase, *sample_rate, *waveform),
        _ => unreachable!(),
    };
    process_oscillator_state(phase, sample_rate, waveform, context);
}

pub(super) fn process_oscillator_state(
    phase: &mut f32,
    sample_rate: f32,
    waveform: crate::oscillator::Waveform,
    context: &mut ProcessContext<'_>,
) {
    for frame in 0..context.frames() {
        let pitch_ratio = context.input_sample(0, frame, 1.0);
        let output = waveform.sample(*phase);
        let freq = OSCILLATOR_BASE_HZ * pitch_ratio;
        let phase_inc = freq / sample_rate;
        *phase += phase_inc;
        if *phase >= 1.0 {
            *phase -= 1.0;
        }
        for channel in 0..context.output_count() {
            context
                .set_output_sample(channel, frame, output)
                .expect("oscillator output buffer should be available in supported arena step");
        }
    }
}

pub(super) fn process_lfo(state: &mut PerModuleState, context: &mut ProcessContext<'_>) {
    let PerModuleState::Lfo { phase, sample_rate } = state else {
        unreachable!()
    };
    process_lfo_state(phase, *sample_rate, context);
}

pub(super) fn process_lfo_state(
    phase: &mut f32,
    sample_rate: f32,
    context: &mut ProcessContext<'_>,
) {
    for frame in 0..context.frames() {
        let rate = context.input_sample(0, frame, 1.0).max(0.0);
        let value = 0.5 + 0.5 * (*phase * std::f32::consts::TAU).sin();
        context
            .set_output_sample(0, frame, value)
            .expect("LFO output is present in a supported arena step");
        *phase = (*phase + rate / sample_rate).rem_euclid(1.0);
    }
}

pub(super) fn process_slew(state: &mut PerModuleState, context: &mut ProcessContext<'_>) {
    let PerModuleState::Slew {
        current,
        sample_rate,
    } = state
    else {
        unreachable!()
    };
    process_slew_state(current, *sample_rate, context);
}

pub(super) fn process_slew_state(
    current: &mut f32,
    sample_rate: f32,
    context: &mut ProcessContext<'_>,
) {
    for frame in 0..context.frames() {
        let target = context.input_sample(0, frame, 0.0);
        let glide = context.input_sample(1, frame, 0.0);
        let time_ms = context.input_sample(2, frame, 60.0);
        let value = super::processing::slew_step(current, sample_rate, target, glide, time_ms);
        context
            .set_output_sample(0, frame, value)
            .expect("slew output is present in a supported arena step");
    }
}

pub(super) fn process_dynamics(state: &mut PerModuleState, context: &mut ProcessContext<'_>) {
    let PerModuleState::DynamicsProcessor { processors, .. } = state else {
        unreachable!()
    };
    process_dynamics_processors(processors, context);
}

pub(super) fn process_dynamics_processors(
    processors: &mut [crate::dynamics_processor::DynamicsProcessor],
    context: &mut ProcessContext<'_>,
) {
    let channels = context.output_count();
    for frame in 0..context.frames() {
        let sidechain = context.input_sample(channels, frame, 0.0);
        let controls = [
            context.input_sample(channels + 1, frame, 0.3),
            context.input_sample(channels + 2, frame, 0.05),
            context.input_sample(channels + 3, frame, 0.077),
            context.input_sample(channels + 4, frame, 0.05),
            context.input_sample(channels + 5, frame, 0.1),
            context.input_sample(channels + 6, frame, 0.0),
            context.input_sample(channels + 7, frame, 0.0),
            context.input_sample(channels + 8, frame, 0.5),
            context.input_sample(channels + 9, frame, 0.5),
        ];
        for (channel, processor) in processors.iter_mut().enumerate() {
            let audio = context.input_sample(channel, frame, 0.0);
            let value = super::processing::dynamics_sample(processor, audio, sidechain, controls);
            context
                .set_output_sample(channel, frame, value)
                .expect("dynamics output is present in a supported arena step");
        }
    }
}

pub(super) fn process_gain(context: &mut ProcessContext<'_>) {
    let channels = context.output_count();
    let gain_is_multichannel = context.input_count() >= channels * 2;
    for channel in 0..channels {
        let gain_input = if gain_is_multichannel {
            channels + channel
        } else {
            channels
        };
        context
            .write_output_from_two_inputs(channel, channel, gain_input, |audio, gain| audio * gain)
            .expect("gain channel buffers should be available in supported arena step");
    }
}

pub(super) fn process_adsr(
    state: &mut PerModuleState,
    context: &mut ProcessContext<'_>,
    events: &[BlockEvent],
    block_start_frame: u64,
) {
    let PerModuleState::Adsr {
        level,
        gate_active,
        release_start_frame,
        release_start_level,
        sample_rate,
    } = state
    else {
        unreachable!()
    };
    process_adsr_state(
        level,
        gate_active,
        release_start_frame,
        release_start_level,
        *sample_rate,
        context,
        events,
        block_start_frame,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn process_adsr_state(
    level: &mut f32,
    gate_active: &mut bool,
    release_start_frame: &mut u64,
    release_start_level: &mut f32,
    sample_rate: f32,
    context: &mut ProcessContext<'_>,
    events: &[BlockEvent],
    block_start_frame: u64,
) {
    let mut final_level = *level;

    for frame in 0..context.frames() {
        let absolute_frame = block_start_frame + frame as u64;
        for event in events {
            if event.frame_offset as usize == frame {
                match &event.event {
                    ScriptEvent::NoteOn { .. } => {
                        *gate_active = true;
                        *release_start_frame = absolute_frame;
                    }
                    ScriptEvent::NoteOff { .. } => {
                        *gate_active = false;
                        *release_start_frame = absolute_frame;
                        *release_start_level = final_level;
                    }
                }
            }
        }

        let attack_ms =
            super::processing::adsr_time_ms(context.input_sample(0, frame, 5.0), 2.0, 100.0);
        let decay_ms =
            super::processing::adsr_time_ms(context.input_sample(1, frame, 30.0), 10.0, 1000.0);
        let sustain = context.input_sample(2, frame, 0.7).clamp(0.0, 1.0);
        let release_ms =
            super::processing::adsr_time_ms(context.input_sample(3, frame, 200.0), 10.0, 3000.0);
        let attack_frames = (sample_rate * attack_ms / 1000.0) as u64;
        let decay_frames = (sample_rate * decay_ms / 1000.0) as u64;
        let release_frames = (sample_rate * release_ms / 1000.0) as u64;

        final_level = if *gate_active {
            let lifetime = absolute_frame - *release_start_frame;
            if lifetime < attack_frames {
                lifetime as f32 / attack_frames as f32
            } else if lifetime < attack_frames + decay_frames {
                let progress = (lifetime - attack_frames) as f32 / decay_frames as f32;
                1.0 - (1.0 - sustain) * progress
            } else {
                sustain
            }
        } else {
            let progress = (absolute_frame - *release_start_frame) as f32 / release_frames as f32;
            if progress >= 1.0 {
                0.0
            } else {
                *release_start_level * (1.0 - progress)
            }
        };
        context
            .set_output_sample(0, frame, final_level)
            .expect("ADSR output buffer should be available in supported arena step");
    }
    *level = final_level;
}

pub(super) fn process_envelope_follower(
    state: &mut PerModuleState,
    context: &mut ProcessContext<'_>,
) {
    let (detector, mode) = match state {
        PerModuleState::EnvelopeFollower { detector, mode } => (detector, *mode),
        _ => unreachable!(),
    };
    process_envelope_follower_state(detector, mode, context);
}

pub(super) fn process_envelope_follower_state(
    detector: &mut crate::envelope_follower::EnvelopeFollower,
    mode: crate::envelope_follower::DetectionMode,
    context: &mut ProcessContext<'_>,
) {
    detector.set_mode(mode);
    for frame in 0..context.frames() {
        let attack_ms = context.input_sample(1, frame, 5.0).max(0.0) as f64;
        let release_ms = context.input_sample(2, frame, 50.0).max(0.0) as f64;
        detector.set_params(attack_ms, release_ms);

        let envelope = detector.process(context.input_sample(0, frame, 0.0) as f64) as f32;
        let shaped = if context.input_sample(5, frame, 0.0) > 0.5 {
            1.0 - envelope
        } else {
            envelope
        };
        let amount = context.input_sample(3, frame, 1.0);
        let offset = context.input_sample(4, frame, 0.0);

        context
            .set_output_sample(
                0,
                frame,
                finite_or_zero(shaped * amount + offset).clamp(0.0, 1.0),
            )
            .expect("envelope follower output buffer should be available in supported arena step");
    }
}

pub(super) fn process_curve_mapper(state: &mut PerModuleState, context: &mut ProcessContext<'_>) {
    let mapper = match state {
        PerModuleState::CurveMapper { mapper } => mapper,
        _ => unreachable!(),
    };
    process_curve_mapper_state(mapper, context);
}

pub(super) fn process_curve_mapper_state(
    mapper: &mut crate::curve_mapper::CurveMapper,
    context: &mut ProcessContext<'_>,
) {
    for frame in 0..context.frames() {
        let output = mapper.process(
            context.input_sample(0, frame, 0.0),
            context.input_sample(1, frame, 1.0),
            context.input_sample(2, frame, 0.0),
            context.input_sample(3, frame, 1.0),
            context.input_sample(4, frame, 0.0),
        );

        context
            .set_output_sample(0, frame, output)
            .expect("curve mapper output buffer should be available in supported arena step");
    }
}

pub(super) fn process_filter(state: &mut PerModuleState, context: &mut ProcessContext<'_>) {
    let (filters, sample_rate) = match state {
        PerModuleState::Filter {
            filters,
            sample_rate,
        } => (filters, *sample_rate),
        _ => unreachable!(),
    };
    process_filter_states(filters, sample_rate, context);
}

pub(super) fn process_filter_states(
    filters: &mut [Box<dyn crate::filter::FilterAlgorithm>],
    sample_rate: f64,
    context: &mut ProcessContext<'_>,
) {
    let channels = filters.len();
    for (channel, filter) in filters.iter_mut().enumerate() {
        for frame in 0..context.frames() {
            filter.set_cutoff_control(context.input_sample(channels, frame, 0.5), sample_rate);
            filter.set_resonance_control(context.input_sample(channels + 1, frame, 0.0));
            filter.set_gain_db(context.input_sample(channels + 2, frame, 0.5) as f64 * 48.0 - 24.0);

            let output = filter.process(context.input_sample(channel, frame, 0.0));
            context
                .set_output_sample(channel, frame, output)
                .expect("filter output channel should be available in supported arena step");
        }
    }
}

pub(super) fn process_saturator(context: &mut ProcessContext<'_>) {
    let channels = context.output_count();
    for channel in 0..channels {
        for frame in 0..context.frames() {
            let drive_db = lerp(0.0, 48.0, context.input_sample(channels, frame, 0.0));
            let bias = lerp(-1.0, 1.0, context.input_sample(channels + 1, frame, 0.0));
            let curve_index = (context.input_sample(channels + 2, frame, 0.0) * 4.0)
                .round()
                .clamp(0.0, 4.0) as usize;
            let sample = Saturator::process_builtin(
                context.input_sample(channel, frame, 0.0) as f64,
                drive_db as f64,
                bias as f64,
                curve_index,
            ) as f32;
            context
                .set_output_sample(channel, frame, sample)
                .expect("saturator output channel should be available");
        }
    }
}

pub(super) fn process_control_to_audio(context: &mut ProcessContext<'_>) {
    for channel in 0..context.output_count() {
        context
            .write_output_from_input(
                channel,
                channel.min(context.input_count().saturating_sub(1)),
                |sample| sample,
            )
            .expect("promotion channel buffers should be available");
    }
}

pub(super) fn process_compensation_delay(
    state: &mut PerModuleState,
    context: &mut ProcessContext<'_>,
) {
    let PerModuleState::CompensationDelay { samples, positions } = state else {
        unreachable!()
    };
    process_compensation_delay_states(samples, positions, context);
}

pub(super) fn process_compensation_delay_states(
    samples: &mut [Box<[f32]>],
    positions: &mut [usize],
    context: &mut ProcessContext<'_>,
) {
    for channel in 0..context.output_count() {
        for frame in 0..context.frames() {
            let position = positions[channel];
            let output = samples[channel][position];
            samples[channel][position] = context.input_sample(channel, frame, 0.0);
            positions[channel] = (position + 1) % samples[channel].len();
            context.set_output_sample(channel, frame, output).unwrap();
        }
    }
}

/// Emit the prior samples before the forward schedule evaluates the feedback
/// tap. `delay_samples` is at least the prepared block size, so the tap can be
/// captured after the schedule without changing any sample emitted here.
pub(super) fn emit_feedback_delay(state: &PerModuleState, context: &mut ProcessContext<'_>) {
    let PerModuleState::FeedbackDelay { samples, position } = state else {
        unreachable!()
    };
    emit_feedback_delay_parts(samples, *position, context);
}

pub(super) fn emit_feedback_delay_parts(
    samples: &[Box<[f32]>],
    position: usize,
    context: &mut ProcessContext<'_>,
) {
    for (channel, ring) in samples.iter().enumerate() {
        for frame in 0..context.frames() {
            context
                .set_output_sample(channel, frame, ring[(position + frame) % ring.len()])
                .expect("feedback output channel should be available");
        }
    }
}

/// Capture the just-computed feedback tap after all forward nodes have run.
pub(super) fn capture_feedback_delay_parts(
    samples: &mut [Box<[f32]>],
    position: &mut usize,
    context: &ProcessContext<'_>,
) {
    let start = *position;
    for (channel, ring) in samples.iter_mut().enumerate() {
        for frame in 0..context.frames() {
            ring[(start + frame) % ring.len()] = context.input_sample(channel, frame, 0.0);
        }
    }
    *position = (start + context.frames()) % samples[0].len();
}

pub(super) fn process_convolution(state: &mut PerModuleState, context: &mut ProcessContext<'_>) {
    let PerModuleState::Convolution { processors } = state else {
        unreachable!()
    };
    process_convolution_processors(processors, context);
}

pub(super) fn process_convolution_processors(
    processors: &mut [crate::convolution::Convolution],
    context: &mut ProcessContext<'_>,
) {
    let channels = processors.len();
    for (channel, processor) in processors.iter_mut().enumerate() {
        for frame in 0..context.frames() {
            processor.set_wet(context.input_sample(channels, frame, 1.0).clamp(0.0, 1.0));
            let output = processor.process(context.input_sample(channel, frame, 0.0));
            context.set_output_sample(channel, frame, output).unwrap();
        }
    }
}

pub(super) fn process_frequency_splitter(
    state: &mut PerModuleState,
    context: &mut ProcessContext<'_>,
) {
    let PerModuleState::FrequencySplitter {
        filters,
        sample_rate,
    } = state
    else {
        unreachable!()
    };
    process_frequency_splitter_states(filters, *sample_rate, context);
}

pub(super) fn process_frequency_splitter_states(
    filters: &mut [(
        crate::crossover::LinkwitzRiley4,
        crate::crossover::LinkwitzRiley4,
    )],
    sample_rate: f64,
    context: &mut ProcessContext<'_>,
) {
    let channels = filters.len();
    for (channel, (first, second)) in filters.iter_mut().enumerate() {
        for frame in 0..context.frames() {
            let hz = (context.input_sample(channels, frame, 0.2) as f64 * 16000.0 + 40.0)
                .clamp(40.0, 20000.0);
            let norm = (hz / sample_rate).clamp(0.0, 0.49);
            first.set_crossover(norm);
            second.set_crossover((norm * 4.0).clamp(0.0, 0.49));
            let (low, rest) = first.process(context.input_sample(channel, frame, 0.0));
            let (mid, high) = second.process(rest);
            context.set_output_sample(channel, frame, low).unwrap();
            context
                .set_output_sample(channels + channel, frame, mid)
                .unwrap();
            context
                .set_output_sample(channels * 2 + channel, frame, high)
                .unwrap();
        }
    }
}

pub(super) fn process_echo(state: &mut PerModuleState, context: &mut ProcessContext<'_>) {
    let PerModuleState::Echo { processor, .. } = state else {
        unreachable!()
    };
    process_echo_state(processor, context);
}

pub(super) fn process_echo_state(
    processor: &mut crate::echo::Echo,
    context: &mut ProcessContext<'_>,
) {
    let channels = context.output_count();
    let time_left_input = channels;
    let time_right_input = time_left_input + 1;
    let feedback_input = time_right_input + 1;
    let damping_input = feedback_input + 1;
    let wet_input = damping_input + 1;
    let dry_input = wet_input + 1;
    let ping_pong_input = dry_input + 2;

    for frame in 0..context.frames() {
        let damping_norm = context.input_sample(damping_input, frame, 0.5);
        processor.set_feedback(context.input_sample(feedback_input, frame, 0.5));
        processor.set_damping_cutoff((20.0 * 1000.0_f32.powf(damping_norm)) as f64);
        processor.set_wet_dry(
            context.input_sample(wet_input, frame, 0.7),
            context.input_sample(dry_input, frame, 0.5),
        );
        processor.set_delay_ms(
            lerp(
                1.0,
                2000.0,
                context.input_sample(time_left_input, frame, 0.5),
            ) as f64,
            lerp(
                1.0,
                2000.0,
                context.input_sample(time_right_input, frame, 0.5),
            ) as f64,
        );
        processor.set_ping_pong(context.input_sample(ping_pong_input, frame, 0.0) > 0.5);

        let left_input = context.input_sample(0, frame, 0.0);
        let right_input = if channels == STEREO_CHANNELS {
            context.input_sample(1, frame, 0.0)
        } else {
            left_input
        };
        let (left, right) = processor.process(left_input, right_input);
        context.set_output_sample(0, frame, left).unwrap();
        if channels == STEREO_CHANNELS {
            context.set_output_sample(1, frame, right).unwrap();
        }
    }
}

pub(super) fn process_reverb(state: &mut PerModuleState, context: &mut ProcessContext<'_>) {
    let PerModuleState::Reverb { processor, .. } = state else {
        unreachable!()
    };
    process_reverb_state(processor, context);
}

pub(super) fn process_reverb_state(
    processor: &mut crate::reverb::Reverb,
    context: &mut ProcessContext<'_>,
) {
    let channels = context.output_count();
    let decay_input = channels;
    let room_size_input = decay_input + 1;
    let pre_delay_input = room_size_input + 1;
    let damping_input = pre_delay_input + 1;
    let diffusion_input = damping_input + 1;
    let stereo_width_input = diffusion_input + 1;
    let wet_input = stereo_width_input + 1;
    let dry_input = wet_input + 1;

    for frame in 0..context.frames() {
        let damping_norm = context.input_sample(damping_input, frame, 0.5);
        processor
            .set_decay_time(lerp(0.1, 10.0, context.input_sample(decay_input, frame, 0.5)) as f64);
        processor.set_room_size(context.input_sample(room_size_input, frame, 0.5));
        processor.set_pre_delay(lerp(
            0.0,
            250.0,
            context.input_sample(pre_delay_input, frame, 0.0),
        ) as f64);
        processor.set_damping((20.0 * 1000.0_f32.powf(damping_norm)) as f64);
        processor.set_diffusion(context.input_sample(diffusion_input, frame, 0.5));
        processor.set_stereo_width(context.input_sample(stereo_width_input, frame, 0.5));
        processor.set_wet_dry(
            context.input_sample(wet_input, frame, 0.7),
            context.input_sample(dry_input, frame, 0.5),
        );

        let left_input = context.input_sample(0, frame, 0.0);
        let right_input = if channels == STEREO_CHANNELS {
            context.input_sample(1, frame, 0.0)
        } else {
            left_input
        };
        let (left, right) = processor.process(left_input, right_input);
        context.set_output_sample(0, frame, left).unwrap();
        if channels == STEREO_CHANNELS {
            context.set_output_sample(1, frame, right).unwrap();
        }
    }
}

fn lerp(min: f32, max: f32, normalized: f32) -> f32 {
    min + (max - min) * normalized.clamp(0.0, 1.0)
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}
