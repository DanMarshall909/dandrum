use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::builtins::{
    BuiltInModuleDefinition, BuiltInModuleRegistry, ParameterMetadata, ParameterValueType,
    SCRIPT_LANGUAGE_PARAMETER, SCRIPT_LANGUAGE_RHAI, SCRIPT_SOURCE_PARAMETER, module_types,
};
use crate::diagnostics::{self, Diagnostic, Diagnostics, Severity, error_codes};
use crate::script::{RhaiScriptRuntime, ScriptPrepareError, ScriptRuntimeLimits};

#[path = "patch_module.rs"]
mod patch_module;

pub use patch_module::{
    ModuleBindingDeclaration, ModuleDefinitionDeclaration, ModuleInputDeclaration,
    ModuleOutputDeclaration,
};

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PatchDocument {
    pub metadata: PatchMetadata,
    #[serde(default)]
    pub instrument: Option<InstrumentIdentity>,
    #[serde(default)]
    pub preset_surface: PresetSurfaceDeclaration,
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
    /// Top-level parameter overrides keyed by module_id.
    #[serde(default)]
    pub parameters: BTreeMap<String, BTreeMap<String, ParameterValue>>,
    /// Named preset parameter sets. Each preset maps module_id -> { param_name: value }.
    #[serde(default)]
    pub presets: BTreeMap<String, BTreeMap<String, BTreeMap<String, ParameterValue>>>,
    /// Optional active preset name. Declared presets are inert unless selected here.
    #[serde(default)]
    pub selected_preset: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct InstrumentIdentity {
    pub id: String,
    pub preset_schema_version: u32,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct PresetSurfaceDeclaration {
    #[serde(default)]
    pub parameters: Vec<PresetParameterTargetDeclaration>,
    #[serde(default)]
    pub assets: Vec<PresetAssetTargetDeclaration>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PresetDocument {
    pub name: String,
    pub instrument: InstrumentIdentity,
    #[serde(default)]
    pub values: BTreeMap<String, ParameterValue>,
    #[serde(default)]
    pub assets: BTreeMap<String, String>,
    #[serde(default)]
    pub metadata: Option<PresetMetadata>,
    #[serde(flatten)]
    pub extra_fields: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct PresetMetadata {
    pub author: Option<String>,
    pub category: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PresetParameterTargetDeclaration {
    pub name: String,
    #[serde(rename = "type")]
    pub value_type: PresetTargetType,
    pub default: ParameterValue,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    pub maps_to: PortReference,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PresetAssetTargetDeclaration {
    pub name: String,
    pub kind: AssetKind,
    pub default: String,
    pub maps_to: PortReference,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PresetTargetType {
    Boolean,
    Number,
    Integer,
    Text,
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
    #[serde(flatten)]
    pub extra_fields: BTreeMap<String, serde_yaml::Value>,
}

const EVENT_ROUTING_SIGNAL_CHAIN_FIELDS: &[&str] = &[
    "module_definitions",
    "modules",
    "connections",
    "assets",
    "audio_outputs",
    "mix_outputs",
];
const EVENT_ROUTING_SEQUENCING_FIELDS: &[&str] = &[
    "pattern",
    "patterns",
    "steps",
    "tempo",
    "transport",
    "clock",
];
const SCRIPT_SOURCE_FIELD: &str = "source";
const SCRIPT_DISALLOWED_API_TOKENS: &[&str] = &[
    "std::fs",
    "fs::",
    "read_file",
    "write_file",
    "network",
    "socket",
    "sleep",
    "thread::",
    "random",
    "alloc",
];
const PRESET_STRUCTURAL_FIELDS: &[&str] = &[
    "module_definitions",
    "modules",
    "connections",
    "render",
    "events",
    "event_sequence",
    "scripts",
    "scheduling",
    "schedule",
    "feedback",
];

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

#[derive(Debug)]
pub enum PresetLoadError {
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

#[derive(Clone, Debug, PartialEq)]
pub struct PatchValidationError {
    diagnostics: Diagnostics,
}

impl PatchValidationError {
    pub fn new() -> Self {
        Self {
            diagnostics: Diagnostics::new(),
        }
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn extend(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
        self.diagnostics.extend(diagnostics);
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }
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

pub fn load_preset_file(path: impl AsRef<Path>) -> Result<PresetDocument, PresetLoadError> {
    let path = path.as_ref();

    if !is_yaml_path(path) {
        return Err(PresetLoadError::UnsupportedFormat {
            path: path.to_path_buf(),
        });
    }

    let yaml = fs::read_to_string(path).map_err(|error| PresetLoadError::ReadFailed {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;

    load_preset_str(&yaml).map_err(|error| match error {
        PresetLoadError::ParseFailed { message, .. } => PresetLoadError::ParseFailed {
            path: Some(path.to_path_buf()),
            message,
        },
        error => error,
    })
}

pub fn load_preset_str(yaml: &str) -> Result<PresetDocument, PresetLoadError> {
    serde_yaml::from_str(yaml).map_err(|error| PresetLoadError::ParseFailed {
        path: None,
        message: error.to_string(),
    })
}

pub fn validate_preset_compatibility(
    patch: &PatchDocument,
    preset: &PresetDocument,
) -> Result<(), PatchValidationError> {
    let mut result = PatchValidationError::new();

    let Some(instrument) = patch.instrument.as_ref() else {
        result.push(Diagnostic::new(
            error_codes::VALIDATION_MISSING_FIELD,
            Severity::Error,
            "patch does not declare instrument preset identity",
        ));
        return Err(result);
    };

    if instrument.id != preset.instrument.id {
        result.push(
            Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Error,
                format!(
                    "preset targets instrument {}, but patch instrument is {}",
                    preset.instrument.id, instrument.id
                ),
            )
            .with_expected(&instrument.id)
            .with_actual(&preset.instrument.id),
        );
    }

    if instrument.preset_schema_version != preset.instrument.preset_schema_version {
        result.push(
            Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Error,
                format!(
                    "preset schema version {} does not match patch preset schema version {}",
                    preset.instrument.preset_schema_version, instrument.preset_schema_version
                ),
            )
            .with_expected(instrument.preset_schema_version.to_string())
            .with_actual(preset.instrument.preset_schema_version.to_string()),
        );
    }

    if result.is_empty() {
        Ok(())
    } else {
        Err(result)
    }
}

pub fn validate_preset(
    patch: &PatchDocument,
    preset: &PresetDocument,
) -> Result<(), PatchValidationError> {
    let mut result = PatchValidationError::new();

    if let Err(error) = validate_preset_compatibility(patch, preset) {
        result.extend(error.diagnostics().iter().cloned());
    }

    validate_preset_structural_fields(preset, &mut result);
    validate_preset_values(patch, preset, &mut result);
    validate_preset_asset_values(patch, preset, &mut result);

    if result.is_empty() {
        Ok(())
    } else {
        Err(result)
    }
}

pub fn apply_preset(
    patch: &PatchDocument,
    preset: &PresetDocument,
) -> Result<PatchDocument, PatchValidationError> {
    validate_preset(patch, preset)?;

    let mut patched = patch.clone();

    for target in &patch.preset_surface.parameters {
        apply_preset_parameter_value(&mut patched, &target.maps_to, target.default.clone());
    }
    for (name, value) in &preset.values {
        let target = patch
            .preset_surface
            .parameters
            .iter()
            .find(|target| target.name == *name)
            .expect("preset validation should reject unknown parameter targets");
        apply_preset_parameter_value(&mut patched, &target.maps_to, value.clone());
    }

    for target in &patch.preset_surface.assets {
        apply_preset_parameter_value(
            &mut patched,
            &target.maps_to,
            ParameterValue::Text(target.default.clone()),
        );
    }
    for (name, asset_id) in &preset.assets {
        let target = patch
            .preset_surface
            .assets
            .iter()
            .find(|target| target.name == *name)
            .expect("preset validation should reject unknown asset targets");
        apply_preset_parameter_value(
            &mut patched,
            &target.maps_to,
            ParameterValue::Text(asset_id.clone()),
        );
    }

    Ok(patched)
}

fn apply_preset_parameter_value(
    patch: &mut PatchDocument,
    destination: &PortReference,
    value: ParameterValue,
) {
    let Some(module) = patch
        .modules
        .iter_mut()
        .find(|module| module.id == destination.module_id)
    else {
        return;
    };

    module
        .parameters
        .insert(destination.port_name.clone(), value);
}

fn validate_preset_structural_fields(
    preset: &PresetDocument,
    diagnostics: &mut PatchValidationError,
) {
    for field in preset.extra_fields.keys() {
        if PRESET_STRUCTURAL_FIELDS.contains(&field.as_str()) {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Error,
                format!(
                    "preset document cannot declare structural field {field}; graph structure belongs in the patch"
                ),
            ));
        }
    }
}

fn validate_preset_values(
    patch: &PatchDocument,
    preset: &PresetDocument,
    diagnostics: &mut PatchValidationError,
) {
    for (name, value) in &preset.values {
        let Some(target) = patch
            .preset_surface
            .parameters
            .iter()
            .find(|target| target.name == *name)
        else {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Error,
                format!("unknown preset target {name}"),
            ));
            continue;
        };

        if !preset_value_matches_type(value, target.value_type) {
            diagnostics.push(
                Diagnostic::new(
                    error_codes::VALIDATION_TYPE_MISMATCH,
                    Severity::Error,
                    format!(
                        "preset target {name} has incompatible value type: expected {}, got {}",
                        preset_target_type_name(target.value_type),
                        parameter_value_type_name(value)
                    ),
                )
                .with_expected(preset_target_type_name(target.value_type))
                .with_actual(parameter_value_type_name(value)),
            );
            continue;
        }

        if let (ParameterValue::Number(actual), Some(min)) = (value, target.min) {
            if *actual < min {
                diagnostics.push(Diagnostic::new(
                    error_codes::VALIDATION_INVALID_VALUE,
                    Severity::Error,
                    format!("preset target {name} is below minimum {min}: {actual}"),
                ));
            }
        }
        if let (ParameterValue::Number(actual), Some(max)) = (value, target.max) {
            if *actual > max {
                diagnostics.push(Diagnostic::new(
                    error_codes::VALIDATION_INVALID_VALUE,
                    Severity::Error,
                    format!("preset target {name} is above maximum {max}: {actual}"),
                ));
            }
        }
    }
}

fn validate_preset_asset_values(
    patch: &PatchDocument,
    preset: &PresetDocument,
    diagnostics: &mut PatchValidationError,
) {
    for (name, asset_id) in &preset.assets {
        let Some(target) = patch
            .preset_surface
            .assets
            .iter()
            .find(|target| target.name == *name)
        else {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Error,
                format!("unknown preset target {name}"),
            ));
            continue;
        };

        let Some(asset) = patch.assets.iter().find(|asset| asset.id == *asset_id) else {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Error,
                format!("preset target {name} references missing asset {asset_id}"),
            ));
            continue;
        };

        if asset.kind != target.kind {
            diagnostics.push(
                Diagnostic::new(
                    error_codes::VALIDATION_TYPE_MISMATCH,
                    Severity::Error,
                    format!(
                        "preset target {name} expected {:?} asset, got {:?}",
                        target.kind, asset.kind
                    ),
                )
                .with_expected(format!("{:?}", target.kind).to_lowercase())
                .with_actual(format!("{:?}", asset.kind).to_lowercase()),
            );
        }
    }
}

pub fn resolve_module_parameters(
    patch: &PatchDocument,
) -> Result<BTreeMap<String, BTreeMap<String, ParameterValue>>, PatchValidationError> {
    validate_patch_schema(patch)?;

    let registry = BuiltInModuleRegistry::new();
    let selected_preset = patch
        .selected_preset
        .as_deref()
        .and_then(|name| patch.presets.get(name));
    let mut resolved = BTreeMap::new();

    for module in &patch.modules {
        let mut values = BTreeMap::new();

        if let Some(definition) = registry.get(&module.module_type) {
            for parameter in definition.parameters() {
                if let Some(default) = default_parameter_value(parameter) {
                    values.insert(parameter.name().to_string(), default);
                }
            }
        }

        for (name, value) in &module.parameters {
            values.insert(name.clone(), value.clone());
        }

        if let Some(preset_modules) = selected_preset {
            if let Some(overrides) = preset_modules.get(&module.id) {
                for (name, value) in overrides {
                    values.insert(name.clone(), value.clone());
                }
            }
        }

        if let Some(overrides) = patch.parameters.get(&module.id) {
            for (name, value) in overrides {
                values.insert(name.clone(), value.clone());
            }
        }

        resolved.insert(module.id.clone(), values);
    }

    Ok(resolved)
}

fn default_parameter_value(metadata: &ParameterMetadata) -> Option<ParameterValue> {
    let default = metadata.default()?;

    match metadata.value_type() {
        ParameterValueType::Integer | ParameterValueType::Number => {
            default.parse::<f64>().ok().map(ParameterValue::Number)
        }
        ParameterValueType::Text => Some(ParameterValue::Text(default.to_string())),
    }
}

pub fn validate_patch_schema(patch: &PatchDocument) -> Result<(), PatchValidationError> {
    let mut result = PatchValidationError::new();
    let registry = BuiltInModuleRegistry::new();

    if patch.metadata.name.trim().is_empty() {
        result.push(Diagnostic::new(
            error_codes::VALIDATION_MISSING_FIELD,
            Severity::Error,
            "metadata.name is required",
        ));
    }

    if patch.render.sample_rate_hz == 0 {
        result.push(Diagnostic::new(
            error_codes::VALIDATION_INVALID_VALUE,
            Severity::Error,
            "render.sample_rate_hz must be greater than zero",
        ));
    }

    if patch.render.block_size_frames == 0 {
        result.push(Diagnostic::new(
            error_codes::VALIDATION_INVALID_VALUE,
            Severity::Error,
            "render.block_size_frames must be greater than zero",
        ));
    }

    if patch.modules.is_empty() {
        result.push(Diagnostic::new(
            error_codes::VALIDATION_MISSING_FIELD,
            Severity::Error,
            "modules must declare at least one module",
        ));
    }

    patch_module::validate_module_definitions(patch, &mut result);

    let mut module_ids = BTreeSet::new();
    for module in &patch.modules {
        if module.id.trim().is_empty() {
            result.push(
                Diagnostic::new(
                    error_codes::VALIDATION_MISSING_FIELD,
                    Severity::Error,
                    "module.id is required",
                )
                .with_module_id(&module.id),
            );
        } else if !module_ids.insert(module.id.as_str()) {
            result.push(
                Diagnostic::new(
                    error_codes::VALIDATION_INVALID_VALUE,
                    Severity::Error,
                    format!("duplicate module id: {}", module.id),
                )
                .with_module_id(&module.id),
            );
        }

        if module.module_type.trim().is_empty() {
            result.push(
                Diagnostic::new(
                    error_codes::VALIDATION_MISSING_FIELD,
                    Severity::Error,
                    format!("module {} type is required", module.id),
                )
                .with_module_id(&module.id),
            );
        }

        for port in module.inputs.iter().chain(module.outputs.iter()) {
            if port.name.trim().is_empty() {
                result.push(
                    Diagnostic::new(
                        error_codes::VALIDATION_MISSING_FIELD,
                        Severity::Error,
                        format!("module {} port name is required", module.id),
                    )
                    .with_module_id(&module.id),
                );
            }
        }

        if module.module_type == module_types::SAMPLER {
            validate_sampler_asset_reference(module, patch, &mut result);
        }

        if module.module_type == module_types::EVENT_FILTER {
            validate_event_routing_module(module, &mut result);
        }

        if module.module_type == module_types::SCRIPT {
            validate_script_module(module, &mut result);
        }

        validate_declared_parameters_for_module(
            "module parameter",
            &module.id,
            &module.module_type,
            &module.parameters,
            &registry,
            &mut result,
        );

        patch_module::validate_module_instance_bindings(module, patch, &mut result);
    }

    if patch.voice_allocation.max_voices == 0 {
        result.push(Diagnostic::new(
            error_codes::VALIDATION_INVALID_VALUE,
            Severity::Error,
            "voice_allocation.max_voices must be greater than zero",
        ));
    }

    validate_selected_preset(patch, &mut result);
    validate_asset_usage(patch, &mut result);
    validate_patch_level_parameters(patch, &registry, &mut result);
    validate_presets(patch, &registry, &mut result);
    validate_preset_surface(patch, &registry, &mut result);

    for connection in &patch.connections {
        validate_port_reference("connection.from", &connection.from, &mut result);
        validate_port_reference("connection.to", &connection.to, &mut result);
    }

    if result.is_empty() {
        Ok(())
    } else {
        Err(result)
    }
}

fn validate_script_module(module: &ModuleDeclaration, diagnostics: &mut PatchValidationError) {
    for input in &module.inputs {
        if input.signal_type == SignalType::Audio {
            diagnostics.push(
                Diagnostic::new(
                    error_codes::SCRIPT_UNSUPPORTED_PORT,
                    Severity::Error,
                    format!(
                        "script module {} input {} cannot be audio-rate in the initial implementation",
                        module.id, input.name
                    ),
                )
                .with_module_id(&module.id)
                .with_port_name(&input.name)
                .with_expected("event or control input")
                .with_actual("audio input")
                .with_suggested_fix("move audio-rate DSP into a Rust primitive or YAML module"),
            );
        }
    }

    for output in &module.outputs {
        if output.signal_type == SignalType::Audio {
            diagnostics.push(
                Diagnostic::new(
                    error_codes::SCRIPT_UNSUPPORTED_PORT,
                    Severity::Error,
                    format!(
                        "script module {} output {} cannot be audio-rate in the initial implementation",
                        module.id, output.name
                    ),
                )
                .with_module_id(&module.id)
                .with_port_name(&output.name)
                .with_expected("event or control output")
                .with_actual("audio output")
                .with_suggested_fix("move audio-rate DSP into a Rust primitive or YAML module"),
            );
        }
    }

    if let Some(language) = module.parameters.get(SCRIPT_LANGUAGE_PARAMETER) {
        match language {
            ParameterValue::Text(value) if value == SCRIPT_LANGUAGE_RHAI => {}
            ParameterValue::Text(value) => {
                diagnostics.push(
                    Diagnostic::new(
                        error_codes::SCRIPT_VALIDATION,
                        Severity::Error,
                        format!(
                            "script module {} language {value} is not supported",
                            module.id
                        ),
                    )
                    .with_module_id(&module.id)
                    .with_expected(SCRIPT_LANGUAGE_RHAI)
                    .with_actual(value)
                    .with_suggested_fix("use language: rhai"),
                );
                return;
            }
            other => {
                diagnostics.push(
                    Diagnostic::new(
                        error_codes::VALIDATION_TYPE_MISMATCH,
                        Severity::Error,
                        format!("script module {} language must be a string", module.id),
                    )
                    .with_module_id(&module.id)
                    .with_expected("string")
                    .with_actual(format!("{other:?}")),
                );
                return;
            }
        }
    }

    let source = match script_source(module) {
        Some(Ok(source)) => source,
        Some(Err(diagnostic)) => {
            diagnostics.push(diagnostic);
            return;
        }
        None => {
            if module.parameters.contains_key(SCRIPT_LANGUAGE_PARAMETER) {
                diagnostics.push(
                    Diagnostic::new(
                        error_codes::VALIDATION_MISSING_FIELD,
                        Severity::Error,
                        format!("script module {} source is required", module.id),
                    )
                    .with_module_id(&module.id)
                    .with_expected("inline source string"),
                );
            }
            return;
        }
    };

    match RhaiScriptRuntime::compile(source, ScriptRuntimeLimits::default()) {
        Ok(_) => {}
        Err(ScriptPrepareError::Parse { message }) => diagnostics.push(
            Diagnostic::new(
                error_codes::SCRIPT_PARSE,
                Severity::Error,
                format!("script module {} Rhai source failed to parse", module.id),
            )
            .with_module_id(&module.id)
            .with_expected("valid Rhai source")
            .with_actual(message),
        ),
        Err(ScriptPrepareError::MissingEntryPoint) => diagnostics.push(
            Diagnostic::new(
                error_codes::SCRIPT_VALIDATION,
                Severity::Error,
                format!("script module {} must define process(ctx)", module.id),
            )
            .with_module_id(&module.id)
            .with_expected("fn process(ctx)"),
        ),
    }

    for token in SCRIPT_DISALLOWED_API_TOKENS {
        if source.contains(token) {
            diagnostics.push(
                Diagnostic::new(
                    error_codes::SCRIPT_UNSUPPORTED_API,
                    Severity::Error,
                    format!(
                        "script module {} uses unsupported API token {token}",
                        module.id
                    ),
                )
                .with_module_id(&module.id)
                .with_expected("deterministic event/control script without filesystem, network, blocking, random, or allocation APIs")
                .with_actual(*token)
                .with_suggested_fix("remove the unsupported API call or implement the behaviour as a Rust primitive"),
            );
        }
    }
}

fn script_source(module: &ModuleDeclaration) -> Option<Result<&str, Diagnostic>> {
    if let Some(source) = module.parameters.get(SCRIPT_SOURCE_PARAMETER) {
        return Some(match source {
            ParameterValue::Text(value) => Ok(value.as_str()),
            other => Err(Diagnostic::new(
                error_codes::VALIDATION_TYPE_MISMATCH,
                Severity::Error,
                format!("script module {} source must be a string", module.id),
            )
            .with_module_id(&module.id)
            .with_expected("string")
            .with_actual(format!("{other:?}"))),
        });
    }

    let Some(source) = module.extra_fields.get(SCRIPT_SOURCE_FIELD) else {
        return None;
    };
    Some(match source.as_str() {
        Some(source) => Ok(source),
        None => Err(Diagnostic::new(
            error_codes::VALIDATION_TYPE_MISMATCH,
            Severity::Error,
            format!("script module {} source must be a string", module.id),
        )
        .with_module_id(&module.id)
        .with_expected("string")
        .with_actual(format!("{:?}", source))),
    })
}

fn validate_event_routing_module(
    module: &ModuleDeclaration,
    diagnostics: &mut PatchValidationError,
) {
    for field in module.extra_fields.keys() {
        if EVENT_ROUTING_SIGNAL_CHAIN_FIELDS.contains(&field.as_str()) {
            diagnostics.push(
                Diagnostic::new(
                    error_codes::VALIDATION_INVALID_VALUE,
                    Severity::Error,
                    format!(
                        "event-routing module {} cannot declare hidden signal-chain field {field}; model signal chains as external patch modules",
                        module.id
                    ),
                )
                .with_module_id(&module.id)
                .with_suggested_fix("move signal-chain behavior into explicit external modules"),
            );
        } else if EVENT_ROUTING_SEQUENCING_FIELDS.contains(&field.as_str()) {
            diagnostics.push(
                Diagnostic::new(
                    error_codes::VALIDATION_INVALID_VALUE,
                    Severity::Error,
                    format!(
                        "event-routing module {} cannot declare sequencing field {field}; model sequencing as explicit external modules",
                        module.id
                    ),
                )
                .with_module_id(&module.id)
                .with_suggested_fix("move sequencing behavior into explicit external modules"),
            );
        } else {
            diagnostics.push(
                Diagnostic::new(
                    error_codes::VALIDATION_INVALID_VALUE,
                    Severity::Error,
                    format!(
                        "event-routing module {} has unsupported field {field}",
                        module.id
                    ),
                )
                .with_module_id(&module.id),
            );
        }
    }
}

fn validate_declared_parameters_for_module(
    source_label: &str,
    module_id: &str,
    module_type: &str,
    parameters: &BTreeMap<String, ParameterValue>,
    registry: &BuiltInModuleRegistry,
    diagnostics: &mut PatchValidationError,
) {
    let Some(definition) = registry.get(module_type) else {
        return;
    };

    for (name, value) in parameters {
        let Some(metadata) = definition.parameters().iter().find(|p| p.name() == name) else {
            diagnostics.push(
                Diagnostic::new(
                    error_codes::VALIDATION_INVALID_VALUE,
                    Severity::Error,
                    format!(
                        "{source_label} {module_id}.{name} is not declared by module type {module_type}"
                    ),
                )
                .with_module_id(module_id)
                .with_expected(declared_parameter_names(definition))
                .with_actual(name)
                .with_suggested_fix(format!(
                    "remove {name} or choose one of the declared parameters for {module_type}"
                )),
            );
            continue;
        };

        validate_parameter_value(source_label, module_id, name, value, metadata, diagnostics);
    }
}

fn validate_parameter_value(
    source_label: &str,
    module_id: &str,
    name: &str,
    value: &ParameterValue,
    metadata: &ParameterMetadata,
    diagnostics: &mut PatchValidationError,
) {
    if !parameter_value_matches_type(value, metadata.value_type()) {
        diagnostics.push(
            Diagnostic::new(
                error_codes::VALIDATION_TYPE_MISMATCH,
                Severity::Error,
                format!(
                    "{source_label} {module_id}.{name} has wrong type: expected {}, got {}",
                    parameter_type_name(metadata.value_type()),
                    parameter_value_type_name(value),
                ),
            )
            .with_module_id(module_id)
            .with_expected(parameter_type_name(metadata.value_type()))
            .with_actual(parameter_value_type_name(value))
            .with_suggested_fix(format!(
                "set {module_id}.{name} to a {} value",
                parameter_type_name(metadata.value_type())
            )),
        );
        return;
    }

    if let Some(values) = metadata.enum_values() {
        if let ParameterValue::Text(actual) = value {
            if !values.iter().any(|allowed| allowed == actual) {
                diagnostics.push(
                    Diagnostic::new(
                        error_codes::VALIDATION_INVALID_VALUE,
                        Severity::Error,
                        format!(
                            "{source_label} {module_id}.{name} has invalid enum value {actual}"
                        ),
                    )
                    .with_module_id(module_id)
                    .with_expected(format!("one of [{}]", values.join(", ")))
                    .with_actual(actual)
                    .with_suggested_fix(format!(
                        "set {module_id}.{name} to one of [{}]",
                        values.join(", ")
                    )),
                );
            }
        }
    }

    if let (ParameterValue::Number(actual), Some((min, max))) = (value, metadata.range()) {
        if *actual < min {
            diagnostics.push(
                Diagnostic::new(
                    error_codes::VALIDATION_INVALID_VALUE,
                    Severity::Error,
                    format!("{source_label} {module_id}.{name} is below minimum {min}: {actual}"),
                )
                .with_module_id(module_id)
                .with_expected(format!(">= {min}"))
                .with_actual(actual.to_string())
                .with_suggested_fix(format!("set {module_id}.{name} to at least {min}")),
            );
        } else if *actual > max {
            diagnostics.push(
                Diagnostic::new(
                    error_codes::VALIDATION_INVALID_VALUE,
                    Severity::Error,
                    format!("{source_label} {module_id}.{name} is above maximum {max}: {actual}"),
                )
                .with_module_id(module_id)
                .with_expected(format!("<= {max}"))
                .with_actual(actual.to_string())
                .with_suggested_fix(format!("set {module_id}.{name} to at most {max}")),
            );
        }
    }
}

fn parameter_value_matches_type(value: &ParameterValue, expected: ParameterValueType) -> bool {
    match (value, expected) {
        (ParameterValue::Number(value), ParameterValueType::Integer) => value.fract() == 0.0,
        (ParameterValue::Number(_), ParameterValueType::Number) => true,
        (ParameterValue::Text(_), ParameterValueType::Text) => true,
        _ => false,
    }
}

fn parameter_type_name(value_type: ParameterValueType) -> &'static str {
    match value_type {
        ParameterValueType::Integer => "integer",
        ParameterValueType::Number => "number",
        ParameterValueType::Text => "string",
    }
}

fn parameter_value_type_name(value: &ParameterValue) -> &'static str {
    match value {
        ParameterValue::Boolean(_) => "boolean",
        ParameterValue::Number(_) => "number",
        ParameterValue::Text(_) => "string",
    }
}

fn declared_parameter_names(definition: &BuiltInModuleDefinition) -> String {
    let names = definition
        .parameters()
        .iter()
        .map(|parameter| parameter.name())
        .collect::<Vec<_>>();

    if names.is_empty() {
        "no declared parameters".to_string()
    } else {
        format!("one of [{}]", names.join(", "))
    }
}

fn validate_sampler_asset_reference(
    module: &ModuleDeclaration,
    patch: &PatchDocument,
    diagnostics: &mut PatchValidationError,
) {
    let Some(asset_parameter) = module.parameters.get("asset") else {
        diagnostics.push(
            Diagnostic::new(
                error_codes::VALIDATION_MISSING_FIELD,
                Severity::Error,
                format!(
                    "sampler module {} missing required asset parameter",
                    module.id
                ),
            )
            .with_module_id(&module.id),
        );
        return;
    };

    let ParameterValue::Text(asset_id) = asset_parameter else {
        diagnostics.push(
            Diagnostic::new(
                error_codes::VALIDATION_TYPE_MISMATCH,
                Severity::Error,
                format!(
                    "sampler module {} asset parameter must be a text asset ID",
                    module.id
                ),
            )
            .with_module_id(&module.id),
        );
        return;
    };

    let Some(asset) = patch.assets.iter().find(|asset| asset.id == *asset_id) else {
        diagnostics.push(
            Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Error,
                format!(
                    "sampler module {} references missing asset {}",
                    module.id, asset_id
                ),
            )
            .with_module_id(&module.id)
            .with_expected(asset_id),
        );
        return;
    };

    if asset.kind != AssetKind::Sample {
        diagnostics.push(
            Diagnostic::new(
                error_codes::VALIDATION_TYPE_MISMATCH,
                Severity::Error,
                format!(
                    "sampler module {} references asset {} with kind {:?}; expected sample",
                    module.id, asset_id, asset.kind
                ),
            )
            .with_module_id(&module.id)
            .with_expected("sample")
            .with_actual(&format!("{:?}", asset.kind)),
        );
    }
}

fn collect_referenced_asset_ids<'a>(patch: &'a PatchDocument) -> BTreeSet<&'a str> {
    let mut ids: BTreeSet<&'a str> = BTreeSet::new();

    for module in &patch.modules {
        if let Some(ParameterValue::Text(asset_id)) = module.parameters.get("asset") {
            ids.insert(asset_id.as_str());
        }
        if let Some(definition) = patch
            .module_definitions
            .iter()
            .find(|d| d.module_type == module.module_type)
        {
            for binding in &definition.asset_bindings {
                if let Some(ParameterValue::Text(asset_id)) =
                    module.parameters.get(binding.name.as_str())
                {
                    ids.insert(asset_id.as_str());
                }
            }
        }
    }

    for definition in &patch.module_definitions {
        for module in &definition.modules {
            if let Some(ParameterValue::Text(asset_id)) = module.parameters.get("asset") {
                ids.insert(asset_id.as_str());
            }
        }
    }

    ids
}

fn validate_asset_usage(patch: &PatchDocument, diagnostics: &mut PatchValidationError) {
    let referenced = collect_referenced_asset_ids(patch);

    for asset in &patch.assets {
        if !referenced.contains(asset.id.as_str()) {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Warning,
                format!(
                    "unused asset {} with kind {:?} declared but not referenced by any module",
                    asset.id, asset.kind
                ),
            ));
        }
    }
}

fn validate_selected_preset(patch: &PatchDocument, diagnostics: &mut PatchValidationError) {
    let Some(name) = patch.selected_preset.as_deref() else {
        return;
    };

    if name.trim().is_empty() {
        diagnostics.push(Diagnostic::new(
            error_codes::VALIDATION_MISSING_FIELD,
            Severity::Error,
            "selected_preset must not be empty",
        ));
        return;
    }

    if !patch.presets.contains_key(name) {
        diagnostics.push(Diagnostic::new(
            error_codes::VALIDATION_INVALID_VALUE,
            Severity::Error,
            format!("selected_preset {name} is not declared"),
        ));
    }
}

fn validate_patch_level_parameters(
    patch: &PatchDocument,
    registry: &BuiltInModuleRegistry,
    diagnostics: &mut PatchValidationError,
) {
    for (module_id, params) in &patch.parameters {
        let Some(module) = patch.modules.iter().find(|m| m.id == *module_id) else {
            diagnostics.push(
                Diagnostic::new(
                    error_codes::VALIDATION_INVALID_VALUE,
                    Severity::Error,
                    format!("patch parameters reference unknown module {module_id}"),
                )
                .with_module_id(module_id),
            );
            continue;
        };

        for param_name in params.keys() {
            if module.parameters.contains_key(param_name) {
                diagnostics.push(
                    Diagnostic::new(
                        error_codes::VALIDATION_INVALID_VALUE,
                        Severity::Error,
                        format!(
                            "patch parameter {module_id}.{param_name} conflicts with module-level parameter"
                        ),
                    )
                    .with_module_id(module_id),
                );
            }
        }

        validate_declared_parameters_for_module(
            "patch parameter",
            module_id,
            &module.module_type,
            params,
            registry,
            diagnostics,
        );
    }
}

fn validate_presets(
    patch: &PatchDocument,
    registry: &BuiltInModuleRegistry,
    diagnostics: &mut PatchValidationError,
) {
    for (preset_name, modules) in &patch.presets {
        if preset_name.trim().is_empty() {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_MISSING_FIELD,
                Severity::Error,
                "preset name must not be empty",
            ));
        }

        for (module_id, params) in modules {
            let Some(module) = patch.modules.iter().find(|m| m.id == *module_id) else {
                diagnostics.push(
                    Diagnostic::new(
                        error_codes::VALIDATION_INVALID_VALUE,
                        Severity::Error,
                        format!("preset {preset_name} references unknown module {module_id}"),
                    )
                    .with_module_id(module_id),
                );
                continue;
            };

            validate_declared_parameters_for_module(
                "preset parameter",
                module_id,
                &module.module_type,
                params,
                registry,
                diagnostics,
            );
        }
    }
}

fn validate_preset_surface(
    patch: &PatchDocument,
    registry: &BuiltInModuleRegistry,
    diagnostics: &mut PatchValidationError,
) {
    let mut target_names = BTreeSet::new();

    for target in &patch.preset_surface.parameters {
        validate_preset_target_name(&target.name, &mut target_names, diagnostics);

        if !preset_value_matches_type(&target.default, target.value_type) {
            diagnostics.push(
                Diagnostic::new(
                    error_codes::VALIDATION_TYPE_MISMATCH,
                    Severity::Error,
                    format!(
                        "preset target {} default has incompatible type",
                        target.name
                    ),
                )
                .with_expected(preset_target_type_name(target.value_type))
                .with_actual(parameter_value_type_name(&target.default)),
            );
        }

        if let (ParameterValue::Number(default), Some(min)) = (&target.default, target.min) {
            if *default < min {
                diagnostics.push(Diagnostic::new(
                    error_codes::VALIDATION_INVALID_VALUE,
                    Severity::Error,
                    format!(
                        "preset target {} default {} is below minimum {}",
                        target.name, default, min
                    ),
                ));
            }
        }
        if let (ParameterValue::Number(default), Some(max)) = (&target.default, target.max) {
            if *default > max {
                diagnostics.push(Diagnostic::new(
                    error_codes::VALIDATION_INVALID_VALUE,
                    Severity::Error,
                    format!(
                        "preset target {} default {} is above maximum {}",
                        target.name, default, max
                    ),
                ));
            }
        }

        validate_preset_parameter_destination(patch, registry, target, diagnostics);
    }

    for target in &patch.preset_surface.assets {
        validate_preset_target_name(&target.name, &mut target_names, diagnostics);
        validate_preset_asset_destination(patch, registry, target, diagnostics);
    }
}

fn validate_preset_target_name(
    name: &str,
    target_names: &mut BTreeSet<String>,
    diagnostics: &mut PatchValidationError,
) {
    if name.trim().is_empty() {
        diagnostics.push(Diagnostic::new(
            error_codes::VALIDATION_MISSING_FIELD,
            Severity::Error,
            "preset target name is required",
        ));
    } else if !target_names.insert(name.to_string()) {
        diagnostics.push(Diagnostic::new(
            error_codes::VALIDATION_INVALID_VALUE,
            Severity::Error,
            format!("duplicate preset target {name}"),
        ));
    }
}

fn validate_preset_parameter_destination(
    patch: &PatchDocument,
    registry: &BuiltInModuleRegistry,
    target: &PresetParameterTargetDeclaration,
    diagnostics: &mut PatchValidationError,
) {
    let Some(module) = patch
        .modules
        .iter()
        .find(|module| module.id == target.maps_to.module_id)
    else {
        diagnostics.push(Diagnostic::new(
            error_codes::VALIDATION_INVALID_VALUE,
            Severity::Error,
            format!(
                "preset target {} maps to unresolved destination {}.{}",
                target.name, target.maps_to.module_id, target.maps_to.port_name
            ),
        ));
        return;
    };

    if let Some(definition) = registry.get(&module.module_type) {
        if !definition
            .parameters()
            .iter()
            .any(|parameter| parameter.name() == target.maps_to.port_name)
        {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Error,
                format!(
                    "preset target {} maps to unresolved destination {}.{}",
                    target.name, target.maps_to.module_id, target.maps_to.port_name
                ),
            ));
        }
        return;
    }

    let Some(definition) = patch
        .module_definitions
        .iter()
        .find(|definition| definition.module_type == module.module_type)
    else {
        return;
    };

    if !definition
        .parameters
        .iter()
        .any(|parameter| parameter.name == target.maps_to.port_name)
    {
        diagnostics.push(Diagnostic::new(
            error_codes::VALIDATION_INVALID_VALUE,
            Severity::Error,
            format!(
                "preset target {} maps to unresolved destination {}.{}",
                target.name, target.maps_to.module_id, target.maps_to.port_name
            ),
        ));
    }
}

fn validate_preset_asset_destination(
    patch: &PatchDocument,
    registry: &BuiltInModuleRegistry,
    target: &PresetAssetTargetDeclaration,
    diagnostics: &mut PatchValidationError,
) {
    let Some(default_asset) = patch.assets.iter().find(|asset| asset.id == target.default) else {
        diagnostics.push(Diagnostic::new(
            error_codes::VALIDATION_INVALID_VALUE,
            Severity::Error,
            format!(
                "preset target {} references missing default asset {}",
                target.name, target.default
            ),
        ));
        return;
    };

    if default_asset.kind != target.kind {
        diagnostics.push(Diagnostic::new(
            error_codes::VALIDATION_TYPE_MISMATCH,
            Severity::Error,
            format!(
                "preset target {} default asset {} has kind {:?}; expected {:?}",
                target.name, target.default, default_asset.kind, target.kind
            ),
        ));
    }

    let Some(module) = patch
        .modules
        .iter()
        .find(|module| module.id == target.maps_to.module_id)
    else {
        diagnostics.push(Diagnostic::new(
            error_codes::VALIDATION_INVALID_VALUE,
            Severity::Error,
            format!(
                "preset target {} maps to unresolved destination {}.{}",
                target.name, target.maps_to.module_id, target.maps_to.port_name
            ),
        ));
        return;
    };

    let destination_is_declared = registry
        .get(&module.module_type)
        .map(|definition| {
            definition
                .parameters()
                .iter()
                .any(|parameter| parameter.name() == target.maps_to.port_name)
        })
        .unwrap_or(false);

    if !destination_is_declared {
        diagnostics.push(Diagnostic::new(
            error_codes::VALIDATION_INVALID_VALUE,
            Severity::Error,
            format!(
                "preset target {} maps to unresolved destination {}.{}",
                target.name, target.maps_to.module_id, target.maps_to.port_name
            ),
        ));
    }
}

fn preset_value_matches_type(value: &ParameterValue, expected: PresetTargetType) -> bool {
    match (value, expected) {
        (ParameterValue::Boolean(_), PresetTargetType::Boolean) => true,
        (ParameterValue::Number(value), PresetTargetType::Integer) => value.fract() == 0.0,
        (ParameterValue::Number(_), PresetTargetType::Number) => true,
        (ParameterValue::Text(_), PresetTargetType::Text) => true,
        _ => false,
    }
}

fn preset_target_type_name(value_type: PresetTargetType) -> &'static str {
    match value_type {
        PresetTargetType::Boolean => "boolean",
        PresetTargetType::Integer => "integer",
        PresetTargetType::Number => "number",
        PresetTargetType::Text => "string",
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

impl fmt::Display for PresetLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFormat { path } => {
                write!(formatter, "unsupported preset format: {}", path.display())
            }
            Self::ReadFailed { path, message } => {
                write!(
                    formatter,
                    "failed to read preset {}: {message}",
                    path.display()
                )
            }
            Self::ParseFailed { path, message } => {
                if let Some(path) = path {
                    write!(
                        formatter,
                        "failed to parse preset {}: {message}",
                        path.display()
                    )
                } else {
                    write!(formatter, "failed to parse preset: {message}")
                }
            }
        }
    }
}

impl std::error::Error for PresetLoadError {}

impl std::error::Error for PatchLoadError {}

impl PatchLoadError {
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            Self::UnsupportedFormat { path } => Diagnostic::new(
                error_codes::LOADING,
                Severity::Error,
                format!("unsupported patch format: {}", path.display()),
            ),
            Self::ReadFailed { path, message } => Diagnostic::new(
                error_codes::LOADING,
                Severity::Error,
                format!("failed to read patch {}: {message}", path.display()),
            ),
            Self::ParseFailed { path, message } => {
                let mut d = Diagnostic::new(
                    error_codes::LOADING,
                    Severity::Error,
                    format!("failed to parse patch: {message}"),
                );
                if let Some(path) = path {
                    d = d.with_source_location(diagnostics::SourceLocation::new(
                        Some(path.to_string_lossy().to_string()),
                        None,
                        None,
                    ));
                }
                d
            }
        }
    }
}

impl PatchValidationError {
    pub fn to_diagnostics(&self) -> Diagnostics {
        self.diagnostics.clone()
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        self.diagnostics.all()
    }
}

impl fmt::Display for PortReference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}", self.module_id, self.port_name)
    }
}

impl fmt::Display for PatchValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "patch validation failed")?;

        for diagnostic in self.diagnostics.all() {
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

pub(super) fn validate_port_reference(
    label: &str,
    reference: &PortReference,
    diagnostics: &mut PatchValidationError,
) {
    if reference.module_id.trim().is_empty() || reference.port_name.trim().is_empty() {
        diagnostics.push(Diagnostic::new(
            error_codes::VALIDATION_INVALID_VALUE,
            Severity::Error,
            format!("{label} must use a non-empty module_id.port_name reference"),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::{
        EVENT_FILTER_NOTE_PARAMETER, EVENT_FILTER_NOTE_SELECTOR, EVENT_FILTER_SELECTOR_PARAMETER,
    };

    fn schema_error(yaml: &str, case: &str) -> PatchValidationError {
        let patch =
            load_patch_str(yaml).unwrap_or_else(|e| panic!("{case}: patch should parse: {e}"));
        validate_patch_schema(&patch)
            .expect_err(&format!("{case}: patch should fail schema validation"))
    }

    fn assert_has_code(error: &PatchValidationError, code: &str, case: &str) {
        assert!(
            error.diagnostics().iter().any(|d| d.error_code() == code),
            "{case}: expected diagnostic {code}, got {:?}",
            error
                .diagnostics()
                .iter()
                .map(|d| (d.error_code(), d.message()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn validate_patch_schema_reports_structural_rule_violations() {
        use error_codes::{VALIDATION_INVALID_VALUE, VALIDATION_MISSING_FIELD};

        let cases: &[(&str, &str, &str)] = &[
            (
                "empty metadata.name",
                r#"
metadata:
  name: ""
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
modules:
  - id: osc
    type: oscillator
"#,
                VALIDATION_MISSING_FIELD,
            ),
            (
                "zero sample rate",
                r#"
metadata:
  name: Zero Rate
render:
  sample_rate_hz: 0
  block_size_frames: 64
  duration_frames: 64
modules:
  - id: osc
    type: oscillator
"#,
                VALIDATION_INVALID_VALUE,
            ),
            (
                "zero block size",
                r#"
metadata:
  name: Zero Block
render:
  sample_rate_hz: 48000
  block_size_frames: 0
  duration_frames: 64
modules:
  - id: osc
    type: oscillator
"#,
                VALIDATION_INVALID_VALUE,
            ),
            (
                "no modules declared",
                r#"
metadata:
  name: Empty
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
modules: []
"#,
                VALIDATION_MISSING_FIELD,
            ),
            (
                "duplicate module id",
                r#"
metadata:
  name: Dupes
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
modules:
  - id: osc
    type: oscillator
  - id: osc
    type: oscillator
"#,
                VALIDATION_INVALID_VALUE,
            ),
            (
                "empty module type",
                r#"
metadata:
  name: No Type
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
modules:
  - id: osc
    type: ""
"#,
                VALIDATION_MISSING_FIELD,
            ),
        ];

        for (case, yaml, expected_code) in cases {
            let error = schema_error(yaml, case);
            assert_has_code(&error, expected_code, case);
        }
    }

    #[test]
    fn patch_load_error_formats_and_maps_to_loading_diagnostic() {
        let variants = [
            PatchLoadError::UnsupportedFormat {
                path: PathBuf::from("song.txt"),
            },
            PatchLoadError::ReadFailed {
                path: PathBuf::from("song.yaml"),
                message: "denied".to_string(),
            },
            PatchLoadError::ParseFailed {
                path: Some(PathBuf::from("song.yaml")),
                message: "bad yaml".to_string(),
            },
            PatchLoadError::ParseFailed {
                path: None,
                message: "bad yaml".to_string(),
            },
        ];

        for variant in &variants {
            assert!(
                !variant.to_string().is_empty(),
                "every load error should render a message"
            );
            assert_eq!(
                variant.to_diagnostic().error_code(),
                error_codes::LOADING,
                "load errors map to the loading diagnostic code"
            );
        }

        assert!(
            PatchLoadError::UnsupportedFormat {
                path: PathBuf::from("song.txt"),
            }
            .to_string()
            .contains("unsupported patch format"),
            "unsupported format message should name the problem"
        );
    }

    #[test]
    fn preset_load_error_renders_each_variant() {
        let variants = [
            PresetLoadError::UnsupportedFormat {
                path: PathBuf::from("preset.txt"),
            },
            PresetLoadError::ReadFailed {
                path: PathBuf::from("preset.yaml"),
                message: "denied".to_string(),
            },
            PresetLoadError::ParseFailed {
                path: Some(PathBuf::from("preset.yaml")),
                message: "bad".to_string(),
            },
            PresetLoadError::ParseFailed {
                path: None,
                message: "bad".to_string(),
            },
        ];

        for variant in &variants {
            assert!(
                variant.to_string().contains("preset"),
                "preset load errors should mention the preset"
            );
        }
    }

    #[test]
    fn port_reference_deserialize_rejects_malformed_references() {
        let malformed = ["nodot", "mod.", ".port", "a.b.c"];
        for reference in malformed {
            let yaml = format!("\"{reference}\"");
            let parsed: Result<PortReference, _> = serde_yaml::from_str(&yaml);
            assert!(
                parsed.is_err(),
                "malformed reference {reference} should be rejected"
            );
        }

        let parsed: PortReference =
            serde_yaml::from_str("\"osc.out\"").expect("valid module_id.port_name should parse");
        assert_eq!(
            parsed.to_string(),
            "osc.out",
            "round-trips to its text form"
        );
    }

    #[test]
    fn event_filter_yaml_preserves_readable_note_selector_configuration() {
        let patch = load_patch_str(
            r#"
metadata:
  name: Event Filter
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
modules:
  - id: kick_filter
    type: event_filter
    parameters:
      selector: note
      note: 36
"#,
        )
        .expect("event_filter patch should parse");

        validate_patch_schema(&patch).expect("event_filter selector should validate");

        assert_eq!(patch.modules[0].module_type, module_types::EVENT_FILTER);
        assert_eq!(
            patch.modules[0]
                .parameters
                .get(EVENT_FILTER_SELECTOR_PARAMETER),
            Some(&ParameterValue::Text(
                EVENT_FILTER_NOTE_SELECTOR.to_string()
            ))
        );
        assert_eq!(
            patch.modules[0].parameters.get(EVENT_FILTER_NOTE_PARAMETER),
            Some(&ParameterValue::Number(36.0))
        );
    }

    #[test]
    fn event_filter_yaml_rejects_hidden_signal_chain_fields() {
        let patch = load_patch_str(
            r#"
metadata:
  name: Hidden Chain
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
modules:
  - id: bad_filter
    type: event_filter
    parameters:
      selector: note
      note: 36
    modules: []
"#,
        )
        .expect("patch should parse");

        let error = validate_patch_schema(&patch).expect_err("hidden chain should fail");

        assert!(
            error
                .to_string()
                .contains("hidden signal-chain field modules")
        );
        assert!(error.to_string().contains("external patch modules"));
    }

    #[test]
    fn event_filter_yaml_rejects_sequencing_fields() {
        let patch = load_patch_str(
            r#"
metadata:
  name: Hidden Sequencer
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
modules:
  - id: bad_filter
    type: event_filter
    parameters:
      selector: note
      note: 36
    pattern: x---
"#,
        )
        .expect("patch should parse");

        let error = validate_patch_schema(&patch).expect_err("hidden sequencing should fail");

        assert!(error.to_string().contains("sequencing field pattern"));
        assert!(error.to_string().contains("explicit external modules"));
    }

    #[test]
    fn script_module_rejects_audio_rate_output_ports_before_graph_preparation() {
        let patch = load_patch_str(
            r#"
metadata:
  name: Script Audio Output
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
modules:
  - id: mapper
    type: script
    outputs:
      - name: audio
        signal_type: audio
"#,
        )
        .expect("patch should parse");

        let diagnostics = validate_patch_schema(&patch)
            .expect_err("audio-rate script output should fail")
            .to_diagnostics();

        assert!(diagnostics.all().iter().any(|diagnostic| {
            diagnostic.error_code() == error_codes::SCRIPT_UNSUPPORTED_PORT
                && diagnostic.module_id() == Some("mapper")
                && diagnostic.port_name() == Some("audio")
        }));
    }

    #[test]
    fn script_module_rejects_audio_rate_input_ports_before_graph_preparation() {
        let patch = load_patch_str(
            r#"
metadata:
  name: Script Audio Input
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
modules:
  - id: mapper
    type: script
    parameters:
      language: rhai
      source: |
        fn process(ctx) {}
    inputs:
      - name: audio
        signal_type: audio
"#,
        )
        .expect("patch should parse");

        let diagnostics = validate_patch_schema(&patch)
            .expect_err("audio-rate script input should fail")
            .to_diagnostics();

        assert!(diagnostics.all().iter().any(|diagnostic| {
            diagnostic.error_code() == error_codes::SCRIPT_UNSUPPORTED_PORT
                && diagnostic.module_id() == Some("mapper")
                && diagnostic.port_name() == Some("audio")
        }));
    }

    #[test]
    fn script_module_rejects_filesystem_network_blocking_random_and_allocation_tokens() {
        let patch = load_patch_str(
            r#"
metadata:
  name: Script Unsupported API
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
modules:
  - id: mapper
    type: script
    parameters:
      language: rhai
      source: |
        fn process(ctx) {
          let data = std::fs::read_file("secret");
          network::send(data);
          thread::sleep(1);
          random();
          alloc(128);
        }
    inputs:
      - name: notes
        signal_type: event
    outputs:
      - name: velocity
        signal_type: control
"#,
        )
        .expect("patch should parse");

        let diagnostics = validate_patch_schema(&patch)
            .expect_err("unsupported script APIs should fail")
            .to_diagnostics();

        for token in ["std::fs", "network", "thread::", "random", "alloc"] {
            assert!(diagnostics.all().iter().any(|diagnostic| {
                diagnostic.error_code() == error_codes::SCRIPT_UNSUPPORTED_API
                    && diagnostic.module_id() == Some("mapper")
                    && diagnostic.actual() == Some(token)
            }));
        }
    }

    #[test]
    fn script_module_accepts_event_and_control_ports_with_deterministic_source() {
        let patch = load_patch_str(
            r#"
metadata:
  name: Script Control Mapper
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
modules:
  - id: mapper
    type: script
    parameters:
      language: rhai
      source: |
        fn process(ctx) {}
    inputs:
      - name: notes
        signal_type: event
    outputs:
      - name: velocity
        signal_type: control
      - name: routed_notes
        signal_type: event
"#,
        )
        .expect("patch should parse");

        validate_patch_schema(&patch).expect("event/control script should validate");
    }

    #[test]
    fn script_module_accepts_rhai_language_and_inline_source_parameters() {
        let patch = load_patch_str(
            r#"
metadata:
  name: Parameter Script
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
modules:
  - id: mapper
    type: script
    parameters:
      language: rhai
      source: |
        fn process(ctx) {}
    inputs:
      - name: notes
        signal_type: event
    outputs:
      - name: accent
        signal_type: control
"#,
        )
        .expect("patch should parse");

        validate_patch_schema(&patch).expect("rhai parameter script should validate");

        assert_eq!(
            patch.modules[0].parameters.get(SCRIPT_LANGUAGE_PARAMETER),
            Some(&ParameterValue::Text(SCRIPT_LANGUAGE_RHAI.to_string()))
        );
        assert!(matches!(
            patch.modules[0].parameters.get(SCRIPT_SOURCE_PARAMETER),
            Some(ParameterValue::Text(source)) if source.contains("fn process(ctx)")
        ));
    }

    #[test]
    fn script_module_rejects_missing_source_for_rhai_language() {
        let patch = load_patch_str(
            r#"
metadata:
  name: Missing Source
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
modules:
  - id: mapper
    type: script
    parameters:
      language: rhai
"#,
        )
        .expect("patch should parse");

        let diagnostics = validate_patch_schema(&patch)
            .expect_err("missing source should fail")
            .to_diagnostics();

        assert!(diagnostics.all().iter().any(|diagnostic| {
            diagnostic.error_code() == error_codes::VALIDATION_MISSING_FIELD
                && diagnostic.module_id() == Some("mapper")
        }));
    }

    #[test]
    fn script_module_rejects_unsupported_language() {
        let patch = load_patch_str(
            r#"
metadata:
  name: Unsupported Language
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
modules:
  - id: mapper
    type: script
    parameters:
      language: lua
      source: |
        function process(ctx) end
"#,
        )
        .expect("patch should parse");

        let diagnostics = validate_patch_schema(&patch)
            .expect_err("unsupported language should fail")
            .to_diagnostics();

        assert!(diagnostics.all().iter().any(|diagnostic| {
            diagnostic.error_code() == error_codes::SCRIPT_VALIDATION
                && diagnostic.expected() == Some(SCRIPT_LANGUAGE_RHAI)
                && diagnostic.actual() == Some("lua")
        }));
    }

    #[test]
    fn script_module_rejects_malformed_rhai_source() {
        let patch = load_patch_str(
            r#"
metadata:
  name: Malformed Rhai
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
modules:
  - id: mapper
    type: script
    parameters:
      language: rhai
      source: |
        fn process(ctx) {
"#,
        )
        .expect("patch should parse");

        let diagnostics = validate_patch_schema(&patch)
            .expect_err("malformed Rhai should fail")
            .to_diagnostics();

        assert!(diagnostics.all().iter().any(|diagnostic| {
            diagnostic.error_code() == error_codes::SCRIPT_PARSE
                && diagnostic.module_id() == Some("mapper")
        }));
    }

    #[test]
    fn script_module_rejects_missing_process_entry_point() {
        let patch = load_patch_str(
            r#"
metadata:
  name: Missing Process
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
modules:
  - id: mapper
    type: script
    parameters:
      language: rhai
      source: |
        fn route(ctx) {}
"#,
        )
        .expect("patch should parse");

        let diagnostics = validate_patch_schema(&patch)
            .expect_err("missing process should fail")
            .to_diagnostics();

        assert!(diagnostics.all().iter().any(|diagnostic| {
            diagnostic.error_code() == error_codes::SCRIPT_VALIDATION
                && diagnostic.expected() == Some("fn process(ctx)")
        }));
    }
}
