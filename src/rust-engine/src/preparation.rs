use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

use crate::compiled_patch::{self, CompileError, CompiledPatch};
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
const TRANSITIONAL_ROOT_EXPECTATION: &str = "mono audio outputs named left and right";

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
/// graph and latency plan for inspection while executing through the unchanged
/// legacy compiled-patch back end during migration.
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
    let validation = root.validate(registry);
    if !validation.is_ok() {
        return Err(validation.diagnostics().clone().into());
    }

    let flattened_graph = root
        .flatten(registry)
        .map_err(KernelPreparationError::from)?;
    validate_transitional_root(&flattened_graph)?;
    validate_atomic_ports(&flattened_graph)?;
    let latency_plan = flattened_graph
        .balance_latency()
        .map_err(KernelPreparationError::from)?;
    let graph = lower_kernel_graph(&flattened_graph, &latency_plan)?;
    graph
        .validate()
        .map_err(|error| KernelPreparationError::from(error.to_diagnostics()))?;
    let compiled_patch = compiled_patch::compile(&graph, render_settings).map_err(|error| {
        KernelPreparationError::from(diagnostics::Diagnostics::from(error.to_diagnostic()))
    })?;

    Ok(PreparedKernelInstrument {
        flattened_graph,
        latency_plan,
        graph,
        compiled_patch,
    })
}

fn validate_transitional_root(flattened: &FlattenedGraph) -> Result<(), KernelPreparationError> {
    let mut ports = flattened.root_ports().iter().collect::<Vec<_>>();
    ports.sort_by_key(|port| port.name());
    let supported = ports.len() == 2
        && ports.iter().all(|port| {
            port.direction() == PortDirection::Output
                && port.signal_type() == SignalType::Audio
                && port.channels() == 1
                && matches!(
                    port.name(),
                    crate::graph::builtin_ports::LEFT | crate::graph::builtin_ports::RIGHT
                )
                && flattened
                    .root_output_sources()
                    .get(port.name())
                    .is_some_and(|sources| sources.len() == 1)
        })
        && ports[0].name() != ports[1].name();
    if supported {
        return Ok(());
    }

    let actual = if ports.is_empty() {
        "no root ports".to_string()
    } else {
        ports
            .iter()
            .map(|port| {
                format!(
                    "{} {:?} {:?} {}ch ({} sources)",
                    port.name(),
                    port.direction(),
                    port.signal_type(),
                    port.channels(),
                    flattened
                        .root_output_sources()
                        .get(port.name())
                        .map_or(0, Vec::len)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    Err(KernelPreparationError::from(diagnostics::Diagnostics::from(
        Diagnostic::new(
            diagnostics::error_codes::KERNEL_PREPARATION_UNSUPPORTED_ROOT,
            Severity::Error,
            "the transitional legacy output bridge supports exactly mono root outputs named 'left' and 'right'",
        )
        .with_expected(TRANSITIONAL_ROOT_EXPECTATION)
        .with_actual(actual)
        .with_suggested_fix("declare mono left/right root outputs until named host buses replace the transitional sink"),
    )))
}

fn validate_atomic_ports(flattened: &FlattenedGraph) -> Result<(), KernelPreparationError> {
    let mut diagnostics = diagnostics::Diagnostics::new();
    for node in flattened.nodes() {
        for port in node.ports().iter().filter(|port| port.channels() != 1) {
            diagnostics.push(
                Diagnostic::new(
                    diagnostics::error_codes::KERNEL_PREPARATION_UNSUPPORTED_ATOMIC_PORT,
                    Severity::Error,
                    format!(
                        "atomic port '{}.{}' has {} channels, but the transitional legacy graph represents only mono ports",
                        node.id().as_str(),
                        port.name(),
                        port.channels()
                    ),
                )
                .with_module_id(node.id().as_str())
                .with_port_name(port.name())
                .with_expected("1 channel")
                .with_actual(format!("{} channels", port.channels())),
            );
        }
    }
    if diagnostics.has_errors() {
        Err(KernelPreparationError::from(diagnostics))
    } else {
        Ok(())
    }
}

fn lower_kernel_graph(
    flattened: &FlattenedGraph,
    latency_plan: &LatencyPlan,
) -> Result<Graph, KernelPreparationError> {
    use crate::builtins::{BuiltInModuleRegistry, module_types};
    use crate::graph::builtin_ports;

    let legacy_registry = BuiltInModuleRegistry::new();
    let mut ids = flattened
        .nodes()
        .iter()
        .map(|node| node.id().as_str().to_string())
        .collect::<BTreeSet<_>>();
    let mut modules = Vec::new();
    for node in flattened.nodes() {
        let legacy_definition = legacy_registry.get(node.definition());
        let mut lowered = ModuleNode::new(ModuleId::new(node.id().as_str()), node.definition())
            .with_execution_scope(ExecutionScope::Global);
        for port in node.ports() {
            lowered = match port.direction() {
                PortDirection::Input => {
                    let mixes = legacy_definition.is_some_and(|definition| {
                        definition.inputs().iter().any(|input| {
                            input.name() == port.name() && input.accepts_multiple_sources()
                        })
                    });
                    if mixes {
                        lowered.with_mixing_input(port.name(), port.signal_type())
                    } else {
                        lowered.with_input(port.name(), port.signal_type())
                    }
                }
                PortDirection::Output => lowered.with_output(port.name(), port.signal_type()),
            };
        }
        let mut parameters = node
            .static_args()
            .iter()
            .map(|(name, value)| (name.clone(), static_value_to_string(value)))
            .collect::<BTreeMap<_, _>>();
        parameters.extend(
            node.port_defaults()
                .iter()
                .map(|(name, value)| (name.clone(), value.to_string())),
        );
        modules.push(lowered.with_params(parameters));
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

    for root_name in [builtin_ports::LEFT, builtin_ports::RIGHT] {
        let source = &flattened.root_output_sources()[root_name][0];
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
            cables.push(Cable::new(
                legacy_ref(source),
                PortRef::new(ModuleId::new(&id), builtin_ports::AUDIO_IN),
            ));
            PortRef::new(ModuleId::new(id), builtin_ports::AUDIO_OUT)
        } else {
            legacy_ref(source)
        };
        cables.push(Cable::new(
            source,
            PortRef::new(ModuleId::new(KERNEL_OUTPUT_NODE_ID), root_name),
        ));
    }

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
    Ok(Graph::new(modules, cables))
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
    let compiled_patch = compile_patch(&graph, &patch_doc)?;

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
    use crate::builtins::{DELAY_SAMPLES_PARAMETER, module_types};
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
    use crate::script::ScriptEvent;
    use std::fs;

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
    fn kernel_preparation_flattens_nested_composites_and_lowers_defaults_and_static_args() {
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
            osc.parameters.get("waveform").map(String::as_str),
            Some("sine")
        );
        assert_eq!(osc.parameters.get("pitch").map(String::as_str), Some("2"));
        let amp = prepared
            .compiled_patch()
            .nodes()
            .iter()
            .find(|node| node.id.as_str() == "layer::voice::amp")
            .expect("gain should compile");
        assert_eq!(amp.parameters.get("gain").map(String::as_str), Some("0.25"));
        assert_eq!(prepared.compiled_patch().audio_output_index(), Some(2));
        assert_eq!(prepared.total_latency_samples(), 0);
    }

    #[test]
    fn kernel_preparation_rejects_unsupported_root_shape_with_structured_diagnostic() {
        let patch = load_kernel_patch_str(
            r#"
metadata: { name: unsupported }
ports:
  - { name: master, direction: output, signal: audio, channels: 2, maps_from: noise.audio }
modules:
  - { id: noise, type: noise }
connections: []
"#,
        )
        .expect("kernel document should load");

        let error = prepare_kernel_patch(&patch, &KERNEL_RENDER_SETTINGS)
            .expect_err("transitional bridge should reject a multichannel master output");
        let diagnostics = error.to_diagnostics();

        assert_eq!(diagnostics.errors().count(), 1);
        let diagnostic = diagnostics.errors().next().expect("one diagnostic");
        assert_eq!(
            diagnostic.error_code(),
            crate::diagnostics::error_codes::KERNEL_PREPARATION_UNSUPPORTED_ROOT
        );
        assert_eq!(
            diagnostic.expected(),
            Some(TRANSITIONAL_ROOT_EXPECTATION)
        );
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
            node.parameters
                .get(DELAY_SAMPLES_PARAMETER)
                .map(String::as_str)
                == Some("1")
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
