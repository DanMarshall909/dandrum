use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use crate::builtins::module_kind::ModuleKind;
use crate::builtins::module_types;
use crate::compiled_patch::{
    self, CompileError, CompiledConstruction, CompiledNodeData, CompiledPatch,
    CompiledPolyOutputAccumulator, CompiledPolyRegion, CompiledPolyVoiceStorage, CompiledPortSpan,
    CompiledResourceHandles, CompiledRootPort, CompiledSampleZone, ImpulseResponseResourceHandle,
    RootBusPlan, SampleChokeMode, SampleInterpolation, SampleResourceHandle, SampleSelectionMode,
};
use crate::diagnostics::{self, Diagnostic, Severity};
use crate::graph::{Cable, Graph, ModuleId, ModuleNode, PortDirection, PortRef, SignalType};
use crate::kernel::document::{
    KernelPatch, SampleAssets, SampleMap, SampleSlice, SampleSource, SampleZone,
};
use crate::kernel::flatten::{FlattenedGraph, FlattenedPolyRegion};
use crate::kernel::latency::LatencyPlan;
use crate::kernel::{
    DefinitionRegistry, GraphDefinition, PortMetadata, ResourceKind, ResourceOrigin, ResourceRef,
    StaticArg, StaticValue,
};
use crate::module_reference::MacroRoots;
#[cfg(test)]
use crate::patch::{self, ParameterValue, PatchDocument};
use crate::patch::{PresetDocument, RenderSettings};
use crate::sample::LoadedSample;
#[cfg(test)]
use crate::sample::{self, PreparedSamplerAssets, SampleLoadError};

const KERNEL_COMPENSATION_EDGE_PREFIX: &str = "compensation::edge::";
const KERNEL_COMPENSATION_ROOT_PREFIX: &str = "compensation::root::";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparationContext {
    document_root: PathBuf,
    sample_rate_hz: u32,
    macro_roots: MacroRoots,
}

impl PreparationContext {
    pub fn new(document_root: impl Into<PathBuf>, sample_rate_hz: u32) -> Self {
        Self {
            document_root: document_root.into(),
            sample_rate_hz,
            macro_roots: MacroRoots::new(),
        }
    }

    pub fn with_macro_roots(mut self, macro_roots: MacroRoots) -> Self {
        self.macro_roots = macro_roots;
        self
    }

    pub fn document_root(&self) -> &Path {
        &self.document_root
    }

    pub fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    pub fn macro_roots(&self) -> &MacroRoots {
        &self.macro_roots
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedResource {
    kind: ResourceKind,
    canonical_path: PathBuf,
    sample: Arc<LoadedSample>,
}

impl ResolvedResource {
    pub fn kind(&self) -> ResourceKind {
        self.kind
    }

    pub fn canonical_path(&self) -> &Path {
        &self.canonical_path
    }

    pub fn sample(&self) -> &LoadedSample {
        &self.sample
    }

    fn shared_sample(&self) -> Arc<LoadedSample> {
        Arc::clone(&self.sample)
    }

    #[cfg(test)]
    fn shares_data_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.sample, &other.sample)
    }
}

pub struct ResourceResolver<'a> {
    context: &'a PreparationContext,
    loaded: BTreeMap<(PathBuf, u32, bool), Arc<LoadedSample>>,
}

impl<'a> ResourceResolver<'a> {
    pub fn new(context: &'a PreparationContext) -> Self {
        Self {
            context,
            loaded: BTreeMap::new(),
        }
    }

    pub fn loaded_resource_count(&self) -> usize {
        self.loaded.len()
    }

    pub fn resolve(
        &mut self,
        reference: &ResourceRef,
    ) -> Result<ResolvedResource, diagnostics::Diagnostics> {
        self.resolve_with_rate_policy(reference, true)
    }

    fn resolve_with_rate_policy(
        &mut self,
        reference: &ResourceRef,
        require_matching_rate: bool,
    ) -> Result<ResolvedResource, diagnostics::Diagnostics> {
        if reference.path().is_absolute()
            || reference
                .path()
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(resource_path_escape(reference, None));
        }

        let origin_root = match reference.origin() {
            ResourceOrigin::Document => self.context.document_root(),
            ResourceOrigin::Package(root) => root,
        };
        let canonical_root = origin_root.canonicalize().map_err(|error| {
            resource_load_failed(
                reference,
                origin_root,
                format!("failed to resolve resource root: {error}"),
            )
        })?;
        let joined = canonical_root.join(reference.path());
        let canonical_path = joined.canonicalize().map_err(|error| {
            resource_load_failed(
                reference,
                &joined,
                format!("failed to resolve resource path: {error}"),
            )
        })?;
        if !canonical_path.starts_with(&canonical_root) {
            return Err(resource_path_escape(reference, Some(&canonical_path)));
        }

        let key = (
            canonical_path.clone(),
            self.context.sample_rate_hz(),
            require_matching_rate,
        );
        let sample = if let Some(sample) = self.loaded.get(&key) {
            Arc::clone(sample)
        } else {
            let loaded = if require_matching_rate {
                crate::audio_loading::load_pcm_wav(&canonical_path, self.context.sample_rate_hz())
            } else {
                crate::audio_loading::load_pcm_wav_any_rate(&canonical_path)
            }
            .map_err(|message| resource_load_failed(reference, &canonical_path, message))?;
            let sample = Arc::new(LoadedSample::with_source_channels(
                loaded.sample_rate_hz(),
                loaded.source_channel_count(),
                loaded.frames().to_vec(),
            ));
            self.loaded.insert(key, Arc::clone(&sample));
            sample
        };

        Ok(ResolvedResource {
            kind: reference.kind(),
            canonical_path,
            sample,
        })
    }
}

fn resource_path_escape(
    reference: &ResourceRef,
    canonical_path: Option<&Path>,
) -> diagnostics::Diagnostics {
    let actual = canonical_path.unwrap_or_else(|| reference.path());
    Diagnostic::new(
        diagnostics::error_codes::KERNEL_RESOURCE_PATH_ESCAPE,
        Severity::Error,
        format!(
            "resource path '{}' escapes its authoring root",
            reference.path().display()
        ),
    )
    .with_expected("a relative canonical path beneath the authoring root")
    .with_actual(actual.display().to_string())
    .into()
}

fn resource_load_failed(
    reference: &ResourceRef,
    path: &Path,
    message: impl fmt::Display,
) -> diagnostics::Diagnostics {
    Diagnostic::new(
        diagnostics::error_codes::KERNEL_RESOURCE_LOAD_FAILED,
        Severity::Error,
        format!(
            "failed to load {} resource '{}' at {}: {message}",
            reference.kind(),
            reference.path().display(),
            path.display()
        ),
    )
    .with_actual(path.display().to_string())
    .into()
}

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
#[cfg(test)]
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
#[cfg(test)]
pub(crate) struct PreparationDiagnostics {
    messages: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg(test)]
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
    sample_assets: PreparedSampleAssets,
    flattened_graph: FlattenedGraph,
    latency_plan: LatencyPlan,
    compensation_metadata: Vec<PreparedCompensationMetadata>,
    graph: Graph,
    compiled_patch: CompiledPatch,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PreparedSampleAssets {
    sources: Vec<PreparedSampleSource>,
    maps: Vec<SampleMap>,
    prepared_maps: Vec<PreparedSampleMap>,
    map_indices: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedSampleSource {
    declaration: SampleSource,
    resource: ResolvedResource,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedSampleMap {
    declaration: SampleMap,
    zones: Vec<PreparedSampleZone>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedSampleZone {
    declaration: SampleZone,
    source_index: usize,
    region_index: usize,
}

impl PreparedSampleAssets {
    pub fn sources(&self) -> &[PreparedSampleSource] {
        &self.sources
    }

    pub fn maps(&self) -> &[SampleMap] {
        &self.maps
    }

    pub fn map_by_id(&self, id: &str) -> Option<&PreparedSampleMap> {
        self.map_indices
            .get(id)
            .and_then(|index| self.prepared_maps.get(*index))
    }
}

impl PreparedSampleMap {
    pub fn selection_seed(&self) -> u64 {
        self.declaration.selection_seed
    }

    pub fn selection_mode(&self) -> Option<&str> {
        self.declaration.selection_mode.as_deref()
    }

    pub fn zones(&self) -> &[PreparedSampleZone] {
        &self.zones
    }
}

impl PreparedSampleZone {
    pub fn declaration(&self) -> &SampleZone {
        &self.declaration
    }

    pub fn source_index(&self) -> usize {
        self.source_index
    }

    pub fn region_index(&self) -> usize {
        self.region_index
    }
}

impl PreparedSampleSource {
    pub fn id(&self) -> &str {
        &self.declaration.id
    }

    pub fn sample(&self) -> &LoadedSample {
        self.resource.sample()
    }

    pub fn declaration(&self) -> &SampleSource {
        &self.declaration
    }

    /// Slice numbers follow the authored table order and need no render-time search.
    pub fn slice(&self, index: usize) -> Option<&SampleSlice> {
        self.declaration.slices.get(index)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedNodeMetadata {
    id: String,
    definition: String,
    ports: Vec<PortMetadata>,
}

/// A compiler-inserted audio delay exposed for prepared-graph inspection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedCompensationMetadata {
    region_path: Option<String>,
    source: crate::kernel::PortRef,
    destination: Option<crate::kernel::PortRef>,
    root_port: Option<String>,
    samples: u32,
}

impl PreparedCompensationMetadata {
    /// The containing poly region, or `None` for the root graph.
    pub fn region_path(&self) -> Option<&str> {
        self.region_path.as_deref()
    }

    pub fn source(&self) -> &crate::kernel::PortRef {
        &self.source
    }

    pub fn destination(&self) -> Option<&crate::kernel::PortRef> {
        self.destination.as_ref()
    }

    pub fn root_port(&self) -> Option<&str> {
        self.root_port.as_deref()
    }

    pub fn samples(&self) -> u32 {
        self.samples
    }
}

impl PreparedNodeMetadata {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn definition(&self) -> &str {
        &self.definition
    }

    pub fn ports(&self) -> &[PortMetadata] {
        &self.ports
    }
}

impl PreparedKernelInstrument {
    pub fn sample_assets(&self) -> &PreparedSampleAssets {
        &self.sample_assets
    }

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

    /// Resolved root ports, expressed in the same schema as declaration
    /// discovery. Static channel references have become literal counts.
    pub fn root_port_metadata(&self) -> Vec<PortMetadata> {
        self.flattened_graph
            .root_ports()
            .iter()
            .map(PortMetadata::resolved)
            .collect()
    }

    /// Includes compiler-generated nodes such as control-to-audio promotion.
    pub fn node_metadata(&self) -> Vec<PreparedNodeMetadata> {
        let mut nodes = Vec::new();
        collect_prepared_node_metadata(&self.flattened_graph, &self.compiled_patch, "", &mut nodes);
        nodes
    }

    /// Reports where audio compensation was inserted during preparation.
    pub fn compensation_metadata(&self) -> &[PreparedCompensationMetadata] {
        &self.compensation_metadata
    }
}

fn append_compensation_metadata(
    plan: &LatencyPlan,
    region_path: Option<&str>,
    reports: &mut Vec<PreparedCompensationMetadata>,
) {
    reports.extend(
        plan.compensations()
            .iter()
            .map(|compensation| PreparedCompensationMetadata {
                region_path: region_path.map(str::to_string),
                source: compensation.connection().source().clone(),
                destination: Some(compensation.connection().destination().clone()),
                root_port: None,
                samples: compensation.samples(),
            }),
    );
    reports.extend(plan.root_compensations().iter().map(|compensation| {
        PreparedCompensationMetadata {
            region_path: region_path.map(str::to_string),
            source: compensation.source().clone(),
            destination: None,
            root_port: Some(compensation.root_port().to_string()),
            samples: compensation.samples(),
        }
    }));
}

fn collect_prepared_node_metadata(
    flattened: &FlattenedGraph,
    compiled: &CompiledPatch,
    prefix: &str,
    nodes: &mut Vec<PreparedNodeMetadata>,
) {
    nodes.extend(flattened.nodes().iter().map(|node| PreparedNodeMetadata {
        id: format!("{prefix}{}", node.id().as_str()),
        definition: node.definition().to_string(),
        ports: node.ports().iter().map(PortMetadata::resolved).collect(),
    }));
    for region in compiled.poly_regions() {
        let child_prefix = format!("{prefix}{}::", region.node_id());
        collect_prepared_node_metadata(
            region.flattened_voice(),
            region.child_patch(),
            &child_prefix,
            nodes,
        );
    }
}

#[cfg(test)]
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

#[cfg(test)]
impl PreparationDiagnostics {
    #[allow(dead_code)]
    pub(crate) fn messages(&self) -> &[String] {
        &self.messages
    }
}

#[cfg(test)]
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
    if !patch.sample_assets().sample_sources.is_empty()
        || !patch.sample_assets().sample_maps.is_empty()
    {
        return Err(sample_preparation_error(
            diagnostics::error_codes::KERNEL_SAMPLE_CONTEXT_REQUIRED,
            "sample assets require a preparation context with a document root",
        ));
    }
    validate_sample_declarations(patch.sample_assets())?;
    validate_sample_module_options(patch)?;
    prepare_kernel_graph(patch.root(), patch.registry(), render_settings)
}

pub fn prepare_kernel_patch_with_preset(
    patch: &KernelPatch,
    preset: &PresetDocument,
    render_settings: &RenderSettings,
) -> Result<PreparedKernelInstrument, KernelPreparationError> {
    let applied = patch
        .apply_preset(preset)
        .map_err(KernelPreparationError::from)?;
    prepare_kernel_patch(&applied, render_settings)
}

pub fn prepare_kernel_patch_with_preset_and_context(
    patch: &KernelPatch,
    preset: &PresetDocument,
    render_settings: &RenderSettings,
    context: &PreparationContext,
) -> Result<PreparedKernelInstrument, KernelPreparationError> {
    let applied = patch
        .apply_preset(preset)
        .map_err(KernelPreparationError::from)?;
    prepare_kernel_patch_with_context(&applied, render_settings, context)
}

pub fn prepare_kernel_patch_with_context(
    patch: &KernelPatch,
    render_settings: &RenderSettings,
    context: &PreparationContext,
) -> Result<PreparedKernelInstrument, KernelPreparationError> {
    validate_sample_declarations(patch.sample_assets())?;
    validate_sample_module_options(patch)?;
    let buses = default_kernel_output_buses(patch.root());
    prepare_kernel_graph_with_buses_internal(
        patch.root(),
        patch.registry(),
        render_settings,
        &buses,
        Some(context),
        false,
        Some(patch.sample_assets()),
    )
}

fn sample_preparation_error(
    code: &'static str,
    message: impl Into<String>,
) -> KernelPreparationError {
    diagnostics::Diagnostics::from(Diagnostic::new(code, Severity::Error, message.into())).into()
}

fn validate_sample_module_options(patch: &KernelPatch) -> Result<(), KernelPreparationError> {
    for node in patch.root().nodes() {
        if node.definition_ref() == module_types::SAMPLE_MAP_PLAYER {
            if let Some(StaticArg::Literal(StaticValue::Int(max_voices))) =
                node.static_args().get("max_voices")
            {
                if !(1..=128).contains(max_voices) {
                    return Err(sample_preparation_error(
                        diagnostics::error_codes::KERNEL_SAMPLE_INVALID_VOICE_LIMIT,
                        format!(
                            "sample_map_player '{}' max_voices must be 1..=128",
                            node.id().as_str()
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_sample_declarations(assets: &SampleAssets) -> Result<(), KernelPreparationError> {
    for source in &assets.sample_sources {
        for region in &source.regions {
            if region.start_frame >= region.end_frame {
                return Err(sample_preparation_error(
                    diagnostics::error_codes::KERNEL_SAMPLE_INVALID_REGION,
                    format!(
                        "sample source '{}' region '{}' must have end_frame after start_frame",
                        source.id, region.id
                    ),
                ));
            }
            if region.pan.is_some_and(|pan| !(-1.0..=1.0).contains(&pan))
                || region
                    .gain_db
                    .is_some_and(|gain| !(-96.0..=24.0).contains(&gain))
                || region.fade_in_ms.is_some_and(|fade| fade < 0.0)
                || region.fade_out_ms.is_some_and(|fade| fade < 0.0)
            {
                return Err(sample_preparation_error(
                    diagnostics::error_codes::KERNEL_SAMPLE_INVALID_REGION,
                    format!(
                        "sample source '{}' region '{}' has invalid gain, pan, or fade values",
                        source.id, region.id
                    ),
                ));
            }
            if let Some(loop_settings) = &region.loop_settings {
                if loop_settings.mode != "forward" {
                    return Err(sample_preparation_error(
                        diagnostics::error_codes::KERNEL_SAMPLE_UNSUPPORTED_MODE,
                        format!(
                            "sample source '{}' region '{}' does not support loop mode '{}'",
                            source.id, region.id, loop_settings.mode
                        ),
                    ));
                }
                if loop_settings.start_frame < region.start_frame
                    || loop_settings.start_frame >= loop_settings.end_frame
                    || loop_settings.end_frame > region.end_frame
                    || loop_settings.crossfade_ms.is_some_and(|fade| fade < 0.0)
                {
                    return Err(sample_preparation_error(
                        diagnostics::error_codes::KERNEL_SAMPLE_INVALID_LOOP,
                        format!(
                            "sample source '{}' region '{}' has loop points outside its playback window",
                            source.id, region.id
                        ),
                    ));
                }
            }
        }
    }
    for map in &assets.sample_maps {
        for zone in &map.zones {
            if zone.key_range[0] > zone.key_range[1]
                || zone.velocity_range[0] > zone.velocity_range[1]
                || zone
                    .gain_db
                    .is_some_and(|gain| !(-96.0..=24.0).contains(&gain))
                || zone.pan.is_some_and(|pan| !(-1.0..=1.0).contains(&pan))
                || zone
                    .pitch_semitones
                    .is_some_and(|pitch| !(-48.0..=48.0).contains(&pitch))
            {
                return Err(sample_preparation_error(
                    diagnostics::error_codes::KERNEL_SAMPLE_INVALID_ZONE,
                    format!(
                        "sample map '{}' has a zone with an invalid key, velocity, gain, pan, or pitch value",
                        map.id
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn prepare_sample_assets(
    assets: &SampleAssets,
    context: &PreparationContext,
) -> Result<PreparedSampleAssets, KernelPreparationError> {
    let mut resolver = ResourceResolver::new(context);
    let mut sources = Vec::with_capacity(assets.sample_sources.len());
    for declaration in &assets.sample_sources {
        let resource = resolver
            .resolve_with_rate_policy(&declaration.resource, false)
            .map_err(|diagnostics| {
                if diagnostics.errors().any(|diagnostic| {
                    diagnostic.error_code() == diagnostics::error_codes::KERNEL_RESOURCE_PATH_ESCAPE
                }) {
                    return KernelPreparationError::from(diagnostics);
                }
                let origin_root = match declaration.resource.origin() {
                    ResourceOrigin::Document => context.document_root(),
                    ResourceOrigin::Package(root) => root,
                };
                let path = origin_root.join(declaration.resource.path());
                let code = if !path.exists() {
                    diagnostics::error_codes::KERNEL_SAMPLE_MISSING_FILE
                } else if diagnostics
                    .errors()
                    .any(|diagnostic| diagnostic.message().contains("unsupported format"))
                {
                    diagnostics::error_codes::KERNEL_SAMPLE_UNSUPPORTED_FORMAT
                } else {
                    diagnostics::error_codes::KERNEL_RESOURCE_LOAD_FAILED
                };
                sample_preparation_error(
                    code,
                    format!(
                        "sample source '{}' at {}: {diagnostics}",
                        declaration.id,
                        path.display()
                    ),
                )
            })?;
        let frame_count = resource.sample().frames().len() as u64;
        let frames_per_ms = resource.sample().sample_rate_hz() as f64 / 1000.0;
        for region in &declaration.regions {
            if region.end_frame > frame_count {
                return Err(sample_preparation_error(
                    diagnostics::error_codes::KERNEL_SAMPLE_INVALID_REGION,
                    format!(
                        "sample source '{}' region '{}' ends at frame {}, beyond source length {}",
                        declaration.id, region.id, region.end_frame, frame_count
                    ),
                ));
            }
            let region_frames = (region.end_frame - region.start_frame) as f64;
            let fade_in_frames = region.fade_in_ms.unwrap_or(0.0) * frames_per_ms;
            let fade_out_frames = region.fade_out_ms.unwrap_or(0.0) * frames_per_ms;
            if fade_in_frames + fade_out_frames > region_frames {
                return Err(sample_preparation_error(
                    diagnostics::error_codes::KERNEL_SAMPLE_INVALID_REGION,
                    format!(
                        "sample source '{}' region '{}' fades exceed its {}-frame window",
                        declaration.id,
                        region.id,
                        region.end_frame - region.start_frame
                    ),
                ));
            }
            if let Some(loop_settings) = &region.loop_settings {
                let loop_frames = (loop_settings.end_frame - loop_settings.start_frame) as f64;
                if loop_settings.crossfade_ms.unwrap_or(0.0) * frames_per_ms > loop_frames * 0.5 {
                    return Err(sample_preparation_error(
                        diagnostics::error_codes::KERNEL_SAMPLE_INVALID_LOOP,
                        format!(
                            "sample source '{}' region '{}' loop crossfade exceeds half the loop window",
                            declaration.id, region.id
                        ),
                    ));
                }
            }
        }
        let mut slice_ids = BTreeSet::new();
        for slice in &declaration.slices {
            if slice.start_frame >= slice.end_frame
                || slice.end_frame > frame_count
                || !slice_ids.insert(slice.id.as_str())
            {
                return Err(sample_preparation_error(
                    diagnostics::error_codes::KERNEL_SAMPLE_INVALID_SLICE,
                    format!(
                        "sample source '{}' has invalid or duplicate slice '{}'",
                        declaration.id, slice.id
                    ),
                ));
            }
        }
        let mut cue_ids = BTreeSet::new();
        if declaration
            .cues
            .iter()
            .any(|cue| cue.frame >= frame_count || !cue_ids.insert(cue.id.as_str()))
        {
            return Err(sample_preparation_error(
                diagnostics::error_codes::KERNEL_SAMPLE_INVALID_TIMING,
                format!(
                    "sample source '{}' has a duplicate or out-of-bounds cue",
                    declaration.id
                ),
            ));
        }
        if let Some(analysis) = &declaration.analysis {
            if analysis.tempo_bpm.is_some_and(|tempo| tempo <= 0.0)
                || analysis
                    .confidence
                    .is_some_and(|confidence| !(0.0..=1.0).contains(&confidence))
            {
                return Err(sample_preparation_error(
                    diagnostics::error_codes::KERNEL_SAMPLE_INVALID_TIMING,
                    format!(
                        "sample source '{}' has invalid tempo or analysis confidence",
                        declaration.id
                    ),
                ));
            }
            if let Some(grid) = &analysis.beat_grid {
                if [&grid.beats, &grid.downbeats].into_iter().any(|markers| {
                    markers.iter().any(|frame| *frame >= frame_count)
                        || markers.windows(2).any(|pair| pair[0] >= pair[1])
                }) || grid
                    .downbeats
                    .iter()
                    .any(|frame| grid.beats.binary_search(frame).is_err())
                {
                    return Err(sample_preparation_error(
                        diagnostics::error_codes::KERNEL_SAMPLE_INVALID_TIMING,
                        format!(
                            "sample source '{}' has an invalid explicit beat grid",
                            declaration.id
                        ),
                    ));
                }
            }
        }
        sources.push(PreparedSampleSource {
            declaration: declaration.clone(),
            resource,
        });
    }
    let mut regions = BTreeMap::new();
    for (source_index, source) in sources.iter().enumerate() {
        for (region_index, region) in source.declaration.regions.iter().enumerate() {
            let qualified = format!("{}.{}", source.id(), region.id);
            if regions
                .insert(qualified.clone(), (source_index, region_index))
                .is_some()
            {
                return Err(sample_preparation_error(
                    diagnostics::error_codes::KERNEL_SAMPLE_INVALID_REGION,
                    format!("duplicate prepared sample region '{qualified}'"),
                ));
            }
        }
    }
    let mut prepared_maps = Vec::with_capacity(assets.sample_maps.len());
    let mut map_indices = BTreeMap::new();
    for map in &assets.sample_maps {
        if map.selection_mode.as_deref().is_some_and(|mode| {
            !matches!(
                mode,
                "first_match" | "round_robin" | "random_weighted" | "round_robin_then_random"
            )
        }) {
            return Err(sample_preparation_error(
                diagnostics::error_codes::KERNEL_SAMPLE_UNSUPPORTED_MODE,
                format!("sample map '{}' has unsupported selection mode", map.id),
            ));
        }
        let mut zones: Vec<PreparedSampleZone> = Vec::with_capacity(map.zones.len());
        for zone in &map.zones {
            let Some(&(source_index, region_index)) = regions.get(&zone.region) else {
                return Err(sample_preparation_error(
                    diagnostics::error_codes::KERNEL_SAMPLE_INVALID_ZONE,
                    format!(
                        "sample map '{}' zone references unknown region '{}'",
                        map.id, zone.region
                    ),
                ));
            };
            if map.selection_mode.is_none()
                && zones.iter().any(|prior| {
                    zone.key_range[0] <= prior.declaration.key_range[1]
                        && prior.declaration.key_range[0] <= zone.key_range[1]
                        && zone.velocity_range[0] <= prior.declaration.velocity_range[1]
                        && prior.declaration.velocity_range[0] <= zone.velocity_range[1]
                })
            {
                return Err(sample_preparation_error(
                    diagnostics::error_codes::KERNEL_SAMPLE_INVALID_ZONE,
                    format!(
                        "sample map '{}' has overlapping zones without a selection mode",
                        map.id
                    ),
                ));
            }
            zones.push(PreparedSampleZone {
                declaration: zone.clone(),
                source_index,
                region_index,
            });
        }
        if map_indices
            .insert(map.id.clone(), prepared_maps.len())
            .is_some()
        {
            return Err(sample_preparation_error(
                diagnostics::error_codes::KERNEL_SAMPLE_INVALID_ZONE,
                format!("duplicate sample map id '{}'", map.id),
            ));
        }
        prepared_maps.push(PreparedSampleMap {
            declaration: map.clone(),
            zones,
        });
    }
    Ok(PreparedSampleAssets {
        sources,
        maps: assets.sample_maps.clone(),
        prepared_maps,
        map_indices,
    })
}

/// Validate, flatten, latency-balance, lower, and compile a kernel root using
/// the supplied definition registry.
pub fn prepare_kernel_graph(
    root: &GraphDefinition,
    registry: &DefinitionRegistry,
    render_settings: &RenderSettings,
) -> Result<PreparedKernelInstrument, KernelPreparationError> {
    let default_buses = default_kernel_output_buses(root);
    prepare_kernel_graph_with_buses(root, registry, render_settings, &default_buses)
}

fn default_kernel_output_buses(root: &GraphDefinition) -> HostBuses {
    HostBuses {
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
    }
}

pub fn prepare_kernel_graph_with_buses(
    root: &GraphDefinition,
    registry: &DefinitionRegistry,
    render_settings: &RenderSettings,
    host_buses: &HostBuses,
) -> Result<PreparedKernelInstrument, KernelPreparationError> {
    prepare_kernel_graph_with_buses_internal(
        root,
        registry,
        render_settings,
        host_buses,
        None,
        false,
        None,
    )
}

pub fn prepare_kernel_graph_with_buses_and_context(
    root: &GraphDefinition,
    registry: &DefinitionRegistry,
    render_settings: &RenderSettings,
    host_buses: &HostBuses,
    context: &PreparationContext,
) -> Result<PreparedKernelInstrument, KernelPreparationError> {
    prepare_kernel_graph_with_buses_internal(
        root,
        registry,
        render_settings,
        host_buses,
        Some(context),
        false,
        None,
    )
}

/// A planar host discovers event root outputs but cannot bind them to float
/// buffers. Bind those outputs internally after package resolution and flattening.
pub(crate) fn prepare_kernel_graph_for_planar_ffi(
    root: &GraphDefinition,
    registry: &DefinitionRegistry,
    render_settings: &RenderSettings,
    host_buses: &HostBuses,
    context: &PreparationContext,
) -> Result<PreparedKernelInstrument, KernelPreparationError> {
    prepare_kernel_graph_with_buses_internal(
        root,
        registry,
        render_settings,
        host_buses,
        Some(context),
        true,
        None,
    )
}

fn prepare_kernel_graph_with_buses_internal(
    root: &GraphDefinition,
    registry: &DefinitionRegistry,
    render_settings: &RenderSettings,
    host_buses: &HostBuses,
    context: Option<&PreparationContext>,
    bind_event_outputs: bool,
    authored_assets: Option<&SampleAssets>,
) -> Result<PreparedKernelInstrument, KernelPreparationError> {
    let (resolved_root, mut resolved_registry, package_assets) = match context {
        Some(context) => resolve_external_definitions(root, registry, context)?,
        None => (root.clone(), registry.clone(), SampleAssets::default()),
    };
    let mut declarations = authored_assets.cloned().unwrap_or_default();
    declarations
        .sample_sources
        .extend(package_assets.sample_sources);
    declarations.sample_maps.extend(package_assets.sample_maps);
    validate_sample_declarations(&declarations)?;
    let sample_assets = match context {
        Some(context) => prepare_sample_assets(&declarations, context)?,
        None => PreparedSampleAssets::default(),
    };
    let validation = resolved_root.validate(&resolved_registry);
    if !validation.is_ok() {
        return Err(validation.diagnostics().clone().into());
    }

    let mut flattened_graph = resolved_root
        .flatten(&resolved_registry)
        .map_err(KernelPreparationError::from)?;
    flattened_graph
        .expand_sample_map_poly_regions(&mut resolved_registry)
        .map_err(KernelPreparationError::from)?;
    let mut host_buses = host_buses.clone();
    if bind_event_outputs {
        for port in flattened_graph.root_ports() {
            if port.direction() == PortDirection::Output && port.signal_type() == SignalType::Event
            {
                host_buses
                    .outputs
                    .insert(port.name().to_string(), port.channels() as usize);
            }
        }
    }
    validate_host_buses(&flattened_graph, &host_buses)?;
    let latency_plan = balance_poly_latencies(
        &mut flattened_graph,
        &mut resolved_registry,
        &mut Vec::new(),
    )?;
    let mut compensation_metadata = Vec::new();
    append_compensation_metadata(&latency_plan, None, &mut compensation_metadata);
    let mut resource_resolver = context.map(ResourceResolver::new);
    let resources = match resource_resolver.as_mut() {
        Some(resolver) => resolve_flattened_resources(&flattened_graph, resolver)?,
        None => BTreeMap::new(),
    };
    let lowered = lower_kernel_graph(
        &flattened_graph,
        &latency_plan,
        &resources,
        &sample_assets,
        render_settings.sample_rate_hz,
    )?;
    lowered
        .graph
        .validate()
        .map_err(|error| KernelPreparationError::from(error.to_diagnostics()))?;
    let mut compiled_patch = compiled_patch::compile_with_node_data(
        &lowered.graph,
        render_settings,
        &lowered.node_data,
        &lowered.root_outputs,
    )
    .map_err(|error| {
        KernelPreparationError::from(diagnostics::Diagnostics::from(error.to_diagnostic()))
    })?;
    let root_input_spans = flattened_graph
        .root_ports()
        .iter()
        .filter(|port| port.direction() == PortDirection::Input)
        // An unbound control input uses its compiled default. Reserving a
        // silent host span here would turn that default into zero instead.
        .filter(|port| {
            port.signal_type() != SignalType::Control || host_buses.inputs.contains_key(port.name())
        })
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
        &host_buses,
        &root_input_spans,
        &lowered.root_outputs,
        &compiled_patch,
    ));
    let poly_regions = compile_poly_regions(
        &flattened_graph,
        &mut resolved_registry,
        render_settings,
        resource_resolver.as_mut(),
        &compiled_patch,
        &sample_assets,
        &mut compensation_metadata,
    )?;
    compiled_patch.set_poly_regions(poly_regions);

    Ok(PreparedKernelInstrument {
        sample_assets,
        flattened_graph,
        latency_plan,
        compensation_metadata,
        graph: lowered.graph,
        compiled_patch,
    })
}

fn resolve_external_definitions(
    root: &GraphDefinition,
    registry: &DefinitionRegistry,
    context: &PreparationContext,
) -> Result<(GraphDefinition, DefinitionRegistry, SampleAssets), KernelPreparationError> {
    let references = std::iter::once(root)
        .chain(registry.definitions())
        .flat_map(|definition| definition.nodes())
        .map(|node| node.definition_ref())
        .filter(|reference| crate::module_reference::is_external_reference(reference))
        .collect::<BTreeSet<_>>();
    if references.is_empty() {
        return Ok((root.clone(), registry.clone(), SampleAssets::default()));
    }

    let packages = references
        .into_iter()
        .map(|reference| {
            crate::module_package::load_referenced_kernel_package(reference, context).map_err(
                |error| {
                    KernelPreparationError::from(diagnostics::Diagnostics::from(
                        error.to_diagnostic(),
                    ))
                },
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Package parsers include the canonical builtins in their registry. Bind
    // caller overrides to fresh identities before importing package definitions,
    // while preserving every name authored by the caller or a package.
    let mut occupied = std::iter::once(root.name().to_string())
        .chain(
            registry
                .definitions()
                .map(|definition| definition.name().to_string()),
        )
        .chain(packages.iter().flat_map(|package| {
            package
                .registry()
                .definitions()
                .map(|definition| definition.name().to_string())
        }))
        .collect::<BTreeSet<_>>();
    let mut caller_names = BTreeMap::new();
    for builtin in crate::kernel::builtins::builtin_registry().definitions() {
        if registry
            .get(builtin.name())
            .is_some_and(|caller| caller != builtin)
        {
            let stem = format!("#caller::{}", builtin.name());
            let mut alias = stem.clone();
            let mut suffix = 1;
            while occupied.contains(&alias) {
                alias = format!("{stem}::{suffix}");
                suffix += 1;
            }
            occupied.insert(alias.clone());
            caller_names.insert(builtin.name().to_string(), alias);
        }
    }
    let resolved_root = root.with_scoped_definition_refs(root.name(), &caller_names);
    let mut resolved = DefinitionRegistry::new();
    for definition in registry.definitions() {
        let name = caller_names
            .get(definition.name())
            .map(String::as_str)
            .unwrap_or_else(|| definition.name());
        resolved =
            resolved.with_definition(definition.with_scoped_definition_refs(name, &caller_names));
    }
    let mut sample_assets = SampleAssets::default();
    for package in packages {
        sample_assets
            .sample_sources
            .extend(package.sample_assets().sample_sources.iter().cloned());
        sample_assets
            .sample_maps
            .extend(package.sample_assets().sample_maps.iter().cloned());
        for definition in package.registry().definitions() {
            resolved = resolved.with_definition(definition.clone());
        }
    }
    Ok((resolved_root, resolved, sample_assets))
}

/// Resolve nested voice latency before balancing the enclosing graph. A poly
/// node is structural at render time, but its child audio path contributes
/// real latency to every parent path and feedback cycle containing that node.
fn balance_poly_latencies(
    flattened: &mut FlattenedGraph,
    registry: &mut DefinitionRegistry,
    path: &mut Vec<String>,
) -> Result<LatencyPlan, KernelPreparationError> {
    for region in flattened.poly_regions().to_vec() {
        validate_poly_region_path(&region, path)?;
        let wrapped_name = region.wrapped_definition();
        path.push(wrapped_name.to_string());
        let wrapped = registry
            .get(wrapped_name)
            .expect("validated poly region references an existing definition")
            .clone();
        let mut voice_scope_diagnostics = diagnostics::Diagnostics::new();
        let scoped_voice = wrapped
            .with_voice_intrinsics(&mut voice_scope_diagnostics)
            .ok_or_else(|| KernelPreparationError::from(voice_scope_diagnostics.clone()))?;
        let mut child = scoped_voice
            .flatten(registry)
            .map_err(KernelPreparationError::from)?;
        child
            .expand_sample_map_poly_regions(registry)
            .map_err(KernelPreparationError::from)?;
        let child_plan = balance_poly_latencies(&mut child, registry, path)?;
        path.pop();
        flattened.set_poly_region_latency(region.node_id(), child_plan.root_latency());
    }
    flattened
        .balance_latency()
        .map_err(KernelPreparationError::from)
}

fn validate_poly_region_path(
    region: &FlattenedPolyRegion,
    path: &[String],
) -> Result<(), KernelPreparationError> {
    let code = if path.iter().any(|name| name == region.wrapped_definition()) {
        Some(diagnostics::error_codes::KERNEL_RECURSIVE_DEFINITION)
    } else if path.len() >= crate::kernel::flatten::MAX_FLATTEN_DEPTH {
        Some(diagnostics::error_codes::KERNEL_MAX_DEPTH_EXCEEDED)
    } else {
        None
    };
    if let Some(code) = code {
        return Err(KernelPreparationError::from(
            diagnostics::Diagnostics::from(
                Diagnostic::new(
                    code,
                    Severity::Error,
                    format!(
                        "poly region '{}' cannot expand wrapped definition '{}' after {}",
                        region.node_id().as_str(),
                        region.wrapped_definition(),
                        path.join(" -> ")
                    ),
                )
                .with_module_id(region.node_id().as_str()),
            ),
        ));
    }
    Ok(())
}

fn resolve_flattened_resources(
    flattened: &FlattenedGraph,
    resolver: &mut ResourceResolver<'_>,
) -> Result<BTreeMap<String, CompiledResourceHandles>, KernelPreparationError> {
    let mut by_node = BTreeMap::new();
    for node in flattened.nodes() {
        let mut handles = CompiledResourceHandles::default();
        for value in node.static_args().values() {
            let StaticValue::Resource(reference) = value else {
                continue;
            };
            let resolved = resolver
                .resolve(reference)
                .map_err(KernelPreparationError::from)?;
            match resolved.kind() {
                ResourceKind::Sample => {
                    handles.sample =
                        Some(SampleResourceHandle::from_shared(resolved.shared_sample()));
                }
                ResourceKind::ImpulseResponse => {
                    handles.impulse_response = Some(ImpulseResponseResourceHandle::from_shared(
                        resolved.shared_sample(),
                    ));
                }
            }
        }
        if handles != CompiledResourceHandles::default() {
            by_node.insert(node.id().as_str().to_string(), handles);
        }
    }
    Ok(by_node)
}

fn compile_poly_regions(
    flattened: &FlattenedGraph,
    registry: &mut DefinitionRegistry,
    render_settings: &RenderSettings,
    resource_resolver: Option<&mut ResourceResolver<'_>>,
    parent: &CompiledPatch,
    sample_assets: &PreparedSampleAssets,
    compensation_metadata: &mut Vec<PreparedCompensationMetadata>,
) -> Result<Vec<CompiledPolyRegion>, KernelPreparationError> {
    compile_poly_regions_with_path(
        flattened,
        registry,
        render_settings,
        resource_resolver,
        parent,
        sample_assets,
        &mut Vec::new(),
        "",
        compensation_metadata,
    )
}

fn compile_poly_regions_with_path(
    flattened: &FlattenedGraph,
    registry: &mut DefinitionRegistry,
    render_settings: &RenderSettings,
    mut resource_resolver: Option<&mut ResourceResolver<'_>>,
    parent: &CompiledPatch,
    sample_assets: &PreparedSampleAssets,
    path: &mut Vec<String>,
    location_prefix: &str,
    compensation_metadata: &mut Vec<PreparedCompensationMetadata>,
) -> Result<Vec<CompiledPolyRegion>, KernelPreparationError> {
    let mut compiled_regions = Vec::with_capacity(flattened.poly_regions().len());
    for region in flattened.poly_regions() {
        validate_poly_region_path(region, path)?;
        path.push(region.wrapped_definition().to_string());
        let wrapped = registry
            .get(region.wrapped_definition())
            .expect("validated poly region references an existing definition")
            .clone();
        let mut voice_scope_diagnostics = diagnostics::Diagnostics::new();
        let scoped_voice = wrapped
            .with_voice_intrinsics(&mut voice_scope_diagnostics)
            .ok_or_else(|| KernelPreparationError::from(voice_scope_diagnostics.clone()))?;
        let mut child_flattened = scoped_voice
            .flatten(registry)
            .map_err(KernelPreparationError::from)?;
        child_flattened
            .expand_sample_map_poly_regions(registry)
            .map_err(KernelPreparationError::from)?;
        let latency_plan = balance_poly_latencies(&mut child_flattened, registry, path)?;
        let region_path = format!("{location_prefix}{}", region.node_id().as_str());
        append_compensation_metadata(&latency_plan, Some(&region_path), compensation_metadata);
        let resources = match resource_resolver.as_deref_mut() {
            Some(resolver) => resolve_flattened_resources(&child_flattened, resolver)?,
            None => BTreeMap::new(),
        };
        let lowered = lower_kernel_graph(
            &child_flattened,
            &latency_plan,
            &resources,
            sample_assets,
            render_settings.sample_rate_hz,
        )?;
        lowered
            .graph
            .validate()
            .map_err(|error| KernelPreparationError::from(error.to_diagnostics()))?;
        let mut child_patch = compiled_patch::compile_with_node_data(
            &lowered.graph,
            render_settings,
            &lowered.node_data,
            &lowered.root_outputs,
        )
        .map_err(|error| {
            KernelPreparationError::from(diagnostics::Diagnostics::from(error.to_diagnostic()))
        })?;
        if let Some(done) = child_flattened
            .root_ports()
            .iter()
            .find(|port| port.name() == crate::kernel::POLY_DONE_OUTPUT)
        {
            let sources = child_flattened
                .root_output_sources()
                .get(crate::kernel::POLY_DONE_OUTPUT)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let valid_source = sources.len() == 1
                && child_patch.nodes().iter().any(|node| {
                    node.id.as_str() == sources[0].node().as_str()
                        && node
                            .output_port_names
                            .iter()
                            .zip(&node.output_port_types)
                            .any(|(name, signal_type)| {
                                name == sources[0].port() && *signal_type == done.signal_type()
                            })
                });
            if !valid_source {
                return Err(KernelPreparationError::from(
                    diagnostics::Diagnostics::from(
                        Diagnostic::new(
                            diagnostics::error_codes::KERNEL_POLY_MALFORMED_INTERFACE,
                            Severity::Error,
                            format!(
                                "poly region '{}' declares a 'done' output without one resolvable source",
                                region.node_id().as_str()
                            ),
                        )
                        .with_module_id(region.node_id().as_str())
                        .with_suggested_fix(
                            "map 'done' from one child event or control output",
                        ),
                    ),
                ));
            }
        }
        let child_input_spans = child_flattened
            .root_ports()
            .iter()
            .filter(|port| port.direction() == PortDirection::Input)
            .map(|port| {
                let span = child_patch.reserve_root_input_span(
                    port.channels() as usize,
                    child_flattened
                        .root_input_destinations()
                        .get(port.name())
                        .map(Vec::as_slice)
                        .unwrap_or_default(),
                );
                (port.name().to_string(), span)
            })
            .collect::<BTreeMap<_, _>>();
        let child_buses = HostBuses {
            inputs: child_flattened
                .root_ports()
                .iter()
                .filter(|port| port.direction() == PortDirection::Input)
                .map(|port| (port.name().to_string(), port.channels() as usize))
                .collect(),
            outputs: child_flattened
                .root_ports()
                .iter()
                .filter(|port| port.direction() == PortDirection::Output)
                .map(|port| (port.name().to_string(), port.channels() as usize))
                .collect(),
        };
        child_patch.set_root_bus_plan(root_bus_plan(
            &child_flattened,
            &child_buses,
            &child_input_spans,
            &lowered.root_outputs,
            &child_patch,
        ));
        let nested_regions = compile_poly_regions_with_path(
            &child_flattened,
            registry,
            render_settings,
            resource_resolver.as_deref_mut(),
            &child_patch,
            sample_assets,
            path,
            &format!("{region_path}::"),
            compensation_metadata,
        )?;
        path.pop();
        child_patch.set_poly_regions(nested_regions);

        let event_queue_capacity = crate::graph_processor::prepared_event_capacity(
            (render_settings.block_size_frames as usize).saturating_mul(2),
        );
        if let Some((child_id, module_type)) = crate::graph_processor::first_unrenderable_poly_child(
            &child_patch,
            render_settings.block_size_frames as usize,
            event_queue_capacity,
        ) {
            let module_id = if child_id.is_empty() {
                region.node_id().as_str().to_string()
            } else {
                format!(
                    "{}{}{}",
                    region.node_id().as_str(),
                    crate::kernel::NAMESPACE_SEPARATOR,
                    child_id
                )
            };
            let child_label = if child_id.is_empty() {
                "wrapped definition".to_string()
            } else {
                format!("child module '{child_id}'")
            };
            return Err(KernelPreparationError::from(
                diagnostics::Diagnostics::from(
                    Diagnostic::new(
                        diagnostics::error_codes::KERNEL_POLY_RUNTIME_UNSUPPORTED,
                        Severity::Error,
                        format!(
                            "poly region '{}' cannot render {} of type '{}'",
                            region.node_id().as_str(),
                            child_label,
                            module_type
                        ),
                    )
                    .with_module_id(module_id)
                    .with_suggested_fix(
                        "use a child module supported by the prepared poly runtime",
                    ),
                ),
            ));
        }

        let state_count = child_patch.nodes().len();
        let audio_buffer_count = child_patch.total_output_buffer_count()
            + child_patch
                .nodes()
                .iter()
                .flat_map(|node| node.input_port_spans.iter())
                .map(|span| span.channel_count)
                .sum::<usize>();
        let event_queue_count = child_patch
            .nodes()
            .iter()
            .flat_map(|node| {
                node.input_port_types
                    .iter()
                    .chain(node.output_port_types.iter())
            })
            .filter(|signal_type| **signal_type == SignalType::Event)
            .count()
            .max(1);
        let voices = (0..region.max_voices())
            .map(|voice| {
                CompiledPolyVoiceStorage::new(
                    voice * state_count..(voice + 1) * state_count,
                    voice * audio_buffer_count..(voice + 1) * audio_buffer_count,
                    voice * event_queue_count..(voice + 1) * event_queue_count,
                )
            })
            .collect();
        let boundary = parent
            .nodes()
            .iter()
            .find(|node| node.id.as_str() == region.node_id().as_str())
            .expect("compiled parent retains its poly boundary node");
        let output_accumulators = boundary
            .output_port_names
            .iter()
            .zip(boundary.output_port_types.iter())
            .zip(boundary.output_port_spans.iter())
            .filter_map(|((name, signal_type), span)| {
                (*signal_type != SignalType::Event)
                    .then(|| CompiledPolyOutputAccumulator::new(name, *signal_type, *span))
            })
            .collect();
        compiled_regions.push(CompiledPolyRegion::new(
            region.node_id().as_str(),
            region.max_voices(),
            region.allocation_policy(),
            region.sample_map_choke().map(|(mode, fade_ms)| {
                let mode = match mode {
                    "fade" => SampleChokeMode::Fade,
                    "release" => SampleChokeMode::Release,
                    _ => SampleChokeMode::Cut,
                };
                (mode, fade_ms)
            }),
            child_flattened,
            child_patch,
            voices,
            event_queue_capacity,
            output_accumulators,
        ));
    }
    Ok(compiled_regions)
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
    resources: &BTreeMap<String, CompiledResourceHandles>,
    sample_assets: &PreparedSampleAssets,
    host_sample_rate_hz: u32,
) -> Result<LoweredKernelGraph, KernelPreparationError> {
    use crate::graph::builtin_ports;

    let mut ids = flattened
        .nodes()
        .iter()
        .map(|node| node.id().as_str().to_string())
        .collect::<BTreeSet<_>>();
    let mut modules = Vec::new();
    let mut node_data = BTreeMap::new();
    for node in flattened.nodes() {
        let mut lowered = ModuleNode::new(ModuleId::new(node.id().as_str()), node.definition());
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
        data.resources = resources
            .get(node.id().as_str())
            .cloned()
            .unwrap_or_default();
        if kind == ModuleKind::SamplePlayer {
            let source_id = match node.static_args().get("source") {
                Some(StaticValue::String(value)) => value.as_str(),
                _ => "",
            };
            let region_id = match node.static_args().get("region") {
                Some(StaticValue::String(value)) => value.as_str(),
                _ => "",
            };
            let source = sample_assets
                .sources()
                .iter()
                .find(|source| source.id() == source_id)
                .ok_or_else(|| {
                    sample_preparation_error(
                        diagnostics::error_codes::KERNEL_SAMPLE_INVALID_REGION,
                        format!(
                            "sample_player '{}' references unknown source '{source_id}'",
                            node.id().as_str()
                        ),
                    )
                })?;
            let region = source
                .declaration()
                .regions
                .iter()
                .find(|region| region.id == region_id)
                .ok_or_else(|| {
                    sample_preparation_error(
                        diagnostics::error_codes::KERNEL_SAMPLE_INVALID_REGION,
                        format!(
                            "sample_player '{}' references unknown region '{region_id}'",
                            node.id().as_str()
                        ),
                    )
                })?;
            let mode = match node.static_args().get("mode") {
                Some(StaticValue::Enum(value)) => value.as_str(),
                _ => "one_shot",
            };
            if mode != "one_shot" && mode != "gated" && mode != "looped" {
                return Err(sample_preparation_error(
                    diagnostics::error_codes::KERNEL_SAMPLE_UNSUPPORTED_MODE,
                    format!(
                        "sample_player '{}' does not yet support mode '{mode}'",
                        node.id().as_str()
                    ),
                ));
            }
            if mode == "looped" && region.loop_settings.is_none() {
                return Err(sample_preparation_error(
                    diagnostics::error_codes::KERNEL_SAMPLE_INVALID_LOOP,
                    format!(
                        "sample_player '{}' requires loop points for looped mode",
                        node.id().as_str()
                    ),
                ));
            }
            data.resources.sample = Some(SampleResourceHandle::from_shared(
                source.resource.shared_sample(),
            ));
            let interpolation = match node.static_args().get("interpolation") {
                Some(StaticValue::Enum(value)) if value == "nearest" => {
                    SampleInterpolation::Nearest
                }
                Some(StaticValue::Enum(value)) if value == "cubic" => SampleInterpolation::Cubic,
                _ => SampleInterpolation::Linear,
            };
            data.construction = CompiledConstruction::SamplePlayer {
                region: region.clone(),
                mode: mode.to_string(),
                interpolation,
                playback_rate_scale: source.sample().sample_rate_hz() as f32
                    / host_sample_rate_hz as f32,
            };
        }
        if kind == ModuleKind::SampleSlicer {
            let source_id = match node.static_args().get("source") {
                Some(StaticValue::String(value)) => value.as_str(),
                _ => "",
            };
            let table_id = match node.static_args().get("slice_table") {
                Some(StaticValue::String(value)) => value.as_str(),
                _ => "",
            };
            let source = sample_assets
                .sources()
                .iter()
                .find(|source| source.id() == source_id)
                .ok_or_else(|| {
                    sample_preparation_error(
                        diagnostics::error_codes::KERNEL_SAMPLE_INVALID_SLICE,
                        format!(
                            "sample_slicer '{}' references unknown source '{source_id}'",
                            node.id().as_str()
                        ),
                    )
                })?;
            if table_id != source_id || source.declaration().slices.is_empty() {
                return Err(sample_preparation_error(
                    diagnostics::error_codes::KERNEL_SAMPLE_INVALID_SLICE,
                    format!(
                        "sample_slicer '{}' requires the nonempty slice table of source '{source_id}'",
                        node.id().as_str()
                    ),
                ));
            }
            data.resources.sample = Some(SampleResourceHandle::from_shared(
                source.resource.shared_sample(),
            ));
            data.construction = CompiledConstruction::SampleSlicer {
                slices: source.declaration().slices.clone().into_boxed_slice(),
                playback_rate_scale: source.sample().sample_rate_hz() as f32
                    / host_sample_rate_hz as f32,
            };
        }
        if kind == ModuleKind::SampleMapPlayer {
            let map_id = match node.static_args().get("sample_map") {
                Some(StaticValue::String(value)) => value.as_str(),
                _ => "",
            };
            let map = sample_assets.map_by_id(map_id).ok_or_else(|| {
                sample_preparation_error(
                    diagnostics::error_codes::KERNEL_SAMPLE_INVALID_ZONE,
                    format!(
                        "sample_map_player '{}' references unknown map '{map_id}'",
                        node.id().as_str()
                    ),
                )
            })?;
            if map.selection_mode().is_some_and(|mode| {
                mode != "first_match" && mode != "round_robin" && mode != "random_weighted"
            }) {
                return Err(sample_preparation_error(
                    diagnostics::error_codes::KERNEL_SAMPLE_UNSUPPORTED_MODE,
                    format!(
                        "sample_map_player '{}' does not yet support map selection mode",
                        node.id().as_str()
                    ),
                ));
            }
            let max_voices = match node.static_args().get("max_voices") {
                Some(StaticValue::Int(value)) => *value,
                _ => 16,
            };
            if max_voices != 1 {
                return Err(sample_preparation_error(
                    diagnostics::error_codes::KERNEL_SAMPLE_UNSUPPORTED_MODE,
                    format!(
                        "sample_map_player '{}' does not yet support multiple voices",
                        node.id().as_str()
                    ),
                ));
            }
            let mut groups = BTreeMap::new();
            let mut choke_groups = BTreeMap::new();
            let zones = map
                .zones()
                .iter()
                .map(|zone| {
                    let source = &sample_assets.sources()[zone.source_index()];
                    let round_robin_group =
                        zone.declaration().round_robin_group.as_ref().map(|name| {
                            if let Some(index) = groups.get(name) {
                                *index
                            } else {
                                let index = groups.len();
                                groups.insert(name.clone(), index);
                                index
                            }
                        });
                    let choke_group = zone.declaration().choke_group.as_ref().map(|name| {
                        if let Some(index) = choke_groups.get(name) {
                            *index
                        } else {
                            let index = choke_groups.len();
                            choke_groups.insert(name.clone(), index);
                            index
                        }
                    });
                    CompiledSampleZone {
                        sample: SampleResourceHandle::from_shared(source.resource.shared_sample()),
                        region: source.declaration().regions[zone.region_index()].clone(),
                        key_range: zone.declaration().key_range,
                        velocity_range: zone.declaration().velocity_range,
                        round_robin_group,
                        choke_group,
                        weight: zone.declaration().weight.unwrap_or(1),
                        gain: 10.0_f64.powf(
                            (source.declaration().regions[zone.region_index()]
                                .gain_db
                                .unwrap_or(0.0)
                                + zone.declaration().gain_db.unwrap_or(0.0))
                                / 20.0,
                        ) as f32,
                        pan: (source.declaration().regions[zone.region_index()]
                            .pan
                            .unwrap_or(0.0)
                            + zone.declaration().pan.unwrap_or(0.0))
                        .clamp(-1.0, 1.0) as f32,
                        pitch_ratio: 2.0_f64
                            .powf(zone.declaration().pitch_semitones.unwrap_or(0.0) / 12.0)
                            as f32,
                        playback_rate_scale: source.sample().sample_rate_hz() as f32
                            / host_sample_rate_hz as f32,
                    }
                })
                .collect::<Vec<_>>()
                .into_boxed_slice();
            let selection_mode = match map.selection_mode() {
                Some("round_robin") => SampleSelectionMode::RoundRobin,
                Some("random_weighted") => SampleSelectionMode::RandomWeighted,
                _ => SampleSelectionMode::FirstMatch,
            };
            data.construction = CompiledConstruction::SampleMapPlayer {
                zones,
                selection_mode,
                group_count: groups.len(),
                selection_seed: map.selection_seed(),
                reject_new_while_active: matches!(
                    node.static_args().get("voice_steal"),
                    Some(StaticValue::Enum(value)) if value == "reject_new"
                ),
            };
        }
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
                .or_insert(0.0);
        }
        node_data.insert(node.id().as_str().to_string(), data);
        modules.push(lowered);
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
            modules.push(compensation_delay_node(&id));
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
            modules.push(compensation_delay_node(&id));
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

fn compensation_delay_node(id: &str) -> ModuleNode {
    ModuleNode::new(
        ModuleId::new(id),
        crate::builtins::module_types::COMPENSATION_DELAY,
    )
    .with_input(crate::graph::builtin_ports::AUDIO_IN, SignalType::Audio)
    .with_output(crate::graph::builtin_ports::AUDIO_OUT, SignalType::Audio)
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

#[cfg(test)]
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
#[cfg(test)]
pub(crate) fn prepare_instrument_document_with_preset(
    patch_doc: PatchDocument,
    preset_doc: &PresetDocument,
    base_dir: impl AsRef<Path>,
) -> Result<PreparedInstrument, PreparationError> {
    let patched_doc =
        patch::apply_preset(&patch_doc, preset_doc).map_err(PreparationError::Schema)?;
    prepare_instrument_document(patched_doc, base_dir)
}

#[cfg(test)]
pub(crate) fn load_patch_document(
    path: impl AsRef<Path>,
) -> Result<PatchDocument, PreparationError> {
    patch::load_patch_file(path).map_err(PreparationError::Load)
}

#[cfg(test)]
pub(crate) fn validate_patch_document(patch_doc: &PatchDocument) -> Result<(), PreparationError> {
    patch::validate_patch_schema(patch_doc).map_err(PreparationError::Schema)
}

#[cfg(test)]
pub(crate) fn resolve_patch_parameters(
    patch_doc: &PatchDocument,
) -> Result<BTreeMap<String, BTreeMap<String, ParameterValue>>, PreparationError> {
    patch::resolve_module_parameters(patch_doc).map_err(PreparationError::Schema)
}

#[allow(dead_code)]
#[cfg(test)]
pub(crate) fn build_validated_graph(patch_doc: &PatchDocument) -> Result<Graph, PreparationError> {
    let resolved_parameters = resolve_patch_parameters(patch_doc)?;
    build_validated_graph_with_resolved_parameters(patch_doc, &resolved_parameters)
}

#[cfg(test)]
fn build_validated_graph_with_resolved_parameters(
    patch_doc: &PatchDocument,
    resolved_parameters: &BTreeMap<String, BTreeMap<String, ParameterValue>>,
) -> Result<Graph, PreparationError> {
    let resolved_patch = patch_document_with_resolved_parameters(patch_doc, resolved_parameters);
    let graph = Graph::from_patch_declarations(&resolved_patch);
    graph.validate().map_err(PreparationError::Graph)?;
    Ok(graph)
}

#[cfg(test)]
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

#[cfg(test)]
pub(crate) fn prepare_assets(
    patch_doc: &PatchDocument,
    base_dir: impl AsRef<Path>,
) -> Result<PreparedSamplerAssets, PreparationError> {
    sample::prepare_sampler_assets(patch_doc, base_dir).map_err(PreparationError::Assets)
}

#[cfg(test)]
pub(crate) fn compile_patch(
    graph: &Graph,
    patch_doc: &PatchDocument,
) -> Result<CompiledPatch, PreparationError> {
    compiled_patch::compile(graph, &patch_doc.render).map_err(PreparationError::Compile)
}

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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
        DELAY_SAMPLES_PARAMETER, SCRIPT_LANGUAGE_PARAMETER, SCRIPT_LANGUAGE_RHAI,
        SCRIPT_SOURCE_PARAMETER, SPECTRAL_FFT_SIZE_PARAMETER, SPECTRAL_MODE_PARAMETER,
        SPECTRAL_MODE_PASSTHROUGH, module_types,
    };
    use crate::compiled_patch::{CompiledConstruction, CompiledScriptLanguage};
    use crate::convolution::Convolution;
    use crate::core::TimedInputEvent;
    use crate::graph::{SignalType, builtin_ports};
    use crate::graph_processor::{RealtimeGraphProcessor, render_kernel_offline_named};
    use crate::kernel::builtins::{IMPULSE_RESPONSE_RESOURCE_PARAM, builtin_registry};
    use crate::kernel::document::load_kernel_patch_str;
    use crate::kernel::{
        Connection, DefinitionRegistry, GraphDefinition, Node, NodeId, Port as KernelPort,
        PortRef as KernelPortRef, ResourceKind, ResourceOrigin, ResourceRef, StaticArg,
        StaticValue,
    };
    use crate::module_reference::{LIB_MACRO, MacroRoots};
    use crate::patch;
    use crate::sample::LoadedSample;
    use crate::script::ScriptEvent;
    use crate::test_allocator::count_current_thread_allocations;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    const WET_NODE_ID: &str = "wet";
    const UNIT_IR_PATH: &str = "unit-ir.wav";

    fn poly_node_with_allocation(
        id: &str,
        definition: &str,
        max_voices: i64,
        allocation: &str,
    ) -> Node {
        Node::new(NodeId::new(id), crate::kernel::POLY_DEFINITION)
            .with_static_arg(
                crate::kernel::POLY_WRAPPED_DEFINITION_PARAM,
                StaticArg::Literal(StaticValue::String(definition.to_string())),
            )
            .with_static_arg(
                crate::kernel::POLY_MAX_VOICES_PARAM,
                StaticArg::Literal(StaticValue::Int(max_voices)),
            )
            .with_static_arg(
                crate::kernel::POLY_ALLOCATION_PARAM,
                StaticArg::Literal(StaticValue::Enum(allocation.to_string())),
            )
    }

    fn poly_node(id: &str, definition: &str, max_voices: i64) -> Node {
        poly_node_with_allocation(
            id,
            definition,
            max_voices,
            crate::kernel::POLY_ALLOCATION_REJECT_NEW,
        )
    }

    #[test]
    fn prepared_discovery_reuses_root_port_metadata_and_lists_promotion_nodes() {
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::input("level", SignalType::Control, 1)
                    .with_control_default(crate::kernel::ControlDefault::new(0.5))
                    .maps_to(kernel_ref("gain", builtin_ports::GAIN)),
            )
            .with_port(
                KernelPort::output("master", SignalType::Audio, 1)
                    .maps_from(kernel_ref("gain", builtin_ports::AUDIO_OUT)),
            )
            .with_node(Node::new(NodeId::new("source"), module_types::CURVE_MAPPER))
            .with_node(Node::new(NodeId::new("gain"), module_types::GAIN))
            .with_connection(Connection::new(
                kernel_ref("source", builtin_ports::VALUE),
                kernel_ref("gain", builtin_ports::AUDIO_IN),
            ));
        let discovered = root.metadata();
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry(),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new().with_output("master", 1),
        )
        .expect("root with promoted connection prepares");

        assert_eq!(discovered.ports().len(), 2);
        let prepared_ports = prepared.root_port_metadata();
        assert_eq!(prepared_ports, discovered.ports());
        let promotion = prepared
            .node_metadata()
            .into_iter()
            .find(|node| node.definition() == crate::kernel::CONTROL_TO_AUDIO_DEFINITION)
            .expect("generated promotion is discoverable");
        assert!(
            promotion
                .id()
                .contains(crate::kernel::PROMOTION_NODE_PREFIX)
        );
        assert_eq!(
            promotion.ports()[0].name(),
            crate::kernel::PROMOTION_INPUT_PORT
        );
        assert_eq!(
            promotion.ports()[0].channels(),
            &crate::kernel::ChannelCount::Literal(1)
        );
        assert_eq!(
            promotion.ports()[1].name(),
            crate::kernel::PROMOTION_OUTPUT_PORT
        );
        assert_eq!(promotion.ports()[1].signal_type(), SignalType::Audio);
    }

    #[test]
    fn prepared_root_discovery_resolves_static_channel_references() {
        let root = GraphDefinition::new("root")
            .with_static_param(
                crate::kernel::StaticParam::new("channels", crate::kernel::StaticType::Int)
                    .with_default(StaticValue::Int(2)),
            )
            .with_port(
                KernelPort::output(
                    "master",
                    SignalType::Audio,
                    crate::kernel::ChannelCount::param("channels"),
                )
                .maps_from(kernel_ref("gain", builtin_ports::AUDIO_OUT)),
            )
            .with_node(
                Node::new(NodeId::new("gain"), module_types::GAIN).with_static_arg(
                    crate::kernel::builtins::CHANNELS_PARAM,
                    StaticArg::ParamRef("channels".into()),
                ),
            );
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry(),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new().with_output("master", 2),
        )
        .expect("static root channel count resolves");

        assert_eq!(
            root.metadata().ports()[0].channels(),
            &crate::kernel::ChannelCount::param("channels")
        );
        assert_eq!(
            prepared.root_port_metadata()[0].channels(),
            &crate::kernel::ChannelCount::Literal(2)
        );
        assert_eq!(
            prepared
                .node_metadata()
                .into_iter()
                .find(|node| node.id() == "gain")
                .unwrap()
                .ports()
                .iter()
                .find(|port| port.name() == builtin_ports::AUDIO_OUT)
                .unwrap()
                .channels(),
            &crate::kernel::ChannelCount::Literal(2)
        );
    }

    #[test]
    fn prepared_discovery_includes_promotions_inside_nested_poly_voices() {
        let inner = GraphDefinition::new("promoted_inner")
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("gain", builtin_ports::AUDIO_OUT)),
            )
            .with_node(Node::new(NodeId::new("source"), module_types::CURVE_MAPPER))
            .with_node(Node::new(NodeId::new("gain"), module_types::GAIN))
            .with_connection(Connection::new(
                kernel_ref("source", builtin_ports::VALUE),
                kernel_ref("gain", builtin_ports::AUDIO_IN),
            ));
        let outer = GraphDefinition::new("outer")
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("inner_voices", "audio")),
            )
            .with_node(poly_node("inner_voices", inner.name(), 1));
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("master", SignalType::Audio, 1)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_node(poly_node("voices", outer.name(), 1));
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry()
                .with_definition(inner)
                .with_definition(outer),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new().with_output("master", 1),
        )
        .expect("nested promotion prepares");

        let nodes = prepared.node_metadata();
        let promotions = nodes
            .iter()
            .filter(|node| node.definition() == crate::kernel::CONTROL_TO_AUDIO_DEFINITION)
            .collect::<Vec<_>>();
        assert_eq!(promotions.len(), 1);
        assert!(promotions[0].id().starts_with("voices::inner_voices::"));
        assert_eq!(promotions[0].ports()[0].signal_type(), SignalType::Control);
        assert_eq!(promotions[0].ports()[1].signal_type(), SignalType::Audio);
    }

    fn gain_voice(name: &str) -> GraphDefinition {
        GraphDefinition::new(name)
            .with_port(
                KernelPort::input("input", SignalType::Audio, 2)
                    .maps_to(kernel_ref("gain", builtin_ports::AUDIO_IN)),
            )
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 2)
                    .maps_from(kernel_ref("gain", builtin_ports::AUDIO_OUT)),
            )
            .with_node(
                Node::new(NodeId::new("gain"), module_types::GAIN).with_static_arg(
                    crate::kernel::builtins::CHANNELS_PARAM,
                    StaticArg::Literal(StaticValue::Int(2)),
                ),
            )
    }

    fn poly_root(wrapped_definition: &str, max_voices: i64) -> GraphDefinition {
        GraphDefinition::new("root")
            .with_port(
                KernelPort::input("input", SignalType::Audio, 2)
                    .maps_to(kernel_ref("voices", "input")),
            )
            .with_port(
                KernelPort::output("master", SignalType::Audio, 2)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_node(poly_node("voices", wrapped_definition, max_voices))
    }

    fn intrinsic_voice() -> GraphDefinition {
        GraphDefinition::new("intrinsic_voice")
            .with_port(
                KernelPort::output("pitch", SignalType::Control, 1)
                    .maps_from(kernel_ref(crate::kernel::VOICE_INTRINSIC_NODE, "note")),
            )
            .with_port(
                KernelPort::output("velocity", SignalType::Control, 1)
                    .maps_from(kernel_ref(crate::kernel::VOICE_INTRINSIC_NODE, "velocity")),
            )
            .with_node(Node::new(NodeId::new("envelope"), module_types::ADSR))
            .with_connection(Connection::new(
                kernel_ref(crate::kernel::VOICE_INTRINSIC_NODE, "gate"),
                kernel_ref("envelope", builtin_ports::GATE),
            ))
    }

    fn oscillator_voice(name: &str) -> GraphDefinition {
        GraphDefinition::new(name)
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("oscillator", builtin_ports::AUDIO)),
            )
            .with_node(Node::new(
                NodeId::new("oscillator"),
                module_types::OSCILLATOR,
            ))
            .with_connection(Connection::new(
                kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_NOTE_OUTPUT,
                ),
                kernel_ref("oscillator", builtin_ports::PITCH),
            ))
    }

    fn noise_voice(name: &str, channels: i64, seed: i64) -> GraphDefinition {
        GraphDefinition::new(name)
            .with_port(
                KernelPort::output("audio", SignalType::Audio, channels as u32)
                    .maps_from(kernel_ref("noise", builtin_ports::AUDIO)),
            )
            .with_node(
                Node::new(NodeId::new("noise"), module_types::NOISE)
                    .with_static_arg(
                        crate::kernel::builtins::CHANNELS_PARAM,
                        StaticArg::Literal(StaticValue::Int(channels)),
                    )
                    .with_static_arg(
                        crate::builtins::NOISE_SEED_PARAMETER,
                        StaticArg::Literal(StaticValue::Int(seed)),
                    ),
            )
    }

    fn constant_voice(name: &str, sample: f64) -> GraphDefinition {
        GraphDefinition::new(name)
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("constant", builtin_ports::OUT)),
            )
            .with_node(
                Node::new(NodeId::new("constant"), module_types::CONTROL_TO_AUDIO)
                    .with_default_override(builtin_ports::IN, sample),
            )
    }

    fn prepare_audio_poly(voice: GraphDefinition, max_voices: i64) -> PreparedKernelInstrument {
        let voice_name = voice.name().to_string();
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("left", SignalType::Audio, 1)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_port(
                KernelPort::output("right", SignalType::Audio, 1)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_node(poly_node_with_allocation(
                "voices",
                &voice_name,
                max_voices,
                crate::kernel::POLY_ALLOCATION_REJECT_NEW,
            ));
        prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry().with_definition(voice),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new()
                .with_output("left", 1)
                .with_output("right", 1),
        )
        .expect("audio poly graph prepares")
    }

    #[test]
    fn stolen_kernel_poly_filter_voice_matches_a_fresh_onset() {
        let voice = GraphDefinition::new("filtered_noise_voice")
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("filter", builtin_ports::AUDIO_OUT)),
            )
            .with_node(
                Node::new(NodeId::new("noise"), module_types::NOISE).with_static_arg(
                    crate::builtins::NOISE_SEED_PARAMETER,
                    StaticArg::Literal(StaticValue::Int(1234)),
                ),
            )
            .with_node(
                Node::new(NodeId::new("filter"), module_types::FILTER)
                    .with_default_override(builtin_ports::CUTOFF, 0.75)
                    .with_default_override(builtin_ports::RESONANCE, 0.9),
            )
            .with_connection(Connection::new(
                kernel_ref("noise", builtin_ports::AUDIO),
                kernel_ref("filter", builtin_ports::AUDIO_IN),
            ));
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("left", SignalType::Audio, 1)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_port(
                KernelPort::output("right", SignalType::Audio, 1)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_node(poly_node_with_allocation(
                "voices",
                voice.name(),
                1,
                crate::kernel::POLY_ALLOCATION_OLDEST_STEAL,
            ));
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry().with_definition(voice),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new()
                .with_output("left", 1)
                .with_output("right", 1),
        )
        .expect("filtered poly voice prepares");
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let render = |runtime: &mut RealtimeGraphProcessor| {
            let mut outputs = vec![vec![vec![0.0; frames]]; 2];
            assert_eq!(runtime.render_root_outputs(&mut outputs), frames);
            outputs[0][0].clone()
        };
        let mut reused = runtime_for(&prepared);
        reused.note_on(60, 100);
        let first = render(&mut reused);
        assert!(first.iter().any(|sample| sample.abs() > 0.001));
        reused.note_on(61, 100);
        let retriggered = render(&mut reused);
        let mut fresh = runtime_for(&prepared);
        fresh.note_on(61, 100);
        assert_eq!(retriggered, render(&mut fresh));
    }

    #[test]
    fn stolen_kernel_poly_oscillator_continues_phase_until_engine_reset() {
        let voice = GraphDefinition::new("free_running_voice")
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("oscillator", builtin_ports::AUDIO)),
            )
            .with_node(Node::new(
                NodeId::new("oscillator"),
                module_types::OSCILLATOR,
            ));
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("left", SignalType::Audio, 1)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_node(poly_node_with_allocation(
                "voices",
                voice.name(),
                1,
                crate::kernel::POLY_ALLOCATION_OLDEST_STEAL,
            ));
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry().with_definition(voice),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new().with_output("left", 1),
        )
        .expect("free-running oscillator voice prepares");
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let render = |runtime: &mut RealtimeGraphProcessor| {
            let mut outputs = vec![vec![vec![0.0; frames]]];
            assert_eq!(runtime.render_root_outputs(&mut outputs), frames);
            outputs[0][0].clone()
        };

        let mut runtime = runtime_for(&prepared);
        runtime.note_on(60, 100);
        let first = render(&mut runtime);
        assert_eq!(
            first[0], -1.0,
            "the default saw starts at negative full scale"
        );
        let expected_continued_onset = -1.0 + 2.0 * 8.0 * 220.0 / 48_000.0;

        runtime.note_on(61, 100);
        let continued = render(&mut runtime);
        assert!(
            (continued[0] - expected_continued_onset).abs() < 0.000001,
            "a stolen oscillator must keep its phase: {continued:?}"
        );

        runtime.reset();
        runtime.note_on(62, 100);
        assert_eq!(render(&mut runtime), first);
    }

    #[test]
    fn engine_reset_retires_kernel_poly_voices_and_allows_a_new_note() {
        let prepared = prepare_audio_poly(constant_voice("constant_voice", 0.25), 1);
        let mut runtime = runtime_for(&prepared);
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let render = |runtime: &mut RealtimeGraphProcessor| {
            let mut outputs = vec![vec![vec![0.0; frames]]; 2];
            assert_eq!(runtime.render_root_outputs(&mut outputs), frames);
            outputs[0][0].clone()
        };

        runtime.note_on(60, 100);
        assert_eq!(render(&mut runtime), vec![0.25; frames]);
        runtime.reset();
        assert_eq!(render(&mut runtime), vec![0.0; frames]);
        runtime.note_on(61, 100);
        assert_eq!(render(&mut runtime), vec![0.25; frames]);
    }

    fn intrinsic_poly_root(allocation: &str, max_voices: i64) -> GraphDefinition {
        GraphDefinition::new("root")
            .with_port(
                KernelPort::output("pitch", SignalType::Control, 1)
                    .maps_from(kernel_ref("voices", "pitch")),
            )
            .with_port(
                KernelPort::output("velocity", SignalType::Control, 1)
                    .maps_from(kernel_ref("voices", "velocity")),
            )
            .with_node(poly_node_with_allocation(
                "voices",
                "intrinsic_voice",
                max_voices,
                allocation,
            ))
    }

    fn prepare_intrinsic_poly(allocation: &str, max_voices: i64) -> PreparedKernelInstrument {
        prepare_kernel_graph_with_buses(
            &intrinsic_poly_root(allocation, max_voices),
            &builtin_registry().with_definition(intrinsic_voice()),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new()
                .with_output("pitch", 1)
                .with_output("velocity", 1),
        )
        .expect("poly voice intrinsics prepare")
    }

    fn runtime_for(prepared: &PreparedKernelInstrument) -> RealtimeGraphProcessor {
        RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
            prepared.graph().clone(),
            prepared.compiled_patch().clone(),
            KERNEL_RENDER_SETTINGS.sample_rate_hz as f32,
            &PreparedSamplerAssets::empty(),
            &crate::patch::VoiceAllocation::default(),
            KERNEL_RENDER_SETTINGS.block_size_frames as usize,
        )
    }

    fn render_one_block(runtime: &mut RealtimeGraphProcessor) {
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let mut left = vec![0.0; frames];
        let mut right = vec![0.0; frames];
        assert_eq!(
            render_two_mono_root_ports(runtime, &mut left, &mut right),
            frames
        );
    }

    fn render_two_mono_root_ports(
        runtime: &mut RealtimeGraphProcessor,
        left: &mut [f32],
        right: &mut [f32],
    ) -> usize {
        assert_eq!(left.len(), right.len());
        let mut outputs = vec![vec![vec![0.0; left.len()]]; 2];
        let frames = runtime.render_root_outputs(&mut outputs);
        left[..frames].copy_from_slice(&outputs[0][0][..frames]);
        right[..frames].copy_from_slice(&outputs[1][0][..frames]);
        frames
    }

    #[test]
    fn preparation_compiles_exactly_max_voices_of_disjoint_poly_storage() {
        let registry = builtin_registry().with_definition(gain_voice("voice"));
        let root = poly_root("voice", 3);
        let buses = HostBuses::new()
            .with_input("input", 2)
            .with_output("master", 2);

        let prepared =
            prepare_kernel_graph_with_buses(&root, &registry, &KERNEL_RENDER_SETTINGS, &buses)
                .expect("one non-nested poly region prepares");
        let repeated =
            prepare_kernel_graph_with_buses(&root, &registry, &KERNEL_RENDER_SETTINGS, &buses)
                .expect("repeated preparation succeeds");

        assert_eq!(prepared.compiled_patch(), repeated.compiled_patch());
        assert_eq!(prepared.compiled_patch().poly_regions().len(), 1);
        let region = &prepared.compiled_patch().poly_regions()[0];
        assert_eq!(region.node_id(), "voices");
        assert_eq!(region.max_voices(), 3);
        assert_eq!(
            region.allocation_policy(),
            crate::kernel::PolyAllocationPolicy::RejectNew
        );
        assert_eq!(region.flattened_voice().nodes().len(), 2);
        assert_eq!(region.child_schedule(), &[0, 1]);
        assert_eq!(region.voices().len(), 3);
        assert_eq!(region.voices()[0].state_range(), 0..2);
        assert_eq!(region.voices()[1].state_range(), 2..4);
        assert_eq!(region.voices()[2].state_range(), 4..6);
        assert_eq!(region.voices()[0].audio_buffer_range(), 0..9);
        assert_eq!(region.voices()[1].audio_buffer_range(), 9..18);
        assert_eq!(region.voices()[2].audio_buffer_range(), 18..27);
        assert_eq!(region.voices()[0].event_queue_range(), 0..1);
        assert_eq!(region.voices()[1].event_queue_range(), 1..2);
        assert_eq!(region.voices()[2].event_queue_range(), 2..3);
        assert_eq!(
            region.event_queue_capacity(),
            crate::graph_processor::prepared_event_capacity(
                (KERNEL_RENDER_SETTINGS.block_size_frames as usize) * 2
            )
        );
        assert_eq!(region.output_accumulators().len(), 1);
        assert_eq!(region.output_accumulators()[0].name(), "audio");
        assert_eq!(region.output_accumulators()[0].span().channel_count, 2);
        let boundary = prepared
            .compiled_patch()
            .nodes()
            .iter()
            .find(|node| node.id.as_str() == "voices")
            .expect("parent retains structural poly node");
        assert_eq!(boundary.module_kind, ModuleKind::Poly);
        assert_eq!(boundary.input_port_names, ["notes", "input"]);
        assert_eq!(boundary.input_port_spans[1].channel_count, 2);
        assert_eq!(boundary.output_port_names, ["audio"]);
        assert_eq!(boundary.output_port_spans[0].channel_count, 2);
        assert!(prepared.compiled_patch().voice_node_indices().is_empty());
    }

    #[test]
    fn preparation_injects_typed_voice_intrinsics_into_the_wrapped_definition() {
        let prepared = prepare_intrinsic_poly(crate::kernel::POLY_ALLOCATION_REJECT_NEW, 2);
        let voice = prepared.compiled_patch().poly_regions()[0].flattened_voice();
        let intrinsic = voice
            .node(&NodeId::new(crate::kernel::VOICE_INTRINSIC_NODE))
            .expect("compiler injects the intrinsic source node");

        assert_eq!(
            intrinsic.definition(),
            crate::kernel::VOICE_INTRINSIC_DEFINITION
        );
        assert_eq!(
            intrinsic
                .ports()
                .iter()
                .map(|port| (port.name(), port.signal_type(), port.direction()))
                .collect::<Vec<_>>(),
            [
                ("note", SignalType::Control, PortDirection::Output),
                ("velocity", SignalType::Control, PortDirection::Output),
                ("gate", SignalType::Event, PortDirection::Output),
            ]
        );
        assert!(voice.connections().iter().any(|connection| {
            connection.source().node().as_str() == crate::kernel::VOICE_INTRINSIC_NODE
                && connection.source().port() == "gate"
                && connection.destination().node().as_str() == "envelope"
                && connection.destination().port() == builtin_ports::GATE
        }));
    }

    #[test]
    fn poly_note_ons_fill_distinct_free_voice_intrinsics() {
        let prepared = prepare_intrinsic_poly(crate::kernel::POLY_ALLOCATION_REJECT_NEW, 2);
        let mut runtime = runtime_for(&prepared);

        runtime.note_on_at(60, 64, 3);
        runtime.note_on_at(67, 127, 7);
        render_one_block(&mut runtime);

        let region = &runtime.prepared_poly_runtime_regions()[0];
        assert_eq!(region.active_voice_count(), 2);
        assert_eq!(region.voice_note(0), Some(60));
        assert_eq!(region.voice_note(1), Some(67));
        assert_eq!(region.voice_velocity(0), Some(64));
        assert_eq!(region.voice_velocity(1), Some(127));
        assert_eq!(region.voice_note_control(0), Some(1.0));
        assert_eq!(region.voice_velocity_control(1), Some(1.0));
    }

    #[test]
    fn poly_oldest_steal_retires_and_reuses_the_longest_active_voice() {
        let prepared = prepare_intrinsic_poly(crate::kernel::POLY_ALLOCATION_OLDEST_STEAL, 2);
        let mut runtime = runtime_for(&prepared);

        runtime.note_on(60, 100);
        render_one_block(&mut runtime);
        runtime.note_on(64, 100);
        render_one_block(&mut runtime);
        runtime.note_on_at(67, 90, 3);
        render_one_block(&mut runtime);

        let region = &runtime.prepared_poly_runtime_regions()[0];
        assert_eq!(region.active_voice_count(), 2);
        assert_eq!(region.voice_note(0), Some(67));
        assert_eq!(region.voice_velocity(0), Some(90));
        assert_eq!(region.voice_note(1), Some(64));
    }

    #[test]
    fn poly_reject_new_keeps_existing_voices_unchanged_at_capacity() {
        let prepared = prepare_intrinsic_poly(crate::kernel::POLY_ALLOCATION_REJECT_NEW, 2);
        let mut runtime = runtime_for(&prepared);

        for note in [60, 64, 67] {
            runtime.note_on(note, 100);
            render_one_block(&mut runtime);
        }

        let region = &runtime.prepared_poly_runtime_regions()[0];
        assert_eq!(region.active_voice_count(), 2);
        assert_eq!(region.voice_note(0), Some(60));
        assert_eq!(region.voice_note(1), Some(64));
        assert!(region.voice_gate_events(0).is_empty());
        assert!(region.voice_gate_events(1).is_empty());
    }

    #[test]
    fn poly_note_off_releases_only_matching_voice_gate() {
        let prepared = prepare_intrinsic_poly(crate::kernel::POLY_ALLOCATION_REJECT_NEW, 2);
        let mut runtime = runtime_for(&prepared);

        runtime.note_on(60, 100);
        runtime.note_on(64, 100);
        render_one_block(&mut runtime);
        runtime.note_off_at(60, 3);
        render_one_block(&mut runtime);

        let region = &runtime.prepared_poly_runtime_regions()[0];
        assert_eq!(region.voice_gate_held(0), Some(false));
        assert_eq!(region.voice_gate_held(1), Some(true));
    }

    #[test]
    fn poly_oldest_steal_gate_queue_covers_the_worst_case_input_block() {
        let prepared = prepare_intrinsic_poly(crate::kernel::POLY_ALLOCATION_OLDEST_STEAL, 1);
        let mut runtime = runtime_for(&prepared);
        let block_events = KERNEL_RENDER_SETTINGS.block_size_frames as u8;

        for note in 0..block_events {
            runtime.note_on(note, 100);
        }
        render_one_block(&mut runtime);

        let region = &runtime.prepared_poly_runtime_regions()[0];
        assert_eq!(
            region.event_queue_capacity(),
            crate::graph_processor::prepared_event_capacity(usize::from(block_events) * 2)
        );
        assert_eq!(
            region.voice_gate_events(0).len(),
            usize::from(block_events) * 2 - 1
        );
    }

    #[test]
    fn sibling_poly_regions_allocate_independently() {
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("first_pitch", SignalType::Control, 1)
                    .maps_from(kernel_ref("first", "pitch")),
            )
            .with_port(
                KernelPort::output("second_pitch", SignalType::Control, 1)
                    .maps_from(kernel_ref("second", "pitch")),
            )
            .with_node(poly_node_with_allocation(
                "first",
                "intrinsic_voice",
                1,
                crate::kernel::POLY_ALLOCATION_OLDEST_STEAL,
            ))
            .with_node(poly_node_with_allocation(
                "second",
                "intrinsic_voice",
                1,
                crate::kernel::POLY_ALLOCATION_REJECT_NEW,
            ));
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry().with_definition(intrinsic_voice()),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new()
                .with_output("first_pitch", 1)
                .with_output("second_pitch", 1),
        )
        .expect("sibling poly regions prepare");
        let mut runtime = runtime_for(&prepared);

        runtime.route_poly_note_event_for_test(
            "first",
            ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
            0,
        );
        runtime.route_poly_note_event_for_test(
            "second",
            ScriptEvent::NoteOn {
                note: 72,
                velocity: 80,
            },
            0,
        );
        runtime.route_poly_note_event_for_test(
            "first",
            ScriptEvent::NoteOn {
                note: 67,
                velocity: 90,
            },
            0,
        );

        let first = runtime
            .prepared_poly_runtime_regions()
            .iter()
            .find(|region| region.node_id() == "first")
            .unwrap();
        let second = runtime
            .prepared_poly_runtime_regions()
            .iter()
            .find(|region| region.node_id() == "second")
            .unwrap();
        assert_eq!(first.voice_note(0), Some(67));
        assert_eq!(second.voice_note(0), Some(72));
        assert_eq!(second.voice_velocity(0), Some(80));
    }

    #[test]
    fn poly_two_active_voices_sum_sample_wise_without_hidden_velocity_scaling() {
        let prepared = prepare_audio_poly(oscillator_voice("oscillator_voice"), 2);
        let render = |notes: &[(u8, u8)]| {
            let mut runtime = runtime_for(&prepared);
            for &(note, velocity) in notes {
                runtime.note_on(note, velocity);
            }
            let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
            let mut left = vec![0.0; frames];
            let mut right = vec![0.0; frames];
            assert_eq!(
                render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
                frames
            );
            assert_eq!(left, right);
            left
        };

        let quiet_velocity = render(&[(60, 1)]);
        let full_velocity = render(&[(60, 127)]);
        let two_voices = render(&[(60, 1), (60, 127)]);

        assert_eq!(quiet_velocity, full_velocity);
        assert!(quiet_velocity.iter().any(|sample| sample.abs() > 0.001));
        for ((single, doubled), second_single) in quiet_velocity
            .iter()
            .zip(two_voices.iter())
            .zip(full_velocity.iter())
        {
            assert!((doubled - (single + second_single)).abs() < 1.0e-6);
        }
    }

    #[test]
    fn poly_voice_oscillator_and_gain_render_own_note_and_velocity() {
        let voice = GraphDefinition::new("pitched_velocity_voice")
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("gain", builtin_ports::AUDIO_OUT)),
            )
            .with_node(
                Node::new(NodeId::new("oscillator"), module_types::OSCILLATOR).with_static_arg(
                    crate::builtins::WAVEFORM_PARAMETER,
                    StaticArg::Literal(StaticValue::Enum(
                        crate::builtins::WAVEFORM_SINE.to_string(),
                    )),
                ),
            )
            .with_node(Node::new(NodeId::new("gain"), module_types::GAIN))
            .with_connection(Connection::new(
                kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_NOTE_OUTPUT,
                ),
                kernel_ref("oscillator", builtin_ports::PITCH),
            ))
            .with_connection(Connection::new(
                kernel_ref("oscillator", builtin_ports::AUDIO),
                kernel_ref("gain", builtin_ports::AUDIO_IN),
            ))
            .with_connection(Connection::new(
                kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_VELOCITY_OUTPUT,
                ),
                kernel_ref("gain", builtin_ports::GAIN),
            ));
        let prepared = prepare_audio_poly(voice, 2);
        let render = |notes: &[(u8, u8)]| {
            let mut runtime = runtime_for(&prepared);
            for &(note, velocity) in notes {
                runtime.note_on(note, velocity);
            }
            let mut left = [0.0; 8];
            let mut right = [0.0; 8];
            assert_eq!(
                render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
                8
            );
            assert_eq!(left, right);
            left
        };

        let base = render(&[(60, 127)]);
        let octave = render(&[(72, 127)]);
        let soft = render(&[(60, 64)]);
        let layered = render(&[(60, 64), (72, 127)]);
        assert!((base[1] - 0.02879395).abs() < 0.00001);
        assert!((octave[1] - 0.05756403).abs() < 0.00001);
        assert!((soft[1] - base[1] * (64.0 / 127.0)).abs() < 0.00001);
        assert!((layered[1] - (soft[1] + octave[1])).abs() < 0.00001);
    }

    #[test]
    fn poly_two_constant_voices_sum_to_a_positive_absolute_value() {
        let prepared = prepare_audio_poly(constant_voice("constant_voice", 0.25), 2);
        let mut runtime = runtime_for(&prepared);
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let mut left = vec![0.0; frames];
        let mut right = vec![0.0; frames];

        runtime.note_on(60, 100);
        assert_eq!(
            render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
            frames
        );
        assert!(left.iter().all(|sample| *sample == 0.25));
        assert_eq!(left, right);

        runtime.note_on(64, 100);
        assert_eq!(
            render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
            frames
        );
        assert!(left.iter().all(|sample| *sample == 0.5));
        assert_eq!(left, right);
    }

    #[test]
    fn poly_retires_released_voice_at_the_silence_threshold() {
        let prepared = prepare_audio_poly(constant_voice("threshold_voice", 1.0e-4), 1);
        let mut runtime = runtime_for(&prepared);

        runtime.note_on(60, 100);
        render_one_block(&mut runtime);
        runtime.note_off(60);
        for _ in 0..59 {
            render_one_block(&mut runtime);
        }
        assert_eq!(
            runtime.prepared_poly_runtime_regions()[0].active_voice_count(),
            1,
            "59 eight-frame blocks are shorter than ten milliseconds"
        );
        render_one_block(&mut runtime);
        assert_eq!(
            runtime.prepared_poly_runtime_regions()[0].active_voice_count(),
            0,
            "a signal exactly at the threshold counts as silent"
        );
    }

    #[test]
    fn poly_audible_block_resets_the_released_silence_window() {
        let prepared = prepare_audio_poly(constant_voice("interrupted_tail", 0.0), 1);
        let mut runtime = runtime_for(&prepared);

        runtime.note_on(60, 100);
        render_one_block(&mut runtime);
        runtime.note_off(60);
        for _ in 0..59 {
            render_one_block(&mut runtime);
        }
        assert_eq!(
            runtime.prepared_poly_runtime_regions()[0].active_voice_count(),
            1
        );

        assert!(runtime.set_poly_child_control_default_for_test(
            "voices",
            "constant",
            builtin_ports::IN,
            0.25,
        ));
        render_one_block(&mut runtime);
        assert_eq!(
            runtime.prepared_poly_runtime_regions()[0].active_voice_count(),
            1
        );

        assert!(runtime.set_poly_child_control_default_for_test(
            "voices",
            "constant",
            builtin_ports::IN,
            0.0,
        ));
        for _ in 0..59 {
            render_one_block(&mut runtime);
        }
        assert_eq!(
            runtime.prepared_poly_runtime_regions()[0].active_voice_count(),
            1,
            "the first quiet window cannot count across the loud block"
        );
        render_one_block(&mut runtime);
        assert_eq!(runtime.prepared_poly_runtime_regions()[0].active_voice_count(), 0);
    }

    #[test]
    fn poly_done_event_retires_voice_and_clears_its_future_output() {
        let voice = noise_voice("done_voice", 1, 1234).with_port(
            KernelPort::output(crate::kernel::POLY_DONE_OUTPUT, SignalType::Event, 1).maps_from(
                kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_GATE_OUTPUT,
                ),
            ),
        );
        let prepared = prepare_audio_poly(voice, 1);
        let mut runtime = runtime_for(&prepared);
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let mut left = vec![0.0; frames];
        let mut right = vec![0.0; frames];

        runtime.note_on(60, 100);
        assert_eq!(
            render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
            frames
        );
        assert!(left.iter().any(|sample| sample.abs() > 0.001));
        assert_eq!(runtime.prepared_poly_runtime_regions()[0].active_voice_count(), 0);

        left.fill(f32::NAN);
        right.fill(f32::NAN);
        assert_eq!(
            render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
            frames
        );
        assert!(left.iter().all(|sample| *sample == 0.0));
        assert!(right.iter().all(|sample| *sample == 0.0));

        runtime.note_on(64, 100);
        assert_eq!(
            render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
            frames
        );
        assert!(left.iter().any(|sample| sample.abs() > 0.001));
    }

    #[test]
    fn poly_child_event_filter_can_signal_done() {
        let voice = noise_voice("filtered_done_voice", 1, 1234)
            .with_port(
                KernelPort::output(crate::kernel::POLY_DONE_OUTPUT, SignalType::Event, 1)
                    .maps_from(kernel_ref("filter", builtin_ports::EVENTS_OUT)),
            )
            .with_node(
                Node::new(NodeId::new("filter"), module_types::EVENT_FILTER).with_static_arg(
                    crate::builtins::EVENT_FILTER_NOTE_PARAMETER,
                    StaticArg::Literal(StaticValue::Int(60)),
                ),
            )
            .with_connection(Connection::new(
                kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_GATE_OUTPUT,
                ),
                kernel_ref("filter", builtin_ports::EVENTS_IN),
            ));
        let prepared = prepare_audio_poly(voice, 1);
        let mut runtime = runtime_for(&prepared);
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let mut left = vec![0.0; frames];
        let mut right = vec![0.0; frames];

        runtime.note_on(60, 100);
        assert_eq!(
            render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
            frames
        );
        assert!(left.iter().any(|sample| sample.abs() > 0.001));
        assert_eq!(
            runtime.prepared_poly_runtime_regions()[0].active_voice_count(),
            0
        );

        assert_eq!(
            render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
            frames
        );
        assert!(left.iter().all(|sample| *sample == 0.0));
        assert!(right.iter().all(|sample| *sample == 0.0));

        let mut unmatched = runtime_for(&prepared);
        unmatched.note_on(61, 100);
        assert_eq!(
            render_two_mono_root_ports(&mut unmatched, &mut left, &mut right),
            frames
        );
        assert!(left.iter().any(|sample| sample.abs() > 0.001));
        assert_eq!(
            unmatched.prepared_poly_runtime_regions()[0].active_voice_count(),
            1,
            "a filter for note 60 must not signal done for note 61"
        );
    }

    #[test]
    fn poly_adsr_child_renders_audible_release_then_retires() {
        let voice = GraphDefinition::new("enveloped_voice")
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("gain", builtin_ports::AUDIO_OUT)),
            )
            .with_node(
                Node::new(NodeId::new("constant"), module_types::CONTROL_TO_AUDIO)
                    .with_default_override(builtin_ports::IN, 0.25),
            )
            .with_node(
                Node::new(NodeId::new("envelope"), module_types::ADSR)
                    .with_default_override(builtin_ports::ATTACK, 0.0)
                    .with_default_override(builtin_ports::SUSTAIN, 0.5)
                    .with_default_override(builtin_ports::RELEASE, 0.0),
            )
            .with_node(Node::new(NodeId::new("gain"), module_types::GAIN))
            .with_connection(Connection::new(
                kernel_ref("constant", builtin_ports::OUT),
                kernel_ref("gain", builtin_ports::AUDIO_IN),
            ))
            .with_connection(Connection::new(
                kernel_ref("envelope", builtin_ports::VALUE),
                kernel_ref("gain", builtin_ports::GAIN),
            ))
            .with_connection(Connection::new(
                kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_GATE_OUTPUT,
                ),
                kernel_ref("envelope", builtin_ports::GATE),
            ));
        let prepared = prepare_audio_poly(voice, 1);
        let mut runtime = runtime_for(&prepared);
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let mut left = vec![0.0; frames];
        let mut right = vec![0.0; frames];

        runtime.note_on(60, 100);
        for _ in 0..200 {
            assert_eq!(
                render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
                frames
            );
        }
        assert!(left.iter().all(|sample| *sample == 0.125));

        runtime.note_off(60);
        assert_eq!(
            render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
            frames
        );
        assert!(left.iter().any(|sample| *sample > 0.1));
        assert_eq!(
            runtime.prepared_poly_runtime_regions()[0].active_voice_count(),
            1
        );

        for _ in 0..120 {
            assert_eq!(
                render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
                frames
            );
        }
        assert_eq!(
            runtime.prepared_poly_runtime_regions()[0].active_voice_count(),
            0
        );
        assert_eq!(
            render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
            frames
        );
        assert!(left.iter().all(|sample| *sample == 0.0));
        assert!(right.iter().all(|sample| *sample == 0.0));

        let mut midblock = runtime_for(&prepared);
        midblock.note_on_at(60, 100, 0);
        midblock.note_off_at(60, 4);
        assert_eq!(
            render_two_mono_root_ports(&mut midblock, &mut left, &mut right),
            frames
        );
        assert!((left[3] - 0.0078125).abs() < 1.0e-6);
        assert!(
            (left[4] - left[3]).abs() < 1.0e-6,
            "release begins at the level reached immediately before note-off"
        );
    }

    #[test]
    fn poly_gate_release_reaches_only_the_matching_adsr_child() {
        let voice = GraphDefinition::new("enveloped_voice")
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("gain", builtin_ports::AUDIO_OUT)),
            )
            .with_node(
                Node::new(NodeId::new("constant"), module_types::CONTROL_TO_AUDIO)
                    .with_default_override(builtin_ports::IN, 0.25),
            )
            .with_node(
                Node::new(NodeId::new("envelope"), module_types::ADSR)
                    .with_default_override(builtin_ports::ATTACK, 0.0)
                    .with_default_override(builtin_ports::SUSTAIN, 0.5)
                    .with_default_override(builtin_ports::RELEASE, 0.0),
            )
            .with_node(Node::new(NodeId::new("gain"), module_types::GAIN))
            .with_connection(Connection::new(
                kernel_ref("constant", builtin_ports::OUT),
                kernel_ref("gain", builtin_ports::AUDIO_IN),
            ))
            .with_connection(Connection::new(
                kernel_ref("envelope", builtin_ports::VALUE),
                kernel_ref("gain", builtin_ports::GAIN),
            ))
            .with_connection(Connection::new(
                kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_GATE_OUTPUT,
                ),
                kernel_ref("envelope", builtin_ports::GATE),
            ));
        let prepared = prepare_audio_poly(voice, 2);
        let mut runtime = runtime_for(&prepared);
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let mut left = vec![0.0; frames];
        let mut right = vec![0.0; frames];

        runtime.note_on(60, 100);
        runtime.note_on(64, 100);
        for _ in 0..200 {
            assert_eq!(
                render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
                frames
            );
        }
        assert!(left.iter().all(|sample| *sample == 0.25));

        runtime.note_off(60);
        assert_eq!(
            render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
            frames
        );
        assert!(left.iter().any(|sample| *sample > 0.125));
        for _ in 0..120 {
            assert_eq!(
                render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
                frames
            );
        }
        assert!(left.iter().all(|sample| *sample == 0.125));
        assert_eq!(
            runtime.prepared_poly_runtime_regions()[0].active_voice_count(),
            1
        );
    }

    #[test]
    fn poly_done_control_retires_only_when_signalled() {
        let voice = noise_voice("control_done_voice", 1, 1234)
            .with_port(
                KernelPort::output(crate::kernel::POLY_DONE_OUTPUT, SignalType::Control, 1)
                    .maps_from(kernel_ref("mapper", builtin_ports::VALUE)),
            )
            .with_node(Node::new(NodeId::new("mapper"), module_types::CURVE_MAPPER))
            .with_connection(Connection::new(
                kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_VELOCITY_OUTPUT,
                ),
                kernel_ref("mapper", builtin_ports::VALUE),
            ));
        let prepared = prepare_audio_poly(voice, 1);
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let mut left = vec![0.0; frames];
        let mut right = vec![0.0; frames];

        let mut unsignalled = runtime_for(&prepared);
        unsignalled.note_on(60, 0);
        assert_eq!(
            render_two_mono_root_ports(&mut unsignalled, &mut left, &mut right),
            frames
        );
        assert_eq!(
            unsignalled.prepared_poly_runtime_regions()[0].active_voice_count(),
            1
        );
        assert!(left.iter().any(|sample| sample.abs() > 0.001));

        let mut signalled = runtime_for(&prepared);
        signalled.note_on(60, 100);
        assert_eq!(
            render_two_mono_root_ports(&mut signalled, &mut left, &mut right),
            frames
        );
        assert_eq!(
            signalled.prepared_poly_runtime_regions()[0].active_voice_count(),
            0
        );
        assert_eq!(
            render_two_mono_root_ports(&mut signalled, &mut left, &mut right),
            frames
        );
        assert!(left.iter().all(|sample| *sample == 0.0));
        assert!(right.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn poly_done_control_pulse_after_first_frame_retires_and_reuses_voice() {
        // At 12 kHz in a 48 kHz render, the LFO starts at 0.5 and reaches 1.0
        // on frame 1. The offset makes `done` zero at frame 0 and positive later.
        let voice = constant_voice("pulsed_done_voice", -0.25)
            .with_port(
                KernelPort::output(crate::kernel::POLY_DONE_OUTPUT, SignalType::Control, 1)
                    .maps_from(kernel_ref("mapper", builtin_ports::VALUE)),
            )
            .with_node(
                Node::new(NodeId::new("lfo"), module_types::LFO)
                    .with_default_override(builtin_ports::RATE, 12_000.0),
            )
            .with_node(
                Node::new(NodeId::new("mapper"), module_types::CURVE_MAPPER)
                    .with_default_override(builtin_ports::OFFSET, -0.5),
            )
            .with_connection(Connection::new(
                kernel_ref("lfo", builtin_ports::VALUE),
                kernel_ref("mapper", builtin_ports::VALUE),
            ));
        let prepared = prepare_audio_poly(voice, 1);
        let mut runtime = runtime_for(&prepared);
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let mut left = vec![0.0; frames];
        let mut right = vec![0.0; frames];

        runtime.note_on(60, 100);
        assert_eq!(
            render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
            frames
        );
        assert_eq!(left, vec![-0.25; frames]);
        assert_eq!(right, left);
        assert_eq!(
            runtime.prepared_poly_runtime_regions()[0].active_voice_count(),
            0
        );

        render_two_mono_root_ports(&mut runtime, &mut left, &mut right);
        assert_eq!(left, vec![0.0; frames]);
        assert_eq!(right, left);

        runtime.note_on(61, 100);
        render_two_mono_root_ports(&mut runtime, &mut left, &mut right);
        assert_eq!(left, vec![-0.25; frames]);
        assert_eq!(right, left);
        assert_eq!(
            runtime.prepared_poly_runtime_regions()[0].active_voice_count(),
            0
        );
    }

    #[test]
    fn poly_done_control_is_observed_before_a_later_child_reuses_its_buffer() {
        let voice = GraphDefinition::new("reused_done_voice")
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("gain2", builtin_ports::AUDIO_OUT)),
            )
            .with_port(
                KernelPort::output(crate::kernel::POLY_DONE_OUTPUT, SignalType::Control, 1)
                    .maps_from(kernel_ref("mapper", builtin_ports::VALUE)),
            )
            .with_node(Node::new(NodeId::new("mapper"), module_types::CURVE_MAPPER))
            .with_node(Node::new(NodeId::new("gain1"), module_types::GAIN))
            .with_node(Node::new(NodeId::new("gain2"), module_types::GAIN))
            .with_connection(Connection::new(
                kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_VELOCITY_OUTPUT,
                ),
                kernel_ref("mapper", builtin_ports::VALUE),
            ))
            .with_connection(Connection::new(
                kernel_ref("mapper", builtin_ports::VALUE),
                kernel_ref("gain1", builtin_ports::GAIN),
            ))
            .with_connection(Connection::new(
                kernel_ref("gain1", builtin_ports::AUDIO_OUT),
                kernel_ref("gain2", builtin_ports::AUDIO_IN),
            ));
        let prepared = prepare_audio_poly(voice, 1);
        let mut runtime = runtime_for(&prepared);
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let mut left = vec![0.0; frames];
        let mut right = vec![0.0; frames];

        runtime.note_on(60, 100);
        assert_eq!(
            render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
            frames
        );
        assert_eq!(
            runtime.prepared_poly_runtime_regions()[0].active_voice_count(),
            0
        );
    }

    #[test]
    fn preparation_rejects_declared_poly_done_without_a_resolvable_source() {
        for done_port in [
            KernelPort::output(crate::kernel::POLY_DONE_OUTPUT, SignalType::Control, 1),
            KernelPort::output(crate::kernel::POLY_DONE_OUTPUT, SignalType::Control, 1)
                .maps_from(kernel_ref("noise", "unknown_output")),
        ] {
            let voice = noise_voice("missing_done_voice", 1, 1234).with_port(done_port);
            let root = GraphDefinition::new("root")
                .with_port(
                    KernelPort::output("master", SignalType::Audio, 1)
                        .maps_from(kernel_ref("voices", "audio")),
                )
                .with_node(poly_node("voices", voice.name(), 1));
            let error = prepare_kernel_graph_with_buses(
                &root,
                &builtin_registry().with_definition(voice),
                &KERNEL_RENDER_SETTINGS,
                &HostBuses::new().with_output("master", 1),
            )
            .expect_err("declared done requires one resolvable source");
            let diagnostic = error.diagnostics().errors().next().unwrap();

            assert_eq!(
                diagnostic.error_code(),
                diagnostics::error_codes::KERNEL_POLY_MALFORMED_INTERFACE
            );
            assert_eq!(diagnostic.module_id(), Some("voices"));
        }
    }

    #[test]
    fn poly_without_done_retires_after_ten_milliseconds_of_released_silence() {
        let voice = GraphDefinition::new("silent_voice")
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("gain", builtin_ports::AUDIO_OUT)),
            )
            .with_port(
                KernelPort::output("velocity", SignalType::Control, 1).maps_from(kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_VELOCITY_OUTPUT,
                )),
            )
            .with_node(Node::new(NodeId::new("gain"), module_types::GAIN));
        let prepared = prepare_audio_poly(voice, 1);
        let mut runtime = runtime_for(&prepared);

        runtime.note_on(60, 100);
        for _ in 0..64 {
            render_one_block(&mut runtime);
        }
        assert_eq!(
            runtime.prepared_poly_runtime_regions()[0].active_voice_count(),
            1,
            "silence while the gate is held must not retire the voice"
        );

        runtime.note_off_at(60, 4);
        for _ in 0..60 {
            render_one_block(&mut runtime);
        }
        assert_eq!(
            runtime.prepared_poly_runtime_regions()[0].active_voice_count(),
            1,
            "a note-off at frame four leaves only 476 silent frames after 60 blocks"
        );
        render_one_block(&mut runtime);
        assert_eq!(
            runtime.prepared_poly_runtime_regions()[0].active_voice_count(),
            0
        );

        runtime.note_on(64, 100);
        render_one_block(&mut runtime);
        assert_eq!(
            runtime.prepared_poly_runtime_regions()[0].voice_note(0),
            Some(64)
        );
    }

    #[test]
    fn poly_without_done_times_out_after_five_seconds_of_audible_release() {
        let voice = noise_voice("sustained_voice", 1, 1234);
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("left", SignalType::Audio, 1)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_port(
                KernelPort::output("right", SignalType::Audio, 1)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_node(poly_node("voices", voice.name(), 1));
        let settings = patch::RenderSettings {
            sample_rate_hz: 1_000,
            block_size_frames: 100,
            duration_frames: 100,
        };
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry().with_definition(voice),
            &settings,
            &HostBuses::new()
                .with_output("left", 1)
                .with_output("right", 1),
        )
        .expect("sustained poly voice prepares");
        let mut runtime = RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
            prepared.graph().clone(),
            prepared.compiled_patch().clone(),
            1_000.0,
            &PreparedSamplerAssets::empty(),
            &crate::patch::VoiceAllocation::default(),
            100,
        );
        let mut left = vec![0.0; 100];
        let mut right = vec![0.0; 100];

        runtime.note_on(60, 100);
        assert_eq!(
            render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
            100
        );
        assert!(left.iter().any(|sample| sample.abs() > 0.001));

        runtime.note_off_at(60, 50);
        for _ in 0..50 {
            assert_eq!(
                render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
                100
            );
        }
        assert_eq!(
            runtime.prepared_poly_runtime_regions()[0].active_voice_count(),
            1,
            "note-off at frame 50 leaves only 4.95 seconds elapsed"
        );
        assert!(left.iter().any(|sample| sample.abs() > 0.001));

        assert_eq!(
            render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
            100
        );
        assert_eq!(
            runtime.prepared_poly_runtime_regions()[0].active_voice_count(),
            0
        );
        assert_eq!(
            render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
            100
        );
        assert!(left.iter().all(|sample| *sample == 0.0));
        assert!(right.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn poly_sums_every_channel_of_a_six_channel_voice_output() {
        let voice = noise_voice("six_channel_voice", 6, 1234);
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("surround", SignalType::Audio, 6)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_node(poly_node_with_allocation(
                "voices",
                voice.name(),
                2,
                crate::kernel::POLY_ALLOCATION_REJECT_NEW,
            ));
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry().with_definition(voice),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new().with_output("surround", 6),
        )
        .expect("six-channel poly prepares");
        let render = |voice_count: usize| {
            let mut runtime = runtime_for(&prepared);
            for note in 0..voice_count {
                runtime.note_on(60 + note as u8, 100);
            }
            let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
            let mut outputs = vec![vec![vec![0.0; frames]; 6]];
            assert_eq!(runtime.render_root_outputs(&mut outputs), frames);
            outputs.remove(0)
        };

        let one = render(1);
        let two = render(2);
        assert_eq!(one.len(), 6);
        for (single_channel, doubled_channel) in one.iter().zip(two.iter()) {
            assert!(single_channel.iter().any(|sample| sample.abs() > 0.001));
            for (single, doubled) in single_channel.iter().zip(doubled_channel.iter()) {
                assert!((doubled - single * 2.0).abs() < 1.0e-6);
            }
        }
    }

    #[test]
    fn sibling_poly_regions_keep_state_queues_and_different_width_mixes_independent() {
        let mono = noise_voice("mono_voice", 1, 111);
        let stereo = noise_voice("stereo_voice", 2, 222);
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("mono", SignalType::Audio, 1)
                    .maps_from(kernel_ref("mono_poly", "audio")),
            )
            .with_port(
                KernelPort::output("stereo", SignalType::Audio, 2)
                    .maps_from(kernel_ref("stereo_poly", "audio")),
            )
            .with_node(poly_node_with_allocation(
                "mono_poly",
                mono.name(),
                1,
                crate::kernel::POLY_ALLOCATION_REJECT_NEW,
            ))
            .with_node(poly_node_with_allocation(
                "stereo_poly",
                stereo.name(),
                1,
                crate::kernel::POLY_ALLOCATION_REJECT_NEW,
            ));
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry()
                .with_definition(mono)
                .with_definition(stereo),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new()
                .with_output("mono", 1)
                .with_output("stereo", 2),
        )
        .expect("mixed-width sibling poly regions prepare");
        let mut runtime = runtime_for(&prepared);
        runtime.route_poly_note_event_for_test(
            "mono_poly",
            ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
            0,
        );
        runtime.route_poly_note_event_for_test(
            "stereo_poly",
            ScriptEvent::NoteOn {
                note: 72,
                velocity: 80,
            },
            0,
        );
        let regions = runtime.prepared_poly_runtime_regions();
        assert_ne!(
            regions[0].state_instance_address(0, 0),
            regions[1].state_instance_address(0, 0)
        );

        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let mut outputs = vec![vec![vec![0.0; frames]], vec![vec![0.0; frames]; 2]];
        assert_eq!(runtime.render_root_outputs(&mut outputs), frames);
        assert_eq!(outputs[0].len(), 1);
        assert_eq!(outputs[1].len(), 2);
        assert!(outputs[0][0].iter().any(|sample| sample.abs() > 0.001));
        assert!(
            outputs[1]
                .iter()
                .flatten()
                .any(|sample| sample.abs() > 0.001)
        );
    }

    #[test]
    fn sibling_poly_regions_render_signed_outputs_at_the_note_sample() {
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("positive", SignalType::Audio, 1)
                    .maps_from(kernel_ref("positive_voices", "audio")),
            )
            .with_port(
                KernelPort::output("negative", SignalType::Audio, 1)
                    .maps_from(kernel_ref("negative_voices", "audio")),
            )
            .with_node(poly_node("positive_voices", "positive_voice", 1))
            .with_node(poly_node("negative_voices", "negative_voice", 1));
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry()
                .with_definition(constant_voice("positive_voice", 0.25))
                .with_definition(constant_voice("negative_voice", -0.5)),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new()
                .with_output("positive", 1)
                .with_output("negative", 1),
        )
        .expect("sibling pools with child DSP should prepare");
        let mut runtime = runtime_for(&prepared);
        let mut outputs = vec![vec![vec![0.0; 8]], vec![vec![0.0; 8]]];

        let allocations = count_current_thread_allocations(|| {
            runtime.note_on_at(60, 100, 3);
            assert_eq!(runtime.render_root_outputs(&mut outputs), 8);
        });
        assert_eq!(allocations, 0);
        assert_eq!(outputs[0][0], [0.0, 0.0, 0.0, 0.25, 0.25, 0.25, 0.25, 0.25]);
        assert_eq!(outputs[1][0], [0.0, 0.0, 0.0, -0.5, -0.5, -0.5, -0.5, -0.5]);
    }

    #[test]
    fn nested_poly_voices_render_through_independent_inner_pools() {
        let inner = constant_voice("inner", 0.25);
        let outer = GraphDefinition::new("outer")
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("inner_voices", "audio")),
            )
            .with_node(poly_node("inner_voices", "inner", 2))
            .with_connection(Connection::new(
                kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_GATE_OUTPUT,
                ),
                kernel_ref("inner_voices", "notes"),
            ));
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("master", SignalType::Audio, 1)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_node(poly_node_with_allocation(
                "voices",
                "outer",
                2,
                crate::kernel::POLY_ALLOCATION_OLDEST_STEAL,
            ));
        let registry = builtin_registry()
            .with_definition(inner.clone())
            .with_definition(outer.clone());
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &registry,
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new().with_output("master", 1),
        )
        .expect("nested poly regions prepare");
        assert_eq!(prepared.compiled_patch().poly_regions().len(), 1);
        assert_eq!(
            prepared.compiled_patch().poly_regions()[0]
                .child_patch()
                .poly_regions()
                .len(),
            1
        );
        let mut runtime = runtime_for(&prepared);
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let mut outputs = vec![vec![vec![0.0; frames]]];

        let allocation_count = count_current_thread_allocations(|| {
            runtime.note_on_at(60, 100, 2);
            runtime.note_on_at(64, 100, 5);
            assert_eq!(runtime.render_root_outputs(&mut outputs), frames);
        });
        assert_eq!(allocation_count, 0);
        assert_eq!(outputs[0][0], [0.0, 0.0, 0.25, 0.25, 0.25, 0.5, 0.5, 0.5]);
        let outer_region = &runtime.prepared_poly_runtime_regions()[0];
        let inner_a = outer_region
            .nested_region_for_voice(0, "inner_voices")
            .unwrap();
        let inner_b = outer_region
            .nested_region_for_voice(1, "inner_voices")
            .unwrap();
        assert_eq!(inner_a.active_voice_count(), 1);
        assert_eq!(inner_b.active_voice_count(), 1);
        assert_eq!(inner_a.voice_count(), 2);
        assert_eq!(inner_b.voice_count(), 2);
        assert_eq!(inner_a.output_accumulator_buffer_count(), 1);
        assert_eq!(inner_b.output_accumulator_buffer_count(), 1);
        assert_eq!(inner_a.voice_note(0), Some(60));
        assert_eq!(inner_b.voice_note(0), Some(64));
        assert_ne!(
            inner_a.state_instance_address(0, 0),
            inner_b.state_instance_address(0, 0)
        );
        let inner_a = runtime.prepared_poly_runtime_regions_mut_for_test()[0]
            .nested_region_for_voice_mut(0, "inner_voices")
            .unwrap();
        inner_a.route_note_event_for_test(
            ScriptEvent::NoteOn {
                note: 72,
                velocity: 100,
            },
            frames,
        );
        inner_a.route_note_event_for_test(
            ScriptEvent::NoteOn {
                note: 75,
                velocity: 100,
            },
            frames,
        );
        assert_eq!(runtime.render_root_outputs(&mut outputs), frames);
        let outer_region = &runtime.prepared_poly_runtime_regions()[0];
        let inner_a = outer_region
            .nested_region_for_voice(0, "inner_voices")
            .unwrap();
        let inner_b = outer_region
            .nested_region_for_voice(1, "inner_voices")
            .unwrap();
        assert_eq!(inner_a.active_voice_count(), 2);
        assert_eq!(inner_a.voice_note(0), Some(60));
        assert_eq!(inner_a.voice_note(1), Some(72));
        assert_eq!(inner_b.active_voice_count(), 1);
        assert_eq!(inner_b.voice_note(0), Some(64));
        assert!(
            outputs[0][0].iter().all(|sample| *sample == 0.75),
            "{outputs:?}"
        );

        let steal_allocations = count_current_thread_allocations(|| {
            runtime.note_on(67, 100);
            assert_eq!(runtime.render_root_outputs(&mut outputs), frames);
        });
        assert_eq!(steal_allocations, 0);
        let outer_region = &runtime.prepared_poly_runtime_regions()[0];
        let inner_a = outer_region
            .nested_region_for_voice(0, "inner_voices")
            .unwrap();
        let inner_b = outer_region
            .nested_region_for_voice(1, "inner_voices")
            .unwrap();
        assert_eq!(inner_a.active_voice_count(), 1);
        assert_eq!(inner_b.active_voice_count(), 1);
        assert_eq!(inner_a.voice_note(0), Some(67));
        assert_eq!(inner_b.voice_note(0), Some(64));
        assert!(
            outputs[0][0].iter().all(|sample| *sample == 0.5),
            "{outputs:?}"
        );

        runtime.reset();
        assert_eq!(runtime.render_root_outputs(&mut outputs), frames);
        assert_eq!(outputs[0][0], vec![0.0; frames]);
        let outer_region = &runtime.prepared_poly_runtime_regions()[0];
        assert_eq!(outer_region.active_voice_count(), 0);
        assert_eq!(
            outer_region
                .nested_region_for_voice(0, "inner_voices")
                .unwrap()
                .active_voice_count(),
            0
        );
        assert_eq!(
            outer_region
                .nested_region_for_voice(1, "inner_voices")
                .unwrap()
                .active_voice_count(),
            0
        );

        let done_outer = outer.with_port(
            KernelPort::output(crate::kernel::POLY_DONE_OUTPUT, SignalType::Event, 1).maps_from(
                kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_GATE_OUTPUT,
                ),
            ),
        );
        let done_prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry()
                .with_definition(inner)
                .with_definition(done_outer),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new().with_output("master", 1),
        )
        .expect("nested poly with parent completion prepares");
        let mut done_runtime = runtime_for(&done_prepared);
        done_runtime.note_on(60, 100);
        assert_eq!(done_runtime.render_root_outputs(&mut outputs), frames);
        assert!(outputs[0][0].iter().all(|sample| *sample == 0.25));
        let retired_outer = &done_runtime.prepared_poly_runtime_regions()[0];
        assert_eq!(retired_outer.active_voice_count(), 0);
        assert_eq!(
            retired_outer
                .nested_region_for_voice(0, "inner_voices")
                .unwrap()
                .active_voice_count(),
            0
        );
        assert_eq!(done_runtime.render_root_outputs(&mut outputs), frames);
        assert!(outputs[0][0].iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn released_nested_poly_voice_retires_its_inner_pool_after_silence() {
        let inner = GraphDefinition::new("inner")
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("gain", builtin_ports::AUDIO_OUT)),
            )
            .with_node(
                Node::new(NodeId::new("constant"), module_types::CONTROL_TO_AUDIO)
                    .with_default_override(builtin_ports::IN, 0.25),
            )
            .with_node(
                Node::new(NodeId::new("envelope"), module_types::ADSR)
                    .with_default_override(builtin_ports::ATTACK, 0.0)
                    .with_default_override(builtin_ports::SUSTAIN, 0.5)
                    .with_default_override(builtin_ports::RELEASE, 0.0),
            )
            .with_node(Node::new(NodeId::new("gain"), module_types::GAIN))
            .with_connection(Connection::new(
                kernel_ref("constant", builtin_ports::OUT),
                kernel_ref("gain", builtin_ports::AUDIO_IN),
            ))
            .with_connection(Connection::new(
                kernel_ref("envelope", builtin_ports::VALUE),
                kernel_ref("gain", builtin_ports::GAIN),
            ))
            .with_connection(Connection::new(
                kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_GATE_OUTPUT,
                ),
                kernel_ref("envelope", builtin_ports::GATE),
            ));
        let outer = GraphDefinition::new("outer")
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("inner_voices", "audio")),
            )
            .with_node(poly_node("inner_voices", "inner", 1))
            .with_connection(Connection::new(
                kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_GATE_OUTPUT,
                ),
                kernel_ref("inner_voices", "notes"),
            ));
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("master", SignalType::Audio, 1)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_node(poly_node("voices", "outer", 1));
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry()
                .with_definition(inner)
                .with_definition(outer),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new().with_output("master", 1),
        )
        .expect("nested envelope prepares");
        let mut runtime = runtime_for(&prepared);
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let mut outputs = vec![vec![vec![0.0; frames]]];
        runtime.note_on(60, 100);
        for _ in 0..200 {
            assert_eq!(runtime.render_root_outputs(&mut outputs), frames);
        }
        assert_eq!(outputs[0][0], vec![0.125; frames]);
        runtime.note_off(60);
        for _ in 0..120 {
            assert_eq!(runtime.render_root_outputs(&mut outputs), frames);
        }
        assert_eq!(outputs[0][0], vec![0.0; frames]);
        let outer_region = &runtime.prepared_poly_runtime_regions()[0];
        assert_eq!(outer_region.active_voice_count(), 0);
        assert_eq!(
            outer_region
                .nested_region_for_voice(0, "inner_voices")
                .unwrap()
                .active_voice_count(),
            0
        );
    }

    #[test]
    fn nested_poly_root_output_survives_later_consumers_of_the_same_source() {
        let inner = constant_voice("inner_liveness", 0.25);
        let outer = GraphDefinition::new("outer_liveness")
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("inner_voices", "audio")),
            )
            .with_node(poly_node("inner_voices", inner.name(), 1))
            .with_node(
                Node::new(NodeId::new("mute"), module_types::GAIN)
                    .with_default_override(builtin_ports::GAIN, 0.0),
            )
            .with_node(Node::new(NodeId::new("later"), module_types::GAIN))
            .with_connection(Connection::new(
                kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_GATE_OUTPUT,
                ),
                kernel_ref("inner_voices", "notes"),
            ))
            .with_connection(Connection::new(
                kernel_ref("inner_voices", "audio"),
                kernel_ref("mute", builtin_ports::AUDIO_IN),
            ))
            .with_connection(Connection::new(
                kernel_ref("mute", builtin_ports::AUDIO_OUT),
                kernel_ref("later", builtin_ports::AUDIO_IN),
            ));
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("master", SignalType::Audio, 1)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_node(poly_node("voices", outer.name(), 1));
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry()
                .with_definition(inner)
                .with_definition(outer),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new().with_output("master", 1),
        )
        .expect("nested graph with a shared root source prepares");
        let mut runtime = runtime_for(&prepared);
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let mut outputs = vec![vec![vec![0.0; frames]]];

        runtime.note_on(60, 100);
        assert_eq!(runtime.render_root_outputs(&mut outputs), frames);
        assert!(
            outputs[0][0].iter().all(|sample| *sample == 0.25),
            "{outputs:?}"
        );
    }

    #[test]
    fn preparation_rejects_recursive_poly_wrapping_before_expansion() {
        let recursive = GraphDefinition::new("recursive")
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("again", "audio")),
            )
            .with_node(poly_node("again", "recursive", 1));
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("master", SignalType::Audio, 1)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_node(poly_node("voices", "recursive", 1));

        let error = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry().with_definition(recursive),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new().with_output("master", 1),
        )
        .expect_err("recursive poly wrapping must fail before expansion");
        assert_eq!(
            error.diagnostics().errors().next().unwrap().error_code(),
            diagnostics::error_codes::KERNEL_RECURSIVE_DEFINITION
        );
    }

    #[test]
    fn nested_poly_forwards_audio_and_control_inputs_to_each_inner_voice() {
        let inner = GraphDefinition::new("inner_gain")
            .with_port(
                KernelPort::input("input", SignalType::Audio, 2)
                    .maps_to(kernel_ref("gain", builtin_ports::AUDIO_IN)),
            )
            .with_port(
                KernelPort::input("level", SignalType::Control, 1)
                    .maps_to(kernel_ref("gain", builtin_ports::GAIN)),
            )
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 2)
                    .maps_from(kernel_ref("gain", builtin_ports::AUDIO_OUT)),
            )
            .with_node(
                Node::new(NodeId::new("gain"), module_types::GAIN).with_static_arg(
                    crate::kernel::builtins::CHANNELS_PARAM,
                    StaticArg::Literal(StaticValue::Int(2)),
                ),
            );
        let outer = GraphDefinition::new("outer_gain")
            .with_port(
                KernelPort::input("input", SignalType::Audio, 2)
                    .maps_to(kernel_ref("inner_voices", "input")),
            )
            .with_port(
                KernelPort::input("level", SignalType::Control, 1)
                    .maps_to(kernel_ref("inner_voices", "level")),
            )
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 2)
                    .maps_from(kernel_ref("inner_voices", "audio")),
            )
            .with_node(poly_node("inner_voices", "inner_gain", 2))
            .with_connection(Connection::new(
                kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_GATE_OUTPUT,
                ),
                kernel_ref("inner_voices", "notes"),
            ));
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::input("input", SignalType::Audio, 2)
                    .maps_to(kernel_ref("voices", "input")),
            )
            .with_port(
                KernelPort::input("level", SignalType::Control, 1)
                    .maps_to(kernel_ref("voices", "level")),
            )
            .with_port(
                KernelPort::output("master", SignalType::Audio, 2)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_node(poly_node("voices", "outer_gain", 2));
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry()
                .with_definition(inner)
                .with_definition(outer),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new()
                .with_input("input", 2)
                .with_input("level", 1)
                .with_output("master", 2),
        )
        .expect("nested forwarded inputs prepare");
        let mut runtime = runtime_for(&prepared);
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let inputs = vec![
            vec![vec![-0.5; frames], vec![0.25; frames]],
            vec![vec![0.5; frames]],
        ];
        let mut outputs = vec![vec![vec![0.0; frames]; 2]];

        let allocations = count_current_thread_allocations(|| {
            runtime.note_on(60, 100);
            runtime.note_on(64, 100);
            assert_eq!(runtime.render_root_buses(&inputs, &mut outputs), frames);
        });
        assert_eq!(allocations, 0);
        assert!(outputs[0][0].iter().all(|sample| *sample == -0.5));
        assert!(outputs[0][1].iter().all(|sample| *sample == 0.25));
    }

    #[test]
    fn nested_poly_forwards_an_event_input_to_inner_completion() {
        let inner = constant_voice("inner_event_voice", 0.25)
            .with_port(
                KernelPort::input("trigger", SignalType::Event, 1)
                    .maps_to(kernel_ref("filter", builtin_ports::EVENTS_IN)),
            )
            .with_port(
                KernelPort::output(crate::kernel::POLY_DONE_OUTPUT, SignalType::Event, 1)
                    .maps_from(kernel_ref("filter", builtin_ports::EVENTS_OUT)),
            )
            .with_node(
                Node::new(NodeId::new("filter"), module_types::EVENT_FILTER).with_static_arg(
                    crate::builtins::EVENT_FILTER_NOTE_PARAMETER,
                    StaticArg::Literal(StaticValue::Int(60)),
                ),
            );
        let outer = GraphDefinition::new("outer_event_voice")
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("inner_voices", "audio")),
            )
            .with_node(poly_node("inner_voices", "inner_event_voice", 1))
            .with_connection(Connection::new(
                kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_GATE_OUTPUT,
                ),
                kernel_ref("inner_voices", "notes"),
            ))
            .with_connection(Connection::new(
                kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_GATE_OUTPUT,
                ),
                kernel_ref("inner_voices", "trigger"),
            ));
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("master", SignalType::Audio, 1)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_node(poly_node("voices", "outer_event_voice", 1));
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry()
                .with_definition(inner)
                .with_definition(outer),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new().with_output("master", 1),
        )
        .expect("nested event forwarding prepares");
        let mut runtime = runtime_for(&prepared);
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let mut outputs = vec![vec![vec![0.0; frames]]];

        let allocations = count_current_thread_allocations(|| {
            runtime.note_on(60, 100);
            assert_eq!(runtime.render_root_outputs(&mut outputs), frames);
        });
        assert_eq!(allocations, 0);
        assert!(outputs[0][0].iter().all(|sample| *sample == 0.25));
        let inner = runtime.prepared_poly_runtime_regions()[0]
            .nested_region_for_voice(0, "inner_voices")
            .unwrap();
        assert_eq!(inner.active_voice_count(), 0);
        assert_eq!(runtime.render_root_outputs(&mut outputs), frames);
        assert!(outputs[0][0].iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn segmented_poly_render_zero_fills_short_bound_audio_input() {
        let voice = GraphDefinition::new("input_voice")
            .with_port(
                KernelPort::input("input", SignalType::Audio, 1)
                    .maps_to(kernel_ref("gain", builtin_ports::AUDIO_IN)),
            )
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("gain", builtin_ports::AUDIO_OUT)),
            )
            .with_node(Node::new(NodeId::new("gain"), module_types::GAIN));
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::input("input", SignalType::Audio, 1)
                    .maps_to(kernel_ref("voices", "input")),
            )
            .with_port(
                KernelPort::output("master", SignalType::Audio, 1)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_node(poly_node_with_allocation(
                "voices",
                "input_voice",
                1,
                crate::kernel::POLY_ALLOCATION_OLDEST_STEAL,
            ));
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry().with_definition(voice),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new()
                .with_input("input", 1)
                .with_output("master", 1),
        )
        .expect("bound-input poly graph prepares");
        let mut runtime = runtime_for(&prepared);
        let inputs = vec![vec![vec![0.25_f32; 2]]];
        let mut outputs = vec![vec![vec![0.0_f32; 8]]];
        runtime.note_on_at(60, 100, 0);
        runtime.note_on_at(61, 100, 4);
        assert_eq!(runtime.render_root_buses(&inputs, &mut outputs), 8);
        assert_eq!(outputs[0][0], [0.25, 0.25, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn preparation_renders_dynamics_inside_a_poly_child() {
        let voice = GraphDefinition::new("dynamics_voice")
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("dynamics", builtin_ports::AUDIO_OUT)),
            )
            .with_node(Node::new(NodeId::new("noise"), module_types::NOISE))
            .with_node(Node::new(
                NodeId::new("dynamics"),
                module_types::DYNAMICS_PROCESSOR,
            ))
            .with_connection(Connection::new(
                kernel_ref("noise", builtin_ports::AUDIO),
                kernel_ref("dynamics", builtin_ports::AUDIO_IN),
            ));
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("master", SignalType::Audio, 1)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_node(poly_node("voices", voice.name(), 1));
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry().with_definition(voice),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new().with_output("master", 1),
        )
        .expect("dynamics poly child prepares");
        let mut runtime = runtime_for(&prepared);
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let mut outputs = vec![vec![vec![0.0; frames]]];
        runtime.note_on(60, 100);
        assert_eq!(runtime.render_root_outputs(&mut outputs), frames);
        assert!(outputs[0][0].iter().any(|sample| sample.abs() > 0.001));
    }

    #[test]
    fn preparation_names_a_direct_script_voice_when_its_runtime_is_unsupported() {
        let voice = GraphDefinition::new("script_voice")
            .with_implementation(crate::kernel::DefinitionImplementation::Script)
            .with_static_param(
                crate::kernel::StaticParam::new(
                    SCRIPT_LANGUAGE_PARAMETER,
                    crate::kernel::StaticType::Enum,
                )
                .with_default(StaticValue::Enum(SCRIPT_LANGUAGE_RHAI.to_string()))
                .with_allowed_values([SCRIPT_LANGUAGE_RHAI]),
            )
            .with_static_param(
                crate::kernel::StaticParam::new(
                    SCRIPT_SOURCE_PARAMETER,
                    crate::kernel::StaticType::String,
                )
                .with_default(StaticValue::String(
                    "fn process(ctx) { ctx.control(\"value\", 0.5); }".to_string(),
                )),
            )
            .with_port(KernelPort::output("value", SignalType::Control, 1));
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("master", SignalType::Control, 1)
                    .maps_from(kernel_ref("voices", "value")),
            )
            .with_node(poly_node("voices", voice.name(), 1));
        let error = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry().with_definition(voice),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new().with_output("master", 1),
        )
        .expect_err("direct script voice cannot render in the prepared poly runtime yet");
        let diagnostic = error.diagnostics().errors().next().unwrap();

        assert_eq!(
            diagnostic.error_code(),
            diagnostics::error_codes::KERNEL_POLY_RUNTIME_UNSUPPORTED
        );
        assert_eq!(diagnostic.module_id(), Some("voices"));
    }

    #[test]
    fn realtime_construction_prepares_independent_stateful_poly_voice_storage() {
        let stateful_voice = noise_voice("stateful_voice", 1, 1234);
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("master", SignalType::Audio, 1)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_node(poly_node("voices", "stateful_voice", 3));
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry().with_definition(stateful_voice),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new().with_output("master", 1),
        )
        .expect("stateful poly child prepares");

        let runtime = RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
            prepared.graph().clone(),
            prepared.compiled_patch().clone(),
            KERNEL_RENDER_SETTINGS.sample_rate_hz as f32,
            &PreparedSamplerAssets::empty(),
            &crate::patch::VoiceAllocation::default(),
            KERNEL_RENDER_SETTINGS.block_size_frames as usize,
        );
        let regions = runtime.prepared_poly_runtime_regions();

        assert_eq!(regions.len(), 1);
        let region = &regions[0];
        assert_eq!(region.node_id(), "voices");
        assert_eq!(region.voice_count(), 3);
        assert_eq!(region.states_per_voice(), region.child_module_kinds().len());
        let noise_index = region
            .child_module_kinds()
            .iter()
            .position(|kind| *kind == ModuleKind::Noise)
            .expect("the prepared voice contains stateful noise");
        let state_addresses = (0..region.voice_count())
            .map(|voice| region.state_instance_address(voice, noise_index).unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            state_addresses.len(),
            3,
            "each voice owns a real state instance"
        );
        assert_eq!(region.voice_arena_count(), 3);
        assert!(region.audio_buffers_per_voice() > 0);
        assert_eq!(region.voice_event_queue_set_count(), 3);
        assert_eq!(region.event_queues_per_voice(), 1);
        assert_eq!(
            region.event_queue_capacity(),
            crate::graph_processor::prepared_event_capacity(
                (KERNEL_RENDER_SETTINGS.block_size_frames as usize) * 2
            )
        );
        assert_eq!(region.output_accumulator_buffer_count(), 1);
    }

    #[test]
    fn named_script_definition_prepares_typed_construction_for_each_instance() {
        let yaml = r#"
metadata: { name: scripted }
ports:
  - { name: left, direction: output, signal: audio, channels: 1, maps_from: left_gain.audio_out }
  - { name: right, direction: output, signal: audio, channels: 1, maps_from: right_gain.audio_out }
module_definitions:
  - type: counter
    implementation: script
    static_params:
      - { name: language, type: enum, default: rhai, allowed_values: [rhai] }
      - { name: source, type: string }
    ports:
      - { name: increment, direction: input, signal: control, channels: 1, default: 1 }
      - { name: count, direction: output, signal: control, channels: 1 }
modules:
  - id: left_counter
    type: counter
    static:
      source: |
        fn process(ctx) {
          let count = ctx.state_get("count") + ctx.controls.increment;
          ctx.state_set("count", count);
          ctx.control("count", count);
        }
  - id: right_counter
    type: counter
    static:
      source: |
        fn process(ctx) {
          let count = ctx.state_get("count") + ctx.controls.increment;
          ctx.state_set("count", count);
          ctx.control("count", count);
        }
  - { id: left_gain, type: gain, defaults: { gain: 1 } }
  - { id: right_gain, type: gain, defaults: { gain: 1 } }
connections:
  - { from: left_counter.count, to: left_gain.audio_in }
  - { from: right_counter.count, to: right_gain.audio_in }
"#;
        let patch = load_kernel_patch_str(yaml).expect("script patch loads");

        let prepared = prepare_kernel_patch(&patch, &KERNEL_RENDER_SETTINGS)
            .expect("script-backed nodes prepare");

        for id in ["left_counter", "right_counter"] {
            let node = prepared
                .compiled_patch()
                .nodes()
                .iter()
                .find(|node| node.id.as_str() == id)
                .expect("compiled script node");
            assert_eq!(node.module_type, module_types::SCRIPT);
            assert_eq!(node.module_kind, ModuleKind::Script);
            assert!(matches!(
                &node.construction,
                CompiledConstruction::Script {
                    language: CompiledScriptLanguage::Rhai,
                    source,
                } if source.contains("state_get")
            ));
            assert_eq!(
                node.input_port_names,
                ["increment"],
                "the named definition supplies the runtime interface"
            );
            assert_eq!(node.output_port_names, ["count"]);
        }

        assert_eq!(
            patch.registry().get("counter").unwrap().static_params()[0].name(),
            SCRIPT_LANGUAGE_PARAMETER
        );
        assert_eq!(
            patch.registry().get("counter").unwrap().static_params()[0].default(),
            Some(&StaticValue::Enum(SCRIPT_LANGUAGE_RHAI.to_string()))
        );
        assert_eq!(
            patch.registry().get("counter").unwrap().static_params()[1].name(),
            SCRIPT_SOURCE_PARAMETER
        );

        let rendered =
            render_kernel_offline_named(&prepared, Vec::new(), &PreparedSamplerAssets::empty())
                .expect("script instances render named root buses offline");
        let realtime = RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
            prepared.graph().clone(),
            prepared.compiled_patch().clone(),
            KERNEL_RENDER_SETTINGS.sample_rate_hz as f32,
            &PreparedSamplerAssets::empty(),
            &patch::VoiceAllocation::default(),
            KERNEL_RENDER_SETTINGS.block_size_frames as usize,
        );
        assert!(realtime.can_render_root_buses_offline());
        assert!(!realtime.can_render_root_buses());
        assert_eq!(rendered.len(), 2);
        assert_eq!(rendered[0].0, builtin_ports::LEFT);
        assert_eq!(rendered[1].0, builtin_ports::RIGHT);
        let left = &rendered[0].1[0];
        let right = &rendered[1].1[0];
        assert_eq!(
            left, right,
            "repeated script instances retain disjoint state"
        );
        assert!(
            left[..KERNEL_RENDER_SETTINGS.block_size_frames as usize]
                .iter()
                .all(|sample| (*sample - 1.0).abs() <= f32::EPSILON),
            "each script instance starts its own counter at one"
        );
        assert!(
            left[KERNEL_RENDER_SETTINGS.block_size_frames as usize..]
                .iter()
                .all(|sample| (*sample - 2.0).abs() <= f32::EPSILON),
            "each script instance advances only its own counter"
        );
    }

    fn with_unit_impulse_response(node: Node) -> Node {
        node.with_static_arg(
            IMPULSE_RESPONSE_RESOURCE_PARAM,
            StaticArg::Literal(StaticValue::Resource(ResourceRef::new(
                ResourceKind::ImpulseResponse,
                UNIT_IR_PATH,
                ResourceOrigin::Document,
            ))),
        )
    }

    #[test]
    fn preparation_context_resolves_and_deduplicates_document_resources() {
        let root = resource_test_dir("deduplicates");
        fs::create_dir_all(&root).unwrap();
        let wav = root.join("hit.wav");
        crate::wav::write_wav_stereo_i16(fs::File::create(&wav).unwrap(), 48_000, &[0.25], &[0.25])
            .unwrap();
        let library_root = root.join("library");
        let context = PreparationContext::new(&root, 48_000)
            .with_macro_roots(MacroRoots::new().with_root(LIB_MACRO, &library_root));
        assert_eq!(context.sample_rate_hz(), 48_000);
        assert_eq!(
            context.macro_roots().root(LIB_MACRO),
            Some(library_root.as_path())
        );
        let reference = ResourceRef::new(ResourceKind::Sample, "hit.wav", ResourceOrigin::Document);
        let mut resolver = ResourceResolver::new(&context);

        let first = resolver.resolve(&reference).expect("first load succeeds");
        let second = resolver.resolve(&reference).expect("cached load succeeds");

        assert_eq!(first.kind(), ResourceKind::Sample);
        assert_eq!(first.canonical_path(), wav.canonicalize().unwrap());
        assert!((first.sample().frames()[0] - 0.25).abs() < 0.0001);
        assert!(first.shares_data_with(&second));
        assert_eq!(resolver.loaded_resource_count(), 1);
    }

    #[test]
    fn kernel_preparation_maps_shared_resources_to_namespaced_compiled_nodes() {
        let root_dir = resource_test_dir("compiled-handles");
        fs::create_dir_all(&root_dir).unwrap();
        crate::wav::write_wav_stereo_i16(
            fs::File::create(root_dir.join("hit.wav")).unwrap(),
            48_000,
            &[0.25, 0.5, 0.75],
            &[0.25, 0.5, 0.75],
        )
        .unwrap();
        let sample_ref = || {
            StaticArg::Literal(StaticValue::Resource(ResourceRef::new(
                ResourceKind::Sample,
                "hit.wav",
                ResourceOrigin::Document,
            )))
        };
        let layer = GraphDefinition::new("sample_layer")
            .with_static_param(crate::kernel::StaticParam::new(
                crate::kernel::builtins::SAMPLE_RESOURCE_PARAM,
                crate::kernel::StaticType::Resource(ResourceKind::Sample),
            ))
            .with_port(
                KernelPort::input("trigger", SignalType::Event, 1)
                    .maps_to(kernel_ref("sampler", builtin_ports::TRIGGER)),
            )
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("sampler", builtin_ports::AUDIO)),
            )
            .with_node(
                Node::new(NodeId::new("sampler"), module_types::SAMPLER).with_static_arg(
                    crate::kernel::builtins::SAMPLE_RESOURCE_PARAM,
                    StaticArg::ParamRef(crate::kernel::builtins::SAMPLE_RESOURCE_PARAM.to_string()),
                ),
            );
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("left", SignalType::Audio, 1)
                    .maps_from(kernel_ref("first", "audio")),
            )
            .with_port(
                KernelPort::output("right", SignalType::Audio, 1)
                    .maps_from(kernel_ref("second", "audio")),
            )
            .with_node(Node::new(NodeId::new("midi"), module_types::MIDI_INPUT))
            .with_node(
                Node::new(NodeId::new("first_filter"), module_types::EVENT_FILTER).with_static_arg(
                    crate::builtins::EVENT_FILTER_NOTE_PARAMETER,
                    StaticArg::Literal(StaticValue::Int(60)),
                ),
            )
            .with_node(
                Node::new(NodeId::new("second_filter"), module_types::EVENT_FILTER)
                    .with_static_arg(
                        crate::builtins::EVENT_FILTER_NOTE_PARAMETER,
                        StaticArg::Literal(StaticValue::Int(61)),
                    ),
            )
            .with_node(
                Node::new(NodeId::new("first"), "sample_layer")
                    .with_static_arg(crate::kernel::builtins::SAMPLE_RESOURCE_PARAM, sample_ref()),
            )
            .with_node(
                Node::new(NodeId::new("second"), "sample_layer")
                    .with_static_arg(crate::kernel::builtins::SAMPLE_RESOURCE_PARAM, sample_ref()),
            )
            .with_connection(Connection::new(
                kernel_ref("midi", builtin_ports::EVENTS),
                kernel_ref("first_filter", builtin_ports::EVENTS_IN),
            ))
            .with_connection(Connection::new(
                kernel_ref("midi", builtin_ports::EVENTS),
                kernel_ref("second_filter", builtin_ports::EVENTS_IN),
            ))
            .with_connection(Connection::new(
                kernel_ref("first_filter", builtin_ports::EVENTS_OUT),
                kernel_ref("first", "trigger"),
            ))
            .with_connection(Connection::new(
                kernel_ref("second_filter", builtin_ports::EVENTS_OUT),
                kernel_ref("second", "trigger"),
            ));
        let registry = builtin_registry().with_definition(layer);
        let context = PreparationContext::new(&root_dir, 48_000);

        let prepared = prepare_kernel_graph_with_buses_and_context(
            &root,
            &registry,
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new()
                .with_output("left", 1)
                .with_output("right", 1),
            &context,
        )
        .expect("resource-backed defined-module instances should prepare");
        let first = prepared
            .compiled_patch()
            .nodes()
            .iter()
            .find(|node| node.id.as_str() == "first::sampler")
            .unwrap()
            .resources
            .sample
            .as_ref()
            .unwrap();
        let second = prepared
            .compiled_patch()
            .nodes()
            .iter()
            .find(|node| node.id.as_str() == "second::sampler")
            .unwrap()
            .resources
            .sample
            .as_ref()
            .unwrap();

        assert!(first.shares_data_with(second));
        assert_eq!(first.frames().len(), 3);

        let mut runtime = RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
            prepared.graph().clone(),
            prepared.compiled_patch().clone(),
            48_000.0,
            &PreparedSamplerAssets::empty(),
            &crate::patch::VoiceAllocation::default(),
            8,
        );
        runtime.note_on_at(60, 100, 0);
        runtime.note_on_at(61, 100, 2);
        let mut left = [0.0; 8];
        let mut right = [0.0; 8];
        assert_eq!(
            render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
            8
        );
        assert!((left[0] - 0.25).abs() < 0.0001);
        assert!((left[1] - 0.5).abs() < 0.0001);
        assert!((left[2] - 0.75).abs() < 0.0001);
        assert_eq!(left[3], 0.0);
        assert_eq!(right[0], 0.0);
        assert_eq!(right[1], 0.0);
        assert!((right[2] - 0.25).abs() < 0.0001);
        assert!((right[3] - 0.5).abs() < 0.0001);
        assert!((right[4] - 0.75).abs() < 0.0001);
    }

    #[test]
    fn resource_resolution_rejects_escape_and_sample_rate_mismatch() {
        let root = resource_test_dir("containment");
        fs::create_dir_all(&root).unwrap();
        let wrong_rate = root.join("wrong-rate.wav");
        crate::wav::write_wav_stereo_i16(
            fs::File::create(&wrong_rate).unwrap(),
            44_100,
            &[0.25],
            &[0.25],
        )
        .unwrap();
        let context = PreparationContext::new(&root, 48_000);
        let mut resolver = ResourceResolver::new(&context);

        for path in [PathBuf::from("../outside.wav"), root.join("absolute.wav")] {
            let error = resolver
                .resolve(&ResourceRef::new(
                    ResourceKind::Sample,
                    path,
                    ResourceOrigin::Document,
                ))
                .expect_err("escaping path must fail");
            assert_eq!(
                error.errors().next().unwrap().error_code(),
                diagnostics::error_codes::KERNEL_RESOURCE_PATH_ESCAPE
            );
        }

        let error = resolver
            .resolve(&ResourceRef::new(
                ResourceKind::Sample,
                "wrong-rate.wav",
                ResourceOrigin::Document,
            ))
            .expect_err("sample-rate mismatch must fail");
        assert_eq!(
            error.errors().next().unwrap().error_code(),
            diagnostics::error_codes::KERNEL_RESOURCE_LOAD_FAILED
        );
    }

    #[cfg(unix)]
    #[test]
    fn resource_resolution_rejects_symlink_escape() {
        let root = resource_test_dir("symlink-root");
        let outside = resource_test_dir("symlink-outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let outside_wav = outside.join("outside.wav");
        crate::wav::write_wav_stereo_i16(
            fs::File::create(&outside_wav).unwrap(),
            48_000,
            &[0.25],
            &[0.25],
        )
        .unwrap();
        std::os::unix::fs::symlink(&outside_wav, root.join("linked.wav")).unwrap();
        let context = PreparationContext::new(&root, 48_000);
        let mut resolver = ResourceResolver::new(&context);

        let error = resolver
            .resolve(&ResourceRef::new(
                ResourceKind::Sample,
                "linked.wav",
                ResourceOrigin::Document,
            ))
            .expect_err("canonical target outside root must fail");

        assert_eq!(
            error.errors().next().unwrap().error_code(),
            diagnostics::error_codes::KERNEL_RESOURCE_PATH_ESCAPE
        );
    }

    #[cfg(unix)]
    #[test]
    fn package_resource_resolution_rejects_symlink_and_parent_escapes() {
        let document_root = resource_test_dir("package-document");
        let package_root = resource_test_dir("package-root");
        let outside = resource_test_dir("package-outside");
        fs::create_dir_all(&document_root).unwrap();
        fs::create_dir_all(&package_root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let outside_wav = outside.join("outside.wav");
        crate::wav::write_wav_stereo_i16(
            fs::File::create(&outside_wav).unwrap(),
            48_000,
            &[0.25],
            &[0.25],
        )
        .unwrap();
        std::os::unix::fs::symlink(&outside_wav, package_root.join("linked.wav")).unwrap();
        let context = PreparationContext::new(&document_root, 48_000);
        let mut resolver = ResourceResolver::new(&context);

        for path in [
            PathBuf::from("linked.wav"),
            PathBuf::from("../outside.wav"),
            outside_wav,
        ] {
            let error = resolver
                .resolve(&ResourceRef::new(
                    ResourceKind::Sample,
                    path,
                    ResourceOrigin::Package(package_root.clone()),
                ))
                .expect_err("package resource must stay under its canonical root");
            assert_eq!(
                error.errors().next().unwrap().error_code(),
                diagnostics::error_codes::KERNEL_RESOURCE_PATH_ESCAPE
            );
        }
    }

    fn resource_test_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "dandrum-preparation-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

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
    fn preparation_pipeline_compiles_resolved_filter_parameters_into_typed_state() {
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
            filt.construction,
            CompiledConstruction::Filter {
                algorithm: crate::compiled_patch::CompiledFilterAlgorithm::Biquad(
                    crate::filter::BiquadMode::Highpass,
                ),
            }
        );
        assert_eq!(
            resolved["filt"]["comb_type"],
            ParameterValue::Text("feedback".to_string())
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

        assert!(
            prepared
                .graph()
                .modules()
                .iter()
                .all(|node| node.params().is_empty()),
            "kernel graph modules carry typed construction and defaults without legacy parameter maps"
        );

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
        assert_eq!(
            osc.construction,
            crate::compiled_patch::CompiledConstruction::Oscillator {
                waveform: crate::oscillator::Waveform::Sine,
            }
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
        assert_eq!(prepared.compiled_patch().audio_output_index(), None);
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
        render_two_mono_root_ports(&mut realtime, &mut left, &mut right);
        assert!(
            left.iter().any(|sample| sample.abs() > f32::EPSILON),
            "the typed gain default reaches the realtime arena path"
        );
        assert!(realtime.set_numeric_parameter_by_target("layer::voice::amp", "gain", 0.0));
        render_two_mono_root_ports(&mut realtime, &mut left, &mut right);
        assert!(
            left.iter().all(|sample| sample.abs() <= f32::EPSILON),
            "the arena reads the current typed control slot each block"
        );
    }

    #[test]
    fn kernel_preparation_does_not_inherit_undeclared_legacy_control_defaults() {
        let gain = GraphDefinition::new(module_types::GAIN)
            .with_latency(crate::kernel::LatencySpec::Zero)
            .with_port(KernelPort::input(
                builtin_ports::AUDIO_IN,
                SignalType::Audio,
                1,
            ))
            .with_port(KernelPort::input(
                builtin_ports::GAIN,
                SignalType::Control,
                1,
            ))
            .with_port(KernelPort::output(
                builtin_ports::AUDIO_OUT,
                SignalType::Audio,
                1,
            ));
        let registry = DefinitionRegistry::new().with_definition(gain);
        let root = GraphDefinition::new("default-source")
            .with_port(
                KernelPort::output("master", SignalType::Audio, 1)
                    .maps_from(kernel_ref("amp", builtin_ports::AUDIO_OUT)),
            )
            .with_node(Node::new(NodeId::new("amp"), module_types::GAIN));

        let prepared = prepare_kernel_graph(&root, &registry, &KERNEL_RENDER_SETTINGS)
            .expect("primitive without a declared control default prepares");

        assert_eq!(
            prepared
                .compiled_patch()
                .numeric_parameter_value("amp", builtin_ports::GAIN),
            Some(0.0),
            "the kernel definition did not declare the legacy gain default"
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

        assert_eq!(source.module_kind, ModuleKind::Noise);
        assert_eq!(gain.module_kind, ModuleKind::Gain);
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
    fn kernel_render_uses_declared_control_default_override_and_connected_value() {
        let render_first = |override_gain: Option<f64>, connect_control: bool| {
            let mut gain = Node::new(NodeId::new("amp"), module_types::GAIN);
            if let Some(value) = override_gain {
                gain = gain.with_default_override(builtin_ports::GAIN, value);
            }
            let mut root = GraphDefinition::new("root")
                .with_port(
                    KernelPort::input("input", SignalType::Audio, 1)
                        .maps_to(kernel_ref("amp", builtin_ports::AUDIO_IN)),
                )
                .with_port(
                    KernelPort::output("master", SignalType::Audio, 1)
                        .maps_from(kernel_ref("amp", builtin_ports::AUDIO_OUT)),
                )
                .with_node(gain);
            if connect_control {
                root = root
                    .with_node(Node::new(NodeId::new("modulation"), module_types::LFO))
                    .with_connection(Connection::new(
                        kernel_ref("modulation", builtin_ports::VALUE),
                        kernel_ref("amp", builtin_ports::GAIN),
                    ));
            }
            let prepared = prepare_kernel_graph_with_buses(
                &root,
                &builtin_registry(),
                &KERNEL_RENDER_SETTINGS,
                &HostBuses::new()
                    .with_input("input", 1)
                    .with_output("master", 1),
            )
            .expect("gain patch prepares");
            let mut runtime = RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
                prepared.graph().clone(),
                prepared.compiled_patch().clone(),
                KERNEL_RENDER_SETTINGS.sample_rate_hz as f32,
                &PreparedSamplerAssets::empty(),
                &crate::patch::VoiceAllocation::default(),
                8,
            );
            let inputs = vec![vec![vec![-0.5; 8]]];
            let mut outputs = vec![vec![vec![0.0; 8]]];
            assert_eq!(runtime.render_root_buses(&inputs, &mut outputs), 8);
            outputs[0][0][0]
        };

        assert_eq!(render_first(None, false), -0.5);
        assert_eq!(render_first(Some(0.25), false), -0.125);
        assert_eq!(render_first(Some(0.25), true), -0.25);
    }

    #[test]
    fn echo_and_reverb_render_mono_and_stereo_channel_spans() {
        for effect in [module_types::ECHO, module_types::REVERB] {
            for channels in [1_i64, 2] {
                let root = GraphDefinition::new(format!("{effect}-{channels}"))
                    .with_port(
                        KernelPort::input("input", SignalType::Audio, channels as u32)
                            .maps_to(kernel_ref("effect", builtin_ports::AUDIO_IN)),
                    )
                    .with_port(
                        KernelPort::output("master", SignalType::Audio, channels as u32)
                            .maps_from(kernel_ref("effect", builtin_ports::AUDIO_OUT)),
                    )
                    .with_node(
                        Node::new(NodeId::new("effect"), effect)
                            .with_static_arg(
                                crate::kernel::builtins::CHANNELS_PARAM,
                                StaticArg::Literal(StaticValue::Int(channels)),
                            )
                            .with_default_override(builtin_ports::WET, 0.0)
                            .with_default_override(builtin_ports::DRY, 1.0),
                    );
                let prepared = prepare_kernel_graph_with_buses(
                    &root,
                    &builtin_registry(),
                    &KERNEL_RENDER_SETTINGS,
                    &HostBuses::new()
                        .with_input("input", channels as usize)
                        .with_output("master", channels as usize),
                )
                .unwrap_or_else(|error| {
                    panic!("{effect} should prepare with {channels} channels: {error}")
                });
                let node = &prepared.compiled_patch().nodes()[0];
                assert_eq!(node.input_port_spans[0].channel_count, channels as usize);
                assert_eq!(node.output_port_spans[0].channel_count, channels as usize);

                let mut runtime = RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
                    prepared.graph().clone(),
                    prepared.compiled_patch().clone(),
                    KERNEL_RENDER_SETTINGS.sample_rate_hz as f32,
                    &PreparedSamplerAssets::empty(),
                    &crate::patch::VoiceAllocation::default(),
                    8,
                );
                let inputs = vec![
                    (0..channels)
                        .map(|channel| vec![0.25 + channel as f32 * 0.25; 8])
                        .collect::<Vec<_>>(),
                ];
                let mut outputs = vec![vec![vec![0.0; 8]; channels as usize]];

                assert_eq!(runtime.render_root_buses(&inputs, &mut outputs), 8);
                assert_eq!(
                    outputs, inputs,
                    "{effect} dry signal preserves channel order"
                );
            }
        }
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
            .with_node(with_unit_impulse_response(
                Node::new(NodeId::new("convolution"), module_types::CONVOLUTION).with_static_arg(
                    crate::kernel::builtins::CHANNELS_PARAM,
                    StaticArg::Literal(StaticValue::Int(6)),
                ),
            ));
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
        assert_eq!(prepared.compiled_patch().audio_output_index(), None);
    }

    #[test]
    fn prepared_discovery_reports_inserted_compensation_locations_and_samples() {
        let (root, registry) = latency_test_graph();
        let prepared = prepare_kernel_graph(&root, &registry, &KERNEL_RENDER_SETTINGS)
            .expect("latency graph prepares");

        let reports = prepared.compensation_metadata();
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].region_path(), None);
        assert_eq!(reports[0].source().node().as_str(), "impulse");
        assert_eq!(reports[0].source().port(), builtin_ports::AUDIO);
        assert_eq!(reports[0].destination().unwrap().node().as_str(), "mix");
        assert_eq!(
            reports[0].destination().unwrap().port(),
            builtin_ports::INPUTS
        );
        assert_eq!(reports[0].root_port(), None);
        assert_eq!(reports[0].samples(), 1);
        assert_eq!(reports[1].source().node().as_str(), "impulse");
        assert_eq!(reports[1].destination(), None);
        assert_eq!(reports[1].root_port(), Some("right"));
        assert_eq!(reports[1].samples(), 1);
    }

    fn compensated_poly_voice() -> GraphDefinition {
        GraphDefinition::new("delayed_voice")
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("mix", builtin_ports::MIX)),
            )
            .with_node(
                Node::new(NodeId::new("constant"), module_types::CONTROL_TO_AUDIO)
                    .with_default_override(builtin_ports::IN, 0.25),
            )
            .with_node(
                Node::new(NodeId::new("wet"), module_types::COMPENSATION_DELAY).with_static_arg(
                    DELAY_SAMPLES_PARAMETER,
                    StaticArg::Literal(StaticValue::Int(1)),
                ),
            )
            .with_node(Node::new(NodeId::new("mix"), module_types::AUDIO_MIXER))
            .with_connection(Connection::new(
                kernel_ref("constant", builtin_ports::OUT),
                kernel_ref("wet", builtin_ports::AUDIO_IN),
            ))
            .with_connection(Connection::new(
                kernel_ref("constant", builtin_ports::OUT),
                kernel_ref("mix", builtin_ports::INPUTS),
            ))
            .with_connection(Connection::new(
                kernel_ref("wet", builtin_ports::AUDIO_OUT),
                kernel_ref("mix", builtin_ports::INPUTS),
            ))
    }

    #[test]
    fn prepared_discovery_reports_compensation_inside_a_poly_voice() {
        let prepared = prepare_audio_poly(compensated_poly_voice(), 1);

        let reports = prepared.compensation_metadata();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].region_path(), Some("voices"));
        assert_eq!(reports[0].source().node().as_str(), "constant");
        assert_eq!(reports[0].destination().unwrap().node().as_str(), "mix");
        assert_eq!(reports[0].samples(), 1);

        let mut runtime = runtime_for(&prepared);
        runtime.note_on(60, 100);
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let mut left = vec![0.0; frames];
        let mut right = vec![0.0; frames];
        assert_eq!(
            render_two_mono_root_ports(&mut runtime, &mut left, &mut right),
            frames
        );
        assert_eq!(left[0], 0.0);
        assert_eq!(left[1], 0.5);
        assert_eq!(right[1], 0.5);
    }

    #[test]
    fn prepared_discovery_qualifies_compensation_inside_nested_poly_regions() {
        let outer = GraphDefinition::new("outer_voice")
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("inner_voices", "audio")),
            )
            .with_node(poly_node("inner_voices", "delayed_voice", 1))
            .with_connection(Connection::new(
                kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_GATE_OUTPUT,
                ),
                kernel_ref("inner_voices", "notes"),
            ));
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("master", SignalType::Audio, 1)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_node(poly_node("voices", "outer_voice", 1));
        let registry = builtin_registry()
            .with_definition(compensated_poly_voice())
            .with_definition(outer);
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &registry,
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new().with_output("master", 1),
        )
        .expect("nested compensated poly graph prepares");

        let reports = prepared.compensation_metadata();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].region_path(), Some("voices::inner_voices"));
        assert_eq!(reports[0].source().node().as_str(), "constant");
        assert_eq!(reports[0].destination().unwrap().node().as_str(), "mix");
        assert_eq!(reports[0].samples(), 1);

        let mut runtime = runtime_for(&prepared);
        runtime.note_on(60, 100);
        let frames = KERNEL_RENDER_SETTINGS.block_size_frames as usize;
        let mut outputs = vec![vec![vec![0.0; frames]]];
        assert_eq!(runtime.render_root_outputs(&mut outputs), frames);
        assert_eq!(outputs[0][0][0], 0.0);
        assert_eq!(outputs[0][0][1], 0.5);
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

        let (offline_left, offline_right) =
            render_named_stereo(&prepared, events, &PreparedSamplerAssets::empty());
        assert_eq!(&offline_left[..3], &[0.0, 2.0, 0.0]);
        assert_eq!(&offline_right[..3], &[0.0, 1.0, 0.0]);

        let mut realtime = RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
            prepared.graph().clone(),
            prepared.compiled_patch().clone(),
            KERNEL_RENDER_SETTINGS.sample_rate_hz as f32,
            &PreparedSamplerAssets::empty(),
            &patch::VoiceAllocation::default(),
            KERNEL_RENDER_SETTINGS.block_size_frames as usize,
        );
        realtime.note_on(60, 100);
        let mut realtime_left = Vec::new();
        let mut realtime_right = Vec::new();
        for _ in 0..KERNEL_RENDER_SETTINGS.duration_frames
            / KERNEL_RENDER_SETTINGS.block_size_frames as u64
        {
            let mut outputs =
                vec![vec![vec![0.0; KERNEL_RENDER_SETTINGS.block_size_frames as usize]]; 2];
            assert_eq!(
                realtime.render_root_outputs(&mut outputs),
                KERNEL_RENDER_SETTINGS.block_size_frames as usize
            );
            realtime_left.extend_from_slice(&outputs[0][0]);
            realtime_right.extend_from_slice(&outputs[1][0]);
        }

        assert_eq!(realtime_left, offline_left);
        assert_eq!(realtime_right, offline_right);
    }

    #[test]
    fn kernel_convolution_dry_and_wet_paths_render_time_aligned() {
        let (root, registry) = latency_builtin_render_graph(with_unit_impulse_response(Node::new(
            NodeId::new(WET_NODE_ID),
            module_types::CONVOLUTION,
        )));
        let expected_frame = Convolution::BLOCK_SIZE;
        let settings = latency_render_settings(expected_frame + 2);
        let prepared = prepare_kernel_graph(&root, &registry, &settings)
            .expect("convolution latency graph should prepare");
        let assets = PreparedSamplerAssets::from_samples_by_module(BTreeMap::from([(
            WET_NODE_ID.to_string(),
            LoadedSample::new(settings.sample_rate_hz, vec![1.0]),
        )]));

        let (left, right) = render_named_stereo(&prepared, vec![note_on_at(0)], &assets);

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

            let (left, right) = render_named_stereo(
                &prepared,
                vec![note_on_at(trigger_frame as u64)],
                &PreparedSamplerAssets::empty(),
            );

            assert_aligned_impulse(&left, expected_frame);
            assert_aligned_impulse(&right, expected_frame);
        }
    }

    #[test]
    fn kernel_spectral_named_root_render_allocates_nothing_after_preparation() {
        let wet = Node::new(NodeId::new(WET_NODE_ID), module_types::SPECTRAL_PROCESSOR)
            .with_static_arg(
                SPECTRAL_FFT_SIZE_PARAMETER,
                StaticArg::Literal(StaticValue::Int(512)),
            )
            .with_static_arg(
                SPECTRAL_MODE_PARAMETER,
                StaticArg::Literal(StaticValue::Enum(SPECTRAL_MODE_PASSTHROUGH.to_string())),
            );
        let (root, registry) = latency_builtin_render_graph(wet);
        let settings = latency_render_settings(512);
        let prepared = prepare_kernel_graph(&root, &registry, &settings).unwrap();
        let mut realtime = RealtimeGraphProcessor::polyphonic_with_compiled_patch_and_sampler_assets_and_max_block_size(
            prepared.graph().clone(),
            prepared.compiled_patch().clone(),
            settings.sample_rate_hz as f32,
            &PreparedSamplerAssets::empty(),
            &patch::VoiceAllocation::default(),
            settings.block_size_frames as usize,
        );
        let mut outputs = vec![vec![vec![0.0; settings.block_size_frames as usize]]; 2];
        realtime.note_on(60, 100);
        let allocations = count_current_thread_allocations(|| {
            for _ in 0..settings.duration_frames / settings.block_size_frames as u64 {
                assert_eq!(
                    realtime.render_root_outputs(&mut outputs),
                    settings.block_size_frames as usize
                );
            }
        });
        assert_eq!(
            allocations, 0,
            "spectral FFT plans and scratch belong to preparation"
        );
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

    fn render_named_stereo(
        prepared: &PreparedKernelInstrument,
        events: Vec<TimedInputEvent>,
        assets: &PreparedSamplerAssets,
    ) -> (Vec<f32>, Vec<f32>) {
        let rendered = render_kernel_offline_named(prepared, events, assets)
            .expect("latency graph renders named root buses");
        assert_eq!(rendered.len(), 2);
        assert_eq!(rendered[0].0, builtin_ports::LEFT);
        assert_eq!(rendered[1].0, builtin_ports::RIGHT);
        assert_eq!(rendered[0].1.len(), 1);
        assert_eq!(rendered[1].1.len(), 1);
        (rendered[0].1[0].clone(), rendered[1].1[0].clone())
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

    fn delayed_poly_voice(name: &str) -> GraphDefinition {
        GraphDefinition::new(name)
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("delay", builtin_ports::AUDIO_OUT)),
            )
            .with_node(
                Node::new(NodeId::new("source"), module_types::CONTROL_TO_AUDIO)
                    .with_default_override(builtin_ports::IN, 0.5),
            )
            .with_node(
                Node::new(NodeId::new("delay"), module_types::COMPENSATION_DELAY).with_static_arg(
                    DELAY_SAMPLES_PARAMETER,
                    StaticArg::Literal(StaticValue::Int(3)),
                ),
            )
            .with_connection(Connection::new(
                kernel_ref("source", builtin_ports::OUT),
                kernel_ref("delay", builtin_ports::AUDIO_IN),
            ))
    }

    fn assert_poly_latency_aligns_parent_outputs(voice_name: &str, registry: &DefinitionRegistry) {
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("dry", SignalType::Audio, 1)
                    .maps_from(kernel_ref("dry_source", builtin_ports::OUT)),
            )
            .with_port(
                KernelPort::output("wet", SignalType::Audio, 1)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_node(
                Node::new(NodeId::new("dry_source"), module_types::CONTROL_TO_AUDIO)
                    .with_default_override(builtin_ports::IN, 0.25),
            )
            .with_node(poly_node("voices", voice_name, 1));
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            registry,
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new().with_output("dry", 1).with_output("wet", 1),
        )
        .expect("delayed poly graph prepares");
        assert_eq!(prepared.total_latency_samples(), 3);
        assert_eq!(prepared.latency_plan().root_compensations().len(), 1);
        let mut runtime = runtime_for(&prepared);
        let mut outputs = vec![vec![vec![0.0; 8]]; 2];
        runtime.note_on(60, 100);
        assert_eq!(runtime.render_root_outputs(&mut outputs), 8);
        assert_eq!(outputs[0][0], [0.0, 0.0, 0.0, 0.25, 0.25, 0.25, 0.25, 0.25]);
        assert_eq!(outputs[1][0], [0.0, 0.0, 0.0, 0.5, 0.5, 0.5, 0.5, 0.5]);
    }

    #[test]
    fn delayed_poly_voice_latency_aligns_parent_dry_and_wet_outputs() {
        let voice = delayed_poly_voice("delayed_voice");
        assert_poly_latency_aligns_parent_outputs(
            voice.name(),
            &builtin_registry().with_definition(voice.clone()),
        );
    }

    #[test]
    fn defined_module_latency_aligns_parent_dry_and_wet_outputs() {
        let delayed = GraphDefinition::new("delayed_module")
            .with_port(
                KernelPort::input("audio_in", SignalType::Audio, 1)
                    .maps_to(kernel_ref("delay", builtin_ports::AUDIO_IN)),
            )
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("delay", builtin_ports::AUDIO_OUT)),
            )
            .with_node(
                Node::new(NodeId::new("delay"), module_types::COMPENSATION_DELAY).with_static_arg(
                    DELAY_SAMPLES_PARAMETER,
                    StaticArg::Literal(StaticValue::Int(3)),
                ),
            );
        let root = GraphDefinition::new("root")
            .with_port(
                KernelPort::output("dry", SignalType::Audio, 1)
                    .maps_from(kernel_ref("source", builtin_ports::OUT)),
            )
            .with_port(
                KernelPort::output("wet", SignalType::Audio, 1)
                    .maps_from(kernel_ref("wet_module", "audio")),
            )
            .with_node(
                Node::new(NodeId::new("source"), module_types::CONTROL_TO_AUDIO)
                    .with_default_override(builtin_ports::IN, 0.25),
            )
            .with_node(Node::new(NodeId::new("wet_module"), "delayed_module"))
            .with_connection(Connection::new(
                kernel_ref("source", builtin_ports::OUT),
                kernel_ref("wet_module", "audio_in"),
            ));
        let prepared = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry().with_definition(delayed),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new().with_output("dry", 1).with_output("wet", 1),
        )
        .expect("delayed defined module prepares");
        assert_eq!(prepared.total_latency_samples(), 3);
        assert_eq!(prepared.latency_plan().root_compensations().len(), 1);

        let mut runtime = runtime_for(&prepared);
        let mut outputs = vec![vec![vec![0.0; 8]]; 2];
        assert_eq!(runtime.render_root_outputs(&mut outputs), 8);
        assert_eq!(outputs[0][0], [0.0, 0.0, 0.0, 0.25, 0.25, 0.25, 0.25, 0.25]);
        assert_eq!(outputs[1][0], [0.0, 0.0, 0.0, 0.25, 0.25, 0.25, 0.25, 0.25]);
    }

    #[test]
    fn nested_delayed_poly_voice_latency_aligns_parent_dry_and_wet_outputs() {
        let inner = delayed_poly_voice("delayed_inner");
        let outer = GraphDefinition::new("delayed_outer")
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("inner_voices", "audio")),
            )
            .with_node(poly_node("inner_voices", inner.name(), 1))
            .with_connection(Connection::new(
                kernel_ref(
                    crate::kernel::VOICE_INTRINSIC_NODE,
                    crate::kernel::VOICE_GATE_OUTPUT,
                ),
                kernel_ref("inner_voices", "notes"),
            ));
        assert_poly_latency_aligns_parent_outputs(
            outer.name(),
            &builtin_registry()
                .with_definition(inner)
                .with_definition(outer.clone()),
        );
    }

    #[test]
    fn delayed_poly_boundary_in_feedback_cycle_is_rejected() {
        let voice = GraphDefinition::new("delayed_feedback_voice")
            .with_port(
                KernelPort::input("audio_in", SignalType::Audio, 1)
                    .maps_to(kernel_ref("delay", builtin_ports::AUDIO_IN)),
            )
            .with_port(
                KernelPort::output("audio", SignalType::Audio, 1)
                    .maps_from(kernel_ref("delay", builtin_ports::AUDIO_OUT)),
            )
            .with_node(
                Node::new(NodeId::new("delay"), module_types::COMPENSATION_DELAY).with_static_arg(
                    DELAY_SAMPLES_PARAMETER,
                    StaticArg::Literal(StaticValue::Int(3)),
                ),
            );
        let root = GraphDefinition::new("feedback_root")
            .with_port(
                KernelPort::output("master", SignalType::Audio, 1)
                    .maps_from(kernel_ref("voices", "audio")),
            )
            .with_node(poly_node("voices", voice.name(), 1))
            .with_node(
                Node::new(NodeId::new("feedback"), module_types::FEEDBACK_DELAY).with_static_arg(
                    DELAY_SAMPLES_PARAMETER,
                    StaticArg::Literal(StaticValue::Int(8)),
                ),
            )
            .with_connection(Connection::new(
                kernel_ref("voices", "audio"),
                kernel_ref("feedback", builtin_ports::AUDIO_IN),
            ))
            .with_connection(Connection::new(
                kernel_ref("feedback", builtin_ports::AUDIO_OUT),
                kernel_ref("voices", "audio_in"),
            ));
        let error = prepare_kernel_graph_with_buses(
            &root,
            &builtin_registry().with_definition(voice),
            &KERNEL_RENDER_SETTINGS,
            &HostBuses::new().with_output("master", 1),
        )
        .expect_err("a delayed poly region cannot be inside a feedback cycle");
        assert_eq!(
            error.diagnostics().errors().next().unwrap().error_code(),
            diagnostics::error_codes::KERNEL_LATENCY_IN_FEEDBACK_CYCLE
        );
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
