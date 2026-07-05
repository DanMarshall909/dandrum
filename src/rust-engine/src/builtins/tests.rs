use builtin_ports as bp;
use module_types::*;
use super::*;
use crate::graph::{PortDirection, SignalType};
use SignalType::{Audio, Control, Event};
use ParameterValueType::{Integer, Text};

#[test]
fn registry_stores_and_finds_module_definitions_by_type() {
    let definition = BuildDefinition();
    let registry = BuiltInModuleRegistry::from_definitions(vec![definition]);

    let gain = registry
        .get(GAIN)
        .expect("gain definition should be registered");

    assert_eq!(gain.module_type(), GAIN);
    assert_eq!(gain.inputs()[0].name(), bp::AUDIO_IN);
    assert_eq!(gain.inputs()[1].signal_type(), Control);
    assert_eq!(gain.outputs()[0].name(), bp::AUDIO_OUT);
}

fn BuildDefinition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(GAIN)
        .with_input(Port::input(bp::AUDIO_IN, Audio))
        .with_input(Port::input(bp::GAIN, Control))
        .with_output(Port::output(bp::AUDIO_OUT, Audio))
}

#[test]
fn registry_returns_none_for_unknown_module_type() {
    let registry = BuiltInModuleRegistry::from_definitions(Vec::new());

    assert_eq!(registry.get("missing"), None);
}

#[test]
fn initialized_registry_contains_midi_input_and_audio_output_definitions() {
    let registry = BuiltInModuleRegistry::new();

    let midi_input = registry
        .get(MIDI_INPUT)
        .expect("midi input should be built in");
    assert_eq!(midi_input.outputs()[0].name(), bp::EVENTS);
    assert_eq!(midi_input.outputs()[0].signal_type(), Event);

    let audio_output = registry
        .get(AUDIO_OUTPUT)
        .expect("audio output should be built in");
    assert_eq!(audio_output.inputs()[0].name(), bp::LEFT);
    assert_eq!(audio_output.inputs()[0].signal_type(), Audio);
    assert_eq!(audio_output.inputs()[1].name(), bp::RIGHT);
    assert_eq!(audio_output.inputs()[1].signal_type(), Audio);
    assert!(audio_output.outputs().is_empty());
}

#[test]
fn initialized_registry_contains_synthesis_control_and_mixer_definitions() {
    let registry = BuiltInModuleRegistry::new();

    let oscillator = registry
        .get(OSCILLATOR)
        .expect("oscillator should be built in");
    assert_has_input(oscillator, bp::PITCH, Control);
    assert_has_output(oscillator, bp::AUDIO, Audio);

    let gain = registry
        .get(GAIN)
        .expect("gain should be built in");
    assert_has_input(gain, bp::AUDIO_IN, Audio);
    assert_has_input(gain, bp::GAIN, Control);
    assert_has_output(gain, bp::AUDIO_OUT, Audio);

    let audio_mixer = registry
        .get(AUDIO_MIXER)
        .expect("audio mixer should be built in");
    assert_has_mixing_input(audio_mixer, bp::INPUTS, Audio);
    assert_has_output(audio_mixer, bp::MIX, Audio);

    let control_mixer = registry
        .get(CONTROL_MIXER)
        .expect("control mixer should be built in");
    assert_has_mixing_input(control_mixer, bp::INPUTS, Control);
    assert_has_output(control_mixer, bp::SUM, Control);

    let adsr = registry
        .get(ADSR)
        .expect("ADSR should be built in");
    assert_has_input(adsr, bp::GATE, Event);
    assert_has_input(adsr, bp::ATTACK, Control);
    assert_has_input(adsr, bp::DECAY, Control);
    assert_has_input(adsr, bp::SUSTAIN, Control);
    assert_has_input(adsr, bp::RELEASE, Control);
    assert_has_output(adsr, bp::VALUE, Control);

    let lfo = registry
        .get(LFO)
        .expect("LFO should be built in");
    assert_has_input(lfo, bp::RATE, Control);
    assert_has_output(lfo, bp::VALUE, Control);

    let filter = registry
        .get(FILTER)
        .expect("filter should be built in");
    assert_has_input(filter, bp::AUDIO_IN, Audio);
    assert_has_input(filter, bp::CUTOFF, Control);
    assert_has_input(filter, bp::RESONANCE, Control);
    assert_has_input(filter, bp::GAIN, Control);
    assert_has_output(filter, bp::AUDIO_OUT, Audio);
}

#[test]
fn initialized_registry_contains_delay_definitions_with_feedback_boundaries() {
    let registry = BuiltInModuleRegistry::new();

    let one_sample_delay = registry
        .get(AUDIO_DELAY_ONE_SAMPLE)
        .expect("one-sample audio delay should be built in");
    assert_has_input(one_sample_delay, bp::AUDIO_IN, Audio);
    assert_has_output(
        one_sample_delay,
        bp::AUDIO_OUT,
        Audio,
    );
    assert_eq!(one_sample_delay.feedback_boundaries(), &[Audio]);

    let block_delay = registry
        .get(BLOCK_DELAY)
        .expect("block delay should be built in");
    assert_has_input(block_delay, bp::AUDIO_IN, Audio);
    assert_has_output(block_delay, bp::AUDIO_OUT, Audio);
    assert_eq!(block_delay.feedback_boundaries(), &[Audio]);

    let control_delay = registry
        .get(CONTROL_DELAY)
        .expect("control delay should be built in");
    assert_has_input(control_delay, bp::VALUE, Control);
    assert_has_output(control_delay, bp::VALUE, Control);
    assert_eq!(control_delay.feedback_boundaries(), &[Control]);
}

#[test]
fn built_in_module_tests_inspect_port_directions_and_feedback_boundaries() {
    let registry = BuiltInModuleRegistry::new();

    for module_type in [
        MIDI_INPUT,
        AUDIO_OUTPUT,
        OSCILLATOR,
        GAIN,
        AUDIO_MIXER,
        CONTROL_MIXER,
        ADSR,
        LFO,
        FILTER,
        AUDIO_DELAY_ONE_SAMPLE,
        BLOCK_DELAY,
        CONTROL_DELAY,
        SCRIPT,
        SAMPLER,
        NOTE_TO_RATE,
        DYNAMICS_PROCESSOR,
        SATURATOR,
        CONVOLUTION,
        ECHO,
        REVERB,
        FREQUENCY_SPLITTER,
        SPECTRAL_PROCESSOR,
        EVENT_FILTER,
    ] {
        let definition = registry
            .get(module_type)
            .unwrap_or_else(|| panic!("{module_type} should be built in"));

        for input in definition.inputs() {
            assert_eq!(input.direction(), PortDirection::Input);
        }

        for output in definition.outputs() {
            assert_eq!(output.direction(), PortDirection::Output);
        }
    }

    assert_eq!(
        registry
            .get(AUDIO_DELAY_ONE_SAMPLE)
            .expect("one-sample delay should be built in")
            .feedback_boundaries(),
        &[Audio]
    );
    assert_eq!(
        registry
            .get(CONTROL_DELAY)
            .expect("control delay should be built in")
            .feedback_boundaries(),
        &[Control]
    );
}

#[test]
fn event_filter_definition_exposes_event_ports_selector_metadata_defaults_and_example() {
    let registry = BuiltInModuleRegistry::new();

    let event_filter = registry
        .get(EVENT_FILTER)
        .expect("event_filter should be built in");

    assert_has_input(event_filter, bp::EVENTS_IN, Event);
    assert_has_output(event_filter, bp::EVENTS_OUT, Event);
    assert!(
        event_filter
            .inputs()
            .iter()
            .all(|port| port.signal_type() == Event)
    );
    assert!(
        event_filter
            .outputs()
            .iter()
            .all(|port| port.signal_type() == Event)
    );
    assert!(event_filter.feedback_boundaries().is_empty());
    assert!(event_filter.examples().iter().any(|example| {
        example.contains(EVENT_FILTER)
            && example.contains(EVENT_FILTER_SELECTOR_PARAMETER)
            && example.contains(EVENT_FILTER_NOTE_PARAMETER)
    }));

    let selector = event_filter
        .parameters()
        .iter()
        .find(|parameter| parameter.name() == EVENT_FILTER_SELECTOR_PARAMETER)
        .expect("event_filter should declare selector metadata");
    assert_eq!(selector.value_type(), Text);
    assert_eq!(selector.default(), Some(EVENT_FILTER_SELECTOR_DEFAULT));
    assert_eq!(
        selector.enum_values(),
        Some(&[EVENT_FILTER_NOTE_SELECTOR.to_string()][..])
    );

    let note = event_filter
        .parameters()
        .iter()
        .find(|parameter| parameter.name() == EVENT_FILTER_NOTE_PARAMETER)
        .expect("event_filter should declare note metadata");
    assert_eq!(note.value_type(), Integer);
    assert_eq!(note.range(), Some((0.0, 127.0)));
    assert!(note.realtime_note().is_some());
}

#[test]
fn initialized_registry_contains_script_definition_with_yaml_declared_ports() {
    let registry = BuiltInModuleRegistry::new();

    let script = registry
        .get(SCRIPT)
        .expect("script should be built in");

    assert!(script.inputs().is_empty());
    assert!(script.outputs().is_empty());
    assert!(script.feedback_boundaries().is_empty());
}

#[test]
fn initialized_registry_contains_sampler_definition() {
    let registry = BuiltInModuleRegistry::new();

    let sampler = registry
        .get(SAMPLER)
        .expect("sampler should be built in");

    assert_has_input(sampler, bp::TRIGGER, Event);
    assert_has_input(sampler, bp::RATE, Control);
    assert_has_input(sampler, bp::START, Control);
    assert_has_input(sampler, bp::LOOP_ENABLED, Control);
    assert_has_input(sampler, bp::LOOP_START, Control);
    assert_has_input(sampler, bp::LOOP_END, Control);
    assert_has_output(sampler, bp::AUDIO, Audio);
}

#[test]
fn initialized_registry_contains_note_to_rate_definition() {
    let registry = BuiltInModuleRegistry::new();

    let note_to_rate = registry
        .get(NOTE_TO_RATE)
        .expect("note_to_rate should be built in");

    assert_has_input(note_to_rate, bp::EVENTS, Event);
    assert_has_output(note_to_rate, bp::RATE, Control);
}

#[test]
fn initialized_registry_contains_envelope_follower_and_curve_mapper_definitions() {
    let registry = BuiltInModuleRegistry::new();

    let follower = registry
        .get(ENVELOPE_FOLLOWER)
        .expect("envelope_follower should be built in");
    assert_has_input(follower, bp::AUDIO_IN, Audio);
    assert_has_input(follower, bp::ATTACK, Control);
    assert_has_input(follower, bp::RELEASE, Control);
    assert_has_input(follower, bp::AMOUNT, Control);
    assert_has_input(follower, bp::OFFSET, Control);
    assert_has_input(follower, bp::INVERT, Control);
    assert_has_output(follower, bp::VALUE, Control);

    let mode = follower
        .parameters()
        .iter()
        .find(|parameter| parameter.name() == DETECTION_MODE_PARAMETER)
        .expect("envelope_follower should expose detection mode metadata");
    assert_eq!(mode.value_type(), Text);
    assert_eq!(mode.default(), Some(DETECTION_MODE_PEAK));
    assert_eq!(
        mode.enum_values(),
        Some(
            &[
                DETECTION_MODE_PEAK.to_string(),
                DETECTION_MODE_RMS.to_string()
            ][..]
        )
    );

    let mapper = registry
        .get(CURVE_MAPPER)
        .expect("curve_mapper should be built in");
    assert_has_input(mapper, bp::VALUE, Control);
    assert_has_input(mapper, bp::AMOUNT, Control);
    assert_has_input(mapper, bp::BIAS, Control);
    assert_has_input(mapper, bp::SCALE, Control);
    assert_has_input(mapper, bp::OFFSET, Control);
    assert_has_output(mapper, bp::VALUE, Control);

    let curve = mapper
        .parameters()
        .iter()
        .find(|parameter| parameter.name() == CURVE_PARAMETER)
        .expect("curve_mapper should expose curve metadata");
    assert_eq!(curve.value_type(), Text);
    assert_eq!(curve.default(), Some(CURVE_LINEAR));
    assert_eq!(
        curve.enum_values(),
        Some(
            &[
                CURVE_LINEAR.to_string(),
                CURVE_EXPONENTIAL.to_string(),
                CURVE_LOGARITHMIC.to_string(),
                CURVE_S_CURVE.to_string(),
                CURVE_SOFT_CLIP.to_string(),
                CURVE_HARD_CLIP.to_string(),
                CURVE_STEP.to_string(),
            ][..]
        )
    );

    let steps = mapper
        .parameters()
        .iter()
        .find(|parameter| parameter.name() == STEPS_PARAMETER)
        .expect("curve_mapper should expose steps metadata");
    assert_eq!(steps.value_type(), Integer);
    assert_eq!(steps.default(), Some("4"));
    assert_eq!(steps.range(), Some((2.0, 128.0)));
}

fn assert_has_input(definition: &BuiltInModuleDefinition, name: &str, signal_type: SignalType) {
    assert!(
        definition
            .inputs()
            .iter()
            .any(|port| port.name() == name && port.signal_type() == signal_type)
    );
}

fn assert_has_mixing_input(
    definition: &BuiltInModuleDefinition,
    name: &str,
    signal_type: SignalType,
) {
    assert!(definition.inputs().iter().any(|port| {
        port.name() == name && port.signal_type() == signal_type && port.accepts_multiple_sources()
    }));
}

#[test]
fn echo_definition_has_correct_ports() {
    let registry = BuiltInModuleRegistry::new();

    let echo = registry
        .get(ECHO)
        .expect("echo should be built in");

    assert_has_input(echo, bp::AUDIO_IN_L, Audio);
    assert_has_input(echo, bp::AUDIO_IN_R, Audio);
    assert_has_output(echo, bp::AUDIO_OUT_L, Audio);
    assert_has_output(echo, bp::AUDIO_OUT_R, Audio);
    assert_has_input(echo, bp::TIME_LEFT_MS, Control);
    assert_has_input(echo, bp::TIME_RIGHT_MS, Control);
    assert_has_input(echo, bp::FEEDBACK, Control);
    assert_has_input(echo, bp::DAMPING_CUTOFF, Control);
    assert_has_input(echo, bp::WET, Control);
    assert_has_input(echo, bp::DRY, Control);
    assert_has_input(echo, bp::SYNC_DIVISION, Control);
    assert_has_input(echo, bp::PING_PONG, Control);
}

#[test]
fn reverb_definition_has_correct_ports() {
    let registry = BuiltInModuleRegistry::new();

    let reverb = registry
        .get(REVERB)
        .expect("reverb should be built in");

    assert_has_input(reverb, bp::AUDIO_IN_L, Audio);
    assert_has_input(reverb, bp::AUDIO_IN_R, Audio);
    assert_has_output(reverb, bp::AUDIO_OUT_L, Audio);
    assert_has_output(reverb, bp::AUDIO_OUT_R, Audio);
    assert_has_input(reverb, bp::DECAY_TIME, Control);
    assert_has_input(reverb, bp::ROOM_SIZE, Control);
    assert_has_input(reverb, bp::PRE_DELAY, Control);
    assert_has_input(reverb, bp::DAMPING, Control);
    assert_has_input(reverb, bp::DIFFUSION, Control);
    assert_has_input(reverb, bp::STEREO_WIDTH, Control);
    assert_has_input(reverb, bp::WET, Control);
    assert_has_input(reverb, bp::DRY, Control);
}

#[test]
fn filter_definition_has_parameter_metadata() {
    let registry = BuiltInModuleRegistry::new();
    let filter = registry
        .get(FILTER)
        .expect("filter should be built in");

    let params = filter.parameters();
    assert!(!params.is_empty(), "filter should have parameter metadata");

    let algorithm = params
        .iter()
        .find(|p| p.name() == "algorithm")
        .expect("filter should have algorithm parameter");
    assert_eq!(algorithm.value_type(), Text);
    assert_eq!(algorithm.default(), Some("moog"));
    let enum_vals = algorithm
        .enum_values()
        .expect("algorithm should have enum values");
    assert!(enum_vals.contains(&"moog".to_string()));
    assert!(enum_vals.contains(&"biquad".to_string()));
    assert!(enum_vals.contains(&"comb".to_string()));

    let mode = params
        .iter()
        .find(|p| p.name() == "mode")
        .expect("filter should have mode parameter");
    assert_eq!(mode.default(), Some("lowpass"));

    let comb_type = params
        .iter()
        .find(|p| p.name() == "comb_type")
        .expect("filter should have comb_type parameter");
    assert_eq!(comb_type.default(), Some("feedback"));
}

#[test]
fn sampler_definition_has_asset_parameter_metadata() {
    let registry = BuiltInModuleRegistry::new();
    let sampler = registry
        .get(SAMPLER)
        .expect("sampler should be built in");

    let params = sampler.parameters();
    let asset = params
        .iter()
        .find(|p| p.name() == "asset")
        .expect("sampler should have asset parameter");
    assert_eq!(asset.value_type(), Text);
    assert!(asset.description().is_some());
    assert!(asset.realtime_note().is_some());
}

#[test]
fn unknown_parameter_not_in_metadata_detected() {
    let registry = BuiltInModuleRegistry::new();
    let filter = registry
        .get(FILTER)
        .expect("filter should be built in");

    let known_params: Vec<&str> = filter.parameters().iter().map(|p| p.name()).collect();
    assert!(known_params.contains(&"algorithm"));
    assert!(known_params.contains(&"mode"));
    assert!(known_params.contains(&"comb_type"));
    assert!(!known_params.contains(&"nonexistent"));
}

#[test]
fn parameter_metadata_queryable_without_renderer() {
    let registry = BuiltInModuleRegistry::new();

    let filter = registry
        .get(FILTER)
        .expect("filter should be built in");
    let sampler = registry
        .get(SAMPLER)
        .expect("sampler should be built in");
    let oscillator = registry
        .get(OSCILLATOR)
        .expect("oscillator should be built in");

    assert!(!filter.parameters().is_empty());
    assert!(!sampler.parameters().is_empty());
    assert!(oscillator.parameters().is_empty());
}

fn assert_has_output(definition: &BuiltInModuleDefinition, name: &str, signal_type: SignalType) {
    assert!(
        definition
            .outputs()
            .iter()
            .any(|port| port.name() == name && port.signal_type() == signal_type)
    );
}

#[test]
fn discovery_can_enumerate_all_module_types_without_constructing_renderer() {
    let registry = BuiltInModuleRegistry::new();
    let types: Vec<&str> = registry.module_types().collect();

    assert!(types.contains(&OSCILLATOR));
    assert!(types.contains(&GAIN));
    assert!(types.contains(&NOISE));
    assert!(types.contains(&MULTIPLY));
    assert!(types.contains(&SCRIPT));
    assert!(types.contains(&ADSR));
    assert!(!types.contains(&"nonexistent_module"));
}

#[test]
fn discovery_can_query_port_and_parameter_metadata_without_constructing_renderer() {
    let registry = BuiltInModuleRegistry::new();

    let osc = registry.get(OSCILLATOR).unwrap();
    assert_eq!(osc.module_type(), OSCILLATOR);
    assert_eq!(osc.inputs().len(), 1);
    assert_eq!(osc.inputs()[0].name(), bp::PITCH);
    assert_eq!(osc.inputs()[0].signal_type(), Control);
    assert_eq!(osc.inputs()[0].direction(), PortDirection::Input);
    assert_eq!(osc.outputs().len(), 1);
    assert_eq!(osc.outputs()[0].name(), bp::AUDIO);
    assert_eq!(osc.outputs()[0].signal_type(), Audio);
    assert_eq!(osc.outputs()[0].direction(), PortDirection::Output);
    assert!(osc.parameters().is_empty());

    let filter = registry.get(FILTER).unwrap();
    let params: Vec<&str> = filter.parameters().iter().map(|p| p.name()).collect();
    assert_eq!(params.len(), 3);
    assert!(params.contains(&"algorithm"));
    assert!(params.contains(&"mode"));
    assert!(params.contains(&"comb_type"));

    let algorithm_param = filter
        .parameters()
        .iter()
        .find(|p| p.name() == "algorithm")
        .unwrap();
    assert_eq!(algorithm_param.value_type(), Text);
    assert!(algorithm_param.enum_values().is_some());
}

#[test]
fn discovery_reports_module_category() {
    let registry = BuiltInModuleRegistry::new();

    let osc = registry.get(OSCILLATOR).unwrap();
    assert_eq!(osc.module_category(), ModuleCategory::Primitive);

    let script = registry.get(SCRIPT).unwrap();
    assert_eq!(script.module_category(), ModuleCategory::Script);

    let filter = registry.get(FILTER).unwrap();
    assert_eq!(filter.module_category(), ModuleCategory::Primitive);
}

#[test]
fn discovery_does_not_require_render_path_or_audio_construction() {
    let registry = BuiltInModuleRegistry::new();

    for module_type in registry.module_types() {
        let definition = registry.get(module_type).unwrap();
        assert_eq!(definition.module_type(), module_type);
        for port in definition.inputs() {
            assert!(!port.name().is_empty());
        }
        for port in definition.outputs() {
            assert!(!port.name().is_empty());
        }
    }
}
