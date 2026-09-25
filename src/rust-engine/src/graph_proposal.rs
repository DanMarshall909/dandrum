use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::sound_matching::SoundMatchManifest;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphProposalCapabilities {
    pub structured_output: bool,
    pub cancellation: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GraphProposalRequest {
    pub version: u32,
    pub goal: String,
    pub residual: GraphProposalResidual,
    pub features: GraphProposalFeatureComparison,
    pub allowed_modules: Vec<String>,
    pub current_topology: GraphTopologySummary,
    pub constraints: Vec<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GraphProposalFeatureComparison {
    pub reference: GraphProposalSoundFeatures,
    pub candidate: GraphProposalSoundFeatures,
    pub delta: GraphProposalFeatureDelta,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GraphProposalSoundFeatures {
    pub frame_count: usize,
    pub centroid_frame_count: usize,
    pub mean_rms: f64,
    pub max_peak: f64,
    pub mean_spectral_centroid_hz: f64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GraphProposalFeatureDelta {
    pub rms_db: f64,
    pub peak_db: f64,
    pub centroid_octaves: f64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GraphProposalResidual {
    pub total: f64,
    pub spectral: f64,
    pub rms: f64,
    pub centroid: f64,
    pub candidate_gain: f64,
    pub completed_evaluations: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GraphTopologySummary {
    pub modules: Vec<GraphTopologyModule>,
    pub connections: Vec<GraphTopologyConnection>,
    pub public_parameters: Vec<GraphTopologyParameter>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GraphTopologyModule {
    pub id: String,
    pub module_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GraphTopologyConnection {
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GraphTopologyParameter {
    pub id: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GraphProposalResponse {
    pub patch_yaml: String,
    pub explanation: String,
    #[serde(default)]
    pub suggested_search_parameters: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ValidatedGraphProposal {
    pub provider_id: String,
    pub patch_name: String,
    pub explanation: String,
    pub suggested_search_parameters: Vec<String>,
    pub patch_yaml: String,
}

pub trait GraphProposalProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn capabilities(&self) -> GraphProposalCapabilities;
    fn propose(
        &self,
        request: &GraphProposalRequest,
        is_cancelled: &dyn Fn() -> bool,
    ) -> Result<GraphProposalResponse, String>;
}

pub fn build_graph_proposal_request(
    patch: &crate::patch::PatchDocument,
    matched: &SoundMatchManifest,
    candidate_metrics: &[crate::sound_analysis::AnalysisFrame],
    reference_metrics: &[crate::sound_analysis::AnalysisFrame],
) -> Result<GraphProposalRequest, String> {
    let allowed_modules = crate::builtins::BuiltInModuleRegistry::new()
        .module_types()
        .filter(|module_type| *module_type != crate::builtins::module_types::SCRIPT)
        .map(str::to_string)
        .collect();
    let modules = patch
        .modules
        .iter()
        .map(|module| GraphTopologyModule {
            id: module.id.clone(),
            module_type: module.module_type.clone(),
        })
        .collect();
    let connections = patch
        .connections
        .iter()
        .map(|connection| GraphTopologyConnection {
            from: connection.from.to_string(),
            to: connection.to.to_string(),
        })
        .collect();
    let public_parameters = patch
        .preset_surface
        .parameters
        .iter()
        .map(|parameter| GraphTopologyParameter {
            id: parameter.name.clone(),
            min: parameter.min,
            max: parameter.max,
        })
        .collect();
    let candidate = aggregate_sound_features(candidate_metrics)?;
    let reference = aggregate_sound_features(reference_metrics)?;
    let db_delta = |candidate: f64, reference: f64| {
        20.0 * (candidate.max(1.0e-12) / reference.max(1.0e-12)).log10()
    };
    let centroid_octaves =
        if candidate.mean_spectral_centroid_hz > 0.0 && reference.mean_spectral_centroid_hz > 0.0 {
            (candidate.mean_spectral_centroid_hz / reference.mean_spectral_centroid_hz).log2()
        } else {
            0.0
        };

    Ok(GraphProposalRequest {
        version: 1,
        goal: "Propose a Dandrum modular graph that reduces the supplied spectral, level, and centroid residuals."
            .to_string(),
        residual: GraphProposalResidual {
            total: matched.best_score.total,
            spectral: matched.best_score.spectral,
            rms: matched.best_score.rms,
            centroid: matched.best_score.centroid,
            candidate_gain: matched.best_score.candidate_gain,
            completed_evaluations: matched.completed_evaluations,
        },
        features: GraphProposalFeatureComparison {
            reference,
            candidate,
            delta: GraphProposalFeatureDelta {
                rms_db: db_delta(candidate.mean_rms, reference.mean_rms),
                peak_db: db_delta(candidate.max_peak, reference.max_peak),
                centroid_octaves,
            },
        },
        allowed_modules,
        current_topology: GraphTopologySummary {
            modules,
            connections,
            public_parameters,
        },
        constraints: vec![
            "Use only module types listed in allowed_modules.".to_string(),
            "Return a complete patch with no assets, scripts, external references, or filesystem paths."
                .to_string(),
            "Expose every suggested search parameter through preset_surface.parameters with finite bounds."
                .to_string(),
            "Use the directional aggregate feature deltas as evidence; positive deltas mean the candidate exceeds the reference."
                .to_string(),
            "Do not include reference audio, credentials, commands, or executable instructions."
                .to_string(),
        ],
    })
}

fn aggregate_sound_features(
    metrics: &[crate::sound_analysis::AnalysisFrame],
) -> Result<GraphProposalSoundFeatures, String> {
    if metrics.is_empty() {
        return Err("graph proposal requires non-empty coherent comparison metrics".to_string());
    }
    if metrics.iter().any(|frame| {
        !frame.rms.is_finite()
            || !frame.peak.is_finite()
            || frame
                .spectral_centroid_hz
                .is_some_and(|centroid| !centroid.is_finite())
    }) {
        return Err("graph proposal comparison metrics must be finite".to_string());
    }
    let frame_count = metrics.len();
    let mean_rms = metrics.iter().map(|frame| frame.rms).sum::<f64>() / frame_count as f64;
    let max_peak = metrics.iter().map(|frame| frame.peak).fold(0.0, f64::max);
    let centroids = metrics
        .iter()
        .filter_map(|frame| frame.spectral_centroid_hz)
        .collect::<Vec<_>>();
    let centroid_frame_count = centroids.len();
    let mean_spectral_centroid_hz = if centroids.is_empty() {
        0.0
    } else {
        centroids.iter().sum::<f64>() / centroids.len() as f64
    };
    Ok(GraphProposalSoundFeatures {
        frame_count,
        centroid_frame_count,
        mean_rms,
        max_peak,
        mean_spectral_centroid_hz,
    })
}

pub fn parse_graph_proposal_response(json: &str) -> Result<GraphProposalResponse, String> {
    serde_json::from_str(json).map_err(|error| format!("invalid graph proposal response: {error}"))
}

pub fn request_validated_graph_proposal(
    provider: &dyn GraphProposalProvider,
    request: &GraphProposalRequest,
    patch_root: &Path,
    is_cancelled: &dyn Fn() -> bool,
) -> Result<ValidatedGraphProposal, String> {
    if !provider.capabilities().structured_output {
        return Err(format!(
            "graph proposal provider {} does not support structured output",
            provider.provider_id()
        ));
    }
    let response = provider.propose(request, is_cancelled)?;
    if is_cancelled() {
        return Err("graph proposal cancelled".to_string());
    }
    validate_graph_proposal(provider.provider_id(), response, patch_root)
}

fn validate_graph_proposal(
    provider_id: &str,
    response: GraphProposalResponse,
    patch_root: &Path,
) -> Result<ValidatedGraphProposal, String> {
    if response.patch_yaml.trim().is_empty() {
        return Err("graph proposal patch_yaml must not be empty".to_string());
    }
    if response.explanation.trim().is_empty() {
        return Err("graph proposal explanation must not be empty".to_string());
    }

    let patch = crate::patch::load_patch_str(&response.patch_yaml)
        .map_err(|error| format!("proposed patch is invalid: {error}"))?;
    if !patch.assets.is_empty() || !patch.preset_surface.assets.is_empty() {
        return Err("proposed patch must not declare assets".to_string());
    }
    if patch_contains_forbidden_modules(&patch) {
        return Err(
            "proposed patch must not contain script modules or external module references"
                .to_string(),
        );
    }
    if let Some(module_type) = first_unknown_module_type(&patch) {
        return Err(format!(
            "proposed patch contains unknown module type {module_type}"
        ));
    }

    for parameter_id in &response.suggested_search_parameters {
        let Some(parameter) = patch
            .preset_surface
            .parameters
            .iter()
            .find(|parameter| parameter.name == *parameter_id)
        else {
            return Err(format!(
                "suggested search parameter {parameter_id} is not exposed by the proposed patch"
            ));
        };
        let (Some(min), Some(max)) = (parameter.min, parameter.max) else {
            return Err(format!(
                "suggested search parameter {parameter_id} must declare finite bounds"
            ));
        };
        if !matches!(
            parameter.value_type,
            crate::patch::PresetTargetType::Number | crate::patch::PresetTargetType::Integer
        ) || !matches!(parameter.default, crate::patch::ParameterValue::Number(_))
            || !min.is_finite()
            || !max.is_finite()
            || min >= max
        {
            return Err(format!(
                "suggested search parameter {parameter_id} must be numeric with finite ordered bounds"
            ));
        }
    }

    crate::preparation::prepare_instrument_document(patch.clone(), patch_root)
        .map_err(|error| format!("proposed patch failed local preparation: {error}"))?;

    Ok(ValidatedGraphProposal {
        provider_id: provider_id.to_string(),
        patch_name: patch.metadata.name,
        explanation: response.explanation,
        suggested_search_parameters: response.suggested_search_parameters,
        patch_yaml: response.patch_yaml,
    })
}

fn patch_contains_forbidden_modules(patch: &crate::patch::PatchDocument) -> bool {
    let forbidden = |module_type: &str| {
        module_type == crate::builtins::module_types::SCRIPT || module_type.starts_with('$')
    };
    patch
        .modules
        .iter()
        .any(|module| forbidden(&module.module_type))
        || patch.module_definitions.iter().any(|definition| {
            definition
                .modules
                .iter()
                .any(|module| forbidden(&module.module_type))
        })
}

fn first_unknown_module_type(patch: &crate::patch::PatchDocument) -> Option<&str> {
    let registry = crate::builtins::BuiltInModuleRegistry::new();
    let is_unknown = |module_type: &str| {
        registry.get(module_type).is_none()
            && !patch
                .module_definitions
                .iter()
                .any(|definition| definition.module_type == module_type)
    };
    patch
        .modules
        .iter()
        .chain(
            patch
                .module_definitions
                .iter()
                .flat_map(|definition| definition.modules.iter()),
        )
        .map(|module| module.module_type.as_str())
        .find(|module_type| is_unknown(module_type))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    const VALID_PATCH: &str = r#"
metadata:
  name: Proposed Acid Voice
instrument:
  id: dandrum.proposed-acid
  preset_schema_version: 1
preset_surface:
  parameters:
    - name: oscillator.pitch
      type: number
      default: 1
      min: 0.25
      max: 4
      maps_to: osc.pitch
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 48000
modules:
  - id: osc
    type: oscillator
  - id: mixer
    type: audio_mixer
  - id: out
    type: audio_output
    inputs:
      - { name: left, signal_type: audio }
      - { name: right, signal_type: audio }
connections:
  - { from: osc.audio, to: mixer.inputs }
  - { from: mixer.mix, to: out.left }
  - { from: mixer.mix, to: out.right }
"#;

    struct RecordingProvider {
        response: Result<GraphProposalResponse, String>,
        requests: Mutex<Vec<GraphProposalRequest>>,
    }

    struct UnstructuredProvider;

    impl GraphProposalProvider for UnstructuredProvider {
        fn provider_id(&self) -> &str {
            "unstructured-provider"
        }

        fn capabilities(&self) -> GraphProposalCapabilities {
            GraphProposalCapabilities {
                structured_output: false,
                cancellation: false,
            }
        }

        fn propose(
            &self,
            _request: &GraphProposalRequest,
            _is_cancelled: &dyn Fn() -> bool,
        ) -> Result<GraphProposalResponse, String> {
            panic!("an incompatible provider must not be invoked")
        }
    }

    struct CancellationIgnoringProvider;

    impl GraphProposalProvider for CancellationIgnoringProvider {
        fn provider_id(&self) -> &str {
            "cancellation-ignoring-provider"
        }

        fn capabilities(&self) -> GraphProposalCapabilities {
            GraphProposalCapabilities {
                structured_output: true,
                cancellation: false,
            }
        }

        fn propose(
            &self,
            _request: &GraphProposalRequest,
            _is_cancelled: &dyn Fn() -> bool,
        ) -> Result<GraphProposalResponse, String> {
            RecordingProvider::valid().response
        }
    }

    impl RecordingProvider {
        fn valid() -> Self {
            Self {
                response: Ok(GraphProposalResponse {
                    patch_yaml: VALID_PATCH.to_string(),
                    explanation: "Add a separately tunable oscillator pitch.".to_string(),
                    suggested_search_parameters: vec!["oscillator.pitch".to_string()],
                }),
                requests: Mutex::new(Vec::new()),
            }
        }
    }

    impl GraphProposalProvider for RecordingProvider {
        fn provider_id(&self) -> &str {
            "test-provider"
        }

        fn capabilities(&self) -> GraphProposalCapabilities {
            GraphProposalCapabilities {
                structured_output: true,
                cancellation: true,
            }
        }

        fn propose(
            &self,
            request: &GraphProposalRequest,
            is_cancelled: &dyn Fn() -> bool,
        ) -> Result<GraphProposalResponse, String> {
            if is_cancelled() {
                return Err("graph proposal cancelled".to_string());
            }
            self.requests.lock().unwrap().push(request.clone());
            self.response.clone()
        }
    }

    fn acid_fixture() -> crate::sound_workbench::SoundFixture {
        crate::sound_workbench::load_sound_fixture_file(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("examples/sound-design/tb303-acid-poc.yaml"),
        )
        .expect("acid fixture should load")
    }

    fn match_manifest() -> SoundMatchManifest {
        SoundMatchManifest {
            version: 2,
            fixture_name: "TB-303 acid proof of concept".to_string(),
            fixture_sha256: "cd".repeat(32),
            patch_sha256: "ef".repeat(32),
            reference_name: "private-hardware-reference.wav".to_string(),
            reference_sha256: "ab".repeat(32),
            optimizer: "dandrum.seeded_evolution.v1".to_string(),
            seed: 303,
            max_evaluations: 16,
            completed_evaluations: 16,
            status: crate::sound_matching::SoundMatchStatus::Completed,
            sample_rate_hz: 48_000,
            block_size_frames: 64,
            render_duration_frames: 528_000,
            analysis: crate::sound_analysis::AnalysisSettings {
                frame_size: 1_024,
                hop_size: 128,
                min_frequency_hz: 30.0,
                max_frequency_hz: 20_000.0,
                silence_rms: 1.0e-5,
            },
            region_start_frame: 48_000,
            region_length_frames: 96_000,
            spectral_windows: vec![256, 1_024, 4_096],
            weights: crate::sound_workbench::SoundMatchWeights {
                spectral: 0.7,
                rms: 0.2,
                centroid: 0.1,
            },
            best_score: crate::sound_matching::SoundMatchScore {
                spectral: 0.4,
                rms: 0.2,
                centroid: 0.1,
                total: 0.33,
                candidate_gain: 1.1,
            },
            best_parameters: Vec::new(),
            history: Vec::new(),
        }
    }

    fn comparison_metrics() -> (
        Vec<crate::sound_analysis::AnalysisFrame>,
        Vec<crate::sound_analysis::AnalysisFrame>,
    ) {
        let frame = |rms, peak, centroid| crate::sound_analysis::AnalysisFrame {
            start_frame: 0,
            time_seconds: 0.01,
            rms,
            peak,
            spectral_centroid_hz: Some(centroid),
        };
        (
            vec![frame(0.4, 0.8, 2_000.0), frame(0.2, 0.6, 1_000.0)],
            vec![frame(0.2, 0.5, 1_000.0), frame(0.1, 0.4, 500.0)],
        )
    }

    fn proposal_request(fixture: &crate::sound_workbench::SoundFixture) -> GraphProposalRequest {
        let (candidate, reference) = comparison_metrics();
        let patch = crate::patch::load_patch_file(&fixture.patch).expect("acid patch should load");
        build_graph_proposal_request(&patch, &match_manifest(), &candidate, &reference)
            .expect("canonical request should build")
    }

    #[test]
    fn canonical_request_contains_derived_residual_catalogue_and_sanitized_topology_only() {
        let fixture = acid_fixture();
        let request = proposal_request(&fixture);
        let json = serde_json::to_string(&request).unwrap();

        assert_eq!(request.version, 1);
        assert_eq!(request.residual.total, 0.33);
        assert!(request.features.candidate.mean_rms > request.features.reference.mean_rms);
        assert!(request.features.candidate.max_peak > request.features.reference.max_peak);
        assert!(
            request.features.candidate.mean_spectral_centroid_hz
                > request.features.reference.mean_spectral_centroid_hz
        );
        assert!(request.features.delta.rms_db > 0.0);
        assert!(request.features.delta.peak_db > 0.0);
        assert!(request.features.delta.centroid_octaves > 0.0);
        assert!(request.allowed_modules.contains(&"oscillator".to_string()));
        assert!(request.allowed_modules.contains(&"filter".to_string()));
        assert!(
            request
                .current_topology
                .modules
                .iter()
                .any(|module| { module.id == "filter" && module.module_type == "filter" })
        );
        assert!(
            request
                .current_topology
                .connections
                .iter()
                .any(|connection| {
                    connection.from == "osc.audio" && connection.to == "filter.audio_in"
                })
        );
        assert!(
            request
                .current_topology
                .public_parameters
                .iter()
                .any(|parameter| { parameter.id == "filter.cutoff" })
        );
        assert!(!json.contains("private-hardware-reference"));
        assert!(!json.contains(".wav"));
        assert!(!json.contains("RIFF"));
        assert!(!json.contains(&fixture.patch.display().to_string()));
        assert!(!json.contains("ab".repeat(32).as_str()));
    }

    #[test]
    fn feature_summary_rejects_missing_or_non_finite_metrics_and_handles_silent_centroids() {
        assert!(
            aggregate_sound_features(&[])
                .unwrap_err()
                .contains("non-empty")
        );
        let frame = |rms, peak, centroid| crate::sound_analysis::AnalysisFrame {
            start_frame: 0,
            time_seconds: 0.01,
            rms,
            peak,
            spectral_centroid_hz: centroid,
        };
        for invalid in [
            frame(f64::NAN, 0.5, Some(1_000.0)),
            frame(0.2, f64::NAN, Some(1_000.0)),
            frame(0.2, 0.5, Some(f64::NAN)),
        ] {
            assert!(
                aggregate_sound_features(&[invalid])
                    .unwrap_err()
                    .contains("finite")
            );
        }

        let fixture = acid_fixture();
        let silent = vec![frame(0.0, 0.0, None)];
        let patch = crate::patch::load_patch_file(&fixture.patch).expect("acid patch should load");
        let request = build_graph_proposal_request(&patch, &match_manifest(), &silent, &silent)
            .expect("silent finite feature summaries remain representable");

        assert_eq!(request.features.candidate.centroid_frame_count, 0);
        assert_eq!(request.features.candidate.mean_spectral_centroid_hz, 0.0);
        assert_eq!(request.features.delta.centroid_octaves, 0.0);
        assert_eq!(request.features.delta.rms_db, 0.0);
        assert_eq!(request.features.delta.peak_db, 0.0);
    }

    #[test]
    fn provider_neutral_orchestration_returns_only_locally_validated_patch_proposals() {
        let fixture = acid_fixture();
        let request = proposal_request(&fixture);
        let provider = RecordingProvider::valid();

        let proposal =
            request_validated_graph_proposal(&provider, &request, Path::new("."), &|| false)
                .expect("valid provider patch should pass local preparation");

        assert_eq!(proposal.provider_id, "test-provider");
        assert_eq!(proposal.patch_name, "Proposed Acid Voice");
        assert_eq!(proposal.suggested_search_parameters, ["oscillator.pitch"]);
        assert_eq!(provider.requests.lock().unwrap().as_slice(), [request]);
    }

    #[test]
    fn response_parser_rejects_malformed_json_and_unknown_fields() {
        let malformed =
            parse_graph_proposal_response("{").expect_err("malformed response should fail");
        let unknown = parse_graph_proposal_response(
            r#"{"patch_yaml":"x","explanation":"y","unexpected":true}"#,
        )
        .expect_err("unknown response fields should fail");

        assert!(malformed.contains("invalid graph proposal response"));
        assert!(unknown.contains("unknown field"));
    }

    #[test]
    fn local_validation_rejects_assets_scripts_unknown_modules_and_unknown_search_controls() {
        let fixture = acid_fixture();
        let request = proposal_request(&fixture);
        let variants = [
            (
                VALID_PATCH.replace(
                    "render:",
                    "assets:\n  - { id: sample, kind: sample, path: private.wav }\nrender:",
                ),
                Vec::new(),
                "assets",
            ),
            (
                VALID_PATCH.replace(
                    "  - id: osc\n    type: oscillator",
                    "  - id: osc\n    type: script\n    parameters:\n      language: rhai\n      source: 'fn process() {}'",
                ),
                Vec::new(),
                "script",
            ),
            (
                VALID_PATCH.replace("type: oscillator", "type: imaginary_oscillator"),
                Vec::new(),
                "unknown",
            ),
            (
                VALID_PATCH.to_string(),
                vec!["missing.control".to_string()],
                "suggested search parameter",
            ),
        ];

        for (patch_yaml, suggested_search_parameters, expected) in variants {
            let provider = RecordingProvider {
                response: Ok(GraphProposalResponse {
                    patch_yaml,
                    explanation: "invalid test response".to_string(),
                    suggested_search_parameters,
                }),
                requests: Mutex::new(Vec::new()),
            };
            let error =
                request_validated_graph_proposal(&provider, &request, Path::new("."), &|| false)
                    .expect_err("unsafe or invalid proposal should be rejected locally");
            assert!(
                error.to_lowercase().contains(expected),
                "unexpected error: {error}"
            );
        }
    }

    #[test]
    fn provider_failure_and_cancellation_remain_non_fatal_diagnostics() {
        let fixture = acid_fixture();
        let request = proposal_request(&fixture);
        let failed = RecordingProvider {
            response: Err("provider unavailable".to_string()),
            requests: Mutex::new(Vec::new()),
        };

        let provider_error =
            request_validated_graph_proposal(&failed, &request, Path::new("."), &|| false)
                .expect_err("provider failure should be returned");
        let cancelled = request_validated_graph_proposal(
            &RecordingProvider::valid(),
            &request,
            Path::new("."),
            &|| true,
        )
        .expect_err("cancellation should be returned");

        assert!(provider_error.contains("provider unavailable"));
        assert!(cancelled.contains("cancelled"));
    }

    #[test]
    fn orchestration_rejects_unstructured_providers_and_post_response_cancellation() {
        let fixture = acid_fixture();
        let request = proposal_request(&fixture);

        let incompatible = request_validated_graph_proposal(
            &UnstructuredProvider,
            &request,
            Path::new("."),
            &|| false,
        )
        .unwrap_err();
        let cancelled = request_validated_graph_proposal(
            &CancellationIgnoringProvider,
            &request,
            Path::new("."),
            &|| true,
        )
        .unwrap_err();

        assert!(incompatible.contains("structured output"));
        assert!(cancelled.contains("cancelled"));
    }

    #[test]
    fn local_validation_rejects_empty_fields_bad_yaml_and_unbounded_or_non_numeric_controls() {
        let fixture = acid_fixture();
        let request = proposal_request(&fixture);
        let variants = [
            (
                "".to_string(),
                "explanation".to_string(),
                Vec::new(),
                "patch_yaml",
            ),
            (
                VALID_PATCH.to_string(),
                "".to_string(),
                Vec::new(),
                "explanation",
            ),
            (
                "not: [valid".to_string(),
                "explanation".to_string(),
                Vec::new(),
                "invalid",
            ),
            (
                VALID_PATCH.replace("      min: 0.25\n      max: 4\n", ""),
                "explanation".to_string(),
                vec!["oscillator.pitch".to_string()],
                "finite bounds",
            ),
            (
                VALID_PATCH.replace(
                    "      min: 0.25\n      max: 4",
                    "      min: 4\n      max: 0.25",
                ),
                "explanation".to_string(),
                vec!["oscillator.pitch".to_string()],
                "numeric with finite ordered bounds",
            ),
            (
                VALID_PATCH.replace(
                    "      type: number\n      default: 1",
                    "      type: boolean\n      default: true",
                ),
                "explanation".to_string(),
                vec!["oscillator.pitch".to_string()],
                "numeric with finite ordered bounds",
            ),
            (
                VALID_PATCH.replace("      default: 1", "      default: saw"),
                "explanation".to_string(),
                vec!["oscillator.pitch".to_string()],
                "numeric with finite ordered bounds",
            ),
            (
                VALID_PATCH.replace("      min: 0.25", "      min: .nan"),
                "explanation".to_string(),
                vec!["oscillator.pitch".to_string()],
                "numeric with finite ordered bounds",
            ),
            (
                VALID_PATCH.replace("      max: 4", "      max: .inf"),
                "explanation".to_string(),
                vec!["oscillator.pitch".to_string()],
                "numeric with finite ordered bounds",
            ),
            (
                VALID_PATCH.replace(
                    "      min: 0.25\n      max: 4",
                    "      min: 1\n      max: 1",
                ),
                "explanation".to_string(),
                vec!["oscillator.pitch".to_string()],
                "numeric with finite ordered bounds",
            ),
        ];

        for (patch_yaml, explanation, suggested_search_parameters, expected) in variants {
            let provider = RecordingProvider {
                response: Ok(GraphProposalResponse {
                    patch_yaml,
                    explanation,
                    suggested_search_parameters,
                }),
                requests: Mutex::new(Vec::new()),
            };
            let error =
                request_validated_graph_proposal(&provider, &request, Path::new("."), &|| false)
                    .unwrap_err();
            assert!(error.contains(expected), "unexpected error: {error}");
        }
    }

    #[test]
    fn local_safety_helpers_cover_top_level_nested_external_and_defined_modules() {
        let top_script =
            crate::patch::load_patch_str(&VALID_PATCH.replace("type: oscillator", "type: script"))
                .unwrap();
        let top_external = crate::patch::load_patch_str(
            &VALID_PATCH.replace("type: oscillator", "type: $unsafe/external@1"),
        )
        .unwrap();
        let nested_script = crate::patch::load_patch_str(&VALID_PATCH.replace(
            "modules:",
            "module_definitions:\n  - type: custom\n    modules:\n      - { id: unsafe, type: script }\nmodules:",
        ))
        .unwrap();
        let nested_external = crate::patch::load_patch_str(&VALID_PATCH.replace(
            "modules:",
            "module_definitions:\n  - type: custom\n    modules:\n      - { id: unsafe, type: '$unsafe/external@1' }\nmodules:",
        ))
        .unwrap();
        let defined = crate::patch::load_patch_str(&VALID_PATCH.replace(
            "modules:\n  - id: osc\n    type: oscillator",
            "module_definitions:\n  - type: custom\n    modules:\n      - { id: internal, type: oscillator }\nmodules:\n  - id: osc\n    type: custom",
        ))
        .unwrap();
        let unknown = crate::patch::load_patch_str(
            &VALID_PATCH.replace("type: oscillator", "type: unknown_oscillator"),
        )
        .unwrap();

        assert!(patch_contains_forbidden_modules(&top_script));
        assert!(patch_contains_forbidden_modules(&top_external));
        assert!(patch_contains_forbidden_modules(&nested_script));
        assert!(patch_contains_forbidden_modules(&nested_external));
        assert!(!patch_contains_forbidden_modules(&defined));
        assert_eq!(first_unknown_module_type(&defined), None);
        assert_eq!(
            first_unknown_module_type(&unknown),
            Some("unknown_oscillator")
        );
    }
}
