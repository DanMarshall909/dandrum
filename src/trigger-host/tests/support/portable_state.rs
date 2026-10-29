//! Test-only inspection of the published portable wire contract.
pub fn metadata(bytes: &[u8]) -> serde_json::Value {
    assert!(bytes.len() >= 24, "complete portable header");
    assert_eq!(&bytes[..8], b"DTRGSTAT");
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 3);
    let metadata = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let payload = usize::try_from(u64::from_le_bytes(bytes[16..24].try_into().unwrap())).unwrap();
    let end = 24_usize.checked_add(metadata).unwrap();
    assert_eq!(
        end.checked_add(payload).unwrap(),
        bytes.len(),
        "exact frame extent"
    );
    serde_json::from_slice(&bytes[24..end]).unwrap()
}
