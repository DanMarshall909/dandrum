use std::collections::BTreeMap;

use super::*;
use crate::convolution::Convolution;
use crate::kernel::{GraphDefinition, StaticValue};

/// Every builtin the kernel registry must declare.
const EXPECTED: [&str; 31] = [
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
    names::CONTROL_TO_AUDIO,
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

// --- 3.1 Port and control-default declarations ---------------------------

use crate::graph::builtin_ports as ports;
use crate::graph::{PortDirection, SignalType};
use crate::kernel::Port;

fn port_of<'a>(
    definition: &'a GraphDefinition,
    name: &str,
    direction: PortDirection,
) -> &'a Port {
    definition
        .ports()
        .iter()
        .find(|port| port.name() == name && port.direction() == direction)
        .unwrap_or_else(|| panic!("'{}' declares port '{name}' ({direction:?})", definition.name()))
}

#[test]
fn gain_declares_audio_ports_and_a_tunable_gain_control() {
    let registry = builtin_registry();
    let gain = registry.get(names::GAIN).expect("gain declared");

    assert_eq!(
        port_of(gain, ports::AUDIO_IN, PortDirection::Input).signal_type(),
        SignalType::Audio
    );
    assert_eq!(
        port_of(gain, ports::AUDIO_OUT, PortDirection::Output).signal_type(),
        SignalType::Audio
    );

    let level = port_of(gain, ports::GAIN, PortDirection::Input);
    assert_eq!(
        level.signal_type(),
        SignalType::Control,
        "the tunable gain parameter became a control input port"
    );
    let default = level.control_default().expect("gain carries a control default");
    assert_eq!(default.default(), 1.0);
    assert_eq!(default.min(), Some(0.0));
    assert_eq!(default.max(), Some(4.0));
}

#[test]
fn adsr_tunable_controls_carry_declared_defaults_and_ranges() {
    let registry = builtin_registry();
    let adsr = registry.get(names::ADSR).expect("adsr declared");

    for (name, default, min, max) in [
        (ports::ATTACK, 5.0, 0.0, 500.0),
        (ports::DECAY, 30.0, 1.0, 5000.0),
        (ports::SUSTAIN, 0.7, 0.0, 1.0),
        (ports::RELEASE, 200.0, 0.0, 10000.0),
    ] {
        let port = port_of(adsr, name, PortDirection::Input);
        let control = port
            .control_default()
            .unwrap_or_else(|| panic!("'{name}' carries a control default"));
        assert_eq!(control.default(), default, "default of '{name}'");
        assert_eq!(control.min(), Some(min), "min of '{name}'");
        assert_eq!(control.max(), Some(max), "max of '{name}'");
    }

    assert_eq!(
        port_of(adsr, ports::GATE, PortDirection::Input).signal_type(),
        SignalType::Event,
        "gate stays an event port"
    );
}

#[test]
fn stereo_builtins_still_expose_left_right_port_pairs() {
    // Channel-polymorphism (§3.3) collapses these into 2-channel ports; until
    // then the legacy Graph the bridge lowers onto only understands _l/_r.
    let registry = builtin_registry();
    for name in [names::ECHO, names::REVERB] {
        let definition = registry.get(name).unwrap_or_else(|| panic!("{name} declared"));
        port_of(definition, ports::AUDIO_IN_L, PortDirection::Input);
        port_of(definition, ports::AUDIO_IN_R, PortDirection::Input);
        port_of(definition, ports::AUDIO_OUT_L, PortDirection::Output);
        port_of(definition, ports::AUDIO_OUT_R, PortDirection::Output);
    }
}

#[test]
fn every_builtin_port_is_mono_until_channel_polymorphism() {
    let registry = builtin_registry();
    for definition in registry.definitions() {
        for port in definition.ports() {
            assert_eq!(
                port.channels(),
                &crate::kernel::ChannelCount::Literal(1),
                "port '{}' of '{}' is mono at current shapes",
                port.name(),
                definition.name()
            );
        }
    }
}

#[test]
fn every_builtin_except_script_declares_ports() {
    let registry = builtin_registry();
    for definition in registry.definitions() {
        if definition.name() == names::SCRIPT {
            assert!(
                definition.ports().is_empty(),
                "script declares its ports in YAML, not on the primitive"
            );
            continue;
        }
        assert!(
            !definition.ports().is_empty(),
            "builtin '{}' must declare its ports",
            definition.name()
        );
    }
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
