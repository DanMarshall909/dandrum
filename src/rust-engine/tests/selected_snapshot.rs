use dandrum_engine::{
    kernel::{ResourceKind, ResourceOrigin, ResourceRef},
    preparation::{PreparationContext, ResourceResolver},
    sample_snapshot::{MAX_ENCODED_SAMPLE_BYTES, SampleSnapshot, SnapshotError},
};
use std::sync::Arc;

const WAV: &[u8] = &[
    82, 73, 70, 70, 52, 0, 0, 0, 87, 65, 86, 69, 102, 109, 116, 32, 16, 0, 0, 0, 1, 0, 2, 0, 0,
    125, 0, 0, 0, 244, 1, 0, 4, 0, 16, 0, 100, 97, 116, 97, 16, 0, 0, 0, 0, 160, 0, 64, 0, 32, 0,
    224, 0, 192, 0, 16, 0, 64, 0, 160,
];
const CONTENT: &str = "2e484972f38876c961b3c23fb4952487dce3e18501c7fb0efd1b8cb7011734d5";

#[test]
fn ordinary_files_in_a_selected_directory_keep_the_existing_resource_contract() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("selected")).unwrap();
    let context = PreparationContext::new(root.path(), 32_000);
    for name in [
        "ordinary.wav".to_owned(),
        "no-extension".to_owned(),
        format!("{}.wav", "A".repeat(64)),
    ] {
        let path = format!("selected/{name}");
        std::fs::write(root.path().join(&path), WAV).unwrap();
        let resource = ResourceRef::new(ResourceKind::Sample, &path, ResourceOrigin::Document);
        let resolved = ResourceResolver::new(&context)
            .resolve(&resource)
            .expect("only canonical content-addressed IDs reserve selected authority");
        assert_eq!(resolved.sample().source_sample(0, 0), Some(-0.75));
        std::fs::remove_file(root.path().join(&path)).unwrap();
        assert!(
            ResourceResolver::new(&context).resolve(&resource).is_err(),
            "ordinary files retain reread/rollback semantics"
        );
    }
}

#[test]
fn owned_snapshot_metadata_and_channels_come_from_the_retained_bytes() {
    let snapshot = SampleSnapshot::decode(WAV.to_vec()).unwrap();
    assert_eq!(snapshot.encoded_bytes(), WAV);
    assert_eq!(snapshot.content_sha256(), CONTENT);
    assert_eq!(snapshot.resource_id(), format!("selected/{CONTENT}.wav"));
    assert_eq!(
        SampleSnapshot::resource_id_for_encoded(WAV).unwrap(),
        snapshot.resource_id()
    );
    assert_eq!(snapshot.sample().sample_rate_hz(), 32_000);
    assert_eq!(snapshot.sample().source_channel_count(), 2);
    assert_eq!(snapshot.sample().frame_count(), 4);
    assert_eq!(snapshot.decoded_capacity_bytes(), 32);
    assert_eq!(snapshot.sample().source_sample(0, 0), Some(-0.75));
    assert_eq!(snapshot.sample().source_sample(1, 0), Some(0.5));
    assert_eq!(snapshot.sample().source_sample(0, 3), Some(0.5));
    assert_eq!(snapshot.sample().source_sample(1, 3), Some(-0.75));
    assert_eq!(snapshot.sample().frames(), &[-0.125, 0.0, -0.1875, -0.125]);
}

#[test]
fn selected_authority_survives_original_and_root_loss_without_filesystem_fallback() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("original.wav");
    std::fs::write(&original, WAV).unwrap();
    let snapshot = Arc::new(SampleSnapshot::decode(std::fs::read(&original).unwrap()).unwrap());
    let retained = snapshot.retention();
    let resource = ResourceRef::new(
        ResourceKind::Sample,
        snapshot.resource_id(),
        ResourceOrigin::Document,
    );
    let context =
        PreparationContext::new(root.path(), 32_000).with_selected_samples([Arc::clone(&snapshot)]);
    let mut resolver = ResourceResolver::new(&context);
    std::fs::write(&original, b"not the admitted sample").unwrap();
    root.close().unwrap();
    let resolved = resolver.resolve(&resource).unwrap();
    assert_eq!(resolver.resolve(&resource).unwrap(), resolved);
    assert_eq!(resolver.loaded_resource_count(), 1);
    assert_eq!(resolved.sample().source_sample(0, 0), Some(-0.75));
    assert_eq!(resolved.sample().source_sample(1, 3), Some(-0.75));
    let precise_context = context
        .clone()
        .with_sample_rate(dandrum_engine::render_settings::SampleRate::new(32_000.0).unwrap());
    assert_eq!(
        ResourceResolver::new(&precise_context)
            .resolve(&resource)
            .unwrap(),
        resolved
    );
    drop(precise_context);
    drop(snapshot);
    drop(resolver);
    drop(context);
    assert!(
        retained.is_live(),
        "resolved samples must pin the encoded budget charge"
    );
    let reacquired = retained
        .upgrade()
        .expect("a runtime-pinned source remains reusable");
    assert_eq!(reacquired.encoded_bytes(), WAV);
    assert_eq!(reacquired.sample().source_sample(0, 0), Some(-0.75));
    drop(resolved);
    assert!(retained.is_live());
    drop(reacquired);
    assert!(
        !retained.is_live(),
        "last sample owner releases its encoded charge"
    );
    assert!(retained.upgrade().is_none());

    let decoy_root = tempfile::tempdir().unwrap();
    let decoy = decoy_root.path().join(format!("selected/{CONTENT}.wav"));
    std::fs::create_dir_all(decoy.parent().unwrap()).unwrap();
    std::fs::write(decoy, WAV).unwrap();
    let context = PreparationContext::new(decoy_root.path(), 32_000);
    assert!(
        ResourceResolver::new(&context).resolve(&resource).is_err(),
        "an unadmitted selected reference cannot gain authority from a decoy file"
    );
    let ordinary = ResourceRef::new(
        ResourceKind::Sample,
        "missing.wav",
        ResourceOrigin::Document,
    );
    assert!(ResourceResolver::new(&context).resolve(&ordinary).is_err());

    let snapshot = Arc::new(SampleSnapshot::decode(WAV.to_vec()).unwrap());
    let wrong_rate = PreparationContext::new(decoy_root.path(), 48_000)
        .with_selected_samples([Arc::clone(&snapshot)]);
    assert!(
        ResourceResolver::new(&wrong_rate)
            .resolve(&resource)
            .is_err()
    );
    let impulse = ResourceRef::new(
        ResourceKind::ImpulseResponse,
        snapshot.resource_id(),
        ResourceOrigin::Document,
    );
    let context =
        PreparationContext::new(decoy_root.path(), 32_000).with_selected_samples([snapshot]);
    assert!(ResourceResolver::new(&context).resolve(&impulse).is_err());
}

#[test]
fn snapshots_reject_oversized_malformed_and_empty_samples_without_an_owner() {
    assert_eq!(
        SampleSnapshot::resource_id_for_encoded(&vec![0; MAX_ENCODED_SAMPLE_BYTES + 1]),
        Err(SnapshotError::EncodedLimit)
    );
    assert!(matches!(
        SampleSnapshot::decode(vec![0; MAX_ENCODED_SAMPLE_BYTES + 1]),
        Err(SnapshotError::EncodedLimit)
    ));
    assert!(matches!(
        SampleSnapshot::decode(b"invalid WAV".to_vec()),
        Err(SnapshotError::InvalidWav(_))
    ));
    let mut empty = WAV[..44].to_vec();
    empty[4..8].copy_from_slice(&36_u32.to_le_bytes());
    empty[40..44].copy_from_slice(&0_u32.to_le_bytes());
    assert!(matches!(
        SampleSnapshot::decode(empty),
        Err(SnapshotError::Empty)
    ));
}
