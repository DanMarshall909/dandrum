use super::process_context::ProcessContext;
use super::state::PerModuleState;
use crate::oscillator::OSCILLATOR_BASE_HZ;

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
        PerModuleState::Noise { states } => states,
        _ => unreachable!(),
    };

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

pub(super) fn process_oscillator(state: &mut PerModuleState, context: &mut ProcessContext<'_>) {
    let (phase, sample_rate, waveform) = match state {
        PerModuleState::Oscillator {
            phase,
            sample_rate,
            waveform,
        } => (phase, *sample_rate, *waveform),
        _ => unreachable!(),
    };

    for frame in 0..context.frames() {
        let pitch_ratio = context.input_sample(0, frame, 1.0);
        let output = waveform.sample(*phase);
        let freq = OSCILLATOR_BASE_HZ * pitch_ratio;
        let phase_inc = freq / sample_rate;
        *phase += phase_inc;
        if *phase >= 1.0 {
            *phase -= 1.0;
        }
        context
            .set_output_sample(0, frame, output)
            .expect("oscillator output buffer should be available in supported arena step");
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

pub(super) fn process_envelope_follower(
    state: &mut PerModuleState,
    context: &mut ProcessContext<'_>,
) {
    let (detector, mode) = match state {
        PerModuleState::EnvelopeFollower { detector, mode } => (detector, *mode),
        _ => unreachable!(),
    };

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

pub(super) fn process_convolution(state: &mut PerModuleState, context: &mut ProcessContext<'_>) {
    let PerModuleState::Convolution { processors } = state else {
        unreachable!()
    };
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
    let channels = filters.len();
    for (channel, (first, second)) in filters.iter_mut().enumerate() {
        for frame in 0..context.frames() {
            let hz = (context.input_sample(channels, frame, 0.2) as f64 * 16000.0 + 40.0)
                .clamp(40.0, 20000.0);
            let norm = (hz / *sample_rate).clamp(0.0, 0.49);
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
