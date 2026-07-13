//! Kernel builtin registry: the authoritative kernel-model declaration of every
//! Rust primitive (see `unify-graph-kernel` §2.4, §3.1).
//!
//! Each builtin is a [`GraphDefinition`] with an empty body — a primitive
//! implemented in Rust rather than authored in YAML — declaring its **ports**
//! (name, direction, signal type, channel count), the **control defaults** of
//! its tunable inputs, its construction-time **static parameters**, and its true
//! processing **latency**.
//!
//! Latencies are the actual per-node values, verified against the DSP: spectral
//! analysis/synthesis has `fft_size - 1` samples (see `crate::spectral`), and
//! uniformly-partitioned overlap-add convolution has one partition block
//! (`crate::convolution::Convolution::BLOCK_SIZE`). Every other builtin declares
//! [`LatencySpec::Zero`] explicitly, so the audit is exhaustive rather than
//! implicit.
//!
//! Channel-independent processors declare a `channels` static parameter and use
//! it for their signal-path ports. Intrinsically stereo echo/reverb retain their
//! L/R interfaces until their dedicated migration.
//!
//! Numeric tunable parameters are declared here as control input ports carrying
//! their default and range. Non-resource text/enum/integer construction-time
//! parameters are static parameters. Resource conversion remains in §3.5.

use crate::builtins::module_types as names;
use crate::builtins::{
    CURVE_EXPONENTIAL, CURVE_HARD_CLIP, CURVE_LINEAR, CURVE_LOGARITHMIC, CURVE_PARAMETER,
    CURVE_S_CURVE, CURVE_SOFT_CLIP, CURVE_STEP, DELAY_SAMPLES_PARAMETER, DETECTION_MODE_PARAMETER,
    DETECTION_MODE_PEAK, DETECTION_MODE_RMS, DYNAMICS_DETECTION_PARAMETER, DYNAMICS_MODE_LEVEL,
    DYNAMICS_MODE_PARAMETER, DYNAMICS_MODE_TRANSIENT, DYNAMICS_TOPOLOGY_FEEDBACK,
    DYNAMICS_TOPOLOGY_FEEDFORWARD, DYNAMICS_TOPOLOGY_PARAMETER, EVENT_FILTER_NOTE_PARAMETER,
    EVENT_FILTER_NOTE_SELECTOR, EVENT_FILTER_SELECTOR_DEFAULT, EVENT_FILTER_SELECTOR_PARAMETER,
    FILTER_ALGORITHM_BIQUAD, FILTER_ALGORITHM_COMB, FILTER_ALGORITHM_MOOG,
    FILTER_ALGORITHM_PARAMETER, FILTER_COMB_TYPE_PARAMETER, FILTER_MODE_HIGHPASS,
    FILTER_MODE_LOWPASS, FILTER_MODE_PARAMETER, FILTER_MODE_PEAKING, INTERPOLATION_CUBIC,
    INTERPOLATION_LINEAR, INTERPOLATION_PARAMETER, NOISE_DEFAULT_SEED, NOISE_SEED_PARAMETER,
    SCRIPT_LANGUAGE_PARAMETER, SCRIPT_LANGUAGE_RHAI, SCRIPT_SOURCE_PARAMETER,
    SPECTRAL_DEFAULT_FFT_SIZE, SPECTRAL_FFT_SIZE_PARAMETER, SPECTRAL_MODE_GATE,
    SPECTRAL_MODE_PARAMETER, SPECTRAL_MODE_PASSTHROUGH, SPECTRAL_WINDOW_HANN,
    SPECTRAL_WINDOW_PARAMETER, STEPS_PARAMETER, WAVEFORM_PARAMETER, WAVEFORM_SAW, WAVEFORM_SINE,
    WAVEFORM_SQUARE, WAVEFORM_TRIANGLE,
};
use crate::convolution::Convolution;
use crate::graph::SignalType;
use crate::graph::builtin_ports as ports;

use super::{
    ChannelCount, ControlDefault, DefinitionRegistry, GraphDefinition, LatencySpec, Multiplicity,
    Port, StaticParam, StaticType, StaticValue,
};

const MONO: u32 = 1;

/// Static parameter that drives the spectral processor's FFT frame size.
pub const SPECTRAL_FFT_SIZE_PARAM: &str = SPECTRAL_FFT_SIZE_PARAMETER;
pub const DELAY_SAMPLES_PARAM: &str = DELAY_SAMPLES_PARAMETER;
pub const CHANNELS_PARAM: &str = "channels";
/// Default FFT frame size, matching the legacy spectral builtin.
/// Spectral processing latency is `fft_size - SPECTRAL_LATENCY_OFFSET` samples.
const SPECTRAL_LATENCY_OFFSET: u32 = 1;

fn audio_in(name: &str) -> Port {
    Port::input(name, SignalType::Audio, MONO)
}

fn audio_out(name: &str) -> Port {
    Port::output(name, SignalType::Audio, MONO)
}

fn poly_audio_in(name: &str) -> Port {
    Port::input(name, SignalType::Audio, ChannelCount::param(CHANNELS_PARAM))
}

fn poly_audio_out(name: &str) -> Port {
    Port::output(name, SignalType::Audio, ChannelCount::param(CHANNELS_PARAM))
}

fn control_in(name: &str) -> Port {
    Port::input(name, SignalType::Control, MONO)
}

fn control_out(name: &str) -> Port {
    Port::output(name, SignalType::Control, MONO)
}

fn poly_control_in(name: &str) -> Port {
    Port::input(
        name,
        SignalType::Control,
        ChannelCount::param(CHANNELS_PARAM),
    )
}

fn poly_control_out(name: &str) -> Port {
    Port::output(
        name,
        SignalType::Control,
        ChannelCount::param(CHANNELS_PARAM),
    )
}

fn event_in(name: &str) -> Port {
    Port::input(name, SignalType::Event, MONO)
}

fn event_out(name: &str) -> Port {
    Port::output(name, SignalType::Event, MONO)
}

/// A control input port carrying the default and range of the tunable parameter
/// it replaces.
fn tunable(name: &str, default: f64, min: f64, max: f64) -> Port {
    control_in(name).with_control_default(ControlDefault::new(default).with_min(min).with_max(max))
}

/// A zero-latency primitive. Stating latency at construction means no builtin is
/// ever implicitly zero.
fn primitive(name: &str) -> GraphDefinition {
    GraphDefinition::new(name).with_latency(LatencySpec::Zero)
}

fn channel_primitive(name: &str) -> GraphDefinition {
    primitive(name).with_static_param(
        StaticParam::new(CHANNELS_PARAM, StaticType::Int).with_default(StaticValue::Int(1)),
    )
}

fn enum_param(name: &str, default: &str, allowed_values: &[&str]) -> StaticParam {
    StaticParam::new(name, StaticType::Enum)
        .with_default(StaticValue::Enum(default.to_string()))
        .with_allowed_values(allowed_values.iter().copied())
}

/// The kernel registry of all builtins with their ports, control defaults, and
/// declared latency.
pub fn builtin_registry() -> DefinitionRegistry {
    let mut registry = DefinitionRegistry::new();
    for definition in builtin_definitions() {
        registry = registry.with_definition(definition);
    }
    registry
}

fn builtin_definitions() -> Vec<GraphDefinition> {
    vec![
        primitive(names::MIDI_INPUT).with_port(event_out(ports::EVENTS)),
        // Deleted in §3.4 in favour of root ports; declared meanwhile so the
        // registry covers every builtin the legacy graph can name.
        primitive(names::AUDIO_OUTPUT)
            .with_port(audio_in(ports::LEFT))
            .with_port(audio_in(ports::RIGHT)),
        primitive(names::OSCILLATOR)
            .with_static_param(enum_param(
                WAVEFORM_PARAMETER,
                WAVEFORM_SAW,
                &[
                    WAVEFORM_SAW,
                    WAVEFORM_SINE,
                    WAVEFORM_TRIANGLE,
                    WAVEFORM_SQUARE,
                ],
            ))
            .with_port(tunable(ports::PITCH, 1.0, 0.0, 64.0))
            .with_port(audio_out(ports::AUDIO)),
        channel_primitive(names::GAIN)
            .with_port(poly_audio_in(ports::AUDIO_IN))
            .with_port(tunable(ports::GAIN, 1.0, 0.0, 4.0))
            .with_port(poly_audio_out(ports::AUDIO_OUT)),
        channel_primitive(names::AUDIO_MIXER)
            .with_port(poly_audio_in(ports::INPUTS).with_multiplicity(Multiplicity::Summing))
            .with_port(poly_audio_out(ports::MIX)),
        channel_primitive(names::CONTROL_MIXER)
            .with_port(poly_control_in(ports::INPUTS).with_multiplicity(Multiplicity::Summing))
            .with_port(poly_control_out(ports::SUM)),
        primitive(names::ADSR)
            .with_port(event_in(ports::GATE))
            .with_port(tunable(ports::ATTACK, 5.0, 0.0, 500.0))
            .with_port(tunable(ports::DECAY, 30.0, 1.0, 5000.0))
            .with_port(tunable(ports::SUSTAIN, 0.7, 0.0, 1.0))
            .with_port(tunable(ports::RELEASE, 200.0, 0.0, 10000.0))
            .with_port(control_out(ports::VALUE)),
        primitive(names::LFO)
            .with_port(control_in(ports::RATE))
            .with_port(control_out(ports::VALUE)),
        channel_primitive(names::FILTER)
            .with_static_param(enum_param(
                FILTER_ALGORITHM_PARAMETER,
                FILTER_ALGORITHM_MOOG,
                &[
                    FILTER_ALGORITHM_MOOG,
                    FILTER_ALGORITHM_BIQUAD,
                    FILTER_ALGORITHM_COMB,
                ],
            ))
            .with_static_param(enum_param(
                FILTER_MODE_PARAMETER,
                FILTER_MODE_LOWPASS,
                &[
                    FILTER_MODE_LOWPASS,
                    FILTER_MODE_HIGHPASS,
                    FILTER_MODE_PEAKING,
                ],
            ))
            .with_static_param(enum_param(
                FILTER_COMB_TYPE_PARAMETER,
                DYNAMICS_TOPOLOGY_FEEDBACK,
                &[DYNAMICS_TOPOLOGY_FEEDBACK, DYNAMICS_TOPOLOGY_FEEDFORWARD],
            ))
            .with_port(poly_audio_in(ports::AUDIO_IN))
            .with_port(control_in(ports::CUTOFF))
            .with_port(control_in(ports::RESONANCE))
            .with_port(control_in(ports::GAIN))
            .with_port(poly_audio_out(ports::AUDIO_OUT)),
        channel_primitive(names::AUDIO_DELAY_ONE_SAMPLE)
            .with_port(poly_audio_in(ports::AUDIO_IN))
            .with_port(poly_audio_out(ports::AUDIO_OUT)),
        channel_primitive(names::BLOCK_DELAY)
            .with_port(poly_audio_in(ports::AUDIO_IN))
            .with_port(poly_audio_out(ports::AUDIO_OUT)),
        primitive(names::CONTROL_DELAY)
            .with_port(control_in(ports::VALUE))
            .with_port(control_out(ports::VALUE)),
        // `script` declares its ports in YAML; the primitive itself has none.
        primitive(names::SCRIPT)
            .with_static_param(enum_param(
                SCRIPT_LANGUAGE_PARAMETER,
                SCRIPT_LANGUAGE_RHAI,
                &[SCRIPT_LANGUAGE_RHAI],
            ))
            .with_static_param(StaticParam::new(
                SCRIPT_SOURCE_PARAMETER,
                StaticType::String,
            )),
        channel_primitive(names::SAMPLER)
            .with_port(event_in(ports::TRIGGER))
            .with_port(control_in(ports::RATE))
            .with_port(control_in(ports::START))
            .with_port(control_in(ports::LOOP_ENABLED))
            .with_port(control_in(ports::LOOP_START))
            .with_port(control_in(ports::LOOP_END))
            .with_port(poly_audio_out(ports::AUDIO)),
        primitive(names::NOTE_TO_RATE)
            .with_port(event_in(ports::EVENTS))
            .with_port(control_out(ports::RATE)),
        primitive(names::EVENT_FILTER)
            .with_static_param(enum_param(
                EVENT_FILTER_SELECTOR_PARAMETER,
                EVENT_FILTER_SELECTOR_DEFAULT,
                &[EVENT_FILTER_NOTE_SELECTOR],
            ))
            .with_static_param(StaticParam::new(
                EVENT_FILTER_NOTE_PARAMETER,
                StaticType::Int,
            ))
            .with_port(event_in(ports::EVENTS_IN))
            .with_port(event_out(ports::EVENTS_OUT)),
        channel_primitive(names::DYNAMICS_PROCESSOR)
            .with_static_param(enum_param(
                DYNAMICS_MODE_PARAMETER,
                DYNAMICS_MODE_LEVEL,
                &[DYNAMICS_MODE_LEVEL, DYNAMICS_MODE_TRANSIENT],
            ))
            .with_static_param(enum_param(
                DYNAMICS_DETECTION_PARAMETER,
                DETECTION_MODE_PEAK,
                &[DETECTION_MODE_PEAK, DETECTION_MODE_RMS],
            ))
            .with_static_param(enum_param(
                DYNAMICS_TOPOLOGY_PARAMETER,
                DYNAMICS_TOPOLOGY_FEEDFORWARD,
                &[DYNAMICS_TOPOLOGY_FEEDFORWARD, DYNAMICS_TOPOLOGY_FEEDBACK],
            ))
            .with_port(poly_audio_in(ports::AUDIO_IN))
            .with_port(control_in(ports::SIDECHAIN_IN))
            .with_port(control_in(ports::THRESHOLD))
            .with_port(control_in(ports::BELOW_RATIO))
            .with_port(control_in(ports::ABOVE_RATIO))
            .with_port(control_in(ports::ATTACK))
            .with_port(control_in(ports::RELEASE))
            .with_port(control_in(ports::KNEE))
            .with_port(control_in(ports::MAKEUP_GAIN))
            .with_port(control_in(ports::ATTACK_GAIN))
            .with_port(control_in(ports::SUSTAIN_GAIN))
            .with_port(poly_audio_out(ports::AUDIO_OUT)),
        channel_primitive(names::SATURATOR)
            .with_port(poly_audio_in(ports::AUDIO_IN))
            .with_port(control_in(ports::DRIVE))
            .with_port(control_in(ports::BIAS))
            .with_port(control_in(ports::CURVE_SELECT))
            .with_port(poly_audio_out(ports::AUDIO_OUT)),
        convolution(),
        channel_primitive(names::FREQUENCY_SPLITTER)
            .with_port(poly_audio_in(ports::AUDIO_IN))
            .with_port(control_in(ports::CROSSOVER_HZ))
            .with_port(poly_audio_out(ports::LOW))
            .with_port(poly_audio_out(ports::MID))
            .with_port(poly_audio_out(ports::HIGH)),
        spectral_processor(),
        primitive(names::ECHO)
            .with_static_param(enum_param(
                INTERPOLATION_PARAMETER,
                INTERPOLATION_LINEAR,
                &[INTERPOLATION_LINEAR, INTERPOLATION_CUBIC],
            ))
            .with_port(audio_in(ports::AUDIO_IN_L))
            .with_port(audio_in(ports::AUDIO_IN_R))
            .with_port(control_in(ports::TIME_LEFT_MS))
            .with_port(control_in(ports::TIME_RIGHT_MS))
            .with_port(control_in(ports::FEEDBACK))
            .with_port(control_in(ports::DAMPING_CUTOFF))
            .with_port(control_in(ports::WET))
            .with_port(control_in(ports::DRY))
            .with_port(control_in(ports::SYNC_DIVISION))
            .with_port(control_in(ports::PING_PONG))
            .with_port(audio_out(ports::AUDIO_OUT_L))
            .with_port(audio_out(ports::AUDIO_OUT_R)),
        primitive(names::REVERB)
            .with_static_param(enum_param(
                INTERPOLATION_PARAMETER,
                INTERPOLATION_LINEAR,
                &[INTERPOLATION_LINEAR, INTERPOLATION_CUBIC],
            ))
            .with_port(audio_in(ports::AUDIO_IN_L))
            .with_port(audio_in(ports::AUDIO_IN_R))
            .with_port(control_in(ports::DECAY_TIME))
            .with_port(control_in(ports::ROOM_SIZE))
            .with_port(control_in(ports::PRE_DELAY))
            .with_port(control_in(ports::DAMPING))
            .with_port(control_in(ports::DIFFUSION))
            .with_port(control_in(ports::STEREO_WIDTH))
            .with_port(control_in(ports::WET))
            .with_port(control_in(ports::DRY))
            .with_port(audio_out(ports::AUDIO_OUT_L))
            .with_port(audio_out(ports::AUDIO_OUT_R)),
        channel_primitive(names::NOISE)
            .with_static_param(
                StaticParam::new(NOISE_SEED_PARAMETER, StaticType::Int)
                    .with_default(StaticValue::Int(NOISE_DEFAULT_SEED as i64)),
            )
            .with_port(poly_audio_out(ports::AUDIO)),
        channel_primitive(names::IMPULSE)
            .with_port(event_in(ports::TRIGGER))
            .with_port(poly_audio_out(ports::AUDIO)),
        // `multiply`'s second input is an audio-rate gain, not a control port.
        channel_primitive(names::MULTIPLY)
            .with_port(poly_audio_in(ports::AUDIO_IN))
            .with_port(poly_audio_in(ports::GAIN))
            .with_port(poly_audio_out(ports::AUDIO_OUT)),
        primitive(names::NOTE_TO_CONTROL)
            .with_port(event_in(ports::EVENTS))
            .with_port(control_out(ports::FREQUENCY))
            .with_port(control_out(ports::PITCH_RATIO))
            .with_port(event_out(ports::GATE))
            .with_port(control_out(ports::VELOCITY)),
        channel_primitive(names::ENVELOPE_FOLLOWER)
            .with_static_param(enum_param(
                DETECTION_MODE_PARAMETER,
                DETECTION_MODE_PEAK,
                &[DETECTION_MODE_PEAK, DETECTION_MODE_RMS],
            ))
            .with_port(poly_audio_in(ports::AUDIO_IN))
            .with_port(control_in(ports::ATTACK))
            .with_port(control_in(ports::RELEASE))
            .with_port(control_in(ports::AMOUNT))
            .with_port(control_in(ports::OFFSET))
            .with_port(control_in(ports::INVERT))
            .with_port(Port::output(
                ports::VALUE,
                SignalType::Control,
                ChannelCount::param(CHANNELS_PARAM),
            )),
        primitive(names::CURVE_MAPPER)
            .with_static_param(enum_param(
                CURVE_PARAMETER,
                CURVE_LINEAR,
                &[
                    CURVE_LINEAR,
                    CURVE_EXPONENTIAL,
                    CURVE_LOGARITHMIC,
                    CURVE_S_CURVE,
                    CURVE_SOFT_CLIP,
                    CURVE_HARD_CLIP,
                    CURVE_STEP,
                ],
            ))
            .with_static_param(
                StaticParam::new(STEPS_PARAMETER, StaticType::Int).with_default(StaticValue::Int(
                    crate::curve_mapper::CurveMapper::DEFAULT_STEPS as i64,
                )),
            )
            .with_port(control_in(ports::VALUE))
            .with_port(tunable(ports::AMOUNT, 1.0, 0.0, 1.0))
            .with_port(tunable(ports::BIAS, 0.0, -1.0, 1.0))
            .with_port(tunable(ports::SCALE, 1.0, -64.0, 64.0))
            .with_port(tunable(ports::OFFSET, 0.0, -64.0, 64.0))
            .with_port(control_out(ports::VALUE)),
        primitive(names::DECAY)
            .with_static_param(enum_param(
                CURVE_PARAMETER,
                CURVE_EXPONENTIAL,
                &[CURVE_LINEAR, CURVE_EXPONENTIAL],
            ))
            .with_port(event_in(ports::TRIGGER))
            .with_port(tunable(ports::TIME_MS, 100.0, 1.0, 5000.0))
            .with_port(control_out(ports::VALUE)),
        // Compiler-generated by flattening (§2.5); declared here so the bridge
        // can lower it and so an authored instance validates like any other.
        channel_primitive(names::CONTROL_TO_AUDIO)
            .with_port(poly_control_in(ports::IN))
            .with_port(poly_audio_out(ports::OUT)),
        compensation_delay(),
    ]
}

/// Spectral processor: latency is `fft_size - 1` samples, driven by the
/// `fft_size` static parameter.
fn spectral_processor() -> GraphDefinition {
    GraphDefinition::new(names::SPECTRAL_PROCESSOR)
        .with_static_param(
            StaticParam::new(CHANNELS_PARAM, StaticType::Int).with_default(StaticValue::Int(1)),
        )
        .with_static_param(
            StaticParam::new(SPECTRAL_FFT_SIZE_PARAM, StaticType::Int)
                .with_default(StaticValue::Int(SPECTRAL_DEFAULT_FFT_SIZE as i64)),
        )
        .with_static_param(enum_param(
            SPECTRAL_MODE_PARAMETER,
            SPECTRAL_MODE_GATE,
            &[SPECTRAL_MODE_GATE, SPECTRAL_MODE_PASSTHROUGH],
        ))
        .with_static_param(enum_param(
            SPECTRAL_WINDOW_PARAMETER,
            SPECTRAL_WINDOW_HANN,
            &[SPECTRAL_WINDOW_HANN],
        ))
        .with_latency(LatencySpec::StaticParam {
            name: SPECTRAL_FFT_SIZE_PARAM.to_string(),
            minus: SPECTRAL_LATENCY_OFFSET,
        })
        .with_port(poly_audio_in(ports::AUDIO_IN))
        .with_port(tunable(ports::THRESHOLD, -40.0, -100.0, 0.0))
        .with_port(tunable(ports::MIX, 1.0, 0.0, 1.0))
        .with_port(poly_audio_out(ports::AUDIO_OUT))
}

/// Uniformly-partitioned overlap-add convolution: latency is one partition
/// block, sourced from the DSP's own block size so the two never drift.
fn convolution() -> GraphDefinition {
    GraphDefinition::new(names::CONVOLUTION)
        .with_static_param(
            StaticParam::new(CHANNELS_PARAM, StaticType::Int).with_default(StaticValue::Int(1)),
        )
        .with_latency(LatencySpec::Samples(Convolution::BLOCK_SIZE as u32))
        .with_port(poly_audio_in(ports::AUDIO_IN))
        .with_port(control_in(ports::MIX))
        .with_port(poly_audio_out(ports::AUDIO_OUT))
}

fn compensation_delay() -> GraphDefinition {
    GraphDefinition::new(names::COMPENSATION_DELAY)
        .with_static_param(
            StaticParam::new(CHANNELS_PARAM, StaticType::Int).with_default(StaticValue::Int(1)),
        )
        .with_static_param(StaticParam::new(DELAY_SAMPLES_PARAM, StaticType::Int))
        .with_latency(LatencySpec::StaticParam {
            name: DELAY_SAMPLES_PARAM.to_string(),
            minus: 0,
        })
        .with_port(poly_audio_in(ports::AUDIO_IN))
        .with_port(poly_audio_out(ports::AUDIO_OUT))
}

#[cfg(test)]
mod tests;
