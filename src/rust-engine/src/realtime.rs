#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RealtimeEvent {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RealtimeEventSubmitStatus {
    Accepted,
    Dropped,
}

pub struct RealtimeEventQueue {
    events: Vec<RealtimeEvent>,
    capacity: usize,
    dropped_events: usize,
}

impl RealtimeEventQueue {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Vec::with_capacity(capacity),
            capacity,
            dropped_events: 0,
        }
    }

    pub fn submit(&mut self, event: RealtimeEvent) -> RealtimeEventSubmitStatus {
        if self.events.len() >= self.capacity {
            self.dropped_events += 1;
            return RealtimeEventSubmitStatus::Dropped;
        }

        self.events.push(event);
        RealtimeEventSubmitStatus::Accepted
    }

    pub fn drain(&mut self) -> Vec<RealtimeEvent> {
        self.events.drain(..).collect()
    }

    pub fn dropped_events(&self) -> usize {
        self.dropped_events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_event_queue_reports_dropped_status_when_full() {
        let mut queue = RealtimeEventQueue::with_capacity(2);

        assert_eq!(
            queue.submit(RealtimeEvent::NoteOn {
                note: 60,
                velocity: 100
            }),
            RealtimeEventSubmitStatus::Accepted
        );
        assert_eq!(
            queue.submit(RealtimeEvent::NoteOff { note: 60 }),
            RealtimeEventSubmitStatus::Accepted
        );
        assert_eq!(
            queue.submit(RealtimeEvent::NoteOn {
                note: 62,
                velocity: 90
            }),
            RealtimeEventSubmitStatus::Dropped
        );

        assert_eq!(queue.dropped_events(), 1);
        assert_eq!(
            queue.drain(),
            vec![
                RealtimeEvent::NoteOn {
                    note: 60,
                    velocity: 100
                },
                RealtimeEvent::NoteOff { note: 60 }
            ]
        );
        assert_eq!(queue.drain(), Vec::<RealtimeEvent>::new());
    }
}
