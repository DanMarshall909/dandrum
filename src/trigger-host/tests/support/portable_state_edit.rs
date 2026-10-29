//! Change only metadata while retaining the original bounded test payload.

pub fn edit(bytes: &[u8], update: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let mut metadata = super::portable_state::metadata(bytes);
    update(&mut metadata);
    let encoded = serde_json::to_vec(&metadata).unwrap();
    let original_length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let mut result = bytes[..12].to_vec();
    result.extend(u32::try_from(encoded.len()).unwrap().to_le_bytes());
    result.extend(&bytes[16..24]);
    result.extend(encoded);
    result.extend(&bytes[24 + original_length..]);
    result
}
