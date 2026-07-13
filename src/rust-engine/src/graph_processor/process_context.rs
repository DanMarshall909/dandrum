use super::audio_arena::AudioArena;
use super::render_plan::BufferId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProcessContextError {
    MissingInput { index: usize },
    MissingOutput { index: usize },
}

pub(super) type ProcessContextResult<T> = Result<T, ProcessContextError>;

pub(super) struct ProcessContext<'a> {
    arena: &'a mut AudioArena,
    input_buffers: &'a [BufferId],
    output_buffers: &'a [BufferId],
    frames: usize,
}

impl<'a> ProcessContext<'a> {
    pub(super) fn new(
        arena: &'a mut AudioArena,
        input_buffers: &'a [BufferId],
        output_buffers: &'a [BufferId],
        frames: usize,
    ) -> Self {
        Self {
            arena,
            input_buffers,
            output_buffers,
            frames,
        }
    }

    pub(super) fn frames(&self) -> usize {
        self.frames
    }

    pub(super) fn input_count(&self) -> usize {
        self.input_buffers.len()
    }

    pub(super) fn output_count(&self) -> usize {
        self.output_buffers.len()
    }

    pub(super) fn input_sample(&self, index: usize, frame: usize, default: f32) -> f32 {
        let Some(buffer) = self.input_buffers.get(index).copied() else {
            return default;
        };
        self.arena.sample(buffer, frame)
    }

    pub(super) fn set_output_sample(
        &mut self,
        index: usize,
        frame: usize,
        value: f32,
    ) -> ProcessContextResult<()> {
        let buffer = self
            .output_buffers
            .get(index)
            .copied()
            .ok_or(ProcessContextError::MissingOutput { index })?;
        self.arena.set_sample(buffer, frame, value);
        Ok(())
    }

    pub(super) fn write_output_from_input(
        &mut self,
        output_index: usize,
        input_index: usize,
        write_sample: impl FnMut(f32) -> f32,
    ) -> ProcessContextResult<()> {
        let input = self
            .input_buffers
            .get(input_index)
            .copied()
            .ok_or(ProcessContextError::MissingInput { index: input_index })?;
        let output = self.output_buffers.get(output_index).copied().ok_or(
            ProcessContextError::MissingOutput {
                index: output_index,
            },
        )?;
        self.arena
            .write_from_input(input, output, self.frames, write_sample);
        Ok(())
    }

    pub(super) fn write_output_from_two_inputs(
        &mut self,
        output_index: usize,
        first_input_index: usize,
        second_input_index: usize,
        write_sample: impl FnMut(f32, f32) -> f32,
    ) -> ProcessContextResult<()> {
        let first_input = self.input_buffers.get(first_input_index).copied().ok_or(
            ProcessContextError::MissingInput {
                index: first_input_index,
            },
        )?;
        let second_input = self.input_buffers.get(second_input_index).copied().ok_or(
            ProcessContextError::MissingInput {
                index: second_input_index,
            },
        )?;
        let output = self.output_buffers.get(output_index).copied().ok_or(
            ProcessContextError::MissingOutput {
                index: output_index,
            },
        )?;
        self.arena.write_from_two_inputs(
            first_input,
            second_input,
            output,
            self.frames,
            write_sample,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::render_plan::AudioBufferPlan;
    use super::*;

    #[test]
    fn process_context_reads_input_samples_and_writes_output_samples() {
        let mut arena = AudioArena::new(AudioBufferPlan {
            buffer_count: 2,
            max_block_frames: 4,
            max_voices: 1,
        });
        arena.fill(BufferId(0), 4, 0.25);
        let input_buffers = [BufferId(0)];
        let output_buffers = [BufferId(1)];
        let mut context = ProcessContext::new(&mut arena, &input_buffers, &output_buffers, 4);

        assert_eq!(context.frames(), 4);
        assert_eq!(context.input_sample(0, 2, 1.0), 0.25);
        assert_eq!(context.input_sample(1, 2, 1.0), 1.0);

        context.set_output_sample(0, 2, 0.75).unwrap();
        assert_eq!(context.input_sample(0, 2, 0.0), 0.25);
    }

    #[test]
    fn process_context_writes_output_from_input_buffers() {
        let mut arena = AudioArena::new(AudioBufferPlan {
            buffer_count: 3,
            max_block_frames: 4,
            max_voices: 1,
        });
        arena.fill(BufferId(0), 4, 0.25);
        arena.fill(BufferId(1), 4, 2.0);
        let input_buffers = [BufferId(0), BufferId(1)];
        let output_buffers = [BufferId(2)];
        let mut context = ProcessContext::new(&mut arena, &input_buffers, &output_buffers, 4);

        context
            .write_output_from_two_inputs(0, 0, 1, |audio, gain| audio * gain)
            .unwrap();

        assert_eq!(context.input_sample(0, 0, 0.0), 0.25);
        assert_eq!(context.input_sample(1, 0, 0.0), 2.0);
    }

    #[test]
    fn process_context_reports_missing_ports_without_panicking() {
        let mut arena = AudioArena::new(AudioBufferPlan {
            buffer_count: 1,
            max_block_frames: 4,
            max_voices: 1,
        });
        let input_buffers: [BufferId; 0] = [];
        let output_buffers: [BufferId; 0] = [];
        let mut context = ProcessContext::new(&mut arena, &input_buffers, &output_buffers, 4);

        assert_eq!(
            context.write_output_from_input(0, 0, |sample| sample),
            Err(ProcessContextError::MissingInput { index: 0 })
        );
        assert_eq!(
            context.set_output_sample(0, 0, 0.5),
            Err(ProcessContextError::MissingOutput { index: 0 })
        );
    }
}
