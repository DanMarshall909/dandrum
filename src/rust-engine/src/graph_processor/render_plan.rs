#![allow(dead_code)]

use crate::builtins::builtin_ports;
use crate::builtins::module_kind::ModuleKind;
use crate::compiled_patch::CompiledPatch;
use crate::graph::SignalType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct BufferId(pub(super) usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct EventQueueId(pub(super) usize);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CompiledEdge {
    pub(super) source: BufferId,
    pub(super) destination: BufferId,
    pub(super) signal_type: SignalType,
    pub(super) gain: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct RenderStep {
    pub(super) module_index: usize,
    pub(super) module_kind: ModuleKind,
    pub(super) input_buffers: Box<[BufferId]>,
    pub(super) output_buffers: Box<[BufferId]>,
    pub(super) incoming_edges: Box<[CompiledEdge]>,
    pub(super) event_inputs: Box<[EventQueueId]>,
    pub(super) event_outputs: Box<[EventQueueId]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AudioBufferPlan {
    pub(super) buffer_count: usize,
    pub(super) max_block_frames: usize,
    pub(super) max_voices: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct EventQueuePlan {
    pub(super) queue_count: usize,
    pub(super) queue_capacity: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AudioOutputBinding {
    pub(super) left: BufferId,
    pub(super) right: BufferId,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct RenderPlan {
    pub(super) voice_steps: Box<[RenderStep]>,
    pub(super) global_steps: Box<[RenderStep]>,
    pub(super) audio_buffers: AudioBufferPlan,
    pub(super) event_queues: EventQueuePlan,
    pub(super) midi_input: Option<EventQueueId>,
    pub(super) audio_output: Option<AudioOutputBinding>,
}

struct RenderPlanBuilder<'a> {
    compiled: &'a CompiledPatch,
    input_buffer_starts: Vec<usize>,
    input_buffer_count: usize,
    event_queue_starts: Vec<usize>,
    event_queue_count: usize,
}

impl RenderPlan {
    pub(super) fn empty(max_block_frames: usize, max_voices: usize) -> Self {
        Self {
            voice_steps: Box::new([]),
            global_steps: Box::new([]),
            audio_buffers: AudioBufferPlan {
                buffer_count: 0,
                max_block_frames,
                max_voices,
            },
            event_queues: EventQueuePlan {
                queue_count: 0,
                queue_capacity: 0,
            },
            midi_input: None,
            audio_output: None,
        }
    }

    pub(super) fn from_compiled_patch(
        compiled: &CompiledPatch,
        max_block_frames: usize,
        max_voices: usize,
        event_queue_capacity: usize,
    ) -> Self {
        let builder = RenderPlanBuilder::new(compiled);
        let voice_steps = compiled
            .voice_node_indices()
            .iter()
            .copied()
            .map(|module_index| builder.step(module_index))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let global_steps = compiled
            .global_node_indices()
            .iter()
            .copied()
            .map(|module_index| builder.step(module_index))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            voice_steps,
            global_steps,
            audio_buffers: AudioBufferPlan {
                buffer_count: compiled.total_output_buffer_count() + builder.input_buffer_count,
                max_block_frames,
                max_voices,
            },
            event_queues: EventQueuePlan {
                queue_count: builder.event_queue_count,
                queue_capacity: event_queue_capacity,
            },
            midi_input: compiled
                .midi_input_index()
                .and_then(|module_index| builder.first_event_output_queue(module_index)),
            audio_output: compiled
                .audio_output_index()
                .and_then(|module_index| builder.audio_output_binding(module_index)),
        }
    }
}

impl RenderPlanBuilder<'_> {
    fn new(compiled: &CompiledPatch) -> RenderPlanBuilder<'_> {
        let mut next_input_buffer = compiled.total_output_buffer_count();
        let mut input_buffer_starts = Vec::with_capacity(compiled.nodes().len());
        let mut next_event_queue = 0;
        let mut event_queue_starts = Vec::with_capacity(compiled.nodes().len());

        for node in compiled.nodes() {
            input_buffer_starts.push(next_input_buffer);
            next_input_buffer += node.input_port_types.len();

            event_queue_starts.push(next_event_queue);
            next_event_queue += node
                .input_port_types
                .iter()
                .chain(node.output_port_types.iter())
                .filter(|signal_type| **signal_type == SignalType::Event)
                .count();
        }

        RenderPlanBuilder {
            compiled,
            input_buffer_starts,
            input_buffer_count: next_input_buffer - compiled.total_output_buffer_count(),
            event_queue_starts,
            event_queue_count: next_event_queue,
        }
    }

    fn step(&self, module_index: usize) -> RenderStep {
        let node = &self.compiled.nodes()[module_index];
        let input_buffers = (0..node.input_port_types.len())
            .map(|port_index| self.input_buffer(module_index, port_index))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let output_buffers = node
            .output_port_map
            .iter()
            .copied()
            .map(BufferId)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let incoming_edges = self.incoming_edges(module_index).into_boxed_slice();
        let event_inputs = self.event_queues_for_ports(module_index, &node.input_port_types, 0);
        let event_outputs = self.event_queues_for_ports(
            module_index,
            &node.output_port_types,
            node.input_port_types.len(),
        );

        RenderStep {
            module_index,
            module_kind: node.module_kind,
            input_buffers,
            output_buffers,
            incoming_edges,
            event_inputs,
            event_outputs,
        }
    }

    fn incoming_edges(&self, module_index: usize) -> Vec<CompiledEdge> {
        let node = &self.compiled.nodes()[module_index];
        let mut edges = Vec::new();

        for (destination_port_index, sources) in node.input_port_map.iter().enumerate() {
            let signal_type = node.input_port_types[destination_port_index];
            if signal_type == SignalType::Event {
                continue;
            }

            for source in sources {
                let source_buffer = self.compiled.nodes()[source.module_index].output_port_map
                    [source.port_index];
                edges.push(CompiledEdge {
                    source: BufferId(source_buffer),
                    destination: self.input_buffer(module_index, destination_port_index),
                    signal_type,
                    gain: 1.0,
                });
            }
        }

        edges
    }

    fn input_buffer(&self, module_index: usize, port_index: usize) -> BufferId {
        BufferId(self.input_buffer_starts[module_index] + port_index)
    }

    fn first_event_output_queue(&self, module_index: usize) -> Option<EventQueueId> {
        let node = &self.compiled.nodes()[module_index];
        let output_offset = node.input_port_types.len();
        node.output_port_types
            .iter()
            .position(|signal_type| *signal_type == SignalType::Event)
            .and_then(|port_index| self.event_queue(module_index, output_offset + port_index))
    }

    fn audio_output_binding(&self, module_index: usize) -> Option<AudioOutputBinding> {
        let node = &self.compiled.nodes()[module_index];
        let left_index = node
            .input_port_names
            .iter()
            .position(|name| name == builtin_ports::LEFT)?;
        let right_index = node
            .input_port_names
            .iter()
            .position(|name| name == builtin_ports::RIGHT)?;

        Some(AudioOutputBinding {
            left: self.input_buffer(module_index, left_index),
            right: self.input_buffer(module_index, right_index),
        })
    }

    fn event_queues_for_ports(
        &self,
        module_index: usize,
        port_types: &[SignalType],
        local_offset: usize,
    ) -> Box<[EventQueueId]> {
        port_types
            .iter()
            .enumerate()
            .filter_map(|(port_index, signal_type)| {
                if *signal_type == SignalType::Event {
                    self.event_queue(module_index, local_offset + port_index)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    fn event_queue(&self, module_index: usize, combined_port_index: usize) -> Option<EventQueueId> {
        let node = &self.compiled.nodes()[module_index];
        let event_ordinal = node
            .input_port_types
            .iter()
            .chain(node.output_port_types.iter())
            .take(combined_port_index + 1)
            .filter(|signal_type| **signal_type == SignalType::Event)
            .count()
            .checked_sub(1)?;

        Some(EventQueueId(
            self.event_queue_starts[module_index] + event_ordinal,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Cable, ExecutionScope, Graph, ModuleId, ModuleNode, PortRef};
    use crate::patch::RenderSettings;

    #[test]
    fn empty_render_plan_records_prepared_size_without_runtime_storage() {
        let plan = RenderPlan::empty(128, 4);

        assert_eq!(plan.audio_buffers.max_block_frames, 128);
        assert_eq!(plan.audio_buffers.max_voices, 4);
        assert_eq!(plan.audio_buffers.buffer_count, 0);
        assert_eq!(plan.event_queues.queue_count, 0);
        assert_eq!(plan.voice_steps.len(), 0);
        assert_eq!(plan.global_steps.len(), 0);
    }

    #[test]
    fn render_plan_resolves_audio_edges_to_buffer_ids() {
        let graph = Graph::new(
            vec![
                ModuleNode::new(ModuleId::new("osc"), "oscillator")
                    .with_execution_scope(ExecutionScope::Voice)
                    .with_output(builtin_ports::AUDIO, SignalType::Audio),
                ModuleNode::new(ModuleId::new("out"), "audio_output")
                    .with_input(builtin_ports::LEFT, SignalType::Audio)
                    .with_input(builtin_ports::RIGHT, SignalType::Audio),
            ],
            vec![Cable::new(
                PortRef::new(ModuleId::new("osc"), builtin_ports::AUDIO),
                PortRef::new(ModuleId::new("out"), builtin_ports::LEFT),
            )],
        );
        let settings = RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 64,
            duration_frames: 64,
        };
        let compiled = crate::compiled_patch::compile(&graph, &settings).expect("graph should compile");
        let plan = RenderPlan::from_compiled_patch(&compiled, 64, 4, 32);

        assert_eq!(plan.voice_steps.len(), 1);
        assert_eq!(plan.global_steps.len(), 1);
        assert_eq!(plan.global_steps[0].incoming_edges.len(), 1);
        assert_eq!(plan.global_steps[0].incoming_edges[0].source, BufferId(0));
        assert_eq!(plan.audio_buffers.max_block_frames, 64);
        assert_eq!(plan.audio_buffers.max_voices, 4);
        assert!(plan.audio_output.is_some());
    }
}
