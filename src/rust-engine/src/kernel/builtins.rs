//! Kernel builtin registry: authoritative per-node latency declarations for
//! every Rust primitive (see `unify-graph-kernel` §2.4).
//!
//! Each builtin is declared as a kernel [`GraphDefinition`] carrying its true
//! processing latency. Only the spectral processor (`fft_size - 1`) and the
//! uniformly-partitioned overlap-add convolution (one block) have nonzero
//! latency; every other builtin declares [`LatencySpec::Zero`] explicitly so the
//! audit is exhaustive rather than implicit.
//!
//! This is the single source of truth for per-node latency consumed by the
//! latency-balancing pass (§2.4b) and the compilation pipeline (§2.6). The full
//! port and control-default contract for these definitions is layered on in
//! §3.1/§3.2; declaring ports here would only be reworked when stereo builtins
//! become channel-polymorphic (§3.3), so latency is declared on its own first.

use crate::builtins::module_types as names;
use crate::convolution::Convolution;

use super::{DefinitionRegistry, GraphDefinition, LatencySpec, StaticParam, StaticType, StaticValue};

/// Static parameter that drives the spectral processor's FFT frame size.
const SPECTRAL_FFT_SIZE_PARAM: &str = "fft_size";
/// Default FFT frame size, matching the legacy spectral builtin.
const SPECTRAL_DEFAULT_FFT_SIZE: i64 = 2048;
/// Spectral processing latency is `fft_size - SPECTRAL_LATENCY_OFFSET` samples.
const SPECTRAL_LATENCY_OFFSET: u32 = 1;

/// The kernel registry of all builtins, each declaring its true processing
/// latency.
pub fn builtin_registry() -> DefinitionRegistry {
    let mut registry = DefinitionRegistry::new();
    for definition in builtin_definitions() {
        registry = registry.with_definition(definition);
    }
    registry
}

fn builtin_definitions() -> Vec<GraphDefinition> {
    let mut definitions = vec![spectral_processor(), convolution()];
    definitions.extend(zero_latency_builtins());
    definitions
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
}

/// Uniformly-partitioned overlap-add convolution: latency is one partition
/// block, sourced from the DSP's own block size so the two never drift.
fn convolution() -> GraphDefinition {
    GraphDefinition::new(names::CONVOLUTION)
        .with_latency(LatencySpec::Samples(Convolution::BLOCK_SIZE as u32))
}

/// Every builtin with no processing latency, each declaring it explicitly.
fn zero_latency_builtins() -> Vec<GraphDefinition> {
    [
        names::MIDI_INPUT,
        names::AUDIO_OUTPUT,
        names::OSCILLATOR,
        names::GAIN,
        names::AUDIO_MIXER,
        names::CONTROL_MIXER,
        names::ADSR,
        names::LFO,
        names::FILTER,
        names::AUDIO_DELAY_ONE_SAMPLE,
        names::BLOCK_DELAY,
        names::CONTROL_DELAY,
        names::SCRIPT,
        names::SAMPLER,
        names::NOTE_TO_RATE,
        names::DYNAMICS_PROCESSOR,
        names::SATURATOR,
        names::ECHO,
        names::REVERB,
        names::FREQUENCY_SPLITTER,
        names::NOISE,
        names::IMPULSE,
        names::MULTIPLY,
        names::NOTE_TO_CONTROL,
        names::EVENT_FILTER,
        names::ENVELOPE_FOLLOWER,
        names::CURVE_MAPPER,
        names::DECAY,
    ]
    .into_iter()
    .map(|name| GraphDefinition::new(name).with_latency(LatencySpec::Zero))
    .collect()
}

#[cfg(test)]
mod tests;
