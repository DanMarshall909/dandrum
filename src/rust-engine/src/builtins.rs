use std::collections::BTreeMap;

use crate::graph::{builtin_ports, ExecutionScope, Port, SignalType};

pub mod module_types;
pub mod module_kind;

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
            echo_definition(),
            reverb_definition(),
            frequency_splitter_definition(),
            spectral_processor_definition(),
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
        .with_input(Port::input(builtin_ports::RESONANCE, SignalType::Control))
        .with_input(Port::input(builtin_ports::GAIN, SignalType::Control))
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
        .with_input(Port::input(
            builtin_ports::SIDECHAIN_IN,
            SignalType::Control,
        ))
        .with_input(Port::input(builtin_ports::THRESHOLD, SignalType::Control))
        .with_input(Port::input(builtin_ports::BELOW_RATIO, SignalType::Control))
        .with_input(Port::input(builtin_ports::ABOVE_RATIO, SignalType::Control))
        .with_input(Port::input(builtin_ports::ATTACK, SignalType::Control))
        .with_input(Port::input(builtin_ports::RELEASE, SignalType::Control))
        .with_input(Port::input(builtin_ports::KNEE, SignalType::Control))
        .with_input(Port::input(builtin_ports::MAKEUP_GAIN, SignalType::Control))
        .with_input(Port::input(builtin_ports::ATTACK_GAIN, SignalType::Control))
        .with_input(Port::input(
            builtin_ports::SUSTAIN_GAIN,
            SignalType::Control,
        ))
        .with_output(Port::output(builtin_ports::AUDIO_OUT, SignalType::Audio))
}

fn saturator_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::SATURATOR)
        .with_input(Port::input(builtin_ports::AUDIO_IN, SignalType::Audio))
        .with_input(Port::input(builtin_ports::DRIVE, SignalType::Control))
        .with_input(Port::input(builtin_ports::BIAS, SignalType::Control))
        .with_input(Port::input(
            builtin_ports::CURVE_SELECT,
            SignalType::Control,
        ))
        .with_output(Port::output(builtin_ports::AUDIO_OUT, SignalType::Audio))
}

fn convolution_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::CONVOLUTION)
        .with_input(Port::input(builtin_ports::AUDIO_IN, SignalType::Audio))
        .with_input(Port::input(builtin_ports::MIX, SignalType::Control))
        .with_output(Port::output(builtin_ports::AUDIO_OUT, SignalType::Audio))
}

fn frequency_splitter_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::FREQUENCY_SPLITTER)
        .with_input(Port::input(builtin_ports::AUDIO_IN, SignalType::Audio))
        .with_input(Port::input(
            builtin_ports::CROSSOVER_HZ,
            SignalType::Control,
        ))
        .with_output(Port::output("low", SignalType::Audio))
        .with_output(Port::output("mid", SignalType::Audio))
        .with_output(Port::output("high", SignalType::Audio))
}

fn spectral_processor_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::SPECTRAL_PROCESSOR)
        .with_input(Port::input(builtin_ports::AUDIO_IN, SignalType::Audio))
        .with_input(Port::input(builtin_ports::THRESHOLD, SignalType::Control))
        .with_input(Port::input(builtin_ports::MIX, SignalType::Control))
        .with_output(Port::output(builtin_ports::AUDIO_OUT, SignalType::Audio))
}

fn echo_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::ECHO)
        .with_input(Port::input(builtin_ports::AUDIO_IN_L, SignalType::Audio))
        .with_input(Port::input(builtin_ports::AUDIO_IN_R, SignalType::Audio))
        .with_output(Port::output(builtin_ports::AUDIO_OUT_L, SignalType::Audio))
        .with_output(Port::output(builtin_ports::AUDIO_OUT_R, SignalType::Audio))
        .with_input(Port::input(
            builtin_ports::TIME_LEFT_MS,
            SignalType::Control,
        ))
        .with_input(Port::input(
            builtin_ports::TIME_RIGHT_MS,
            SignalType::Control,
        ))
        .with_input(Port::input(builtin_ports::FEEDBACK, SignalType::Control))
        .with_input(Port::input(
            builtin_ports::DAMPING_CUTOFF,
            SignalType::Control,
        ))
        .with_input(Port::input(builtin_ports::WET, SignalType::Control))
        .with_input(Port::input(builtin_ports::DRY, SignalType::Control))
        .with_input(Port::input(
            builtin_ports::SYNC_DIVISION,
            SignalType::Control,
        ))
        .with_input(Port::input(builtin_ports::PING_PONG, SignalType::Control))
}

fn reverb_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::REVERB)
        .with_input(Port::input(builtin_ports::AUDIO_IN_L, SignalType::Audio))
        .with_input(Port::input(builtin_ports::AUDIO_IN_R, SignalType::Audio))
        .with_output(Port::output(builtin_ports::AUDIO_OUT_L, SignalType::Audio))
        .with_output(Port::output(builtin_ports::AUDIO_OUT_R, SignalType::Audio))
        .with_input(Port::input(builtin_ports::DECAY_TIME, SignalType::Control))
        .with_input(Port::input(builtin_ports::ROOM_SIZE, SignalType::Control))
        .with_input(Port::input(builtin_ports::PRE_DELAY, SignalType::Control))
        .with_input(Port::input(builtin_ports::DAMPING, SignalType::Control))
        .with_input(Port::input(builtin_ports::DIFFUSION, SignalType::Control))
        .with_input(Port::input(
            builtin_ports::STEREO_WIDTH,
            SignalType::Control,
        ))
        .with_input(Port::input(builtin_ports::WET, SignalType::Control))
        .with_input(Port::input(builtin_ports::DRY, SignalType::Control))
}

#[cfg(test)]
mod tests;
