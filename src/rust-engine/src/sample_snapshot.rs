//! Immutable source bytes and their decode, constructed only away from audio.
use crate::sample::LoadedSample;
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::sync::{Arc, Weak};

pub const MAX_ENCODED_SAMPLE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum SnapshotError {
    EncodedLimit,
    InvalidWav(String),
    Empty,
}

#[derive(Debug, PartialEq)]
pub struct SampleSnapshot {
    encoded: Arc<[u8]>,
    sample: Arc<LoadedSample>,
    content_sha256: String,
    resource_id: String,
}

/// Observes the encoded lease pinned by every prepared sample consumer.
/// Weak ownership never keeps a released resource charged by itself.
#[derive(Clone)]
pub struct SnapshotRetention {
    encoded: Weak<[u8]>,
    sample: Weak<LoadedSample>,
}
impl SnapshotRetention {
    pub fn is_live(&self) -> bool {
        self.encoded.strong_count() != 0
    }

    /// Reuse a source still pinned by prepared audio, even after a catalog
    /// result was released. Rebuild only the small owner, never the arrays.
    pub fn upgrade(&self) -> Option<Arc<SampleSnapshot>> {
        let sample = self.sample.upgrade()?;
        let encoded = self
            .encoded
            .upgrade()
            .expect("a snapshot sample pins its encoded source");
        Some(Arc::new(SampleSnapshot::from_decoded(encoded, sample)))
    }
}

impl SampleSnapshot {
    /// Lookup identity only; this does not validate or admit sample content.
    pub fn resource_id_for_encoded(encoded: &[u8]) -> Result<String, SnapshotError> {
        if encoded.len() > MAX_ENCODED_SAMPLE_BYTES {
            return Err(SnapshotError::EncodedLimit);
        }
        Ok(format!(
            "selected/{}.wav",
            content_hex(&Sha256::digest(encoded))
        ))
    }

    pub fn decode(encoded: Vec<u8>) -> Result<Self, SnapshotError> {
        if encoded.len() > MAX_ENCODED_SAMPLE_BYTES {
            return Err(SnapshotError::EncodedLimit);
        }
        // Compact before decoding: input Vec spare capacity must not become a
        // hidden retained allocation or overlap the decoder's large scratch.
        let encoded: Arc<[u8]> = encoded.into();
        let audio = crate::audio_loading::decode_pcm_wav_any_rate(&encoded)
            .map_err(SnapshotError::InvalidWav)?;
        if audio.frames.is_empty() {
            return Err(SnapshotError::Empty);
        }
        let mut sample = LoadedSample::from_loaded_audio(audio);
        sample.encoded_snapshot = Some(Arc::clone(&encoded));
        Ok(Self::from_decoded(encoded, Arc::new(sample)))
    }

    fn from_decoded(encoded: Arc<[u8]>, sample: Arc<LoadedSample>) -> Self {
        let content_sha256 = content_hex(sample.content_revision());
        let resource_id = format!("selected/{content_sha256}.wav");
        Self {
            encoded,
            sample,
            content_sha256,
            resource_id,
        }
    }

    pub fn encoded_bytes(&self) -> &[u8] {
        &self.encoded
    }
    pub fn content_sha256(&self) -> &str {
        &self.content_sha256
    }
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }
    pub fn sample(&self) -> &LoadedSample {
        &self.sample
    }
    pub fn decoded_capacity_bytes(&self) -> usize {
        self.sample.decoded_capacity_bytes()
    }
    pub fn retention(&self) -> SnapshotRetention {
        SnapshotRetention {
            encoded: Arc::downgrade(&self.encoded),
            sample: Arc::downgrade(&self.sample),
        }
    }
    pub(crate) fn shared_sample(&self) -> Arc<LoadedSample> {
        Arc::clone(&self.sample)
    }
}

fn content_hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(64);
    for byte in bytes {
        write!(text, "{byte:02x}").expect("writing to String is infallible");
    }
    text
}
