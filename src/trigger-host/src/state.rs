//! Bounded native Trigger state codec. Runtime/session policy stays with their owner.

use dandrum_engine::sample_snapshot::SampleSnapshot;
use dandrum_instrument_session::SessionError;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::sync::Arc;

pub const STATE_LIMIT: u64 = 65_536;
const PORTABLE_MAGIC: &[u8; 8] = b"DTRGSTAT";
const PORTABLE_METADATA_LIMIT: u32 = 1_048_576;
pub const PRODUCT_ID: &str = "org.dandrum.trigger";
pub const RESOURCE_ID: &str = "dandrum.trigger.advanced-drums.v1";

#[derive(Debug, PartialEq, Eq)]
pub enum StateError {
    Io,
    Invalid,
    TooLarge,
    Busy,
    IdentityExhausted,
    Session(SessionError),
    Runtime(crate::runtime::RuntimeError),
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedParameter {
    pub id: String,
    pub value: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedState {
    pub version: u32,
    pub product: String,
    pub resource: String,
    pub document: String,
    pub parameters: Vec<SavedParameter>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PortableMetadata {
    product: String,
    document: String,
    parameters: Vec<SavedParameter>,
    resources: Vec<ManifestResource>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManifestResource {
    pub path: String,
    sha256: String,
    encoded_bytes: u64,
}
/// Bounded framing/metadata only; native admission and sample content are pending.
pub struct StateHeader {
    pub(crate) state: SavedState,
    pub(crate) resources: Vec<ManifestResource>,
    payload_bytes: usize,
}

/// Owned foreign input, still subject to current native admission and decoding.
pub struct BufferedState {
    pub(crate) header: StateHeader,
    pub(crate) payload: Vec<u8>,
}
impl StateHeader {
    /// Read only the validated payload extent, outside any controller borrow.
    pub fn buffer_payload(self, input: &mut impl Read) -> Result<BufferedState, StateError> {
        let mut payload = vec![0; self.payload_bytes];
        if self.state.version == 3 {
            input.read_exact(&mut payload).map_err(read_error)?;
            if input.take(1).read(&mut [0]).map_err(|_| StateError::Io)? != 0 {
                return Err(StateError::Invalid);
            }
        }
        Ok(BufferedState {
            header: self,
            payload,
        })
    }

    pub(crate) fn read_samples(
        &self,
        input: &mut impl Read,
        catalog: &mut crate::samples::SampleCatalog,
    ) -> Result<Vec<Arc<SampleSnapshot>>, StateError> {
        let descriptors = self
            .resources
            .iter()
            .map(|r| {
                (
                    format!("selected/{}.wav", r.sha256),
                    r.encoded_bytes as usize,
                )
            })
            .collect::<Vec<_>>();
        restore_samples(input, catalog, &descriptors)
    }
}

/// Complete bounded payload input is shared by restore and packaged export.
pub(crate) fn restore_samples(
    input: &mut impl Read,
    catalog: &mut crate::samples::SampleCatalog,
    descriptors: &[(String, usize)],
) -> Result<Vec<Arc<SampleSnapshot>>, StateError> {
    let samples = catalog
        .restore(input, descriptors)
        .map_err(|error| match error {
            crate::samples::RestoreError::Input(kind) => {
                if kind == std::io::ErrorKind::UnexpectedEof {
                    StateError::Invalid
                } else {
                    StateError::Io
                }
            }
            crate::samples::RestoreError::Sample(crate::samples::SampleError::Busy) => {
                StateError::Busy
            }
            crate::samples::RestoreError::Sample(
                crate::samples::SampleError::EncodedLimit
                | crate::samples::SampleError::DecodedLimit
                | crate::samples::SampleError::ResourceLimit,
            ) => StateError::TooLarge,
            crate::samples::RestoreError::Sample(_) => StateError::Invalid,
        })?;
    if input.take(1).read(&mut [0]).map_err(|_| StateError::Io)? != 0 {
        return Err(StateError::Invalid);
    }
    Ok(samples)
}

fn validate_manifest(resources: &[ManifestResource], payload: u64) -> Result<(), StateError> {
    if resources.len() > crate::samples::MAX_RESOURCE_COUNT {
        return Err(StateError::TooLarge);
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut total = 0u64;
    for resource in resources {
        if resource.sha256.len() != 64
            || !resource
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || (resource.path != "assets/advanced-drums.wav"
                && resource.path != format!("selected/{}.wav", resource.sha256))
            || !seen.insert(&resource.path)
            || resource.encoded_bytes == 0
        {
            return Err(StateError::Invalid);
        }
        if resource.encoded_bytes > crate::samples::MAX_ENCODED_SAMPLE_BYTES as u64 {
            return Err(StateError::TooLarge);
        }
        total = total
            .checked_add(resource.encoded_bytes)
            .ok_or(StateError::TooLarge)?;
        if total > crate::samples::MAX_RETAINED_ENCODED_BYTES as u64 {
            return Err(StateError::TooLarge);
        }
    }
    if total != payload {
        return Err(StateError::Invalid);
    }
    Ok(())
}

/// Parse the existing bounded v2 input or just the v3 framing and metadata.
pub fn read_header(input: &mut impl Read) -> Result<StateHeader, StateError> {
    let mut bytes = Vec::new();
    input
        .take(8)
        .read_to_end(&mut bytes)
        .map_err(|_| StateError::Io)?;
    if bytes == PORTABLE_MAGIC {
        let mut framing = [0; 16];
        input.read_exact(&mut framing).map_err(read_error)?;
        let version = u32::from_le_bytes(framing[..4].try_into().expect("four version bytes"));
        let length = u32::from_le_bytes(framing[4..8].try_into().expect("four length bytes"));
        let payload = u64::from_le_bytes(framing[8..].try_into().expect("eight payload bytes"));
        if version != 3 {
            return Err(StateError::Invalid);
        }
        if length > PORTABLE_METADATA_LIMIT {
            return Err(StateError::TooLarge);
        }
        let mut metadata = vec![0; length as usize];
        input.read_exact(&mut metadata).map_err(read_error)?;
        let state: PortableMetadata =
            serde_json::from_slice(&metadata).map_err(|_| StateError::Invalid)?;
        if state.product != PRODUCT_ID {
            return Err(StateError::Invalid);
        }
        validate_manifest(&state.resources, payload)?;
        return Ok(StateHeader {
            state: SavedState {
                version: 3,
                product: state.product,
                resource: RESOURCE_ID.into(),
                document: state.document,
                parameters: state.parameters,
            },
            resources: state.resources,
            payload_bytes: payload as usize,
        });
    }
    let remaining = STATE_LIMIT + 1 - bytes.len() as u64;
    input
        .take(remaining)
        .read_to_end(&mut bytes)
        .map_err(|_| StateError::Io)?;
    if bytes.len() as u64 > STATE_LIMIT {
        return Err(StateError::TooLarge);
    }
    let state: SavedState = serde_json::from_slice(&bytes).map_err(|_| StateError::Invalid)?;
    if state.version != 2 || state.product != PRODUCT_ID || state.resource != RESOURCE_ID {
        return Err(StateError::Invalid);
    }
    Ok(StateHeader {
        state,
        resources: Vec::new(),
        payload_bytes: 0,
    })
}

fn read_error(error: std::io::Error) -> StateError {
    if error.kind() == std::io::ErrorKind::UnexpectedEof {
        StateError::Invalid
    } else {
        StateError::Io
    }
}

pub(crate) fn write(
    state: &SavedState,
    samples: &[(String, Arc<SampleSnapshot>)],
    output: &mut impl Write,
) -> Result<(), StateError> {
    if state.version == 3 {
        let resources = samples
            .iter()
            .map(|(path, sample)| ManifestResource {
                path: path.clone(),
                sha256: sample.content_sha256().to_owned(),
                encoded_bytes: sample.encoded_bytes().len() as u64,
            })
            .collect::<Vec<_>>();
        let payload = resources
            .iter()
            .try_fold(0u64, |total, r| total.checked_add(r.encoded_bytes))
            .ok_or(StateError::TooLarge)?;
        validate_manifest(&resources, payload)?;
        let metadata = serde_json::to_vec(&PortableMetadata {
            product: state.product.clone(),
            document: state.document.clone(),
            parameters: state.parameters.clone(),
            resources,
        })
        .expect("validated state contains finite scalar values");
        if metadata.len() > PORTABLE_METADATA_LIMIT as usize {
            return Err(StateError::TooLarge);
        }
        output
            .write_all(PORTABLE_MAGIC)
            .and_then(|()| output.write_all(&3u32.to_le_bytes()))
            .and_then(|()| output.write_all(&(metadata.len() as u32).to_le_bytes()))
            .and_then(|()| output.write_all(&payload.to_le_bytes()))
            .and_then(|()| output.write_all(&metadata))
            .map_err(|_| StateError::Io)?;
        for (_, sample) in samples {
            output
                .write_all(sample.encoded_bytes())
                .map_err(|_| StateError::Io)?;
        }
        Ok(())
    } else {
        let bytes =
            serde_json::to_vec(state).expect("validated state contains finite scalar values");
        if bytes.len() as u64 > STATE_LIMIT {
            return Err(StateError::TooLarge);
        }
        output.write_all(&bytes).map_err(|_| StateError::Io)
    }
}

/// A saved document may remap the packaged kit, but cannot expand the product's
/// resource identity into arbitrary paths, definitions, or sample sources.
fn validate_resources(
    patch: &dandrum_engine::kernel::document::KernelPatch,
) -> Result<(), StateError> {
    use dandrum_engine::kernel::{
        ResourceKind, ResourceOrigin, ResourceRef, StaticArg, StaticValue,
    };
    let allowed = |resource: &ResourceRef| {
        resource.kind() == ResourceKind::Sample
            && resource.origin() == &ResourceOrigin::Document
            && resource.path() == std::path::Path::new("assets/advanced-drums.wav")
    };
    let sources = &patch.sample_assets().sample_sources;
    if sources.len() != 1 || sources[0].id != "drums" || !allowed(&sources[0].resource) {
        return Err(StateError::Invalid);
    }
    for definition in std::iter::once(patch.root()).chain(patch.registry().definitions()) {
        for parameter in definition.static_params() {
            if let Some(StaticValue::Resource(resource)) = parameter.default()
                && !allowed(resource)
            {
                return Err(StateError::Invalid);
            }
        }
        for node in definition.nodes() {
            for argument in node.static_args().values() {
                if let StaticArg::Literal(StaticValue::Resource(resource)) = argument
                    && !allowed(resource)
                {
                    return Err(StateError::Invalid);
                }
            }
        }
    }
    Ok(())
}

impl SavedState {
    /// Private to this host crate; the controller owns both arguments together.
    pub(crate) fn values_for(
        &self,
        table: &[crate::parameters::PublicParameter],
        runtime: &crate::runtime::RuntimeControl,
    ) -> Result<Vec<f64>, StateError> {
        let patch = dandrum_engine::kernel::document::load_kernel_patch_str(&self.document)
            .map_err(|_| StateError::Invalid)?;
        if self.version == 2 {
            validate_resources(&patch)?;
        }
        let aliases = patch.preset_surface().parameters();
        if aliases.len() != table.len() || self.parameters.len() != table.len() {
            return Err(StateError::Invalid);
        }
        let mut values = vec![0.0; table.len()];
        for descriptor in table {
            let mut matching = aliases.iter().filter(|alias| alias.name() == descriptor.id);
            let alias = matching.next().ok_or(StateError::Invalid)?;
            let range = alias.control_default();
            if matching.next().is_some()
                || runtime.parameter_index(alias.port_name()) != Some(descriptor.root_index)
                || range.min() != Some(descriptor.physical_min)
                || range.max() != Some(descriptor.physical_max)
                || (self.version == 2 && range.default() != descriptor.physical_default)
            {
                return Err(StateError::Invalid);
            }
            let mut matches = self
                .parameters
                .iter()
                .filter(|parameter| parameter.id == descriptor.id);
            let value = matches.next().ok_or(StateError::Invalid)?.value;
            if matches.next().is_some()
                || !value.is_finite()
                || !(descriptor.physical_min..=descriptor.physical_max).contains(&value)
            {
                return Err(StateError::Invalid);
            }
            values[descriptor.root_index] = value;
        }
        Ok(values)
    }
}

/// Restore increments the accepted revision once; its public generation is
/// that new revision plus one. Check both additions before any publication.
pub(crate) fn restored_generation(revision: u64) -> Result<u64, StateError> {
    revision
        .checked_add(2)
        .filter(|generation| *generation <= crate::protocol::MAX_SAFE_GENERATION)
        .ok_or(StateError::IdentityExhausted)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_metadata_budget_refuses_before_writing_to_stream() {
        let state = SavedState {
            version: 3,
            product: PRODUCT_ID.into(),
            resource: RESOURCE_ID.into(),
            document: "x".repeat(PORTABLE_METADATA_LIMIT as usize),
            parameters: Vec::new(),
        };
        let mut output = vec![7, 8, 9];
        assert_eq!(write(&state, &[], &mut output), Err(StateError::TooLarge));
        assert_eq!(output, [7, 8, 9], "oversize emits no framing or body");
    }
    #[test]
    fn selected_payload_writes_retry_short_progress_and_report_stream_failure() {
        let mut wav = Vec::new();
        dandrum_engine::wav::write_wav_stereo_i16(&mut wav, 32_000, &[-0.75, -0.5], &[-0.25, -0.5])
            .unwrap();
        let sample = Arc::new(SampleSnapshot::decode(wav.clone()).unwrap());
        let state = SavedState {
            version: 3,
            product: PRODUCT_ID.into(),
            resource: RESOURCE_ID.into(),
            document: String::new(),
            parameters: Vec::new(),
        };
        let resource = (sample.resource_id().to_owned(), sample);
        let mut complete = Vec::new();
        write(&state, std::slice::from_ref(&resource), &mut complete).unwrap();
        let start = 24 + u32::from_le_bytes(complete[12..16].try_into().unwrap()) as usize;
        assert_eq!(&complete[start..], wav);
        struct Short(Vec<u8>);
        impl Write for Short {
            fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
                let count = input.len().min(1);
                self.0.extend_from_slice(&input[..count]);
                Ok(count)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut short = Short(Vec::new());
        write(&state, std::slice::from_ref(&resource), &mut short).unwrap();
        assert_eq!(short.0, complete);
        struct Fault {
            bytes: Vec<u8>,
            remaining: usize,
        }
        impl Write for Fault {
            fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
                if self.remaining == 0 {
                    return Err(std::io::Error::other("payload write failed"));
                }
                let count = input.len().min(self.remaining);
                self.bytes.extend_from_slice(&input[..count]);
                self.remaining -= count;
                Ok(count)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        for offset in [start, start + 1, complete.len() - 1] {
            let mut output = Fault {
                bytes: Vec::new(),
                remaining: offset,
            };
            assert_eq!(
                write(&state, std::slice::from_ref(&resource), &mut output),
                Err(StateError::Io)
            );
            assert_eq!(output.bytes, complete[..offset]);
        }
    }
    #[test]
    fn restored_generation_is_checked_before_publication_can_wrap_or_lose_javascript_precision() {
        assert_eq!(restored_generation(0), Ok(2));
        assert_eq!(
            restored_generation(crate::protocol::MAX_SAFE_GENERATION - 2),
            Ok(crate::protocol::MAX_SAFE_GENERATION)
        );
        assert_eq!(
            restored_generation(crate::protocol::MAX_SAFE_GENERATION - 1),
            Err(StateError::IdentityExhausted)
        );
        assert_eq!(
            restored_generation(u64::MAX),
            Err(StateError::IdentityExhausted)
        );
    }
}
