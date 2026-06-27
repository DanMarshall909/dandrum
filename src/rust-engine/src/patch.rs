use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PatchDocument {
    pub metadata: PatchMetadata,
    pub render: RenderSettings,
    #[serde(default)]
    pub assets: Vec<AssetDeclaration>,
    pub modules: Vec<ModuleDeclaration>,
    #[serde(default)]
    pub connections: Vec<ConnectionDeclaration>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct PatchMetadata {
    pub name: String,
    pub version: Option<String>,
    pub author: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct RenderSettings {
    pub sample_rate_hz: u32,
    pub block_size_frames: u32,
    pub duration_frames: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AssetDeclaration {
    pub id: String,
    pub kind: AssetKind,
    pub path: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Sample,
    Script,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ModuleDeclaration {
    pub id: String,
    #[serde(rename = "type")]
    pub module_type: String,
    #[serde(default)]
    pub inputs: Vec<PortDeclaration>,
    #[serde(default)]
    pub outputs: Vec<PortDeclaration>,
    #[serde(default)]
    pub parameters: BTreeMap<String, ParameterValue>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PortDeclaration {
    pub name: String,
    pub signal_type: SignalType,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    Audio,
    Control,
    Event,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ParameterValue {
    Boolean(bool),
    Number(f64),
    Text(String),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ConnectionDeclaration {
    pub from: PortReference,
    pub to: PortReference,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortReference {
    pub module_id: String,
    pub port_name: String,
}

#[derive(Debug)]
pub enum PatchLoadError {
    UnsupportedFormat {
        path: PathBuf,
    },
    ReadFailed {
        path: PathBuf,
        message: String,
    },
    ParseFailed {
        path: Option<PathBuf>,
        message: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatchValidationError {
    diagnostics: Vec<String>,
}

pub fn load_patch_file(path: impl AsRef<Path>) -> Result<PatchDocument, PatchLoadError> {
    let path = path.as_ref();

    if !is_yaml_path(path) {
        return Err(PatchLoadError::UnsupportedFormat {
            path: path.to_path_buf(),
        });
    }

    let yaml = fs::read_to_string(path).map_err(|error| PatchLoadError::ReadFailed {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;

    load_patch_str(&yaml).map_err(|error| match error {
        PatchLoadError::ParseFailed { message, .. } => PatchLoadError::ParseFailed {
            path: Some(path.to_path_buf()),
            message,
        },
        error => error,
    })
}

pub fn load_patch_str(yaml: &str) -> Result<PatchDocument, PatchLoadError> {
    serde_yaml::from_str(yaml).map_err(|error| PatchLoadError::ParseFailed {
        path: None,
        message: error.to_string(),
    })
}

pub fn validate_patch_schema(patch: &PatchDocument) -> Result<(), PatchValidationError> {
    let mut diagnostics = Vec::new();

    if patch.metadata.name.trim().is_empty() {
        diagnostics.push("metadata.name is required".to_string());
    }

    if patch.render.sample_rate_hz == 0 {
        diagnostics.push("render.sample_rate_hz must be greater than zero".to_string());
    }

    if patch.render.block_size_frames == 0 {
        diagnostics.push("render.block_size_frames must be greater than zero".to_string());
    }

    if patch.modules.is_empty() {
        diagnostics.push("modules must declare at least one module".to_string());
    }

    let mut module_ids = BTreeSet::new();
    for module in &patch.modules {
        if module.id.trim().is_empty() {
            diagnostics.push("module.id is required".to_string());
        } else if !module_ids.insert(module.id.as_str()) {
            diagnostics.push(format!("duplicate module id: {}", module.id));
        }

        if module.module_type.trim().is_empty() {
            diagnostics.push(format!("module {} type is required", module.id));
        }

        for port in module.inputs.iter().chain(module.outputs.iter()) {
            if port.name.trim().is_empty() {
                diagnostics.push(format!("module {} port name is required", module.id));
            }
        }

        if module.module_type == "sampler" {
            validate_sampler_asset_reference(module, patch, &mut diagnostics);
        }
    }

    for connection in &patch.connections {
        validate_port_reference("connection.from", &connection.from, &mut diagnostics);
        validate_port_reference("connection.to", &connection.to, &mut diagnostics);
    }

    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(PatchValidationError { diagnostics })
    }
}

fn validate_sampler_asset_reference(
    module: &ModuleDeclaration,
    patch: &PatchDocument,
    diagnostics: &mut Vec<String>,
) {
    let Some(asset_parameter) = module.parameters.get("asset") else {
        diagnostics.push(format!(
            "sampler module {} missing required asset parameter",
            module.id
        ));
        return;
    };

    let ParameterValue::Text(asset_id) = asset_parameter else {
        diagnostics.push(format!(
            "sampler module {} asset parameter must be a text asset ID",
            module.id
        ));
        return;
    };

    let Some(asset) = patch.assets.iter().find(|asset| asset.id == *asset_id) else {
        diagnostics.push(format!(
            "sampler module {} references missing asset {}",
            module.id, asset_id
        ));
        return;
    };

    if asset.kind != AssetKind::Sample {
        diagnostics.push(format!(
            "sampler module {} references asset {} with kind {:?}; expected sample",
            module.id, asset_id, asset.kind
        ));
    }
}

impl<'de> Deserialize<'de> for PortReference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let (module_id, port_name) = value.split_once('.').ok_or_else(|| {
            serde::de::Error::custom("port reference must use module_id.port_name")
        })?;

        if module_id.is_empty() || port_name.is_empty() || port_name.contains('.') {
            return Err(serde::de::Error::custom(
                "port reference must use module_id.port_name",
            ));
        }

        Ok(Self {
            module_id: module_id.to_string(),
            port_name: port_name.to_string(),
        })
    }
}

impl fmt::Display for PatchLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFormat { path } => {
                write!(formatter, "unsupported patch format: {}", path.display())
            }
            Self::ReadFailed { path, message } => {
                write!(
                    formatter,
                    "failed to read patch {}: {message}",
                    path.display()
                )
            }
            Self::ParseFailed { path, message } => match path {
                Some(path) => write!(
                    formatter,
                    "failed to parse patch {}: {message}",
                    path.display()
                ),
                None => write!(formatter, "failed to parse patch: {message}"),
            },
        }
    }
}

impl std::error::Error for PatchLoadError {}

impl PatchValidationError {
    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }
}

impl fmt::Display for PatchValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "patch validation failed")?;

        for diagnostic in &self.diagnostics {
            write!(formatter, "\n- {diagnostic}")?;
        }

        Ok(())
    }
}

impl std::error::Error for PatchValidationError {}

fn is_yaml_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "yaml" | "yml"))
}

fn validate_port_reference(label: &str, reference: &PortReference, diagnostics: &mut Vec<String>) {
    if reference.module_id.trim().is_empty() || reference.port_name.trim().is_empty() {
        diagnostics.push(format!(
            "{label} must use a non-empty module_id.port_name reference"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_schema_separates_metadata_render_assets_modules_and_connections() {
        let patch = PatchDocument {
            metadata: PatchMetadata {
                name: "Basic Voice".to_string(),
                version: Some("0.1.0".to_string()),
                author: None,
            },
            render: RenderSettings {
                sample_rate_hz: 48_000,
                block_size_frames: 128,
                duration_frames: 48_000,
            },
            assets: vec![AssetDeclaration {
                id: "kick".to_string(),
                kind: AssetKind::Sample,
                path: "samples/kick.wav".to_string(),
            }],
            modules: vec![ModuleDeclaration {
                id: "out".to_string(),
                module_type: "audio_output".to_string(),
                inputs: vec![PortDeclaration {
                    name: "left".to_string(),
                    signal_type: SignalType::Audio,
                }],
                outputs: vec![],
                parameters: BTreeMap::new(),
            }],
            connections: vec![ConnectionDeclaration {
                from: PortReference {
                    module_id: "osc".to_string(),
                    port_name: "audio".to_string(),
                },
                to: PortReference {
                    module_id: "out".to_string(),
                    port_name: "left".to_string(),
                },
            }],
        };

        assert_eq!(patch.metadata.name, "Basic Voice");
        assert_eq!(patch.render.block_size_frames, 128);
        assert_eq!(patch.assets[0].kind, AssetKind::Sample);
        assert_eq!(patch.modules[0].id, "out");
        assert_eq!(patch.connections[0].to.module_id, "out");
    }

    #[test]
    fn script_modules_can_declare_custom_event_and_control_ports() {
        let script = ModuleDeclaration {
            id: "accent_script".to_string(),
            module_type: "script".to_string(),
            inputs: vec![PortDeclaration {
                name: "notes".to_string(),
                signal_type: SignalType::Event,
            }],
            outputs: vec![PortDeclaration {
                name: "accent".to_string(),
                signal_type: SignalType::Control,
            }],
            parameters: BTreeMap::from([(
                "source".to_string(),
                ParameterValue::Text("scripts/accent.dan".to_string()),
            )]),
        };

        assert_eq!(script.inputs[0].signal_type, SignalType::Event);
        assert_eq!(script.outputs[0].signal_type, SignalType::Control);
        assert!(script.parameters.contains_key("source"));
    }

    #[test]
    fn loads_valid_yaml_patch_document() {
        let patch = load_patch_str(
            r#"
metadata:
  name: Basic Voice
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
assets:
  - id: wavetable
    kind: sample
    path: assets/wavetable.wav
modules:
  - id: osc
    type: oscillator
    outputs:
      - name: audio
        signal_type: audio
    parameters:
      frequency: 220.0
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
connections:
  - from: osc.audio
    to: out.left
"#,
        )
        .expect("valid YAML patch should load");

        assert_eq!(patch.metadata.name, "Basic Voice");
        assert_eq!(patch.assets[0].kind, AssetKind::Sample);
        assert_eq!(patch.modules[0].module_type, "oscillator");
        assert_eq!(patch.connections[0].from.module_id, "osc");
        assert_eq!(patch.connections[0].from.port_name, "audio");
    }

    #[test]
    fn invalid_yaml_reports_parse_error() {
        let error = load_patch_str("metadata: [unterminated").expect_err("invalid YAML must fail");

        assert!(matches!(error, PatchLoadError::ParseFailed { .. }));
        assert!(error.to_string().contains("failed to parse patch"));
    }

    #[test]
    fn yaml_patch_missing_modules_section_is_rejected() {
        let error = load_patch_str(
            r#"
metadata:
  name: Missing Modules
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
"#,
        )
        .expect_err("modules section is required");

        assert!(matches!(error, PatchLoadError::ParseFailed { .. }));
        assert!(error.to_string().contains("missing field `modules`"));
    }

    #[test]
    fn non_yaml_patch_file_is_rejected_before_reading() {
        let error = load_patch_file("patches/basic.json").expect_err("JSON is not supported");

        assert!(matches!(error, PatchLoadError::UnsupportedFormat { .. }));
        assert!(error.to_string().contains("unsupported patch format"));
        assert!(error.to_string().contains("patches/basic.json"));
    }

    #[test]
    fn schema_validation_accepts_minimal_valid_patch() {
        let patch = minimal_patch(vec![ModuleDeclaration {
            id: "out".to_string(),
            module_type: "audio_output".to_string(),
            inputs: vec![PortDeclaration {
                name: "left".to_string(),
                signal_type: SignalType::Audio,
            }],
            outputs: vec![],
            parameters: BTreeMap::new(),
        }]);

        validate_patch_schema(&patch).expect("valid schema should pass");
    }

    #[test]
    fn schema_validation_reports_duplicate_module_ids() {
        let patch = minimal_patch(vec![
            ModuleDeclaration {
                id: "osc".to_string(),
                module_type: "oscillator".to_string(),
                inputs: vec![],
                outputs: vec![],
                parameters: BTreeMap::new(),
            },
            ModuleDeclaration {
                id: "osc".to_string(),
                module_type: "audio_output".to_string(),
                inputs: vec![],
                outputs: vec![],
                parameters: BTreeMap::new(),
            },
        ]);

        let error = validate_patch_schema(&patch).expect_err("duplicate IDs must fail");

        assert!(
            error
                .diagnostics()
                .contains(&"duplicate module id: osc".to_string())
        );
    }

    #[test]
    fn schema_validation_reports_missing_required_values() {
        let patch = PatchDocument {
            metadata: PatchMetadata::default(),
            render: RenderSettings {
                sample_rate_hz: 0,
                block_size_frames: 0,
                duration_frames: 0,
            },
            assets: vec![],
            modules: vec![],
            connections: vec![],
        };

        let error = validate_patch_schema(&patch).expect_err("missing required values must fail");

        assert!(
            error
                .diagnostics()
                .contains(&"metadata.name is required".to_string())
        );
        assert!(
            error
                .diagnostics()
                .contains(&"render.sample_rate_hz must be greater than zero".to_string())
        );
        assert!(
            error
                .diagnostics()
                .contains(&"modules must declare at least one module".to_string())
        );
    }

    #[test]
    fn schema_validation_reports_malformed_connection_references() {
        let mut patch = minimal_patch(vec![ModuleDeclaration {
            id: "out".to_string(),
            module_type: "audio_output".to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: BTreeMap::new(),
        }]);
        patch.connections.push(ConnectionDeclaration {
            from: PortReference {
                module_id: String::new(),
                port_name: "audio".to_string(),
            },
            to: PortReference {
                module_id: "out".to_string(),
                port_name: String::new(),
            },
        });

        let error = validate_patch_schema(&patch).expect_err("malformed references must fail");

        assert!(error.diagnostics().contains(
            &"connection.from must use a non-empty module_id.port_name reference".to_string()
        ));
        assert!(error.diagnostics().contains(
            &"connection.to must use a non-empty module_id.port_name reference".to_string()
        ));
    }

    #[test]
    fn schema_validation_accepts_sampler_with_declared_sample_asset() {
        let mut patch = minimal_patch(vec![sampler_module(Some(ParameterValue::Text(
            "hit".to_string(),
        )))]);
        patch.assets.push(AssetDeclaration {
            id: "hit".to_string(),
            kind: AssetKind::Sample,
            path: "hit.wav".to_string(),
        });

        validate_patch_schema(&patch).expect("sampler asset reference should validate");
    }

    #[test]
    fn schema_validation_reports_sampler_missing_asset_parameter() {
        let patch = minimal_patch(vec![sampler_module(None)]);

        let error = validate_patch_schema(&patch).expect_err("missing asset should fail");

        assert!(
            error
                .diagnostics()
                .contains(&"sampler module sampler missing required asset parameter".to_string())
        );
    }

    #[test]
    fn schema_validation_reports_sampler_missing_asset_id() {
        let patch = minimal_patch(vec![sampler_module(Some(ParameterValue::Text(
            "missing".to_string(),
        )))]);

        let error = validate_patch_schema(&patch).expect_err("missing asset ID should fail");

        assert!(
            error
                .diagnostics()
                .contains(&"sampler module sampler references missing asset missing".to_string())
        );
    }

    #[test]
    fn schema_validation_reports_sampler_non_sample_asset_kind() {
        let mut patch = minimal_patch(vec![sampler_module(Some(ParameterValue::Text(
            "script".to_string(),
        )))]);
        patch.assets.push(AssetDeclaration {
            id: "script".to_string(),
            kind: AssetKind::Script,
            path: "script.dan".to_string(),
        });

        let error = validate_patch_schema(&patch).expect_err("non-sample asset should fail");

        assert!(error.diagnostics().iter().any(|diagnostic| {
            diagnostic.contains("sampler module sampler")
                && diagnostic.contains("asset script")
                && diagnostic.contains("expected sample")
        }));
    }

    fn minimal_patch(modules: Vec<ModuleDeclaration>) -> PatchDocument {
        PatchDocument {
            metadata: PatchMetadata {
                name: "Minimal".to_string(),
                version: None,
                author: None,
            },
            render: RenderSettings {
                sample_rate_hz: 48_000,
                block_size_frames: 128,
                duration_frames: 48_000,
            },
            assets: vec![],
            modules,
            connections: vec![],
        }
    }

    fn sampler_module(asset: Option<ParameterValue>) -> ModuleDeclaration {
        ModuleDeclaration {
            id: "sampler".to_string(),
            module_type: "sampler".to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: asset
                .map(|value| BTreeMap::from([("asset".to_string(), value)]))
                .unwrap_or_default(),
        }
    }
}
