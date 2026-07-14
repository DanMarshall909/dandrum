use std::collections::BTreeMap;

use super::*;
use crate::builtins::{
    CURVE_EXPONENTIAL, CURVE_HARD_CLIP, CURVE_LINEAR, CURVE_LOGARITHMIC, CURVE_PARAMETER,
    CURVE_S_CURVE, CURVE_SOFT_CLIP, CURVE_STEP, DETECTION_MODE_PARAMETER, DETECTION_MODE_PEAK,
    DETECTION_MODE_RMS, DYNAMICS_DETECTION_PARAMETER, DYNAMICS_MODE_LEVEL, DYNAMICS_MODE_PARAMETER,
    DYNAMICS_MODE_TRANSIENT, DYNAMICS_TOPOLOGY_FEEDBACK, DYNAMICS_TOPOLOGY_FEEDFORWARD,
    DYNAMICS_TOPOLOGY_PARAMETER, EVENT_FILTER_NOTE_PARAMETER, EVENT_FILTER_NOTE_SELECTOR,
    EVENT_FILTER_SELECTOR_DEFAULT, EVENT_FILTER_SELECTOR_PARAMETER, FILTER_ALGORITHM_BIQUAD,
    FILTER_ALGORITHM_COMB, FILTER_ALGORITHM_MOOG, FILTER_ALGORITHM_PARAMETER,
    FILTER_COMB_TYPE_PARAMETER, FILTER_MODE_HIGHPASS, FILTER_MODE_LOWPASS, FILTER_MODE_PARAMETER,
    FILTER_MODE_PEAKING, INTERPOLATION_CUBIC, INTERPOLATION_LINEAR, INTERPOLATION_PARAMETER,
    NOISE_DEFAULT_SEED, NOISE_SEED_PARAMETER, SCRIPT_LANGUAGE_PARAMETER, SCRIPT_LANGUAGE_RHAI,
    SCRIPT_SOURCE_PARAMETER, SPECTRAL_DEFAULT_FFT_SIZE, SPECTRAL_FFT_SIZE_PARAMETER,
    SPECTRAL_MODE_GATE, SPECTRAL_MODE_PARAMETER, SPECTRAL_MODE_PASSTHROUGH, SPECTRAL_WINDOW_HANN,
    SPECTRAL_WINDOW_PARAMETER, STEPS_PARAMETER, WAVEFORM_PARAMETER, WAVEFORM_SAW, WAVEFORM_SINE,
    WAVEFORM_SQUARE, WAVEFORM_TRIANGLE,
};
use crate::convolution::Convolution;
use crate::diagnostics::error_codes;
use crate::kernel::{
    GraphDefinition, Node, NodeId, ResourceKind, ResourceOrigin, ResourceRef, StaticArg,
    StaticType, StaticValue,
};

/// Every builtin the kernel registry must declare.
const EXPECTED: [&str; 33] = [
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
    names::COMPENSATION_DELAY,
    POLY_DEFINITION,
];

/// Resolve a definition's latency against its declared static-parameter
/// defaults, exactly as compilation would for an instance with no overrides.
fn latency_with_defaults(definition: &GraphDefinition) -> u32 {
    let args: BTreeMap<String, StaticValue> = definition
        .static_params()
        .iter()
        .filter_map(|param| {
            param
                .default()
                .map(|value| (param.name().to_string(), value.clone()))
        })
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
fn poly_declares_structural_static_arguments_and_note_event_input() {
    let registry = builtin_registry();
    let poly = registry.get(POLY_DEFINITION).expect("poly declared");

    assert_eq!(
        static_param_of(poly, POLY_WRAPPED_DEFINITION_PARAM).static_type(),
        StaticType::String
    );
    assert_eq!(
        static_param_of(poly, POLY_MAX_VOICES_PARAM).static_type(),
        StaticType::Int
    );
    assert_eq!(
        static_param_of(poly, POLY_ALLOCATION_PARAM).allowed_values(),
        [POLY_ALLOCATION_OLDEST_STEAL, POLY_ALLOCATION_REJECT_NEW]
    );
    let notes = port_of(poly, POLY_NOTE_EVENTS_INPUT, PortDirection::Input);
    assert_eq!(notes.signal_type(), SignalType::Event);
    assert_eq!(notes.channels(), &ChannelCount::Literal(1));
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
fn compensation_delay_latency_equals_its_resolved_length() {
    let registry = builtin_registry();
    let delay = registry
        .get(names::COMPENSATION_DELAY)
        .expect("compensation delay declared");

    let delay_samples = delay
        .static_params()
        .iter()
        .find(|parameter| parameter.name() == DELAY_SAMPLES_PARAM)
        .expect("delay length static parameter should be declared");
    assert_eq!(delay_samples.default(), None);

    for samples in [1, 6, 257] {
        let args = BTreeMap::from([(DELAY_SAMPLES_PARAM.to_string(), StaticValue::Int(samples))]);
        assert_eq!(delay.latency().resolve(&args), samples as u32);
    }
}

// --- 3.1 Port and control-default declarations ---------------------------

use crate::graph::builtin_ports as ports;
use crate::graph::{PortDirection, SignalType};
use crate::kernel::Port;

fn port_of<'a>(definition: &'a GraphDefinition, name: &str, direction: PortDirection) -> &'a Port {
    definition
        .ports()
        .iter()
        .find(|port| port.name() == name && port.direction() == direction)
        .unwrap_or_else(|| {
            panic!(
                "'{}' declares port '{name}' ({direction:?})",
                definition.name()
            )
        })
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
    let default = level
        .control_default()
        .expect("gain carries a control default");
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
fn echo_and_reverb_expose_constrained_multichannel_ports() {
    let registry = builtin_registry();
    for name in [names::ECHO, names::REVERB] {
        let definition = registry
            .get(name)
            .unwrap_or_else(|| panic!("{name} declared"));
        assert_eq!(
            static_param_of(definition, CHANNELS_PARAM).default(),
            Some(&StaticValue::Int(2))
        );
        assert_eq!(
            static_param_of(definition, CHANNELS_PARAM).allowed_values(),
            &["1".to_string(), "2".to_string()]
        );
        assert_eq!(
            port_of(definition, ports::AUDIO_IN, PortDirection::Input).channels(),
            &ChannelCount::param(CHANNELS_PARAM)
        );
        assert_eq!(
            port_of(definition, ports::AUDIO_OUT, PortDirection::Output).channels(),
            &ChannelCount::param(CHANNELS_PARAM)
        );
        assert_eq!(
            definition
                .ports()
                .iter()
                .filter(|port| port.signal_type() == SignalType::Audio)
                .count(),
            2
        );
    }
}

#[test]
fn echo_and_reverb_reject_channel_counts_wider_than_stereo() {
    let registry = builtin_registry();
    for name in [names::ECHO, names::REVERB] {
        let root = GraphDefinition::new("root").with_node(
            Node::new(NodeId::new("effect"), name)
                .with_static_arg(CHANNELS_PARAM, StaticArg::Literal(StaticValue::Int(6))),
        );

        let validation = root.validate(&registry);
        let diagnostic = validation
            .diagnostics()
            .errors()
            .find(|diagnostic| {
                diagnostic.error_code() == error_codes::KERNEL_STATIC_ARGUMENT_UNSUPPORTED_VALUE
            })
            .unwrap_or_else(|| panic!("{name} should reject six channels"));
        assert_eq!(diagnostic.module_id(), Some("effect"));
        assert_eq!(diagnostic.expected(), Some("1, 2"));
        assert_eq!(diagnostic.actual(), Some("6"));
    }
}

#[test]
fn non_polymorphic_builtin_ports_remain_mono() {
    let registry = builtin_registry();
    for definition in registry.definitions() {
        for port in definition.ports() {
            if port.channels() != &crate::kernel::ChannelCount::Param(CHANNELS_PARAM.to_string()) {
                assert_eq!(
                    port.channels(),
                    &crate::kernel::ChannelCount::Literal(1),
                    "non-polymorphic port '{}' of '{}' remains mono",
                    port.name(),
                    definition.name()
                );
            }
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
            || definition.name() == names::COMPENSATION_DELAY
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

fn static_param_of<'a>(definition: &'a GraphDefinition, name: &str) -> &'a StaticParam {
    definition
        .static_params()
        .iter()
        .find(|param| param.name() == name)
        .unwrap_or_else(|| panic!("'{}' declares static parameter '{name}'", definition.name()))
}

fn assert_enum_param(
    definition: &GraphDefinition,
    name: &str,
    default: &str,
    allowed_values: &[&str],
) {
    let param = static_param_of(definition, name);
    assert_eq!(param.static_type(), StaticType::Enum);
    assert_eq!(
        param.default(),
        Some(&StaticValue::Enum(default.to_string()))
    );
    assert_eq!(
        param.allowed_values(),
        allowed_values,
        "enum values of '{}.{name}'",
        definition.name()
    );
}

#[test]
fn builtins_declare_all_non_resource_construction_parameters() {
    let registry = builtin_registry();

    assert_enum_param(
        registry.get(names::OSCILLATOR).unwrap(),
        WAVEFORM_PARAMETER,
        WAVEFORM_SAW,
        &[
            WAVEFORM_SAW,
            WAVEFORM_SINE,
            WAVEFORM_TRIANGLE,
            WAVEFORM_SQUARE,
        ],
    );

    let filter = registry.get(names::FILTER).unwrap();
    assert_enum_param(
        filter,
        FILTER_ALGORITHM_PARAMETER,
        FILTER_ALGORITHM_MOOG,
        &[
            FILTER_ALGORITHM_MOOG,
            FILTER_ALGORITHM_BIQUAD,
            FILTER_ALGORITHM_COMB,
        ],
    );
    assert_enum_param(
        filter,
        FILTER_MODE_PARAMETER,
        FILTER_MODE_LOWPASS,
        &[
            FILTER_MODE_LOWPASS,
            FILTER_MODE_HIGHPASS,
            FILTER_MODE_PEAKING,
        ],
    );
    assert_enum_param(
        filter,
        FILTER_COMB_TYPE_PARAMETER,
        DYNAMICS_TOPOLOGY_FEEDBACK,
        &[DYNAMICS_TOPOLOGY_FEEDBACK, DYNAMICS_TOPOLOGY_FEEDFORWARD],
    );

    let script = registry.get(names::SCRIPT).unwrap();
    assert_enum_param(
        script,
        SCRIPT_LANGUAGE_PARAMETER,
        SCRIPT_LANGUAGE_RHAI,
        &[SCRIPT_LANGUAGE_RHAI],
    );
    assert_eq!(
        static_param_of(script, SCRIPT_SOURCE_PARAMETER).static_type(),
        StaticType::String
    );
    assert_eq!(
        static_param_of(script, SCRIPT_SOURCE_PARAMETER).default(),
        None
    );

    let event_filter = registry.get(names::EVENT_FILTER).unwrap();
    assert_enum_param(
        event_filter,
        EVENT_FILTER_SELECTOR_PARAMETER,
        EVENT_FILTER_SELECTOR_DEFAULT,
        &[EVENT_FILTER_NOTE_SELECTOR],
    );
    assert_eq!(
        static_param_of(event_filter, EVENT_FILTER_NOTE_PARAMETER).static_type(),
        StaticType::Int
    );
    assert_eq!(
        static_param_of(event_filter, EVENT_FILTER_NOTE_PARAMETER).default(),
        None
    );

    let dynamics = registry.get(names::DYNAMICS_PROCESSOR).unwrap();
    assert_enum_param(
        dynamics,
        DYNAMICS_MODE_PARAMETER,
        DYNAMICS_MODE_LEVEL,
        &[DYNAMICS_MODE_LEVEL, DYNAMICS_MODE_TRANSIENT],
    );
    assert_enum_param(
        dynamics,
        DYNAMICS_DETECTION_PARAMETER,
        DETECTION_MODE_PEAK,
        &[DETECTION_MODE_PEAK, DETECTION_MODE_RMS],
    );
    assert_enum_param(
        dynamics,
        DYNAMICS_TOPOLOGY_PARAMETER,
        DYNAMICS_TOPOLOGY_FEEDFORWARD,
        &[DYNAMICS_TOPOLOGY_FEEDFORWARD, DYNAMICS_TOPOLOGY_FEEDBACK],
    );

    for definition_name in [names::ECHO, names::REVERB] {
        assert_enum_param(
            registry.get(definition_name).unwrap(),
            INTERPOLATION_PARAMETER,
            INTERPOLATION_LINEAR,
            &[INTERPOLATION_LINEAR, INTERPOLATION_CUBIC],
        );
    }

    let spectral = registry.get(names::SPECTRAL_PROCESSOR).unwrap();
    assert_eq!(
        static_param_of(spectral, SPECTRAL_FFT_SIZE_PARAMETER).default(),
        Some(&StaticValue::Int(SPECTRAL_DEFAULT_FFT_SIZE as i64))
    );
    assert_enum_param(
        spectral,
        SPECTRAL_MODE_PARAMETER,
        SPECTRAL_MODE_GATE,
        &[SPECTRAL_MODE_GATE, SPECTRAL_MODE_PASSTHROUGH],
    );
    assert_enum_param(
        spectral,
        SPECTRAL_WINDOW_PARAMETER,
        SPECTRAL_WINDOW_HANN,
        &[SPECTRAL_WINDOW_HANN],
    );

    assert_eq!(
        static_param_of(registry.get(names::NOISE).unwrap(), NOISE_SEED_PARAMETER).default(),
        Some(&StaticValue::Int(NOISE_DEFAULT_SEED as i64))
    );
    assert_enum_param(
        registry.get(names::ENVELOPE_FOLLOWER).unwrap(),
        DETECTION_MODE_PARAMETER,
        DETECTION_MODE_PEAK,
        &[DETECTION_MODE_PEAK, DETECTION_MODE_RMS],
    );

    let curve_mapper = registry.get(names::CURVE_MAPPER).unwrap();
    assert_enum_param(
        curve_mapper,
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
    );
    assert_eq!(
        static_param_of(curve_mapper, STEPS_PARAMETER).static_type(),
        StaticType::Int
    );
    assert_eq!(
        static_param_of(curve_mapper, STEPS_PARAMETER).default(),
        Some(&StaticValue::Int(
            crate::curve_mapper::CurveMapper::DEFAULT_STEPS as i64
        ))
    );
    assert_enum_param(
        registry.get(names::DECAY).unwrap(),
        CURVE_PARAMETER,
        CURVE_EXPONENTIAL,
        &[CURVE_LINEAR, CURVE_EXPONENTIAL],
    );
}

#[test]
fn sampler_and_convolution_declare_typed_resource_arguments() {
    let registry = builtin_registry();
    let sample = static_param_of(registry.get(names::SAMPLER).unwrap(), SAMPLE_RESOURCE_PARAM);
    let impulse_response = static_param_of(
        registry.get(names::CONVOLUTION).unwrap(),
        IMPULSE_RESPONSE_RESOURCE_PARAM,
    );

    assert_eq!(
        sample.static_type(),
        StaticType::Resource(ResourceKind::Sample)
    );
    assert_eq!(sample.default(), None);
    assert_eq!(
        impulse_response.static_type(),
        StaticType::Resource(ResourceKind::ImpulseResponse)
    );
    assert_eq!(impulse_response.default(), None);
}

#[test]
fn convolution_rejects_a_sample_resource_before_preparation() {
    let registry = builtin_registry();
    let root = GraphDefinition::new("wrong-resource-kind").with_node(
        Node::new(NodeId::new("convolution"), names::CONVOLUTION).with_static_arg(
            IMPULSE_RESPONSE_RESOURCE_PARAM,
            StaticArg::Literal(StaticValue::Resource(ResourceRef::new(
                ResourceKind::Sample,
                "not-an-ir.wav",
                ResourceOrigin::Document,
            ))),
        ),
    );

    let validation = root.validate(&registry);
    let diagnostic = validation.diagnostics().errors().next().unwrap();
    assert_eq!(
        diagnostic.error_code(),
        error_codes::KERNEL_RESOURCE_KIND_MISMATCH
    );
    assert_eq!(diagnostic.module_id(), Some("convolution"));
    assert_eq!(diagnostic.expected(), Some("impulse_response"));
    assert_eq!(diagnostic.actual(), Some("sample"));
}

#[test]
fn builtin_static_arguments_validate_through_graph_definition() {
    let registry = builtin_registry();
    let valid = GraphDefinition::new("valid_builtin_static_args")
        .with_node(
            Node::new(NodeId::new("osc"), names::OSCILLATOR).with_static_arg(
                WAVEFORM_PARAMETER,
                StaticArg::Literal(StaticValue::Enum(WAVEFORM_SINE.to_string())),
            ),
        )
        .with_node(
            Node::new(NodeId::new("script"), names::SCRIPT).with_static_arg(
                SCRIPT_SOURCE_PARAMETER,
                StaticArg::Literal(StaticValue::String("fn process(ctx) {}".to_string())),
            ),
        )
        .with_node(
            Node::new(NodeId::new("filter"), names::EVENT_FILTER).with_static_arg(
                EVENT_FILTER_NOTE_PARAMETER,
                StaticArg::Literal(StaticValue::Int(36)),
            ),
        );

    assert!(valid.validate(&registry).is_ok());

    for (node, expected_code) in [
        (
            Node::new(NodeId::new("unknown"), names::OSCILLATOR).with_static_arg(
                SCRIPT_SOURCE_PARAMETER,
                StaticArg::Literal(StaticValue::String(String::new())),
            ),
            error_codes::KERNEL_UNKNOWN_STATIC_ARGUMENT,
        ),
        (
            Node::new(NodeId::new("mismatched"), names::NOISE).with_static_arg(
                NOISE_SEED_PARAMETER,
                StaticArg::Literal(StaticValue::Enum(WAVEFORM_SAW.to_string())),
            ),
            error_codes::KERNEL_STATIC_ARGUMENT_TYPE_MISMATCH,
        ),
        (
            Node::new(NodeId::new("invalid_enum"), names::OSCILLATOR).with_static_arg(
                WAVEFORM_PARAMETER,
                StaticArg::Literal(StaticValue::Enum(SCRIPT_LANGUAGE_RHAI.to_string())),
            ),
            error_codes::KERNEL_STATIC_ARGUMENT_INVALID_ENUM_VALUE,
        ),
    ] {
        let validation = GraphDefinition::new("invalid_builtin_static_arg")
            .with_node(node)
            .validate(&registry);
        assert_eq!(validation.diagnostics().len(), 1);
        assert_eq!(
            validation.diagnostics().all()[0].error_code(),
            expected_code
        );
    }
}

// --- 3.3 Input multiplicity declarations ----------------------------------

#[test]
fn audio_mixer_inputs_port_is_summing() {
    let registry = builtin_registry();
    let mixer = registry
        .get(names::AUDIO_MIXER)
        .expect("audio_mixer declared");
    let inputs = port_of(mixer, ports::INPUTS, PortDirection::Input);
    assert_eq!(
        inputs.multiplicity(),
        crate::kernel::Multiplicity::Summing,
        "audio_mixer inputs must be summing"
    );
}

#[test]
fn control_mixer_inputs_port_is_summing() {
    let registry = builtin_registry();
    let mixer = registry
        .get(names::CONTROL_MIXER)
        .expect("control_mixer declared");
    let inputs = port_of(mixer, ports::INPUTS, PortDirection::Input);
    assert_eq!(
        inputs.multiplicity(),
        crate::kernel::Multiplicity::Summing,
        "control_mixer inputs must be summing"
    );
}

#[test]
fn non_mixer_builtin_inputs_are_single_source() {
    let registry = builtin_registry();
    for name in [names::GAIN, names::FILTER, names::OSCILLATOR] {
        let definition = registry
            .get(name)
            .unwrap_or_else(|| panic!("{name} declared"));
        for port in definition.ports() {
            if port.direction() == PortDirection::Input {
                assert_eq!(
                    port.multiplicity(),
                    crate::kernel::Multiplicity::SingleSource,
                    "port '{}' of '{}' defaults to single-source",
                    port.name(),
                    name
                );
            }
        }
    }
}

#[test]
fn generic_builtins_resolve_mono_stereo_and_six_channel_signal_ports() {
    let registry = builtin_registry();
    let cases = [
        (names::GAIN, ports::AUDIO_IN, ports::AUDIO_OUT),
        (names::AUDIO_MIXER, ports::INPUTS, ports::MIX),
        (names::FILTER, ports::AUDIO_IN, ports::AUDIO_OUT),
        (
            names::AUDIO_DELAY_ONE_SAMPLE,
            ports::AUDIO_IN,
            ports::AUDIO_OUT,
        ),
        (names::BLOCK_DELAY, ports::AUDIO_IN, ports::AUDIO_OUT),
        (names::COMPENSATION_DELAY, ports::AUDIO_IN, ports::AUDIO_OUT),
        (names::CONVOLUTION, ports::AUDIO_IN, ports::AUDIO_OUT),
        (names::FREQUENCY_SPLITTER, ports::AUDIO_IN, ports::LOW),
        (names::MULTIPLY, ports::AUDIO_IN, ports::AUDIO_OUT),
        (names::CONTROL_TO_AUDIO, ports::IN, ports::OUT),
    ];

    for channels in [1_i64, 2, 6] {
        for (definition, input, output) in cases {
            let mut node = Node::new(NodeId::new("processor"), definition).with_static_arg(
                CHANNELS_PARAM,
                StaticArg::Literal(StaticValue::Int(channels)),
            );
            if definition == names::COMPENSATION_DELAY {
                node = node
                    .with_static_arg(DELAY_SAMPLES_PARAM, StaticArg::Literal(StaticValue::Int(1)));
            } else if definition == names::CONVOLUTION {
                node = node.with_static_arg(
                    IMPULSE_RESPONSE_RESOURCE_PARAM,
                    StaticArg::Literal(StaticValue::Resource(ResourceRef::new(
                        ResourceKind::ImpulseResponse,
                        "unit-ir.wav",
                        ResourceOrigin::Document,
                    ))),
                );
            }
            let flattened = GraphDefinition::new("root")
                .with_node(node)
                .flatten(&registry)
                .unwrap_or_else(|diagnostics| panic!("{definition} {channels}ch: {diagnostics}"));
            let processor = &flattened.nodes()[0];
            assert_eq!(
                processor
                    .ports()
                    .iter()
                    .find(|port| port.name() == input)
                    .unwrap()
                    .channels(),
                channels as u32,
                "{definition}.{input}"
            );
            assert_eq!(
                processor
                    .ports()
                    .iter()
                    .find(|port| port.name() == output)
                    .unwrap()
                    .channels(),
                channels as u32,
                "{definition}.{output}"
            );
        }
    }
}
