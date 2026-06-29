use crate::graph::Graph;
use crate::patch::{PatchDocument, RenderSettings};
use crate::script::ScriptEvent;

pub struct Engine;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderBlock {
    start_frame: u64,
    frame_count: u32,
    input_events: Vec<ScheduledInputEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimedInputEvent {
    frame: u64,
    event: ScriptEvent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledInputEvent {
    frame_offset: u32,
    event: ScriptEvent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockScheduler {
    duration_frames: u64,
    block_size_frames: u32,
    next_start_frame: u64,
    input_events: Vec<TimedInputEvent>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OfflineRenderResult {
    left: Vec<f32>,
    right: Vec<f32>,
}

impl Engine {
    pub fn new() -> Self {
        Self
    }

    pub fn package_name(&self) -> &'static str {
        "dandrum-engine-core"
    }

    pub fn is_frontend_independent(&self) -> bool {
        true
    }

    pub fn block_scheduler(&self, settings: &RenderSettings) -> BlockScheduler {
        BlockScheduler::new(settings.duration_frames, settings.block_size_frames)
    }

    pub fn render_offline(
        &self,
        patch: &PatchDocument,
        input_events: Vec<TimedInputEvent>,
    ) -> OfflineRenderResult {
        let graph = Graph::from_patch_declarations(patch);
        let (left, right) =
            crate::graph_processor::render_offline(&graph, &patch.render, input_events);
        OfflineRenderResult { left, right }
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderBlock {
    pub fn start_frame(&self) -> u64 {
        self.start_frame
    }

    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }

    pub fn input_events(&self) -> &[ScheduledInputEvent] {
        &self.input_events
    }
}

impl TimedInputEvent {
    pub fn new(frame: u64, event: ScriptEvent) -> Self {
        Self { frame, event }
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }

    pub fn event(&self) -> &ScriptEvent {
        &self.event
    }
}

impl ScheduledInputEvent {
    pub fn frame_offset(&self) -> u32 {
        self.frame_offset
    }

    pub fn event(&self) -> &ScriptEvent {
        &self.event
    }
}

impl OfflineRenderResult {
    pub fn left(&self) -> &[f32] {
        &self.left
    }

    pub fn right(&self) -> &[f32] {
        &self.right
    }
}

impl BlockScheduler {
    pub fn new(duration_frames: u64, block_size_frames: u32) -> Self {
        Self {
            duration_frames,
            block_size_frames,
            next_start_frame: 0,
            input_events: Vec::new(),
        }
    }

    pub fn with_input_events(mut self, mut input_events: Vec<TimedInputEvent>) -> Self {
        input_events.sort_by_key(TimedInputEvent::frame);
        self.input_events = input_events;
        self
    }
}

impl Iterator for BlockScheduler {
    type Item = RenderBlock;

    fn next(&mut self) -> Option<Self::Item> {
        if self.block_size_frames == 0 {
            return None;
        }

        if self.next_start_frame >= self.duration_frames {
            return None;
        }

        let remaining_frames = self.duration_frames - self.next_start_frame;
        let frame_count = remaining_frames.min(u64::from(self.block_size_frames)) as u32;
        let start_frame = self.next_start_frame;
        let end_frame = start_frame
            .checked_add(u64::from(frame_count))
            .unwrap_or(self.duration_frames);
        let input_events = self
            .input_events
            .iter()
            .filter(|input_event| input_event.frame >= start_frame && input_event.frame < end_frame)
            .map(|input_event| ScheduledInputEvent {
                frame_offset: (input_event.frame - start_frame) as u32,
                event: input_event.event.clone(),
            })
            .collect();
        let block = RenderBlock {
            start_frame,
            frame_count,
            input_events,
        };

        self.next_start_frame = end_frame;
        Some(block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch;

    #[test]
    fn block_scheduler_splits_render_duration_into_blocks() {
        let blocks: Vec<RenderBlock> = BlockScheduler::new(300, 128).collect();

        assert_eq!(
            blocks,
            vec![
                RenderBlock {
                    start_frame: 0,
                    frame_count: 128,
                    input_events: Vec::new(),
                },
                RenderBlock {
                    start_frame: 128,
                    frame_count: 128,
                    input_events: Vec::new(),
                },
                RenderBlock {
                    start_frame: 256,
                    frame_count: 44,
                    input_events: Vec::new(),
                },
            ]
        );
    }

    #[test]
    fn engine_creates_block_scheduler_from_render_settings() {
        let engine = Engine::new();
        let settings = RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: 64,
            duration_frames: 130,
        };

        let blocks: Vec<(u64, u32)> = engine
            .block_scheduler(&settings)
            .map(|block| (block.start_frame(), block.frame_count()))
            .collect();

        assert_eq!(blocks, vec![(0, 64), (64, 64), (128, 2)]);
    }

    #[test]
    fn engine_metadata_identifies_frontend_independent_core() {
        let engine = Engine::new();

        assert_eq!(engine.package_name(), "dandrum-engine-core");
        assert!(engine.is_frontend_independent());
    }

    #[test]
    fn timed_input_event_preserves_absolute_frame() {
        let event = TimedInputEvent::new(
            48_123,
            ScriptEvent::NoteOn {
                note: 64,
                velocity: 90,
            },
        );

        assert_eq!(event.frame(), 48_123);
        assert_eq!(
            event.event(),
            &ScriptEvent::NoteOn {
                note: 64,
                velocity: 90,
            }
        );
    }

    #[test]
    fn block_scheduler_terminates_after_exact_duration() {
        let mut scheduler = BlockScheduler::new(256, 128);

        assert_eq!(scheduler.next().map(|block| block.start_frame()), Some(0));
        assert_eq!(scheduler.next().map(|block| block.start_frame()), Some(128));
        assert_eq!(scheduler.next(), None);
    }

    #[test]
    fn block_scheduler_rejects_zero_sized_blocks() {
        let mut scheduler = BlockScheduler::new(256, 0);

        assert_eq!(scheduler.next(), None);
    }

    #[test]
    fn block_scheduler_sequences_input_events_by_block_with_relative_offsets() {
        let blocks: Vec<RenderBlock> = BlockScheduler::new(256, 128)
            .with_input_events(vec![
                TimedInputEvent::new(130, ScriptEvent::NoteOff { note: 60 }),
                TimedInputEvent::new(
                    0,
                    ScriptEvent::NoteOn {
                        note: 60,
                        velocity: 100,
                    },
                ),
            ])
            .collect();

        assert_eq!(
            blocks[0].input_events(),
            &[ScheduledInputEvent {
                frame_offset: 0,
                event: ScriptEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                },
            }]
        );
        assert_eq!(
            blocks[1].input_events(),
            &[ScheduledInputEvent {
                frame_offset: 2,
                event: ScriptEvent::NoteOff { note: 60 },
            }]
        );
    }

    #[test]
    fn block_scheduler_excludes_events_on_block_end_until_next_block() {
        let blocks: Vec<RenderBlock> = BlockScheduler::new(256, 128)
            .with_input_events(vec![TimedInputEvent::new(
                128,
                ScriptEvent::NoteOn {
                    note: 67,
                    velocity: 80,
                },
            )])
            .collect();

        assert!(blocks[0].input_events().is_empty());
        assert_eq!(
            blocks[1].input_events(),
            &[ScheduledInputEvent {
                frame_offset: 0,
                event: ScriptEvent::NoteOn {
                    note: 67,
                    velocity: 80,
                },
            }]
        );
    }

    #[test]
    fn offline_render_is_deterministic_for_same_patch_settings_and_events() {
        let patch = patch::load_patch_str(
            r#"
metadata:
  name: Deterministic Render
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 130
modules:
  - id: out
    type: audio_output
    inputs:
      - name: left
        signal_type: audio
      - name: right
        signal_type: audio
"#,
        )
        .expect("patch should parse");
        let engine = Engine::new();
        let events = vec![TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
        )];

        let first = engine.render_offline(&patch, events.clone());
        let second = engine.render_offline(&patch, events);

        assert_eq!(first, second);
        assert_eq!(first.left().len(), 130);
        assert_eq!(first.right().len(), 130);
    }
}
