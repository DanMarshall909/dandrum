use crate::script::ScriptEvent;

use super::outputs::BlockEvent;
use super::render_plan::{CompiledEventEdge, EventQueueId};

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

    pub(super) fn drain_into_buffer(&mut self, dest: &mut [BlockEvent]) -> usize {
        let count = self.events.len().min(dest.len());
        for (i, event) in self.events.drain(..count).enumerate() {
            dest[i] = BlockEvent {
                frame_offset: 0,
                event,
            };
        }
        let remaining = self.events.len();
        self.dropped_events += remaining;
        self.events.clear();
        count
    }
}

#[cfg(test)]
impl BoundedEventQueue {
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

impl Default for BoundedEventQueue {
    fn default() -> Self {
        Self::with_capacity(64)
    }
}

#[cfg(test)]
pub(super) struct EventWriter<'a> {
    queue: &'a mut BoundedEventQueue,
}

#[cfg(test)]
impl<'a> EventWriter<'a> {
    pub(super) fn new(queue: &'a mut BoundedEventQueue) -> Self {
        Self { queue }
    }

    pub(super) fn write(&mut self, event: ScriptEvent) -> EventQueueResult<()> {
        self.queue.push(event)
    }

    #[cfg(test)]
    pub(super) fn dropped_events(&self) -> usize {
        self.queue.dropped_events()
    }
}

pub(super) struct PreparedEventQueues {
    queues: Box<[BoundedEventQueue]>,
}

impl PreparedEventQueues {
    pub(super) fn new(queue_count: usize, queue_capacity: usize) -> Self {
        let mut queues = Vec::with_capacity(queue_count);
        for _ in 0..queue_count {
            queues.push(BoundedEventQueue::with_capacity(queue_capacity));
        }
        Self {
            queues: queues.into_boxed_slice(),
        }
    }

    #[cfg(test)]
    pub(super) fn queue(&mut self, id: usize) -> Option<&mut BoundedEventQueue> {
        self.queues.get_mut(id)
    }

    #[cfg(test)]
    pub(super) fn writer(&mut self, id: usize) -> Option<EventWriter<'_>> {
        self.queues.get_mut(id).map(EventWriter::new)
    }

    pub(super) fn route_event_edge(&mut self, edge: CompiledEventEdge) -> EventQueueResult<()> {
        if edge.source == edge.destination {
            return Ok(());
        }

        let (source, destination) = self.queue_pair(edge.source, edge.destination)?;
        for event in source.events.iter().cloned() {
            destination.push(event)?;
        }

        Ok(())
    }

    #[cfg(test)]
    pub(super) fn queue_count(&self) -> usize {
        self.queues.len()
    }

    #[cfg(test)]
    pub(super) fn capacity_per_queue(&self) -> usize {
        self.queues.first().map_or(0, |q| q.capacity())
    }

    #[cfg(test)]
    pub(super) fn revert(&mut self, events: std::collections::HashMap<usize, Vec<ScriptEvent>>) {
        for (id, events) in events {
            if let Some(queue) = self.queues.get_mut(id) {
                for event in events {
                    let _ = queue.push(event);
                }
            }
        }
    }

    fn queue_pair(
        &mut self,
        source_id: EventQueueId,
        destination_id: EventQueueId,
    ) -> EventQueueResult<(&BoundedEventQueue, &mut BoundedEventQueue)> {
        let source_index = source_id.0;
        let destination_index = destination_id.0;

        if source_index < destination_index {
            let (before_destination, from_destination) =
                self.queues.split_at_mut(destination_index);
            return Ok((&before_destination[source_index], &mut from_destination[0]));
        }

        let (before_source, from_source) = self.queues.split_at_mut(source_index);
        Ok((&from_source[0], &mut before_source[destination_index]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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

    #[test]
    fn drain_into_buffer_writes_up_to_buffer_capacity_and_reports_overflow() {
        let mut queue = BoundedEventQueue::with_capacity(4);
        queue
            .push(ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            })
            .unwrap();
        queue
            .push(ScriptEvent::NoteOn {
                note: 61,
                velocity: 90,
            })
            .unwrap();
        queue
            .push(ScriptEvent::NoteOn {
                note: 62,
                velocity: 80,
            })
            .unwrap();

        let default = BlockEvent {
            frame_offset: 0,
            event: ScriptEvent::NoteOn {
                note: 0,
                velocity: 0,
            },
        };
        let mut buf = vec![default; 2];
        let count = queue.drain_into_buffer(&mut buf);

        assert_eq!(count, 2);
        assert_eq!(
            buf[0].event,
            ScriptEvent::NoteOn {
                note: 60,
                velocity: 100
            }
        );
        assert_eq!(
            buf[1].event,
            ScriptEvent::NoteOn {
                note: 61,
                velocity: 90
            }
        );
        assert!(queue.is_empty());
        assert_eq!(queue.dropped_events(), 1);
    }

    #[test]
    fn drain_into_buffer_handles_empty_queue() {
        let mut queue = BoundedEventQueue::with_capacity(4);
        let default = BlockEvent {
            frame_offset: 0,
            event: ScriptEvent::NoteOn {
                note: 0,
                velocity: 0,
            },
        };
        let mut buf = vec![default; 2];
        let count = queue.drain_into_buffer(&mut buf);

        assert_eq!(count, 0);
        assert_eq!(queue.dropped_events(), 0);
    }

    #[test]
    fn event_writer_writes_to_underlying_queue_and_tracks_dropped() {
        let mut queue = BoundedEventQueue::with_capacity(2);
        let mut writer = EventWriter::new(&mut queue);

        assert_eq!(
            writer.write(ScriptEvent::NoteOn {
                note: 60,
                velocity: 100
            }),
            Ok(())
        );
        assert_eq!(writer.write(ScriptEvent::NoteOff { note: 60 }), Ok(()));
        assert_eq!(
            writer.write(ScriptEvent::NoteOn {
                note: 62,
                velocity: 90
            }),
            Err(EventQueueOverflow { dropped_events: 1 })
        );
        assert_eq!(writer.dropped_events(), 1);
    }

    #[test]
    fn prepared_event_queues_allocates_requested_queues() {
        let mut queues = PreparedEventQueues::new(3, 8);

        assert_eq!(queues.queue_count(), 3);
        assert_eq!(queues.capacity_per_queue(), 8);
        assert!(queues.queue(0).is_some());
        assert!(queues.queue(1).is_some());
        assert!(queues.queue(2).is_some());
        assert!(queues.queue(3).is_none());
    }

    #[test]
    fn prepared_event_queues_writer_routes_to_correct_queue() {
        let mut queues = PreparedEventQueues::new(2, 4);
        {
            let mut writer = queues.writer(1).expect("queue 1 should exist");
            writer
                .write(ScriptEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                })
                .unwrap();
        }
        assert!(!queues.queue(1).unwrap().is_empty());
        assert!(queues.queue(0).unwrap().is_empty());
    }

    #[test]
    fn prepared_event_queues_route_compiled_event_edge_by_queue_id() {
        let mut queues = PreparedEventQueues::new(2, 4);
        {
            let mut writer = queues.writer(0).unwrap();
            writer
                .write(ScriptEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                })
                .unwrap();
        }

        queues
            .route_event_edge(CompiledEventEdge {
                source: EventQueueId(0),
                destination: EventQueueId(1),
            })
            .unwrap();

        assert!(!queues.queue(0).unwrap().is_empty());
        assert!(!queues.queue(1).unwrap().is_empty());
    }

    #[test]
    fn prepared_event_queues_revert_restores_events() {
        let mut queues = PreparedEventQueues::new(2, 8);
        {
            let mut writer = queues.writer(0).unwrap();
            writer
                .write(ScriptEvent::NoteOn {
                    note: 60,
                    velocity: 100,
                })
                .unwrap();
        }

        let mut events = HashMap::new();
        events.insert(0, vec![ScriptEvent::NoteOff { note: 60 }]);
        queues.revert(events);

        let default = BlockEvent {
            frame_offset: 0,
            event: ScriptEvent::NoteOn {
                note: 0,
                velocity: 0,
            },
        };
        let mut buf = vec![default; 4];
        let count = queues.queue(0).unwrap().drain_into_buffer(&mut buf);
        assert_eq!(count, 2);
    }
}
