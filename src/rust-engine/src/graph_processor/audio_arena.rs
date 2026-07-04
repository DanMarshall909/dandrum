#![allow(dead_code)]

use super::render_plan::{AudioBufferPlan, BufferId, CompiledEdge};

pub(super) struct AudioArena {
    buffers: Box<[f32]>,
    frames: usize,
    buffer_count: usize,
}

impl AudioArena {
    pub(super) fn new(plan: AudioBufferPlan) -> Self {
        let frames = plan.max_block_frames.max(1);
        let buffer_count = plan.buffer_count;
        Self {
            buffers: vec![0.0; frames * buffer_count].into_boxed_slice(),
            frames,
            buffer_count,
        }
    }

    pub(super) fn buffer_count(&self) -> usize {
        self.buffer_count
    }

    pub(super) fn frames(&self) -> usize {
        self.frames
    }

    pub(super) fn capacity_samples(&self) -> usize {
        self.buffers.len()
    }

    pub(super) fn clear(&mut self, buffer: BufferId, frames: usize) {
        self.slice_mut(buffer, frames).fill(0.0);
    }

    pub(super) fn fill(&mut self, buffer: BufferId, frames: usize, value: f32) {
        self.slice_mut(buffer, frames).fill(value);
    }

    pub(super) fn add_edge(&mut self, edge: CompiledEdge, frames: usize) {
        let source = self.slice(edge.source, frames).to_vec();
        let destination = self.slice_mut(edge.destination, frames);

        for (dst, src) in destination.iter_mut().zip(source.iter()) {
            *dst += src * edge.gain;
        }
    }

    pub(super) fn slice(&self, buffer: BufferId, frames: usize) -> &[f32] {
        let range = self.range(buffer, frames);
        &self.buffers[range]
    }

    pub(super) fn slice_mut(&mut self, buffer: BufferId, frames: usize) -> &mut [f32] {
        let range = self.range(buffer, frames);
        &mut self.buffers[range]
    }

    fn range(&self, buffer: BufferId, frames: usize) -> std::ops::Range<usize> {
        assert!(
            buffer.0 < self.buffer_count,
            "buffer id {} exceeds prepared buffer count {}",
            buffer.0,
            self.buffer_count
        );
        assert!(
            frames <= self.frames,
            "requested {} frames exceeds prepared frame count {}",
            frames,
            self.frames
        );
        let start = buffer.0 * self.frames;
        start..start + frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::SignalType;

    #[test]
    fn prepared_audio_arena_allocates_expected_capacity() {
        let arena = AudioArena::new(AudioBufferPlan {
            buffer_count: 3,
            max_block_frames: 64,
            max_voices: 1,
        });

        assert_eq!(arena.buffer_count(), 3);
        assert_eq!(arena.frames(), 64);
        assert_eq!(arena.capacity_samples(), 192);
    }

    #[test]
    fn prepared_audio_arena_clears_and_fills_selected_buffer() {
        let mut arena = AudioArena::new(AudioBufferPlan {
            buffer_count: 2,
            max_block_frames: 4,
            max_voices: 1,
        });

        arena.fill(BufferId(0), 4, 0.5);
        arena.fill(BufferId(1), 4, 1.0);
        arena.clear(BufferId(0), 4);

        assert_eq!(arena.slice(BufferId(0), 4), &[0.0, 0.0, 0.0, 0.0]);
        assert_eq!(arena.slice(BufferId(1), 4), &[1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn prepared_audio_arena_sums_compiled_edge_into_destination() {
        let mut arena = AudioArena::new(AudioBufferPlan {
            buffer_count: 2,
            max_block_frames: 4,
            max_voices: 1,
        });
        arena.fill(BufferId(0), 4, 0.25);
        arena.fill(BufferId(1), 4, 0.5);

        arena.add_edge(
            CompiledEdge {
                source: BufferId(0),
                destination: BufferId(1),
                signal_type: SignalType::Audio,
                gain: 2.0,
            },
            4,
        );

        assert_eq!(arena.slice(BufferId(1), 4), &[1.0, 1.0, 1.0, 1.0]);
    }
}
