//! Host-independent accepted instrument state and authoring history.
use dandrum_engine::kernel::document::load_kernel_patch_str;
use dandrum_engine::patch::RenderSettings;
use dandrum_engine::preparation::{
    PreparationContext, PreparedKernelInstrument, prepare_kernel_patch_with_context,
};
use std::sync::{Arc, Weak};

const ATTACHMENT_LIMIT: usize = 16;

/// An opaque lifetime identity, never a serialized pointer or platform handle.
#[derive(Clone, Debug)]
pub struct SessionIdentity(Arc<()>);
impl PartialEq for SessionIdentity {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for SessionIdentity {}

/// An editor connection; retaining a copy cannot reverse explicit retirement.
#[derive(Clone, Debug)]
pub struct Attachment {
    owner: SessionIdentity,
    marker: Arc<()>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub instance: SessionIdentity,
    pub revision: u64,
    pub yaml: String,
    pub undo_len: usize,
    pub redo_len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionError {
    InvalidDocument(String),
    PreparationFailed(String),
    InvalidAttachment,
    AttachmentLimit,
    InvalidMapping,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// A musical intent addressed by native identities; no client inverse is accepted.
pub struct SetZoneKeyRange {
    pub map_id: String,
    pub zone_id: String,
    pub key_range: [u8; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Native before/after data for the future instrument-owned journal.
pub struct ZoneKeyRangeChange {
    pub map_id: String,
    pub zone_id: String,
    pub before: [u8; 2],
    pub after: [u8; 2],
}

/// An uncommitted candidate. Its creation never installs a runtime or edits history.
pub struct PreparedMapping {
    yaml: String,
    prepared: PreparedKernelInstrument,
    change: ZoneKeyRangeChange,
}
impl PreparedMapping {
    pub fn yaml(&self) -> &str {
        &self.yaml
    }
    pub fn prepared(&self) -> &PreparedKernelInstrument {
        &self.prepared
    }
    pub fn change(&self) -> &ZoneKeyRangeChange {
        &self.change
    }
}

/// Off-audio owner. Renderers consume the prepared engine directly.
pub struct InstrumentSession {
    yaml: String,
    prepared: PreparedKernelInstrument,
    identity: SessionIdentity,
    attachments: Vec<Weak<()>>,
    context: PreparationContext,
    settings: RenderSettings,
}
impl InstrumentSession {
    /// Prepare off audio without publishing or changing accepted state.
    pub fn prepare_mapping(
        &self,
        attachment: &Attachment,
        intent: SetZoneKeyRange,
    ) -> Result<PreparedMapping, SessionError> {
        self.attachment_index(attachment)
            .ok_or(SessionError::InvalidAttachment)?;
        if intent.key_range[0] > intent.key_range[1] || intent.key_range[1] > 127 {
            return Err(SessionError::InvalidMapping);
        }
        // Only the native accepted document supplies identities and inverse data.
        let mut document: serde_yaml::Value =
            serde_yaml::from_str(&self.yaml).expect("accepted YAML remains valid");
        let maps = document["assets"]["sample_maps"]
            .as_sequence_mut()
            .ok_or(SessionError::InvalidMapping)?;
        let map = maps
            .iter_mut()
            .find(|map| map["id"].as_str() == Some(&intent.map_id))
            .ok_or(SessionError::InvalidMapping)?;
        let zones = map["zones"]
            .as_sequence_mut()
            .expect("accepted sample map has zones");
        let mut matching = zones
            .iter_mut()
            .filter(|zone| zone["id"].as_str() == Some(&intent.zone_id));
        let zone = matching.next().ok_or(SessionError::InvalidMapping)?;
        if matching.next().is_some() {
            return Err(SessionError::InvalidMapping);
        }
        let before = serde_yaml::from_value(zone["key_range"].clone())
            .expect("accepted zone has a typed key range");
        zone["key_range"] = serde_yaml::Value::Sequence(
            intent
                .key_range
                .into_iter()
                .map(|note| note.into())
                .collect(),
        );
        // Value serialization preserves authored values; presentation is normalized.
        let yaml = serde_yaml::to_string(&document).expect("YAML values serialize");
        let prepared = Self::prepare_document(&yaml, &self.context, &self.settings)?;
        Ok(PreparedMapping {
            yaml,
            prepared,
            change: ZoneKeyRangeChange {
                map_id: intent.map_id,
                zone_id: intent.zone_id,
                before,
                after: intent.key_range,
            },
        })
    }

    pub fn new(
        yaml: &str,
        context: PreparationContext,
        settings: RenderSettings,
    ) -> Result<Self, SessionError> {
        let prepared = Self::prepare_document(yaml, &context, &settings)?;
        Ok(Self {
            yaml: yaml.into(),
            prepared,
            identity: SessionIdentity(Arc::new(())),
            attachments: Vec::with_capacity(ATTACHMENT_LIMIT),
            context,
            settings,
        })
    }

    pub fn attach(&mut self) -> Result<(Attachment, SessionSnapshot), SessionError> {
        self.attachments.retain(|marker| marker.strong_count() != 0);
        if self.attachments.len() == ATTACHMENT_LIMIT {
            return Err(SessionError::AttachmentLimit);
        }
        let attachment = Attachment {
            owner: self.identity.clone(),
            marker: Arc::new(()),
        };
        self.attachments.push(Arc::downgrade(&attachment.marker));
        Ok((attachment, self.accepted_snapshot()))
    }

    pub fn detach(&mut self, attachment: &Attachment) -> Result<(), SessionError> {
        let index = self
            .attachment_index(attachment)
            .ok_or(SessionError::InvalidAttachment)?;
        self.attachments.swap_remove(index);
        Ok(())
    }

    pub fn snapshot(&self, attachment: &Attachment) -> Result<SessionSnapshot, SessionError> {
        self.attachment_index(attachment)
            .ok_or(SessionError::InvalidAttachment)?;
        Ok(self.accepted_snapshot())
    }

    pub fn prepared(&self) -> &PreparedKernelInstrument {
        &self.prepared
    }

    fn prepare_document(
        yaml: &str,
        context: &PreparationContext,
        settings: &RenderSettings,
    ) -> Result<PreparedKernelInstrument, SessionError> {
        let patch = load_kernel_patch_str(yaml)
            .map_err(|error| SessionError::InvalidDocument(error.to_string()))?;
        prepare_kernel_patch_with_context(&patch, settings, context)
            .map_err(|error| SessionError::PreparationFailed(error.to_string()))
    }

    fn attachment_index(&self, attachment: &Attachment) -> Option<usize> {
        if self.identity != attachment.owner {
            return None;
        }
        let marker = Arc::downgrade(&attachment.marker);
        self.attachments
            .iter()
            .position(|active| active.ptr_eq(&marker))
    }

    fn accepted_snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            instance: self.identity.clone(),
            revision: 0,
            yaml: self.yaml.clone(),
            undo_len: 0,
            redo_len: 0,
        }
    }
}
