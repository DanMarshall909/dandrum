use std::collections::{BTreeMap, BTreeSet};

use crate::builtins::{BuiltInModuleRegistry, module_types};
use crate::patch::{
    AssetKind, ConnectionDeclaration, ModuleDeclaration, ParameterValue, PatchDocument,
    PortReference, SignalType, validate_port_reference,
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

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CompositeBindingDeclaration {
    pub name: String,
    #[serde(default)]
    pub maps_to: Vec<PortReference>,
}

pub(super) fn validate_module_definitions(patch: &PatchDocument, diagnostics: &mut Vec<String>) {
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

pub(super) fn validate_composite_instance_bindings(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::{
        AssetDeclaration, AssetKind, PatchMetadata, RenderSettings, VoiceAllocation,
    };
    use std::collections::BTreeMap;

    #[test]
    fn validation_reports_blank_composite_type() {
        let patch = patch_with_definitions(vec![definition("")]);
        let mut diagnostics = Vec::new();

        validate_module_definitions(&patch, &mut diagnostics);

        assert!(diagnostics.contains(&"composite module type is required".to_string()));
    }

    #[test]
    fn asset_binding_without_instance_value_is_allowed() {
        let mut definition = definition("voice");
        definition.asset_bindings = vec![CompositeBindingDeclaration {
            name: "sample".to_string(),
            maps_to: vec![port_ref("sampler.asset")],
        }];
        let patch = patch_with_definitions(vec![definition]);
        let instance = module("voice", "voice");
        let mut diagnostics = Vec::new();

        validate_composite_instance_bindings(&instance, &patch, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn missing_internal_mapping_target_is_left_for_later_graph_validation() {
        let mut definition = definition("voice");
        definition.inputs[0].maps_to = vec![port_ref("missing.gate")];
        let patch = patch_with_definitions(vec![definition]);
        let mut diagnostics = Vec::new();

        validate_module_definitions(&patch, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn script_declared_input_mapping_validates() {
        let mut definition = definition("voice");
        definition.modules = vec![script_module()];
        definition.inputs[0].maps_to = vec![port_ref("script.events")];
        let patch = patch_with_definitions(vec![definition]);
        let mut diagnostics = Vec::new();

        validate_module_definitions(&patch, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn script_declared_output_wrong_direction_is_rejected() {
        let mut definition = definition("voice");
        definition.modules = vec![script_module()];
        definition.outputs[0].maps_from = vec![port_ref("script.events")];
        let patch = patch_with_definitions(vec![definition]);
        let mut diagnostics = Vec::new();

        validate_module_definitions(&patch, &mut diagnostics);

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.contains("script.events") && diagnostic.contains("internal output port")
        }));
    }

    #[test]
    fn script_missing_declared_port_is_left_for_later_graph_validation() {
        let mut definition = definition("voice");
        definition.modules = vec![script_module()];
        definition.inputs[0].maps_to = vec![port_ref("script.missing")];
        let patch = patch_with_definitions(vec![definition]);
        let mut diagnostics = Vec::new();

        validate_module_definitions(&patch, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn recursion_walk_handles_definitions_without_dependencies() {
        let dependencies = BTreeMap::new();
        let mut stack = Vec::new();
        let mut reported = BTreeSet::new();

        collect_recursive_composite_paths("voice", &dependencies, &mut stack, &mut reported);

        assert!(reported.is_empty());
        assert!(stack.is_empty());
    }

    #[test]
    fn built_in_missing_internal_port_is_left_for_later_graph_validation() {
        let mut definition = definition("voice");
        definition.inputs[0].maps_to = vec![port_ref("env.missing")];
        let patch = patch_with_definitions(vec![definition]);
        let mut diagnostics = Vec::new();

        validate_module_definitions(&patch, &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn built_in_wrong_direction_output_mapping_is_rejected() {
        let mut definition = definition("voice");
        definition.outputs[0].maps_from = vec![port_ref("vca.audio_in")];
        let patch = patch_with_definitions(vec![definition]);
        let mut diagnostics = Vec::new();

        validate_module_definitions(&patch, &mut diagnostics);

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.contains("vca.audio_in") && diagnostic.contains("internal output port")
        }));
    }

    fn patch_with_definitions(
        module_definitions: Vec<ModuleDefinitionDeclaration>,
    ) -> PatchDocument {
        PatchDocument {
            metadata: PatchMetadata {
                name: "Test".to_string(),
                version: None,
                author: None,
            },
            render: RenderSettings {
                sample_rate_hz: 48_000,
                block_size_frames: 128,
                duration_frames: 48_000,
            },
            assets: vec![AssetDeclaration {
                id: "hit".to_string(),
                kind: AssetKind::Sample,
                path: "hit.wav".to_string(),
            }],
            module_definitions,
            modules: vec![module("voice", "voice")],
            connections: vec![],
            voice_allocation: VoiceAllocation::default(),
        }
    }

    fn definition(module_type: &str) -> ModuleDefinitionDeclaration {
        ModuleDefinitionDeclaration {
            module_type: module_type.to_string(),
            inputs: vec![CompositeInputDeclaration {
                name: "trigger".to_string(),
                signal_type: SignalType::Event,
                maps_to: vec![port_ref("env.gate")],
            }],
            outputs: vec![CompositeOutputDeclaration {
                name: "audio".to_string(),
                signal_type: SignalType::Audio,
                maps_from: vec![port_ref("vca.audio_out")],
            }],
            parameters: vec![],
            asset_bindings: vec![],
            modules: vec![module("env", "adsr"), module("vca", "gain")],
            connections: vec![],
        }
    }

    fn script_module() -> ModuleDeclaration {
        let mut script = module("script", "script");
        script.inputs = vec![crate::patch::PortDeclaration {
            name: "events".to_string(),
            signal_type: SignalType::Event,
        }];
        script.outputs = vec![crate::patch::PortDeclaration {
            name: "audio".to_string(),
            signal_type: SignalType::Audio,
        }];
        script
    }

    fn module(id: &str, module_type: &str) -> ModuleDeclaration {
        ModuleDeclaration {
            id: id.to_string(),
            module_type: module_type.to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: BTreeMap::new(),
        }
    }

    fn port_ref(reference: &str) -> PortReference {
        let (module_id, port_name) = reference.split_once('.').unwrap();
        PortReference {
            module_id: module_id.to_string(),
            port_name: port_name.to_string(),
        }
    }
}
