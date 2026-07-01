use std::collections::{BTreeMap, VecDeque};
use std::fmt;

use crate::builtins::module_kind::ModuleKind;
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledNode {
    pub id: ModuleId,
    pub module_type: String,
    pub module_kind: ModuleKind,
    pub execution_scope: ExecutionScope,
    pub input_port_map: Vec<Vec<CompiledPortRef>>,
    pub output_port_map: Vec<usize>,
    pub input_port_names: Vec<String>,
    pub input_port_types: Vec<SignalType>,
    pub output_port_names: Vec<String>,
    pub output_port_types: Vec<SignalType>,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompiledPortRef {
    pub module_index: usize,
    pub port_index: usize,
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

pub fn compile(
    graph: &Graph,
    render_settings: &RenderSettings,
) -> Result<CompiledPatch, CompileError> {
    let module_indices = module_indices_by_id(graph);
    let topological_order = topological_sort(graph, &module_indices)?;
    let mut next_output_buffer = 0;
    let mut module_output_buffer_layout = Vec::with_capacity(graph.modules().len());
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
            Ok(CompiledNode {
                id: module.id().clone(),
                module_type: module_type_str.to_string(),
                module_kind: kind,
                execution_scope: module.execution_scope(),
                input_port_map: vec![Vec::new(); input_count],
                output_port_map: (output_buffer_start..next_output_buffer).collect(),
                input_port_names: module
                    .inputs()
                    .iter()
                    .map(|p| p.name().to_string())
                    .collect(),
                input_port_types: module.inputs().iter().map(|p| p.signal_type()).collect(),
                output_port_names: module
                    .outputs()
                    .iter()
                    .map(|p| p.name().to_string())
                    .collect(),
                output_port_types: module.outputs().iter().map(|p| p.signal_type()).collect(),
                parameters: module.params().clone(),
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
    })
}

impl CompiledPatch {
    pub fn nodes(&self) -> &[CompiledNode] {
        &self.nodes
    }

    pub fn topological_order(&self) -> &[ExecutionStep] {
        &self.topological_order
    }

    /// Returns the execution order as scope-ordered metadata.
    ///
    /// Global-scoped indices appear before voice-scoped indices.
    /// This is NOT a render iteration order — use [`voice_node_indices`]
    /// and [`global_node_indices`] for rendering to ensure voice
    /// producers execute before global consumers.
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
    fn disconnected_modules_all_appear_exactly_once_in_execution_order() {
        let graph = Graph::new(
            vec![audio_source("x"), audio_source("y"), audio_sink("z")],
            vec![],
        );

        let compiled = compile_graph(&graph);
        let mut sorted_order = compiled.execution_order().to_vec();
        sorted_order.sort_unstable();

        assert_eq!(sorted_order, vec![0, 1, 2]);
        assert_eq!(compiled.execution_order().len(), 3);
        assert_eq!(compiled.topological_order().len(), 3);
    }

    #[test]
    fn graph_with_cycle_returns_cycle_detected() {
        let graph = Graph::new(
            vec![audio_processor("a"), audio_processor("b")],
            vec![
                connect("a", "audio_out", "b", "audio_in"),
                connect("b", "audio_out", "a", "audio_in"),
            ],
        );

        let error = compile(&graph, &render_settings()).expect_err("cycle must fail");

        assert_eq!(error, CompileError::CycleDetected);
    }

    #[test]
    fn unknown_source_port_returns_missing_port() {
        let graph = Graph::new(
            vec![audio_source("a"), audio_sink("b")],
            vec![connect("a", "missing", "b", "left")],
        );

        let error = compile(&graph, &render_settings()).expect_err("missing source must fail");

        assert_eq!(
            error,
            CompileError::MissingPort {
                module_id: "a".to_string(),
                port_name: "missing".to_string(),
            }
        );
    }

    #[test]
    fn unknown_destination_port_returns_missing_port() {
        let graph = Graph::new(
            vec![audio_source("a"), audio_sink("b")],
            vec![connect("a", "audio", "b", "missing")],
        );

        let error = compile(&graph, &render_settings()).expect_err("missing destination must fail");

        assert_eq!(
            error,
            CompileError::MissingPort {
                module_id: "b".to_string(),
                port_name: "missing".to_string(),
            }
        );
    }

    #[test]
    fn valid_ports_compile_with_correct_compiled_port_refs() {
        let graph = Graph::new(
            vec![audio_source("a"), audio_sink("b")],
            vec![connect("a", "audio", "b", "left")],
        );

        let compiled = compile_graph(&graph);

        assert_eq!(compiled.nodes()[1].input_port_map[0].len(), 1);
        assert_eq!(
            compiled.nodes()[1].input_port_map[0][0],
            CompiledPortRef {
                module_index: 0,
                port_index: 0,
            }
        );
    }

    #[test]
    fn voice_scoped_nodes_appear_only_in_voice_node_indices() {
        let graph = Graph::new(
            vec![
                audio_source("global"),
                audio_processor("voice").with_execution_scope(ExecutionScope::Voice),
            ],
            vec![],
        );

        let compiled = compile_graph(&graph);

        assert_eq!(compiled.voice_node_indices(), &[1]);
        assert!(!compiled.global_node_indices().contains(&1));
    }

    #[test]
    fn global_scoped_nodes_appear_only_in_global_node_indices() {
        let graph = Graph::new(
            vec![
                audio_source("global"),
                audio_processor("voice").with_execution_scope(ExecutionScope::Voice),
            ],
            vec![],
        );

        let compiled = compile_graph(&graph);

        assert_eq!(compiled.global_node_indices(), &[0]);
        assert!(!compiled.voice_node_indices().contains(&0));
    }

    #[test]
    fn mixed_voice_global_graph_separates_correctly_and_voice_nodes_are_at_the_end() {
        let graph = Graph::new(
            vec![
                audio_source("global_a"),
                audio_processor("voice_a").with_execution_scope(ExecutionScope::Voice),
                audio_sink("global_b"),
                audio_processor("voice_b").with_execution_scope(ExecutionScope::Voice),
            ],
            vec![connect("voice_a", "audio_out", "voice_b", "audio_in")],
        );

        let compiled = compile_graph(&graph);

        assert_eq!(compiled.global_node_indices(), &[0, 2]);
        assert_eq!(compiled.voice_node_indices(), &[1, 3]);
        assert_eq!(compiled.execution_order(), &[0, 2, 1, 3]);
        assert_eq!(compiled.topological_order(), &[0, 1, 2, 3]);
    }

    #[test]
    fn compiled_patch_preserves_all_module_id_values() {
        let graph = Graph::new(
            vec![
                audio_source("first"),
                audio_processor("second"),
                audio_sink("third"),
            ],
            vec![],
        );

        let compiled = compile_graph(&graph);
        let ids = compiled
            .nodes()
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ids, vec!["first", "second", "third"]);
    }

    #[test]
    fn render_settings_are_preserved_in_compiled_patch() {
        let graph = Graph::new(vec![audio_source("a")], vec![]);
        let settings = RenderSettings {
            sample_rate_hz: 44_100,
            block_size_frames: 64,
            duration_frames: 2_048,
        };

        let compiled = compile(&graph, &settings).expect("graph should compile");

        assert_eq!(compiled.render_settings(), &settings);
    }

    #[test]
    fn compiled_routing_uses_vec_based_collections() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("a"), "oscillator")
                    .with_output("left", SignalType::Audio)
                    .with_output("right", SignalType::Audio),
                ModuleNode::new(ModuleId::new("b"), "audio_output")
                    .with_input("left_in", SignalType::Audio)
                    .with_input("right_in", SignalType::Audio),
            ],
            vec![
                connect("a", "left", "b", "left_in"),
                connect("a", "right", "b", "right_in"),
            ],
        );

        let compiled = compile_graph(&graph);

        assert_eq!(compiled.nodes()[1].input_port_map.len(), 2);
        assert_eq!(compiled.nodes()[1].input_port_map[0][0].port_index, 0);
        assert_eq!(compiled.nodes()[1].input_port_map[1][0].port_index, 1);
        assert_eq!(compiled.nodes()[0].output_port_map, vec![0, 1]);
    }

    #[test]
    fn compiled_patch_records_midi_and_audio_output_indices() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("midi"), "midi_input"),
                audio_source("source"),
                ModuleNode::new(ModuleId::new("out"), "audio_output"),
            ],
            vec![],
        );

        let compiled = compile_graph(&graph);

        assert_eq!(compiled.midi_input_index(), Some(0));
        assert_eq!(compiled.audio_output_index(), Some(2));
    }

    #[test]
    fn compiled_patch_preserves_module_configuration_metadata() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("sampler"), "sampler")
                    .with_params(BTreeMap::from([("asset".to_string(), "hit".to_string())])),
            ],
            vec![],
        );

        let compiled = compile_graph(&graph);

        assert_eq!(compiled.nodes()[0].parameters["asset"], "hit");
    }

    #[test]
    fn compiled_patch_records_output_buffer_layout() {
        let graph = Graph::new(
            vec![
                audio_source("source"),
                ModuleNode::new(ModuleId::new("stereo"), "gain")
                    .with_output("left", SignalType::Audio)
                    .with_output("right", SignalType::Audio),
                audio_sink("sink"),
            ],
            vec![],
        );

        let compiled = compile_graph(&graph);

        assert_eq!(
            compiled.module_output_buffer_layout(),
            &[
                CompiledModuleBufferLayout {
                    output_buffer_start: 0,
                    output_buffer_count: 1,
                },
                CompiledModuleBufferLayout {
                    output_buffer_start: 1,
                    output_buffer_count: 2,
                },
                CompiledModuleBufferLayout {
                    output_buffer_start: 3,
                    output_buffer_count: 0,
                },
            ]
        );
        assert_eq!(compiled.nodes()[1].output_port_map, vec![1, 2]);
        assert_eq!(compiled.total_output_buffer_count(), 3);
    }

    #[test]
    fn unknown_module_type_fails_compilation() {
        let graph = Graph::new(
            vec![ModuleNode::new(ModuleId::new("x"), "nonexistent_module")],
            vec![],
        );

        let error = compile(&graph, &render_settings()).expect_err("unknown module type must fail");

        assert_eq!(
            error,
            CompileError::UnknownModuleType {
                module_type: "nonexistent_module".to_string(),
            }
        );
    }

    #[test]
    fn render_unsupported_module_type_fails_compilation() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("delay"), "block_delay")
                    .with_input("audio_in", SignalType::Audio)
                    .with_output("audio_out", SignalType::Audio),
            ],
            vec![],
        );

        let error = compile(&graph, &render_settings())
            .expect_err("known but render-unsupported module type must fail");

        assert_eq!(
            error,
            CompileError::UnsupportedModuleType {
                module_type: "block_delay".to_string(),
            }
        );
    }
}
