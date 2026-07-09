use std::collections::BTreeMap;

use super::*;
use crate::convolution::Convolution;
use crate::kernel::{GraphDefinition, StaticValue};

/// Every builtin the kernel registry must declare.
const EXPECTED: [&str; 30] = [
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
    names::CONVOLUTION,
    names::ECHO,
    names::REVERB,
    names::FREQUENCY_SPLITTER,
    names::SPECTRAL_PROCESSOR,
    names::NOISE,
    names::IMPULSE,
    names::MULTIPLY,
    names::NOTE_TO_CONTROL,
    names::EVENT_FILTER,
    names::ENVELOPE_FOLLOWER,
    names::CURVE_MAPPER,
    names::DECAY,
];

/// Resolve a definition's latency against its declared static-parameter
/// defaults, exactly as compilation would for an instance with no overrides.
fn latency_with_defaults(definition: &GraphDefinition) -> u32 {
    let args: BTreeMap<String, StaticValue> = definition
        .static_params()
        .iter()
        .filter_map(|param| param.default().map(|value| (param.name().to_string(), value.clone())))
        .collect();
    definition.latency().resolve(&args)
}

#[test]
fn registry_declares_every_builtin_and_no_others() {
    let registry = builtin_registry();
    for name in EXPECTED {
        assert!(
            registry.get(name).is_some(),
            "builtin '{name}' is missing from the kernel registry"
        );
    }
    assert_eq!(
        registry.definitions().count(),
        EXPECTED.len(),
        "registry must contain exactly the declared builtins"
    );
}

#[test]
fn spectral_processor_declares_fft_size_minus_one_latency() {
    let registry = builtin_registry();
    let spectral = registry
        .get(names::SPECTRAL_PROCESSOR)
        .expect("spectral processor declared");

    assert_eq!(
        latency_with_defaults(spectral),
        2047,
        "default fft_size 2048 yields fft_size - 1 latency"
    );

    let args = BTreeMap::from([(SPECTRAL_FFT_SIZE_PARAM.to_string(), StaticValue::Int(512))]);
    assert_eq!(
        spectral.latency().resolve(&args),
        511,
        "fft_size 512 yields 511 samples of latency"
    );
}

#[test]
fn convolution_declares_one_block_latency() {
    let registry = builtin_registry();
    let convolution = registry
        .get(names::CONVOLUTION)
        .expect("convolution declared");

    assert_eq!(
        latency_with_defaults(convolution),
        Convolution::BLOCK_SIZE as u32,
        "convolution latency is one overlap-add partition block"
    );
}

#[test]
fn every_other_builtin_declares_zero_latency() {
    let registry = builtin_registry();
    for definition in registry.definitions() {
        if definition.name() == names::SPECTRAL_PROCESSOR
            || definition.name() == names::CONVOLUTION
        {
            continue;
        }
        assert_eq!(
            latency_with_defaults(definition),
            0,
            "builtin '{}' must declare zero latency explicitly",
            definition.name()
        );
    }
}
