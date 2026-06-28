use std::collections::BTreeMap;

use crate::patch;

pub(super) fn expand_patch_declarations(
    patch: &patch::PatchDocument,
) -> (
    Vec<patch::ModuleDeclaration>,
    Vec<patch::ConnectionDeclaration>,
) {
    let definitions = patch
        .module_definitions
        .iter()
        .map(|definition| (definition.module_type.as_str(), definition))
        .collect::<BTreeMap<_, _>>();
    let instances = patch
        .modules
        .iter()
        .filter_map(|module| {
            definitions
                .get(module.module_type.as_str())
                .map(|definition| (module.id.as_str(), *definition))
        })
        .collect::<BTreeMap<_, _>>();
    if instances.is_empty() {
        return (patch.modules.clone(), patch.connections.clone());
    }

    let mut modules = Vec::new();
    let mut connections = Vec::new();

    for module in &patch.modules {
        let Some(definition) = definitions.get(module.module_type.as_str()) else {
            modules.push(module.clone());
            continue;
        };

        for internal in &definition.modules {
            let mut expanded = internal.clone();
            expanded.id = namespaced_id(&module.id, &internal.id);
            modules.push(expanded);
        }

        for connection in &definition.connections {
            connections.push(patch::ConnectionDeclaration {
                from: patch::PortReference {
                    module_id: namespaced_id(&module.id, &connection.from.module_id),
                    port_name: connection.from.port_name.clone(),
                },
                to: patch::PortReference {
                    module_id: namespaced_id(&module.id, &connection.to.module_id),
                    port_name: connection.to.port_name.clone(),
                },
            });
        }
    }

    for connection in &patch.connections {
        let sources = expand_source_reference(&connection.from, &instances);
        let destinations = expand_destination_reference(&connection.to, &instances);

        for source in &sources {
            for destination in &destinations {
                connections.push(patch::ConnectionDeclaration {
                    from: source.clone(),
                    to: destination.clone(),
                });
            }
        }
    }

    (modules, connections)
}

fn expand_source_reference(
    reference: &patch::PortReference,
    instances: &BTreeMap<&str, &patch::ModuleDefinitionDeclaration>,
) -> Vec<patch::PortReference> {
    let Some(definition) = instances.get(reference.module_id.as_str()) else {
        return vec![reference.clone()];
    };
    let Some(output) = definition
        .outputs
        .iter()
        .find(|output| output.name == reference.port_name)
    else {
        return vec![reference.clone()];
    };

    output
        .maps_from
        .iter()
        .map(|mapped| patch::PortReference {
            module_id: namespaced_id(&reference.module_id, &mapped.module_id),
            port_name: mapped.port_name.clone(),
        })
        .collect()
}

fn expand_destination_reference(
    reference: &patch::PortReference,
    instances: &BTreeMap<&str, &patch::ModuleDefinitionDeclaration>,
) -> Vec<patch::PortReference> {
    let Some(definition) = instances.get(reference.module_id.as_str()) else {
        return vec![reference.clone()];
    };
    let Some(input) = definition
        .inputs
        .iter()
        .find(|input| input.name == reference.port_name)
    else {
        return vec![reference.clone()];
    };

    input
        .maps_to
        .iter()
        .map(|mapped| patch::PortReference {
            module_id: namespaced_id(&reference.module_id, &mapped.module_id),
            port_name: mapped.port_name.clone(),
        })
        .collect()
}

fn namespaced_id(instance_id: &str, internal_id: &str) -> String {
    format!("{instance_id}::{internal_id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::{
        CompositeInputDeclaration, CompositeOutputDeclaration, ModuleDeclaration,
        ModuleDefinitionDeclaration, PatchDocument, PatchMetadata, PortReference, RenderSettings,
        SignalType, VoiceAllocation,
    };
    use std::collections::BTreeMap;

    #[test]
    fn patch_without_composite_instances_is_returned_unchanged() {
        let patch = patch_with(vec![ordinary("osc", "oscillator")], vec![]);

        let (modules, connections) = expand_patch_declarations(&patch);

        assert_eq!(modules, patch.modules);
        assert_eq!(connections, patch.connections);
    }

    #[test]
    fn unknown_composite_public_ports_are_left_for_graph_validation() {
        let patch = patch_with(
            vec![
                ordinary("voice", "drum_voice"),
                ordinary("out", "audio_output"),
            ],
            vec![definition("drum_voice")],
        );
        let mut patch = patch;
        patch.connections = vec![connection("voice.hidden", "out.left")];

        let (_modules, connections) = expand_patch_declarations(&patch);

        assert_eq!(
            connections.last(),
            Some(&connection("voice.hidden", "out.left"))
        );
    }

    #[test]
    fn unknown_composite_public_input_is_left_for_graph_validation() {
        let patch = patch_with(
            vec![
                ordinary("midi", "midi_input"),
                ordinary("voice", "drum_voice"),
            ],
            vec![definition("drum_voice")],
        );
        let mut patch = patch;
        patch.connections = vec![connection("midi.events", "voice.hidden")];

        let (_modules, connections) = expand_patch_declarations(&patch);

        assert_eq!(
            connections.last(),
            Some(&connection("midi.events", "voice.hidden"))
        );
    }

    fn patch_with(
        modules: Vec<ModuleDeclaration>,
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
            assets: vec![],
            module_definitions,
            modules,
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
                maps_from: vec![port_ref("osc.audio")],
            }],
            parameters: vec![],
            asset_bindings: vec![],
            modules: vec![ordinary("osc", "oscillator"), ordinary("env", "adsr")],
            connections: vec![connection("osc.audio", "env.gate")],
        }
    }

    fn ordinary(id: &str, module_type: &str) -> ModuleDeclaration {
        ModuleDeclaration {
            id: id.to_string(),
            module_type: module_type.to_string(),
            inputs: vec![],
            outputs: vec![],
            parameters: BTreeMap::new(),
        }
    }

    fn connection(from: &str, to: &str) -> patch::ConnectionDeclaration {
        patch::ConnectionDeclaration {
            from: port_ref(from),
            to: port_ref(to),
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
