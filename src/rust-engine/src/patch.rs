use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::builtins::{BuiltInModuleRegistry, module_types};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PatchDocument {
    pub metadata: PatchMetadata,
    pub render: RenderSettings,
    #[serde(default)]
    pub assets: Vec<AssetDeclaration>,
    #[serde(default)]
    pub module_definitions: Vec<ModuleDefinitionDeclaration>,
    pub modules: Vec<ModuleDeclaration>,
    #[serde(default)]
    pub connections: Vec<ConnectionDeclaration>,
    #[serde(default)]
    pub voice_allocation: VoiceAllocation,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ModuleDefinitionDeclaration {
    #[serde(rename = "type")]
    pub module_type: String,
    #[serde(default)]
    pub inputs: Vec<CompositeInputDeclaration>,
    #[serde(default)]
    pub outputs: Vec<CompositeOutputDeclaration>,
    #[serde(default)]
    pub parameters: Vec<CompositeBindingDeclaration>,
    #[serde(default)]
    pub asset_bindings: Vec<CompositeBindingDeclaration>,
    #[serde(default)]
    pub modules: Vec<ModuleDeclaration>,
    #[serde(default)]
    pub connections: Vec<ConnectionDeclaration>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CompositeInputDeclaration {
    pub name: String,
    pub signal_type: SignalType,
    #[serde(default)]
    pub maps_to: Vec<PortReference>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CompositeOutputDeclaration {
    pub name: String,
    pub signal_type: SignalType,
    #[serde(default)]
    pub maps_from: Vec<PortReference>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CompositeBindingDeclaration {
    pub name: String,
    #[serde(default)]
    pub maps_to: Vec<PortReference>,
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

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct VoiceAllocation {
    pub max_voices: u32,
    #[serde(default)]
    pub stealing: VoiceStealingPolicy,
}

impl Default for VoiceAllocation {
    fn default() -> Self {
        Self {
            max_voices: 1,
            stealing: VoiceStealingPolicy::Disabled,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VoiceStealingPolicy {
    #[default]
    Disabled,
    OldestActive,
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

    validate_module_definitions(patch, &mut diagnostics);

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

        validate_composite_instance_bindings(module, patch, &mut diagnostics);
    }

    if patch.voice_allocation.max_voices == 0 {
        diagnostics.push("voice_allocation.max_voices must be greater than zero".to_string());
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

fn validate_module_definitions(patch: &PatchDocument, diagnostics: &mut Vec<String>) {
    let mut module_types = BTreeSet::new();
    let registry = BuiltInModuleRegistry::new();

    for definition in &patch.module_definitions {
        if definition.module_type.trim().is_empty() {
            diagnostics.push("composite module type is required".to_string());
        } else if !module_types.insert(definition.module_type.as_str()) {
            diagnostics.push(format!(
                "duplicate composite module type: {}",
                definition.module_type
            ));
        }

        for input in &definition.inputs {
            let port_name = composite_port_name(&input.name);
            if input.name.trim().is_empty() {
                diagnostics.push(format!(
                    "composite {} input name is required",
                    definition.module_type
                ));
            }

            for reference in &input.maps_to {
                validate_port_reference(
                    &format!(
                        "composite {} input {port_name} maps_to",
                        definition.module_type
                    ),
                    reference,
                    diagnostics,
                );
                validate_composite_mapping(
                    &definition.module_type,
                    "input",
                    &input.name,
                    input.signal_type.clone(),
                    "maps_to",
                    reference,
                    CompositeMappingDirection::PublicInputToInternalInput,
                    definition,
                    &registry,
                    diagnostics,
                );
            }
        }

        for output in &definition.outputs {
            let port_name = composite_port_name(&output.name);
            if output.name.trim().is_empty() {
                diagnostics.push(format!(
                    "composite {} output name is required",
                    definition.module_type
                ));
            }

            for reference in &output.maps_from {
                validate_port_reference(
                    &format!(
                        "composite {} output {port_name} maps_from",
                        definition.module_type
                    ),
                    reference,
                    diagnostics,
                );
                validate_composite_mapping(
                    &definition.module_type,
                    "output",
                    &output.name,
                    output.signal_type.clone(),
                    "maps_from",
                    reference,
                    CompositeMappingDirection::PublicOutputFromInternalOutput,
                    definition,
                    &registry,
                    diagnostics,
                );
            }
        }
    }

    validate_recursive_composite_definitions(patch, diagnostics);
}

fn validate_recursive_composite_definitions(patch: &PatchDocument, diagnostics: &mut Vec<String>) {
    let composite_types = patch
        .module_definitions
        .iter()
        .map(|definition| definition.module_type.as_str())
        .collect::<BTreeSet<_>>();
    let dependencies = patch
        .module_definitions
        .iter()
        .map(|definition| {
            let nested = definition
                .modules
                .iter()
                .filter(|module| composite_types.contains(module.module_type.as_str()))
                .map(|module| module.module_type.as_str())
                .collect::<Vec<_>>();
            (definition.module_type.as_str(), nested)
        })
        .collect::<BTreeMap<_, _>>();
    let mut reported_paths = BTreeSet::new();

    for definition in &patch.module_definitions {
        let mut stack = Vec::new();
        collect_recursive_composite_paths(
            definition.module_type.as_str(),
            &dependencies,
            &mut stack,
            &mut reported_paths,
        );
    }

    for path in reported_paths {
        diagnostics.push(format!("recursive composite definition: {path}"));
    }
}

fn collect_recursive_composite_paths<'a>(
    current: &'a str,
    dependencies: &BTreeMap<&'a str, Vec<&'a str>>,
    stack: &mut Vec<&'a str>,
    reported_paths: &mut BTreeSet<String>,
) {
    if let Some(position) = stack.iter().position(|module_type| *module_type == current) {
        let mut path = stack[position..].to_vec();
        path.push(current);
        reported_paths.insert(path.join(" -> "));
        return;
    }

    stack.push(current);
    if let Some(nested) = dependencies.get(current) {
        for dependency in nested {
            collect_recursive_composite_paths(dependency, dependencies, stack, reported_paths);
        }
    }
    stack.pop();
}

fn validate_composite_instance_bindings(
    module: &ModuleDeclaration,
    patch: &PatchDocument,
    diagnostics: &mut Vec<String>,
) {
    let Some(definition) = patch
        .module_definitions
        .iter()
        .find(|definition| definition.module_type == module.module_type)
    else {
        return;
    };

    let declared_bindings = definition
        .parameters
        .iter()
        .chain(definition.asset_bindings.iter())
        .map(|binding| binding.name.as_str())
        .collect::<BTreeSet<_>>();

    for key in module.parameters.keys() {
        if !declared_bindings.contains(key.as_str()) {
            diagnostics.push(format!(
                "composite {} instance {} sets undeclared parameter {}",
                definition.module_type, module.id, key
            ));
        }
    }

    for binding in &definition.asset_bindings {
        let Some(value) = module.parameters.get(&binding.name) else {
            continue;
        };
        let ParameterValue::Text(asset_id) = value else {
            diagnostics.push(format!(
                "composite {} instance {} asset binding {} must be a text asset ID",
                definition.module_type, module.id, binding.name
            ));
            continue;
        };
        let Some(asset) = patch.assets.iter().find(|asset| asset.id == *asset_id) else {
            diagnostics.push(format!(
                "composite {} instance {} asset binding {} references missing asset {}",
                definition.module_type, module.id, binding.name, asset_id
            ));
            continue;
        };
        if asset.kind != AssetKind::Sample {
            diagnostics.push(format!(
                "composite {} instance {} asset binding {} references asset {} with kind {:?}; expected sample",
                definition.module_type, module.id, binding.name, asset_id, asset.kind
            ));
        }
    }
}

#[derive(Clone, Copy)]
enum CompositeMappingDirection {
    PublicInputToInternalInput,
    PublicOutputFromInternalOutput,
}

fn validate_composite_mapping(
    definition_type: &str,
    public_direction_label: &str,
    public_name: &str,
    public_signal_type: SignalType,
    mapping_label: &str,
    reference: &PortReference,
    direction: CompositeMappingDirection,
    definition: &ModuleDefinitionDeclaration,
    registry: &BuiltInModuleRegistry,
    diagnostics: &mut Vec<String>,
) {
    if reference.module_id.trim().is_empty() || reference.port_name.trim().is_empty() {
        return;
    }

    let resolved = resolve_internal_port_type(definition, reference, direction, registry);
    if resolved == InternalPortResolution::WrongDirection {
        diagnostics.push(format!(
            "composite {definition_type} {public_direction_label} {} {mapping_label} {reference} must reference an internal {} port",
            composite_port_name(public_name),
            match direction {
                CompositeMappingDirection::PublicInputToInternalInput => "input",
                CompositeMappingDirection::PublicOutputFromInternalOutput => "output",
            }
        ));
        return;
    }

    let InternalPortResolution::Found(internal_type) = resolved else {
        return;
    };

    if public_signal_type != internal_type {
        diagnostics.push(format!(
            "composite {definition_type} {public_direction_label} {} {mapping_label} {reference} has incompatible signal types: public {:?}, internal {:?}",
            composite_port_name(public_name),
            public_signal_type,
            internal_type
        ));
    }
}

fn resolve_internal_port_type(
    definition: &ModuleDefinitionDeclaration,
    reference: &PortReference,
    direction: CompositeMappingDirection,
    registry: &BuiltInModuleRegistry,
) -> InternalPortResolution {
    let module = definition
        .modules
        .iter()
        .find(|module| module.id == reference.module_id);
    let Some(module) = module else {
        return InternalPortResolution::Missing;
    };

    let built_in = registry.get(&module.module_type);

    if built_in.is_none() || module.module_type == module_types::SCRIPT {
        let expected_ports = match direction {
            CompositeMappingDirection::PublicInputToInternalInput => &module.inputs,
            CompositeMappingDirection::PublicOutputFromInternalOutput => &module.outputs,
        };
        if let Some(port) = expected_ports
            .iter()
            .find(|port| port.name == reference.port_name)
        {
            return InternalPortResolution::Found(port.signal_type.clone());
        }

        let opposite_ports = match direction {
            CompositeMappingDirection::PublicInputToInternalInput => &module.outputs,
            CompositeMappingDirection::PublicOutputFromInternalOutput => &module.inputs,
        };
        if opposite_ports
            .iter()
            .any(|port| port.name == reference.port_name)
        {
            return InternalPortResolution::WrongDirection;
        }

        return InternalPortResolution::Missing;
    }

    let Some(built_in) = built_in else {
        return InternalPortResolution::Missing;
    };
    let expected_ports = match direction {
        CompositeMappingDirection::PublicInputToInternalInput => built_in.inputs(),
        CompositeMappingDirection::PublicOutputFromInternalOutput => built_in.outputs(),
    };
    if let Some(port) = expected_ports
        .iter()
        .find(|port| port.name() == reference.port_name)
    {
        return InternalPortResolution::Found(SignalType::from_graph(port.signal_type()));
    }

    let opposite_ports = match direction {
        CompositeMappingDirection::PublicInputToInternalInput => built_in.outputs(),
        CompositeMappingDirection::PublicOutputFromInternalOutput => built_in.inputs(),
    };
    if opposite_ports
        .iter()
        .any(|port| port.name() == reference.port_name)
    {
        return InternalPortResolution::WrongDirection;
    }

    InternalPortResolution::Missing
}

#[derive(PartialEq, Eq)]
enum InternalPortResolution {
    Found(SignalType),
    Missing,
    WrongDirection,
}

fn composite_port_name(name: &str) -> &str {
    if name.trim().is_empty() {
        "<unnamed>"
    } else {
        name
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

impl SignalType {
    fn from_graph(signal_type: crate::graph::SignalType) -> Self {
        match signal_type {
            crate::graph::SignalType::Audio => Self::Audio,
            crate::graph::SignalType::Control => Self::Control,
            crate::graph::SignalType::Event => Self::Event,
        }
    }
}

impl fmt::Display for PortReference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}", self.module_id, self.port_name)
    }
}

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
            module_definitions: vec![],
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
            voice_allocation: VoiceAllocation::default(),
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
    fn loads_yaml_composite_module_definitions() {
        let patch = load_patch_str(
            r#"
metadata:
  name: Composite Voice
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
assets:
  - id: kick
    kind: sample
    path: assets/kick.wav
module_definitions:
  - type: drum_voice
    inputs:
      - name: trigger
        signal_type: event
        maps_to:
          - env.gate
      - name: pitch
        signal_type: control
        maps_to:
          - osc.pitch
    outputs:
      - name: audio
        signal_type: audio
        maps_from:
          - vca.audio_out
    parameters:
      - name: level
        maps_to:
          - vca.gain
    asset_bindings:
      - name: sample
        maps_to:
          - sampler.asset
    modules:
      - id: osc
        type: oscillator
      - id: env
        type: adsr
      - id: sampler
        type: sampler
        parameters:
          asset: kick
      - id: vca
        type: gain
    connections:
      - from: osc.audio
        to: vca.audio_in
modules:
  - id: voice
    type: drum_voice
    parameters:
      level: 0.5
      sample: kick
  - id: out
    type: audio_output
connections:
  - from: voice.audio
    to: out.left
"#,
        )
        .expect("composite module definition YAML should parse");

        assert_eq!(patch.module_definitions.len(), 1);
        let definition = &patch.module_definitions[0];
        assert_eq!(definition.module_type, "drum_voice");
        assert_eq!(definition.inputs[0].name, "trigger");
        assert_eq!(definition.inputs[0].signal_type, SignalType::Event);
        assert_eq!(definition.inputs[0].maps_to[0].module_id, "env");
        assert_eq!(definition.inputs[0].maps_to[0].port_name, "gate");
        assert_eq!(definition.outputs[0].name, "audio");
        assert_eq!(definition.outputs[0].maps_from[0].module_id, "vca");
        assert_eq!(definition.parameters[0].name, "level");
        assert_eq!(definition.parameters[0].maps_to[0].module_id, "vca");
        assert_eq!(definition.asset_bindings[0].name, "sample");
        assert_eq!(definition.asset_bindings[0].maps_to[0].port_name, "asset");
        assert_eq!(definition.modules[0].id, "osc");
        assert_eq!(definition.connections[0].from.module_id, "osc");
    }

    #[test]
    fn missing_voice_allocation_defaults_to_monophonic_without_stealing() {
        let patch = load_patch_str(
            r#"
metadata:
  name: Monophonic Default
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
modules:
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
"#,
        )
        .expect("patch without voice allocation should parse");

        assert_eq!(patch.voice_allocation.max_voices, 1);
        assert_eq!(
            patch.voice_allocation.stealing,
            VoiceStealingPolicy::Disabled
        );
        validate_patch_schema(&patch).expect("default voice allocation should validate");
    }

    #[test]
    fn loads_polyphonic_voice_allocation_from_yaml() {
        let patch = load_patch_str(
            r#"
metadata:
  name: Polyphonic
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
voice_allocation:
  max_voices: 8
  stealing: oldest_active
modules:
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
"#,
        )
        .expect("polyphonic voice allocation should parse");

        assert_eq!(patch.voice_allocation.max_voices, 8);
        assert_eq!(
            patch.voice_allocation.stealing,
            VoiceStealingPolicy::OldestActive
        );
        validate_patch_schema(&patch).expect("positive polyphony should validate");
    }

    #[test]
    fn zero_max_voices_is_rejected_by_validation() {
        let patch = load_patch_str(
            r#"
metadata:
  name: Zero Voices
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
voice_allocation:
  max_voices: 0
modules:
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
"#,
        )
        .expect("zero voice limit should parse for validation");

        let error = validate_patch_schema(&patch).expect_err("zero max_voices must fail");

        assert!(error.diagnostics().contains(
            &"voice_allocation.max_voices must be greater than zero".to_string()
        ));
    }

    #[test]
    fn negative_max_voices_is_rejected_at_parse_time() {
        let error = load_patch_str(
            r#"
metadata:
  name: Negative Voices
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 48000
voice_allocation:
  max_voices: -1
modules:
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
"#,
        )
        .expect_err("negative voice limit must fail at parse time");

        assert!(matches!(error, PatchLoadError::ParseFailed { .. }));
        assert!(error.to_string().contains("voice_allocation.max_voices"));
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
    fn schema_validation_reports_duplicate_composite_module_types() {
        let mut patch = minimal_patch(vec![ModuleDeclaration {
            id: "out".to_string(),
            module_type: "audio_output".to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: BTreeMap::new(),
        }]);
        patch.module_definitions = vec![
            minimal_composite_definition("drum_voice"),
            minimal_composite_definition("drum_voice"),
        ];

        let error = validate_patch_schema(&patch).expect_err("duplicate composite types must fail");

        assert!(error.diagnostics().contains(
            &"duplicate composite module type: drum_voice".to_string()
        ));
    }

    #[test]
    fn schema_validation_reports_malformed_composite_public_and_internal_references() {
        let mut patch = minimal_patch(vec![ModuleDeclaration {
            id: "voice".to_string(),
            module_type: "drum_voice".to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: BTreeMap::new(),
        }]);
        let mut definition = minimal_composite_definition("drum_voice");
        definition.inputs[0].name = String::new();
        definition.inputs[0].maps_to[0].module_id = String::new();
        definition.outputs[0].name = String::new();
        definition.outputs[0].maps_from[0].port_name = String::new();
        patch.module_definitions = vec![definition];

        let error = validate_patch_schema(&patch).expect_err("malformed composite refs must fail");

        assert!(error.diagnostics().contains(
            &"composite drum_voice input name is required".to_string()
        ));
        assert!(error.diagnostics().contains(
            &"composite drum_voice input <unnamed> maps_to must use a non-empty module_id.port_name reference".to_string()
        ));
        assert!(error.diagnostics().contains(
            &"composite drum_voice output name is required".to_string()
        ));
        assert!(error.diagnostics().contains(
            &"composite drum_voice output <unnamed> maps_from must use a non-empty module_id.port_name reference".to_string()
        ));
    }

    #[test]
    fn schema_validation_accepts_compatible_composite_public_mappings() {
        let mut patch = minimal_patch(vec![ModuleDeclaration {
            id: "voice".to_string(),
            module_type: "drum_voice".to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: BTreeMap::new(),
        }]);
        patch.module_definitions = vec![minimal_composite_definition("drum_voice")];

        validate_patch_schema(&patch).expect("compatible composite mappings should validate");
    }

    #[test]
    fn schema_validation_reports_incompatible_composite_public_mappings() {
        let mut patch = minimal_patch(vec![ModuleDeclaration {
            id: "voice".to_string(),
            module_type: "drum_voice".to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: BTreeMap::new(),
        }]);
        let mut definition = minimal_composite_definition("drum_voice");
        definition.inputs[0].maps_to[0] = PortReference {
            module_id: "vca".to_string(),
            port_name: "gain".to_string(),
        };
        definition.outputs[0].maps_from[0] = PortReference {
            module_id: "env".to_string(),
            port_name: "value".to_string(),
        };
        patch.module_definitions = vec![definition];

        let error = validate_patch_schema(&patch).expect_err("incompatible mappings must fail");

        assert!(error.diagnostics().contains(
            &"composite drum_voice input trigger maps_to vca.gain has incompatible signal types: public Event, internal Control".to_string()
        ));
        assert!(error.diagnostics().contains(
            &"composite drum_voice output audio maps_from env.value has incompatible signal types: public Audio, internal Control".to_string()
        ));
    }

    #[test]
    fn schema_validation_reports_composite_mapping_wrong_internal_port_direction() {
        let mut patch = minimal_patch(vec![ModuleDeclaration {
            id: "voice".to_string(),
            module_type: "drum_voice".to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: BTreeMap::new(),
        }]);
        let mut definition = minimal_composite_definition("drum_voice");
        definition.inputs[0].maps_to[0] = PortReference {
            module_id: "env".to_string(),
            port_name: "value".to_string(),
        };
        definition.outputs[0].maps_from[0] = PortReference {
            module_id: "vca".to_string(),
            port_name: "audio_in".to_string(),
        };
        patch.module_definitions = vec![definition];

        let error = validate_patch_schema(&patch).expect_err("wrong internal directions must fail");

        assert!(error.diagnostics().contains(
            &"composite drum_voice input trigger maps_to env.value must reference an internal input port".to_string()
        ));
        assert!(error.diagnostics().contains(
            &"composite drum_voice output audio maps_from vca.audio_in must reference an internal output port".to_string()
        ));
    }

    #[test]
    fn schema_validation_reports_undeclared_composite_instance_parameter() {
        let mut patch = minimal_patch(vec![ModuleDeclaration {
            id: "voice".to_string(),
            module_type: "drum_voice".to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: BTreeMap::from([("loudness".to_string(), ParameterValue::Number(0.5))]),
        }]);
        patch.module_definitions = vec![minimal_composite_definition("drum_voice")];

        let error = validate_patch_schema(&patch).expect_err("undeclared binding must fail");

        assert!(error.diagnostics().contains(
            &"composite drum_voice instance voice sets undeclared parameter loudness".to_string()
        ));
    }

    #[test]
    fn schema_validation_accepts_declared_composite_parameter_binding() {
        let mut patch = minimal_patch(vec![ModuleDeclaration {
            id: "voice".to_string(),
            module_type: "drum_voice".to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: BTreeMap::from([("level".to_string(), ParameterValue::Number(0.5))]),
        }]);
        let mut definition = minimal_composite_definition("drum_voice");
        definition.parameters = vec![CompositeBindingDeclaration {
            name: "level".to_string(),
            maps_to: vec![PortReference {
                module_id: "vca".to_string(),
                port_name: "gain".to_string(),
            }],
        }];
        patch.module_definitions = vec![definition];

        validate_patch_schema(&patch).expect("declared parameter binding should validate");
    }

    #[test]
    fn schema_validation_accepts_declared_composite_asset_binding() {
        let mut patch = minimal_patch(vec![ModuleDeclaration {
            id: "voice".to_string(),
            module_type: "drum_voice".to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: BTreeMap::from([("sample".to_string(), ParameterValue::Text("kick".to_string()))]),
        }]);
        patch.assets.push(AssetDeclaration {
            id: "kick".to_string(),
            kind: AssetKind::Sample,
            path: "kick.wav".to_string(),
        });
        let mut definition = minimal_composite_definition("drum_voice");
        definition.asset_bindings = vec![CompositeBindingDeclaration {
            name: "sample".to_string(),
            maps_to: vec![PortReference {
                module_id: "sampler".to_string(),
                port_name: "asset".to_string(),
            }],
        }];
        definition.modules.push(ModuleDeclaration {
            id: "sampler".to_string(),
            module_type: "sampler".to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: BTreeMap::new(),
        });
        patch.module_definitions = vec![definition];

        validate_patch_schema(&patch).expect("declared asset binding should validate");
    }

    #[test]
    fn schema_validation_reports_composite_asset_binding_with_non_text_value() {
        let patch = composite_asset_binding_patch(ParameterValue::Number(1.0));

        let error = validate_patch_schema(&patch).expect_err("non-text asset binding must fail");

        assert!(error.diagnostics().contains(
            &"composite drum_voice instance voice asset binding sample must be a text asset ID".to_string()
        ));
    }

    #[test]
    fn schema_validation_reports_composite_asset_binding_with_missing_asset() {
        let patch = composite_asset_binding_patch(ParameterValue::Text("missing".to_string()));

        let error = validate_patch_schema(&patch).expect_err("missing asset binding must fail");

        assert!(error.diagnostics().contains(
            &"composite drum_voice instance voice asset binding sample references missing asset missing".to_string()
        ));
    }

    #[test]
    fn schema_validation_reports_composite_asset_binding_with_non_sample_asset() {
        let mut patch = composite_asset_binding_patch(ParameterValue::Text("script".to_string()));
        patch.assets.push(AssetDeclaration {
            id: "script".to_string(),
            kind: AssetKind::Script,
            path: "script.dan".to_string(),
        });

        let error = validate_patch_schema(&patch).expect_err("non-sample asset binding must fail");

        assert!(error.diagnostics().iter().any(|diagnostic| {
            diagnostic.contains("composite drum_voice instance voice asset binding sample")
                && diagnostic.contains("asset script")
                && diagnostic.contains("expected sample")
        }));
    }

    #[test]
    fn schema_validation_reports_direct_recursive_composite_definition() {
        let mut patch = minimal_patch(vec![ModuleDeclaration {
            id: "voice".to_string(),
            module_type: "drum_voice".to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: BTreeMap::new(),
        }]);
        let mut definition = minimal_composite_definition("drum_voice");
        definition.modules.push(ModuleDeclaration {
            id: "child".to_string(),
            module_type: "drum_voice".to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: BTreeMap::new(),
        });
        patch.module_definitions = vec![definition];

        let error = validate_patch_schema(&patch).expect_err("direct recursion must fail");

        assert!(error.diagnostics().contains(
            &"recursive composite definition: drum_voice -> drum_voice".to_string()
        ));
    }

    #[test]
    fn schema_validation_reports_indirect_recursive_composite_definition() {
        let mut patch = minimal_patch(vec![ModuleDeclaration {
            id: "voice".to_string(),
            module_type: "a".to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: BTreeMap::new(),
        }]);
        let mut a = minimal_composite_definition("a");
        a.modules.push(ModuleDeclaration {
            id: "b_child".to_string(),
            module_type: "b".to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: BTreeMap::new(),
        });
        let mut b = minimal_composite_definition("b");
        b.modules.push(ModuleDeclaration {
            id: "a_child".to_string(),
            module_type: "a".to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: BTreeMap::new(),
        });
        patch.module_definitions = vec![a, b];

        let error = validate_patch_schema(&patch).expect_err("indirect recursion must fail");

        assert!(error.diagnostics().contains(
            &"recursive composite definition: a -> b -> a".to_string()
        ));
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
            module_definitions: vec![],
            modules: vec![],
            connections: vec![],
            voice_allocation: VoiceAllocation::default(),
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
            module_definitions: vec![],
            modules,
            connections: vec![],
            voice_allocation: VoiceAllocation::default(),
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

    fn minimal_composite_definition(module_type: &str) -> ModuleDefinitionDeclaration {
        ModuleDefinitionDeclaration {
            module_type: module_type.to_string(),
            inputs: vec![CompositeInputDeclaration {
                name: "trigger".to_string(),
                signal_type: SignalType::Event,
                maps_to: vec![PortReference {
                    module_id: "env".to_string(),
                    port_name: "gate".to_string(),
                }],
            }],
            outputs: vec![CompositeOutputDeclaration {
                name: "audio".to_string(),
                signal_type: SignalType::Audio,
                maps_from: vec![PortReference {
                    module_id: "vca".to_string(),
                    port_name: "audio_out".to_string(),
                }],
            }],
            parameters: vec![],
            asset_bindings: vec![],
            modules: vec![
                ModuleDeclaration {
                    id: "env".to_string(),
                    module_type: "adsr".to_string(),
                    inputs: vec![],
                    outputs: vec![],
                    parameters: BTreeMap::new(),
                },
                ModuleDeclaration {
                    id: "vca".to_string(),
                    module_type: "gain".to_string(),
                    inputs: vec![],
                    outputs: vec![],
                    parameters: BTreeMap::new(),
                },
            ],
            connections: vec![],
        }
    }

    fn composite_asset_binding_patch(value: ParameterValue) -> PatchDocument {
        let mut patch = minimal_patch(vec![ModuleDeclaration {
            id: "voice".to_string(),
            module_type: "drum_voice".to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: BTreeMap::from([("sample".to_string(), value)]),
        }]);
        let mut definition = minimal_composite_definition("drum_voice");
        definition.asset_bindings = vec![CompositeBindingDeclaration {
            name: "sample".to_string(),
            maps_to: vec![PortReference {
                module_id: "sampler".to_string(),
                port_name: "asset".to_string(),
            }],
        }];
        definition.modules.push(ModuleDeclaration {
            id: "sampler".to_string(),
            module_type: "sampler".to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: BTreeMap::new(),
        });
        patch.module_definitions = vec![definition];
        patch
    }
}
