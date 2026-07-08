use crate::graph::{ExecutionScope, Port, SignalType, builtin_ports};
use SignalType::*;
use builtin_ports::*;
use module_types::*;
use std::collections::BTreeMap;

pub mod module_kind;
pub mod module_types;

/// The type of value a parameter accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterValueType {
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
pub const DETECTION_MODE_PARAMETER: &str = "mode";
pub const DETECTION_MODE_PEAK: &str = "peak";
pub const DETECTION_MODE_RMS: &str = "rms";
pub const DYNAMICS_MODE_PARAMETER: &str = "mode";
pub const DYNAMICS_MODE_LEVEL: &str = "level";
pub const DYNAMICS_MODE_TRANSIENT: &str = "transient";
pub const DYNAMICS_DETECTION_PARAMETER: &str = "detection";
pub const DYNAMICS_TOPOLOGY_PARAMETER: &str = "topology";
pub const DYNAMICS_TOPOLOGY_FEEDFORWARD: &str = "feedforward";
pub const DYNAMICS_TOPOLOGY_FEEDBACK: &str = "feedback";
pub const INTERPOLATION_PARAMETER: &str = "interpolation";
pub const INTERPOLATION_LINEAR: &str = "linear";
pub const INTERPOLATION_CUBIC: &str = "cubic";
pub const CURVE_PARAMETER: &str = "curve";
pub const CURVE_LINEAR: &str = "linear";
pub const CURVE_EXPONENTIAL: &str = "exponential";
pub const CURVE_LOGARITHMIC: &str = "logarithmic";
pub const CURVE_S_CURVE: &str = "s_curve";
pub const CURVE_SOFT_CLIP: &str = "soft_clip";
pub const CURVE_HARD_CLIP: &str = "hard_clip";
pub const CURVE_STEP: &str = "step";
pub const STEPS_PARAMETER: &str = "steps";
pub const WAVEFORM_PARAMETER: &str = "waveform";
pub const WAVEFORM_SAW: &str = "saw";
pub const WAVEFORM_SINE: &str = "sine";
pub const WAVEFORM_TRIANGLE: &str = "triangle";
pub const WAVEFORM_SQUARE: &str = "square";

impl ParameterMetadata {
    pub fn new(name: impl Into<String>, value_type: ParameterValueType) -> Self {
        Self {
            name: name.into(),
            value_type,
            default: None,
            range: None,
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

    pub fn enum_values(&self) -> Option<&[String]> {
        self.enum_values.as_deref()
    }

    #[allow(dead_code)]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    pub fn with_inputs<'a>(
        mut self,
        ports: impl IntoIterator<Item = (&'a str, SignalType)>,
    ) -> Self {
        self.inputs.extend(
            ports
                .into_iter()
                .map(|(name, signal_type)| Port::input(name, signal_type)),
        );
        self
    }

    pub fn with_output(mut self, port: Port) -> Self {
        self.outputs.push(port);
        self
    }

    pub fn with_output_ports<'a>(
        mut self,
        ports: impl IntoIterator<Item = (&'a str, SignalType)>,
    ) -> Self {
        self.outputs.extend(
            ports
                .into_iter()
                .map(|(name, signal_type)| Port::output(name, signal_type)),
        );
        self
    }

    pub fn with_feedback_boundary(mut self, signal_type: SignalType) -> Self {
        self.feedback_boundaries.push(signal_type);
        self
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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
            envelope_follower_definition(),
            curve_mapper_definition(),
            decay_definition(),
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

    #[allow(dead_code)]
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
    BuiltInModuleDefinition::new(MIDI_INPUT).with_output(Port::output(EVENTS, Event))
}

fn audio_output_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(AUDIO_OUTPUT).with_inputs([(LEFT, Audio), (RIGHT, Audio)])
}

fn oscillator_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(OSCILLATOR)
        .with_execution_scope(ExecutionScope::Voice)
        .with_inputs([(PITCH, Control)])
        .with_output(Port::output(AUDIO, Audio))
        .with_parameter(
            ParameterMetadata::new(PITCH, ParameterValueType::Number)
                .with_default("1")
                .with_range(0.0, 64.0)
                .with_description(
                    "fixed pitch ratio of the 220 Hz base used when the pitch input is not \
                     connected (e.g. 2.45 ≈ 539 Hz)",
                )
                .with_realtime_note("updates the runtime pitch without rebuilding the graph"),
        )
        .with_parameter(
            ParameterMetadata::new(WAVEFORM_PARAMETER, ParameterValueType::Text)
                .with_default(WAVEFORM_SAW)
                .with_enum_values(vec![
                    WAVEFORM_SAW,
                    WAVEFORM_SINE,
                    WAVEFORM_TRIANGLE,
                    WAVEFORM_SQUARE,
                ])
                .with_description(
                    "output waveform shape; the pitch input carries a ratio of the 220 Hz base",
                ),
        )
        .with_example("- id: osc\n  type: oscillator\n  parameters:\n    waveform: sine")
}

fn gain_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::GAIN)
        .with_execution_scope(ExecutionScope::Voice)
        .with_inputs([(AUDIO_IN, Audio), (builtin_ports::GAIN, Control)])
        .with_output(Port::output(AUDIO_OUT, Audio))
        .with_parameter(
            ParameterMetadata::new(builtin_ports::GAIN, ParameterValueType::Number)
                .with_default("1")
                .with_range(0.0, 4.0)
                .with_description("static gain applied when the gain input is not connected")
                .with_realtime_note("updates the runtime gain without rebuilding the graph"),
        )
}

fn audio_mixer_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(AUDIO_MIXER)
        .with_input(Port::mixing_input(INPUTS, Audio))
        .with_output(Port::output(MIX, Audio))
}

fn control_mixer_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(CONTROL_MIXER)
        .with_input(Port::mixing_input(INPUTS, Control))
        .with_output(Port::output(SUM, Control))
}

fn adsr_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(ADSR)
        .with_execution_scope(ExecutionScope::Voice)
        .with_inputs([
            (GATE, Event),
            (ATTACK, Control),
            (builtin_ports::DECAY, Control),
            (SUSTAIN, Control),
            (RELEASE, Control),
        ])
        .with_output(Port::output(VALUE, Control))
        .with_parameter(
            ParameterMetadata::new(ATTACK, ParameterValueType::Number)
                .with_default("5")
                .with_range(0.0, 500.0)
                .with_description("attack time in milliseconds (direct value) or 0-1 normalized"),
        )
        .with_parameter(
            ParameterMetadata::new(builtin_ports::DECAY, ParameterValueType::Number)
                .with_default("30")
                .with_range(1.0, 5000.0)
                .with_description("decay time in milliseconds (direct value) or 0-1 normalized"),
        )
        .with_parameter(
            ParameterMetadata::new(SUSTAIN, ParameterValueType::Number)
                .with_default("0.7")
                .with_range(0.0, 1.0)
                .with_description("sustain level"),
        )
        .with_parameter(
            ParameterMetadata::new(RELEASE, ParameterValueType::Number)
                .with_default("200")
                .with_range(0.0, 10000.0)
                .with_description("release time in milliseconds (direct value) or 0-1 normalized"),
        )
}

fn lfo_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(LFO)
        .with_inputs([(RATE, Control)])
        .with_output(Port::output(VALUE, Control))
}

fn filter_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(FILTER)
        .with_execution_scope(ExecutionScope::Voice)
        .with_inputs([
            (AUDIO_IN, Audio),
            (CUTOFF, Control),
            (RESONANCE, Control),
            (builtin_ports::GAIN, Control),
        ])
        .with_output(Port::output(AUDIO_OUT, Audio))
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
    BuiltInModuleDefinition::new(AUDIO_DELAY_ONE_SAMPLE)
        .with_inputs([(AUDIO_IN, Audio)])
        .with_output(Port::output(AUDIO_OUT, Audio))
        .with_feedback_boundary(Audio)
}

fn block_delay_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(BLOCK_DELAY)
        .with_inputs([(AUDIO_IN, Audio)])
        .with_output(Port::output(AUDIO_OUT, Audio))
        .with_feedback_boundary(Audio)
}

fn control_delay_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(CONTROL_DELAY)
        .with_inputs([(VALUE, Control)])
        .with_output(Port::output(VALUE, Control))
        .with_feedback_boundary(Control)
}

fn script_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(SCRIPT)
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
    BuiltInModuleDefinition::new(SAMPLER)
        .with_execution_scope(ExecutionScope::Voice)
        .with_inputs([
            (TRIGGER, Event),
            (RATE, Control),
            (START, Control),
            (LOOP_ENABLED, Control),
            (LOOP_START, Control),
            (LOOP_END, Control),
        ])
        .with_output(Port::output(AUDIO, Audio))
        .with_parameter(
            ParameterMetadata::new("asset", ParameterValueType::Text)
                .with_description("asset ID of the sample to play")
                .with_realtime_note("must reference an asset declared in the patch assets section"),
        )
}

fn note_to_rate_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(NOTE_TO_RATE)
        .with_execution_scope(ExecutionScope::Voice)
        .with_inputs([(EVENTS, Event)])
        .with_output(Port::output(RATE, Control))
}

fn event_filter_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(EVENT_FILTER)
        .with_execution_scope(ExecutionScope::Voice)
        .with_inputs([(EVENTS_IN, Event)])
        .with_output(Port::output(EVENTS_OUT, Event))
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
    BuiltInModuleDefinition::new(DYNAMICS_PROCESSOR)
        .with_inputs([
            (AUDIO_IN, Audio),
            (SIDECHAIN_IN, Control),
            (THRESHOLD, Control),
            (BELOW_RATIO, Control),
            (ABOVE_RATIO, Control),
            (ATTACK, Control),
            (RELEASE, Control),
            (KNEE, Control),
            (MAKEUP_GAIN, Control),
            (ATTACK_GAIN, Control),
            (SUSTAIN_GAIN, Control),
        ])
        .with_output(Port::output(AUDIO_OUT, Audio))
        .with_parameter(
            ParameterMetadata::new(DYNAMICS_MODE_PARAMETER, ParameterValueType::Text)
                .with_default(DYNAMICS_MODE_LEVEL)
                .with_enum_values(vec![DYNAMICS_MODE_LEVEL, DYNAMICS_MODE_TRANSIENT])
                .with_description("gain computation mode: level compression or transient shaping"),
        )
        .with_parameter(
            ParameterMetadata::new(DYNAMICS_DETECTION_PARAMETER, ParameterValueType::Text)
                .with_default(DETECTION_MODE_PEAK)
                .with_enum_values(vec![DETECTION_MODE_PEAK, DETECTION_MODE_RMS])
                .with_description("envelope detection mode"),
        )
        .with_parameter(
            ParameterMetadata::new(DYNAMICS_TOPOLOGY_PARAMETER, ParameterValueType::Text)
                .with_default(DYNAMICS_TOPOLOGY_FEEDFORWARD)
                .with_enum_values(vec![DYNAMICS_TOPOLOGY_FEEDFORWARD, DYNAMICS_TOPOLOGY_FEEDBACK])
                .with_description("detector topology: feedforward or feedback"),
        )
}

fn saturator_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(SATURATOR)
        .with_inputs([
            (AUDIO_IN, Audio),
            (DRIVE, Control),
            (BIAS, Control),
            (CURVE_SELECT, Control),
        ])
        .with_output(Port::output(AUDIO_OUT, Audio))
}

fn convolution_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(CONVOLUTION)
        .with_inputs([(AUDIO_IN, Audio), (MIX, Control)])
        .with_output(Port::output(AUDIO_OUT, Audio))
}

fn frequency_splitter_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(FREQUENCY_SPLITTER)
        .with_inputs([(AUDIO_IN, Audio), (CROSSOVER_HZ, Control)])
        .with_output(Port::output("low", Audio))
        .with_output(Port::output("mid", Audio))
        .with_output(Port::output("high", Audio))
}

fn spectral_processor_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(SPECTRAL_PROCESSOR)
        .with_inputs([(AUDIO_IN, Audio), (THRESHOLD, Control), (MIX, Control)])
        .with_output(Port::output(AUDIO_OUT, Audio))
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
    BuiltInModuleDefinition::new(ECHO)
        .with_inputs([(AUDIO_IN_L, Audio), (AUDIO_IN_R, Audio)])
        .with_output(Port::output(AUDIO_OUT_L, Audio))
        .with_output(Port::output(AUDIO_OUT_R, Audio))
        .with_inputs([
            (TIME_LEFT_MS, Control),
            (TIME_RIGHT_MS, Control),
            (FEEDBACK, Control),
            (DAMPING_CUTOFF, Control),
            (WET, Control),
            (DRY, Control),
            (SYNC_DIVISION, Control),
            (PING_PONG, Control),
        ])
        .with_parameter(interpolation_parameter())
}

fn interpolation_parameter() -> ParameterMetadata {
    ParameterMetadata::new(INTERPOLATION_PARAMETER, ParameterValueType::Text)
        .with_default(INTERPOLATION_LINEAR)
        .with_enum_values(vec![INTERPOLATION_LINEAR, INTERPOLATION_CUBIC])
        .with_description("fractional-delay interpolation quality")
}

fn noise_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(NOISE)
        .with_execution_scope(ExecutionScope::Voice)
        .with_output(Port::output(AUDIO, Audio))
        .with_parameter(
            ParameterMetadata::new("seed", ParameterValueType::Number)
                .with_default("0")
                .with_description("random seed for deterministic noise output"),
        )
}

fn impulse_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(IMPULSE)
        .with_execution_scope(ExecutionScope::Voice)
        .with_inputs([(TRIGGER, Event)])
        .with_output(Port::output(AUDIO, Audio))
}

fn multiply_definition() -> BuiltInModuleDefinition {
    // Multiply is audio-only. Both inputs accept audio signals and produce
    // an audio-rate product. Control-rate multiplication is deferred until
    // polymorphic port support or a dedicated control_multiply primitive.
    BuiltInModuleDefinition::new(MULTIPLY)
        .with_execution_scope(ExecutionScope::Global)
        .with_inputs([(AUDIO_IN, Audio), (builtin_ports::GAIN, Audio)])
        .with_output(Port::output(AUDIO_OUT, Audio))
}

fn note_to_control_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(NOTE_TO_CONTROL)
        .with_execution_scope(ExecutionScope::Voice)
        .with_inputs([(EVENTS, Event)])
        .with_output(Port::output("frequency", Control))
        .with_output(Port::output("pitch_ratio", Control))
        .with_output(Port::output("gate", Event))
        .with_output(Port::output("velocity", Control))
}

fn envelope_follower_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(ENVELOPE_FOLLOWER)
        .with_execution_scope(ExecutionScope::Voice)
        .with_inputs([
            (AUDIO_IN, Audio),
            (ATTACK, Control),
            (RELEASE, Control),
            (AMOUNT, Control),
            (OFFSET, Control),
            (INVERT, Control),
        ])
        .with_output_ports([(VALUE, Control)])
        .with_parameter(
            ParameterMetadata::new(DETECTION_MODE_PARAMETER, ParameterValueType::Text)
                .with_default(DETECTION_MODE_PEAK)
                .with_enum_values(vec![DETECTION_MODE_PEAK, DETECTION_MODE_RMS])
                .with_description("level detection mode"),
        )
        .with_example("- id: follower\n  type: envelope_follower\n  parameters:\n    mode: peak")
}

fn curve_mapper_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(CURVE_MAPPER)
        .with_execution_scope(ExecutionScope::Voice)
        .with_inputs([
            (VALUE, Control),
            (AMOUNT, Control),
            (BIAS, Control),
            (SCALE, Control),
            (OFFSET, Control),
        ])
        .with_output_ports([(VALUE, Control)])
        .with_parameter(
            ParameterMetadata::new(CURVE_PARAMETER, ParameterValueType::Text)
                .with_default(CURVE_LINEAR)
                .with_enum_values(vec![
                    CURVE_LINEAR,
                    CURVE_EXPONENTIAL,
                    CURVE_LOGARITHMIC,
                    CURVE_S_CURVE,
                    CURVE_SOFT_CLIP,
                    CURVE_HARD_CLIP,
                    CURVE_STEP,
                ])
                .with_description("control mapping curve"),
        )
        .with_parameter(
            ParameterMetadata::new(STEPS_PARAMETER, ParameterValueType::Integer)
                .with_default("4")
                .with_range(2.0, 128.0)
                .with_description("quantisation levels used by the step curve"),
        )
        .with_parameter(
            ParameterMetadata::new(AMOUNT, ParameterValueType::Number)
                .with_default("1")
                .with_range(0.0, 1.0)
                .with_description("blend between the dry input and the shaped curve"),
        )
        .with_parameter(
            ParameterMetadata::new(BIAS, ParameterValueType::Number)
                .with_default("0")
                .with_range(-1.0, 1.0)
                .with_description("offset added to the input before the curve is applied"),
        )
        .with_parameter(
            ParameterMetadata::new(SCALE, ParameterValueType::Number)
                .with_default("1")
                .with_range(-64.0, 64.0)
                .with_description("multiplier applied to the mapped output"),
        )
        .with_parameter(
            ParameterMetadata::new(OFFSET, ParameterValueType::Number)
                .with_default("0")
                .with_range(-64.0, 64.0)
                .with_description("value added to the scaled output"),
        )
        .with_example(
            "- id: mapper\n  type: curve_mapper\n  parameters:\n    curve: s_curve\n    steps: 4",
        )
}

fn decay_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(module_types::DECAY)
        .with_execution_scope(ExecutionScope::Voice)
        .with_inputs([(TRIGGER, Event), (TIME_MS, Control)])
        .with_output(Port::output(VALUE, Control))
        .with_parameter(
            ParameterMetadata::new(TIME_MS, ParameterValueType::Number)
                .with_default("100")
                .with_range(1.0, 5000.0)
                .with_description("decay time in milliseconds")
                .with_realtime_note(
                    "updates the runtime decay length without rebuilding the graph",
                ),
        )
        .with_parameter(
            ParameterMetadata::new("curve", ParameterValueType::Text)
                .with_default("exponential")
                .with_enum_values(vec!["linear".to_string(), "exponential".to_string()])
                .with_description("decay curve shape"),
        )
}

fn reverb_definition() -> BuiltInModuleDefinition {
    BuiltInModuleDefinition::new(REVERB)
        .with_inputs([
            (AUDIO_IN_L, Audio),
            (AUDIO_IN_R, Audio),
            (DECAY_TIME, Control),
            (ROOM_SIZE, Control),
            (PRE_DELAY, Control),
            (DAMPING, Control),
            (DIFFUSION, Control),
            (STEREO_WIDTH, Control),
            (WET, Control),
            (DRY, Control),
        ])
        .with_output(Port::output(AUDIO_OUT_L, Audio))
        .with_output(Port::output(AUDIO_OUT_R, Audio))
        .with_parameter(interpolation_parameter())
}

#[cfg(test)]
pub(crate) fn build_definition() -> BuiltInModuleDefinition {
    gain_definition()
}

#[cfg(test)]
mod tests;
