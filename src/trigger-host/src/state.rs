//! Bounded native Trigger state codec. Runtime/session policy stays with their owner.

use dandrum_instrument_session::SessionError;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

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
    resources: [(); 0],
}

pub(crate) fn read(input: &mut impl Read) -> Result<SavedState, StateError> {
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
        if version != 3 || payload != 0 {
            return Err(StateError::Invalid);
        }
        if length > PORTABLE_METADATA_LIMIT {
            return Err(StateError::TooLarge);
        }
        let mut metadata = vec![0; length as usize];
        input.read_exact(&mut metadata).map_err(read_error)?;
        if input.take(1).read(&mut [0]).map_err(|_| StateError::Io)? != 0 {
            return Err(StateError::Invalid);
        }
        let state: PortableMetadata =
            serde_json::from_slice(&metadata).map_err(|_| StateError::Invalid)?;
        if state.product != PRODUCT_ID {
            return Err(StateError::Invalid);
        }
        return Ok(SavedState {
            version: 3,
            product: state.product,
            resource: RESOURCE_ID.into(),
            document: state.document,
            parameters: state.parameters,
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
    Ok(state)
}

fn read_error(error: std::io::Error) -> StateError {
    if error.kind() == std::io::ErrorKind::UnexpectedEof {
        StateError::Invalid
    } else {
        StateError::Io
    }
}

pub(crate) fn write(state: &SavedState, output: &mut impl Write) -> Result<(), StateError> {
    if state.version == 3 {
        let metadata = serde_json::to_vec(&PortableMetadata {
            product: state.product.clone(),
            document: state.document.clone(),
            parameters: state.parameters.clone(),
            resources: [],
        })
        .expect("validated state contains finite scalar values");
        if metadata.len() > PORTABLE_METADATA_LIMIT as usize {
            return Err(StateError::TooLarge);
        }
        output
            .write_all(PORTABLE_MAGIC)
            .and_then(|()| output.write_all(&3u32.to_le_bytes()))
            .and_then(|()| output.write_all(&(metadata.len() as u32).to_le_bytes()))
            .and_then(|()| output.write_all(&0u64.to_le_bytes()))
            .and_then(|()| output.write_all(&metadata))
            .map_err(|_| StateError::Io)
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
        assert_eq!(write(&state, &mut output), Err(StateError::TooLarge));
        assert_eq!(output, [7, 8, 9], "oversize emits no framing or body");
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
