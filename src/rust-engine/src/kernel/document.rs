//! YAML front end for the unified graph kernel.
//!
//! Root patches and inline defined modules pass through the same graph-declaration
//! conversion into [`GraphDefinition`].

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use serde::Deserialize;
use serde_yaml::{Mapping, Value};

use crate::diagnostics::{Diagnostic, Diagnostics, Severity, error_codes};
use crate::graph::{PortDirection, SignalType};
use crate::patch::{InstrumentIdentity, ParameterValue, PresetDocument};

use super::{
    ChannelCount, Connection, ControlDefault, DefinitionImplementation, DefinitionRegistry,
    GraphDefinition, Multiplicity, Node, NodeId, Port, PortRef, ResourceKind, ResourceOrigin,
    ResourceRef, StaticArg, StaticParam, StaticType, StaticValue,
};

const YAML_EXTENSION: &str = "yaml";
const YML_EXTENSION: &str = "yml";
const ROOT_DEFINITION_NAME: &str = "root";
const FIELD_RENDER: &str = "render";
const FIELD_VOICE_ALLOCATION: &str = "voice_allocation";
const FIELD_PARAMETERS: &str = "parameters";
const FIELD_ASSET_BINDINGS: &str = "asset_bindings";
const FIELD_MODULE_DEFINITIONS: &str = "module_definitions";
const FIELD_MODULES: &str = "modules";
const FIELD_TYPE: &str = "type";
const FIELD_ID: &str = "id";
const LEGACY_BINDING_PREFIX: &str = "${";

/// Optional descriptive data carried beside the root graph definition.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KernelPatchMetadata {
    name: Option<String>,
    version: Option<String>,
    author: Option<String>,
}

impl KernelPatchMetadata {
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    pub fn author(&self) -> Option<&str> {
        self.author.as_deref()
    }
}

/// Parsed kernel document ready for definition resolution and flattening.
#[derive(Clone, Debug)]
pub struct KernelPatch {
    metadata: KernelPatchMetadata,
    instrument: Option<InstrumentIdentity>,
    preset_surface: KernelPresetSurface,
    root: GraphDefinition,
    registry: DefinitionRegistry,
    local_definition_names: Vec<String>,
}

impl KernelPatch {
    pub fn instrument(&self) -> Option<&InstrumentIdentity> {
        self.instrument.as_ref()
    }

    pub fn preset_surface(&self) -> &KernelPresetSurface {
        &self.preset_surface
    }

    pub fn metadata(&self) -> &KernelPatchMetadata {
        &self.metadata
    }

    pub fn root(&self) -> &GraphDefinition {
        &self.root
    }

    pub fn registry(&self) -> &DefinitionRegistry {
        &self.registry
    }

    pub(crate) fn local_definition_names(&self) -> &[String] {
        &self.local_definition_names
    }

    /// Apply a compatible preset to root declarations before graph flattening.
    pub fn apply_preset(&self, preset: &PresetDocument) -> Result<Self, Diagnostics> {
        let mut diagnostics = Diagnostics::new();
        match &self.instrument {
            None => diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_MISSING_FIELD,
                Severity::Error,
                "patch does not declare instrument preset identity",
            )),
            Some(identity) => {
                if identity.id != preset.instrument.id {
                    diagnostics.push(
                        Diagnostic::new(
                            error_codes::VALIDATION_INVALID_VALUE,
                            Severity::Error,
                            format!(
                                "preset instrument {} does not match patch instrument {}",
                                preset.instrument.id, identity.id
                            ),
                        )
                        .with_expected(&identity.id)
                        .with_actual(&preset.instrument.id),
                    );
                }
                if identity.preset_schema_version != preset.instrument.preset_schema_version {
                    diagnostics.push(
                        Diagnostic::new(
                            error_codes::VALIDATION_INVALID_VALUE,
                            Severity::Error,
                            format!("preset schema version {} does not match patch instrument schema version {}", preset.instrument.preset_schema_version, identity.preset_schema_version),
                        )
                        .with_expected(identity.preset_schema_version.to_string())
                        .with_actual(preset.instrument.preset_schema_version.to_string()),
                    );
                }
            }
        }
        for name in preset.values.keys() {
            if !self
                .preset_surface
                .parameters
                .iter()
                .any(|alias| alias.name == *name)
            {
                diagnostics.push(Diagnostic::new(
                    error_codes::VALIDATION_INVALID_VALUE,
                    Severity::Error,
                    format!("unknown preset target {name}"),
                ));
            }
        }
        for name in preset.assets.keys() {
            if !self
                .preset_surface
                .assets
                .iter()
                .any(|alias| alias.name == *name)
            {
                diagnostics.push(Diagnostic::new(
                    error_codes::VALIDATION_INVALID_VALUE,
                    Severity::Error,
                    format!("unknown preset target {name}"),
                ));
            }
        }
        for alias in &self.preset_surface.parameters {
            if let Some(value) = preset.values.get(&alias.name) {
                match value {
                    ParameterValue::Number(value)
                        if value.is_finite()
                            && alias.default.min().is_none_or(|min| *value >= min)
                            && alias.default.max().is_none_or(|max| *value <= max) => {}
                    _ => diagnostics.push(Diagnostic::new(
                        error_codes::VALIDATION_INVALID_VALUE,
                        Severity::Error,
                        format!(
                            "preset target {} has incompatible type or range",
                            alias.name
                        ),
                    )),
                }
            }
        }
        for field in preset.extra_fields.keys() {
            if [
                "modules",
                "connections",
                "ports",
                "static_params",
                "module_definitions",
                "preset_surface",
                "render",
                "voice_allocation",
            ]
            .contains(&field.as_str())
            {
                diagnostics.push(Diagnostic::new(
                    error_codes::VALIDATION_INVALID_VALUE,
                    Severity::Error,
                    format!("preset cannot declare structural field {field}"),
                ));
            }
        }
        if diagnostics.has_errors() {
            return Err(diagnostics);
        }

        let mut applied = self.clone();
        for alias in &self.preset_surface.parameters {
            if let Some(ParameterValue::Number(value)) = preset.values.get(&alias.name) {
                let port = applied
                    .root
                    .ports
                    .iter_mut()
                    .find(|port| port.name == alias.port_name)
                    .expect("alias destination was validated at load time");
                port.control_default
                    .as_mut()
                    .expect("control alias has a default")
                    .default = *value;
            }
        }
        for alias in &self.preset_surface.assets {
            if let Some(path) = preset.assets.get(&alias.name) {
                let param = applied
                    .root
                    .static_params
                    .iter_mut()
                    .find(|param| param.name == alias.static_param_name)
                    .expect("alias destination was validated at load time");
                param.default = Some(StaticValue::Resource(ResourceRef::new(
                    alias.kind,
                    path,
                    ResourceOrigin::Document,
                )));
            }
        }
        Ok(applied)
    }
}

/// Preset names mapped to typed root declarations.
#[derive(Clone, Debug, Default)]
pub struct KernelPresetSurface {
    parameters: Vec<KernelPresetValueAlias>,
    assets: Vec<KernelPresetAssetAlias>,
}

impl KernelPresetSurface {
    pub fn parameters(&self) -> &[KernelPresetValueAlias] {
        &self.parameters
    }
    pub fn assets(&self) -> &[KernelPresetAssetAlias] {
        &self.assets
    }
}

#[derive(Clone, Debug)]
pub struct KernelPresetValueAlias {
    name: String,
    port_name: String,
    default: ControlDefault,
}

impl KernelPresetValueAlias {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn port_name(&self) -> &str {
        &self.port_name
    }
    pub fn control_default(&self) -> &ControlDefault {
        &self.default
    }
}

#[derive(Clone, Debug)]
pub struct KernelPresetAssetAlias {
    name: String,
    static_param_name: String,
    kind: ResourceKind,
    default: Option<ResourceRef>,
}

impl KernelPresetAssetAlias {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn static_param_name(&self) -> &str {
        &self.static_param_name
    }
    pub fn kind(&self) -> ResourceKind {
        self.kind
    }
    pub fn default(&self) -> Option<&ResourceRef> {
        self.default.as_ref()
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct MetadataDocument {
    name: Option<String>,
    version: Option<String>,
    author: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PatchDocument {
    #[serde(default)]
    metadata: MetadataDocument,
    instrument: Option<InstrumentIdentity>,
    #[serde(default)]
    preset_surface: PresetSurfaceDocument,
    #[serde(default)]
    static_params: Vec<StaticParamDocument>,
    #[serde(default)]
    ports: Vec<PortDocument>,
    #[serde(default)]
    module_definitions: Vec<DefinedModuleDocument>,
    #[serde(default)]
    modules: Vec<NodeDocument>,
    #[serde(default)]
    connections: Vec<ConnectionDocument>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct PresetSurfaceDocument {
    #[serde(default)]
    parameters: Vec<PresetAliasDocument>,
    #[serde(default)]
    assets: Vec<PresetAliasDocument>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PresetAliasDocument {
    name: String,
    maps_to: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DefinedModuleDocument {
    #[serde(rename = "type")]
    definition_type: String,
    implementation: Option<String>,
    #[serde(default)]
    static_params: Vec<StaticParamDocument>,
    #[serde(default)]
    ports: Vec<PortDocument>,
    #[serde(default)]
    modules: Vec<NodeDocument>,
    #[serde(default)]
    connections: Vec<ConnectionDocument>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StaticParamDocument {
    name: String,
    #[serde(rename = "type")]
    static_type: StaticTypeDocument,
    resource_kind: Option<ResourceKindDocument>,
    default: Option<Value>,
    #[serde(default)]
    allowed_values: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StaticTypeDocument {
    Int,
    Enum,
    String,
    Resource,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResourceKindDocument {
    Sample,
    ImpulseResponse,
}

impl ResourceKindDocument {
    fn into_kernel(self) -> ResourceKind {
        match self {
            Self::Sample => ResourceKind::Sample,
            Self::ImpulseResponse => ResourceKind::ImpulseResponse,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResourceRefDocument {
    kind: ResourceKindDocument,
    path: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PortDocument {
    name: String,
    direction: DirectionDocument,
    signal: SignalDocument,
    channels: ChannelDocument,
    #[serde(default)]
    multiplicity: MultiplicityDocument,
    default: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
    unit: Option<String>,
    #[serde(default)]
    maps_to: ReferenceList,
    #[serde(default)]
    maps_from: ReferenceList,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DirectionDocument {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SignalDocument {
    Audio,
    Control,
    Event,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MultiplicityDocument {
    #[default]
    SingleSource,
    Summing,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum ChannelDocument {
    Literal(u32),
    Param(String),
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(untagged)]
enum ReferenceList {
    One(String),
    Many(Vec<String>),
    #[default]
    Missing,
}

impl ReferenceList {
    fn iter(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        match self {
            Self::One(reference) => Box::new(std::iter::once(reference.as_str())),
            Self::Many(references) => Box::new(references.iter().map(String::as_str)),
            Self::Missing => Box::new(std::iter::empty()),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeDocument {
    id: String,
    #[serde(rename = "type")]
    definition_type: String,
    #[serde(default, rename = "static")]
    static_args: BTreeMap<String, Value>,
    #[serde(default)]
    defaults: BTreeMap<String, f64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectionDocument {
    from: String,
    to: String,
}

/// Load a kernel patch from a `.yaml` or `.yml` file.
pub fn load_kernel_patch_file(path: impl AsRef<Path>) -> Result<KernelPatch, Diagnostics> {
    let path = path.as_ref();
    let extension = path.extension().and_then(|value| value.to_str());
    if !matches!(extension, Some(YAML_EXTENSION | YML_EXTENSION)) {
        return Err(Diagnostic::new(
            error_codes::KERNEL_DOCUMENT_UNSUPPORTED_FORMAT,
            Severity::Error,
            format!("unsupported kernel patch format: {}", path.display()),
        )
        .with_expected(format!(".{YAML_EXTENSION} or .{YML_EXTENSION}"))
        .into());
    }

    let yaml = fs::read_to_string(path).map_err(|error| {
        Diagnostics::from(Diagnostic::new(
            error_codes::KERNEL_DOCUMENT_READ_FAILED,
            Severity::Error,
            format!("failed to read kernel patch {}: {error}", path.display()),
        ))
    })?;
    load_kernel_patch_str(&yaml)
}

/// Parse YAML directly into the kernel graph model.
pub fn load_kernel_patch_str(yaml: &str) -> Result<KernelPatch, Diagnostics> {
    load_kernel_document_str(yaml, None, ResourceOrigin::Document, true)
}

pub(crate) fn load_kernel_definition_str(
    yaml: &str,
    definition_name: &str,
    origin: ResourceOrigin,
) -> Result<KernelPatch, Diagnostics> {
    load_kernel_document_str(yaml, Some(definition_name), origin, false)
}

fn load_kernel_document_str(
    yaml: &str,
    definition_name: Option<&str>,
    origin: ResourceOrigin,
    require_output: bool,
) -> Result<KernelPatch, Diagnostics> {
    let value: Value = serde_yaml::from_str(yaml).map_err(parse_diagnostic)?;
    reject_legacy_document_shape(&value)?;
    validate_kernel_schema(&value)?;
    let document: PatchDocument = serde_yaml::from_value(value).map_err(parse_diagnostic)?;

    let metadata = KernelPatchMetadata {
        name: document.metadata.name,
        version: document.metadata.version,
        author: document.metadata.author,
    };
    let root_name = definition_name
        .or(metadata.name.as_deref())
        .unwrap_or(ROOT_DEFINITION_NAME);

    let mut registry = super::builtins::builtin_registry();
    for defined_module in &document.module_definitions {
        registry = registry.with_definition(convert_declaration(
            &defined_module.definition_type,
            defined_module.implementation.as_deref(),
            &defined_module.static_params,
            &defined_module.ports,
            &origin,
        )?);
    }
    for defined_module in &document.module_definitions {
        let definition = convert_graph(
            &defined_module.definition_type,
            defined_module.implementation.as_deref(),
            &defined_module.static_params,
            &defined_module.ports,
            &defined_module.modules,
            &defined_module.connections,
            &registry,
            &origin,
        )?;
        registry = registry.with_definition(definition);
    }

    let root = convert_graph(
        root_name,
        None,
        &document.static_params,
        &document.ports,
        &document.modules,
        &document.connections,
        &registry,
        &origin,
    )?;
    if require_output
        && !root
            .ports()
            .iter()
            .any(|port| port.direction() == PortDirection::Output)
    {
        return Err(Diagnostic::new(
            error_codes::KERNEL_DOCUMENT_NO_OUTPUT,
            Severity::Error,
            "kernel patch declares no root output port, so the instrument has no observable output",
        )
        .with_suggested_fix("declare at least one output in ports")
        .into());
    }

    let preset_surface = resolve_preset_surface(&document.preset_surface, &root)?;

    Ok(KernelPatch {
        metadata,
        instrument: document.instrument,
        preset_surface,
        root,
        registry,
        local_definition_names: document
            .module_definitions
            .iter()
            .map(|definition| definition.definition_type.clone())
            .collect(),
    })
}

/// Validate the parsed YAML against the repository schema before Serde or
/// graph construction. The schema is embedded from its checked-in YAML file
/// so installed binaries do not depend on a source-tree path at runtime.
fn validate_kernel_schema(value: &Value) -> Result<(), Diagnostics> {
    static VALIDATOR: OnceLock<Result<jsonschema::Validator, String>> = OnceLock::new();
    let validator = VALIDATOR.get_or_init(|| {
        let schema: serde_json::Value = serde_yaml::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../schema/patch.schema.yaml"
        )))
        .map_err(|error| format!("invalid kernel schema YAML: {error}"))?;
        jsonschema::validator_for(&schema)
            .map_err(|error| format!("invalid kernel JSON Schema: {error}"))
    });
    let validator = validator.as_ref().map_err(|message| {
        Diagnostic::new(
            error_codes::KERNEL_DOCUMENT_SCHEMA_FAILED,
            Severity::Error,
            message,
        )
    })?;
    let instance = serde_json::to_value(value).map_err(|error| {
        Diagnostic::new(
            error_codes::KERNEL_DOCUMENT_SCHEMA_FAILED,
            Severity::Error,
            format!("kernel patch YAML cannot be represented as JSON: {error}"),
        )
    })?;
    let mut diagnostics = Diagnostics::new();
    for error in validator.iter_errors(&instance) {
        diagnostics.push(Diagnostic::new(
            error_codes::KERNEL_DOCUMENT_SCHEMA_FAILED,
            Severity::Error,
            format!("kernel patch schema at {}: {error}", error.instance_path()),
        ));
    }
    if diagnostics.has_errors() {
        Err(diagnostics)
    } else {
        Ok(())
    }
}

fn resolve_preset_surface(
    document: &PresetSurfaceDocument,
    root: &GraphDefinition,
) -> Result<KernelPresetSurface, Diagnostics> {
    let mut surface = KernelPresetSurface::default();
    let mut names = BTreeSet::new();
    let mut diagnostics = Diagnostics::new();
    for alias in &document.parameters {
        if alias.name.trim().is_empty() {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_MISSING_FIELD,
                Severity::Error,
                "preset target name is required",
            ));
            continue;
        }
        if !names.insert(alias.name.as_str()) {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Error,
                format!("duplicate preset target {}", alias.name),
            ));
        }
        let port = root
            .ports()
            .iter()
            .find(|port| port.name() == alias.maps_to);
        match port {
            Some(port)
                if port.direction() == PortDirection::Input
                    && port.signal_type() == SignalType::Control
                    && port.control_default().is_some() =>
            {
                surface.parameters.push(KernelPresetValueAlias {
                    name: alias.name.clone(),
                    port_name: alias.maps_to.clone(),
                    default: port.control_default().expect("checked above").clone(),
                });
            }
            _ => diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Error,
                format!(
                    "preset target {} maps to unresolved control input {}",
                    alias.name, alias.maps_to
                ),
            )),
        }
    }
    for alias in &document.assets {
        if alias.name.trim().is_empty() {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_MISSING_FIELD,
                Severity::Error,
                "preset target name is required",
            ));
            continue;
        }
        if !names.insert(alias.name.as_str()) {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Error,
                format!("duplicate preset target {}", alias.name),
            ));
        }
        let param = root
            .static_params()
            .iter()
            .find(|param| param.name() == alias.maps_to);
        match param {
            Some(param) if matches!(param.static_type(), StaticType::Resource(_)) => {
                let StaticType::Resource(kind) = param.static_type() else {
                    unreachable!()
                };
                let Some(StaticValue::Resource(default)) = param.default() else {
                    diagnostics.push(Diagnostic::new(
                        error_codes::VALIDATION_MISSING_FIELD,
                        Severity::Error,
                        format!(
                            "preset target {} requires a declared resource default",
                            alias.name
                        ),
                    ));
                    continue;
                };
                surface.assets.push(KernelPresetAssetAlias {
                    name: alias.name.clone(),
                    static_param_name: alias.maps_to.clone(),
                    kind,
                    default: Some(default.clone()),
                });
            }
            _ => diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Error,
                format!(
                    "preset target {} maps to unresolved resource static parameter {}",
                    alias.name, alias.maps_to
                ),
            )),
        }
    }
    if diagnostics.has_errors() {
        Err(diagnostics)
    } else {
        Ok(surface)
    }
}

fn parse_diagnostic(error: serde_yaml::Error) -> Diagnostics {
    Diagnostic::new(
        error_codes::KERNEL_DOCUMENT_PARSE_FAILED,
        Severity::Error,
        format!("kernel patch YAML is invalid: {error}"),
    )
    .into()
}

fn reject_legacy_document_shape(value: &Value) -> Result<(), Diagnostics> {
    let Some(root) = value.as_mapping() else {
        return Ok(());
    };
    reject_field(
        root,
        FIELD_RENDER,
        error_codes::KERNEL_DOCUMENT_LEGACY_RENDER,
        "render settings belong to the host or render invocation",
        None,
    )?;
    reject_field(
        root,
        FIELD_VOICE_ALLOCATION,
        error_codes::KERNEL_DOCUMENT_LEGACY_VOICE_ALLOCATION,
        "voice allocation is expressed with a poly node",
        None,
    )?;
    reject_field(
        root,
        FIELD_ASSET_BINDINGS,
        error_codes::KERNEL_DOCUMENT_LEGACY_ASSET_BINDINGS,
        "declare assets as resource static parameters",
        None,
    )?;
    reject_modules(root.get(Value::String(FIELD_MODULES.into())))?;

    if let Some(definitions) = root
        .get(Value::String(FIELD_MODULE_DEFINITIONS.into()))
        .and_then(Value::as_sequence)
    {
        for definition in definitions {
            let Some(mapping) = definition.as_mapping() else {
                continue;
            };
            let definition_name = string_field(mapping, FIELD_TYPE);
            reject_field(
                mapping,
                FIELD_ASSET_BINDINGS,
                error_codes::KERNEL_DOCUMENT_LEGACY_ASSET_BINDINGS,
                "declare assets as resource static parameters",
                definition_name,
            )?;
            reject_field(
                mapping,
                FIELD_PARAMETERS,
                error_codes::KERNEL_DOCUMENT_LEGACY_PARAMETERS,
                "declare tunables as public control input ports",
                definition_name,
            )?;
            reject_modules(mapping.get(Value::String(FIELD_MODULES.into())))?;
        }
    }
    Ok(())
}

fn reject_modules(value: Option<&Value>) -> Result<(), Diagnostics> {
    let Some(modules) = value.and_then(Value::as_sequence) else {
        return Ok(());
    };
    for module in modules {
        let Some(mapping) = module.as_mapping() else {
            continue;
        };
        let module_id = string_field(mapping, FIELD_ID);
        reject_field(
            mapping,
            FIELD_PARAMETERS,
            error_codes::KERNEL_DOCUMENT_LEGACY_PARAMETERS,
            "use static for construction-time values or defaults for control input overrides",
            module_id,
        )?;
        if contains_legacy_binding(module) {
            let mut diagnostic = Diagnostic::new(
                error_codes::KERNEL_DOCUMENT_LEGACY_BINDING,
                Severity::Error,
                "legacy '${name}' binding syntax is not supported; use '$name' only for static parameter pass-through and maps_to for public control ports",
            )
            .with_suggested_fix("replace static ${name} with $name, or map a public control port with maps_to");
            if let Some(module_id) = module_id {
                diagnostic = diagnostic.with_module_id(module_id);
            }
            return Err(diagnostic.into());
        }
    }
    Ok(())
}

fn contains_legacy_binding(value: &Value) -> bool {
    match value {
        Value::String(value) => value.contains(LEGACY_BINDING_PREFIX),
        Value::Sequence(values) => values.iter().any(contains_legacy_binding),
        Value::Mapping(values) => values
            .iter()
            .any(|(key, value)| contains_legacy_binding(key) || contains_legacy_binding(value)),
        Value::Tagged(value) => contains_legacy_binding(&value.value),
        _ => false,
    }
}

fn reject_field(
    mapping: &Mapping,
    field: &str,
    code: &str,
    replacement: &str,
    context: Option<&str>,
) -> Result<(), Diagnostics> {
    if !mapping.contains_key(Value::String(field.into())) {
        return Ok(());
    }
    let mut diagnostic = Diagnostic::new(
        code,
        Severity::Error,
        format!("legacy field '{field}' is not supported in kernel documents; {replacement}"),
    )
    .with_suggested_fix(replacement);
    if let Some(context) = context {
        diagnostic = diagnostic.with_module_id(context);
    }
    Err(diagnostic.into())
}

fn string_field<'a>(mapping: &'a Mapping, field: &str) -> Option<&'a str> {
    mapping
        .get(Value::String(field.into()))
        .and_then(Value::as_str)
}

fn convert_graph(
    name: &str,
    implementation: Option<&str>,
    static_params: &[StaticParamDocument],
    ports: &[PortDocument],
    nodes: &[NodeDocument],
    connections: &[ConnectionDocument],
    registry: &DefinitionRegistry,
    origin: &ResourceOrigin,
) -> Result<GraphDefinition, Diagnostics> {
    let mut graph = convert_declaration(name, implementation, static_params, ports, origin)?;
    for node in nodes {
        graph = graph.with_node(convert_node(node, registry, origin)?);
    }
    for connection in connections {
        graph = graph.with_connection(Connection::new(
            parse_reference(&connection.from)?,
            parse_reference(&connection.to)?,
        ));
    }
    let mut diagnostics = Diagnostics::new();
    graph.validate_definition_structure(&mut diagnostics);
    if diagnostics.has_errors() {
        return Err(diagnostics);
    }
    Ok(graph)
}

fn convert_declaration(
    name: &str,
    implementation: Option<&str>,
    static_params: &[StaticParamDocument],
    ports: &[PortDocument],
    origin: &ResourceOrigin,
) -> Result<GraphDefinition, Diagnostics> {
    let implementation = match implementation {
        None => DefinitionImplementation::Graph,
        Some("script") => DefinitionImplementation::Script,
        Some(unsupported) => {
            return Err(Diagnostic::new(
                error_codes::KERNEL_DEFINITION_IMPLEMENTATION_UNSUPPORTED,
                Severity::Error,
                format!("definition '{name}' selects unsupported implementation '{unsupported}'"),
            )
            .with_module_id(name)
            .with_expected("script")
            .with_actual(unsupported)
            .into());
        }
    };
    let mut graph = GraphDefinition::new(name).with_implementation(implementation);
    for param in static_params {
        graph = graph.with_static_param(convert_static_param(param, origin)?);
    }
    for port in ports {
        graph = graph.with_port(convert_port(port)?);
    }
    let mut diagnostics = Diagnostics::new();
    graph.validate_definition_structure(&mut diagnostics);
    if diagnostics.has_errors() {
        return Err(diagnostics);
    }
    Ok(graph)
}

fn convert_static_param(
    document: &StaticParamDocument,
    origin: &ResourceOrigin,
) -> Result<StaticParam, Diagnostics> {
    let static_type = match document.static_type {
        StaticTypeDocument::Int => StaticType::Int,
        StaticTypeDocument::Enum => StaticType::Enum,
        StaticTypeDocument::String => StaticType::String,
        StaticTypeDocument::Resource => {
            let Some(kind) = document.resource_kind else {
                return Err(Diagnostic::new(
                    error_codes::KERNEL_DOCUMENT_PARSE_FAILED,
                    Severity::Error,
                    format!(
                        "resource static parameter '{}' must declare resource_kind",
                        document.name
                    ),
                )
                .with_expected("resource_kind: sample or impulse_response")
                .into());
            };
            StaticType::Resource(kind.into_kernel())
        }
    };
    let mut param = StaticParam::new(&document.name, static_type)
        .with_allowed_values(document.allowed_values.iter().map(String::as_str));
    if let Some(default) = &document.default {
        param = param.with_default(convert_static_value(default, static_type, origin)?);
    }
    Ok(param)
}

fn convert_port(document: &PortDocument) -> Result<Port, Diagnostics> {
    let signal = match document.signal {
        SignalDocument::Audio => SignalType::Audio,
        SignalDocument::Control => SignalType::Control,
        SignalDocument::Event => SignalType::Event,
    };
    let channels = match &document.channels {
        ChannelDocument::Literal(value) => ChannelCount::Literal(*value),
        ChannelDocument::Param(value) => {
            ChannelCount::Param(value.strip_prefix('$').unwrap_or(value).to_string())
        }
    };
    let multiplicity = match document.multiplicity {
        MultiplicityDocument::SingleSource => Multiplicity::SingleSource,
        MultiplicityDocument::Summing => Multiplicity::Summing,
    };
    let mut port = match document.direction {
        DirectionDocument::Input => Port::input(&document.name, signal, channels),
        DirectionDocument::Output => Port::output(&document.name, signal, channels),
    };
    port = port.with_multiplicity(multiplicity);
    if let Some(default) = document.default {
        let mut control_default = ControlDefault::new(default);
        if let Some(min) = document.min {
            control_default = control_default.with_min(min);
        }
        if let Some(max) = document.max {
            control_default = control_default.with_max(max);
        }
        if let Some(unit) = &document.unit {
            control_default = control_default.with_unit(unit);
        }
        port = port.with_control_default(control_default);
    }
    for reference in document.maps_to.iter() {
        port = port.maps_to(parse_reference(reference)?);
    }
    for reference in document.maps_from.iter() {
        port = port.maps_from(parse_reference(reference)?);
    }
    Ok(port)
}

fn convert_node(
    document: &NodeDocument,
    registry: &DefinitionRegistry,
    origin: &ResourceOrigin,
) -> Result<Node, Diagnostics> {
    let mut node = Node::new(NodeId::new(&document.id), &document.definition_type);
    for (name, value) in &document.static_args {
        let arg = match value.as_str() {
            Some(binding) if binding.starts_with(LEGACY_BINDING_PREFIX) => {
                return Err(Diagnostic::new(
                    error_codes::KERNEL_DOCUMENT_LEGACY_BINDING,
                    Severity::Error,
                    format!("node '{}' uses legacy binding '{binding}'; use '$name' for static parameter pass-through", document.id),
                )
                .with_module_id(&document.id)
                .with_suggested_fix("replace ${name} with $name for static pass-through, or map a public control port")
                .into());
            }
            Some(reference) if reference.starts_with('$') && !reference.contains(' ') => {
                StaticArg::ParamRef(reference[1..].to_string())
            }
            Some(expression) if expression.starts_with('$') => {
                StaticArg::Expression(expression.to_string())
            }
            _ => {
                let expected = registry
                    .get(&document.definition_type)
                    .and_then(|definition| {
                        definition
                            .static_params()
                            .iter()
                            .find(|param| param.name() == name)
                    })
                    .map(StaticParam::static_type)
                    .unwrap_or_else(|| infer_static_type(value));
                StaticArg::Literal(convert_static_value(value, expected, origin)?)
            }
        };
        node = node.with_static_arg(name, arg);
    }
    for (name, value) in &document.defaults {
        node = node.with_default_override(name, *value);
    }
    Ok(node)
}

fn infer_static_type(value: &Value) -> StaticType {
    if value.as_i64().is_some() {
        StaticType::Int
    } else if let Ok(reference) = serde_yaml::from_value::<ResourceRefDocument>(value.clone()) {
        StaticType::Resource(reference.kind.into_kernel())
    } else {
        StaticType::String
    }
}

fn convert_static_value(
    value: &Value,
    expected: StaticType,
    origin: &ResourceOrigin,
) -> Result<StaticValue, Diagnostics> {
    let converted = match expected {
        StaticType::Int => value.as_i64().map(StaticValue::Int),
        StaticType::Enum => value.as_str().map(|value| StaticValue::Enum(value.into())),
        StaticType::String => value
            .as_str()
            .map(|value| StaticValue::String(value.into())),
        StaticType::Resource(_) => serde_yaml::from_value::<ResourceRefDocument>(value.clone())
            .ok()
            .map(|reference| {
                StaticValue::Resource(ResourceRef::new(
                    reference.kind.into_kernel(),
                    reference.path,
                    origin.clone(),
                ))
            }),
    };
    converted.ok_or_else(|| {
        Diagnostic::new(
            error_codes::KERNEL_DOCUMENT_PARSE_FAILED,
            Severity::Error,
            format!("static value {value:?} does not match declared type {expected:?}"),
        )
        .with_expected(format!("{expected:?}"))
        .into()
    })
}

fn parse_reference(reference: &str) -> Result<PortRef, Diagnostics> {
    let Some((node, port)) = reference.split_once('.') else {
        return Err(Diagnostic::new(
            error_codes::KERNEL_DOCUMENT_PARSE_FAILED,
            Severity::Error,
            format!("port reference '{reference}' must have the form module.port"),
        )
        .with_expected("module.port")
        .with_actual(reference)
        .into());
    };
    Ok(PortRef::new(NodeId::new(node), port))
}

#[cfg(test)]
mod tests;
