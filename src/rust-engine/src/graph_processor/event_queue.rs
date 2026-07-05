use crate::script::ScriptEvent;

use super::outputs::BlockEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct EventQueueOverflow {
    pub(super) dropped_events: usize,
}

pub(super) type EventQueueResult<T> = Result<T, EventQueueOverflow>;

pub(super) struct BoundedEventQueue {
    events: Vec<ScriptEvent>,
    dropped_events: usize,
}

impl BoundedEventQueue {
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Vec::with_capacity(capacity),
            dropped_events: 0,
        }
    }

    pub(super) fn capacity(&self) -> usize {
        self.events.capacity()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub(super) fn dropped_events(&self) -> usize {
        self.dropped_events
    }

    pub(super) fn push(&mut self, event: ScriptEvent) -> EventQueueResult<()> {
        if self.events.len() == self.events.capacity() {
            self.dropped_events += 1;
            return Err(EventQueueOverflow {
                dropped_events: self.dropped_events,
            });
        }

        self.events.push(event);
        Ok(())
    }

    pub(super) fn drain_block_events(&mut self) -> Vec<BlockEvent> {
        self.events
            .drain(..)
            .map(|event| BlockEvent {
                frame_offset: 0,
                event,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_event_queue_reports_overflow_without_growing_capacity() {
        let mut queue = BoundedEventQueue::with_capacity(1);
        let event = ScriptEvent::NoteOn {
            note: 60,
            velocity: 100,
        };

        assert_eq!(queue.push(event.clone()), Ok(()));
        assert_eq!(
            queue.push(event),
            Err(EventQueueOverflow { dropped_events: 1 })
        );
        assert_eq!(queue.capacity(), 1);
        assert_eq!(queue.dropped_events(), 1);
    }

    #[test]
    fn bounded_event_queue_drains_script_events_as_block_events() {
        let mut queue = BoundedEventQueue::with_capacity(2);
        queue
            .push(ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            })
            .unwrap();
        queue.push(ScriptEvent::NoteOff { note: 60 }).unwrap();

        let events = queue.drain_block_events();

        assert!(queue.is_empty());
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].frame_offset, 0);
        assert_eq!(events[1].frame_offset, 0);
    }
}
