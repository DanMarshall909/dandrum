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
use std::num::NonZeroU64;
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
    ForeignEndpoints,
    EditorOwnerConflict,
    EditorSequenceExhausted,
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
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
        if !(1..=MAX_BLOCK_FRAMES).contains(&configuration.max_block_frames)
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
                configuration.sample_rate_hz.hz() as f32,
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
pub struct ParameterFlushReport {
    pub admission_closed: bool,
    pub parameter_updates: usize,
    pub dropped_feedback: usize,
    /// An ordinary note head was observed and retained. False does not promise
    /// an empty queue: the bounded inspection budget may have been exhausted.
    pub deferred_at_note: bool,
}

/// Progress made off audio while exclusively borrowing a matched endpoint pair.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuiescentReport {
    pub adopted: bool,
    /// Number of predecessor runtimes destroyed on the calling control thread.
    pub reclaimed_runtimes: usize,
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
    epoch: u64,
}

#[derive(Clone, Copy)]
struct QueuedFeedback {
    parameter: ParameterFeedback,
    settlement: u64,
    epoch: u64,
}

pub struct RuntimeControl {
    pending: Producer<PreparedRuntime>,
    retired: Consumer<PreparedRuntime>,
    events: Producer<QueuedEvent>,
    feedback: Consumer<QueuedFeedback>,
    values: Arc<[AtomicU64]>,
    settlement: Arc<AtomicU64>,
    epoch: Arc<AtomicU64>,
    admission: Arc<AtomicU64>,
    scalar_stamp: Arc<AtomicU64>,
    // Each word packs a 50-bit sequence, 7-bit onset, and 7-bit desired velocity.
    editor_intent: Arc<[AtomicU64; 128]>,
    editor_owners: [Option<NonZeroU64>; 128],
    editor_publishers: [Option<NonZeroU64>; 128],
    in_flight: bool,
    configuration: TriggerHostConfiguration,
    layout: Vec<PortMetadata>,
}

pub struct RuntimeAudio {
    state: AudioState,
    admission: Arc<AtomicU64>,
    scalar_stamp: Arc<AtomicU64>,
}

struct AudioState {
    current: PreparedRuntime,
    pending: Consumer<PreparedRuntime>,
    retired: Producer<PreparedRuntime>,
    events: Consumer<QueuedEvent>,
    feedback: Producer<QueuedFeedback>,
    values: Arc<[AtomicU64]>,
    settlement: Arc<AtomicU64>,
    epoch: Arc<AtomicU64>,
    event_capacity: usize,
    editor_intent: Arc<[AtomicU64; 128]>,
    editor_gates: [u64; 128],
    editor_seen: [u64; 128],
    editor_suppressed: [u64; 128],
    host_held: [bool; 128],
    editor_cursor: usize,
}

/// Reserving this lease does not publish a processor. Dropping it cancels the
/// candidate on the control thread without altering accepted or audio state.
pub struct RuntimeLease<'a> {
    // Only restore writes shared scalar/epoch state before audio consumes the
    // candidate. Ordinary immutable mapping reservations keep old audio live.
    _admission: Option<ClosedAdmission<'a>>,
    slot: rtrb::chunks::WriteChunkUninit<'a, PreparedRuntime>,
    in_flight: &'a mut bool,
    candidate: PreparedRuntime,
    values: &'a [AtomicU64],
    settlement: &'a AtomicU64,
    restored: Option<RestoredScalars>,
    epoch: &'a AtomicU64,
    scalar_stamp: &'a AtomicU64,
    editor_intent: &'a [AtomicU64; 128],
    editor_owners: &'a mut [Option<NonZeroU64>; 128],
    editor_publishers: &'a mut [Option<NonZeroU64>; 128],
}

struct RestoredScalars {
    values: Vec<f64>,
    epoch: u64,
}

pub(crate) struct RestoringControl<'a> {
    control: &'a mut RuntimeControl,
    values: Option<Vec<f64>>,
}

pub(crate) struct QuiescentControl<'a> {
    control: &'a mut RuntimeControl,
    audio: &'a mut RuntimeAudio,
    configuration: TriggerHostConfiguration,
}

pub(crate) struct QuiescentLease<'a> {
    configuration: &'a mut TriggerHostConfiguration,
    audio: &'a mut AudioState,
    candidate: PreparedRuntime,
}

impl ActivationHost for QuiescentControl<'_> {
    type Error = RuntimeError;
    type Lease<'a>
        = QuiescentLease<'a>
    where
        Self: 'a;

    fn activate<'a>(
        &'a mut self,
        prepared: &PreparedKernelInstrument,
        _: &SessionSnapshot,
    ) -> Result<Self::Lease<'a>, Self::Error> {
        // Session has not published yet: construction, probing and every
        // fallible check belong before its infallible publication boundary.
        let candidate = PreparedRuntime::new(prepared, self.configuration)?;
        if !compatible_layout(&candidate.0.layout, &self.control.layout)
            || self.audio.state.event_capacity > candidate.0.graph.pending_event_capacity() / 2
        {
            return Err(RuntimeError::InvalidConfiguration);
        }
        Ok(QuiescentLease {
            configuration: &mut self.control.configuration,
            audio: &mut self.audio.state,
            candidate,
        })
    }
}

impl ActivationLease for QuiescentLease<'_> {
    fn publish(self, _: Publication<'_>) {
        *self.configuration = self.candidate.0.configuration;
        let previous = std::mem::replace(&mut self.audio.current, self.candidate);
        let intents =
            std::array::from_fn(|pitch| self.audio.editor_intent[pitch].load(Ordering::Acquire));
        self.audio.adopt_editor_intent(&intents);
        drop(previous);
    }
}

impl ActivationHost for RestoringControl<'_> {
    type Error = RuntimeError;
    type Lease<'a>
        = RuntimeLease<'a>
    where
        Self: 'a;
    fn activate<'a>(
        &'a mut self,
        prepared: &PreparedKernelInstrument,
        next: &SessionSnapshot,
    ) -> Result<Self::Lease<'a>, Self::Error> {
        // Native session revisions are checked, monotonic publication identities.
        let restored = RestoredScalars {
            values: self
                .values
                .take()
                .expect("one activation per restore transaction"),
            epoch: next.revision,
        };
        self.control.activation(prepared, Some(restored))
    }
}

impl ActivationLease for RuntimeLease<'_> {
    fn publish(self, _: Publication<'_>) {
        *self.in_flight = true;
        self.slot.fill_from_iter(std::iter::once(self.candidate));
        if let Some(restored) = self.restored {
            let _write = ScalarWrite::begin(self.scalar_stamp);
            for (destination, value) in self.values.iter().zip(restored.values) {
                destination.store(value.to_bits(), Ordering::SeqCst);
            }
            self.settlement.store(0, Ordering::Release);
            self.epoch.store(restored.epoch, Ordering::Relaxed);
            // The retained admission guard excludes audio until all restore
            // state is published. Preserve sequence exhaustion across restore.
            for intent in self.editor_intent {
                let sequence = intent.load(Ordering::Relaxed) >> 14;
                intent.store(sequence << 14, Ordering::Release);
            }
            self.editor_owners.fill(None);
            self.editor_publishers.fill(None);
        }
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
        self.activation(prepared, None)
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
    let editor_intent = Arc::new(std::array::from_fn(|_| AtomicU64::new(0)));
    let admission = Arc::new(AtomicU64::new(0));
    let settlement = Arc::new(AtomicU64::new(0));
    let epoch = Arc::new(AtomicU64::new(0));
    let scalar_stamp = Arc::new(AtomicU64::new(0));
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
        epoch: epoch.clone(),
        admission: admission.clone(),
        scalar_stamp: scalar_stamp.clone(),
        editor_intent: editor_intent.clone(),
        editor_owners: [None; 128],
        editor_publishers: [None; 128],
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
            epoch,
            event_capacity,
            editor_intent,
            editor_gates: [0; 128],
            editor_seen: [0; 128],
            editor_suppressed: [0; 128],
            host_held: [false; 128],
            editor_cursor: 0,
        },
        admission,
        scalar_stamp,
    };
    Ok((control, audio))
}

impl RuntimeControl {
    /// Adopt an accepted pending runtime while audio ownership is returned to
    /// control. Reclaim at most one retired runtime and adopt at most one pending
    /// runtime, destroying predecessors on this calling thread. Both exclusive
    /// borrows keep this boundary separate from a real callback. A foreign pair
    /// is rejected before either endpoint is changed.
    pub fn service_quiescent(
        &mut self,
        audio: &mut RuntimeAudio,
    ) -> Result<QuiescentReport, RuntimeError> {
        self.validate_endpoint(audio)?;
        let mut report = QuiescentReport::default();
        if let Some(retired) = self.reclaim() {
            drop(retired);
            report.reclaimed_runtimes += 1;
        }
        if let Ok(next) = audio.state.pending.pop() {
            let previous = std::mem::replace(&mut audio.state.current, next);
            let intents: [u64; 128] = std::array::from_fn(|pitch| {
                audio.state.editor_intent[pitch].load(Ordering::Acquire)
            });
            audio.state.adopt_editor_intent(&intents);
            drop(previous);
            self.in_flight = false;
            report.adopted = true;
            report.reclaimed_runtimes += 1;
        }
        Ok(report)
    }

    pub(crate) fn validate_endpoint(&self, audio: &RuntimeAudio) -> Result<(), RuntimeError> {
        if Arc::ptr_eq(&self.admission, &audio.admission) {
            Ok(())
        } else {
            Err(RuntimeError::ForeignEndpoints)
        }
    }

    pub(crate) fn configuration(&self) -> TriggerHostConfiguration {
        self.configuration
    }

    pub(crate) fn reconfiguring<'a>(
        &'a mut self,
        audio: &'a mut RuntimeAudio,
        configuration: TriggerHostConfiguration,
    ) -> QuiescentControl<'a> {
        QuiescentControl {
            control: self,
            audio,
            configuration,
        }
    }

    pub(crate) fn activation_available(&self) -> bool {
        !self.in_flight
    }

    /// The caller authenticates the attachment; publication is one coherent word
    /// per MIDI pitch, independent of the ordinary event queue's capacity.
    pub fn editor_note_on(
        &mut self,
        owner: NonZeroU64,
        note: u8,
        velocity: u8,
    ) -> Result<(), RuntimeError> {
        if note > 127 || !(1..=127).contains(&velocity) {
            return Err(RuntimeError::InvalidEvent);
        }
        let pitch = usize::from(note);
        if self.editor_owners[pitch].is_some_and(|held| held != owner) {
            return Err(RuntimeError::EditorOwnerConflict);
        }
        let previous = self.editor_intent[pitch].load(Ordering::Relaxed) >> 14;
        if previous == u64::MAX >> 14 {
            return Err(RuntimeError::EditorSequenceExhausted);
        }
        self.editor_owners[pitch] = Some(owner);
        self.editor_publishers[pitch] = Some(owner);
        self.editor_intent[pitch].store(
            ((previous + 1) << 14) | (u64::from(velocity) << 7) | u64::from(velocity),
            Ordering::Release,
        );
        Ok(())
    }

    /// Ordinary release retains an unheard onset, so a short keyboard tap
    /// remains audible. Only its holder may release a currently held pitch.
    pub fn editor_note_off(&mut self, owner: NonZeroU64, note: u8) -> Result<(), RuntimeError> {
        if note > 127 {
            return Err(RuntimeError::InvalidEvent);
        }
        let pitch = usize::from(note);
        if self.editor_owners[pitch].is_some_and(|held| held != owner) {
            return Err(RuntimeError::EditorOwnerConflict);
        }
        if self.editor_owners[pitch] == Some(owner) {
            self.editor_owners[pitch] = None;
            let intent = self.editor_intent[pitch].load(Ordering::Relaxed);
            self.editor_intent[pitch].store(intent & !127, Ordering::Release);
        }
        Ok(())
    }

    /// Detachment cancels both held intent and an unheard released tap. A newer
    /// publisher owns its row, so stale attachment cleanup cannot clear it.
    pub fn detach_editor_notes(&mut self, owner: NonZeroU64) {
        for (pitch, publisher) in self.editor_publishers.iter_mut().enumerate() {
            if *publisher == Some(owner) {
                *publisher = None;
                self.editor_owners[pitch] = None;
                let sequence = self.editor_intent[pitch].load(Ordering::Relaxed) >> 14;
                self.editor_intent[pitch].store(sequence << 14, Ordering::Release);
            }
        }
    }

    pub(crate) fn restoring(&mut self, values: Vec<f64>) -> RestoringControl<'_> {
        RestoringControl {
            control: self,
            values: Some(values),
        }
    }
    fn activation(
        &mut self,
        prepared: &PreparedKernelInstrument,
        restored: Option<RestoredScalars>,
    ) -> Result<RuntimeLease<'_>, RuntimeError> {
        if !self.activation_available() {
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
        let admission = restored
            .as_ref()
            .map(|_| ClosedAdmission::close(&self.admission));
        Ok(RuntimeLease {
            _admission: admission,
            slot,
            in_flight: &mut self.in_flight,
            candidate,
            values: &self.values,
            settlement: &self.settlement,
            restored,
            epoch: &self.epoch,
            scalar_stamp: &self.scalar_stamp,
            editor_intent: &self.editor_intent,
            editor_owners: &mut self.editor_owners,
            editor_publishers: &mut self.editor_publishers,
        })
    }
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
        self.current_feedback().map(|feedback| feedback.parameter)
    }
    pub(crate) fn take_sequenced_feedback(&mut self) -> Option<(ParameterFeedback, u64, bool)> {
        self.current_feedback().map(|feedback| {
            (
                feedback.parameter,
                feedback.settlement >> 1,
                feedback.settlement & 1 != 0,
            )
        })
    }
    fn current_feedback(&mut self) -> Option<QueuedFeedback> {
        let epoch = self.epoch.load(Ordering::Relaxed);
        for _ in 0..self.feedback.buffer().capacity() {
            let feedback = self.feedback.pop().ok()?;
            if feedback.epoch == epoch {
                return Some(feedback);
            }
        }
        None
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

    /// Read a vector from a completed scalar-write interval. Writers are
    /// serialized by callback admission; only this off-audio reader retries.
    /// Every stamp/data access is SeqCst, so equal even stamps exclude an
    /// intervening writer in their single global atomic order.
    pub(crate) fn snapshot_state_values(&self) -> Vec<f64> {
        let mut values = vec![0.0; self.values.len()];
        loop {
            let before = self.scalar_stamp.load(Ordering::SeqCst);
            if before & 1 != 0 {
                std::thread::yield_now();
                continue;
            }
            for (destination, source) in values.iter_mut().zip(self.values.iter()) {
                *destination = f64::from_bits(source.load(Ordering::SeqCst));
            }
            if before == self.scalar_stamp.load(Ordering::SeqCst) {
                return values;
            }
        }
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
                epoch: self.epoch.load(Ordering::Relaxed),
            })
            .map_err(|_| RuntimeError::QueueFull)
    }
    fn compatible(&self, next: &PreparedRuntime) -> bool {
        next.0.configuration == self.configuration
            && compatible_layout(&next.0.layout, &self.layout)
    }

    /// A rejected value remains owned by the caller, including under backpressure.
    pub fn offer(&mut self, next: PreparedRuntime) -> Result<(), PreparedRuntime> {
        if !self.activation_available() || !self.compatible(&next) {
            return Err(next);
        }
        // Capacity returns only after a consumed candidate's predecessor is
        // reclaimed, either through retirement or direct quiescent adoption.
        self.pending
            .write_chunk_uninit(1)
            .expect("idle activation slot")
            .fill_from_iter(std::iter::once(next));
        self.in_flight = true;
        Ok(())
    }

    /// Return a running callback's retired runtime to the control thread.
    /// Activation stays busy until reclamation or paired quiescent service.
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

// At two increments per callback, the u64 stamp cannot wrap during a
// practical snapshot read; a reader must not span 2^63 complete write intervals.
struct ScalarWrite<'a>(&'a AtomicU64);
impl<'a> ScalarWrite<'a> {
    fn begin(stamp: &'a AtomicU64) -> Self {
        stamp.fetch_add(1, Ordering::SeqCst);
        Self(stamp)
    }
}
impl Drop for ScalarWrite<'_> {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
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
    /// Apply a bounded scalar prefix and parameter-only host batch without
    /// advancing DSP or runtime ownership. Validate the complete batch before
    /// admission; an ordinary queued note retains itself and its FIFO suffix.
    pub fn flush_parameters(
        &mut self,
        host: &[ControlEvent],
    ) -> Result<ParameterFlushReport, RuntimeError> {
        if host.len() > self.state.event_capacity
            || host.iter().any(|event| {
                !matches!(event, ControlEvent::Parameter { .. })
                    || !event.valid(&self.state.current.0.layout)
            })
        {
            return Err(RuntimeError::InvalidEvent);
        }
        let Some(_admission) = CallbackAdmission::enter(&self.admission) else {
            return Ok(ParameterFlushReport {
                admission_closed: true,
                ..ParameterFlushReport::default()
            });
        };
        let _write = ScalarWrite::begin(&self.scalar_stamp);
        let mut report = ParameterFlushReport::default();
        for _ in 0..self.state.event_capacity {
            let Ok(&queued) = self.state.events.peek() else {
                break;
            };
            let ControlEvent::Parameter { index, value } = queued.event else {
                report.deferred_at_note = true;
                break;
            };
            if queued.epoch == self.state.epoch.load(Ordering::Relaxed) {
                report.parameter_updates += 1;
                report.dropped_feedback += usize::from(self.state.apply_parameter(
                    index,
                    value,
                    0,
                    queued.command_sequence,
                ));
            }
            let _consumed = self.state.events.pop().expect("observed scalar head");
        }
        for &event in host {
            if let ControlEvent::Parameter { index, value } = event {
                report.parameter_updates += 1;
                report.dropped_feedback +=
                    usize::from(self.state.apply_parameter(index, value, 0, 0));
            }
        }
        Ok(report)
    }
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
        let _write = ScalarWrite::begin(&self.scalar_stamp);
        self.state.process_events(left, right, events)
    }
}

impl AudioState {
    fn adopt_editor_intent(&mut self, intents: &[u64; 128]) {
        // Accepted gates belonged to the retired processor. A fresh graph
        // resumes desired pitches but must not replay a released tap.
        self.editor_gates.fill(0);
        for (seen, intent) in self.editor_seen.iter_mut().zip(intents) {
            *seen = *intent >> 14;
        }
    }

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
            if event.epoch == self.epoch.load(Ordering::Relaxed) && event.command_sequence != 0 {
                report.cancelled_parameter_commands += 1;
                cancelled = event.command_sequence;
            }
        }
        self.current.0.graph.reset();
        self.editor_gates.fill(0);
        self.host_held.fill(false);
        for (pitch, intent) in self.editor_intent.iter().enumerate() {
            let sequence = intent.load(Ordering::Acquire) >> 14;
            self.editor_suppressed[pitch] = sequence;
            self.editor_seen[pitch] = sequence;
        }
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
        let intents =
            std::array::from_fn(|pitch| self.editor_intent[pitch].load(Ordering::Acquire));
        if report.swapped {
            self.adopt_editor_intent(&intents);
        }
        let mut touched = [false; 128];
        let mut takeover_credit = [false; 128];
        for event in events {
            match event.event {
                ControlEvent::NoteOn { note, .. } => {
                    let pitch = usize::from(note);
                    touched[pitch] = true;
                    takeover_credit[pitch] = self.editor_gates[pitch] != 0;
                }
                ControlEvent::NoteOff { note } => touched[usize::from(note)] = true,
                ControlEvent::Parameter { .. } => {}
            }
        }
        // The validated host count is <= E and possible takeovers <= E;
        // construction proves 2E <= P. Reserve both before any other work.
        let mut budget = self.current.0.graph.pending_event_capacity()
            - events.len()
            - takeover_credit.iter().filter(|&&reserved| reserved).count();
        self.release_editor_notes(&intents, &mut takeover_credit, &mut budget);
        // A producer may refill concurrently. The finite bound is E, never
        // "until empty". Peek preserves a deferred or rejected queue head.
        for _ in 0..self.event_capacity {
            let Ok(&event) = self.events.peek() else {
                break;
            };
            if event.epoch != self.epoch.load(Ordering::Relaxed) {
                let _consumed = self.events.pop().expect("observed queue head");
                continue;
            }
            let takeover = match event.event {
                ControlEvent::NoteOn { note, .. } if self.editor_gates[usize::from(note)] != 0 => {
                    Some(usize::from(note))
                }
                _ => None,
            };
            let cost = 1 + usize::from(takeover.is_some_and(|pitch| !takeover_credit[pitch]));
            if cost > budget
                || self
                    .apply_event(event.event, 0, event.command_sequence, &mut report)
                    .is_err()
            {
                break;
            }
            budget -= cost;
            if let Some(pitch) = takeover {
                takeover_credit[pitch] = false;
            }
            let _consumed = self.events.pop().expect("accepted queue head");
            if let ControlEvent::NoteOn { note, .. } | ControlEvent::NoteOff { note } = event.event
            {
                touched[usize::from(note)] = true;
            }
        }
        self.start_editor_notes(&intents, &touched, &mut budget);
        self.editor_cursor = (self.editor_cursor + 1) % 128;
        let mut start = 0;
        for event in events {
            let end = event.frame_offset as usize;
            if end > start {
                self.current
                    .0
                    .render(&self.values, &mut left[start..end], &mut right[start..end]);
            }
            self.apply_event(event.event, event.frame_offset, 0, &mut report)?;
            start = end;
        }
        self.current
            .0
            .render(&self.values, &mut left[start..], &mut right[start..]);
        Ok(report)
    }

    fn release_editor_notes(
        &mut self,
        intents: &[u64; 128],
        takeover_credit: &mut [bool; 128],
        budget: &mut usize,
    ) {
        for offset in 0..128 {
            let pitch = (self.editor_cursor + offset) % 128;
            let intent = intents[pitch];
            if self.editor_gates[pitch] != 0
                && (intent & 127 == 0 || intent >> 14 != self.editor_gates[pitch])
                && (takeover_credit[pitch] || *budget != 0)
                && self.current.0.graph.try_note_off_at(pitch as u8, 0)
            {
                self.editor_gates[pitch] = 0;
                if takeover_credit[pitch] {
                    takeover_credit[pitch] = false;
                } else {
                    *budget -= 1;
                }
            }
        }
    }

    fn start_editor_notes(
        &mut self,
        intents: &[u64; 128],
        touched: &[bool; 128],
        budget: &mut usize,
    ) {
        for offset in 0..128 {
            let pitch = (self.editor_cursor + offset) % 128;
            let intent = intents[pitch];
            let sequence = intent >> 14;
            if self.host_held[pitch] || touched[pitch] {
                self.editor_seen[pitch] = sequence;
                continue;
            }
            let onset = ((intent >> 7) & 127) as u8;
            if onset != 0
                && sequence != self.editor_suppressed[pitch]
                && (intent & 127 != 0 || sequence != self.editor_seen[pitch])
                && self.editor_gates[pitch] == 0
                && *budget != 0
                && self.current.0.graph.try_note_on_at(pitch as u8, onset, 0)
            {
                self.editor_gates[pitch] = sequence;
                self.editor_seen[pitch] = sequence;
                *budget -= 1;
            }
        }
    }

    fn apply_event(
        &mut self,
        event: ControlEvent,
        frame_offset: u32,
        command_sequence: u64,
        report: &mut ProcessReport,
    ) -> Result<(), RuntimeError> {
        match event {
            ControlEvent::NoteOn { note, velocity } => {
                let pitch = usize::from(note);
                if self.editor_gates[pitch] != 0 {
                    if !self.current.0.graph.try_note_off_at(note, 0) {
                        return Err(RuntimeError::QueueFull);
                    }
                    self.editor_gates[pitch] = 0;
                }
                if !self.current.0.graph.try_note_on_at(note, velocity, 0) {
                    return Err(RuntimeError::QueueFull);
                }
                self.host_held[pitch] = true;
            }
            ControlEvent::NoteOff { note } => {
                if !self.current.0.graph.try_note_off_at(note, 0) {
                    return Err(RuntimeError::QueueFull);
                }
                self.host_held[usize::from(note)] = false;
                self.editor_gates[usize::from(note)] = 0;
            }
            ControlEvent::Parameter { index, value } => {
                report.parameter_updates += 1;
                report.dropped_feedback +=
                    usize::from(self.apply_parameter(index, value, frame_offset, command_sequence));
            }
        }
        Ok(())
    }

    /// Scalar application is infallible; only its bounded feedback may drop.
    /// Admission, queue budgets and note ordering remain the caller's policy.
    fn apply_parameter(
        &mut self,
        index: usize,
        value: f64,
        frame_offset: u32,
        command_sequence: u64,
    ) -> bool {
        self.values[index].store(value.to_bits(), Ordering::SeqCst);
        if command_sequence != 0 {
            self.settlement
                .store(command_sequence << 1, Ordering::Release);
        }
        self.feedback
            .push(QueuedFeedback {
                parameter: ParameterFeedback {
                    index,
                    value,
                    frame_offset,
                },
                settlement: self.settlement.load(Ordering::Relaxed),
                epoch: self.epoch.load(Ordering::Relaxed),
            })
            .is_err()
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

fn compatible_layout(left: &[PortMetadata], right: &[PortMetadata]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(a, b)| {
            a.name() == b.name()
                && a.direction() == b.direction()
                && a.signal_type() == b.signal_type()
                && a.channels() == b.channels()
                && a.multiplicity() == b.multiplicity()
                && a.control_default().map(|d| (d.min(), d.max(), d.unit()))
                    == b.control_default().map(|d| (d.min(), d.max(), d.unit()))
        })
}

#[cfg(test)]
mod admission_tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    #[test]
    fn stopped_host_scalar_cannot_acknowledge_a_current_ui_command_behind_a_note() {
        let (mut control, mut audio) = gain_runtime();
        control
            .try_send(ControlEvent::NoteOn {
                note: 60,
                velocity: 100,
            })
            .unwrap();
        control.send_parameter(0, 0.5, 31).unwrap();
        let audit = allocation_counter::measure(|| {
            assert_eq!(
                audio.flush_parameters(&[ControlEvent::Parameter {
                    index: 0,
                    value: 2.0
                }]),
                Ok(ParameterFlushReport {
                    parameter_updates: 1,
                    deferred_at_note: true,
                    ..ParameterFlushReport::default()
                })
            );
        });
        assert_eq!(audit, allocation_counter::AllocationInfo::default());
        assert_eq!(control.snapshot_parameters(), (0, false, vec![2.0]));
        assert_eq!(
            control.take_feedback(),
            Some(ParameterFeedback {
                index: 0,
                value: 2.0,
                frame_offset: 0
            })
        );
        let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
        assert_eq!(
            audio
                .process(&mut left, &mut right)
                .unwrap()
                .parameter_updates,
            1
        );
        assert_eq!(control.snapshot_parameters(), (31, false, vec![0.5]));
    }

    #[test]
    fn successful_state_publication_retires_queued_notes_and_reset_cannot_settle_old_generation_commands()
     {
        let repository = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let source = repository.join("examples/patches/advanced-drum-kit.yaml");
        let yaml = std::fs::read_to_string(&source).unwrap();
        let patch = dandrum_engine::kernel::document::load_kernel_patch_str(&yaml).unwrap();
        let buses = patch.root().ports().iter().fold(
            dandrum_engine::preparation::HostBuses::new(),
            |buses, port| match port.direction() {
                PortDirection::Input => buses.with_input(port.name(), 1),
                PortDirection::Output => buses.with_output(port.name(), 2),
            },
        );
        let config = TriggerHostConfiguration {
            sample_rate_hz: crate::SampleRate::new(48_000.0).unwrap(),
            max_block_frames: 16,
        };
        let mut session = dandrum_instrument_session::InstrumentSession::new_for_planar_host(
            &yaml,
            dandrum_engine::preparation::PreparationContext::new(source.parent().unwrap(), 48_000),
            dandrum_engine::patch::RenderSettings {
                sample_rate_hz: 48_000,
                block_size_frames: 16,
                duration_frames: 16,
            },
            buses,
        )
        .unwrap();
        let (mut control, mut audio) =
            runtime_handoff(PreparedRuntime::new(session.prepared(), config).unwrap(), 8).unwrap();
        control
            .try_send(ControlEvent::NoteOn {
                note: 60,
                velocity: 100,
            })
            .unwrap();
        control.send_parameter(2, 3.0, 9).unwrap();
        let changed = yaml.replacen("key_range: [36, 36]", "key_range: [60, 60]", 1);
        let candidate = session
            .prepare_document_replacement_owned(&changed)
            .unwrap();
        let mut values = control.snapshot_parameters().2;
        values[2] = 0.5;
        // A paused external restore still gates callbacks: it must serialize
        // its scalar/epoch writer, unlike an immutable mapping reservation.
        let before = session.owner_snapshot();
        let old_values = control.snapshot_parameters();
        let mut restoration = control.restoring(values.clone());
        let lease = restoration.activate(session.prepared(), &before).unwrap();
        let (mut blocked_left, mut blocked_right) = ([1.0; 16], [1.0; 16]);
        let callback = allocation_counter::measure(|| {
            assert_eq!(
                audio.flush_parameters(&[ControlEvent::Parameter {
                    index: 2,
                    value: 1.0
                }]),
                Ok(ParameterFlushReport {
                    admission_closed: true,
                    ..ParameterFlushReport::default()
                })
            );
            assert_eq!(
                audio.flush_parameters(&[
                    ControlEvent::Parameter {
                        index: 2,
                        value: 1.0
                    },
                    ControlEvent::NoteOff { note: 36 }
                ]),
                Err(RuntimeError::InvalidEvent)
            );
            assert!(audio.reset().admission_closed);
            let report = audio
                .process(&mut blocked_left, &mut blocked_right)
                .unwrap();
            assert!(report.admission_closed);
            assert!(!report.swapped);
            assert_eq!((blocked_left, blocked_right), ([0.0; 16], [0.0; 16]));
            assert!(
                audio
                    .process_events(
                        &mut blocked_left,
                        &mut blocked_right,
                        &[TimedEvent {
                            frame_offset: 16,
                            event: ControlEvent::Parameter {
                                index: usize::MAX,
                                value: f64::NAN
                            },
                        }]
                    )
                    .unwrap()
                    .admission_closed,
                "process retains admission-before-validation precedence"
            );
        });
        assert_eq!(
            (
                callback.count_total,
                callback.count_current,
                callback.bytes_current
            ),
            (0, 0, 0)
        );
        let cancelled = allocation_counter::measure(|| drop(lease));
        assert_eq!(cancelled.count_total, 0);
        assert!(
            cancelled.count_current < 0,
            "cancelled restore candidate is destroyed off audio"
        );
        drop(restoration);
        assert_eq!(session.owner_snapshot(), before);
        assert_eq!(control.snapshot_parameters(), old_values);
        session
            .commit_document_replacement(candidate, &mut control.restoring(values))
            .unwrap();
        assert_eq!(
            audio.flush_parameters(&[]),
            Ok(ParameterFlushReport {
                deferred_at_note: true,
                ..ParameterFlushReport::default()
            }),
            "an obsolete note is still a stopped-flush barrier before epoch filtering"
        );
        let (mut left, mut right) = ([1.0; 16], [1.0; 16]);
        audio.process(&mut left, &mut right).unwrap();
        assert_eq!(
            (left, right),
            ([0.0; 16], [0.0; 16]),
            "old queued note60 must not start a restored voice"
        );
        assert_eq!(control.parameter_value(2), Some(0.5));
        assert!(control.take_feedback().is_none());
        drop(control.reclaim());
        // Keep reset's original obsolete-command witness separate from flush:
        // reset must inspect this nonzero sequence rather than an empty queue.
        control.send_parameter(2, 3.0, 10).unwrap();
        let candidate = session.prepare_document_replacement_owned(&yaml).unwrap();
        let mut values = control.snapshot_parameters().2;
        values[2] = 0.25;
        session
            .commit_document_replacement(candidate, &mut control.restoring(values))
            .unwrap();
        let restored = control.snapshot_parameters();
        assert_eq!((restored.0, restored.1, restored.2[2]), (0, false, 0.25));
        let mut reset = None;
        let audit = allocation_counter::measure(|| reset = Some(audio.reset()));
        assert_eq!(audit, allocation_counter::AllocationInfo::default());
        let reset = reset.unwrap();
        assert_eq!(
            (reset.discarded_events, reset.cancelled_parameter_commands),
            (1, 0),
            "reset discards obsolete UI commands without cancelling this epoch"
        );
        assert_eq!(control.snapshot_parameters(), restored);
        assert!(control.take_feedback().is_none());
        audio.process(&mut left, &mut right).unwrap();
        drop(control.reclaim());
        for sequence in 20..28 {
            control.send_parameter(2, 3.0, sequence).unwrap();
        }
        let candidate = session.prepare_document_replacement_owned(&yaml).unwrap();
        let mut values = control.snapshot_parameters().2;
        values[2] = 0.25;
        session
            .commit_document_replacement(candidate, &mut control.restoring(values))
            .unwrap();
        assert_eq!(
            audio.flush_parameters(&[]),
            Ok(ParameterFlushReport::default()),
            "an entire capacity of obsolete scalar prefix is discarded without settlement/feedback or adopting the pending graph"
        );
        assert_eq!(
            audio.reset().discarded_events,
            0,
            "flush consumed the obsolete scalar prefix"
        );
        assert_eq!(
            control.snapshot_parameters(),
            (0, false, control.snapshot_parameters().2),
            "reset cannot acknowledge an old epoch's UI command"
        );
        audio
            .process_events(
                &mut left,
                &mut right,
                &[TimedEvent {
                    frame_offset: 0,
                    event: ControlEvent::NoteOn {
                        note: 36,
                        velocity: 100,
                    },
                }],
            )
            .unwrap();
        assert_eq!((left[0], right[0]), (-0.125, -0.125));
    }

    fn gain_runtime() -> (RuntimeControl, RuntimeAudio) {
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
                sample_rate_hz: crate::SampleRate::new(48_000.0).unwrap(),
                max_block_frames: 16,
            },
        )
        .unwrap();
        runtime_handoff(prepared, 4).unwrap()
    }

    #[test]
    fn whole_state_reader_waits_only_for_the_scalar_interval_and_never_closes_callback_admission() {
        let (control, audio) = gain_runtime();
        let stamp = control.scalar_stamp.clone();
        let gate = audio.admission.clone();
        let callback = CallbackAdmission::enter(&gate).unwrap();
        let write = ScalarWrite::begin(&stamp);
        control.values[0].store(0.5f64.to_bits(), Ordering::SeqCst);
        let (ready_tx, ready_rx) = mpsc::sync_channel(0);
        let (value_tx, value_rx) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            ready_tx.send(()).unwrap();
            value_tx.send(control.snapshot_state_values()).unwrap();
        });
        ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let early = value_rx.recv_timeout(Duration::from_millis(10));
        drop(write);
        assert!(
            matches!(early, Err(mpsc::RecvTimeoutError::Timeout)),
            "save cannot return an unfinished scalar write"
        );
        let finished = value_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("completed scalar interval releases the main reader");
        reader.join().unwrap();
        let admission_during_read = gate.load(Ordering::Acquire);
        drop(callback);
        assert_eq!(finished, vec![0.5]);
        assert_eq!(admission_during_read, CALLBACK_ACTIVE);
    }

    mod editor_note_fixture {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/editor_notes.rs"
        ));
    }

    fn callback(audio: &mut RuntimeAudio) -> ([f32; 16], [f32; 16]) {
        let (mut left, mut right) = ([1.0; 16], [1.0; 16]);
        let measured = allocation_counter::measure(|| {
            audio.process(&mut left, &mut right).unwrap();
        });
        assert_eq!(
            (
                measured.count_total,
                measured.count_current,
                measured.bytes_current
            ),
            (0, 0, 0)
        );
        (left, right)
    }

    fn assert_gated_prefix(output: ([f32; 16], [f32; 16])) {
        assert_eq!(&output.0[..4], &editor_note_fixture::PREFIX_L);
        assert_eq!(&output.1[..4], &editor_note_fixture::PREFIX_R);
        assert_eq!(&output.0[4..], &[-0.125; 12]);
        assert_eq!(&output.1[4..], &[-0.0625; 12]);
    }

    // Valid public calls obey the P-budget and cannot prefill this queue.
    // These internal preconditions protect defensive retention, while the
    // integration suite separately proves real public saturation and PCM.
    fn fill_actual_graph_queue(audio: &mut RuntimeAudio, available: usize) {
        let capacity = audio.state.current.0.graph.pending_event_capacity();
        assert_eq!(capacity, 128);
        for _ in 0..capacity - available {
            assert!(audio.state.current.0.graph.try_note_off_at(127, 0));
        }
    }

    #[test]
    fn editor_note_lifecycle_and_restore_are_independent_between_runtime_instances() {
        let mut fixture = editor_note_fixture::Fixture::new("gated", 4);
        let other_fixture = editor_note_fixture::Fixture::with_pan("gated", 4, 0.5);
        let (mut control, mut audio) = fixture.runtime(4);
        let (mut other, mut other_audio) = other_fixture.runtime(4);
        let owner = NonZeroU64::new(17).unwrap();
        control.editor_note_on(owner, 60, 100).unwrap();
        other.editor_note_on(owner, 60, 100).unwrap();
        assert_gated_prefix(callback(&mut audio));
        let output = callback(&mut other_audio);
        assert_eq!(&output.0[..4], &editor_note_fixture::PREFIX_R);
        assert_eq!(&output.1[..4], &editor_note_fixture::PREFIX_L);
        control.detach_editor_notes(owner);
        assert_eq!(callback(&mut audio), ([0.0; 16], [0.0; 16]));
        assert_eq!(callback(&mut other_audio), ([-0.0625; 16], [-0.125; 16]));
        control.editor_note_on(owner, 60, 100).unwrap();
        assert_gated_prefix(callback(&mut audio));
        let reset = allocation_counter::measure(|| {
            audio.reset();
        });
        assert_eq!(
            (reset.count_total, reset.count_current, reset.bytes_current),
            (0, 0, 0)
        );
        assert_eq!(callback(&mut audio), ([0.0; 16], [0.0; 16]));
        assert_eq!(callback(&mut other_audio), ([-0.0625; 16], [-0.125; 16]));
        control.editor_note_on(owner, 60, 100).unwrap();
        assert!(control.offer(other_fixture.prepared()).is_ok());
        let output = callback(&mut audio);
        assert_eq!(&output.0[..4], &editor_note_fixture::PREFIX_R);
        assert_eq!(&output.1[..4], &editor_note_fixture::PREFIX_L);
        drop(control.reclaim().unwrap());
        assert_eq!(callback(&mut other_audio), ([-0.0625; 16], [-0.125; 16]));
        let yaml = fixture.session.owner_snapshot().yaml;
        let candidate = fixture
            .session
            .prepare_document_replacement_owned(&yaml)
            .unwrap();
        fixture
            .session
            .commit_document_replacement(candidate, &mut control.restoring(Vec::new()))
            .unwrap();
        assert_eq!(callback(&mut audio), ([0.0; 16], [0.0; 16]));
        drop(control.reclaim().unwrap());
        assert_eq!(callback(&mut other_audio), ([-0.0625; 16], [-0.125; 16]));
        other.detach_editor_notes(owner);
        assert_eq!(callback(&mut other_audio), ([0.0; 16], [0.0; 16]));
    }

    #[test]
    fn cancelled_and_busy_restore_preserve_held_and_unheard_editor_intent() {
        for heard in [false, true] {
            let fixture = editor_note_fixture::Fixture::new("gated", 4);
            let (mut control, mut audio) = fixture.runtime(4);
            let owner = NonZeroU64::new(17).unwrap();
            control.editor_note_on(owner, 60, 100).unwrap();
            if heard {
                assert_gated_prefix(callback(&mut audio));
            }
            let before = fixture.session.owner_snapshot();
            let mut restoration = control.restoring(Vec::new());
            let lease = restoration
                .activate(fixture.session.prepared(), &before)
                .unwrap();
            assert_eq!(callback(&mut audio), ([0.0; 16], [0.0; 16]));
            drop(lease);
            drop(restoration);
            if heard {
                assert_eq!(callback(&mut audio), ([-0.125; 16], [-0.0625; 16]));
            } else {
                assert_gated_prefix(callback(&mut audio));
            }
            assert!(control.offer(fixture.prepared()).is_ok());
            assert!(matches!(
                control
                    .restoring(Vec::new())
                    .activate(fixture.session.prepared(), &before),
                Err(RuntimeError::Busy)
            ));
            assert_gated_prefix(callback(&mut audio));
            drop(control.reclaim().unwrap());
            control.detach_editor_notes(owner);
            assert_eq!(callback(&mut audio), ([0.0; 16], [0.0; 16]));
        }
    }

    #[test]
    fn successful_restore_clears_old_editor_intent_and_allows_a_new_owner() {
        for (heard, released) in [(true, false), (false, false), (false, true)] {
            let mut fixture = editor_note_fixture::Fixture::new("gated", 4);
            let (mut control, mut audio) = fixture.runtime(4);
            let old = NonZeroU64::new(17).unwrap();
            let new = NonZeroU64::new(18).unwrap();
            control.editor_note_on(old, 60, 100).unwrap();
            if heard {
                assert_gated_prefix(callback(&mut audio));
            }
            if released {
                control.editor_note_off(old, 60).unwrap();
            }
            let changed = fixture
                .session
                .owner_snapshot()
                .yaml
                .replace("pan: -0.5", "pan: 0.5");
            let candidate = fixture
                .session
                .prepare_document_replacement_owned(&changed)
                .unwrap();
            fixture
                .session
                .commit_document_replacement(candidate, &mut control.restoring(Vec::new()))
                .unwrap();
            assert_eq!(
                callback(&mut audio),
                ([0.0; 16], [0.0; 16]),
                "restore retires old desired and unheard intent"
            );
            drop(control.reclaim().unwrap());
            control.editor_note_on(new, 60, 100).unwrap();
            control.detach_editor_notes(old);
            let output = callback(&mut audio);
            assert_eq!(&output.0[..4], &editor_note_fixture::PREFIX_R);
            assert_eq!(&output.1[..4], &editor_note_fixture::PREFIX_L);
            assert_eq!(callback(&mut audio), ([-0.0625; 16], [-0.125; 16]));
            control.detach_editor_notes(new);
            assert_eq!(callback(&mut audio), ([0.0; 16], [0.0; 16]));
        }
    }

    #[test]
    fn failed_checked_release_retains_gate_and_new_owner_onset_until_release_succeeds() {
        let (mut control, mut audio) = editor_note_fixture::Fixture::new("gated", 4).runtime(4);
        let old = NonZeroU64::new(17).unwrap();
        let new = NonZeroU64::new(18).unwrap();
        control.editor_note_on(old, 60, 100).unwrap();
        assert_gated_prefix(callback(&mut audio));
        control.editor_note_off(old, 60).unwrap();
        control.editor_note_on(new, 60, 100).unwrap();
        fill_actual_graph_queue(&mut audio, 0);
        assert_eq!(callback(&mut audio), ([-0.125; 16], [-0.0625; 16]));
        assert_gated_prefix(callback(&mut audio));
        control.detach_editor_notes(old);
        assert_eq!(callback(&mut audio), ([-0.125; 16], [-0.0625; 16]));
        control.detach_editor_notes(new);
        assert_eq!(callback(&mut audio), ([0.0; 16], [0.0; 16]));
    }

    #[test]
    fn failed_checked_onset_and_ordinary_takeover_keep_their_pending_work() {
        let (mut control, mut audio) = editor_note_fixture::Fixture::new("gated", 4).runtime(4);
        let owner = NonZeroU64::new(17).unwrap();
        control.editor_note_on(owner, 60, 100).unwrap();
        fill_actual_graph_queue(&mut audio, 0);
        assert_eq!(callback(&mut audio), ([0.0; 16], [0.0; 16]));
        assert_gated_prefix(callback(&mut audio));
        control
            .try_send(ControlEvent::NoteOn {
                note: 60,
                velocity: 100,
            })
            .unwrap();
        fill_actual_graph_queue(&mut audio, 0);
        assert_eq!(callback(&mut audio), ([-0.125; 16], [-0.0625; 16]));
        assert_gated_prefix(callback(&mut audio));
        control.detach_editor_notes(owner);
        assert_eq!(callback(&mut audio), ([-0.125; 16], [-0.0625; 16]));
        control
            .try_send(ControlEvent::NoteOff { note: 60 })
            .unwrap();
        fill_actual_graph_queue(&mut audio, 0);
        assert_eq!(callback(&mut audio), ([-0.125; 16], [-0.0625; 16]));
        assert_eq!(callback(&mut audio), ([0.0; 16], [0.0; 16]));
    }

    #[test]
    fn checked_takeover_release_success_does_not_lose_queued_onset_when_only_one_slot_was_free() {
        let (mut control, mut audio) = editor_note_fixture::Fixture::new("gated", 4).runtime(4);
        let owner = NonZeroU64::new(17).unwrap();
        control.editor_note_on(owner, 60, 100).unwrap();
        assert_gated_prefix(callback(&mut audio));
        control
            .try_send(ControlEvent::NoteOn {
                note: 60,
                velocity: 100,
            })
            .unwrap();
        fill_actual_graph_queue(&mut audio, 1);
        assert_eq!(callback(&mut audio), ([0.0; 16], [0.0; 16]));
        assert_gated_prefix(callback(&mut audio));
        control.detach_editor_notes(owner);
        assert_eq!(callback(&mut audio), ([-0.125; 16], [-0.0625; 16]));
    }

    #[test]
    fn exhausted_editor_sequence_is_rejected_and_release_does_not_wrap_it() {
        let (mut control, _audio) = gain_runtime();
        let owner = NonZeroU64::new(17).unwrap();
        // Seed the finite counter boundary: reaching it by 2^50 calls is not a
        // useful test. Assertions use the public admission/release contract.
        control.editor_intent[60].store(!16383, Ordering::Relaxed);
        assert_eq!(
            control.editor_note_on(owner, 60, 100),
            Err(RuntimeError::EditorSequenceExhausted)
        );
        control.editor_note_off(owner, 60).unwrap();
        control.detach_editor_notes(owner);
        assert_eq!(
            control.editor_note_on(owner, 60, 100),
            Err(RuntimeError::EditorSequenceExhausted)
        );
        control.editor_note_on(owner, 61, 100).unwrap();
        control.editor_note_off(owner, 61).unwrap();
    }

    #[test]
    fn activation_readiness_returns_only_after_control_reclaims_retired_runtime() {
        let (mut control, mut audio) = gain_runtime();
        assert!(control.activation_available());
        let (_, replacement) = gain_runtime();
        assert!(control.offer(replacement.state.current).is_ok());
        assert!(
            !control.activation_available(),
            "pending publication owns the slot"
        );
        audio.process(&mut [0.0; 16], &mut [0.0; 16]).unwrap();
        assert!(
            !control.activation_available(),
            "audio consumption does not reclaim"
        );
        drop(control.reclaim().unwrap());
        assert!(control.activation_available());
    }

    #[test]
    fn parameter_snapshot_does_not_close_or_wait_for_callback_admission() {
        let (mut control, mut audio) = gain_runtime();
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
