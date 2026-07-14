use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::sync::Arc;

use crate::builtins::module_kind::ModuleKind;
use crate::builtins::{
    CURVE_LINEAR, CURVE_PARAMETER, DELAY_SAMPLES_PARAMETER, DETECTION_MODE_PARAMETER,
    DETECTION_MODE_RMS, DYNAMICS_DETECTION_PARAMETER, DYNAMICS_MODE_PARAMETER,
    DYNAMICS_MODE_TRANSIENT, DYNAMICS_TOPOLOGY_FEEDBACK, DYNAMICS_TOPOLOGY_PARAMETER,
    EVENT_FILTER_NOTE_PARAMETER, EVENT_FILTER_NOTE_SELECTOR, EVENT_FILTER_SELECTOR_PARAMETER,
    FILTER_ALGORITHM_BIQUAD, FILTER_ALGORITHM_COMB, FILTER_ALGORITHM_PARAMETER,
    FILTER_COMB_TYPE_PARAMETER, FILTER_MODE_HIGHPASS, FILTER_MODE_PARAMETER, FILTER_MODE_PEAKING,
    INTERPOLATION_CUBIC, INTERPOLATION_PARAMETER, NOISE_DEFAULT_SEED, NOISE_SEED_PARAMETER,
    SCRIPT_SOURCE_PARAMETER, SPECTRAL_DEFAULT_FFT_SIZE, SPECTRAL_FFT_SIZE_PARAMETER,
    SPECTRAL_MODE_PARAMETER, SPECTRAL_MODE_PASSTHROUGH, STEPS_PARAMETER, WAVEFORM_PARAMETER,
};
use crate::curve_mapper::{CurveKind, CurveMapper};
use crate::decay::DecayCurve;
use crate::delay_line::InterpolationMode;
use crate::diagnostics::{Diagnostic, Severity, error_codes};
use crate::dynamics_processor::{ProcessorMode, Topology};
use crate::envelope_follower::DetectionMode;
use crate::filter::{BiquadMode, CombType};
use crate::graph::{ExecutionScope, Graph, ModuleId, ModuleNode, SignalType, builtin_ports};
use crate::kernel::PolyAllocationPolicy;
use crate::kernel::StaticValue;
use crate::kernel::flatten::FlattenedGraph;
use crate::oscillator::Waveform;
use crate::patch::RenderSettings;
use crate::sample::{LoadedSample, PreparedSamplerAssets};
use crate::spectral::SpectralMode;

pub type ExecutionStep = usize;

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledPatch {
    nodes: Vec<CompiledNode>,
    topological_order: Vec<ExecutionStep>,
    execution_order: Vec<ExecutionStep>,
    voice_node_indices: Vec<usize>,
    global_node_indices: Vec<usize>,
    midi_input_index: Option<usize>,
    audio_output_index: Option<usize>,
    module_output_buffer_layout: Vec<CompiledModuleBufferLayout>,
    total_output_buffer_count: usize,
    render_settings: RenderSettings,
    parameter_slots: Vec<ParameterSlot>,
    root_bus_plan: RootBusPlan,
    poly_regions: Vec<CompiledPolyRegion>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledPolyRegion {
    node_id: String,
    max_voices: usize,
    allocation_policy: PolyAllocationPolicy,
    flattened_voice: FlattenedGraph,
    child_patch: Box<CompiledPatch>,
    child_schedule: Box<[ExecutionStep]>,
    voices: Box<[CompiledPolyVoiceStorage]>,
    event_queue_capacity: usize,
    output_accumulators: Box<[CompiledPolyOutputAccumulator]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledPolyVoiceStorage {
    state_range: std::ops::Range<usize>,
    audio_buffer_range: std::ops::Range<usize>,
    event_queue_range: std::ops::Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledPolyOutputAccumulator {
    name: String,
    signal_type: SignalType,
    span: CompiledPortSpan,
}

impl CompiledPolyRegion {
    pub(crate) fn new(
        node_id: impl Into<String>,
        max_voices: usize,
        allocation_policy: PolyAllocationPolicy,
        flattened_voice: FlattenedGraph,
        child_patch: CompiledPatch,
        voices: Vec<CompiledPolyVoiceStorage>,
        event_queue_capacity: usize,
        output_accumulators: Vec<CompiledPolyOutputAccumulator>,
    ) -> Self {
        let child_schedule = child_patch.execution_order().to_vec().into_boxed_slice();
        Self {
            node_id: node_id.into(),
            max_voices,
            allocation_policy,
            flattened_voice,
            child_patch: Box::new(child_patch),
            child_schedule,
            voices: voices.into_boxed_slice(),
            event_queue_capacity,
            output_accumulators: output_accumulators.into_boxed_slice(),
        }
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }
    pub fn max_voices(&self) -> usize {
        self.max_voices
    }
    pub fn allocation_policy(&self) -> PolyAllocationPolicy {
        self.allocation_policy
    }
    pub fn flattened_voice(&self) -> &FlattenedGraph {
        &self.flattened_voice
    }
    pub fn child_patch(&self) -> &CompiledPatch {
        &self.child_patch
    }
    pub fn child_schedule(&self) -> &[ExecutionStep] {
        &self.child_schedule
    }
    pub fn voices(&self) -> &[CompiledPolyVoiceStorage] {
        &self.voices
    }
    pub fn event_queue_capacity(&self) -> usize {
        self.event_queue_capacity
    }
    pub fn output_accumulators(&self) -> &[CompiledPolyOutputAccumulator] {
        &self.output_accumulators
    }
}

impl CompiledPolyVoiceStorage {
    pub(crate) fn new(
        state_range: std::ops::Range<usize>,
        audio_buffer_range: std::ops::Range<usize>,
        event_queue_range: std::ops::Range<usize>,
    ) -> Self {
        Self {
            state_range,
            audio_buffer_range,
            event_queue_range,
        }
    }

    pub fn state_range(&self) -> std::ops::Range<usize> {
        self.state_range.clone()
    }
    pub fn audio_buffer_range(&self) -> std::ops::Range<usize> {
        self.audio_buffer_range.clone()
    }
    pub fn event_queue_range(&self) -> std::ops::Range<usize> {
        self.event_queue_range.clone()
    }
}

impl CompiledPolyOutputAccumulator {
    pub(crate) fn new(
        name: impl Into<String>,
        signal_type: SignalType,
        span: CompiledPortSpan,
    ) -> Self {
        Self {
            name: name.into(),
            signal_type,
            span,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn signal_type(&self) -> SignalType {
        self.signal_type
    }
    pub fn span(&self) -> CompiledPortSpan {
        self.span
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledNode {
    pub id: ModuleId,
    pub module_type: String,
    pub module_kind: ModuleKind,
    pub execution_scope: ExecutionScope,
    pub input_port_map: Vec<Vec<CompiledPortRef>>,
    pub input_routes: Vec<Vec<CompiledInputSource>>,
    pub output_port_map: Vec<usize>,
    pub input_port_spans: Vec<CompiledPortSpan>,
    pub output_port_spans: Vec<CompiledPortSpan>,
    pub input_port_indices: BTreeMap<String, usize>,
    pub input_port_names: Vec<String>,
    pub input_port_types: Vec<SignalType>,
    pub output_port_names: Vec<String>,
    pub output_port_types: Vec<SignalType>,
    pub construction: CompiledConstruction,
    pub control_defaults: Vec<CompiledControlDefault>,
    pub resources: CompiledResourceHandles,
    /// Transitional source data retained for legacy callers until task 7.8.
    pub parameters: BTreeMap<String, String>,
    pub parameter_slot_indices: BTreeMap<String, usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ControlSlotId(usize);

impl ControlSlotId {
    pub(crate) fn from_index(index: usize) -> Self {
        Self(index)
    }

    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompiledControlDefault {
    pub input_port_index: usize,
    pub slot: ControlSlotId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompiledFilterAlgorithm {
    Moog,
    Biquad(BiquadMode),
    Comb(CombType),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompiledConstruction {
    None,
    Script {
        language: CompiledScriptLanguage,
        source: String,
    },
    Oscillator {
        waveform: Waveform,
    },
    CompensationDelay {
        samples: usize,
    },
    Dynamics {
        mode: ProcessorMode,
        detection: DetectionMode,
        topology: Topology,
    },
    Filter {
        algorithm: CompiledFilterAlgorithm,
    },
    Echo {
        interpolation: InterpolationMode,
    },
    Reverb {
        interpolation: InterpolationMode,
    },
    SpectralProcessor {
        fft_size: usize,
        mode: SpectralMode,
    },
    Noise {
        seed: u32,
    },
    EventFilter {
        note: Option<u8>,
    },
    EnvelopeFollower {
        mode: DetectionMode,
    },
    CurveMapper {
        curve: CurveKind,
        steps: u32,
    },
    Decay {
        curve: DecayCurve,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompiledScriptLanguage {
    Rhai,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SampleResourceHandle(Arc<LoadedSample>);

impl SampleResourceHandle {
    pub fn new(sample: LoadedSample) -> Self {
        Self(Arc::new(sample))
    }

    pub fn sample(&self) -> &LoadedSample {
        &self.0
    }

    pub fn frames(&self) -> &[f32] {
        self.0.frames()
    }

    pub fn sample_rate_hz(&self) -> u32 {
        self.0.sample_rate_hz()
    }

    pub(crate) fn from_shared(sample: Arc<LoadedSample>) -> Self {
        Self(sample)
    }

    #[cfg(test)]
    pub(crate) fn shares_data_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl From<LoadedSample> for SampleResourceHandle {
    fn from(sample: LoadedSample) -> Self {
        Self::new(sample)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImpulseResponseResourceHandle(Arc<LoadedSample>);

impl ImpulseResponseResourceHandle {
    pub fn new(sample: LoadedSample) -> Self {
        Self(Arc::new(sample))
    }

    pub fn sample(&self) -> &LoadedSample {
        &self.0
    }

    pub(crate) fn from_shared(sample: Arc<LoadedSample>) -> Self {
        Self(sample)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompiledResourceHandles {
    pub sample: Option<SampleResourceHandle>,
    pub impulse_response: Option<ImpulseResponseResourceHandle>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledNodeData {
    pub construction: CompiledConstruction,
    pub control_defaults: BTreeMap<String, f32>,
    pub resources: CompiledResourceHandles,
    pub port_channels: BTreeMap<String, usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompiledPortSpan {
    pub first_buffer: usize,
    pub channel_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RootBusPlan {
    inputs: Vec<CompiledRootPort>,
    outputs: Vec<CompiledRootPort>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledRootPort {
    name: String,
    channel_count: usize,
    span: Option<CompiledPortSpan>,
    bound: bool,
}

impl CompiledRootPort {
    pub(crate) fn new(
        name: impl Into<String>,
        channel_count: usize,
        span: Option<CompiledPortSpan>,
        bound: bool,
    ) -> Self {
        Self {
            name: name.into(),
            channel_count,
            span,
            bound,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn channel_count(&self) -> usize {
        self.channel_count
    }
    pub fn span(&self) -> Option<CompiledPortSpan> {
        self.span
    }
    pub fn is_bound(&self) -> bool {
        self.bound
    }
}

impl RootBusPlan {
    pub(crate) fn new(inputs: Vec<CompiledRootPort>, outputs: Vec<CompiledRootPort>) -> Self {
        Self { inputs, outputs }
    }

    pub fn inputs(&self) -> &[CompiledRootPort] {
        &self.inputs
    }
    pub fn outputs(&self) -> &[CompiledRootPort] {
        &self.outputs
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParameterSlot {
    value: f32,
}

impl Eq for ParameterSlot {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompiledPortRef {
    pub module_index: usize,
    pub port_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledInputSource {
    pub module_index: usize,
    pub port_index: usize,
    pub output_buffer_id: usize,
    pub output_port_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompiledModuleBufferLayout {
    pub output_buffer_start: usize,
    pub output_buffer_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompileError {
    MissingPort {
        module_id: String,
        port_name: String,
    },
    CycleDetected,
    UnknownModuleType {
        module_type: String,
    },
    UnsupportedModuleType {
        module_type: String,
    },
    InvalidConstructionData {
        module_id: String,
        parameter_name: String,
    },
}

impl CompileError {
    #[allow(dead_code)]
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            Self::MissingPort {
                module_id,
                port_name,
            } => Diagnostic::new(
                error_codes::GRAPH_MISSING_PORT,
                Severity::Error,
                format!("missing port: {module_id}.{port_name}"),
            )
            .with_module_id(module_id.clone())
            .with_port_name(port_name.clone()),
            Self::CycleDetected => Diagnostic::new(
                error_codes::GRAPH_CYCLE_DETECTED,
                Severity::Error,
                "routing cycle detected during compilation",
            ),
            Self::UnknownModuleType { module_type } => Diagnostic::new(
                error_codes::GRAPH_UNKNOWN_MODULE_TYPE,
                Severity::Error,
                format!("unknown module type: {module_type}"),
            ),
            Self::UnsupportedModuleType { module_type } => Diagnostic::new(
                error_codes::GRAPH_UNSUPPORTED_MODULE_TYPE,
                Severity::Error,
                format!("unsupported module type for rendering: {module_type}"),
            ),
            Self::InvalidConstructionData {
                module_id,
                parameter_name,
            } => Diagnostic::new(
                error_codes::LOADING,
                Severity::Error,
                format!("invalid construction value for {module_id}.{parameter_name}"),
            )
            .with_module_id(module_id)
            .with_port_name(parameter_name),
        }
    }
}

impl CompiledNodeData {
    pub(crate) fn none() -> Self {
        Self {
            construction: CompiledConstruction::None,
            control_defaults: BTreeMap::new(),
            resources: CompiledResourceHandles::default(),
            port_channels: BTreeMap::new(),
        }
    }

    pub(crate) fn from_kernel(
        module_id: &str,
        kind: ModuleKind,
        static_args: &BTreeMap<String, StaticValue>,
        control_defaults: &BTreeMap<String, f64>,
    ) -> Result<Self, CompileError> {
        Ok(Self {
            construction: construction_from_static_values(module_id, kind, static_args)?,
            control_defaults: control_defaults
                .iter()
                .map(|(name, value)| (name.clone(), *value as f32))
                .collect(),
            resources: CompiledResourceHandles::default(),
            port_channels: BTreeMap::new(),
        })
    }

    pub(crate) fn compensation_delay(samples: u32) -> Self {
        Self {
            construction: CompiledConstruction::CompensationDelay {
                samples: samples as usize,
            },
            control_defaults: BTreeMap::new(),
            resources: CompiledResourceHandles::default(),
            port_channels: BTreeMap::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_legacy(module: &ModuleNode) -> Result<Self, CompileError> {
        let kind = ModuleKind::from_str(module.module_type()).ok_or_else(|| {
            CompileError::UnknownModuleType {
                module_type: module.module_type().to_string(),
            }
        })?;
        legacy_node_data(module.id().as_str(), kind, module)
    }
}

fn construction_from_static_values(
    module_id: &str,
    kind: ModuleKind,
    args: &BTreeMap<String, StaticValue>,
) -> Result<CompiledConstruction, CompileError> {
    let enum_value = |name: &str| match args.get(name) {
        Some(StaticValue::Enum(value)) => Some(value.as_str()),
        _ => None,
    };
    let string_value = |name: &str| match args.get(name) {
        Some(StaticValue::String(value)) => Some(value.as_str()),
        _ => None,
    };
    let int_value = |name: &str| match args.get(name) {
        Some(StaticValue::Int(value)) => Some(*value),
        _ => None,
    };
    let invalid = |parameter_name: &str| CompileError::InvalidConstructionData {
        module_id: module_id.to_string(),
        parameter_name: parameter_name.to_string(),
    };

    Ok(match kind {
        ModuleKind::Script => CompiledConstruction::Script {
            language: match enum_value(crate::builtins::SCRIPT_LANGUAGE_PARAMETER) {
                Some(crate::builtins::SCRIPT_LANGUAGE_RHAI) => CompiledScriptLanguage::Rhai,
                _ => return Err(invalid(crate::builtins::SCRIPT_LANGUAGE_PARAMETER)),
            },
            source: string_value(SCRIPT_SOURCE_PARAMETER)
                .ok_or_else(|| invalid(SCRIPT_SOURCE_PARAMETER))?
                .to_string(),
        },
        ModuleKind::Oscillator => CompiledConstruction::Oscillator {
            waveform: enum_value(WAVEFORM_PARAMETER)
                .and_then(Waveform::from_str)
                .unwrap_or(Waveform::DEFAULT),
        },
        ModuleKind::CompensationDelay => CompiledConstruction::CompensationDelay {
            samples: usize::try_from(
                int_value(DELAY_SAMPLES_PARAMETER)
                    .ok_or_else(|| invalid(DELAY_SAMPLES_PARAMETER))?,
            )
            .ok()
            .filter(|samples| *samples > 0)
            .ok_or_else(|| invalid(DELAY_SAMPLES_PARAMETER))?,
        },
        ModuleKind::DynamicsProcessor => CompiledConstruction::Dynamics {
            mode: match enum_value(DYNAMICS_MODE_PARAMETER) {
                Some(DYNAMICS_MODE_TRANSIENT) => ProcessorMode::Transient,
                _ => ProcessorMode::Level,
            },
            detection: match enum_value(DYNAMICS_DETECTION_PARAMETER) {
                Some(DETECTION_MODE_RMS) => DetectionMode::Rms,
                _ => DetectionMode::Peak,
            },
            topology: match enum_value(DYNAMICS_TOPOLOGY_PARAMETER) {
                Some(DYNAMICS_TOPOLOGY_FEEDBACK) => Topology::Feedback,
                _ => Topology::Feedforward,
            },
        },
        ModuleKind::Filter => CompiledConstruction::Filter {
            algorithm: filter_construction(
                enum_value(FILTER_ALGORITHM_PARAMETER),
                enum_value(FILTER_MODE_PARAMETER),
                enum_value(FILTER_COMB_TYPE_PARAMETER),
            ),
        },
        ModuleKind::Echo => CompiledConstruction::Echo {
            interpolation: interpolation_construction(enum_value(INTERPOLATION_PARAMETER)),
        },
        ModuleKind::Reverb => CompiledConstruction::Reverb {
            interpolation: interpolation_construction(enum_value(INTERPOLATION_PARAMETER)),
        },
        ModuleKind::SpectralProcessor => CompiledConstruction::SpectralProcessor {
            fft_size: int_value(SPECTRAL_FFT_SIZE_PARAMETER)
                .and_then(|value| usize::try_from(value).ok())
                .unwrap_or(SPECTRAL_DEFAULT_FFT_SIZE),
            mode: match enum_value(SPECTRAL_MODE_PARAMETER) {
                Some(SPECTRAL_MODE_PASSTHROUGH) => SpectralMode::Passthrough,
                _ => SpectralMode::Gate,
            },
        },
        ModuleKind::Noise => CompiledConstruction::Noise {
            seed: int_value(NOISE_SEED_PARAMETER)
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(NOISE_DEFAULT_SEED),
        },
        ModuleKind::EventFilter => CompiledConstruction::EventFilter {
            note: if enum_value(EVENT_FILTER_SELECTOR_PARAMETER)
                .unwrap_or(EVENT_FILTER_NOTE_SELECTOR)
                == EVENT_FILTER_NOTE_SELECTOR
            {
                int_value(EVENT_FILTER_NOTE_PARAMETER).and_then(|value| u8::try_from(value).ok())
            } else {
                None
            },
        },
        ModuleKind::EnvelopeFollower => CompiledConstruction::EnvelopeFollower {
            mode: match enum_value(DETECTION_MODE_PARAMETER) {
                Some(DETECTION_MODE_RMS) => DetectionMode::Rms,
                _ => DetectionMode::Peak,
            },
        },
        ModuleKind::CurveMapper => CompiledConstruction::CurveMapper {
            curve: enum_value(CURVE_PARAMETER)
                .and_then(CurveKind::from_str)
                .unwrap_or(CurveKind::Linear),
            steps: int_value(STEPS_PARAMETER)
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(CurveMapper::DEFAULT_STEPS),
        },
        ModuleKind::Decay => CompiledConstruction::Decay {
            curve: enum_value(CURVE_PARAMETER)
                .and_then(DecayCurve::from_str)
                .unwrap_or(DecayCurve::Exponential),
        },
        _ => CompiledConstruction::None,
    })
}

fn legacy_node_data(
    module_id: &str,
    kind: ModuleKind,
    module: &ModuleNode,
) -> Result<CompiledNodeData, CompileError> {
    let params = module.params();
    let construction = construction_from_legacy_values(module_id, kind, params)?;
    let mut control_defaults: BTreeMap<String, f32> = module
        .inputs()
        .iter()
        .filter(|port| port.signal_type() == SignalType::Control)
        .filter_map(|port| {
            params
                .get(port.name())
                .and_then(|value| value.parse::<f32>().ok())
                .or_else(|| effective_legacy_control_default(kind, port.name()))
                .map(|value| (port.name().to_string(), value))
        })
        .collect();
    for (name, value) in params {
        if is_static_construction_parameter(kind, name) {
            continue;
        }
        if let Ok(value) = value.parse::<f32>() {
            control_defaults.entry(name.clone()).or_insert(value);
        }
    }
    Ok(CompiledNodeData {
        construction,
        control_defaults,
        resources: CompiledResourceHandles::default(),
        port_channels: BTreeMap::new(),
    })
}

fn is_static_construction_parameter(kind: ModuleKind, name: &str) -> bool {
    match kind {
        ModuleKind::Script => matches!(
            name,
            crate::builtins::SCRIPT_LANGUAGE_PARAMETER | SCRIPT_SOURCE_PARAMETER
        ),
        ModuleKind::Oscillator => name == WAVEFORM_PARAMETER,
        ModuleKind::CompensationDelay => name == DELAY_SAMPLES_PARAMETER,
        ModuleKind::DynamicsProcessor => matches!(
            name,
            DYNAMICS_MODE_PARAMETER | DYNAMICS_DETECTION_PARAMETER | DYNAMICS_TOPOLOGY_PARAMETER
        ),
        ModuleKind::Filter => matches!(
            name,
            FILTER_ALGORITHM_PARAMETER | FILTER_MODE_PARAMETER | FILTER_COMB_TYPE_PARAMETER
        ),
        ModuleKind::Echo | ModuleKind::Reverb => name == INTERPOLATION_PARAMETER,
        ModuleKind::SpectralProcessor => matches!(
            name,
            SPECTRAL_FFT_SIZE_PARAMETER
                | SPECTRAL_MODE_PARAMETER
                | crate::builtins::SPECTRAL_WINDOW_PARAMETER
        ),
        ModuleKind::Noise => name == NOISE_SEED_PARAMETER,
        ModuleKind::EventFilter => {
            matches!(
                name,
                EVENT_FILTER_SELECTOR_PARAMETER | EVENT_FILTER_NOTE_PARAMETER
            )
        }
        ModuleKind::EnvelopeFollower => name == DETECTION_MODE_PARAMETER,
        ModuleKind::CurveMapper => matches!(name, CURVE_PARAMETER | STEPS_PARAMETER),
        ModuleKind::Decay => name == CURVE_PARAMETER,
        _ => false,
    }
}

fn construction_from_legacy_values(
    module_id: &str,
    kind: ModuleKind,
    params: &BTreeMap<String, String>,
) -> Result<CompiledConstruction, CompileError> {
    let value = |name: &str| params.get(name).map(String::as_str);
    let invalid = |parameter_name: &str| CompileError::InvalidConstructionData {
        module_id: module_id.to_string(),
        parameter_name: parameter_name.to_string(),
    };

    Ok(match kind {
        ModuleKind::Script => CompiledConstruction::Script {
            language: match value(crate::builtins::SCRIPT_LANGUAGE_PARAMETER) {
                None | Some(crate::builtins::SCRIPT_LANGUAGE_RHAI) => CompiledScriptLanguage::Rhai,
                _ => return Err(invalid(crate::builtins::SCRIPT_LANGUAGE_PARAMETER)),
            },
            source: value(SCRIPT_SOURCE_PARAMETER)
                .ok_or_else(|| invalid(SCRIPT_SOURCE_PARAMETER))?
                .to_string(),
        },
        ModuleKind::Oscillator => CompiledConstruction::Oscillator {
            waveform: value(WAVEFORM_PARAMETER)
                .and_then(Waveform::from_str)
                .unwrap_or(Waveform::DEFAULT),
        },
        ModuleKind::CompensationDelay => CompiledConstruction::CompensationDelay {
            samples: value(DELAY_SAMPLES_PARAMETER)
                .and_then(|value| value.parse::<usize>().ok())
                .filter(|samples| *samples > 0)
                .ok_or_else(|| invalid(DELAY_SAMPLES_PARAMETER))?,
        },
        ModuleKind::DynamicsProcessor => CompiledConstruction::Dynamics {
            mode: match value(DYNAMICS_MODE_PARAMETER) {
                Some(DYNAMICS_MODE_TRANSIENT) => ProcessorMode::Transient,
                _ => ProcessorMode::Level,
            },
            detection: match value(DYNAMICS_DETECTION_PARAMETER) {
                Some(DETECTION_MODE_RMS) => DetectionMode::Rms,
                _ => DetectionMode::Peak,
            },
            topology: match value(DYNAMICS_TOPOLOGY_PARAMETER) {
                Some(DYNAMICS_TOPOLOGY_FEEDBACK) => Topology::Feedback,
                _ => Topology::Feedforward,
            },
        },
        ModuleKind::Filter => CompiledConstruction::Filter {
            algorithm: filter_construction(
                value(FILTER_ALGORITHM_PARAMETER),
                value(FILTER_MODE_PARAMETER),
                value(FILTER_COMB_TYPE_PARAMETER),
            ),
        },
        ModuleKind::Echo => CompiledConstruction::Echo {
            interpolation: interpolation_construction(value(INTERPOLATION_PARAMETER)),
        },
        ModuleKind::Reverb => CompiledConstruction::Reverb {
            interpolation: interpolation_construction(value(INTERPOLATION_PARAMETER)),
        },
        ModuleKind::SpectralProcessor => CompiledConstruction::SpectralProcessor {
            fft_size: value(SPECTRAL_FFT_SIZE_PARAMETER)
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(SPECTRAL_DEFAULT_FFT_SIZE),
            mode: match value(SPECTRAL_MODE_PARAMETER) {
                Some(SPECTRAL_MODE_PASSTHROUGH) => SpectralMode::Passthrough,
                _ => SpectralMode::Gate,
            },
        },
        ModuleKind::Noise => CompiledConstruction::Noise {
            seed: value(NOISE_SEED_PARAMETER)
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(NOISE_DEFAULT_SEED),
        },
        ModuleKind::EventFilter => CompiledConstruction::EventFilter {
            note: if value(EVENT_FILTER_SELECTOR_PARAMETER).unwrap_or(EVENT_FILTER_NOTE_SELECTOR)
                == EVENT_FILTER_NOTE_SELECTOR
            {
                value(EVENT_FILTER_NOTE_PARAMETER).and_then(|value| value.parse::<u8>().ok())
            } else {
                None
            },
        },
        ModuleKind::EnvelopeFollower => CompiledConstruction::EnvelopeFollower {
            mode: match value(DETECTION_MODE_PARAMETER) {
                Some(DETECTION_MODE_RMS) => DetectionMode::Rms,
                _ => DetectionMode::Peak,
            },
        },
        ModuleKind::CurveMapper => CompiledConstruction::CurveMapper {
            curve: value(CURVE_PARAMETER)
                .and_then(CurveKind::from_str)
                .unwrap_or_else(|| CurveKind::from_str(CURVE_LINEAR).unwrap()),
            steps: value(STEPS_PARAMETER)
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(CurveMapper::DEFAULT_STEPS),
        },
        ModuleKind::Decay => CompiledConstruction::Decay {
            curve: value(CURVE_PARAMETER)
                .and_then(DecayCurve::from_str)
                .unwrap_or(DecayCurve::Exponential),
        },
        _ => CompiledConstruction::None,
    })
}

fn filter_construction(
    algorithm: Option<&str>,
    mode: Option<&str>,
    comb_type: Option<&str>,
) -> CompiledFilterAlgorithm {
    match algorithm {
        Some(FILTER_ALGORITHM_BIQUAD) => CompiledFilterAlgorithm::Biquad(match mode {
            Some(FILTER_MODE_HIGHPASS) => BiquadMode::Highpass,
            Some(FILTER_MODE_PEAKING) => BiquadMode::Peaking,
            _ => BiquadMode::Lowpass,
        }),
        Some(FILTER_ALGORITHM_COMB) => CompiledFilterAlgorithm::Comb(match comb_type {
            Some(crate::builtins::DYNAMICS_TOPOLOGY_FEEDFORWARD) => CombType::Feedforward,
            _ => CombType::Feedback,
        }),
        _ => CompiledFilterAlgorithm::Moog,
    }
}

fn interpolation_construction(value: Option<&str>) -> InterpolationMode {
    match value {
        Some(INTERPOLATION_CUBIC) => InterpolationMode::Cubic,
        _ => InterpolationMode::Linear,
    }
}

pub(crate) fn effective_legacy_control_default(
    module_kind: ModuleKind,
    port_name: &str,
) -> Option<f32> {
    match (module_kind, port_name) {
        (ModuleKind::Oscillator, builtin_ports::PITCH) => Some(1.0),
        (ModuleKind::ControlToAudio, builtin_ports::IN) => Some(0.0),
        (ModuleKind::Decay, builtin_ports::TIME_MS) => Some(100.0),
        (ModuleKind::Sampler, builtin_ports::RATE) => Some(1.0),
        (ModuleKind::EnvelopeFollower, builtin_ports::ATTACK) => Some(5.0),
        (ModuleKind::EnvelopeFollower, builtin_ports::RELEASE) => Some(50.0),
        (ModuleKind::EnvelopeFollower, builtin_ports::AMOUNT) => Some(1.0),
        (ModuleKind::EnvelopeFollower, builtin_ports::OFFSET) => Some(0.0),
        (ModuleKind::EnvelopeFollower, builtin_ports::INVERT) => Some(0.0),
        (ModuleKind::Gain, builtin_ports::GAIN) => Some(1.0),
        (ModuleKind::CurveMapper, builtin_ports::AMOUNT) => Some(1.0),
        (ModuleKind::CurveMapper, builtin_ports::BIAS) => Some(0.0),
        (ModuleKind::CurveMapper, builtin_ports::SCALE) => Some(1.0),
        (ModuleKind::CurveMapper, builtin_ports::OFFSET) => Some(0.0),
        (ModuleKind::DynamicsProcessor, builtin_ports::THRESHOLD) => Some(0.3),
        (ModuleKind::DynamicsProcessor, builtin_ports::BELOW_RATIO) => Some(0.05),
        (ModuleKind::DynamicsProcessor, builtin_ports::ABOVE_RATIO) => Some(0.077),
        (ModuleKind::DynamicsProcessor, builtin_ports::ATTACK) => Some(0.05),
        (ModuleKind::DynamicsProcessor, builtin_ports::RELEASE) => Some(0.1),
        (ModuleKind::DynamicsProcessor, builtin_ports::KNEE) => Some(0.0),
        (ModuleKind::DynamicsProcessor, builtin_ports::MAKEUP_GAIN) => Some(0.0),
        (ModuleKind::DynamicsProcessor, builtin_ports::ATTACK_GAIN) => Some(0.5),
        (ModuleKind::DynamicsProcessor, builtin_ports::SUSTAIN_GAIN) => Some(0.5),
        (ModuleKind::Filter, builtin_ports::CUTOFF) => Some(0.5),
        (ModuleKind::Filter, builtin_ports::RESONANCE) => Some(0.0),
        (ModuleKind::Filter, builtin_ports::GAIN) => Some(0.5),
        (ModuleKind::Saturator, builtin_ports::DRIVE) => Some(0.0),
        (ModuleKind::Saturator, builtin_ports::BIAS) => Some(0.0),
        (ModuleKind::Saturator, builtin_ports::CURVE_SELECT) => Some(0.0),
        (ModuleKind::Convolution, builtin_ports::MIX) => Some(1.0),
        (ModuleKind::Echo, builtin_ports::FEEDBACK) => Some(0.5),
        (ModuleKind::Echo, builtin_ports::DAMPING_CUTOFF) => Some(0.5),
        (ModuleKind::Echo, builtin_ports::WET) => Some(0.7),
        (ModuleKind::Echo, builtin_ports::DRY) => Some(0.5),
        (ModuleKind::Echo, builtin_ports::TIME_LEFT_MS) => Some(0.3),
        (ModuleKind::Echo, builtin_ports::TIME_RIGHT_MS) => Some(0.3),
        (ModuleKind::Echo, builtin_ports::PING_PONG) => Some(0.0),
        (ModuleKind::Reverb, builtin_ports::DECAY_TIME) => Some(0.35),
        (ModuleKind::Reverb, builtin_ports::ROOM_SIZE) => Some(0.7),
        (ModuleKind::Reverb, builtin_ports::DAMPING) => Some(0.3),
        (ModuleKind::Reverb, builtin_ports::DIFFUSION) => Some(0.5),
        (ModuleKind::Reverb, builtin_ports::WET) => Some(0.7),
        (ModuleKind::Reverb, builtin_ports::DRY) => Some(0.5),
        (ModuleKind::Reverb, builtin_ports::PRE_DELAY) => Some(0.0),
        (ModuleKind::Reverb, builtin_ports::STEREO_WIDTH) => Some(0.5),
        (ModuleKind::FrequencySplitter, builtin_ports::CROSSOVER_HZ) => Some(0.2),
        (ModuleKind::SpectralProcessor, builtin_ports::THRESHOLD) => Some(0.5),
        (ModuleKind::SpectralProcessor, builtin_ports::MIX) => Some(0.5),
        (ModuleKind::Adsr, builtin_ports::ATTACK) => Some(5.0),
        (ModuleKind::Adsr, builtin_ports::DECAY) => Some(30.0),
        (ModuleKind::Adsr, builtin_ports::SUSTAIN) => Some(0.7),
        (ModuleKind::Adsr, builtin_ports::RELEASE) => Some(200.0),
        (ModuleKind::Slew, builtin_ports::VALUE) => Some(0.0),
        (ModuleKind::Slew, builtin_ports::GLIDE) => Some(0.0),
        (ModuleKind::Slew, builtin_ports::TIME_MS) => Some(60.0),
        _ => None,
    }
}

pub fn compile(
    graph: &Graph,
    render_settings: &RenderSettings,
) -> Result<CompiledPatch, CompileError> {
    compile_internal(graph, render_settings, None)
}

pub(crate) fn compile_with_node_data(
    graph: &Graph,
    render_settings: &RenderSettings,
    node_data: &BTreeMap<String, CompiledNodeData>,
) -> Result<CompiledPatch, CompileError> {
    compile_internal(graph, render_settings, Some(node_data))
}

fn compile_internal(
    graph: &Graph,
    render_settings: &RenderSettings,
    supplied_node_data: Option<&BTreeMap<String, CompiledNodeData>>,
) -> Result<CompiledPatch, CompileError> {
    let module_indices = module_indices_by_id(graph);
    let topological_order = topological_sort(graph, &module_indices)?;
    let (planned_output_spans, total_output_buffer_count) = color_output_spans(
        graph,
        &module_indices,
        &topological_order,
        supplied_node_data,
    )?;
    let mut module_output_buffer_layout = Vec::with_capacity(graph.modules().len());
    let mut parameter_slots = Vec::new();
    let nodes: Vec<_> = graph
        .modules()
        .iter()
        .map(|module| {
            let module_type_str = module.module_type();
            let kind = ModuleKind::from_str(module_type_str).ok_or_else(|| {
                CompileError::UnknownModuleType {
                    module_type: module_type_str.to_string(),
                }
            })?;
            if !kind.is_render_supported() {
                return Err(CompileError::UnsupportedModuleType {
                    module_type: module_type_str.to_string(),
                });
            }
            let input_count = module.inputs().len();
            let output_channel_counts = module
                .outputs()
                .iter()
                .map(|port| {
                    if port.signal_type() == SignalType::Event {
                        0
                    } else {
                        data_channel_count(supplied_node_data, module, port.name())
                    }
                })
                .collect::<Vec<_>>();
            let module_index = module_indices[module.id().as_str()];
            let output_port_spans = planned_output_spans[module_index].clone();
            let output_buffer_start = output_port_spans
                .iter()
                .map(|span| span.first_buffer)
                .min()
                .unwrap_or(0);
            let output_count = output_channel_counts.iter().sum();
            module_output_buffer_layout.push(CompiledModuleBufferLayout {
                output_buffer_start,
                output_buffer_count: output_count,
            });
            let input_port_names: Vec<String> = module
                .inputs()
                .iter()
                .map(|p| p.name().to_string())
                .collect();
            let output_port_names: Vec<String> = module
                .outputs()
                .iter()
                .map(|p| p.name().to_string())
                .collect();
            let input_port_indices = input_port_names
                .iter()
                .enumerate()
                .map(|(index, name)| (name.clone(), index))
                .collect();
            let parameters = if supplied_node_data.is_some() {
                BTreeMap::new()
            } else {
                module.params().clone()
            };
            let data = match supplied_node_data {
                Some(all_data) => all_data.get(module.id().as_str()).cloned().ok_or_else(|| {
                    CompileError::InvalidConstructionData {
                        module_id: module.id().as_str().to_string(),
                        parameter_name: "compiled_node_data".to_string(),
                    }
                })?,
                None => legacy_node_data(module.id().as_str(), kind, module)?,
            };
            let input_channel_counts = module
                .inputs()
                .iter()
                .map(|port| {
                    if port.signal_type() == SignalType::Event {
                        0
                    } else {
                        data.port_channels.get(port.name()).copied().unwrap_or(1)
                    }
                })
                .collect::<Vec<_>>();
            let mut next_input_channel = 0;
            let input_port_spans = input_channel_counts
                .iter()
                .map(|channel_count| {
                    let span = CompiledPortSpan {
                        first_buffer: next_input_channel,
                        channel_count: *channel_count,
                    };
                    next_input_channel += channel_count;
                    span
                })
                .collect();
            let mut control_defaults = Vec::new();
            let mut parameter_slot_indices = BTreeMap::new();
            for (name, value) in &data.control_defaults {
                let slot_index = parameter_slots.len();
                parameter_slots.push(ParameterSlot { value: *value });
                parameter_slot_indices.insert(name.clone(), slot_index);
                if let Some(input_port_index) =
                    input_port_names.iter().position(|port| port == name)
                {
                    control_defaults.push(CompiledControlDefault {
                        input_port_index,
                        slot: ControlSlotId::from_index(slot_index),
                    });
                }
            }

            Ok(CompiledNode {
                id: module.id().clone(),
                module_type: module_type_str.to_string(),
                module_kind: kind,
                execution_scope: module.execution_scope(),
                input_port_map: vec![Vec::new(); input_count],
                input_routes: vec![Vec::new(); input_count],
                output_port_map: output_port_spans
                    .iter()
                    .flat_map(|span| span.first_buffer..span.first_buffer + span.channel_count)
                    .collect(),
                input_port_spans,
                output_port_spans,
                input_port_indices,
                input_port_names,
                input_port_types: module.inputs().iter().map(|p| p.signal_type()).collect(),
                output_port_names,
                output_port_types: module.outputs().iter().map(|p| p.signal_type()).collect(),
                construction: data.construction,
                control_defaults,
                resources: data.resources,
                parameters,
                parameter_slot_indices,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut nodes = nodes;

    resolve_routing(graph, &module_indices, &mut nodes)?;

    let global_node_indices = topological_order
        .iter()
        .copied()
        .filter(|index| nodes[*index].execution_scope == ExecutionScope::Global)
        .collect::<Vec<_>>();
    let voice_node_indices = topological_order
        .iter()
        .copied()
        .filter(|index| nodes[*index].execution_scope == ExecutionScope::Voice)
        .collect::<Vec<_>>();
    let execution_order = global_node_indices
        .iter()
        .chain(voice_node_indices.iter())
        .copied()
        .collect();

    Ok(CompiledPatch {
        nodes,
        topological_order,
        execution_order,
        voice_node_indices,
        global_node_indices,
        midi_input_index: graph
            .modules()
            .iter()
            .position(|module| module.module_type() == "midi_input"),
        audio_output_index: graph
            .modules()
            .iter()
            .position(|module| module.module_type() == "audio_output"),
        module_output_buffer_layout,
        total_output_buffer_count,
        render_settings: render_settings.clone(),
        parameter_slots,
        root_bus_plan: RootBusPlan::default(),
        poly_regions: Vec::new(),
    })
}

fn color_output_spans(
    graph: &Graph,
    module_indices: &BTreeMap<&str, usize>,
    topological_order: &[usize],
    supplied_node_data: Option<&BTreeMap<String, CompiledNodeData>>,
) -> Result<(Vec<Vec<CompiledPortSpan>>, usize), CompileError> {
    let order_position = topological_order
        .iter()
        .enumerate()
        .map(|(position, module_index)| (*module_index, position))
        .collect::<BTreeMap<_, _>>();
    let mut intervals = Vec::new();
    for (module_index, module) in graph.modules().iter().enumerate() {
        for (port_index, port) in module.outputs().iter().enumerate() {
            if port.signal_type() == SignalType::Event {
                continue;
            }
            let start = order_position[&module_index];
            let end = graph
                .cables()
                .iter()
                .filter(|cable| {
                    cable.source().module_id() == module.id()
                        && cable.source().port_name() == port.name()
                })
                .filter_map(|cable| module_indices.get(cable.destination().module_id().as_str()))
                .map(|destination| order_position[destination])
                .max()
                .unwrap_or(topological_order.len());
            intervals.push((
                start,
                module_index,
                port_index,
                end,
                data_channel_count(supplied_node_data, module, port.name()),
            ));
        }
    }
    intervals.sort_by_key(|(start, module, port, _, _)| (*start, *module, *port));

    let mut spans = graph
        .modules()
        .iter()
        .map(|module| {
            vec![
                CompiledPortSpan {
                    first_buffer: 0,
                    channel_count: 0
                };
                module.outputs().len()
            ]
        })
        .collect::<Vec<_>>();
    let mut active: Vec<(usize, CompiledPortSpan)> = Vec::new();
    let mut buffer_count = 0;
    for (start, module_index, port_index, end, channel_count) in intervals {
        active.retain(|(active_end, _)| *active_end >= start);
        let mut first_buffer = 0;
        loop {
            let candidate_end = first_buffer + channel_count;
            let overlap = active.iter().find(|(_, span)| {
                first_buffer < span.first_buffer + span.channel_count
                    && span.first_buffer < candidate_end
            });
            match overlap {
                Some((_, span)) => first_buffer = span.first_buffer + span.channel_count,
                None => break,
            }
        }
        let span = CompiledPortSpan {
            first_buffer,
            channel_count,
        };
        spans[module_index][port_index] = span;
        active.push((end, span));
        active.sort_by_key(|(_, span)| span.first_buffer);
        buffer_count = buffer_count.max(first_buffer + channel_count);
    }
    Ok((spans, buffer_count))
}

fn data_channel_count(
    supplied_node_data: Option<&BTreeMap<String, CompiledNodeData>>,
    module: &ModuleNode,
    port_name: &str,
) -> usize {
    supplied_node_data
        .and_then(|all_data| all_data.get(module.id().as_str()))
        .and_then(|data| data.port_channels.get(port_name))
        .copied()
        .unwrap_or(1)
}

impl CompiledPatch {
    pub fn nodes(&self) -> &[CompiledNode] {
        &self.nodes
    }

    pub fn topological_order(&self) -> &[ExecutionStep] {
        &self.topological_order
    }

    pub fn execution_order(&self) -> &[ExecutionStep] {
        &self.execution_order
    }

    pub fn voice_node_indices(&self) -> &[usize] {
        &self.voice_node_indices
    }

    pub fn global_node_indices(&self) -> &[usize] {
        &self.global_node_indices
    }

    pub fn midi_input_index(&self) -> Option<usize> {
        self.midi_input_index
    }

    pub fn audio_output_index(&self) -> Option<usize> {
        self.audio_output_index
    }

    pub fn module_output_buffer_layout(&self) -> &[CompiledModuleBufferLayout] {
        &self.module_output_buffer_layout
    }

    pub fn total_output_buffer_count(&self) -> usize {
        self.total_output_buffer_count
    }

    pub fn render_settings(&self) -> &RenderSettings {
        &self.render_settings
    }

    pub fn root_bus_plan(&self) -> &RootBusPlan {
        &self.root_bus_plan
    }

    pub fn poly_regions(&self) -> &[CompiledPolyRegion] {
        &self.poly_regions
    }

    pub(crate) fn set_poly_regions(&mut self, regions: Vec<CompiledPolyRegion>) {
        self.poly_regions = regions;
    }

    pub(crate) fn set_root_bus_plan(&mut self, plan: RootBusPlan) {
        self.root_bus_plan = plan;
    }

    pub(crate) fn reserve_root_input_span(
        &mut self,
        channel_count: usize,
        destinations: &[crate::kernel::PortRef],
    ) -> CompiledPortSpan {
        let span = CompiledPortSpan {
            first_buffer: self.total_output_buffer_count,
            channel_count,
        };
        self.total_output_buffer_count += channel_count;
        for destination in destinations {
            let Some(node_index) = self
                .nodes
                .iter()
                .position(|node| node.id.as_str() == destination.node().as_str())
            else {
                continue;
            };
            let Some(port_index) = self.nodes[node_index]
                .input_port_names
                .iter()
                .position(|name| name == destination.port())
            else {
                continue;
            };
            for channel in 0..channel_count {
                self.nodes[node_index].input_routes[port_index].push(CompiledInputSource {
                    module_index: usize::MAX,
                    port_index,
                    output_buffer_id: span.first_buffer + channel,
                    output_port_name: destination.port().to_string(),
                });
            }
        }
        span
    }

    pub fn parameter_slot_value(&self, slot_index: usize) -> Option<f32> {
        self.parameter_slots.get(slot_index).map(|slot| slot.value)
    }

    pub fn numeric_parameter_value(&self, module_id: &str, parameter_name: &str) -> Option<f32> {
        let slot_index = self.parameter_slot_index(module_id, parameter_name)?;
        self.parameter_slot_value(slot_index)
    }

    pub fn set_numeric_parameter_by_target(
        &mut self,
        module_id: &str,
        parameter_name: &str,
        value: f32,
    ) -> bool {
        let Some(slot_index) = self.parameter_slot_index(module_id, parameter_name) else {
            return false;
        };
        self.set_parameter_slot(slot_index, value)
    }

    /// O(1) parameter update by a previously-resolved slot index, with no string
    /// comparisons or module search. Safe to call from a realtime audio callback.
    pub fn set_parameter_slot(&mut self, slot_index: usize, value: f32) -> bool {
        let Some(slot) = self.parameter_slots.get_mut(slot_index) else {
            return false;
        };

        slot.value = value;
        true
    }

    pub fn parameter_slot_index(&self, module_id: &str, parameter_name: &str) -> Option<usize> {
        self.nodes
            .iter()
            .find(|node| node.id.as_str() == module_id)?
            .parameter_slot_indices
            .get(parameter_name)
            .copied()
    }

    pub(crate) fn attach_legacy_resources(&mut self, assets: &PreparedSamplerAssets) {
        for node in &mut self.nodes {
            let Some(sample) = assets.get(node.id.as_str()).cloned() else {
                continue;
            };
            match node.module_kind {
                ModuleKind::Sampler => {
                    node.resources.sample = Some(SampleResourceHandle::new(sample));
                }
                ModuleKind::Convolution => {
                    node.resources.impulse_response =
                        Some(ImpulseResponseResourceHandle::new(sample));
                }
                _ => {}
            }
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPort {
                module_id,
                port_name,
            } => write!(formatter, "missing port: {module_id}.{port_name}"),
            Self::CycleDetected => write!(formatter, "routing cycle detected"),
            Self::UnknownModuleType { module_type } => {
                write!(formatter, "unknown module type: {module_type}")
            }
            Self::UnsupportedModuleType { module_type } => {
                write!(formatter, "unsupported module type: {module_type}")
            }
            Self::InvalidConstructionData {
                module_id,
                parameter_name,
            } => write!(
                formatter,
                "invalid construction value for {module_id}.{parameter_name}"
            ),
        }
    }
}

impl std::error::Error for CompileError {}

fn module_indices_by_id(graph: &Graph) -> BTreeMap<&str, usize> {
    graph
        .modules()
        .iter()
        .enumerate()
        .map(|(index, module)| (module.id().as_str(), index))
        .collect()
}

fn topological_sort(
    graph: &Graph,
    module_indices: &BTreeMap<&str, usize>,
) -> Result<Vec<usize>, CompileError> {
    let module_count = graph.modules().len();
    let mut in_degree = vec![0usize; module_count];
    let mut adjacency = vec![Vec::new(); module_count];

    for cable in graph.cables() {
        let source = module_index(module_indices, cable.source().module_id().as_str(), "")?;
        let destination =
            module_index(module_indices, cable.destination().module_id().as_str(), "")?;
        adjacency[source].push(destination);
        in_degree[destination] += 1;
    }

    let mut ready = in_degree
        .iter()
        .enumerate()
        .filter_map(|(index, degree)| (*degree == 0).then_some(index))
        .collect::<VecDeque<_>>();
    let mut sorted = Vec::with_capacity(module_count);

    while let Some(index) = ready.pop_front() {
        sorted.push(index);

        for &next in &adjacency[index] {
            in_degree[next] -= 1;
            if in_degree[next] == 0 {
                ready.push_back(next);
            }
        }
    }

    if sorted.len() == module_count {
        Ok(sorted)
    } else {
        Err(CompileError::CycleDetected)
    }
}

fn resolve_routing(
    graph: &Graph,
    module_indices: &BTreeMap<&str, usize>,
    nodes: &mut [CompiledNode],
) -> Result<(), CompileError> {
    for cable in graph.cables() {
        let source_module_id = cable.source().module_id().as_str();
        let destination_module_id = cable.destination().module_id().as_str();
        let source_module_index =
            module_index(module_indices, source_module_id, cable.source().port_name())?;
        let destination_module_index = module_index(
            module_indices,
            destination_module_id,
            cable.destination().port_name(),
        )?;
        let source_port_index = graph.modules()[source_module_index]
            .outputs()
            .iter()
            .position(|port| port.name() == cable.source().port_name())
            .ok_or_else(|| CompileError::MissingPort {
                module_id: source_module_id.to_string(),
                port_name: cable.source().port_name().to_string(),
            })?;
        let destination_port_index = graph.modules()[destination_module_index]
            .inputs()
            .iter()
            .position(|port| port.name() == cable.destination().port_name())
            .ok_or_else(|| CompileError::MissingPort {
                module_id: destination_module_id.to_string(),
                port_name: cable.destination().port_name().to_string(),
            })?;

        nodes[destination_module_index].input_port_map[destination_port_index].push(
            CompiledPortRef {
                module_index: source_module_index,
                port_index: source_port_index,
            },
        );
        let source_span = nodes[source_module_index].output_port_spans[source_port_index];
        let destination_span =
            nodes[destination_module_index].input_port_spans[destination_port_index];
        debug_assert_eq!(source_span.channel_count, destination_span.channel_count);
        let route_count = if nodes[source_module_index].output_port_types[source_port_index]
            == SignalType::Event
        {
            1
        } else {
            source_span.channel_count
        };
        for channel in 0..route_count {
            nodes[destination_module_index].input_routes[destination_port_index].push(
                CompiledInputSource {
                    module_index: source_module_index,
                    port_index: source_port_index,
                    output_buffer_id: source_span.first_buffer + channel,
                    output_port_name: nodes[source_module_index].output_port_names
                        [source_port_index]
                        .clone(),
                },
            );
        }
    }

    Ok(())
}

fn module_index(
    module_indices: &BTreeMap<&str, usize>,
    module_id: &str,
    port_name: &str,
) -> Result<usize, CompileError> {
    module_indices
        .get(module_id)
        .copied()
        .ok_or_else(|| CompileError::MissingPort {
            module_id: module_id.to_string(),
            port_name: port_name.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Cable, ModuleNode, PortRef, SignalType};
    use crate::sample::{LoadedSample, PreparedSamplerAssets};

    fn render_settings() -> RenderSettings {
        RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 128,
            duration_frames: 1_024,
        }
    }

    fn audio_source(id: &str) -> ModuleNode {
        ModuleNode::new(ModuleId::new(id), "oscillator").with_output("audio", SignalType::Audio)
    }

    fn audio_processor(id: &str) -> ModuleNode {
        ModuleNode::new(ModuleId::new(id), "gain")
            .with_input("audio_in", SignalType::Audio)
            .with_output("audio_out", SignalType::Audio)
    }

    fn audio_sink(id: &str) -> ModuleNode {
        ModuleNode::new(ModuleId::new(id), "audio_output").with_input("left", SignalType::Audio)
    }

    fn connect(from_id: &str, from_port: &str, to_id: &str, to_port: &str) -> Cable {
        Cable::new(
            PortRef::new(ModuleId::new(from_id), from_port),
            PortRef::new(ModuleId::new(to_id), to_port),
        )
    }

    fn compile_graph(graph: &Graph) -> CompiledPatch {
        compile(graph, &render_settings()).expect("graph should compile")
    }

    #[test]
    fn nodes_are_compiled_in_dependency_order_for_linear_chain() {
        let graph = Graph::new(
            vec![audio_source("a"), audio_processor("b"), audio_sink("c")],
            vec![
                connect("a", "audio", "b", "audio_in"),
                connect("b", "audio_out", "c", "left"),
            ],
        );

        let compiled = compile_graph(&graph);

        assert_eq!(compiled.execution_order(), &[0, 1, 2]);
        assert_eq!(compiled.topological_order(), &[0, 1, 2]);
    }

    #[test]
    fn numeric_parameters_are_compiled_into_slots() {
        let graph = Graph::new(
            vec![
                audio_processor("gain")
                    .with_params(BTreeMap::from([("gain".to_string(), "0.5".to_string())])),
            ],
            vec![],
        );

        let mut compiled = compile_graph(&graph);

        assert_eq!(compiled.numeric_parameter_value("gain", "gain"), Some(0.5));
        assert!(compiled.set_numeric_parameter_by_target("gain", "gain", 0.25));
        assert_eq!(compiled.numeric_parameter_value("gain", "gain"), Some(0.25));
    }

    #[test]
    fn parameter_slot_index_resolves_once_and_can_be_reused_for_o1_updates() {
        let graph = Graph::new(
            vec![
                audio_processor("gain")
                    .with_params(BTreeMap::from([("gain".to_string(), "0.5".to_string())])),
            ],
            vec![],
        );

        let mut compiled = compile_graph(&graph);

        let slot_index = compiled
            .parameter_slot_index("gain", "gain")
            .expect("gain parameter should resolve to a slot");

        assert!(compiled.set_parameter_slot(slot_index, 0.75));
        assert_eq!(compiled.numeric_parameter_value("gain", "gain"), Some(0.75));
    }

    #[test]
    fn parameter_slot_index_returns_none_for_unknown_target() {
        let graph = Graph::new(
            vec![
                audio_processor("gain")
                    .with_params(BTreeMap::from([("gain".to_string(), "0.5".to_string())])),
            ],
            vec![],
        );

        let compiled = compile_graph(&graph);

        assert_eq!(compiled.parameter_slot_index("gain", "missing"), None);
        assert_eq!(compiled.parameter_slot_index("missing", "gain"), None);
    }

    #[test]
    fn legacy_assets_attach_as_kind_specific_compiled_handles() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("sample"), "sampler"),
                ModuleNode::new(ModuleId::new("ir"), "convolution"),
            ],
            vec![],
        );
        let assets = PreparedSamplerAssets::from_samples_by_module(BTreeMap::from([
            ("sample".to_string(), LoadedSample::new(48_000, vec![0.25])),
            ("ir".to_string(), LoadedSample::new(48_000, vec![1.0])),
        ]));
        let mut compiled = compile_graph(&graph);

        compiled.attach_legacy_resources(&assets);

        assert!(compiled.nodes()[0].resources.sample.is_some());
        assert!(compiled.nodes()[0].resources.impulse_response.is_none());
        assert!(compiled.nodes()[1].resources.sample.is_none());
        assert!(compiled.nodes()[1].resources.impulse_response.is_some());
    }

    #[test]
    fn legacy_numeric_static_argument_is_typed_without_a_runtime_slot() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("delay"), "compensation_delay")
                    .with_input(builtin_ports::AUDIO_IN, SignalType::Audio)
                    .with_output(builtin_ports::AUDIO_OUT, SignalType::Audio)
                    .with_params(BTreeMap::from([(
                        DELAY_SAMPLES_PARAMETER.to_string(),
                        "4".to_string(),
                    )])),
            ],
            vec![],
        );

        let compiled = compile_graph(&graph);

        assert_eq!(
            compiled.nodes()[0].construction,
            CompiledConstruction::CompensationDelay { samples: 4 }
        );
        assert_eq!(
            compiled.parameter_slot_index("delay", DELAY_SAMPLES_PARAMETER),
            None
        );
    }

    #[test]
    fn output_buffer_coloring_reuses_expired_spans_deterministically() {
        let graph = Graph::new(
            vec![
                audio_source("source"),
                audio_processor("middle"),
                audio_processor("last"),
                audio_sink("out"),
            ],
            vec![
                connect("source", "audio", "middle", "audio_in"),
                connect("middle", "audio_out", "last", "audio_in"),
                connect("last", "audio_out", "out", "left"),
            ],
        );

        let first = compile_graph(&graph);
        let second = compile_graph(&graph);

        assert_eq!(first.total_output_buffer_count(), 2);
        assert_eq!(first.nodes()[0].output_port_spans[0].first_buffer, 0);
        assert_eq!(first.nodes()[1].output_port_spans[0].first_buffer, 1);
        assert_eq!(first.nodes()[2].output_port_spans[0].first_buffer, 0);
        assert_eq!(
            first
                .nodes()
                .iter()
                .map(|node| node.output_port_spans.clone())
                .collect::<Vec<_>>(),
            second
                .nodes()
                .iter()
                .map(|node| node.output_port_spans.clone())
                .collect::<Vec<_>>()
        );
    }
}
