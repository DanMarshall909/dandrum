//! Bounded mirror of the controller ledger. Only collected host successes retire it.
use dandrum_trigger_host::parameters::{AutomationEvent, ParameterController};
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const CAPACITY: usize = 64;
pub(super) struct Main {
    queue: Producer<AutomationEvent>,
    accepted: Arc<AtomicUsize>,
    in_flight: usize,
}
pub(super) struct Output {
    queue: Consumer<AutomationEvent>,
    accepted: Arc<AtomicUsize>,
}
pub(super) fn pair() -> (Main, Output) {
    let (producer, consumer) = RingBuffer::new(CAPACITY);
    let accepted = Arc::new(AtomicUsize::new(0));
    (
        Main {
            queue: producer,
            accepted: Arc::clone(&accepted),
            in_flight: 0,
        },
        Output {
            queue: consumer,
            accepted,
        },
    )
}
impl Main {
    pub(super) fn service(&mut self, controller: &mut ParameterController) {
        let accepted = self.accepted.swap(0, Ordering::AcqRel);
        self.in_flight = self
            .in_flight
            .checked_sub(accepted)
            .expect("accepted credits belong to staged ledger prefix");
        for _ in 0..accepted {
            assert!(
                controller.acknowledge_automation(),
                "every accepted credit owns one ledger entry"
            );
        }
        let mut staged = 0;
        for event in controller
            .pending_automation()
            .skip(self.in_flight)
            .take(CAPACITY - self.in_flight)
        {
            self.queue
                .push(event)
                .expect("in-flight budget reserves queue capacity");
            staged += 1;
        }
        self.in_flight += staged;
    }
    pub(super) fn pending(&self) -> bool {
        self.in_flight != 0
    }
}
impl Output {
    pub(super) fn deliver(&mut self, mut accept: impl FnMut(AutomationEvent) -> bool) -> usize {
        let mut accepted = 0;
        for _ in 0..CAPACITY {
            let Ok(event) = self.queue.peek().copied() else {
                break;
            };
            if !accept(event) {
                break;
            }
            self.queue
                .pop()
                .expect("accepted head remains owned until popped");
            self.accepted.fetch_add(1, Ordering::Release);
            accepted += 1;
        }
        accepted
    }
}
