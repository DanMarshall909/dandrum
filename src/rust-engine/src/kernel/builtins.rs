//! Kernel builtin registry: the authoritative kernel-model declaration of every
//! Rust primitive (see `unify-graph-kernel` §2.4, §3.1).
//!
//! Each builtin is a [`GraphDefinition`] with an empty body — a primitive
//! implemented in Rust rather than authored in YAML — declaring its **ports**
//! (name, direction, signal type, channel count), the **control defaults** of
//! its tunable inputs, and its true processing **latency**.
//!
//! Latencies are the actual per-node values, verified against the DSP: spectral
//! analysis/synthesis has `fft_size - 1` samples (see `crate::spectral`), and
//! uniformly-partitioned overlap-add convolution has one partition block
//! (`crate::convolution::Convolution::BLOCK_SIZE`). Every other builtin declares
//! [`LatencySpec::Zero`] explicitly, so the audit is exhaustive rather than
//! implicit.
//!
//! Ports are declared at their **current** shapes: every port is mono, and the
//! stereo builtins (`echo`, `reverb`, `audio_output`) still expose `_l`/`_r`
//! pairs. This is deliberate — the legacy `Graph` the compilation bridge (§2.6)
//! lowers onto has no multichannel port representation, so collapsing these into
//! channel-polymorphic ports happens in §3.3 together with the dispatch adapter.
//!
//! Numeric tunable parameters are declared here as control input ports carrying
//! their default and range. Text/enum/integer construction-time parameters
//! (waveform, mode, fft_size, asset, …) are static parameters and land in §3.2;
//! only `fft_size` is declared here because latency resolves from it.

use crate::builtins::module_types as names;
use crate::convolution::Convolution;
use crate::graph::SignalType;
use crate::graph::builtin_ports as ports;

use super::{
    ControlDefault, DefinitionRegistry, GraphDefinition, LatencySpec, Port, StaticParam, StaticType,
    StaticValue,
};

/// Every builtin port is single-channel until §3.3 makes the stereo builtins
/// channel-polymorphic.
const MONO: u32 = 1;

/// Static parameter that drives the spectral processor's FFT frame size.
pub const SPECTRAL_FFT_SIZE_PARAM: &str = "fft_size";
/// Default FFT frame size, matching the legacy spectral builtin.
const SPECTRAL_DEFAULT_FFT_SIZE: i64 = 2048;
/// Spectral processing latency is `fft_size - SPECTRAL_LATENCY_OFFSET` samples.
const SPECTRAL_LATENCY_OFFSET: u32 = 1;

fn audio_in(name: &str) -> Port {
    Port::input(name, SignalType::Audio, MONO)
}

fn audio_out(name: &str) -> Port {
    Port::output(name, SignalType::Audio, MONO)
}

fn control_in(name: &str) -> Port {
    Port::input(name, SignalType::Control, MONO)
}

fn control_out(name: &str) -> Port {
    Port::output(name, SignalType::Control, MONO)
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
            .with_port(tunable(ports::PITCH, 1.0, 0.0, 64.0))
            .with_port(audio_out(ports::AUDIO)),
        primitive(names::GAIN)
            .with_port(audio_in(ports::AUDIO_IN))
            .with_port(tunable(ports::GAIN, 1.0, 0.0, 4.0))
            .with_port(audio_out(ports::AUDIO_OUT)),
        primitive(names::AUDIO_MIXER)
            .with_port(audio_in(ports::INPUTS))
            .with_port(audio_out(ports::MIX)),
        primitive(names::CONTROL_MIXER)
            .with_port(control_in(ports::INPUTS))
            .with_port(control_out(ports::SUM)),
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
        primitive(names::FILTER)
            .with_port(audio_in(ports::AUDIO_IN))
            .with_port(control_in(ports::CUTOFF))
            .with_port(control_in(ports::RESONANCE))
            .with_port(control_in(ports::GAIN))
            .with_port(audio_out(ports::AUDIO_OUT)),
        primitive(names::AUDIO_DELAY_ONE_SAMPLE)
            .with_port(audio_in(ports::AUDIO_IN))
            .with_port(audio_out(ports::AUDIO_OUT)),
        primitive(names::BLOCK_DELAY)
            .with_port(audio_in(ports::AUDIO_IN))
            .with_port(audio_out(ports::AUDIO_OUT)),
        primitive(names::CONTROL_DELAY)
            .with_port(control_in(ports::VALUE))
            .with_port(control_out(ports::VALUE)),
        // `script` declares its ports in YAML; the primitive itself has none.
        primitive(names::SCRIPT),
        primitive(names::SAMPLER)
            .with_port(event_in(ports::TRIGGER))
            .with_port(control_in(ports::RATE))
            .with_port(control_in(ports::START))
            .with_port(control_in(ports::LOOP_ENABLED))
            .with_port(control_in(ports::LOOP_START))
            .with_port(control_in(ports::LOOP_END))
            .with_port(audio_out(ports::AUDIO)),
        primitive(names::NOTE_TO_RATE)
            .with_port(event_in(ports::EVENTS))
            .with_port(control_out(ports::RATE)),
        primitive(names::EVENT_FILTER)
            .with_port(event_in(ports::EVENTS_IN))
            .with_port(event_out(ports::EVENTS_OUT)),
        primitive(names::DYNAMICS_PROCESSOR)
            .with_port(audio_in(ports::AUDIO_IN))
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
            .with_port(audio_out(ports::AUDIO_OUT)),
        primitive(names::SATURATOR)
            .with_port(audio_in(ports::AUDIO_IN))
            .with_port(control_in(ports::DRIVE))
            .with_port(control_in(ports::BIAS))
            .with_port(control_in(ports::CURVE_SELECT))
            .with_port(audio_out(ports::AUDIO_OUT)),
        convolution(),
        primitive(names::FREQUENCY_SPLITTER)
            .with_port(audio_in(ports::AUDIO_IN))
            .with_port(control_in(ports::CROSSOVER_HZ))
            .with_port(audio_out(ports::LOW))
            .with_port(audio_out(ports::MID))
            .with_port(audio_out(ports::HIGH)),
        spectral_processor(),
        primitive(names::ECHO)
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
        primitive(names::NOISE).with_port(audio_out(ports::AUDIO)),
        primitive(names::IMPULSE)
            .with_port(event_in(ports::TRIGGER))
            .with_port(audio_out(ports::AUDIO)),
        // `multiply`'s second input is an audio-rate gain, not a control port.
        primitive(names::MULTIPLY)
            .with_port(audio_in(ports::AUDIO_IN))
            .with_port(audio_in(ports::GAIN))
            .with_port(audio_out(ports::AUDIO_OUT)),
        primitive(names::NOTE_TO_CONTROL)
            .with_port(event_in(ports::EVENTS))
            .with_port(control_out(ports::FREQUENCY))
            .with_port(control_out(ports::PITCH_RATIO))
            .with_port(event_out(ports::GATE))
            .with_port(control_out(ports::VELOCITY)),
        primitive(names::ENVELOPE_FOLLOWER)
            .with_port(audio_in(ports::AUDIO_IN))
            .with_port(control_in(ports::ATTACK))
            .with_port(control_in(ports::RELEASE))
            .with_port(control_in(ports::AMOUNT))
            .with_port(control_in(ports::OFFSET))
            .with_port(control_in(ports::INVERT))
            .with_port(control_out(ports::VALUE)),
        primitive(names::CURVE_MAPPER)
            .with_port(control_in(ports::VALUE))
            .with_port(tunable(ports::AMOUNT, 1.0, 0.0, 1.0))
            .with_port(tunable(ports::BIAS, 0.0, -1.0, 1.0))
            .with_port(tunable(ports::SCALE, 1.0, -64.0, 64.0))
            .with_port(tunable(ports::OFFSET, 0.0, -64.0, 64.0))
            .with_port(control_out(ports::VALUE)),
        primitive(names::DECAY)
            .with_port(event_in(ports::TRIGGER))
            .with_port(tunable(ports::TIME_MS, 100.0, 1.0, 5000.0))
            .with_port(control_out(ports::VALUE)),
    ]
}

/// Spectral processor: latency is `fft_size - 1` samples, driven by the
/// `fft_size` static parameter.
fn spectral_processor() -> GraphDefinition {
    GraphDefinition::new(names::SPECTRAL_PROCESSOR)
        .with_static_param(
            StaticParam::new(SPECTRAL_FFT_SIZE_PARAM, StaticType::Int)
                .with_default(StaticValue::Int(SPECTRAL_DEFAULT_FFT_SIZE)),
        )
        .with_latency(LatencySpec::StaticParam {
            name: SPECTRAL_FFT_SIZE_PARAM.to_string(),
            minus: SPECTRAL_LATENCY_OFFSET,
        })
        .with_port(audio_in(ports::AUDIO_IN))
        .with_port(tunable(ports::THRESHOLD, -40.0, -100.0, 0.0))
        .with_port(tunable(ports::MIX, 1.0, 0.0, 1.0))
        .with_port(audio_out(ports::AUDIO_OUT))
}

/// Uniformly-partitioned overlap-add convolution: latency is one partition
/// block, sourced from the DSP's own block size so the two never drift.
fn convolution() -> GraphDefinition {
    GraphDefinition::new(names::CONVOLUTION)
        .with_latency(LatencySpec::Samples(Convolution::BLOCK_SIZE as u32))
        .with_port(audio_in(ports::AUDIO_IN))
        .with_port(control_in(ports::MIX))
        .with_port(audio_out(ports::AUDIO_OUT))
}

#[cfg(test)]
mod tests;
