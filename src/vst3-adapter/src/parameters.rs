//! Version1 explicit VST3 IDs over the shared named physical-value policy.
use dandrum_trigger_host::parameters::{ParameterError, PublicParameter};
pub(crate) const IDS: [(u32, &str); 23] = [
    (0x1001, "drums.pitch_ratio"),
    (0x1002, "drums.start_offset"),
    (0x1003, "drums.level"),
    (0x1004, "drums.pan"),
    (0x1005, "drums.variation"),
    (0x2001, "drums.kick.pitch_ratio"),
    (0x2002, "drums.kick.start_offset"),
    (0x2003, "drums.kick.level"),
    (0x2004, "drums.kick.pan"),
    (0x3001, "drums.snare.pitch_ratio"),
    (0x3002, "drums.snare.start_offset"),
    (0x3003, "drums.snare.level"),
    (0x3004, "drums.snare.pan"),
    (0x3005, "drums.snare.variation"),
    (0x4001, "drums.closed_hat.pitch_ratio"),
    (0x4002, "drums.closed_hat.start_offset"),
    (0x4003, "drums.closed_hat.level"),
    (0x4004, "drums.closed_hat.pan"),
    (0x5001, "drums.open_hat.pitch_ratio"),
    (0x5002, "drums.open_hat.start_offset"),
    (0x5003, "drums.open_hat.level"),
    (0x5004, "drums.open_hat.pan"),
    (0x5005, "drums.open_hat.variation"),
];
pub(crate) fn index(id: u32) -> Option<usize> {
    IDS.iter().position(|(candidate, _)| *candidate == id)
}
pub(crate) fn ordered(source: &[PublicParameter]) -> Result<Vec<PublicParameter>, ParameterError> {
    if source.len() != IDS.len() {
        return Err(ParameterError::InvalidSurface);
    }
    IDS.iter()
        .map(|(_, name)| {
            source
                .iter()
                .find(|p| p.id == *name)
                .cloned()
                .ok_or(ParameterError::InvalidSurface)
        })
        .collect()
}
