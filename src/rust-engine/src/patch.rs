use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::builtins::{
    BuiltInModuleDefinition, BuiltInModuleRegistry, ParameterMetadata, ParameterValueType,
    module_types,
};
use crate::diagnostics::{self, Diagnostic, Diagnostics, Severity, error_codes};

#[path = "patch_composite.rs"]
mod patch_composite;

pub use patch_composite::{
    CompositeBindingDeclaration, CompositeInputDeclaration, CompositeOutputDeclaration,
    ModuleDefinitionDeclaration,
};

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
        ParameterValueType::Boolean => default.parse::<bool>().ok().map(ParameterValue::Boolean),
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

    patch_composite::validate_module_definitions(patch, &mut result);

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

        validate_declared_parameters_for_module(
            "module parameter",
            &module.id,
            &module.module_type,
            &module.parameters,
            &registry,
            &mut result,
        );

        patch_composite::validate_composite_instance_bindings(module, patch, &mut result);
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
        (ParameterValue::Boolean(_), ParameterValueType::Boolean) => true,
        (ParameterValue::Number(value), ParameterValueType::Integer) => value.fract() == 0.0,
        (ParameterValue::Number(_), ParameterValueType::Number) => true,
        (ParameterValue::Text(_), ParameterValueType::Text) => true,
        _ => false,
    }
}

fn parameter_type_name(value_type: ParameterValueType) -> &'static str {
    match value_type {
        ParameterValueType::Boolean => "boolean",
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
    pub fn to_diagnostics(&self) -> diagnostics::Diagnostics {
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
}
