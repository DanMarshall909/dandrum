use super::process_context::ProcessContext;
use super::state::PerModuleState;

pub(super) fn process_audio_mixer(context: &mut ProcessContext<'_>) {
    context
        .write_output_from_input(0, 0, |sample| sample)
        .expect("audio mixer input and output buffers should be available in supported arena step");
}

pub(super) fn process_noise(state: &mut PerModuleState, context: &mut ProcessContext<'_>) {
    let rng_state = match state {
        PerModuleState::Noise { state } => state,
        _ => unreachable!(),
    };

    for frame in 0..context.frames() {
        let mut x = *rng_state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        let sample = (x as f32) / (u32::MAX as f32) * 2.0 - 1.0;
        context
            .set_output_sample(0, frame, sample)
            .expect("noise output buffer should be available in supported arena step");
        *rng_state = x;
    }
}

pub(super) fn process_oscillator(state: &mut PerModuleState, context: &mut ProcessContext<'_>) {
    let (phase, sample_rate) = match state {
        PerModuleState::Oscillator { phase, sample_rate } => (phase, *sample_rate),
        _ => unreachable!(),
    };

    for frame in 0..context.frames() {
        let pitch_ratio = context.input_sample(0, frame, 1.0);
        let output = *phase * 2.0 - 1.0;
        let base_hz = 220.0;
        let freq = base_hz * pitch_ratio;
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
    context
        .write_output_from_two_inputs(0, 0, 1, |audio, gain| audio * gain)
        .expect("gain input and output buffers should be available in supported arena step");
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
    let (filter, sample_rate) = match state {
        PerModuleState::Filter {
            filter,
            sample_rate,
        } => (filter, *sample_rate),
        _ => unreachable!(),
    };

    for frame in 0..context.frames() {
        filter.set_cutoff_control(context.input_sample(1, frame, 0.5), sample_rate);
        filter.set_resonance_control(context.input_sample(2, frame, 0.0));
        filter.set_gain_db(context.input_sample(3, frame, 0.5) as f64 * 48.0 - 24.0);

        let output = filter.process(context.input_sample(0, frame, 0.0));
        context
            .set_output_sample(0, frame, output)
            .expect("filter output buffer should be available in supported arena step");
    }
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}
