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
        assert_ne!(
            edge.source, edge.destination,
            "compiled render edge cannot sum a buffer into itself"
        );
        self.assert_valid(edge.source, frames);
        self.assert_valid(edge.destination, frames);

        let source_start = edge.source.0 * self.frames;
        let destination_start = edge.destination.0 * self.frames;

        if source_start < destination_start {
            let (before_destination, destination_and_after) = self.buffers.split_at_mut(destination_start);
            let source = &before_destination[source_start..source_start + frames];
            let destination = &mut destination_and_after[..frames];
            Self::add_slices(source, destination, edge.gain);
        } else {
            let (before_source, source_and_after) = self.buffers.split_at_mut(source_start);
            let destination = &mut before_source[destination_start..destination_start + frames];
            let source = &source_and_after[..frames];
            Self::add_slices(source, destination, edge.gain);
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

    pub(super) fn write_from_input(
        &mut self,
        input: BufferId,
        output: BufferId,
        frames: usize,
        mut write_sample: impl FnMut(f32) -> f32,
    ) {
        self.assert_valid(input, frames);
        self.assert_valid(output, frames);
        let input_start = input.0 * self.frames;
        let output_start = output.0 * self.frames;
        for frame in 0..frames {
            let input_sample = self.buffers[input_start + frame];
            self.buffers[output_start + frame] = write_sample(input_sample);
        }
    }

    pub(super) fn write_from_two_inputs(
        &mut self,
        first_input: BufferId,
        second_input: BufferId,
        output: BufferId,
        frames: usize,
        mut write_sample: impl FnMut(f32, f32) -> f32,
    ) {
        self.assert_valid(first_input, frames);
        self.assert_valid(second_input, frames);
        self.assert_valid(output, frames);
        let first_input_start = first_input.0 * self.frames;
        let second_input_start = second_input.0 * self.frames;
        let output_start = output.0 * self.frames;
        for frame in 0..frames {
            let first = self.buffers[first_input_start + frame];
            let second = self.buffers[second_input_start + frame];
            self.buffers[output_start + frame] = write_sample(first, second);
        }
    }

    pub(super) fn copy_to_slices(
        &self,
        left: BufferId,
        right: BufferId,
        frames: usize,
        left_out: &mut [f32],
        right_out: &mut [f32],
    ) {
        let actual = frames.min(left_out.len()).min(right_out.len());
        left_out[..actual].copy_from_slice(self.slice(left, actual));
        right_out[..actual].copy_from_slice(self.slice(right, actual));
        left_out[actual..frames].fill(0.0);
        right_out[actual..frames].fill(0.0);
    }

    fn add_slices(source: &[f32], destination: &mut [f32], gain: f32) {
        for (dst, src) in destination.iter_mut().zip(source.iter()) {
            *dst += src * gain;
        }
    }

    fn range(&self, buffer: BufferId, frames: usize) -> std::ops::Range<usize> {
        self.assert_valid(buffer, frames);
        let start = buffer.0 * self.frames;
        start..start + frames
    }

    fn assert_valid(&self, buffer: BufferId, frames: usize) {
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

    #[test]
    fn prepared_audio_arena_writes_output_from_input_buffers() {
        let mut arena = AudioArena::new(AudioBufferPlan {
            buffer_count: 3,
            max_block_frames: 4,
            max_voices: 1,
        });
        arena.fill(BufferId(0), 4, 0.25);
        arena.fill(BufferId(1), 4, 2.0);

        arena.write_from_two_inputs(BufferId(0), BufferId(1), BufferId(2), 4, |audio, gain| {
            audio * gain
        });

        assert_eq!(arena.slice(BufferId(2), 4), &[0.5, 0.5, 0.5, 0.5]);
    }
}
