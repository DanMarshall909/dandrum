use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

use crate::builtins::module_kind::ModuleKind;
use crate::compiled_patch::{
    self, CompileError, CompiledNodeData, CompiledPatch, CompiledPortSpan, CompiledRootPort,
    RootBusPlan,
};
use crate::diagnostics::{self, Diagnostic, Severity};
use crate::graph::{
    Cable, ExecutionScope, Graph, ModuleId, ModuleNode, PortDirection, PortRef, SignalType,
};
use crate::kernel::document::KernelPatch;
use crate::kernel::flatten::FlattenedGraph;
use crate::kernel::latency::LatencyPlan;
use crate::kernel::{DefinitionRegistry, GraphDefinition, StaticValue};
use crate::patch::{self, ParameterValue, PatchDocument, PresetDocument, RenderSettings};
use crate::sample::{self, PreparedSamplerAssets, SampleLoadError};

const KERNEL_COMPENSATION_EDGE_PREFIX: &str = "compensation::edge::";
const KERNEL_COMPENSATION_ROOT_PREFIX: &str = "compensation::root::";
const KERNEL_OUTPUT_NODE_ID: &str = "kernel::audio_output";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HostBuses {
    inputs: BTreeMap<String, usize>,
    outputs: BTreeMap<String, usize>,
}

impl HostBuses {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_input(mut self, name: impl Into<String>, channel_count: usize) -> Self {
        self.inputs.insert(name.into(), channel_count);
        self
    }

    pub fn with_output(mut self, name: impl Into<String>, channel_count: usize) -> Self {
        self.outputs.insert(name.into(), channel_count);
        self
    }
}

#[derive(Debug)]
pub(crate) enum PreparationError {
    Load(patch::PatchLoadError),
    Schema(patch::PatchValidationError),
    Graph(crate::graph::GraphValidationError),
    Assets(SampleLoadError),
    Compile(CompileError),
}

#[derive(Clone, Debug, PartialEq)]
pub struct KernelPreparationError {
    diagnostics: diagnostics::Diagnostics,
}

impl KernelPreparationError {
    pub fn diagnostics(&self) -> &diagnostics::Diagnostics {
        &self.diagnostics
    }

    pub fn to_diagnostics(&self) -> diagnostics::Diagnostics {
        self.diagnostics.clone()
    }
}

impl From<diagnostics::Diagnostics> for KernelPreparationError {
    fn from(diagnostics: diagnostics::Diagnostics) -> Self {
        Self { diagnostics }
    }
}

impl fmt::Display for KernelPreparationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "kernel preparation failed: {}", self.diagnostics)
    }
}

impl std::error::Error for KernelPreparationError {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PreparationDiagnostics {
    messages: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PreparedInstrument {
    patch_doc: PatchDocument,
    resolved_parameters: BTreeMap<String, BTreeMap<String, ParameterValue>>,
    graph: Graph,
    compiled_patch: CompiledPatch,
    sampler_assets: PreparedSamplerAssets,
    diagnostics: PreparationDiagnostics,
}

/// Prepared result for the unified kernel front end. It retains the flattened
/// graph and latency plan for inspection and executes through channel-aware
/// compiled spans while legacy callers continue to use the adapter.
#[derive(Clone, Debug, PartialEq)]
pub struct PreparedKernelInstrument {
    flattened_graph: FlattenedGraph,
    latency_plan: LatencyPlan,
    graph: Graph,
    compiled_patch: CompiledPatch,
}

impl PreparedKernelInstrument {
    pub fn flattened_graph(&self) -> &FlattenedGraph {
        &self.flattened_graph
    }

    pub fn latency_plan(&self) -> &LatencyPlan {
        &self.latency_plan
    }

    pub fn total_latency_samples(&self) -> u32 {
        self.latency_plan.root_latency()
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn compiled_patch(&self) -> &CompiledPatch {
        &self.compiled_patch
    }
}

impl PreparedInstrument {
    pub(crate) fn new(
        patch_doc: PatchDocument,
        resolved_parameters: BTreeMap<String, BTreeMap<String, ParameterValue>>,
        graph: Graph,
        compiled_patch: CompiledPatch,
        sampler_assets: PreparedSamplerAssets,
        diagnostics: PreparationDiagnostics,
    ) -> Self {
        Self {
            patch_doc,
            resolved_parameters,
            graph,
            compiled_patch,
            sampler_assets,
            diagnostics,
        }
    }

    pub(crate) fn patch_doc(&self) -> &PatchDocument {
        &self.patch_doc
    }

    #[allow(dead_code)]
    pub(crate) fn resolved_parameters(
        &self,
    ) -> &BTreeMap<String, BTreeMap<String, ParameterValue>> {
        &self.resolved_parameters
    }

    pub(crate) fn graph(&self) -> &Graph {
        &self.graph
    }

    pub(crate) fn compiled_patch(&self) -> &CompiledPatch {
        &self.compiled_patch
    }

    pub(crate) fn sampler_assets(&self) -> &PreparedSamplerAssets {
        &self.sampler_assets
    }

    #[allow(dead_code)]
    pub(crate) fn diagnostics(&self) -> &PreparationDiagnostics {
        &self.diagnostics
    }
}

impl PreparationDiagnostics {
    #[allow(dead_code)]
    pub(crate) fn messages(&self) -> &[String] {
        &self.messages
    }
}

pub(crate) fn prepare_instrument_file(
    path: impl AsRef<Path>,
) -> Result<PreparedInstrument, PreparationError> {
    let path = path.as_ref();
    let patch_doc = load_patch_document(path)?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));

    prepare_instrument_document(patch_doc, base_dir)
}

/// Prepare an already-loaded kernel patch with render settings supplied by the
/// host. Legacy file preparation remains separate until examples, presets,
/// assets, and FFI callers migrate to the kernel document shape.
pub fn prepare_kernel_patch(
    patch: &KernelPatch,
    render_settings: &RenderSettings,
) -> Result<PreparedKernelInstrument, KernelPreparationError> {
    prepare_kernel_graph(patch.root(), patch.registry(), render_settings)
}

/// Validate, flatten, latency-balance, lower, and compile a kernel root using
/// the supplied definition registry.
pub fn prepare_kernel_graph(
    root: &GraphDefinition,
    registry: &DefinitionRegistry,
    render_settings: &RenderSettings,
) -> Result<PreparedKernelInstrument, KernelPreparationError> {
    let default_buses = HostBuses {
        inputs: BTreeMap::new(),
        outputs: root
            .ports()
            .iter()
            .filter(|port| port.direction() == PortDirection::Output)
            .filter_map(|port| match port.channels() {
                crate::kernel::ChannelCount::Literal(channels) => {
                    Some((port.name().to_string(), *channels as usize))
                }
                crate::kernel::ChannelCount::Param(_) => None,
            })
            .collect(),
    };
    prepare_kernel_graph_with_buses(root, registry, render_settings, &default_buses)
}

pub fn prepare_kernel_graph_with_buses(
    root: &GraphDefinition,
    registry: &DefinitionRegistry,
    render_settings: &RenderSettings,
    host_buses: &HostBuses,
) -> Result<PreparedKernelInstrument, KernelPreparationError> {
    let validation = root.validate(registry);
    if !validation.is_ok() {
        return Err(validation.diagnostics().clone().into());
    }

    let flattened_graph = root
        .flatten(registry)
        .map_err(KernelPreparationError::from)?;
    validate_host_buses(&flattened_graph, host_buses)?;
    let latency_plan = flattened_graph
        .balance_latency()
        .map_err(KernelPreparationError::from)?;
    let lowered = lower_kernel_graph(&flattened_graph, &latency_plan)?;
    lowered
        .graph
        .validate()
        .map_err(|error| KernelPreparationError::from(error.to_diagnostics()))?;
    let mut compiled_patch =
        compiled_patch::compile_with_node_data(&lowered.graph, render_settings, &lowered.node_data)
            .map_err(|error| {
                KernelPreparationError::from(diagnostics::Diagnostics::from(error.to_diagnostic()))
            })?;
    let root_input_spans = flattened_graph
        .root_ports()
        .iter()
        .filter(|port| port.direction() == PortDirection::Input)
        .map(|port| {
            let span = compiled_patch.reserve_root_input_span(
                port.channels() as usize,
                flattened_graph
                    .root_input_destinations()
                    .get(port.name())
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
            );
            (port.name().to_string(), span)
        })
        .collect::<BTreeMap<_, _>>();
    compiled_patch.set_root_bus_plan(root_bus_plan(
        &flattened_graph,
        host_buses,
        &root_input_spans,
        &lowered.root_outputs,
        &compiled_patch,
    ));

    Ok(PreparedKernelInstrument {
        flattened_graph,
        latency_plan,
        graph: lowered.graph,
        compiled_patch,
    })
}

fn validate_host_buses(
    flattened: &FlattenedGraph,
    host_buses: &HostBuses,
) -> Result<(), KernelPreparationError> {
    let mut diagnostics = diagnostics::Diagnostics::new();
    for port in flattened.root_ports() {
        let host_channels = match port.direction() {
            PortDirection::Input => host_buses.inputs.get(port.name()),
            PortDirection::Output => host_buses.outputs.get(port.name()),
        };
        if port.direction() == PortDirection::Output && host_channels.is_none() {
            diagnostics.push(
                Diagnostic::new(
                    diagnostics::error_codes::KERNEL_HOST_BUS_MISSING_OUTPUT,
                    Severity::Error,
                    format!(
                        "root output '{}' has no matching host output bus",
                        port.name()
                    ),
                )
                .with_port_name(port.name())
                .with_suggested_fix(
                    "declare a same-named host output bus with the root port's channel count",
                ),
            );
        } else if let Some(host_channels) = host_channels {
            if *host_channels != port.channels() as usize {
                diagnostics.push(
                    Diagnostic::new(
                        diagnostics::error_codes::KERNEL_HOST_BUS_CHANNEL_MISMATCH,
                        Severity::Error,
                        format!(
                            "root port '{}' has {} channels but its host bus has {host_channels}",
                            port.name(),
                            port.channels()
                        ),
                    )
                    .with_port_name(port.name())
                    .with_expected(format!("{} channels", port.channels()))
                    .with_actual(format!("{host_channels} channels")),
                );
            }
        }
    }
    if diagnostics.has_errors() {
        Err(KernelPreparationError::from(diagnostics))
    } else {
        Ok(())
    }
}

struct LoweredKernelGraph {
    graph: Graph,
    node_data: BTreeMap<String, CompiledNodeData>,
    root_outputs: BTreeMap<String, crate::kernel::PortRef>,
}

fn lower_kernel_graph(
    flattened: &FlattenedGraph,
    latency_plan: &LatencyPlan,
) -> Result<LoweredKernelGraph, KernelPreparationError> {
    use crate::builtins::module_types;
    use crate::graph::builtin_ports;

    let mut ids = flattened
        .nodes()
        .iter()
        .map(|node| node.id().as_str().to_string())
        .collect::<BTreeSet<_>>();
    let mut modules = Vec::new();
    let mut node_data = BTreeMap::new();
    for node in flattened.nodes() {
        let mut lowered = ModuleNode::new(ModuleId::new(node.id().as_str()), node.definition())
            .with_execution_scope(ExecutionScope::Global);
        for port in node.ports() {
            lowered = match port.direction() {
                PortDirection::Input => {
                    if port.multiplicity() == super::kernel::Multiplicity::Summing {
                        lowered.with_mixing_input(port.name(), port.signal_type())
                    } else {
                        lowered.with_input(port.name(), port.signal_type())
                    }
                }
                PortDirection::Output => lowered.with_output(port.name(), port.signal_type()),
            };
        }
        let kind = ModuleKind::from_str(node.definition()).ok_or_else(|| {
            KernelPreparationError::from(diagnostics::Diagnostics::from(
                CompileError::UnknownModuleType {
                    module_type: node.definition().to_string(),
                }
                .to_diagnostic(),
            ))
        })?;
        let mut data = CompiledNodeData::from_kernel(
            node.id().as_str(),
            kind,
            node.static_args(),
            node.port_defaults(),
        )
        .map_err(|error| {
            KernelPreparationError::from(diagnostics::Diagnostics::from(error.to_diagnostic()))
        })?;
        data.port_channels.extend(
            node.ports()
                .iter()
                .map(|port| (port.name().to_string(), port.channels() as usize)),
        );
        for port in node.ports().iter().filter(|port| {
            port.direction() == PortDirection::Input && port.signal_type() == SignalType::Control
        }) {
            data.control_defaults
                .entry(port.name().to_string())
                .or_insert_with(|| {
                    compiled_patch::effective_legacy_control_default(kind, port.name())
                        .unwrap_or(0.0)
                });
        }
        node_data.insert(node.id().as_str().to_string(), data);
        let mut legacy_parameters = node
            .static_args()
            .iter()
            .map(|(name, value)| (name.clone(), static_value_to_string(value)))
            .collect::<BTreeMap<_, _>>();
        legacy_parameters.extend(
            node.port_defaults()
                .iter()
                .map(|(name, value)| (name.clone(), value.to_string())),
        );
        modules.push(lowered.with_params(legacy_parameters));
    }

    let mut cables = Vec::new();
    for connection in flattened.connections() {
        if let Some((index, compensation)) = latency_plan
            .compensations()
            .iter()
            .enumerate()
            .find(|(_, compensation)| compensation.connection() == connection)
        {
            let id = format!("{KERNEL_COMPENSATION_EDGE_PREFIX}{index}");
            reserve_generated_id(&mut ids, &id)?;
            modules.push(compensation_delay_node(&id, compensation.samples()));
            let mut data = CompiledNodeData::compensation_delay(compensation.samples());
            data.port_channels.insert(
                builtin_ports::AUDIO_IN.to_string(),
                compensation.channels() as usize,
            );
            data.port_channels.insert(
                builtin_ports::AUDIO_OUT.to_string(),
                compensation.channels() as usize,
            );
            node_data.insert(id.clone(), data);
            cables.push(Cable::new(
                legacy_ref(connection.source()),
                PortRef::new(ModuleId::new(&id), builtin_ports::AUDIO_IN),
            ));
            cables.push(Cable::new(
                PortRef::new(ModuleId::new(&id), builtin_ports::AUDIO_OUT),
                legacy_ref(connection.destination()),
            ));
        } else {
            cables.push(Cable::new(
                legacy_ref(connection.source()),
                legacy_ref(connection.destination()),
            ));
        }
    }

    let mut root_outputs = BTreeMap::new();
    for (root_name, sources) in flattened.root_output_sources() {
        let Some(source) = sources.first() else {
            continue;
        };
        let source = if let Some((index, compensation)) = latency_plan
            .root_compensations()
            .iter()
            .enumerate()
            .find(|(_, compensation)| {
                compensation.root_port() == root_name && compensation.source() == source
            }) {
            let id = format!("{KERNEL_COMPENSATION_ROOT_PREFIX}{root_name}::{index}");
            reserve_generated_id(&mut ids, &id)?;
            modules.push(compensation_delay_node(&id, compensation.samples()));
            let mut data = CompiledNodeData::compensation_delay(compensation.samples());
            data.port_channels.insert(
                builtin_ports::AUDIO_IN.to_string(),
                compensation.channels() as usize,
            );
            data.port_channels.insert(
                builtin_ports::AUDIO_OUT.to_string(),
                compensation.channels() as usize,
            );
            node_data.insert(id.clone(), data);
            cables.push(Cable::new(
                legacy_ref(source),
                PortRef::new(ModuleId::new(&id), builtin_ports::AUDIO_IN),
            ));
            PortRef::new(ModuleId::new(id), builtin_ports::AUDIO_OUT)
        } else {
            legacy_ref(source)
        };
        root_outputs.insert(root_name.clone(), kernel_ref_from_legacy(&source));
    }

    let legacy_stereo = root_outputs.len() == 2
        && [builtin_ports::LEFT, builtin_ports::RIGHT]
            .iter()
            .all(|name| {
                flattened
                    .root_ports()
                    .iter()
                    .any(|port| port.name() == *name && port.channels() == 1)
            });
    if legacy_stereo {
        reserve_generated_id(&mut ids, KERNEL_OUTPUT_NODE_ID)?;
        modules.push(
            ModuleNode::new(
                ModuleId::new(KERNEL_OUTPUT_NODE_ID),
                module_types::AUDIO_OUTPUT,
            )
            .with_execution_scope(ExecutionScope::Global)
            .with_input(builtin_ports::LEFT, SignalType::Audio)
            .with_input(builtin_ports::RIGHT, SignalType::Audio),
        );
        for root_name in [builtin_ports::LEFT, builtin_ports::RIGHT] {
            cables.push(Cable::new(
                legacy_ref(&root_outputs[root_name]),
                PortRef::new(ModuleId::new(KERNEL_OUTPUT_NODE_ID), root_name),
            ));
        }
        node_data.insert(KERNEL_OUTPUT_NODE_ID.to_string(), CompiledNodeData::none());
    }
    Ok(LoweredKernelGraph {
        graph: Graph::new(modules, cables),
        node_data,
        root_outputs,
    })
}

fn kernel_ref_from_legacy(reference: &PortRef) -> crate::kernel::PortRef {
    crate::kernel::PortRef::new(
        crate::kernel::NodeId::new(reference.module_id().as_str()),
        reference.port_name(),
    )
}

fn root_bus_plan(
    flattened: &FlattenedGraph,
    host_buses: &HostBuses,
    root_input_spans: &BTreeMap<String, CompiledPortSpan>,
    root_outputs: &BTreeMap<String, crate::kernel::PortRef>,
    compiled: &CompiledPatch,
) -> RootBusPlan {
    let ports = flattened.root_ports();
    let inputs = ports
        .iter()
        .filter(|port| port.direction() == PortDirection::Input)
        .map(|port| {
            CompiledRootPort::new(
                port.name(),
                port.channels() as usize,
                root_input_spans.get(port.name()).copied(),
                host_buses.inputs.contains_key(port.name()),
            )
        })
        .collect();
    let outputs = ports
        .iter()
        .filter(|port| port.direction() == PortDirection::Output)
        .map(|port| {
            let span = root_outputs.get(port.name()).and_then(|source| {
                compiled
                    .nodes()
                    .iter()
                    .find(|node| node.id.as_str() == source.node().as_str())
                    .and_then(|node| {
                        node.output_port_names
                            .iter()
                            .position(|name| name == source.port())
                            .map(|index| node.output_port_spans[index])
                    })
            });
            CompiledRootPort::new(port.name(), port.channels() as usize, span, true)
        })
        .collect();
    RootBusPlan::new(inputs, outputs)
}

fn compensation_delay_node(id: &str, samples: u32) -> ModuleNode {
    ModuleNode::new(
        ModuleId::new(id),
        crate::builtins::module_types::COMPENSATION_DELAY,
    )
    .with_execution_scope(ExecutionScope::Global)
    .with_input(crate::graph::builtin_ports::AUDIO_IN, SignalType::Audio)
    .with_output(crate::graph::builtin_ports::AUDIO_OUT, SignalType::Audio)
    .with_params(BTreeMap::from([(
        crate::builtins::DELAY_SAMPLES_PARAMETER.to_string(),
        samples.to_string(),
    )]))
}

fn reserve_generated_id(
    ids: &mut BTreeSet<String>,
    id: &str,
) -> Result<(), KernelPreparationError> {
    if ids.insert(id.to_string()) {
        return Ok(());
    }
    Err(KernelPreparationError::from(diagnostics::Diagnostics::from(
        Diagnostic::new(
            diagnostics::error_codes::KERNEL_PREPARATION_GENERATED_ID_COLLISION,
            Severity::Error,
            format!("compiler-generated node id '{id}' collides with an existing node"),
        )
        .with_module_id(id)
        .with_suggested_fix("rename the authored node so compiler-generated compensation nodes remain unambiguous"),
    )))
}

fn legacy_ref(reference: &crate::kernel::PortRef) -> PortRef {
    PortRef::new(ModuleId::new(reference.node().as_str()), reference.port())
}

fn static_value_to_string(value: &StaticValue) -> String {
    match value {
        StaticValue::Int(value) => value.to_string(),
        StaticValue::Enum(value) | StaticValue::String(value) | StaticValue::Resource(value) => {
            value.clone()
        }
    }
}

pub(crate) fn prepare_instrument_document(
    patch_doc: PatchDocument,
    base_dir: impl AsRef<Path>,
) -> Result<PreparedInstrument, PreparationError> {
    validate_patch_document(&patch_doc)?;
    let resolved_parameters = resolve_patch_parameters(&patch_doc)?;
    let graph = build_validated_graph_with_resolved_parameters(&patch_doc, &resolved_parameters)?;
    let sampler_assets = prepare_assets(&patch_doc, base_dir)?;
    let mut compiled_patch = compile_patch(&graph, &patch_doc)?;
    compiled_patch.attach_legacy_resources(&sampler_assets);

    Ok(PreparedInstrument::new(
        patch_doc,
        resolved_parameters,
        graph,
        compiled_patch,
        sampler_assets,
        PreparationDiagnostics::default(),
    ))
}

#[allow(dead_code)]
pub(crate) fn prepare_instrument_document_with_preset(
    patch_doc: PatchDocument,
    preset_doc: &PresetDocument,
    base_dir: impl AsRef<Path>,
) -> Result<PreparedInstrument, PreparationError> {
    let patched_doc =
        patch::apply_preset(&patch_doc, preset_doc).map_err(PreparationError::Schema)?;
    prepare_instrument_document(patched_doc, base_dir)
}

pub(crate) fn load_patch_document(
    path: impl AsRef<Path>,
) -> Result<PatchDocument, PreparationError> {
    patch::load_patch_file(path).map_err(PreparationError::Load)
}

pub(crate) fn validate_patch_document(patch_doc: &PatchDocument) -> Result<(), PreparationError> {
    patch::validate_patch_schema(patch_doc).map_err(PreparationError::Schema)
}

pub(crate) fn resolve_patch_parameters(
    patch_doc: &PatchDocument,
) -> Result<BTreeMap<String, BTreeMap<String, ParameterValue>>, PreparationError> {
    patch::resolve_module_parameters(patch_doc).map_err(PreparationError::Schema)
}

#[allow(dead_code)]
pub(crate) fn build_validated_graph(patch_doc: &PatchDocument) -> Result<Graph, PreparationError> {
    let resolved_parameters = resolve_patch_parameters(patch_doc)?;
    build_validated_graph_with_resolved_parameters(patch_doc, &resolved_parameters)
}

fn build_validated_graph_with_resolved_parameters(
    patch_doc: &PatchDocument,
    resolved_parameters: &BTreeMap<String, BTreeMap<String, ParameterValue>>,
) -> Result<Graph, PreparationError> {
    let resolved_patch = patch_document_with_resolved_parameters(patch_doc, resolved_parameters);
    let graph = Graph::from_patch_declarations(&resolved_patch);
    graph.validate().map_err(PreparationError::Graph)?;
    Ok(graph)
}

fn patch_document_with_resolved_parameters(
    patch_doc: &PatchDocument,
    resolved_parameters: &BTreeMap<String, BTreeMap<String, ParameterValue>>,
) -> PatchDocument {
    let mut resolved_patch = patch_doc.clone();

    for module in &mut resolved_patch.modules {
        if let Some(parameters) = resolved_parameters.get(&module.id) {
            module.parameters = parameters.clone();
        }
    }

    resolved_patch.parameters.clear();
    resolved_patch
}

pub(crate) fn prepare_assets(
    patch_doc: &PatchDocument,
    base_dir: impl AsRef<Path>,
) -> Result<PreparedSamplerAssets, PreparationError> {
    sample::prepare_sampler_assets(patch_doc, base_dir).map_err(PreparationError::Assets)
}

pub(crate) fn compile_patch(
    graph: &Graph,
    patch_doc: &PatchDocument,
) -> Result<CompiledPatch, PreparationError> {
    compiled_patch::compile(graph, &patch_doc.render).map_err(PreparationError::Compile)
}

impl PreparationError {
    #[allow(dead_code)]
    pub fn to_diagnostics(&self) -> diagnostics::Diagnostics {
        match self {
            Self::Load(error) => error.to_diagnostic().into(),
            Self::Schema(error) => error.to_diagnostics(),
            Self::Graph(error) => error.to_diagnostics(),
            Self::Assets(error) => Diagnostic::new(
                diagnostics::error_codes::LOADING,
                Severity::Error,
                error.to_string(),
            )
            .into(),
            Self::Compile(error) => error.to_diagnostic().into(),
        }
    }
}

impl fmt::Display for PreparationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Load(error) => write!(formatter, "patch load failed: {error}"),
            Self::Schema(error) => write!(formatter, "patch schema validation failed: {error}"),
            Self::Graph(error) => write!(formatter, "graph validation failed: {error}"),
            Self::Assets(error) => write!(formatter, "asset preparation failed: {error}"),
            Self::Compile(error) => write!(formatter, "patch compilation failed: {error}"),
        }
    }
}

impl std::error::Error for PreparationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Load(error) => Some(error),
            Self::Schema(error) => Some(error),
            Self::Graph(error) => Some(error),
            Self::Assets(error) => Some(error),
            Self::Compile(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::{
        DELAY_SAMPLES_PARAMETER, SPECTRAL_FFT_SIZE_PARAMETER, SPECTRAL_MODE_PARAMETER,
        SPECTRAL_MODE_PASSTHROUGH, module_types,
    };
    use crate::convolution::Convolution;
    use crate::core::TimedInputEvent;
    use crate::graph::{SignalType, builtin_ports};
    use crate::graph_processor::{RealtimeGraphProcessor, render_offline_compiled};
    use crate::kernel::builtins::builtin_registry;
    use crate::kernel::document::load_kernel_patch_str;
    use crate::kernel::{
        Connection, DefinitionRegistry, GraphDefinition, Node, NodeId, Port as KernelPort,
        PortRef as KernelPortRef, StaticArg, StaticValue,
    };
    use crate::patch;
    use crate::sample::LoadedSample;
    use crate::script::ScriptEvent;
    use std::collections::BTreeMap;
    use std::fs;

    const WET_NODE_ID: &str = "wet";
    const IMPULSE_TOLERANCE: f32 = 1.0e-5;

    const MINIMAL_PATCH: &str = r#"
metadata:
  name: Prepared Instrument
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 128
modules:
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
      - name: right
        signal_type: audio
"#;

    const PRESETTABLE_FILTER_PATCH: &str = r#"
metadata:
  name: Presettable Filter
instrument:
  id: dandrum.filter
  preset_schema_version: 1
preset_surface:
  parameters:
    - name: tone.algorithm
      type: text
      default: moog
      maps_to: filt.algorithm
    - name: tone.mode
      type: text
      default: lowpass
      maps_to: filt.mode
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 128
modules:
  - id: filt
    type: filter
"#;

    const BRIGHT_FILTER_PRESET: &str = r#"
name: Bright Filter
instrument:
  id: dandrum.filter
  preset_schema_version: 1
values:
  tone.algorithm: biquad
"#;

    #[test]
    fn prepared_instrument_owns_validated_patch_graph_compiled_patch_assets_and_diagnostics() {
        let patch_doc = patch::load_patch_str(MINIMAL_PATCH).expect("patch should parse");
        patch::validate_patch_schema(&patch_doc).expect("patch schema should validate");
        let resolved_parameters =
            patch::resolve_module_parameters(&patch_doc).expect("parameters should resolve");
        let graph =
            build_validated_graph_with_resolved_parameters(&patch_doc, &resolved_parameters)
                .expect("graph should validate");
        let compiled_patch =
            compiled_patch::compile(&graph, &patch_doc.render).expect("graph should compile");

        let prepared = PreparedInstrument::new(
            patch_doc,
            resolved_parameters,
            graph,
            compiled_patch,
            PreparedSamplerAssets::empty(),
            PreparationDiagnostics {
                messages: vec!["prepared".to_string()],
            },
        );

        assert_eq!(prepared.patch_doc().metadata.name, "Prepared Instrument");
        assert_eq!(prepared.resolved_parameters().len(), 1);
        assert_eq!(prepared.graph().modules().len(), 1);
        assert_eq!(prepared.compiled_patch().nodes().len(), 1);
        assert_eq!(
            prepared.compiled_patch().render_settings().sample_rate_hz,
            48_000
        );
        assert_eq!(prepared.sampler_assets(), &PreparedSamplerAssets::empty());
        assert_eq!(prepared.diagnostics().messages(), &["prepared".to_string()]);
    }

    #[test]
    fn prepare_instrument_file_runs_explicit_pipeline_and_returns_prepared_instrument() {
        let temp_dir =
            std::env::temp_dir().join(format!("dandrum-preparation-test-{}", std::process::id()));
        fs::create_dir_all(&temp_dir).expect("temp directory should be created");
        let patch_path = temp_dir.join("patch.yaml");
        fs::write(&patch_path, MINIMAL_PATCH).expect("patch file should be written");

        let prepared = prepare_instrument_file(&patch_path).expect("patch should prepare");

        assert_eq!(prepared.patch_doc().metadata.name, "Prepared Instrument");
        assert_eq!(prepared.graph().modules().len(), 1);
        assert_eq!(prepared.compiled_patch().nodes().len(), 1);
        assert_eq!(prepared.resolved_parameters().len(), 1);
    }

    #[test]
    fn preparation_pipeline_resolves_declared_parameter_defaults_before_graph_preparation() {
        let patch_doc = patch::load_patch_str(
            r#"
metadata:
  name: Prepared Defaults
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 128
modules:
  - id: filt
    type: filter
"#,
        )
        .expect("patch should parse");

        validate_patch_document(&patch_doc).expect("schema should validate");
        let resolved = resolve_patch_parameters(&patch_doc).expect("parameters should resolve");
        let graph = build_validated_graph(&patch_doc).expect("graph should still build");

        assert_eq!(
            resolved
                .get("filt")
                .and_then(|params| params.get("algorithm")),
            Some(&ParameterValue::Text("moog".to_string()))
        );
        assert_eq!(
            graph
                .modules()
                .iter()
                .find(|module| module.id().as_str() == "filt")
                .and_then(|module| module.params().get("algorithm")),
            Some(&"moog".to_string())
        );
    }

    #[test]
    fn preparation_pipeline_passes_resolved_parameters_into_compiled_nodes() {
        let patch_doc = patch::load_patch_str(
            r#"
metadata:
  name: Prepared Compiled Params
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 128
parameters:
  filt:
    mode: highpass
modules:
  - id: filt
    type: filter
    parameters:
      algorithm: biquad
"#,
        )
        .expect("patch should parse");

        validate_patch_document(&patch_doc).expect("schema should validate");
        let resolved = resolve_patch_parameters(&patch_doc).expect("parameters should resolve");
        let graph = build_validated_graph_with_resolved_parameters(&patch_doc, &resolved)
            .expect("graph should validate");
        let compiled = compile_patch(&graph, &patch_doc).expect("graph should compile");
        let filt = compiled
            .nodes()
            .iter()
            .find(|node| node.id.as_str() == "filt")
            .expect("filter node should compile");

        assert_eq!(
            filt.parameters.get("algorithm"),
            Some(&"biquad".to_string())
        );
        assert_eq!(filt.parameters.get("mode"), Some(&"highpass".to_string()));
        assert_eq!(
            filt.parameters.get("comb_type"),
            Some(&"feedback".to_string())
        );
    }

    #[test]
    fn preparation_pipeline_reports_schema_errors_with_typed_error() {
        let patch_doc = patch::load_patch_str(
            r#"
metadata:
  name: Invalid Prepared Instrument
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 128
modules: []
"#,
        )
        .expect("patch should parse");

        let error = validate_patch_document(&patch_doc).expect_err("schema should fail");

        assert!(matches!(error, PreparationError::Schema(_)));
        assert!(
            error
                .to_string()
                .starts_with("patch schema validation failed: patch validation failed")
        );
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn preparation_pipeline_reports_graph_errors_with_typed_error() {
        let patch_doc = patch::load_patch_str(
            r#"
metadata:
  name: Invalid Graph
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 128
modules:
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
      - name: right
        signal_type: audio
connections:
  - from: missing.audio
    to: out.left
"#,
        )
        .expect("patch should parse");
        validate_patch_document(&patch_doc).expect("schema should validate");

        let error = build_validated_graph(&patch_doc).expect_err("graph should fail");

        assert!(matches!(error, PreparationError::Graph(_)));
        assert!(
            error
                .to_string()
                .starts_with("graph validation failed: graph validation failed")
        );
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn external_preset_values_override_surface_defaults_before_graph_construction() {
        let patch_doc = patch::load_patch_str(PRESETTABLE_FILTER_PATCH).expect("patch parses");
        let preset_doc = patch::load_preset_str(BRIGHT_FILTER_PRESET).expect("preset parses");

        let prepared = prepare_instrument_document_with_preset(patch_doc, &preset_doc, ".")
            .expect("patch plus preset should prepare");
        let filt = prepared
            .resolved_parameters()
            .get("filt")
            .expect("filter params should resolve");

        assert_eq!(
            filt.get("algorithm"),
            Some(&ParameterValue::Text("biquad".to_string()))
        );
        assert_eq!(
            filt.get("mode"),
            Some(&ParameterValue::Text("lowpass".to_string()))
        );
        assert_eq!(
            prepared
                .graph()
                .modules()
                .iter()
                .find(|module| module.id().as_str() == "filt")
                .and_then(|module| module.params().get("algorithm")),
            Some(&"biquad".to_string())
        );
    }

    #[test]
    fn external_preset_rendering_is_deterministic_for_same_patch_preset_and_inputs() {
        let patch_doc = patch::load_patch_str(
            r#"
metadata:
  name: Presettable Noise
instrument:
  id: dandrum.noise
  preset_schema_version: 1
preset_surface:
  parameters:
    - name: noise.seed
      type: number
      default: 1
      min: 0
      max: 4294967295
      maps_to: noise.seed
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 128
modules:
  - id: noise
    type: noise
  - id: mixer
    type: audio_mixer
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
      - name: right
        signal_type: audio
connections:
  - from: noise.audio
    to: mixer.inputs
  - from: mixer.mix
    to: out.left
  - from: mixer.mix
    to: out.right
"#,
        )
        .expect("patch parses");
        let preset_doc = patch::load_preset_str(
            r#"
name: A Noise
instrument:
  id: dandrum.noise
  preset_schema_version: 1
values:
  noise.seed: 330
"#,
        )
        .expect("preset parses");

        let first = prepare_instrument_document_with_preset(patch_doc.clone(), &preset_doc, ".")
            .expect("first render should prepare");
        let second = prepare_instrument_document_with_preset(patch_doc, &preset_doc, ".")
            .expect("second render should prepare");
        let (first_left, first_right) = crate::graph_processor::render_offline(
            first.graph(),
            &first.patch_doc().render,
            Vec::new(),
        );
        let (second_left, second_right) = crate::graph_processor::render_offline(
            second.graph(),
            &second.patch_doc().render,
            Vec::new(),
        );

        assert_eq!(first_left, second_left);
        assert_eq!(first_right, second_right);
    }

    #[test]
    fn external_preset_application_does_not_bypass_graph_validation() {
        let patch_doc = patch::load_patch_str(
            r#"
metadata:
  name: Presettable Invalid Routing
instrument:
  id: dandrum.invalid-routing
  preset_schema_version: 1
preset_surface:
  parameters:
    - name: tone.algorithm
      type: text
      default: moog
      maps_to: tone.algorithm
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 128
modules:
  - id: tone
    type: filter
connections:
  - from: tone.audio_out
    to: missing.left
"#,
        )
        .expect("patch parses");
        let preset_doc = patch::load_preset_str(
            r#"
name: Invalid Routing Tone
instrument:
  id: dandrum.invalid-routing
  preset_schema_version: 1
values:
  tone.algorithm: biquad
"#,
        )
        .expect("preset parses");

        let error = prepare_instrument_document_with_preset(patch_doc, &preset_doc, ".")
            .expect_err("graph validation should still run");

        assert!(matches!(error, PreparationError::Graph(_)));
    }

    const KERNEL_RENDER_SETTINGS: patch::RenderSettings = patch::RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 8,
        duration_frames: 16,
    };

    #[test]
    fn kernel_preparation_keeps_static_construction_and_control_defaults_distinct() {
        let patch = load_kernel_patch_str(
            r#"
metadata: { name: nested }
ports:
  - { name: left, direction: output, signal: audio, channels: 1, maps_from: layer.audio }
  - { name: right, direction: output, signal: audio, channels: 1, maps_from: layer.audio }
module_definitions:
  - type: voice
    ports:
      - { name: audio, direction: output, signal: audio, channels: 1, maps_from: amp.audio_out }
    modules:
      - { id: osc, type: oscillator, static: { waveform: sine }, defaults: { pitch: 2.0 } }
      - { id: amp, type: gain, defaults: { gain: 0.25 } }
    connections:
      - { from: osc.audio, to: amp.audio_in }
  - type: layer
    ports:
      - { name: audio, direction: output, signal: audio, channels: 1, maps_from: voice.audio }
    modules:
      - { id: voice, type: voice }
modules:
  - { id: layer, type: layer }
connections: []
"#,
        )
        .expect("kernel document should load");

        let prepared = prepare_kernel_patch(&patch, &KERNEL_RENDER_SETTINGS)
            .expect("kernel patch should prepare");

        let ids = prepared
            .flattened_graph()
            .nodes()
            .iter()
            .map(|node| node.id().as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, ["layer::voice::osc", "layer::voice::amp"]);
        let osc = prepared
            .compiled_patch()
            .nodes()
            .iter()
            .find(|node| node.id.as_str() == "layer::voice::osc")
            .expect("oscillator should compile");
        assert_eq!(osc.execution_scope, crate::graph::ExecutionScope::Global);
        assert_eq!(
            osc.construction,
            crate::compiled_patch::CompiledConstruction::Oscillator {
                waveform: crate::oscillator::Waveform::Sine,
            }
        );
        assert!(
            osc.parameters.is_empty(),
            "kernel construction/default data does not pass through the legacy parameter map"
        );
        assert_eq!(
            prepared
                .compiled_patch()
                .numeric_parameter_value("layer::voice::osc", "pitch"),
            Some(2.0)
        );
        assert_eq!(
            prepared
                .compiled_patch()
                .parameter_slot_index("layer::voice::osc", "waveform"),
            None,
            "a static argument is immutable and has no runtime slot"
        );
        prepared
            .compiled_patch()
            .nodes()
            .iter()
            .find(|node| node.id.as_str() == "layer::voice::amp")
            .expect("gain should compile");
        assert_eq!(
            prepared
                .compiled_patch()
                .numeric_parameter_value("layer::voice::amp", "gain"),
            Some(0.25)
        );
        assert_eq!(prepared.compiled_patch().audio_output_index(), Some(2));
        assert_eq!(prepared.total_latency_samples(), 0);

        let mut realtime = RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
            prepared.graph().clone(),
            prepared.compiled_patch().clone(),
            KERNEL_RENDER_SETTINGS.sample_rate_hz as f32,
            &PreparedSamplerAssets::empty(),
            &crate::patch::VoiceAllocation::default(),
            KERNEL_RENDER_SETTINGS.block_size_frames as usize,
        );
        let mut left = [0.0; KERNEL_RENDER_SETTINGS.block_size_frames as usize];
        let mut right = [0.0; KERNEL_RENDER_SETTINGS.block_size_frames as usize];
        realtime.render(&mut left, &mut right);
        assert!(
            left.iter().any(|sample| sample.abs() > f32::EPSILON),
            "the typed gain default reaches the realtime arena path"
        );
        assert!(realtime.set_numeric_parameter_by_target("layer::voice::amp", "gain", 0.0));
        realtime.render(&mut left, &mut right);
        assert!(
            left.iter().all(|sample| sample.abs() <= f32::EPSILON),
            "the arena reads the current typed control slot each block"
        );
    }

    #[test]
    fn kernel_preparation_compiles_six_channel_ports_to_contiguous_spans_and_routes() {
        let patch = load_kernel_patch_str(
            r#"
metadata: { name: surround }
ports:
  - { name: master, direction: output, signal: audio, channels: 6, maps_from: gain.audio_out }
modules:
  - { id: source, type: noise, static: { channels: 6 } }
  - { id: gain, type: gain, static: { channels: 6 }, defaults: { gain: 0.5 } }
connections:
  - { from: source.audio, to: gain.audio_in }
"#,
        )
        .expect("kernel document should load");

        let prepared = prepare_kernel_patch(&patch, &KERNEL_RENDER_SETTINGS)
            .expect("six-channel kernel patch should prepare");
        let source = prepared
            .compiled_patch()
            .nodes()
            .iter()
            .find(|node| node.id.as_str() == "source")
            .expect("source should compile");
        let gain = prepared
            .compiled_patch()
            .nodes()
            .iter()
            .find(|node| node.id.as_str() == "gain")
            .expect("gain should compile");

        assert_eq!(source.output_port_spans[0].channel_count, 6);
        assert_eq!(gain.input_port_spans[0].channel_count, 6);
        assert_eq!(gain.input_routes[0].len(), 6);
        assert_eq!(
            gain.input_routes[0]
                .iter()
                .map(|route| route.output_buffer_id)
                .collect::<Vec<_>>(),
            (source.output_port_spans[0].first_buffer
                ..source.output_port_spans[0].first_buffer + 6)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            prepared.compiled_patch().root_bus_plan().outputs()[0].channel_count(),
            6
        );

        let mut realtime = RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
            prepared.graph().clone(),
            prepared.compiled_patch().clone(),
            KERNEL_RENDER_SETTINGS.sample_rate_hz as f32,
            &PreparedSamplerAssets::empty(),
            &crate::patch::VoiceAllocation::default(),
            KERNEL_RENDER_SETTINGS.block_size_frames as usize,
        );
        let mut outputs = vec![vec![vec![0.0; 8]; 6]];
        assert_eq!(realtime.render_root_outputs(&mut outputs), 8);
        assert!(outputs[0].iter().all(|channel| {
            channel.iter().any(|sample| sample.abs() > f32::EPSILON)
                && channel.iter().all(|sample| sample.abs() <= 0.5)
        }));
    }

    #[test]
    fn named_bus_planning_validates_outputs_and_tolerates_missing_or_extra_inputs() {
        let root = GraphDefinition::new("bus-test")
            .with_port(
                KernelPort::input("sidechain", SignalType::Audio, 2)
                    .maps_to(kernel_ref("gain", builtin_ports::AUDIO_IN)),
            )
            .with_port(
                KernelPort::output("master", SignalType::Audio, 2)
                    .maps_from(kernel_ref("gain", builtin_ports::AUDIO_OUT)),
            )
            .with_node(
                Node::new(NodeId::new("gain"), module_types::GAIN)
                    .with_static_arg("channels", StaticArg::Literal(StaticValue::Int(2))),
            );
        let registry = builtin_registry();
        let buses = HostBuses::new()
            .with_input("unused", 6)
            .with_output("master", 2);

        let prepared =
            prepare_kernel_graph_with_buses(&root, &registry, &KERNEL_RENDER_SETTINGS, &buses)
                .expect(
                    "missing sidechain should bind to silence and extra input should be ignored",
                );

        assert_eq!(prepared.compiled_patch().root_bus_plan().inputs().len(), 1);
        assert!(!prepared.compiled_patch().root_bus_plan().inputs()[0].is_bound());
        let mut silent_runtime = RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
            prepared.graph().clone(),
            prepared.compiled_patch().clone(),
            KERNEL_RENDER_SETTINGS.sample_rate_hz as f32,
            &PreparedSamplerAssets::empty(),
            &crate::patch::VoiceAllocation::default(),
            8,
        );
        let mut silent_outputs = vec![vec![vec![1.0; 8]; 2]];
        assert_eq!(silent_runtime.render_root_outputs(&mut silent_outputs), 8);
        assert!(
            silent_outputs[0]
                .iter()
                .flatten()
                .all(|sample| *sample == 0.0)
        );

        let bound = prepare_kernel_graph_with_buses(
            &root,
            &registry,
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new()
                .with_input("sidechain", 2)
                .with_input("unused", 6)
                .with_output("master", 2),
        )
        .expect("matching root input should bind");
        let mut bound_runtime = RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
            bound.graph().clone(),
            bound.compiled_patch().clone(),
            KERNEL_RENDER_SETTINGS.sample_rate_hz as f32,
            &PreparedSamplerAssets::empty(),
            &crate::patch::VoiceAllocation::default(),
            8,
        );
        let inputs = vec![vec![vec![0.25; 8], vec![-0.5; 8]]];
        let mut outputs = vec![vec![vec![0.0; 8]; 2]];
        assert_eq!(bound_runtime.render_root_buses(&inputs, &mut outputs), 8);
        assert_eq!(outputs[0][0], vec![0.25; 8]);
        assert_eq!(outputs[0][1], vec![-0.5; 8]);

        let missing = prepare_kernel_graph_with_buses(
            &root,
            &registry,
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new(),
        )
        .expect_err("a missing root output bus must fail");
        assert_eq!(
            missing.diagnostics().errors().next().unwrap().error_code(),
            crate::diagnostics::error_codes::KERNEL_HOST_BUS_MISSING_OUTPUT
        );

        let mismatch = prepare_kernel_graph_with_buses(
            &root,
            &registry,
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new().with_output("master", 1),
        )
        .expect_err("a channel mismatch must fail");
        let diagnostic = mismatch.diagnostics().errors().next().unwrap();
        assert_eq!(
            diagnostic.error_code(),
            crate::diagnostics::error_codes::KERNEL_HOST_BUS_CHANNEL_MISMATCH
        );
        assert_eq!(diagnostic.expected(), Some("2 channels"));
        assert_eq!(diagnostic.actual(), Some("1 channels"));
    }

    #[test]
    fn six_channel_compensation_delay_allocates_and_preserves_each_channel() {
        let patch = load_kernel_patch_str(
            r#"
metadata: { name: surround-delay }
ports:
  - { name: master, direction: output, signal: audio, channels: 6, maps_from: delay.audio_out }
modules:
  - { id: source, type: noise, static: { channels: 6 } }
  - { id: delay, type: compensation_delay, static: { channels: 6, delay_samples: 1 } }
connections:
  - { from: source.audio, to: delay.audio_in }
"#,
        )
        .expect("six-channel delay patch should load");
        let prepared = prepare_kernel_patch(&patch, &KERNEL_RENDER_SETTINGS)
            .expect("six-channel delay patch should prepare");
        let delay = prepared
            .compiled_patch()
            .nodes()
            .iter()
            .find(|node| node.id.as_str() == "delay")
            .expect("delay should compile");
        assert_eq!(delay.input_port_spans[0].channel_count, 6);
        assert_eq!(delay.output_port_spans[0].channel_count, 6);

        let mut runtime = RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
            prepared.graph().clone(),
            prepared.compiled_patch().clone(),
            KERNEL_RENDER_SETTINGS.sample_rate_hz as f32,
            &PreparedSamplerAssets::empty(),
            &crate::patch::VoiceAllocation::default(),
            8,
        );
        let mut outputs = vec![vec![vec![0.0; 8]; 6]];
        assert_eq!(runtime.render_root_outputs(&mut outputs), 8);
        assert!(outputs[0].iter().all(|channel| {
            channel[0] == 0.0
                && channel[1..]
                    .iter()
                    .any(|sample| sample.abs() > f32::EPSILON)
        }));
    }

    #[test]
    fn six_channel_convolution_processes_each_channel_with_disjoint_state() {
        let root = GraphDefinition::new("surround-convolution")
            .with_port(
                KernelPort::input("input", SignalType::Audio, 6)
                    .maps_to(kernel_ref("convolution", builtin_ports::AUDIO_IN)),
            )
            .with_port(
                KernelPort::output("master", SignalType::Audio, 6)
                    .maps_from(kernel_ref("convolution", builtin_ports::AUDIO_OUT)),
            )
            .with_node(
                Node::new(NodeId::new("convolution"), module_types::CONVOLUTION).with_static_arg(
                    crate::kernel::builtins::CHANNELS_PARAM,
                    StaticArg::Literal(StaticValue::Int(6)),
                ),
            );
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry(),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new()
                .with_input("input", 6)
                .with_output("master", 6),
        )
        .expect("six-channel convolution should prepare");
        let mut runtime = RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
            prepared.graph().clone(),
            prepared.compiled_patch().clone(),
            KERNEL_RENDER_SETTINGS.sample_rate_hz as f32,
            &PreparedSamplerAssets::empty(),
            &crate::patch::VoiceAllocation::default(),
            8,
        );
        let inputs = vec![
            (1..=6)
                .map(|channel| vec![channel as f32 / 10.0; 8])
                .collect::<Vec<_>>(),
        ];
        let mut outputs = vec![vec![vec![0.0; 8]; 6]];

        assert_eq!(runtime.render_root_buses(&inputs, &mut outputs), 8);
        assert_eq!(outputs[0], inputs[0]);
    }

    #[test]
    fn kernel_preparation_wires_edge_and_root_compensation_and_reports_total_latency() {
        let (root, registry) = latency_test_graph();

        let prepared = prepare_kernel_graph(&root, &registry, &KERNEL_RENDER_SETTINGS)
            .expect("latency graph should prepare");

        assert_eq!(prepared.latency_plan().compensations().len(), 1);
        assert_eq!(prepared.latency_plan().root_compensations().len(), 1);
        assert_eq!(prepared.total_latency_samples(), 1);
        let delays = prepared
            .compiled_patch()
            .nodes()
            .iter()
            .filter(|node| {
                node.id
                    .as_str()
                    .starts_with(KERNEL_COMPENSATION_EDGE_PREFIX)
                    || node
                        .id
                        .as_str()
                        .starts_with(KERNEL_COMPENSATION_ROOT_PREFIX)
            })
            .collect::<Vec<_>>();
        assert_eq!(delays.len(), 2);
        assert_eq!(
            delays[0].id.as_str(),
            format!("{KERNEL_COMPENSATION_EDGE_PREFIX}0")
        );
        assert_eq!(
            delays[1].id.as_str(),
            format!(
                "{KERNEL_COMPENSATION_ROOT_PREFIX}{}::0",
                builtin_ports::RIGHT
            )
        );
        assert!(delays.iter().all(|node| {
            node.construction
                == crate::compiled_patch::CompiledConstruction::CompensationDelay { samples: 1 }
                && prepared
                    .compiled_patch()
                    .parameter_slot_index(node.id.as_str(), DELAY_SAMPLES_PARAMETER)
                    .is_none()
        }));
        assert_eq!(prepared.compiled_patch().audio_output_index(), Some(6));
    }

    #[test]
    fn kernel_compensation_aligns_impulses_and_preserves_offline_realtime_parity() {
        let (root, registry) = latency_test_graph();
        let prepared = prepare_kernel_graph(&root, &registry, &KERNEL_RENDER_SETTINGS)
            .expect("latency graph should prepare");
        let events = vec![TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
        )];

        let (offline_left, offline_right) = render_offline_compiled(
            prepared.compiled_patch(),
            events,
            &PreparedSamplerAssets::empty(),
        );
        assert_eq!(&offline_left[..3], &[0.0, 2.0, 0.0]);
        assert_eq!(&offline_right[..3], &[0.0, 1.0, 0.0]);

        let mut realtime = RealtimeGraphProcessor::new(
            prepared.graph().clone(),
            KERNEL_RENDER_SETTINGS.sample_rate_hz as f32,
        );
        realtime.note_on(60, 100);
        let mut realtime_left = vec![0.0; KERNEL_RENDER_SETTINGS.duration_frames as usize];
        let mut realtime_right = vec![0.0; KERNEL_RENDER_SETTINGS.duration_frames as usize];
        realtime.render(&mut realtime_left, &mut realtime_right);

        assert_eq!(realtime_left, offline_left);
        assert_eq!(realtime_right, offline_right);
    }

    #[test]
    fn kernel_convolution_dry_and_wet_paths_render_time_aligned() {
        let (root, registry) = latency_builtin_render_graph(Node::new(
            NodeId::new(WET_NODE_ID),
            module_types::CONVOLUTION,
        ));
        let expected_frame = Convolution::BLOCK_SIZE;
        let settings = latency_render_settings(expected_frame + 2);
        let prepared = prepare_kernel_graph(&root, &registry, &settings)
            .expect("convolution latency graph should prepare");
        let assets = PreparedSamplerAssets::from_samples_by_module(BTreeMap::from([(
            WET_NODE_ID.to_string(),
            LoadedSample::new(settings.sample_rate_hz, vec![1.0]),
        )]));

        let (left, right) =
            render_offline_compiled(prepared.compiled_patch(), vec![note_on_at(0)], &assets);

        assert_aligned_impulse(&left, expected_frame);
        assert_aligned_impulse(&right, expected_frame);
    }

    #[test]
    fn kernel_spectral_dry_and_wet_paths_render_time_aligned_per_fft_size() {
        for fft_size in [512_usize, 1024] {
            let wet = Node::new(NodeId::new(WET_NODE_ID), module_types::SPECTRAL_PROCESSOR)
                .with_static_arg(
                    SPECTRAL_FFT_SIZE_PARAMETER,
                    StaticArg::Literal(StaticValue::Int(fft_size as i64)),
                )
                .with_static_arg(
                    SPECTRAL_MODE_PARAMETER,
                    StaticArg::Literal(StaticValue::Enum(SPECTRAL_MODE_PASSTHROUGH.to_string())),
                );
            let (root, registry) = latency_builtin_render_graph(wet);
            let trigger_frame = fft_size / 2;
            let expected_frame = trigger_frame + fft_size - 1;
            let settings = latency_render_settings(expected_frame + 2);
            let prepared = prepare_kernel_graph(&root, &registry, &settings)
                .expect("spectral latency graph should prepare");
            let compiled_wet = prepared
                .compiled_patch()
                .nodes()
                .iter()
                .find(|node| node.id.as_str() == WET_NODE_ID)
                .expect("spectral node should compile");
            assert_eq!(
                compiled_wet.construction,
                crate::compiled_patch::CompiledConstruction::SpectralProcessor {
                    fft_size,
                    mode: crate::spectral::SpectralMode::Passthrough,
                }
            );
            assert_eq!(
                prepared
                    .compiled_patch()
                    .parameter_slot_index(WET_NODE_ID, SPECTRAL_FFT_SIZE_PARAMETER),
                None,
                "numeric static arguments must not become runtime slots"
            );
            assert_eq!(
                prepared
                    .compiled_patch()
                    .numeric_parameter_value(WET_NODE_ID, builtin_ports::THRESHOLD),
                Some(-40.0)
            );
            assert_eq!(
                prepared
                    .compiled_patch()
                    .numeric_parameter_value(WET_NODE_ID, builtin_ports::MIX),
                Some(1.0)
            );

            let (left, right) = render_offline_compiled(
                prepared.compiled_patch(),
                vec![note_on_at(trigger_frame as u64)],
                &PreparedSamplerAssets::empty(),
            );

            assert_aligned_impulse(&left, expected_frame);
            assert_aligned_impulse(&right, expected_frame);
        }
    }

    fn latency_builtin_render_graph(wet: Node) -> (GraphDefinition, DefinitionRegistry) {
        let registry = builtin_registry();
        let root = GraphDefinition::new("builtin-latency-render-test")
            .with_port(
                KernelPort::output(builtin_ports::LEFT, SignalType::Audio, 1)
                    .maps_from(kernel_ref("mix", builtin_ports::MIX)),
            )
            .with_port(
                KernelPort::output(builtin_ports::RIGHT, SignalType::Audio, 1)
                    .maps_from(kernel_ref("mix", builtin_ports::MIX)),
            )
            .with_node(Node::new(NodeId::new("midi"), module_types::MIDI_INPUT))
            .with_node(Node::new(NodeId::new("impulse"), module_types::IMPULSE))
            .with_node(wet)
            .with_node(Node::new(NodeId::new("mix"), module_types::AUDIO_MIXER))
            .with_connection(Connection::new(
                kernel_ref("midi", builtin_ports::EVENTS),
                kernel_ref("impulse", builtin_ports::TRIGGER),
            ))
            .with_connection(Connection::new(
                kernel_ref("impulse", builtin_ports::AUDIO),
                kernel_ref(WET_NODE_ID, builtin_ports::AUDIO_IN),
            ))
            .with_connection(Connection::new(
                kernel_ref("impulse", builtin_ports::AUDIO),
                kernel_ref("mix", builtin_ports::INPUTS),
            ))
            .with_connection(Connection::new(
                kernel_ref(WET_NODE_ID, builtin_ports::AUDIO_OUT),
                kernel_ref("mix", builtin_ports::INPUTS),
            ));
        (root, registry)
    }

    fn latency_render_settings(duration_frames: usize) -> patch::RenderSettings {
        patch::RenderSettings {
            sample_rate_hz: KERNEL_RENDER_SETTINGS.sample_rate_hz,
            block_size_frames: 64,
            duration_frames: duration_frames as u64,
        }
    }

    fn note_on_at(frame: u64) -> TimedInputEvent {
        TimedInputEvent::new(
            frame,
            ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
        )
    }

    fn assert_aligned_impulse(samples: &[f32], expected_frame: usize) {
        for (frame, sample) in samples.iter().copied().enumerate() {
            let expected = if frame == expected_frame { 2.0 } else { 0.0 };
            assert!(
                (sample - expected).abs() <= IMPULSE_TOLERANCE,
                "frame {frame} was {sample}, expected {expected} within {IMPULSE_TOLERANCE}"
            );
        }
    }

    fn latency_test_graph() -> (GraphDefinition, DefinitionRegistry) {
        let registry = builtin_registry();
        let root = GraphDefinition::new("latency-test")
            .with_port(
                KernelPort::output("left", SignalType::Audio, 1)
                    .maps_from(kernel_ref("mix", builtin_ports::MIX)),
            )
            .with_port(
                KernelPort::output("right", SignalType::Audio, 1)
                    .maps_from(kernel_ref("impulse", builtin_ports::AUDIO)),
            )
            .with_node(Node::new(NodeId::new("midi"), module_types::MIDI_INPUT))
            .with_node(Node::new(NodeId::new("impulse"), module_types::IMPULSE))
            .with_node(
                Node::new(NodeId::new("wet"), module_types::COMPENSATION_DELAY).with_static_arg(
                    DELAY_SAMPLES_PARAMETER,
                    StaticArg::Literal(StaticValue::Int(1)),
                ),
            )
            .with_node(Node::new(NodeId::new("mix"), module_types::AUDIO_MIXER))
            .with_connection(Connection::new(
                kernel_ref("midi", builtin_ports::EVENTS),
                kernel_ref("impulse", builtin_ports::TRIGGER),
            ))
            .with_connection(Connection::new(
                kernel_ref("impulse", builtin_ports::AUDIO),
                kernel_ref("wet", builtin_ports::AUDIO_IN),
            ))
            .with_connection(Connection::new(
                kernel_ref("impulse", builtin_ports::AUDIO),
                kernel_ref("mix", builtin_ports::INPUTS),
            ))
            .with_connection(Connection::new(
                kernel_ref("wet", builtin_ports::AUDIO_OUT),
                kernel_ref("mix", builtin_ports::INPUTS),
            ));
        (root, registry)
    }

    fn kernel_ref(node: &str, port: &str) -> KernelPortRef {
        KernelPortRef::new(NodeId::new(node), port)
    }
}
