//! YAML front end for the unified graph kernel.
//!
//! This remains additive while the legacy [`crate::patch::PatchDocument`]
//! drives preparation. Both root patches and inline composites pass through the
//! same graph-declaration conversion into [`GraphDefinition`].

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use serde_yaml::{Mapping, Value};

use crate::diagnostics::{Diagnostic, Diagnostics, Severity, error_codes};
use crate::graph::{PortDirection, SignalType};

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
    root: GraphDefinition,
    registry: DefinitionRegistry,
}

impl KernelPatch {
    pub fn metadata(&self) -> &KernelPatchMetadata {
        &self.metadata
    }

    pub fn root(&self) -> &GraphDefinition {
        &self.root
    }

    pub fn registry(&self) -> &DefinitionRegistry {
        &self.registry
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
    #[serde(default)]
    static_params: Vec<StaticParamDocument>,
    #[serde(default)]
    ports: Vec<PortDocument>,
    #[serde(default)]
    module_definitions: Vec<CompositeDocument>,
    #[serde(default)]
    modules: Vec<NodeDocument>,
    #[serde(default)]
    connections: Vec<ConnectionDocument>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompositeDocument {
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
    for composite in &document.module_definitions {
        registry = registry.with_definition(convert_declaration(
            &composite.definition_type,
            composite.implementation.as_deref(),
            &composite.static_params,
            &composite.ports,
            &origin,
        )?);
    }
    for composite in &document.module_definitions {
        let definition = convert_graph(
            &composite.definition_type,
            composite.implementation.as_deref(),
            &composite.static_params,
            &composite.ports,
            &composite.modules,
            &composite.connections,
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

    Ok(KernelPatch {
        metadata,
        root,
        registry,
    })
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
