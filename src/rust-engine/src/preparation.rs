use std::fmt;
use std::path::Path;

use crate::compiled_patch::{self, CompileError, CompiledPatch};
use crate::graph::Graph;
use crate::patch::{self, PatchDocument};
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
    graph: Graph,
    compiled_patch: CompiledPatch,
    sampler_assets: PreparedSamplerAssets,
    diagnostics: PreparationDiagnostics,
}

impl PreparedInstrument {
    pub(crate) fn new(
        patch_doc: PatchDocument,
        graph: Graph,
        compiled_patch: CompiledPatch,
        sampler_assets: PreparedSamplerAssets,
        diagnostics: PreparationDiagnostics,
    ) -> Self {
        Self {
            patch_doc,
            graph,
            compiled_patch,
            sampler_assets,
            diagnostics,
        }
    }

    pub(crate) fn patch_doc(&self) -> &PatchDocument {
        &self.patch_doc
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
    validate_patch_document(&patch_doc)?;
    let graph = build_validated_graph(&patch_doc)?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let sampler_assets = prepare_assets(&patch_doc, base_dir)?;
    let compiled_patch = compile_patch(&graph, &patch_doc)?;

    Ok(PreparedInstrument::new(
        patch_doc,
        graph,
        compiled_patch,
        sampler_assets,
        PreparationDiagnostics::default(),
    ))
}

pub(crate) fn load_patch_document(path: impl AsRef<Path>) -> Result<PatchDocument, PreparationError> {
    patch::load_patch_file(path).map_err(PreparationError::Load)
}

pub(crate) fn validate_patch_document(
    patch_doc: &PatchDocument,
) -> Result<(), PreparationError> {
    patch::validate_patch_schema(patch_doc).map_err(PreparationError::Schema)
}

pub(crate) fn build_validated_graph(patch_doc: &PatchDocument) -> Result<Graph, PreparationError> {
    let graph = Graph::from_patch_declarations(patch_doc);
    graph.validate().map_err(PreparationError::Graph)?;
    Ok(graph)
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

    #[test]
    fn prepared_instrument_owns_validated_patch_graph_compiled_patch_assets_and_diagnostics() {
        let patch_doc = patch::load_patch_str(MINIMAL_PATCH).expect("patch should parse");
        patch::validate_patch_schema(&patch_doc).expect("patch schema should validate");
        let graph = Graph::from_patch_declarations(&patch_doc);
        graph.validate().expect("graph should validate");
        let compiled_patch =
            compiled_patch::compile(&graph, &patch_doc.render).expect("graph should compile");

        let prepared = PreparedInstrument::new(
            patch_doc,
            graph,
            compiled_patch,
            PreparedSamplerAssets::empty(),
            PreparationDiagnostics::default(),
        );

        assert_eq!(prepared.patch_doc().metadata.name, "Prepared Instrument");
        assert_eq!(prepared.graph().modules().len(), 1);
        assert_eq!(prepared.compiled_patch().nodes().len(), 1);
        assert_eq!(
            prepared.compiled_patch().render_settings().sample_rate_hz,
            48_000
        );
        assert_eq!(prepared.sampler_assets(), &PreparedSamplerAssets::empty());
        assert!(prepared.diagnostics().messages().is_empty());
    }

    #[test]
    fn prepare_instrument_file_runs_explicit_pipeline_and_returns_prepared_instrument() {
        let temp_dir = std::env::temp_dir().join(format!(
            "dandrum-preparation-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&temp_dir).expect("temp directory should be created");
        let patch_path = temp_dir.join("patch.yaml");
        fs::write(&patch_path, MINIMAL_PATCH).expect("patch file should be written");

        let prepared = prepare_instrument_file(&patch_path).expect("patch should prepare");

        assert_eq!(prepared.patch_doc().metadata.name, "Prepared Instrument");
        assert_eq!(prepared.graph().modules().len(), 1);
        assert_eq!(prepared.compiled_patch().nodes().len(), 1);
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
    }
}
