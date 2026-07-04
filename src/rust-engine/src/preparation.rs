use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use crate::compiled_patch::{self, CompileError, CompiledPatch};
use crate::diagnostics::{self, Diagnostic, Severity};
use crate::graph::Graph;
use crate::patch::{self, ParameterValue, PatchDocument, PresetDocument};
use crate::sample::{self, PreparedSamplerAssets, SampleLoadError};

#[derive(Debug)]
pub(crate) enum PreparationError {
    Load(patch::PatchLoadError),
    Schema(patch::PatchValidationError),
    Graph(crate::graph::GraphValidationError),
    Assets(SampleLoadError),
    Compile(CompileError),
}

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

    pub(crate) fn diagnostics(&self) -> &PreparationDiagnostics {
        &self.diagnostics
    }
}

impl PreparationDiagnostics {
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
    use crate::patch;
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
}
