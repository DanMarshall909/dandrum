use std::collections::BTreeMap;

use crate::graph::{ExecutionScope, Port, SignalType, builtin_ports};

pub mod module_types {
    pub const MIDI_INPUT: &str = "midi_input";
    pub const AUDIO_OUTPUT: &str = "audio_output";
    pub const OSCILLATOR: &str = "oscillator";
    pub const GAIN: &str = "gain";
    pub const AUDIO_MIXER: &str = "audio_mixer";
    pub const CONTROL_MIXER: &str = "control_mixer";
    pub const ADSR: &str = "adsr";
    pub const LFO: &str = "lfo";
    pub const FILTER: &str = "filter";
    pub const AUDIO_DELAY_ONE_SAMPLE: &str = "audio_delay_one_sample";
    pub const BLOCK_DELAY: &str = "block_delay";
    pub const CONTROL_DELAY: &str = "control_delay";
    pub const SCRIPT: &str = "script";
    pub const SAMPLER: &str = "sampler";
    pub const NOTE_TO_RATE: &str = "note_to_rate";
    pub const DYNAMICS_PROCESSOR: &str = "dynamics-processor";
    pub const SATURATOR: &str = "saturator";
    pub const CONVOLUTION: &str = "convolution";
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuiltInModuleDefinition {
    module_type: String,
    inputs: Vec<Port>,
    outputs: Vec<Port>,
    feedback_boundaries: Vec<SignalType>,
    execution_scope: ExecutionScope,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuiltInModuleRegistry {
    definitions: BTreeMap<String, BuiltInModuleDefinition>,
}

impl BuiltInModuleDefinition {
    pub fn new(module_type: impl Into<String>) -> Self {
        Self {
            module_type: module_type.into(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            feedback_boundaries: Vec::new(),
            execution_scope: ExecutionScope::Global,
        }
    }

    pub fn with_execution_scope(mut self, scope: ExecutionScope) -> Self {
        self.execution_scope = scope;
        self
    }

    pub fn execution_scope(&self) -> ExecutionScope {
        self.execution_scope
    }

    pub fn with_input(mut self, port: Port) -> Self {
        self.inputs.push(port);
        self
    }

    pub fn with_output(mut self, port: Port) -> Self {
        self.outputs.push(port);
        self
    }

    pub fn with_feedback_boundary(mut self, signal_type: SignalType) -> Self {
        self.feedback_boundaries.push(signal_type);
        self
    }

    pub fn module_type(&self) -> &str {
        &self.module_type
    }

    pub fn inputs(&self) -> &[Port] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[Port] {
        &self.outputs
    }

    pub fn feedback_boundaries(&self) -> &[SignalType] {
        &self.feedback_boundaries
    }
}

impl BuiltInModuleRegistry {
    pub fn new() -> Self {
        Self::from_definitions(vec![
            midi_input_definition(),
            audio_output_definition(),
            oscillator_definition(),
            gain_definition(),
            audio_mixer_definition(),
            control_mixer_definition(),
            adsr_definition(),
            lfo_definition(),
            filter_definition(),
            audio_delay_one_sample_definition(),
            block_delay_definition(),
            control_delay_definition(),
            script_definition(),
            sampler_definition(),
            note_to_rate_definition(),
            dynamics_processor_definition(),
            saturator_definition(),
            convolution_definition(),
        ])
    }

    pub fn from_definitions(definitions: Vec<BuiltInModuleDefinition>) -> Self {
        Self {
            definitions: definitions
                .into_iter()
                .map(|definition| (definition.module_type.clone(), definition))
                .collect(),
        }
    }

    pub fn get(&self, module_type: &str) -> Option<&BuiltInModuleDefinition> {
        self.definitions.get(module_type)
    }
}

impl Default for BuiltInModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn midi_input_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::MIDI_INPUT)
        .with_output(Port::output(builtin_ports::EVENTS, SignalType::Event))
}

fn audio_output_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::AUDIO_OUTPUT)
        .with_input(Port::input(builtin_ports::LEFT, SignalType::Audio))
        .with_input(Port::input(builtin_ports::RIGHT, SignalType::Audio))
}

fn oscillator_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::OSCILLATOR)
        .with_execution_scope(ExecutionScope::Voice)
        .with_input(Port::input(builtin_ports::PITCH, SignalType::Control))
        .with_output(Port::output(builtin_ports::AUDIO, SignalType::Audio))
}

fn gain_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::GAIN)
        .with_execution_scope(ExecutionScope::Voice)
        .with_input(Port::input(builtin_ports::AUDIO_IN, SignalType::Audio))
        .with_input(Port::input(builtin_ports::GAIN, SignalType::Control))
        .with_output(Port::output(builtin_ports::AUDIO_OUT, SignalType::Audio))
}

fn audio_mixer_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::AUDIO_MIXER)
        .with_input(Port::mixing_input(builtin_ports::INPUTS, SignalType::Audio))
        .with_output(Port::output(builtin_ports::MIX, SignalType::Audio))
}

fn control_mixer_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::CONTROL_MIXER)
        .with_input(Port::mixing_input(
            builtin_ports::INPUTS,
            SignalType::Control,
        ))
        .with_output(Port::output(builtin_ports::SUM, SignalType::Control))
}

fn adsr_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::ADSR)
        .with_execution_scope(ExecutionScope::Voice)
        .with_input(Port::input(builtin_ports::GATE, SignalType::Event))
        .with_input(Port::input(builtin_ports::ATTACK, SignalType::Control))
        .with_input(Port::input(builtin_ports::DECAY, SignalType::Control))
        .with_input(Port::input(builtin_ports::SUSTAIN, SignalType::Control))
        .with_input(Port::input(builtin_ports::RELEASE, SignalType::Control))
        .with_output(Port::output(builtin_ports::VALUE, SignalType::Control))
}

fn lfo_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::LFO)
        .with_input(Port::input(builtin_ports::RATE, SignalType::Control))
        .with_output(Port::output(builtin_ports::VALUE, SignalType::Control))
}

fn filter_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::FILTER)
        .with_execution_scope(ExecutionScope::Voice)
        .with_input(Port::input(builtin_ports::AUDIO_IN, SignalType::Audio))
        .with_input(Port::input(builtin_ports::CUTOFF, SignalType::Control))
        .with_output(Port::output(builtin_ports::AUDIO_OUT, SignalType::Audio))
}

fn audio_delay_one_sample_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::AUDIO_DELAY_ONE_SAMPLE)
        .with_input(Port::input(builtin_ports::AUDIO_IN, SignalType::Audio))
        .with_output(Port::output(builtin_ports::AUDIO_OUT, SignalType::Audio))
        .with_feedback_boundary(SignalType::Audio)
}

fn block_delay_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::BLOCK_DELAY)
        .with_input(Port::input(builtin_ports::AUDIO_IN, SignalType::Audio))
        .with_output(Port::output(builtin_ports::AUDIO_OUT, SignalType::Audio))
        .with_feedback_boundary(SignalType::Audio)
}

fn control_delay_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::CONTROL_DELAY)
        .with_input(Port::input(builtin_ports::VALUE, SignalType::Control))
        .with_output(Port::output(builtin_ports::VALUE, SignalType::Control))
        .with_feedback_boundary(SignalType::Control)
}

fn script_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::SCRIPT).with_execution_scope(ExecutionScope::Voice)
}

fn sampler_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::SAMPLER)
        .with_execution_scope(ExecutionScope::Voice)
        .with_input(Port::input(builtin_ports::TRIGGER, SignalType::Event))
        .with_input(Port::input(builtin_ports::RATE, SignalType::Control))
        .with_input(Port::input(builtin_ports::START, SignalType::Control))
        .with_input(Port::input(
            builtin_ports::LOOP_ENABLED,
            SignalType::Control,
        ))
        .with_input(Port::input(builtin_ports::LOOP_START, SignalType::Control))
        .with_input(Port::input(builtin_ports::LOOP_END, SignalType::Control))
        .with_output(Port::output(builtin_ports::AUDIO, SignalType::Audio))
}

fn note_to_rate_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::NOTE_TO_RATE)
        .with_execution_scope(ExecutionScope::Voice)
        .with_input(Port::input(builtin_ports::EVENTS, SignalType::Event))
        .with_output(Port::output(builtin_ports::RATE, SignalType::Control))
}

fn dynamics_processor_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::DYNAMICS_PROCESSOR)
        .with_input(Port::input(builtin_ports::AUDIO_IN, SignalType::Audio))
        .with_input(Port::input(builtin_ports::SIDECHAIN_IN, SignalType::Control))
        .with_input(Port::input(builtin_ports::THRESHOLD, SignalType::Control))
        .with_input(Port::input(builtin_ports::BELOW_RATIO, SignalType::Control))
        .with_input(Port::input(builtin_ports::ABOVE_RATIO, SignalType::Control))
        .with_input(Port::input(builtin_ports::ATTACK, SignalType::Control))
        .with_input(Port::input(builtin_ports::RELEASE, SignalType::Control))
        .with_input(Port::input(builtin_ports::KNEE, SignalType::Control))
        .with_input(Port::input(builtin_ports::MAKEUP_GAIN, SignalType::Control))
        .with_input(Port::input(builtin_ports::ATTACK_GAIN, SignalType::Control))
        .with_input(Port::input(builtin_ports::SUSTAIN_GAIN, SignalType::Control))
        .with_output(Port::output(builtin_ports::AUDIO_OUT, SignalType::Audio))
}

fn saturator_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::SATURATOR)
        .with_input(Port::input(builtin_ports::AUDIO_IN, SignalType::Audio))
        .with_input(Port::input(builtin_ports::DRIVE, SignalType::Control))
        .with_input(Port::input(builtin_ports::BIAS, SignalType::Control))
        .with_input(Port::input(builtin_ports::CURVE_SELECT, SignalType::Control))
        .with_output(Port::output(builtin_ports::AUDIO_OUT, SignalType::Audio))
}

fn convolution_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::CONVOLUTION)
        .with_input(Port::input(builtin_ports::AUDIO_IN, SignalType::Audio))
        .with_input(Port::input(builtin_ports::MIX, SignalType::Control))
        .with_output(Port::output(builtin_ports::AUDIO_OUT, SignalType::Audio))
}

#[cfg(test)]
mod tests {
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
            port.name() == name
                && port.signal_type() == signal_type
                && port.accepts_multiple_sources()
        }));
    }

    fn assert_has_output(
        definition: &BuiltInModuleDefinition,
        name: &str,
        signal_type: SignalType,
    ) {
        assert!(
            definition
                .outputs()
                .iter()
                .any(|port| port.name() == name && port.signal_type() == signal_type)
        );
    }
}
