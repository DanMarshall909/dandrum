use super::*;
use crate::graph::{PortDirection, SignalType};

#[test]
fn registry_stores_and_finds_module_definitions_by_type() {
    let definition = BuiltInModuleDefinition::new(module_types::GAIN)
        .with_input(Port::input(builtin_ports::AUDIO_IN, SignalType::Audio))
        .with_input(Port::input(builtin_ports::GAIN, SignalType::Control))
        .with_output(Port::output(builtin_ports::AUDIO_OUT, SignalType::Audio));
    let registry = BuiltInModuleRegistry::from_definitions(vec![definition]);

    let gain = registry
        .get(module_types::GAIN)
        .expect("gain definition should be registered");

    assert_eq!(gain.module_type(), module_types::GAIN);
    assert_eq!(gain.inputs()[0].name(), builtin_ports::AUDIO_IN);
    assert_eq!(gain.inputs()[1].signal_type(), SignalType::Control);
    assert_eq!(gain.outputs()[0].name(), builtin_ports::AUDIO_OUT);
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
        .get(module_types::MIDI_INPUT)
        .expect("midi input should be built in");
    assert_eq!(midi_input.outputs()[0].name(), builtin_ports::EVENTS);
    assert_eq!(midi_input.outputs()[0].signal_type(), SignalType::Event);

    let audio_output = registry
        .get(module_types::AUDIO_OUTPUT)
        .expect("audio output should be built in");
    assert_eq!(audio_output.inputs()[0].name(), builtin_ports::LEFT);
    assert_eq!(audio_output.inputs()[0].signal_type(), SignalType::Audio);
    assert_eq!(audio_output.inputs()[1].name(), builtin_ports::RIGHT);
    assert_eq!(audio_output.inputs()[1].signal_type(), SignalType::Audio);
    assert!(audio_output.outputs().is_empty());
}

#[test]
fn initialized_registry_contains_synthesis_control_and_mixer_definitions() {
    let registry = BuiltInModuleRegistry::new();

    let oscillator = registry
        .get(module_types::OSCILLATOR)
        .expect("oscillator should be built in");
    assert_has_input(oscillator, builtin_ports::PITCH, SignalType::Control);
    assert_has_output(oscillator, builtin_ports::AUDIO, SignalType::Audio);

    let gain = registry
        .get(module_types::GAIN)
        .expect("gain should be built in");
    assert_has_input(gain, builtin_ports::AUDIO_IN, SignalType::Audio);
    assert_has_input(gain, builtin_ports::GAIN, SignalType::Control);
    assert_has_output(gain, builtin_ports::AUDIO_OUT, SignalType::Audio);

    let audio_mixer = registry
        .get(module_types::AUDIO_MIXER)
        .expect("audio mixer should be built in");
    assert_has_mixing_input(audio_mixer, builtin_ports::INPUTS, SignalType::Audio);
    assert_has_output(audio_mixer, builtin_ports::MIX, SignalType::Audio);

    let control_mixer = registry
        .get(module_types::CONTROL_MIXER)
        .expect("control mixer should be built in");
    assert_has_mixing_input(control_mixer, builtin_ports::INPUTS, SignalType::Control);
    assert_has_output(control_mixer, builtin_ports::SUM, SignalType::Control);

    let adsr = registry
        .get(module_types::ADSR)
        .expect("ADSR should be built in");
    assert_has_input(adsr, builtin_ports::GATE, SignalType::Event);
    assert_has_input(adsr, builtin_ports::ATTACK, SignalType::Control);
    assert_has_input(adsr, builtin_ports::DECAY, SignalType::Control);
    assert_has_input(adsr, builtin_ports::SUSTAIN, SignalType::Control);
    assert_has_input(adsr, builtin_ports::RELEASE, SignalType::Control);
    assert_has_output(adsr, builtin_ports::VALUE, SignalType::Control);

    let lfo = registry
        .get(module_types::LFO)
        .expect("LFO should be built in");
    assert_has_input(lfo, builtin_ports::RATE, SignalType::Control);
    assert_has_output(lfo, builtin_ports::VALUE, SignalType::Control);

    let filter = registry
        .get(module_types::FILTER)
        .expect("filter should be built in");
    assert_has_input(filter, builtin_ports::AUDIO_IN, SignalType::Audio);
    assert_has_input(filter, builtin_ports::CUTOFF, SignalType::Control);
    assert_has_input(filter, builtin_ports::RESONANCE, SignalType::Control);
    assert_has_input(filter, builtin_ports::GAIN, SignalType::Control);
    assert_has_output(filter, builtin_ports::AUDIO_OUT, SignalType::Audio);
}

#[test]
fn initialized_registry_contains_delay_definitions_with_feedback_boundaries() {
    let registry = BuiltInModuleRegistry::new();

    let one_sample_delay = registry
        .get(module_types::AUDIO_DELAY_ONE_SAMPLE)
        .expect("one-sample audio delay should be built in");
    assert_has_input(one_sample_delay, builtin_ports::AUDIO_IN, SignalType::Audio);
    assert_has_output(
        one_sample_delay,
        builtin_ports::AUDIO_OUT,
        SignalType::Audio,
    );
    assert_eq!(one_sample_delay.feedback_boundaries(), &[SignalType::Audio]);

    let block_delay = registry
        .get(module_types::BLOCK_DELAY)
        .expect("block delay should be built in");
    assert_has_input(block_delay, builtin_ports::AUDIO_IN, SignalType::Audio);
    assert_has_output(block_delay, builtin_ports::AUDIO_OUT, SignalType::Audio);
    assert_eq!(block_delay.feedback_boundaries(), &[SignalType::Audio]);

    let control_delay = registry
        .get(module_types::CONTROL_DELAY)
        .expect("control delay should be built in");
    assert_has_input(control_delay, builtin_ports::VALUE, SignalType::Control);
    assert_has_output(control_delay, builtin_ports::VALUE, SignalType::Control);
    assert_eq!(control_delay.feedback_boundaries(), &[SignalType::Control]);
}

#[test]
fn built_in_module_tests_inspect_port_directions_and_feedback_boundaries() {
    let registry = BuiltInModuleRegistry::new();

    for module_type in [
        module_types::MIDI_INPUT,
        module_types::AUDIO_OUTPUT,
        module_types::OSCILLATOR,
        module_types::GAIN,
        module_types::AUDIO_MIXER,
        module_types::CONTROL_MIXER,
        module_types::ADSR,
        module_types::LFO,
        module_types::FILTER,
        module_types::AUDIO_DELAY_ONE_SAMPLE,
        module_types::BLOCK_DELAY,
        module_types::CONTROL_DELAY,
        module_types::SCRIPT,
        module_types::SAMPLER,
        module_types::NOTE_TO_RATE,
        module_types::DYNAMICS_PROCESSOR,
        module_types::SATURATOR,
        module_types::CONVOLUTION,
        module_types::ECHO,
        module_types::REVERB,
        module_types::FREQUENCY_SPLITTER,
        module_types::SPECTRAL_PROCESSOR,
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
            .get(module_types::AUDIO_DELAY_ONE_SAMPLE)
            .expect("one-sample delay should be built in")
            .feedback_boundaries(),
        &[SignalType::Audio]
    );
    assert_eq!(
        registry
            .get(module_types::CONTROL_DELAY)
            .expect("control delay should be built in")
            .feedback_boundaries(),
        &[SignalType::Control]
    );
}

#[test]
fn initialized_registry_contains_script_definition_with_yaml_declared_ports() {
    let registry = BuiltInModuleRegistry::new();

    let script = registry
        .get(module_types::SCRIPT)
        .expect("script should be built in");

    assert!(script.inputs().is_empty());
    assert!(script.outputs().is_empty());
    assert!(script.feedback_boundaries().is_empty());
}

#[test]
fn initialized_registry_contains_sampler_definition() {
    let registry = BuiltInModuleRegistry::new();

    let sampler = registry
        .get(module_types::SAMPLER)
        .expect("sampler should be built in");

    assert_has_input(sampler, builtin_ports::TRIGGER, SignalType::Event);
    assert_has_input(sampler, builtin_ports::RATE, SignalType::Control);
    assert_has_input(sampler, builtin_ports::START, SignalType::Control);
    assert_has_input(sampler, builtin_ports::LOOP_ENABLED, SignalType::Control);
    assert_has_input(sampler, builtin_ports::LOOP_START, SignalType::Control);
    assert_has_input(sampler, builtin_ports::LOOP_END, SignalType::Control);
    assert_has_output(sampler, builtin_ports::AUDIO, SignalType::Audio);
}

#[test]
fn initialized_registry_contains_note_to_rate_definition() {
    let registry = BuiltInModuleRegistry::new();

    let note_to_rate = registry
        .get(module_types::NOTE_TO_RATE)
        .expect("note_to_rate should be built in");

    assert_has_input(note_to_rate, builtin_ports::EVENTS, SignalType::Event);
    assert_has_output(note_to_rate, builtin_ports::RATE, SignalType::Control);
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
        .get(module_types::ECHO)
        .expect("echo should be built in");

    assert_has_input(echo, builtin_ports::AUDIO_IN_L, SignalType::Audio);
    assert_has_input(echo, builtin_ports::AUDIO_IN_R, SignalType::Audio);
    assert_has_output(echo, builtin_ports::AUDIO_OUT_L, SignalType::Audio);
    assert_has_output(echo, builtin_ports::AUDIO_OUT_R, SignalType::Audio);
    assert_has_input(echo, builtin_ports::TIME_LEFT_MS, SignalType::Control);
    assert_has_input(echo, builtin_ports::TIME_RIGHT_MS, SignalType::Control);
    assert_has_input(echo, builtin_ports::FEEDBACK, SignalType::Control);
    assert_has_input(echo, builtin_ports::DAMPING_CUTOFF, SignalType::Control);
    assert_has_input(echo, builtin_ports::WET, SignalType::Control);
    assert_has_input(echo, builtin_ports::DRY, SignalType::Control);
    assert_has_input(echo, builtin_ports::SYNC_DIVISION, SignalType::Control);
    assert_has_input(echo, builtin_ports::PING_PONG, SignalType::Control);
}

#[test]
fn reverb_definition_has_correct_ports() {
    let registry = BuiltInModuleRegistry::new();

    let reverb = registry
        .get(module_types::REVERB)
        .expect("reverb should be built in");

    assert_has_input(reverb, builtin_ports::AUDIO_IN_L, SignalType::Audio);
    assert_has_input(reverb, builtin_ports::AUDIO_IN_R, SignalType::Audio);
    assert_has_output(reverb, builtin_ports::AUDIO_OUT_L, SignalType::Audio);
    assert_has_output(reverb, builtin_ports::AUDIO_OUT_R, SignalType::Audio);
    assert_has_input(reverb, builtin_ports::DECAY_TIME, SignalType::Control);
    assert_has_input(reverb, builtin_ports::ROOM_SIZE, SignalType::Control);
    assert_has_input(reverb, builtin_ports::PRE_DELAY, SignalType::Control);
    assert_has_input(reverb, builtin_ports::DAMPING, SignalType::Control);
    assert_has_input(reverb, builtin_ports::DIFFUSION, SignalType::Control);
    assert_has_input(reverb, builtin_ports::STEREO_WIDTH, SignalType::Control);
    assert_has_input(reverb, builtin_ports::WET, SignalType::Control);
    assert_has_input(reverb, builtin_ports::DRY, SignalType::Control);
}

fn assert_has_output(definition: &BuiltInModuleDefinition, name: &str, signal_type: SignalType) {
    assert!(
        definition
            .outputs()
            .iter()
            .any(|port| port.name() == name && port.signal_type() == signal_type)
    );
}
