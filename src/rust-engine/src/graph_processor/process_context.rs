use super::audio_arena::AudioArena;
use super::outputs::BlockEvent;
use super::render_plan::BufferId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProcessContextError {
    MissingInput { index: usize },
    MissingOutput { index: usize },
    MissingEventInput { index: usize },
    MissingEventOutput { index: usize },
}

pub(super) type ProcessContextResult<T> = Result<T, ProcessContextError>;

pub(super) struct ProcessContext<'a> {
    arena: &'a mut AudioArena,
    input_buffers: &'a [BufferId],
    output_buffers: &'a [BufferId],
    input_events: &'a [&'a [BlockEvent]],
    output_events: &'a mut [Vec<BlockEvent>],
    frames: usize,
    block_start_frame: u64,
    sample_rate: f32,
}

impl<'a> ProcessContext<'a> {
    pub(super) fn new(
        arena: &'a mut AudioArena,
        input_buffers: &'a [BufferId],
        output_buffers: &'a [BufferId],
        input_events: &'a [&'a [BlockEvent]],
        output_events: &'a mut [Vec<BlockEvent>],
        frames: usize,
        block_start_frame: u64,
        sample_rate: f32,
    ) -> Self {
        Self {
            arena,
            input_buffers,
            output_buffers,
            input_events,
            output_events,
            frames,
            block_start_frame,
            sample_rate,
        }
    }

    pub(super) fn frames(&self) -> usize {
        self.frames
    }

    pub(super) fn block_start_frame(&self) -> u64 {
        self.block_start_frame
    }

    pub(super) fn sample_rate(&self) -> f32 {
        self.sample_rate
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

    pub(super) fn input_events(&self, index: usize) -> ProcessContextResult<&[BlockEvent]> {
        self.input_events
            .get(index)
            .copied()
            .ok_or(ProcessContextError::MissingEventInput { index })
    }

    pub(super) fn write_event(
        &mut self,
        index: usize,
        event: BlockEvent,
    ) -> ProcessContextResult<()> {
        let queue = self
            .output_events
            .get_mut(index)
            .ok_or(ProcessContextError::MissingEventOutput { index })?;
        queue.push(event);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::ScriptEvent;
    use super::super::render_plan::AudioBufferPlan;

    #[test]
    fn process_context_exposes_typed_audio_slices_and_render_metadata() {
        let mut arena = AudioArena::new(AudioBufferPlan {
            buffer_count: 2,
            max_block_frames: 4,
            max_voices: 1,
        });
        arena.fill(BufferId(0), 4, 0.25);
        let input_buffers = [BufferId(0)];
        let output_buffers = [BufferId(1)];
        let input_events: [&[BlockEvent]; 0] = [];
        let mut output_events = Vec::new();
        let mut context = ProcessContext::new(
            &mut arena,
            &input_buffers,
            &output_buffers,
            &input_events,
            output_events.as_mut_slice(),
            4,
            128,
            48_000.0,
        );

        assert_eq!(context.frames(), 4);
        assert_eq!(context.block_start_frame(), 128);
        assert_eq!(context.sample_rate(), 48_000.0);
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
        let input_events: [&[BlockEvent]; 0] = [];
        let mut output_events = Vec::new();
        let mut context = ProcessContext::new(
            &mut arena,
            &input_buffers,
            &output_buffers,
            &input_events,
            output_events.as_mut_slice(),
            4,
            0,
            44_100.0,
        );

        assert_eq!(
            context.input(0),
            Err(ProcessContextError::MissingInput { index: 0 })
        );
        assert_eq!(
            context.output(0),
            Err(ProcessContextError::MissingOutput { index: 0 })
        );
    }

    #[test]
    fn process_context_exposes_event_inputs_and_writes_event_outputs() {
        let mut arena = AudioArena::new(AudioBufferPlan {
            buffer_count: 1,
            max_block_frames: 4,
            max_voices: 1,
        });
        let input_buffers: [BufferId; 0] = [];
        let output_buffers: [BufferId; 0] = [];
        let source_event = BlockEvent {
            frame_offset: 2,
            event: ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
        };
        let event_input = [source_event.clone()];
        let input_events: [&[BlockEvent]; 1] = [&event_input];
        let mut output_events = vec![Vec::new()];
        let mut context = ProcessContext::new(
            &mut arena,
            &input_buffers,
            &output_buffers,
            &input_events,
            output_events.as_mut_slice(),
            4,
            0,
            44_100.0,
        );

        assert_eq!(context.input_events(0).unwrap(), &[source_event.clone()]);
        context.write_event(0, source_event.clone()).unwrap();
        assert_eq!(
            context.input_events(1),
            Err(ProcessContextError::MissingEventInput { index: 1 })
        );
        drop(context);

        assert_eq!(output_events[0], vec![source_event]);
    }
}
