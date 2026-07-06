use std::collections::{BTreeMap, VecDeque};
use std::fmt;

use crate::builtins::module_kind::ModuleKind;
use crate::diagnostics::{Diagnostic, Severity, error_codes};
use crate::graph::{ExecutionScope, Graph, ModuleId, SignalType};
use crate::patch::RenderSettings;

pub type ExecutionStep = usize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledPatch {
    nodes: Vec<CompiledNode>,
    topological_order: Vec<ExecutionStep>,
    execution_order: Vec<ExecutionStep>,
    voice_node_indices: Vec<usize>,
    global_node_indices: Vec<usize>,
    midi_input_index: Option<usize>,
    audio_output_index: Option<usize>,
    module_output_buffer_layout: Vec<CompiledModuleBufferLayout>,
    total_output_buffer_count: usize,
    render_settings: RenderSettings,
    parameter_slots: Vec<ParameterSlot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledNode {
    pub id: ModuleId,
    pub module_type: String,
    pub module_kind: ModuleKind,
    pub execution_scope: ExecutionScope,
    pub input_port_map: Vec<Vec<CompiledPortRef>>,
    pub input_routes: Vec<Vec<CompiledInputSource>>,
    pub output_port_map: Vec<usize>,
    pub input_port_indices: BTreeMap<String, usize>,
    pub input_port_names: Vec<String>,
    pub input_port_types: Vec<SignalType>,
    pub output_port_names: Vec<String>,
    pub output_port_types: Vec<SignalType>,
    pub parameters: BTreeMap<String, String>,
    pub parameter_slot_indices: BTreeMap<String, usize>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParameterSlot {
    value: f32,
}

impl Eq for ParameterSlot {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompiledPortRef {
    pub module_index: usize,
    pub port_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledInputSource {
    pub module_index: usize,
    pub port_index: usize,
    pub output_buffer_id: usize,
    pub output_port_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompiledModuleBufferLayout {
    pub output_buffer_start: usize,
    pub output_buffer_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompileError {
    MissingPort {
        module_id: String,
        port_name: String,
    },
    CycleDetected,
    UnknownModuleType {
        module_type: String,
    },
    UnsupportedModuleType {
        module_type: String,
    },
}

impl CompileError {
    #[allow(dead_code)]
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            Self::MissingPort {
                module_id,
                port_name,
            } => Diagnostic::new(
                error_codes::GRAPH_MISSING_PORT,
                Severity::Error,
                format!("missing port: {module_id}.{port_name}"),
            )
            .with_module_id(module_id.clone())
            .with_port_name(port_name.clone()),
            Self::CycleDetected => Diagnostic::new(
                error_codes::GRAPH_CYCLE_DETECTED,
                Severity::Error,
                "routing cycle detected during compilation",
            ),
            Self::UnknownModuleType { module_type } => Diagnostic::new(
                error_codes::GRAPH_UNKNOWN_MODULE_TYPE,
                Severity::Error,
                format!("unknown module type: {module_type}"),
            ),
            Self::UnsupportedModuleType { module_type } => Diagnostic::new(
                error_codes::GRAPH_UNSUPPORTED_MODULE_TYPE,
                Severity::Error,
                format!("unsupported module type for rendering: {module_type}"),
            ),
        }
    }
}

pub fn compile(
    graph: &Graph,
    render_settings: &RenderSettings,
) -> Result<CompiledPatch, CompileError> {
    let module_indices = module_indices_by_id(graph);
    let topological_order = topological_sort(graph, &module_indices)?;
    let mut next_output_buffer = 0;
    let mut module_output_buffer_layout = Vec::with_capacity(graph.modules().len());
    let mut parameter_slots = Vec::new();
    let nodes: Vec<_> = graph
        .modules()
        .iter()
        .map(|module| {
            let module_type_str = module.module_type();
            let kind = ModuleKind::from_str(module_type_str).ok_or_else(|| {
                CompileError::UnknownModuleType {
                    module_type: module_type_str.to_string(),
                }
            })?;
            if !kind.is_render_supported() {
                return Err(CompileError::UnsupportedModuleType {
                    module_type: module_type_str.to_string(),
                });
            }
            let input_count = module.inputs().len();
            let output_count = module.outputs().len();
            let output_buffer_start = next_output_buffer;
            next_output_buffer += output_count;
            module_output_buffer_layout.push(CompiledModuleBufferLayout {
                output_buffer_start,
                output_buffer_count: output_count,
            });
            let input_port_names: Vec<String> = module
                .inputs()
                .iter()
                .map(|p| p.name().to_string())
                .collect();
            let output_port_names: Vec<String> = module
                .outputs()
                .iter()
                .map(|p| p.name().to_string())
                .collect();
            let input_port_indices = input_port_names
                .iter()
                .enumerate()
                .map(|(index, name)| (name.clone(), index))
                .collect();
            let parameters = module.params().clone();
            let parameter_slot_indices = parameters
                .iter()
                .filter_map(|(name, value)| {
                    let value = value.parse::<f32>().ok()?;
                    let slot_index = parameter_slots.len();
                    parameter_slots.push(ParameterSlot { value });
                    Some((name.clone(), slot_index))
                })
                .collect();

            Ok(CompiledNode {
                id: module.id().clone(),
                module_type: module_type_str.to_string(),
                module_kind: kind,
                execution_scope: module.execution_scope(),
                input_port_map: vec![Vec::new(); input_count],
                input_routes: vec![Vec::new(); input_count],
                output_port_map: (output_buffer_start..next_output_buffer).collect(),
                input_port_indices,
                input_port_names,
                input_port_types: module.inputs().iter().map(|p| p.signal_type()).collect(),
                output_port_names,
                output_port_types: module.outputs().iter().map(|p| p.signal_type()).collect(),
                parameters,
                parameter_slot_indices,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut nodes = nodes;

    resolve_routing(graph, &module_indices, &mut nodes)?;

    let global_node_indices = topological_order
        .iter()
        .copied()
        .filter(|index| nodes[*index].execution_scope == ExecutionScope::Global)
        .collect::<Vec<_>>();
    let voice_node_indices = topological_order
        .iter()
        .copied()
        .filter(|index| nodes[*index].execution_scope == ExecutionScope::Voice)
        .collect::<Vec<_>>();
    let execution_order = global_node_indices
        .iter()
        .chain(voice_node_indices.iter())
        .copied()
        .collect();

    Ok(CompiledPatch {
        nodes,
        topological_order,
        execution_order,
        voice_node_indices,
        global_node_indices,
        midi_input_index: graph
            .modules()
            .iter()
            .position(|module| module.module_type() == "midi_input"),
        audio_output_index: graph
            .modules()
            .iter()
            .position(|module| module.module_type() == "audio_output"),
        module_output_buffer_layout,
        total_output_buffer_count: next_output_buffer,
        render_settings: render_settings.clone(),
        parameter_slots,
    })
}\n
impl CompiledPatch {
    pub fn nodes(&self) -> &[CompiledNode] {
        &self.nodes
    }

    pub fn topological_order(&self) -> &[ExecutionStep] {
        &self.topological_order
    }

    pub fn execution_order(&self) -> &[ExecutionStep] {
        &self.execution_order
    }

    pub fn voice_node_indices(&self) -> &[usize] {
        &self.voice_node_indices
    }

    pub fn global_node_indices(&self) -> &[usize] {
        &self.global_node_indices
    }

    pub fn midi_input_index(&self) -> Option<usize> {
        self.midi_input_index
    }

    pub fn audio_output_index(&self) -> Option<usize> {
        self.audio_output_index
    }

    pub fn module_output_buffer_layout(&self) -> &[CompiledModuleBufferLayout] {
        &self.module_output_buffer_layout
    }

    pub fn total_output_buffer_count(&self) -> usize {
        self.total_output_buffer_count
    }

    pub fn render_settings(&self) -> &RenderSettings {
        &self.render_settings
    }

    pub fn parameter_slot_value(&self, slot_index: usize) -> Option<f32> {
        self.parameter_slots.get(slot_index).map(|slot| slot.value)
    }

    pub fn numeric_parameter_value(&self, module_id: &str, parameter_name: &str) -> Option<f32> {
        let slot_index = self.parameter_slot_index(module_id, parameter_name)?;
        self.parameter_slot_value(slot_index)
    }

    pub fn set_numeric_parameter_by_target(
        &mut self,
        module_id: &str,
        parameter_name: &str,
        value: f32,
    ) -> bool {
        let Some(slot_index) = self.parameter_slot_index(module_id, parameter_name) else {
            return false;
        };
        let Some(slot) = self.parameter_slots.get_mut(slot_index) else {
            return false;
        };

        slot.value = value;
        true
    }

    fn parameter_slot_index(&self, module_id: &str, parameter_name: &str) -> Option<usize> {
        self.nodes
            .iter()
            .find(|node| node.id.as_str() == module_id)?
            .parameter_slot_indices
            .get(parameter_name)
            .copied()
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPort {
                module_id,
                port_name,
            } => write!(formatter, "missing port: {module_id}.{port_name}"),
            Self::CycleDetected => write!(formatter, "routing cycle detected"),
            Self::UnknownModuleType { module_type } => {
                write!(formatter, "unknown module type: {module_type}")
            }
            Self::UnsupportedModuleType { module_type } => {
                write!(formatter, "unsupported module type: {module_type}")
            }
        }
    }
}

impl std::error::Error for CompileError {}

fn module_indices_by_id(graph: &Graph) -> BTreeMap<&str, usize> {
    graph
        .modules()
        .iter()
        .enumerate()
        .map(|(index, module)| (module.id().as_str(), index))
        .collect()
}

fn topological_sort(
    graph: &Graph,
    module_indices: &BTreeMap<&str, usize>,
) -> Result<Vec<usize>, CompileError> {
    let module_count = graph.modules().len();
    let mut in_degree = vec![0usize; module_count];
    let mut adjacency = vec![Vec::new(); module_count];

    for cable in graph.cables() {
        let source = module_index(module_indices, cable.source().module_id().as_str(), "")?;
        let destination =
            module_index(module_indices, cable.destination().module_id().as_str(), "")?;
        adjacency[source].push(destination);
        in_degree[destination] += 1;
    }

    let mut ready = in_degree
        .iter()
        .enumerate()
        .filter_map(|(index, degree)| (*degree == 0).then_some(index))
        .collect::<VecDeque<_>>();
    let mut sorted = Vec::with_capacity(module_count);

    while let Some(index) = ready.pop_front() {
        sorted.push(index);

        for &next in &adjacency[index] {
            in_degree[next] -= 1;
            if in_degree[next] == 0 {
                ready.push_back(next);
            }
        }
    }

    if sorted.len() == module_count {
        Ok(sorted)
    } else {
        Err(CompileError::CycleDetected)
    }
}

fn resolve_routing(
    graph: &Graph,
    module_indices: &BTreeMap<&str, usize>,
    nodes: &mut [CompiledNode],
) -> Result<(), CompileError> {
    for cable in graph.cables() {
        let source_module_id = cable.source().module_id().as_str();
        let destination_module_id = cable.destination().module_id().as_str();
        let source_module_index =
            module_index(module_indices, source_module_id, cable.source().port_name())?;
        let destination_module_index = module_index(
            module_indices,
            destination_module_id,
            cable.destination().port_name(),
        )?;
        let source_port_index = graph.modules()[source_module_index]
            .outputs()
            .iter()
            .position(|port| port.name() == cable.source().port_name())
            .ok_or_else(|| CompileError::MissingPort {
                module_id: source_module_id.to_string(),
                port_name: cable.source().port_name().to_string(),
            })?;
        let destination_port_index = graph.modules()[destination_module_index]
            .inputs()
            .iter()
            .position(|port| port.name() == cable.destination().port_name())
            .ok_or_else(|| CompileError::MissingPort {
                module_id: destination_module_id.to_string(),
                port_name: cable.destination().port_name().to_string(),
            })?;

        nodes[destination_module_index].input_port_map[destination_port_index].push(
            CompiledPortRef {
                module_index: source_module_index,
                port_index: source_port_index,
            },
        );
        nodes[destination_module_index].input_routes[destination_port_index].push(
            CompiledInputSource {
                module_index: source_module_index,
                port_index: source_port_index,
                output_buffer_id: nodes[source_module_index].output_port_map[source_port_index],
                output_port_name: nodes[source_module_index].output_port_names[source_port_index]
                    .clone(),
            },
        );
    }

    Ok(())
}

fn module_index(
    module_indices: &BTreeMap<&str, usize>,
    module_id: &str,
    port_name: &str,
) -> Result<usize, CompileError> {
    module_indices
        .get(module_id)
        .copied()
        .ok_or_else(|| CompileError::MissingPort {
            module_id: module_id.to_string(),
            port_name: port_name.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Cable, ModuleNode, PortRef, SignalType};

    fn render_settings() -> RenderSettings {
        RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 128,
            duration_frames: 1_024,
        }
    }

    fn audio_source(id: &str) -> ModuleNode {
        ModuleNode::new(ModuleId::new(id), "oscillator").with_output("audio", SignalType::Audio)
    }

    fn audio_processor(id: &str) -> ModuleNode {
        ModuleNode::new(ModuleId::new(id), "gain")
            .with_input("audio_in", SignalType::Audio)
            .with_output("audio_out", SignalType::Audio)
    }

    fn audio_sink(id: &str) -> ModuleNode {
        ModuleNode::new(ModuleId::new(id), "audio_output").with_input("left", SignalType::Audio)
    }

    fn connect(from_id: &str, from_port: &str, to_id: &str, to_port: &str) -> Cable {
        Cable::new(
            PortRef::new(ModuleId::new(from_id), from_port),
            PortRef::new(ModuleId::new(to_id), to_port),
        )
    }

    fn compile_graph(graph: &Graph) -> CompiledPatch {
        compile(graph, &render_settings()).expect("graph should compile")
    }

    #[test]
    fn nodes_are_compiled_in_dependency_order_for_linear_chain() {
        let graph = Graph::new(
            vec![audio_source("a"), audio_processor("b"), audio_sink("c")],
            vec![
                connect("a", "audio", "b", "audio_in"),
                connect("b", "audio_out", "c", "left"),
            ],
        );

        let compiled = compile_graph(&graph);

        assert_eq!(compiled.execution_order(), &[0, 1, 2]);
        assert_eq!(compiled.topological_order(), &[0, 1, 2]);
    }

    #[test]
    fn numeric_parameters_are_compiled_into_slots() {
        let graph = Graph::new(
            vec![audio_processor("gain").with_params(BTreeMap::from([(
                "gain".to_string(),
                "0.5".to_string(),
            )]))],
            vec![],
        );

        let mut compiled = compile_graph(&graph);

        assert_eq!(compiled.numeric_parameter_value("gain", "gain"), Some(0.5));
        assert!(compiled.set_numeric_parameter_by_target("gain", "gain", 0.25));
        assert_eq!(compiled.numeric_parameter_value("gain", "gain"), Some(0.25));
    }
}
