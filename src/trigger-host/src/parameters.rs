//! Public parameter policy and its uniquely owned Trigger runtime endpoint.
//!
//! Only immutable descriptions are cached here. Actual scalar values and applied
//! UI command acknowledgements belong to the runtime's atomics. Commands use the
//! controller's protocol/session attachment namespace, never an arbitrary runtime.

use crate::protocol::{
    Admission, AdmittedRequest, EditorCommand, EditorConnection, EditorProtocol,
    MAX_SAFE_GENERATION, ProtocolError,
};
use crate::runtime::{
    PreparedRuntime, RuntimeAudio, RuntimeControl, RuntimeError, runtime_handoff,
};
use crate::{TriggerHost, TriggerHostConfiguration};
use dandrum_engine::kernel::document::load_kernel_patch_str;
use std::{collections::VecDeque, num::NonZeroU64, path::Path};

pub const MAX_AUTOMATION_EVENTS: usize = 1_024;

#[derive(Debug, PartialEq, Eq)]
pub enum ParameterError {
    Host(String),
    Runtime(RuntimeError),
    Protocol(ProtocolError),
    InvalidCapacity,
    InvalidSurface,
    IdentityExhausted,
    StaleGeneration,
    UnknownParameter,
    InvalidValue,
    QueueFull,
    UnsupportedCommand,
    GestureAlreadyActive,
    GestureConflict,
    GestureNotActive,
}

/// JavaScript-facing counters must be exact integers, including their next value.
pub fn document_generation(revision: u64) -> Result<u64, ParameterError> {
    revision
        .checked_add(1)
        .filter(|value| *value <= MAX_SAFE_GENERATION)
        .ok_or(ParameterError::IdentityExhausted)
}

#[derive(Clone, Debug, PartialEq)]
pub struct PublicParameter {
    pub id: String,
    pub root_index: usize,
    pub physical_min: f64,
    pub physical_max: f64,
    pub physical_default: f64,
}

impl PublicParameter {
    fn project(&self, physical: f64) -> ParameterValue {
        ParameterValue {
            id: self.id.clone(),
            name: self.id.clone(),
            value: (physical - self.physical_min) / (self.physical_max - self.physical_min),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParameterValue {
    pub id: String,
    pub name: String,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParameterState {
    pub generation: u64,
    pub sequence: u64,
    /// Whether the latest settled UI command was cancelled by reset rather
    /// than applied. A later host value retains this cause; a new UI write
    /// clears it when applied. It is not a history of all earlier cancellations.
    pub reset_cancelled: bool,
    pub parameters: Vec<ParameterValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParameterFeedback {
    pub generation: u64,
    pub sequence: u64,
    pub reset_cancelled: bool,
    pub frame_offset: u32,
    pub parameters: Vec<ParameterValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParameterReply {
    Accepted { generation: u64, sequence: u64 },
    State(ParameterState),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AutomationEvent {
    Begin { parameter_index: usize },
    Value { parameter_index: usize, value: f64 },
    End { parameter_index: usize },
}

pub struct ParameterController {
    host: TriggerHost,
    runtime: RuntimeControl,
    protocol: EditorProtocol,
    parameters: Vec<PublicParameter>,
    generation: u64,
    admitted_sequence: u64,
    automation: VecDeque<AutomationEvent>,
    automation_capacity: usize,
    gestures: Vec<Option<u64>>,
    reserved_ends: usize,
}

impl ParameterController {
    pub fn detach_editor(&mut self, attachment_id: &str) -> Result<(), ParameterError> {
        self.protocol
            .detach(self.host.session_mut(), attachment_id)
            .map_err(ParameterError::Protocol)?;
        let owner = attachment_id
            .parse::<u64>()
            .expect("protocol validated the retired attachment identity");
        for (index, gesture) in self.gestures.iter_mut().enumerate() {
            if *gesture == Some(owner) {
                self.automation.push_back(AutomationEvent::End {
                    parameter_index: index,
                });
                *gesture = None;
                self.reserved_ends -= 1;
            }
        }
        Ok(())
    }
    pub fn take_feedback(&mut self) -> Option<ParameterFeedback> {
        let (feedback, sequence, reset_cancelled) = self.runtime.take_sequenced_feedback()?;
        let parameter = self
            .parameters
            .iter()
            .find(|parameter| parameter.root_index == feedback.index)
            .expect("public aliases cover every prepared root control exactly once");
        Some(ParameterFeedback {
            generation: self.generation,
            sequence,
            reset_cancelled,
            frame_offset: feedback.frame_offset,
            parameters: vec![parameter.project(feedback.value)],
        })
    }
    pub fn open(
        root: impl AsRef<Path>,
        configuration: TriggerHostConfiguration,
        instance_id: NonZeroU64,
        event_capacity: usize,
        automation_capacity: usize,
    ) -> Result<(Self, RuntimeAudio), ParameterError> {
        if !(3..=MAX_AUTOMATION_EVENTS).contains(&automation_capacity) {
            return Err(ParameterError::InvalidCapacity);
        }
        let host = TriggerHost::open(root, configuration)
            .map_err(|error| ParameterError::Host(error.to_string()))?;
        let prepared = PreparedRuntime::new(host.session().prepared(), configuration)
            .map_err(ParameterError::Runtime)?;
        let (runtime, audio) =
            runtime_handoff(prepared, event_capacity).map_err(ParameterError::Runtime)?;
        // The freshly opened host has prepared this immutable cached document.
        let patch = load_kernel_patch_str(host.resources.document())
            .expect("fresh Trigger document was prepared");
        let mut parameters = Vec::new();
        for alias in patch.preset_surface().parameters() {
            let range = alias.control_default();
            let (Some(min), Some(max)) = (range.min(), range.max()) else {
                return Err(ParameterError::InvalidSurface);
            };
            if min >= max || !(min as f32).is_finite() || !(max as f32).is_finite() {
                return Err(ParameterError::InvalidSurface);
            }
            let root_index = runtime
                .parameter_index(alias.port_name())
                .expect("prepared alias names a mono root control");
            if parameters
                .iter()
                .any(|parameter: &PublicParameter| parameter.root_index == root_index)
            {
                return Err(ParameterError::InvalidSurface);
            }
            parameters.push(PublicParameter {
                id: alias.name().into(),
                root_index,
                physical_min: min,
                physical_max: max,
                physical_default: range.default(),
            });
        }
        if parameters.len()
            != host
                .session()
                .prepared()
                .root_port_metadata()
                .iter()
                .filter(|port| port.direction() == dandrum_engine::graph::PortDirection::Input)
                .count()
        {
            return Err(ParameterError::InvalidSurface);
        }
        let gestures = vec![None; parameters.len()];
        Ok((
            Self {
                host,
                runtime,
                protocol: EditorProtocol::new(instance_id),
                parameters,
                gestures,
                reserved_ends: 0,
                generation: document_generation(0)?,
                admitted_sequence: 0,
                automation: VecDeque::with_capacity(automation_capacity),
                automation_capacity,
            },
            audio,
        ))
    }

    pub fn parameters(&self) -> &[PublicParameter] {
        &self.parameters
    }
    pub fn session(&self) -> &crate::InstrumentSession {
        self.host.session()
    }
    pub fn attach_editor(&mut self) -> Result<EditorConnection, ParameterError> {
        self.protocol
            .attach(self.host.session_mut())
            .map_err(ParameterError::Protocol)
    }
    pub fn admit(&mut self, message: &str) -> Result<Admission, ParameterError> {
        self.protocol
            .admit(self.host.session(), message)
            .map_err(ParameterError::Protocol)
    }

    pub fn apply(&mut self, request: &AdmittedRequest) -> Result<ParameterReply, ParameterError> {
        let snapshot = self
            .host
            .session()
            .snapshot(&request.attachment)
            .map_err(|_| ParameterError::Protocol(ProtocolError::StaleAttachment))?;
        let generation = document_generation(snapshot.revision)?;
        match &request.command {
            EditorCommand::GetParameterState => {
                return Ok(ParameterReply::State(self.parameter_state()));
            }
            EditorCommand::BeginGesture {
                parameter_id,
                generation: expected,
            } => {
                let index = self.resolve(parameter_id, *expected, generation)?;
                if self.gesture_owned(index, request.attachment_id)? {
                    return Err(ParameterError::GestureAlreadyActive);
                }
                // One slot publishes Begin; the other remains reserved until
                // this owner ends/detaches, even while other values fill the queue.
                if self.automation_slots() < 2 {
                    return Err(ParameterError::QueueFull);
                }
                self.automation.push_back(AutomationEvent::Begin {
                    parameter_index: index,
                });
                self.gestures[index] = Some(request.attachment_id);
                self.reserved_ends += 1;
            }
            EditorCommand::EndGesture {
                parameter_id,
                generation: expected,
            } => {
                let index = self.resolve(parameter_id, *expected, generation)?;
                if !self.gesture_owned(index, request.attachment_id)? {
                    return Err(ParameterError::GestureNotActive);
                }
                self.automation.push_back(AutomationEvent::End {
                    parameter_index: index,
                });
                self.gestures[index] = None;
                self.reserved_ends -= 1;
            }
            EditorCommand::SetParameter {
                parameter_id,
                value,
                generation: expected,
            } => {
                let index = self.resolve(parameter_id, *expected, generation)?;
                if !value.is_finite() || !(0.0..=1.0).contains(value) {
                    return Err(ParameterError::InvalidValue);
                }
                let active = self.gesture_owned(index, request.attachment_id)?;
                let sequence = document_generation(self.admitted_sequence)?;
                if self.automation_slots() < if active { 1 } else { 3 }
                    || self.runtime.event_slots() == 0
                {
                    return Err(ParameterError::QueueFull);
                }
                let parameter = &self.parameters[index];
                // Rounding can otherwise put normalized endpoints just outside
                // an accepted nonbinary physical interval (for example -0.1..0.3).
                let physical = (parameter.physical_min
                    + value * (parameter.physical_max - parameter.physical_min))
                    .clamp(parameter.physical_min, parameter.physical_max);
                // Both producers are exclusively owned here. The audio consumer
                // can only free runtime slots, and no caller can fill our main
                // queue between this check and publication of the complete value.
                self.runtime
                    .send_parameter(parameter.root_index, physical, sequence)
                    .map_err(ParameterError::Runtime)?;
                if !active {
                    self.automation.push_back(AutomationEvent::Begin {
                        parameter_index: index,
                    });
                }
                self.automation.push_back(AutomationEvent::Value {
                    parameter_index: index,
                    value: physical,
                });
                if !active {
                    self.automation.push_back(AutomationEvent::End {
                        parameter_index: index,
                    });
                }
                self.admitted_sequence = sequence;
            }
            _ => return Err(ParameterError::UnsupportedCommand),
        }
        Ok(ParameterReply::Accepted {
            generation,
            sequence: self.admitted_sequence,
        })
    }

    fn resolve(&self, id: &str, expected: u64, generation: u64) -> Result<usize, ParameterError> {
        if expected != generation {
            return Err(ParameterError::StaleGeneration);
        }
        self.parameters
            .iter()
            .position(|parameter| parameter.id == id)
            .ok_or(ParameterError::UnknownParameter)
    }

    fn gesture_owned(&self, index: usize, owner: u64) -> Result<bool, ParameterError> {
        match self.gestures[index] {
            None => Ok(false),
            Some(current) if current == owner => Ok(true),
            _ => Err(ParameterError::GestureConflict),
        }
    }

    fn automation_slots(&self) -> usize {
        self.automation_capacity - self.automation.len() - self.reserved_ends
    }

    pub fn parameter_state(&self) -> ParameterState {
        let (sequence, reset_cancelled, physical) = self.runtime.snapshot_parameters();
        ParameterState {
            generation: self.generation,
            sequence,
            reset_cancelled,
            parameters: self
                .parameters
                .iter()
                .map(|parameter| parameter.project(physical[parameter.root_index]))
                .collect(),
        }
    }

    /// A format adapter acknowledges only after its host accepts this event.
    pub fn peek_automation(&self) -> Option<AutomationEvent> {
        self.automation.front().copied()
    }
    pub fn acknowledge_automation(&mut self) -> bool {
        self.automation.pop_front().is_some()
    }
}
