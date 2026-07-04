use std::collections::BTreeMap;

use crate::graph::{ExecutionScope, Port, SignalType, builtin_ports};

pub mod module_kind;
pub mod module_types;

/// The type of value a parameter accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterValueType {
    Boolean,
    Integer,
    Number,
    Text,
}

/// Metadata describing a single configurable parameter of a built-in module.
#[derive(Clone, Debug, PartialEq)]
pub struct ParameterMetadata {
    name: String,
    value_type: ParameterValueType,
    default: Option<String>,
    range: Option<(f64, f64)>,
    unit: Option<String>,
    enum_values: Option<Vec<String>>,
    description: Option<String>,
    realtime_note: Option<String>,
}

pub const EVENT_FILTER_SELECTOR_PARAMETER: &str = "selector";
pub const EVENT_FILTER_NOTE_PARAMETER: &str = "note";
pub const EVENT_FILTER_NOTE_SELECTOR: &str = "note";
pub const EVENT_FILTER_SELECTOR_DEFAULT: &str = EVENT_FILTER_NOTE_SELECTOR;
pub const SCRIPT_LANGUAGE_PARAMETER: &str = "language";
pub const SCRIPT_SOURCE_PARAMETER: &str = "source";
pub const SCRIPT_LANGUAGE_RHAI: &str = "rhai";

impl ParameterMetadata {
    pub fn new(name: impl Into<String>, value_type: ParameterValueType) -> Self {
        Self {
            name: name.into(),
            value_type,
            default: None,
            range: None,
            unit: None,
            enum_values: None,
            description: None,
            realtime_note: None,
        }
    }

    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }

    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.range = Some((min, max));
        self
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    pub fn with_enum_values(mut self, values: Vec<impl Into<String>>) -> Self {
        self.enum_values = Some(values.into_iter().map(|v| v.into()).collect());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_realtime_note(mut self, note: impl Into<String>) -> Self {
        self.realtime_note = Some(note.into());
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value_type(&self) -> ParameterValueType {
        self.value_type
    }

    pub fn default(&self) -> Option<&str> {
        self.default.as_deref()
    }

    pub fn range(&self) -> Option<(f64, f64)> {
        self.range
    }

    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }

    pub fn enum_values(&self) -> Option<&[String]> {
        self.enum_values.as_deref()
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn realtime_note(&self) -> Option<&str> {
        self.realtime_note.as_deref()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleCategory {
    Primitive,
    Script,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuiltInModuleDefinition {
    module_type: String,
    inputs: Vec<Port>,
    outputs: Vec<Port>,
    feedback_boundaries: Vec<SignalType>,
    execution_scope: ExecutionScope,
    parameters: Vec<ParameterMetadata>,
    examples: Vec<String>,
    category: ModuleCategory,
}

#[derive(Clone, Debug, PartialEq)]
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
            parameters: Vec::new(),
            examples: Vec::new(),
            category: ModuleCategory::Primitive,
        }
    }

    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }

    pub fn examples(&self) -> &[String] {
        &self.examples
    }

    pub fn with_parameter(mut self, param: ParameterMetadata) -> Self {
        self.parameters.push(param);
        self
    }

    pub fn parameters(&self) -> &[ParameterMetadata] {
        &self.parameters
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

    pub fn with_module_category(mut self, category: ModuleCategory) -> Self {
        self.category = category;
        self
    }

    pub fn module_category(&self) -> ModuleCategory {
        self.category
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
            noise_definition(),
            impulse_definition(),
            multiply_definition(),
            note_to_control_definition(),
            event_filter_definition(),
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

    pub fn module_types(&self) -> impl Iterator<Item = &str> + '_ {
        self.definitions.keys().map(|s| s.as_str())
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
        .with_parameter(
            ParameterMetadata::new("algorithm", ParameterValueType::Text)
                .with_default("moog")
                .with_enum_values(vec!["moog", "biquad", "comb"])
                .with_description("filter topology"),
        )
        .with_parameter(
            ParameterMetadata::new("mode", ParameterValueType::Text)
                .with_default("lowpass")
                .with_enum_values(vec!["lowpass", "highpass", "peaking"])
                .with_description("biquad filter mode"),
        )
        .with_parameter(
            ParameterMetadata::new("comb_type", ParameterValueType::Text)
                .with_default("feedback")
                .with_enum_values(vec!["feedback", "feedforward"])
                .with_description("comb filter type"),
        )
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
    BuiltInModuleDefinition::new(module_types::SCRIPT)
        .with_execution_scope(ExecutionScope::Voice)
        .with_module_category(ModuleCategory::Script)
        .with_parameter(
            ParameterMetadata::new(SCRIPT_LANGUAGE_PARAMETER, ParameterValueType::Text)
                .with_default(SCRIPT_LANGUAGE_RHAI)
                .with_enum_values(vec![SCRIPT_LANGUAGE_RHAI])
                .with_description("script language"),
        )
        .with_parameter(
            ParameterMetadata::new(SCRIPT_SOURCE_PARAMETER, ParameterValueType::Text)
                .with_description("inline script source"),
        )
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
        .with_parameter(
            ParameterMetadata::new("asset", ParameterValueType::Text)
                .with_description("asset ID of the sample to play")
                .with_realtime_note("must reference an asset declared in the patch assets section"),
        )
}

fn note_to_rate_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::NOTE_TO_RATE)
        .with_execution_scope(ExecutionScope::Voice)
        .with_input(Port::input(builtin_ports::EVENTS, SignalType::Event))
        .with_output(Port::output(builtin_ports::RATE, SignalType::Control))
}

fn event_filter_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::EVENT_FILTER)
        .with_execution_scope(ExecutionScope::Voice)
        .with_input(Port::input(builtin_ports::EVENTS_IN, SignalType::Event))
        .with_output(Port::output(builtin_ports::EVENTS_OUT, SignalType::Event))
        .with_parameter(
            ParameterMetadata::new(EVENT_FILTER_SELECTOR_PARAMETER, ParameterValueType::Text)
                .with_default(EVENT_FILTER_SELECTOR_DEFAULT)
                .with_enum_values(vec![EVENT_FILTER_NOTE_SELECTOR])
                .with_description("Selects which event field the filter matches."),
        )
        .with_parameter(
            ParameterMetadata::new(EVENT_FILTER_NOTE_PARAMETER, ParameterValueType::Integer)
                .with_range(0.0, 127.0)
                .with_description("MIDI note number passed by the note selector.")
                .with_realtime_note("Filtering compares incoming event metadata without generating audio or control output."),
        )
        .with_example(
            "- id: kick_filter\n  type: event_filter\n  parameters:\n    selector: note\n    note: 36",
        )
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
        .with_parameter(
            ParameterMetadata::new("mode", ParameterValueType::Text)
                .with_default("gate")
                .with_enum_values(vec!["gate", "passthrough"])
                .with_description("spectral processing mode"),
        )
        .with_parameter(
            ParameterMetadata::new("fft_size", ParameterValueType::Number)
                .with_default("2048")
                .with_range(256.0, 8192.0)
                .with_description("FFT frame size in samples")
                .with_realtime_note(
                    "latency = fft_size - 1 samples; hop_size = fft_size / 2; \
                     block_size should be >= hop_size to avoid stuttering; \
                     allocates input_buf(fft_size) + output_buf(fft_size*2) + \
                     window(fft_size) + FFT scratch on first frame",
                ),
        )
        .with_parameter(
            ParameterMetadata::new("window", ParameterValueType::Text)
                .with_default("hann")
                .with_enum_values(vec!["hann"])
                .with_description("analysis/synthesis window function"),
        )
        .with_parameter(
            ParameterMetadata::new("threshold", ParameterValueType::Number)
                .with_default("-40")
                .with_range(-100.0, 0.0)
                .with_unit("dB")
                .with_description("gate threshold in dBFS"),
        )
        .with_parameter(
            ParameterMetadata::new("mix", ParameterValueType::Number)
                .with_default("1.0")
                .with_range(0.0, 1.0)
                .with_description("wet/dry mix (0 = dry, 1 = wet)"),
        )
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

fn noise_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::NOISE)
        .with_execution_scope(ExecutionScope::Voice)
        .with_output(Port::output(builtin_ports::AUDIO, SignalType::Audio))
        .with_parameter(
            ParameterMetadata::new("seed", ParameterValueType::Number)
                .with_default("0")
                .with_description("random seed for deterministic noise output"),
        )
}

fn impulse_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::IMPULSE)
        .with_execution_scope(ExecutionScope::Voice)
        .with_input(Port::input(builtin_ports::TRIGGER, SignalType::Event))
        .with_output(Port::output(builtin_ports::AUDIO, SignalType::Audio))
}

fn multiply_definition() -> BuiltInModuleDefinition {
    // Multiply is audio-only. Both inputs accept audio signals and produce
    // an audio-rate product. Control-rate multiplication is deferred until
    // polymorphic port support or a dedicated control_multiply primitive.
    BuiltInModuleDefinition::new(module_types::MULTIPLY)
        .with_execution_scope(ExecutionScope::Global)
        .with_input(Port::input(builtin_ports::AUDIO_IN, SignalType::Audio))
        .with_input(Port::input(builtin_ports::GAIN, SignalType::Audio))
        .with_output(Port::output(builtin_ports::AUDIO_OUT, SignalType::Audio))
}

fn note_to_control_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::NOTE_TO_CONTROL)
        .with_execution_scope(ExecutionScope::Voice)
        .with_input(Port::input(builtin_ports::EVENTS, SignalType::Event))
        .with_output(Port::output("frequency", SignalType::Control))
        .with_output(Port::output("pitch_ratio", SignalType::Control))
        .with_output(Port::output("gate", SignalType::Event))
        .with_output(Port::output("velocity", SignalType::Control))
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
