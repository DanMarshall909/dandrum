#![allow(dead_code)]

use crate::builtins::module_kind::ModuleKind;
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
