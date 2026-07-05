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

    pub(super) fn input(&self, index: usize) -> ProcessContextResult<&[f32]> {
        let buffer = self
            .input_buffers
            .get(index)
            .copied()
            .ok_or(ProcessContextError::MissingInput { index })?;
        Ok(self.arena.slice(buffer, self.frames))
    }

    pub(super) fn output(&mut self, index: usize) -> ProcessContextResult<&mut [f32]> {
        let buffer = self
            .output_buffers
            .get(index)
            .copied()
            .ok_or(ProcessContextError::MissingOutput { index })?;
        Ok(self.arena.slice_mut(buffer, self.frames))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::render_plan::AudioBufferPlan;

    #[test]
    fn process_context_exposes_typed_audio_slices() {
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
        assert_eq!(context.input(0).unwrap(), &[0.25, 0.25, 0.25, 0.25]);

        context.output(0).unwrap().copy_from_slice(&[0.0, 0.5, 1.0, 0.5]);
        assert_eq!(context.output(0).unwrap(), &[0.0, 0.5, 1.0, 0.5]);
    }

    #[test]
    fn process_context_reports_missing_audio_ports_without_panicking() {
        let mut arena = AudioArena::new(AudioBufferPlan {
            buffer_count: 1,
            max_block_frames: 4,
            max_voices: 1,
        });
        let input_buffers: [BufferId; 0] = [];
        let output_buffers: [BufferId; 0] = [];
        let mut context = ProcessContext::new(&mut arena, &input_buffers, &output_buffers, 4);

        assert_eq!(
            context.input(0),
            Err(ProcessContextError::MissingInput { index: 0 })
        );
        assert_eq!(
            context.output(0),
            Err(ProcessContextError::MissingOutput { index: 0 })
        );
    }
}
