//! Bounded ownership transfer between the control thread and audio callback.
//!
//! Construct both endpoints and every replacement off audio. The audio endpoint
//! must also be returned to the control thread before it is dropped. `process`
//! never destroys a runtime, even if the control endpoint has already gone away.

use crate::TriggerHostConfiguration;
use dandrum_engine::PreparedSamplerAssets;
use dandrum_engine::graph::{PortDirection, SignalType};
use dandrum_engine::graph_processor::RealtimeGraphProcessor;
use dandrum_engine::kernel::{ChannelCount, PortMetadata};
use dandrum_engine::preparation::PreparedKernelInstrument;
use dandrum_instrument_session::{ActivationHost, ActivationLease, Publication, SessionSnapshot};
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

pub const MAX_BLOCK_FRAMES: u32 = 16_384;
pub const MAX_EVENTS: usize = 1_024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeError {
    InvalidConfiguration,
    UnsupportedLayout,
    InvalidBlock,
    InvalidEvent,
    QueueFull,
    Busy,
    IncompatibleRuntime,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ControlEvent {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
    Parameter { index: usize, value: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParameterFeedback {
    pub index: usize,
    pub value: f64,
    pub frame_offset: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimedEvent {
    pub frame_offset: u32,
    pub event: ControlEvent,
}

/// Fully constructed processor and scratch storage; moving it does not allocate.
pub struct PreparedRuntime(Box<Processor>);

struct Processor {
    graph: RealtimeGraphProcessor,
    configuration: TriggerHostConfiguration,
    layout: Vec<PortMetadata>,
    inputs: Vec<Vec<Vec<f32>>>,
    outputs: Vec<Vec<Vec<f32>>>,
}

impl PreparedRuntime {
    pub fn new(
        prepared: &PreparedKernelInstrument,
        configuration: TriggerHostConfiguration,
    ) -> Result<Self, RuntimeError> {
        if configuration.sample_rate_hz == 0
            || !(1..=MAX_BLOCK_FRAMES).contains(&configuration.max_block_frames)
            || prepared.compiled_patch().render_settings().sample_rate_hz
                != configuration.sample_rate_hz
            || prepared
                .compiled_patch()
                .render_settings()
                .block_size_frames
                != configuration.max_block_frames
        {
            return Err(RuntimeError::InvalidConfiguration);
        }
        let layout = prepared.root_port_metadata();
        let outputs = layout
            .iter()
            .filter(|port| port.direction() == PortDirection::Output)
            .collect::<Vec<_>>();
        if outputs.len() != 1
            || outputs[0].signal_type() != SignalType::Audio
            || outputs[0].channels() != &ChannelCount::Literal(2)
            || layout.iter().any(|port| {
                port.direction() == PortDirection::Input
                    && (port.signal_type() != SignalType::Control
                        || port.channels() != &ChannelCount::Literal(1))
            })
            || prepared
                .compiled_patch()
                .root_bus_plan()
                .inputs()
                .iter()
                .any(|port| !port.is_bound())
        {
            return Err(RuntimeError::UnsupportedLayout);
        }
        let frames = configuration.max_block_frames as usize;
        let inputs = layout
            .iter()
            .filter(|port| port.direction() == PortDirection::Input)
            .map(|port| {
                vec![vec![
                    port.control_default().map_or(0.0, |value| value.default())
                        as f32;
                    frames
                ]]
            })
            .collect();
        let mut processor = Processor {
            graph: RealtimeGraphProcessor::from_compiled_patch(
                prepared.compiled_patch().clone(),
                configuration.sample_rate_hz as f32,
                &PreparedSamplerAssets::empty(),
                frames,
            ),
            configuration,
            layout,
            inputs,
            outputs: vec![vec![vec![0.0; frames]; 2]],
        };
        // The engine checks the immutable root schedule before rendering any
        // voices; preparation already rejects unsupported poly child schedules.
        // Probe off audio, then restore fresh DSP state before publication.
        if processor
            .graph
            .render_root_buses(&processor.inputs, &mut processor.outputs)
            != frames
        {
            return Err(RuntimeError::UnsupportedLayout);
        }
        processor.graph.reset();
        Ok(Self(Box::new(processor)))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProcessReport {
    pub swapped: bool,
    pub admission_closed: bool,
    pub parameter_updates: usize,
    pub dropped_feedback: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResetReport {
    /// True means the structural publication lease prevented any reset work.
    pub admission_closed: bool,
    pub discarded_events: usize,
    pub cancelled_parameter_commands: usize,
}

#[derive(Clone, Copy)]
struct QueuedEvent {
    event: ControlEvent,
    command_sequence: u64,
}

#[derive(Clone, Copy)]
struct QueuedFeedback {
    parameter: ParameterFeedback,
    settlement: u64,
}

pub struct RuntimeControl {
    pending: Producer<PreparedRuntime>,
    retired: Consumer<PreparedRuntime>,
    events: Producer<QueuedEvent>,
    feedback: Consumer<QueuedFeedback>,
    values: Arc<[AtomicU64]>,
    settlement: Arc<AtomicU64>,
    admission: Arc<AtomicU64>,
    in_flight: bool,
    configuration: TriggerHostConfiguration,
    layout: Vec<PortMetadata>,
}

pub struct RuntimeAudio {
    state: AudioState,
    admission: Arc<AtomicU64>,
}

struct AudioState {
    current: PreparedRuntime,
    pending: Consumer<PreparedRuntime>,
    retired: Producer<PreparedRuntime>,
    events: Consumer<QueuedEvent>,
    feedback: Producer<QueuedFeedback>,
    values: Arc<[AtomicU64]>,
    settlement: Arc<AtomicU64>,
    event_capacity: usize,
}

/// Reserving this lease does not publish a processor. Dropping it cancels the
/// candidate on the control thread without altering accepted or audio state.
pub struct RuntimeLease<'a> {
    _admission: ClosedAdmission<'a>,
    slot: rtrb::chunks::WriteChunkUninit<'a, PreparedRuntime>,
    in_flight: &'a mut bool,
    candidate: PreparedRuntime,
}

impl ActivationLease for RuntimeLease<'_> {
    fn publish(self, _: Publication<'_>) {
        *self.in_flight = true;
        self.slot.fill_from_iter(std::iter::once(self.candidate));
    }
}

impl ActivationHost for RuntimeControl {
    type Error = RuntimeError;
    type Lease<'a> = RuntimeLease<'a>;

    fn activate<'a>(
        &'a mut self,
        prepared: &PreparedKernelInstrument,
        _: &SessionSnapshot,
    ) -> Result<Self::Lease<'a>, Self::Error> {
        if self.in_flight {
            return Err(RuntimeError::Busy);
        }
        let candidate = PreparedRuntime::new(prepared, self.configuration)?;
        if !self.compatible(&candidate) {
            return Err(RuntimeError::IncompatibleRuntime);
        }
        let slot = self
            .pending
            .write_chunk_uninit(1)
            .expect("idle activation slot");
        let admission = ClosedAdmission::close(&self.admission);
        Ok(RuntimeLease {
            _admission: admission,
            slot,
            in_flight: &mut self.in_flight,
            candidate,
        })
    }
}

/// Allocate the two capacity-one runtime slots off audio. `event_capacity`
/// bounds each control/feedback queue and the host event count in one block.
pub fn runtime_handoff(
    initial: PreparedRuntime,
    event_capacity: usize,
) -> Result<(RuntimeControl, RuntimeAudio), RuntimeError> {
    if event_capacity == 0
        || event_capacity > MAX_EVENTS
        || event_capacity > initial.0.graph.pending_event_capacity() / 2
    {
        return Err(RuntimeError::InvalidConfiguration);
    }
    let admission = Arc::new(AtomicU64::new(0));
    let settlement = Arc::new(AtomicU64::new(0));
    let (event_tx, event_rx) = RingBuffer::new(event_capacity);
    let (feedback_tx, feedback_rx) = RingBuffer::new(event_capacity);
    let values: Arc<[AtomicU64]> = initial
        .0
        .layout
        .iter()
        .filter(|port| port.direction() == PortDirection::Input)
        .map(|port| {
            AtomicU64::new(
                port.control_default()
                    .map_or(0.0, |value| value.default())
                    .to_bits(),
            )
        })
        .collect();
    let (pending_tx, pending_rx) = RingBuffer::new(1);
    let (retired_tx, retired_rx) = RingBuffer::new(1);
    let control = RuntimeControl {
        pending: pending_tx,
        retired: retired_rx,
        events: event_tx,
        feedback: feedback_rx,
        values: values.clone(),
        settlement: settlement.clone(),
        admission: admission.clone(),
        in_flight: false,
        configuration: initial.0.configuration,
        layout: initial.0.layout.clone(),
    };
    let audio = RuntimeAudio {
        state: AudioState {
            current: initial,
            pending: pending_rx,
            retired: retired_tx,
            events: event_rx,
            feedback: feedback_tx,
            values,
            settlement,
            event_capacity,
        },
        admission,
    };
    Ok((control, audio))
}

impl RuntimeControl {
    /// Parameter indices are the prepared root control-input order. Format
    /// adapters map their public parameter identities to these stable names.
    pub fn parameter_index(&self, root_name: &str) -> Option<usize> {
        self.layout
            .iter()
            .filter(|port| port.direction() == PortDirection::Input)
            .position(|port| port.name() == root_name)
    }
    pub fn parameter_value(&self, index: usize) -> Option<f64> {
        self.values
            .get(index)
            .map(|value| f64::from_bits(value.load(Ordering::Relaxed)))
    }
    pub fn take_feedback(&mut self) -> Option<ParameterFeedback> {
        self.feedback.pop().ok().map(|feedback| feedback.parameter)
    }
    pub(crate) fn take_sequenced_feedback(&mut self) -> Option<(ParameterFeedback, u64, bool)> {
        self.feedback.pop().ok().map(|feedback| {
            (
                feedback.parameter,
                feedback.settlement >> 1,
                feedback.settlement & 1 != 0,
            )
        })
    }
    pub fn try_send(&mut self, event: ControlEvent) -> Result<(), RuntimeError> {
        self.send_sequenced(event, 0)
    }

    pub(crate) fn event_slots(&mut self) -> usize {
        self.events.slots()
    }

    pub(crate) fn send_parameter(
        &mut self,
        index: usize,
        value: f64,
        command_sequence: u64,
    ) -> Result<(), RuntimeError> {
        if command_sequence == 0 || command_sequence > u64::MAX / 2 {
            return Err(RuntimeError::InvalidEvent);
        }
        self.send_sequenced(ControlEvent::Parameter { index, value }, command_sequence)
    }

    pub(crate) fn snapshot_parameters(&self) -> (u64, bool, Vec<f64>) {
        // Read the packed settlement first. Acquire observes preceding scalar
        // stores or reset cancellation. A racing newer value can only carry an
        // older acknowledgement; sequence/cause cannot tear. This read never
        // closes callback admission or makes the audio thread wait.
        let settlement = self.settlement.load(Ordering::Acquire);
        let values = self
            .values
            .iter()
            .map(|value| f64::from_bits(value.load(Ordering::Relaxed)))
            .collect();
        (settlement >> 1, settlement & 1 != 0, values)
    }

    fn send_sequenced(
        &mut self,
        event: ControlEvent,
        command_sequence: u64,
    ) -> Result<(), RuntimeError> {
        if !event.valid(&self.layout) {
            return Err(RuntimeError::InvalidEvent);
        }
        self.events
            .push(QueuedEvent {
                event,
                command_sequence,
            })
            .map_err(|_| RuntimeError::QueueFull)
    }
    fn compatible(&self, next: &PreparedRuntime) -> bool {
        next.0.configuration == self.configuration && next.0.layout == self.layout
    }

    /// A rejected value remains owned by the caller, including under backpressure.
    pub fn offer(&mut self, next: PreparedRuntime) -> Result<(), PreparedRuntime> {
        if self.in_flight || !self.compatible(&next) {
            return Err(next);
        }
        // The control-owned in-flight flag is cleared only after retirement;
        // therefore this slot cannot still hold a previous candidate.
        self.pending
            .write_chunk_uninit(1)
            .expect("idle activation slot")
            .fill_from_iter(std::iter::once(next));
        self.in_flight = true;
        Ok(())
    }

    /// Reclamation and destruction belong to the control thread. Until this
    /// succeeds a second replacement is refused, even after the first swap.
    pub fn reclaim(&mut self) -> Option<PreparedRuntime> {
        let retired = self.retired.pop().ok()?;
        self.in_flight = false;
        Some(retired)
    }
}

impl ControlEvent {
    fn valid(self, layout: &[PortMetadata]) -> bool {
        match self {
            Self::NoteOn { note, velocity } => note <= 127 && (1..=127).contains(&velocity),
            Self::NoteOff { note } => note <= 127,
            Self::Parameter { index, value } => {
                let port = layout
                    .iter()
                    .filter(|port| port.direction() == PortDirection::Input)
                    .nth(index);
                port.is_some_and(|port| {
                    value.is_finite()
                        && (value as f32).is_finite()
                        && port.control_default().is_none_or(|range| {
                            range.min().is_none_or(|min| value >= min)
                                && range.max().is_none_or(|max| value <= max)
                        })
                })
            }
        }
    }
}

const ADMISSION_CLOSED: u64 = 1;
const CALLBACK_ACTIVE: u64 = 2;

struct CallbackAdmission<'a>(&'a AtomicU64);

impl<'a> CallbackAdmission<'a> {
    fn enter(gate: &'a AtomicU64) -> Option<Self> {
        gate.compare_exchange(0, CALLBACK_ACTIVE, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| Self(gate))
    }
}

impl Drop for CallbackAdmission<'_> {
    fn drop(&mut self) {
        self.0.fetch_and(!CALLBACK_ACTIVE, Ordering::Release);
    }
}

struct ClosedAdmission<'a>(&'a AtomicU64);

impl<'a> ClosedAdmission<'a> {
    fn close(gate: &'a AtomicU64) -> Self {
        gate.fetch_or(ADMISSION_CLOSED, Ordering::AcqRel);
        // Waiting is confined to the control thread. Once closed, no new
        // callback can enter. Acquire observes the previous callback's release.
        while gate.load(Ordering::Acquire) & CALLBACK_ACTIVE != 0 {
            std::thread::yield_now();
        }
        Self(gate)
    }
}

impl Drop for ClosedAdmission<'_> {
    fn drop(&mut self) {
        self.0.store(0, Ordering::Release);
    }
}

impl RuntimeAudio {
    pub fn reset(&mut self) -> ResetReport {
        let Some(_admission) = CallbackAdmission::enter(&self.admission) else {
            return ResetReport {
                admission_closed: true,
                ..ResetReport::default()
            };
        };
        self.state.reset()
    }

    pub fn process(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
    ) -> Result<ProcessReport, RuntimeError> {
        self.process_events(left, right, &[])
    }

    pub fn process_events(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        events: &[TimedEvent],
    ) -> Result<ProcessReport, RuntimeError> {
        let Some(_admission) = CallbackAdmission::enter(&self.admission) else {
            left.fill(0.0);
            right.fill(0.0);
            return Ok(ProcessReport {
                admission_closed: true,
                ..ProcessReport::default()
            });
        };
        self.state.process_events(left, right, events)
    }
}

impl AudioState {
    /// Panic/reset discards pre-reset queued activity and resets voices/DSP,
    /// while retaining host-owned scalar values and admitted replacements.
    pub fn reset(&mut self) -> ResetReport {
        let mut report = ResetReport::default();
        let mut cancelled = 0;
        for _ in 0..self.event_capacity {
            let Ok(event) = self.events.pop() else {
                break;
            };
            report.discarded_events += 1;
            if event.command_sequence != 0 {
                report.cancelled_parameter_commands += 1;
                cancelled = event.command_sequence;
            }
        }
        self.current.0.graph.reset();
        if cancelled != 0 {
            // The low bit marks that the latest UI settlement was cancelled by
            // reset. A single Release store pairs cause/sequence atomically;
            // scalar values remain the values actually applied before reset.
            self.settlement
                .store((cancelled << 1) | 1, Ordering::Release);
        }
        report
    }
    /// Control-thread events are applied at the next block boundary; host events
    /// carry ordered offsets in this block. A rejected batch changes no state.
    pub fn process_events(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        events: &[TimedEvent],
    ) -> Result<ProcessReport, RuntimeError> {
        let error = if left.len() != right.len()
            || left.is_empty()
            || left.len() > self.current.0.configuration.max_block_frames as usize
        {
            Some(RuntimeError::InvalidBlock)
        } else if events.len() > self.event_capacity
            || events.iter().any(|event| {
                event.frame_offset as usize >= left.len()
                    || !event.event.valid(&self.current.0.layout)
            })
            || events
                .windows(2)
                .any(|pair| pair[0].frame_offset > pair[1].frame_offset)
        {
            Some(RuntimeError::InvalidEvent)
        } else {
            None
        };
        if let Some(error) = error {
            left.fill(0.0);
            right.fill(0.0);
            return Err(error);
        }
        let mut report = ProcessReport::default();
        // Reserve retirement storage before consuming a candidate. The safe
        // chunk API transfers ownership infallibly once a slot is available.
        if let Ok(slot) = self.retired.write_chunk_uninit(1)
            && let Ok(next) = self.pending.pop()
        {
            let previous = std::mem::replace(&mut self.current, next);
            slot.fill_from_iter(std::iter::once(previous));
            report.swapped = true;
        }
        // A producer may refill concurrently. The finite iteration bound is the
        // configured queue capacity, never "until the queue becomes empty".
        for _ in 0..self.event_capacity {
            let Ok(event) = self.events.pop() else {
                break;
            };
            self.apply_event(event.event, 0, event.command_sequence, &mut report);
        }
        let mut start = 0;
        for event in events {
            let end = event.frame_offset as usize;
            if end > start {
                self.current
                    .0
                    .render(&self.values, &mut left[start..end], &mut right[start..end]);
            }
            self.apply_event(event.event, event.frame_offset, 0, &mut report);
            start = end;
        }
        self.current
            .0
            .render(&self.values, &mut left[start..], &mut right[start..]);
        Ok(report)
    }

    fn apply_event(
        &mut self,
        event: ControlEvent,
        frame_offset: u32,
        command_sequence: u64,
        report: &mut ProcessReport,
    ) {
        match event {
            ControlEvent::NoteOn { note, velocity } => {
                self.current.0.graph.note_on_at(note, velocity, 0)
            }
            ControlEvent::NoteOff { note } => self.current.0.graph.note_off_at(note, 0),
            ControlEvent::Parameter { index, value } => {
                self.values[index].store(value.to_bits(), Ordering::Relaxed);
                if command_sequence != 0 {
                    self.settlement
                        .store(command_sequence << 1, Ordering::Release);
                }
                report.parameter_updates += 1;
                if self
                    .feedback
                    .push(QueuedFeedback {
                        parameter: ParameterFeedback {
                            index,
                            value,
                            frame_offset,
                        },
                        settlement: self.settlement.load(Ordering::Relaxed),
                    })
                    .is_err()
                {
                    // Scalar values remain available atomically even when the
                    // bounded event queue is full. The caller requests a main
                    // callback and reports/reconciles this explicit overflow.
                    report.dropped_feedback += 1;
                }
            }
        }
    }
}

impl Processor {
    fn render(&mut self, values: &[AtomicU64], left: &mut [f32], right: &mut [f32]) {
        for (plane, value) in self.inputs.iter_mut().flatten().zip(values) {
            let value = f64::from_bits(value.load(Ordering::Relaxed)) as f32;
            plane.resize(left.len(), value);
            plane.fill(value);
        }
        for plane in self.outputs.iter_mut().flatten() {
            plane.resize(left.len(), 0.0);
        }
        // Construction proves schedule support, and the callback validates
        // 1..=max frames in owned stereo planes. Neither the immutable schedule
        // nor its support flag can change: the engine returns exactly this size.
        let rendered = self
            .graph
            .render_root_buses(&self.inputs, &mut self.outputs);
        debug_assert_eq!(rendered, left.len(), "validated realtime render invariant");
        left.copy_from_slice(&self.outputs[0][0]);
        right.copy_from_slice(&self.outputs[0][1]);
    }
}

#[cfg(test)]
mod admission_tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    #[test]
    fn parameter_snapshot_does_not_close_or_wait_for_callback_admission() {
        let yaml = "ports:\n  - { name: level, direction: input, signal: control, channels: 1, default: 1, min: 0, max: 4, maps_to: amp.gain }\n  - { name: master, direction: output, signal: audio, channels: 2, maps_from: amp.audio_out }\nmodules:\n  - { id: amp, type: gain, static: { channels: 2 } }\nconnections: []\n";
        let session = dandrum_instrument_session::InstrumentSession::new_for_planar_host(
            yaml,
            dandrum_engine::preparation::PreparationContext::new(".", 48_000),
            dandrum_engine::patch::RenderSettings {
                sample_rate_hz: 48_000,
                block_size_frames: 16,
                duration_frames: 16,
            },
            dandrum_engine::preparation::HostBuses::new()
                .with_input("level", 1)
                .with_output("master", 2),
        )
        .unwrap();
        let prepared = PreparedRuntime::new(
            session.prepared(),
            TriggerHostConfiguration {
                sample_rate_hz: 48_000,
                max_block_frames: 16,
            },
        )
        .unwrap();
        let (mut control, mut audio) = runtime_handoff(prepared, 4).unwrap();
        assert_eq!(
            control.send_parameter(0, 0.5, 0),
            Err(RuntimeError::InvalidEvent)
        );
        assert_eq!(
            control.send_parameter(0, 0.5, u64::MAX),
            Err(RuntimeError::InvalidEvent)
        );
        control.send_parameter(0, 0.5, 1).unwrap();
        audio.process(&mut [0.0; 16], &mut [0.0; 16]).unwrap();
        let gate = audio.admission.clone();
        let callback = CallbackAdmission::enter(&gate).unwrap();
        let (tx, rx) = mpsc::channel();
        let reader = std::thread::spawn(move || tx.send(control.snapshot_parameters()).unwrap());
        let snapshot = rx.recv_timeout(Duration::from_secs(5));
        let gate_during_read = gate.load(Ordering::Acquire);
        drop(callback);
        reader.join().unwrap();
        assert_eq!(snapshot.unwrap(), (1, false, vec![0.5]));
        assert_eq!(
            gate_during_read, CALLBACK_ACTIVE,
            "ordinary reads cannot silence a racing callback"
        );
        let closed = ClosedAdmission::close(&gate);
        assert!(
            audio.reset().admission_closed,
            "denied reset must report that no work ran"
        );
        drop(closed);
    }

    #[test]
    fn publication_waits_for_admitted_callback_then_cancel_reopens_admission() {
        let gate = Arc::new(AtomicU64::new(0));
        let callback = CallbackAdmission::enter(&gate).expect("callback admitted");
        let (closed_tx, closed_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let worker_gate = gate.clone();
        let worker = std::thread::spawn(move || {
            let closed = ClosedAdmission::close(&worker_gate);
            closed_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            drop(closed);
        });
        let deadline = Instant::now() + Duration::from_secs(5);
        while gate.load(Ordering::Acquire) & ADMISSION_CLOSED == 0 {
            assert!(
                Instant::now() < deadline,
                "control thread must close admission"
            );
            std::thread::yield_now();
        }
        assert!(
            matches!(closed_rx.try_recv(), Err(mpsc::TryRecvError::Empty)),
            "publication cannot proceed while the callback owns its guard"
        );
        drop(callback);
        closed_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(CallbackAdmission::enter(&gate).is_none());
        release_tx.send(()).unwrap();
        worker.join().unwrap();
        assert!(CallbackAdmission::enter(&gate).is_some());
    }
}
