use std::collections::{BTreeMap, BTreeSet};

use crate::builtins::{BuiltInModuleRegistry, module_types};
use crate::diagnostics::{Diagnostic, Severity, error_codes};
use crate::patch::{
    AssetKind, ConnectionDeclaration, ModuleDeclaration, ParameterValue, PatchDocument,
    PatchValidationError, PortReference, SignalType, validate_port_reference,
};
use serde::Deserialize;

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

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CompositeBindingDeclaration {
    pub name: String,
    #[serde(rename = "type")]
    pub value_type: Option<CompositeParameterValueType>,
    pub default: Option<ParameterValue>,
    pub value: Option<ParameterValue>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub unit: Option<String>,
    pub description: Option<String>,
    pub required: Option<bool>,
    pub expression: Option<String>,
    #[serde(default)]
    pub maps_to: Vec<PortReference>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompositeParameterValueType {
    Boolean,
    Number,
    String,
}

pub(super) fn validate_module_definitions(
    patch: &PatchDocument,
    diagnostics: &mut PatchValidationError,
) {
    let mut module_types = BTreeSet::new();
    let registry = BuiltInModuleRegistry::new();

    for definition in &patch.module_definitions {
        if definition.module_type.trim().is_empty() {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_MISSING_FIELD,
                Severity::Error,
                "composite module type is required",
            ));
        } else if !module_types.insert(definition.module_type.as_str()) {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Error,
                format!("duplicate composite module type: {}", definition.module_type),
            ));
        }

        validate_composite_parameter_declarations(definition, diagnostics);
        validate_public_inputs(definition, &registry, diagnostics);
        validate_public_outputs(definition, &registry, diagnostics);
    }

    validate_recursive_composite_definitions(patch, diagnostics);
}

fn validate_public_inputs(
    definition: &ModuleDefinitionDeclaration,
    registry: &BuiltInModuleRegistry,
    diagnostics: &mut PatchValidationError,
) {
    for input in &definition.inputs {
        let port_name = composite_port_name(&input.name);
        if input.name.trim().is_empty() {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_MISSING_FIELD,
                Severity::Error,
                format!("composite {} input name is required", definition.module_type),
            ));
        }

        for reference in &input.maps_to {
            validate_port_reference(
                &format!("composite {} input {port_name} maps_to", definition.module_type),
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
                registry,
                diagnostics,
            );
        }
    }
}

fn validate_public_outputs(
    definition: &ModuleDefinitionDeclaration,
    registry: &BuiltInModuleRegistry,
    diagnostics: &mut PatchValidationError,
) {
    for output in &definition.outputs {
        let port_name = composite_port_name(&output.name);
        if output.name.trim().is_empty() {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_MISSING_FIELD,
                Severity::Error,
                format!("composite {} output name is required", definition.module_type),
            ));
        }

        for reference in &output.maps_from {
            validate_port_reference(
                &format!("composite {} output {port_name} maps_from", definition.module_type),
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
                registry,
                diagnostics,
            );
        }
    }
}

fn validate_composite_parameter_declarations(
    definition: &ModuleDefinitionDeclaration,
    diagnostics: &mut PatchValidationError,
) {
    let mut names = BTreeSet::new();

    for parameter in &definition.parameters {
        let name = composite_parameter_name(&parameter.name);
        if parameter.name.trim().is_empty() {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_MISSING_FIELD,
                Severity::Error,
                format!("composite {} parameter name is required", definition.module_type),
            ));
            continue;
        }

        if !names.insert(parameter.name.as_str()) {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Error,
                format!(
                    "composite {} duplicate parameter name {}",
                    definition.module_type, name
                ),
            ));
        }

        if parameter.expression.is_some() {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Error,
                format!(
                    "composite {} parameter {} uses unsupported expression syntax",
                    definition.module_type, name
                ),
            ));
        }

        validate_composite_parameter_default(definition, parameter, diagnostics);
        validate_composite_parameter_literal_value(definition, parameter, diagnostics);
        validate_composite_parameter_constraints(definition, parameter, diagnostics);
    }
}

fn validate_composite_parameter_default(
    definition: &ModuleDefinitionDeclaration,
    parameter: &CompositeBindingDeclaration,
    diagnostics: &mut PatchValidationError,
) {
    let Some(default) = &parameter.default else { return; };
    let name = composite_parameter_name(&parameter.name);

    if let Some(value_type) = parameter.value_type {
        if !composite_value_matches_type(default, value_type) {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_TYPE_MISMATCH,
                Severity::Error,
                format!(
                    "composite {} parameter {} default has wrong type: expected {}, got {}",
                    definition.module_type,
                    name,
                    composite_type_name(value_type),
                    parameter_value_type_name(default)
                ),
            ));
            return;
        }
    }

    validate_composite_numeric_range(definition, parameter, "default", default, diagnostics);
}

fn validate_composite_parameter_literal_value(
    definition: &ModuleDefinitionDeclaration,
    parameter: &CompositeBindingDeclaration,
    diagnostics: &mut PatchValidationError,
) {
    let Some(value) = &parameter.value else { return; };
    let name = composite_parameter_name(&parameter.name);

    if let Some(value_type) = parameter.value_type {
        if !composite_value_matches_type(value, value_type) {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_TYPE_MISMATCH,
                Severity::Error,
                format!(
                    "composite {} parameter {} literal value has wrong type: expected {}, got {}",
                    definition.module_type,
                    name,
                    composite_type_name(value_type),
                    parameter_value_type_name(value)
                ),
            ));
            return;
        }
    }

    validate_composite_numeric_range(definition, parameter, "value", value, diagnostics);
}

fn validate_composite_parameter_constraints(
    definition: &ModuleDefinitionDeclaration,
    parameter: &CompositeBindingDeclaration,
    diagnostics: &mut PatchValidationError,
) {
    let name = composite_parameter_name(&parameter.name);

    if let (Some(min), Some(max)) = (parameter.min, parameter.max) {
        if min > max {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Error,
                format!(
                    "composite {} parameter {} has invalid range: min {} is greater than max {}",
                    definition.module_type, name, min, max
                ),
            ));
        }
    }

    if matches!(
        parameter.value_type,
        Some(CompositeParameterValueType::Boolean | CompositeParameterValueType::String)
    ) && (parameter.min.is_some() || parameter.max.is_some())
    {
        diagnostics.push(Diagnostic::new(
            error_codes::VALIDATION_INVALID_VALUE,
            Severity::Error,
            format!(
                "composite {} parameter {} has numeric constraints on a {} parameter",
                definition.module_type,
                name,
                composite_type_name(parameter.value_type.expect("checked above"))
            ),
        ));
    }
}

fn validate_composite_numeric_range(
    definition: &ModuleDefinitionDeclaration,
    parameter: &CompositeBindingDeclaration,
    value_label: &str,
    value: &ParameterValue,
    diagnostics: &mut PatchValidationError,
) {
    let ParameterValue::Number(actual) = value else { return; };
    let name = composite_parameter_name(&parameter.name);

    if let Some(min) = parameter.min {
        if *actual < min {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Error,
                format!(
                    "composite {} parameter {} {value_label} is below minimum {}: {}",
                    definition.module_type, name, min, actual
                ),
            ));
        }
    }

    if let Some(max) = parameter.max {
        if *actual > max {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Error,
                format!(
                    "composite {} parameter {} {value_label} is above maximum {}: {}",
                    definition.module_type, name, max, actual
                ),
            ));
        }
    }
}

fn composite_value_matches_type(value: &ParameterValue, value_type: CompositeParameterValueType) -> bool {
    matches!(
        (value, value_type),
        (ParameterValue::Boolean(_), CompositeParameterValueType::Boolean)
            | (ParameterValue::Number(_), CompositeParameterValueType::Number)
            | (ParameterValue::Text(_), CompositeParameterValueType::String)
    )
}

fn composite_type_name(value_type: CompositeParameterValueType) -> &'static str {
    match value_type {
        CompositeParameterValueType::Boolean => "boolean",
        CompositeParameterValueType::Number => "number",
        CompositeParameterValueType::String => "string",
    }
}

fn parameter_value_type_name(value: &ParameterValue) -> &'static str {
    match value {
        ParameterValue::Boolean(_) => "boolean",
        ParameterValue::Number(_) => "number",
        ParameterValue::Text(_) => "string",
    }
}

fn validate_recursive_composite_definitions(
    patch: &PatchDocument,
    diagnostics: &mut PatchValidationError,
) {
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
        diagnostics.push(Diagnostic::new(
            error_codes::VALIDATION_INVALID_VALUE,
            Severity::Error,
            format!("recursive composite definition: {path}"),
        ));
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

pub(super) fn validate_composite_instance_bindings(
    module: &ModuleDeclaration,
    patch: &PatchDocument,
    diagnostics: &mut PatchValidationError,
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
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_INVALID_VALUE,
                Severity::Error,
                format!(
                    "composite {} instance {} sets undeclared parameter {}",
                    definition.module_type, module.id, key
                ),
            ));
        }
    }

    for parameter in &definition.parameters {
        let Some(value) = module.parameters.get(&parameter.name) else {
            continue;
        };

        if let Some(value_type) = parameter.value_type {
            if !composite_value_matches_type(value, value_type) {
                diagnostics.push(Diagnostic::new(
                    error_codes::VALIDATION_TYPE_MISMATCH,
                    Severity::Error,
                    format!(
                        "composite {} instance {} parameter {} has wrong type: expected {}, got {}",
                        definition.module_type,
                        module.id,
                        parameter.name,
                        composite_type_name(value_type),
                        parameter_value_type_name(value)
                    ),
                ));
                continue;
            }
        }

        validate_composite_numeric_range(definition, parameter, "value", value, diagnostics);
    }

    for binding in &definition.asset_bindings {
        let Some(value) = module.parameters.get(&binding.name) else {
            continue;
        };
        let ParameterValue::Text(asset_id) = value else {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_TYPE_MISMATCH,
                Severity::Error,
                format!(
                    "composite {} instance {} asset binding {} must be a text asset ID",
                    definition.module_type, module.id, binding.name
                ),
            ));
            continue;
        };
        let Some(asset) = patch.assets.iter().find(|asset| asset.id == *asset_id) else {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_UNKNOWN_MODULE,
                Severity::Error,
                format!(
                    "composite {} instance {} asset binding {} references missing asset {}",
                    definition.module_type, module.id, binding.name, asset_id
                ),
            ));
            continue;
        };
        if asset.kind != AssetKind::Sample {
            diagnostics.push(Diagnostic::new(
                error_codes::VALIDATION_TYPE_MISMATCH,
                Severity::Error,
                format!(
                    "composite {} instance {} asset binding {} references asset {} with kind {:?}; expected sample",
                    definition.module_type, module.id, binding.name, asset_id, asset.kind
                ),
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
    diagnostics: &mut PatchValidationError,
) {
    if reference.module_id.trim().is_empty() || reference.port_name.trim().is_empty() {
        return;
    }

    let resolved = resolve_internal_port_type(definition, reference, direction, registry);
    if resolved == InternalPortResolution::WrongDirection {
        diagnostics.push(Diagnostic::new(
            error_codes::VALIDATION_INVALID_VALUE,
            Severity::Error,
            format!(
                "composite {definition_type} {public_direction_label} {} {mapping_label} {reference} must reference an internal {} port",
                composite_port_name(public_name),
                match direction {
                    CompositeMappingDirection::PublicInputToInternalInput => "input",
                    CompositeMappingDirection::PublicOutputFromInternalOutput => "output",
                }
            ),
        ));
        return;
    }

    let InternalPortResolution::Found(internal_type) = resolved else {
        return;
    };

    if public_signal_type != internal_type {
        diagnostics.push(Diagnostic::new(
            error_codes::VALIDATION_TYPE_MISMATCH,
            Severity::Error,
            format!(
                "composite {definition_type} {public_direction_label} {} {mapping_label} {reference} has incompatible signal types: public {:?}, internal {:?}",
                composite_port_name(public_name),
                public_signal_type,
                internal_type
            ),
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

    let built_in = built_in.expect("built-in module definition checked above");
    let expected_ports = match direction {
        CompositeMappingDirection::PublicInputToInternalInput => built_in.inputs(),
        CompositeMappingDirection::PublicOutputFromInternalOutput => built_in.outputs(),
    };
    if let Some(port) = expected_ports
        .iter()
        .find(|port| port.name() == reference.port_name)
    {
        return InternalPortResolution::Found(signal_type_from_graph(port.signal_type()));
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

fn signal_type_from_graph(signal_type: crate::graph::SignalType) -> SignalType {
    match signal_type {
        crate::graph::SignalType::Audio => SignalType::Audio,
        crate::graph::SignalType::Control => SignalType::Control,
        crate::graph::SignalType::Event => SignalType::Event,
    }
}

fn composite_port_name(name: &str) -> &str {
    if name.trim().is_empty() {
        "<unnamed>"
    } else {
        name
    }
}

fn composite_parameter_name(name: &str) -> &str {
    if name.trim().is_empty() {
        "<unnamed>"
    } else {
        name
    }
}
