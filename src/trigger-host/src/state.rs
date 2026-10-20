//! Bounded native Trigger state codec. Runtime/session policy stays with their owner.

use dandrum_instrument_session::SessionError;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

pub const STATE_LIMIT: u64 = 65_536;
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

#[derive(Serialize, Deserialize)]
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

pub(crate) fn read(input: &mut impl Read) -> Result<SavedState, StateError> {
    let mut bytes = Vec::new();
    input
        .take(STATE_LIMIT + 1)
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

pub(crate) fn write(state: &SavedState, output: &mut impl Write) -> Result<(), StateError> {
    let bytes =
        serde_json::to_vec(state).expect("state contains only strings and finite scalar numbers");
    if bytes.len() as u64 > STATE_LIMIT {
        return Err(StateError::TooLarge);
    }
    output.write_all(&bytes).map_err(|_| StateError::Io)
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
        validate_resources(&patch)?;
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
                || range.default() != descriptor.physical_default
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
