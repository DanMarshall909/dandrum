use super::*;
use crate::graph::{PortDirection, SignalType};
use ParameterValueType::{Integer, Text};
use SignalType::{Audio, Control, Event};

macro_rules! assert_control_inputs {
    ($definition:expr, $($name:expr),+ $(,)?) => {
        $(assert_has_control_input($definition, $name);)+
    };
}

macro_rules! assert_audio_inputs {
    ($definition:expr, $($name:expr),+ $(,)?) => {
        $(assert_has_audio_input($definition, $name);)+
    };
}

macro_rules! assert_audio_outputs {
    ($definition:expr, $($name:expr),+ $(,)?) => {
        $(assert_has_audio_output($definition, $name);)+
    };
}

#[test]
fn registry_stores_and_finds_module_definitions_by_type() {
    let definition = build_definition();
    let registry = BuiltInModuleRegistry::from_definitions(vec![definition]);

    let gain = registry
        .get(module_types::GAIN)
        .expect("gain definition should be registered");

    assert_eq!(gain.module_type(), module_types::GAIN);
    assert_eq!(gain.inputs()[0].name(), AUDIO_IN);
    assert_eq!(gain.inputs()[1].signal_type(), Control);
    assert_eq!(gain.outputs()[0].name(), AUDIO_OUT);
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
    assert_eq!(midi_input.outputs()[0].name(), EVENTS);
    assert_eq!(midi_input.outputs()[0].signal_type(), Event);

    let audio_output = registry
        .get(AUDIO_OUTPUT)
        .expect("audio output should be built in");
    let output_inputs = audio_output.inputs();
    assert_eq!(output_inputs[0].name(), LEFT);
    assert_eq!(output_inputs[0].signal_type(), Audio);
    assert_eq!(output_inputs[1].name(), RIGHT);
    assert_eq!(output_inputs[1].signal_type(), Audio);
    assert!(audio_output.outputs().is_empty());
}

#[test]
fn initialized_registry_contains_synthesis_control_and_mixer_definitions() {
    let registry = BuiltInModuleRegistry::new();

    let oscillator = registry
        .get(OSCILLATOR)
        .expect("oscillator should be built in");
    assert_has_control_input(oscillator, PITCH);
    assert_has_audio_output(oscillator, AUDIO);

    let gain = registry
        .get(module_types::GAIN)
        .expect("gain should be built in");
    assert_has_audio_input(gain, AUDIO_IN);
    assert_has_control_input(gain, builtin_ports::GAIN);
    assert_has_audio_output(gain, AUDIO_OUT);

    let audio_mixer = registry
        .get(AUDIO_MIXER)
        .expect("audio mixer should be built in");
    assert_has_audio_mixing_input(audio_mixer, INPUTS);
    assert_has_audio_output(audio_mixer, MIX);

    let control_mixer = registry
        .get(CONTROL_MIXER)
        .expect("control mixer should be built in");
    assert_has_control_mixing_input(control_mixer, INPUTS);
    assert_has_control_output(control_mixer, SUM);

    let adsr = registry.get(ADSR).expect("ADSR should be built in");
    assert_has_event_input(adsr, GATE);
    assert_control_inputs!(adsr, ATTACK, DECAY, SUSTAIN, RELEASE,);
    assert_has_control_output(adsr, VALUE);

    let lfo = registry.get(LFO).expect("LFO should be built in");
    assert_has_control_input(lfo, RATE);
    assert_has_control_output(lfo, VALUE);

    let filter = registry.get(FILTER).expect("filter should be built in");
    assert_has_audio_input(filter, AUDIO_IN);
    assert_control_inputs!(filter, CUTOFF, RESONANCE, builtin_ports::GAIN,);
    assert_has_audio_output(filter, AUDIO_OUT);
}

#[test]
fn initialized_registry_contains_delay_definitions_with_feedback_boundaries() {
    let registry = BuiltInModuleRegistry::new();

    let one_sample_delay = registry
        .get(AUDIO_DELAY_ONE_SAMPLE)
        .expect("one-sample audio delay should be built in");
    assert_has_audio_input(one_sample_delay, AUDIO_IN);
    assert_has_audio_output(one_sample_delay, AUDIO_OUT);
    assert_eq!(one_sample_delay.feedback_boundaries(), &[Audio]);

    let block_delay = registry
        .get(BLOCK_DELAY)
        .expect("block delay should be built in");
    assert_has_audio_input(block_delay, AUDIO_IN);
    assert_has_audio_output(block_delay, AUDIO_OUT);
    assert_eq!(block_delay.feedback_boundaries(), &[Audio]);

    let control_delay = registry
        .get(CONTROL_DELAY)
        .expect("control delay should be built in");
    assert_has_control_input(control_delay, VALUE);
    assert_has_control_output(control_delay, VALUE);
    assert_eq!(control_delay.feedback_boundaries(), &[Control]);
}

fn assert_has_control_output(definition: &BuiltInModuleDefinition, port_name: &str) {
    assert_has_output(Control, definition, port_name);
}

fn assert_has_control_input(definition: &BuiltInModuleDefinition, port_name: &str) {
    assert_has_input(Control, definition, port_name);
}

fn assert_has_audio_input(definition: &BuiltInModuleDefinition, port_name: &str) {
    assert_has_input(Audio, definition, port_name);
}

fn assert_has_event_input(definition: &BuiltInModuleDefinition, port_name: &str) {
    assert_has_input(Event, definition, port_name);
}

fn assert_has_audio_output(definition: &BuiltInModuleDefinition, port_name: &str) {
    assert_has_output(Audio, definition, port_name);
}

fn assert_has_event_output(definition: &BuiltInModuleDefinition, port_name: &str) {
    assert_has_output(Event, definition, port_name);
}

fn assert_has_audio_mixing_input(definition: &BuiltInModuleDefinition, port_name: &str) {
    assert_has_mixing_input(definition, port_name, Audio);
}

fn assert_has_control_mixing_input(definition: &BuiltInModuleDefinition, port_name: &str) {
    assert_has_mixing_input(definition, port_name, Control);
}

#[test]
fn built_in_module_tests_inspect_port_directions_and_feedback_boundaries() {
    let registry = BuiltInModuleRegistry::new();

    for module_type in [
        MIDI_INPUT,
        AUDIO_OUTPUT,
        OSCILLATOR,
        module_types::GAIN,
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

    assert_has_event_input(event_filter, EVENTS_IN);
    assert_has_event_output(event_filter, EVENTS_OUT);
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

    let script = registry.get(SCRIPT).expect("script should be built in");

    assert!(script.inputs().is_empty());
    assert!(script.outputs().is_empty());
    assert!(script.feedback_boundaries().is_empty());
}

#[test]
fn initialized_registry_contains_sampler_definition() {
    let registry = BuiltInModuleRegistry::new();

    let sampler = registry.get(SAMPLER).expect("sampler should be built in");

    assert_has_event_input(sampler, TRIGGER);
    assert_control_inputs!(sampler, RATE, START, LOOP_ENABLED, LOOP_START, LOOP_END,);
    assert_has_audio_output(sampler, AUDIO);
}

#[test]
fn initialized_registry_contains_note_to_rate_definition() {
    let registry = BuiltInModuleRegistry::new();

    let note_to_rate = registry
        .get(NOTE_TO_RATE)
        .expect("note_to_rate should be built in");

    assert_has_event_input(note_to_rate, EVENTS);
    assert_has_control_output(note_to_rate, RATE);
}

#[test]
fn initialized_registry_contains_envelope_follower_and_curve_mapper_definitions() {
    let registry = BuiltInModuleRegistry::new();

    let follower = registry
        .get(ENVELOPE_FOLLOWER)
        .expect("envelope_follower should be built in");
    assert_has_audio_input(follower, AUDIO_IN);
    assert_control_inputs!(follower, ATTACK, RELEASE, AMOUNT, OFFSET, INVERT,);
    assert_has_control_output(follower, VALUE);

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
    assert_control_inputs!(mapper, VALUE, AMOUNT, BIAS, SCALE, OFFSET,);
    assert_has_control_output(mapper, VALUE);

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

fn assert_has_input(signal_type: SignalType, definition: &BuiltInModuleDefinition, name: &str) {
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

    let echo = registry.get(ECHO).expect("echo should be built in");

    assert_audio_inputs!(echo, AUDIO_IN_L, AUDIO_IN_R,);
    assert_audio_outputs!(echo, AUDIO_OUT_L, AUDIO_OUT_R,);
    assert_control_inputs!(
        echo,
        TIME_LEFT_MS,
        TIME_RIGHT_MS,
        FEEDBACK,
        DAMPING_CUTOFF,
        WET,
        DRY,
        SYNC_DIVISION,
        PING_PONG,
    );
}

#[test]
fn reverb_definition_has_correct_ports() {
    let registry = BuiltInModuleRegistry::new();

    let reverb = registry.get(REVERB).expect("reverb should be built in");

    assert_audio_inputs!(reverb, AUDIO_IN_L, AUDIO_IN_R,);
    assert_audio_outputs!(reverb, AUDIO_OUT_L, AUDIO_OUT_R,);
    assert_control_inputs!(
        reverb,
        DECAY_TIME,
        ROOM_SIZE,
        PRE_DELAY,
        DAMPING,
        DIFFUSION,
        STEREO_WIDTH,
        WET,
        DRY,
    );
}

#[test]
fn filter_definition_has_parameter_metadata() {
    let registry = BuiltInModuleRegistry::new();
    let filter = registry.get(FILTER).expect("filter should be built in");

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
    let sampler = registry.get(SAMPLER).expect("sampler should be built in");

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
    let filter = registry.get(FILTER).expect("filter should be built in");

    let known_params: Vec<&str> = filter.parameters().iter().map(|p| p.name()).collect();
    assert!(known_params.contains(&"algorithm"));
    assert!(known_params.contains(&"mode"));
    assert!(known_params.contains(&"comb_type"));
    assert!(!known_params.contains(&"nonexistent"));
}

#[test]
fn parameter_metadata_queryable_without_renderer() {
    let registry = BuiltInModuleRegistry::new();

    let filter = registry.get(FILTER).expect("filter should be built in");
    let sampler = registry.get(SAMPLER).expect("sampler should be built in");
    let oscillator = registry
        .get(OSCILLATOR)
        .expect("oscillator should be built in");

    assert!(!filter.parameters().is_empty());
    assert!(!sampler.parameters().is_empty());
    assert!(oscillator.parameters().is_empty());
}

fn assert_has_output(signal_type: SignalType, definition: &BuiltInModuleDefinition, name: &str) {
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
    assert!(types.contains(&module_types::GAIN));
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
    let osc_inputs = osc.inputs();
    let osc_outputs = osc.outputs();
    assert_eq!(osc_inputs.len(), 1);
    assert_eq!(osc_inputs[0].name(), PITCH);
    assert_eq!(osc_inputs[0].signal_type(), Control);
    assert_eq!(osc_inputs[0].direction(), PortDirection::Input);
    assert_eq!(osc_outputs.len(), 1);
    assert_eq!(osc_outputs[0].name(), AUDIO);
    assert_eq!(osc_outputs[0].signal_type(), Audio);
    assert_eq!(osc_outputs[0].direction(), PortDirection::Output);
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
