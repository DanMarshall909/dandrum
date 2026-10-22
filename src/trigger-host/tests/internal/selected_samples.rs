use super::*;

const SELECTED_PATCH: &str = r#"
assets:
  sample_sources:
    - id: hit
      path: RESOURCE
      regions: [{ id: body, start_frame: 0, end_frame: 2 }]
  sample_maps:
    - id: kit
      zones: [{ id: hit, region: hit.body, key_range: [KEY, KEY], velocity_range: [1, 127] }]
ports:
  - { name: master, direction: output, signal: audio, channels: 2, maps_from: player.audio }
modules:
  - { id: midi, type: midi_input }
  - { id: player, type: sample_map_player, static: { sample_map: kit, max_voices: 1, channels: 2 } }
connections:
  - { from: midi.events, to: player.note }
"#;

fn selected_runtime(catalog: &SampleCatalog, id: &str, key: u8) -> crate::runtime::PreparedRuntime {
    use dandrum_engine::{
        preparation::{HostBuses, PreparationContext},
        render_settings::RealtimeRenderSettings,
    };
    let rate = crate::SampleRate::new(48_000.0).unwrap();
    let session = crate::InstrumentSession::new_realtime_for_planar_host(
        &SELECTED_PATCH
            .replace("RESOURCE", id)
            .replace("KEY", &key.to_string()),
        PreparationContext::new_realtime("/selected-runtime-no-filesystem-root", rate)
            .with_selected_samples([catalog.acquire(id).unwrap()]),
        RealtimeRenderSettings {
            sample_rate_hz: rate,
            block_size_frames: 16,
            duration_frames: 16,
        },
        HostBuses::new().with_output("master", 2),
    )
    .unwrap();
    crate::runtime::PreparedRuntime::new(
        session.prepared(),
        crate::TriggerHostConfiguration {
            sample_rate_hz: rate,
            max_block_frames: 16,
        },
    )
    .unwrap()
}
fn signed_callback(
    mut audio: crate::runtime::RuntimeAudio,
    key: u8,
    expected: f32,
    swapped: bool,
) -> crate::runtime::RuntimeAudio {
    use crate::runtime::{ControlEvent, TimedEvent};
    let control = std::thread::current().id();
    std::thread::spawn(move || {
        assert_ne!(std::thread::current().id(), control);
        let mut left = [123.0; 16];
        let mut right = [123.0; 16];
        let events = [TimedEvent {
            frame_offset: 0,
            event: ControlEvent::NoteOn {
                note: key,
                velocity: 100,
            },
        }];
        let audit = allocation_counter::measure(|| {
            audio.reset();
            assert_eq!(
                audio
                    .process_events(&mut left, &mut right, &events)
                    .unwrap()
                    .swapped,
                swapped
            );
        });
        assert_eq!(
            audit,
            allocation_counter::AllocationInfo::default(),
            "selected callback allocates and deallocates nothing"
        );
        for channel in [left, right] {
            assert_eq!(
                &channel[..2],
                &[expected; 2],
                "literal selected PCM after the real engine consumer"
            );
            assert_eq!(&channel[4..], &[0.0; 12]);
        }
        audio
    })
    .join()
    .unwrap()
}

#[test]
fn selected_runtime_pins_bytes_until_control_reclamation_without_callback_destruction() {
    use crate::runtime::runtime_handoff;
    let mut negative = wav();
    for pcm in negative[44..].chunks_exact_mut(2) {
        pcm.copy_from_slice(&(-24_576_i16).to_le_bytes());
    }
    let mut positive = negative.clone();
    for pcm in positive[44..].chunks_exact_mut(2) {
        pcm.copy_from_slice(&8_192_i16.to_le_bytes());
    }
    let mut third = negative.clone();
    for pcm in third[44..].chunks_exact_mut(2) {
        pcm.copy_from_slice(&16_384_i16.to_le_bytes());
    }
    let mut independent = negative.clone();
    for pcm in independent[44..].chunks_exact_mut(2) {
        pcm.copy_from_slice(&(-8_192_i16).to_le_bytes());
    }
    let mut independent_catalog = SampleCatalog::new();
    let independent_files = finish(&mut independent_catalog, vec![input(&independent)]);
    let independent_id = independent_files[0].sample.resource_id().to_owned();
    let (_, independent_audio) = runtime_handoff(
        selected_runtime(&independent_catalog, &independent_id, 60),
        8,
    )
    .unwrap();
    drop(independent_files);
    let independent_audio = signed_callback(independent_audio, 60, -0.25, false);
    let mut catalog = SampleCatalog::new();
    catalog.limits.encoded = negative.len() * 2;
    let files = finish(&mut catalog, vec![input(&negative)]);
    let first_id = files[0].sample.resource_id().to_owned();
    let first = selected_runtime(&catalog, &first_id, 36);
    drop(files);
    let (mut control, audio) = runtime_handoff(first, 8).unwrap();
    let audio = signed_callback(audio, 36, -0.75, false);
    assert_eq!(
        catalog.usage().encoded_bytes,
        negative.len(),
        "only the real runtime retains the selected bytes"
    );
    let files = finish(&mut catalog, vec![input(&positive)]);
    let second_id = files[0].sample.resource_id().to_owned();
    let replacement = selected_runtime(&catalog, &second_id, 36);
    drop(files);
    assert!(control.offer(replacement).is_ok());
    let audio = signed_callback(audio, 36, 0.25, true);
    assert_eq!(
        catalog.usage().encoded_bytes,
        negative.len() * 2,
        "retired runtime remains charged before reclaim"
    );
    let work = catalog.begin(vec![input(&third)]).unwrap();
    assert_eq!(
        catalog.accept(work.prepare()).unwrap().unwrap_err(),
        SampleError::EncodedLimit
    );
    let retired = control
        .reclaim()
        .expect("the audio thread returned its old runtime");
    assert_eq!(
        catalog.usage().encoded_bytes,
        negative.len() * 2,
        "returned ownership alone does not destroy resources"
    );
    drop(retired);
    assert_eq!(catalog.usage().encoded_bytes, positive.len());
    assert_eq!(
        catalog.acquire(&first_id).unwrap_err(),
        SampleError::UnknownResource
    );
    let audio = signed_callback(audio, 36, 0.25, false);
    assert_eq!(finish(&mut catalog, vec![input(&third)]).len(), 1);
    let independent_audio = signed_callback(independent_audio, 60, -0.25, false);
    assert_eq!(independent_catalog.usage().encoded_bytes, independent.len());
    assert_eq!(
        independent_catalog.acquire(&first_id).unwrap_err(),
        SampleError::UnknownResource
    );
    drop(independent_audio);
    assert_eq!(independent_catalog.usage(), CatalogUsage::default());
    drop(audio); // Returned to this constructing control thread before destruction.
}

#[test]
fn pending_work_is_charged_until_its_cross_thread_completion_is_consumed() {
    let bytes = wav();
    let mut catalog = SampleCatalog::new();
    let job = catalog.begin(vec![input(&bytes)]).unwrap();
    assert_eq!(
        catalog.usage().encoded_bytes,
        64 * 1024 * 1024,
        "one pending file reserves its maximum retained encoded content"
    );
    assert_eq!(catalog.usage().decoded_bytes, 128 * 1024 * 1024);
    assert_eq!(catalog.usage().resource_count, 1);
    let completion = std::thread::spawn(move || job.prepare()).join().unwrap();
    assert_eq!(
        catalog.usage().encoded_bytes,
        64 * 1024 * 1024,
        "a returned but unconsumed completion still owns its reservation"
    );
    let files = catalog.accept(completion).unwrap().unwrap();
    assert_eq!(catalog.usage().encoded_bytes, bytes.len());
    assert_eq!(catalog.usage().decoded_bytes, 16);
    drop(files);
    assert_eq!(catalog.usage(), CatalogUsage::default());
}

fn wav() -> Vec<u8> {
    let mut bytes = Vec::new();
    dandrum_engine::wav::write_wav_stereo_i16(&mut bytes, 32_000, &[-0.75, -0.5], &[-0.25, -0.5])
        .unwrap();
    bytes
}
fn input(bytes: &[u8]) -> NativeSampleFile {
    use std::io::{Seek, Write};
    let mut file = tempfile::tempfile().unwrap();
    file.write_all(bytes).unwrap();
    file.rewind().unwrap();
    NativeSampleFile::new("selected.wav".into(), file).unwrap()
}
fn finish(catalog: &mut SampleCatalog, files: Vec<NativeSampleFile>) -> Vec<SelectedFile> {
    let completion = catalog.begin(files).unwrap().prepare();
    catalog.accept(completion).unwrap().unwrap()
}

#[test]
fn retained_results_and_prepared_consumers_stay_charged_and_reuse_content() {
    use dandrum_engine::{
        kernel::{ResourceKind, ResourceOrigin, ResourceRef},
        preparation::{PreparationContext, ResourceResolver},
    };
    let mut catalog = SampleCatalog::new();
    let bytes = wav();
    catalog.limits.encoded = bytes.len();
    let mut files = finish(&mut catalog, vec![input(&bytes)]);
    let id = files[0].sample.resource_id().to_owned();
    assert_eq!(
        catalog.usage(),
        CatalogUsage {
            encoded_bytes: bytes.len(),
            decoded_bytes: 16,
            resource_count: 1
        }
    );
    let authority = catalog.acquire(&id).unwrap();
    let context =
        PreparationContext::new("/absent-selected-root", 32_000).with_selected_samples([authority]);
    let resource = ResourceRef::new(ResourceKind::Sample, &id, ResourceOrigin::Document);
    let resolved = ResourceResolver::new(&context).resolve(&resource).unwrap();
    drop(context);
    files.clear();
    assert_eq!(
        catalog.usage().encoded_bytes,
        bytes.len(),
        "prepared sample pins encoded authority"
    );
    assert_eq!(resolved.sample().source_sample(0, 0), Some(-0.75));
    let duplicate = finish(&mut catalog, vec![input(&bytes), input(&bytes)]);
    assert_eq!(catalog.usage().resource_count, 1);
    assert_eq!(
        duplicate[0].sample.sample().source_sample(1, 0),
        Some(-0.25)
    );
    let mut changed = bytes.clone();
    changed[44] ^= 1;
    let job = catalog.begin(vec![input(&changed)]).unwrap();
    assert_eq!(
        catalog.accept(job.prepare()).unwrap().unwrap_err(),
        SampleError::EncodedLimit
    );
    assert_eq!(catalog.usage().encoded_bytes, bytes.len());
    drop(duplicate);
    drop(resolved);
    assert_eq!(catalog.usage(), CatalogUsage::default());
    assert_eq!(
        catalog.acquire(&id).unwrap_err(),
        SampleError::UnknownResource
    );
    assert_eq!(finish(&mut catalog, vec![input(&changed)]).len(), 1);
}

#[test]
fn batch_failure_cancellation_and_foreign_completion_preserve_capacity() {
    let mut catalog = SampleCatalog::new();
    let bytes = wav();
    let job = catalog.begin(vec![input(&bytes)]).unwrap();
    assert!(matches!(
        catalog.begin(vec![input(&bytes)]),
        Err(SampleError::Busy)
    ));
    drop(job);
    let job = catalog
        .begin(vec![input(&bytes), input(b"not a WAV")])
        .unwrap();
    assert_eq!(
        catalog.accept(job.prepare()).unwrap().unwrap_err(),
        SampleError::InvalidSample
    );
    assert_eq!(catalog.usage(), CatalogUsage::default());
    let prepared = catalog.begin(vec![input(&bytes)]).unwrap().prepare();
    let mut other = SampleCatalog::new();
    let rejected = other.accept(prepared).unwrap_err();
    let diagnostic = format!("{rejected:?}");
    assert!(
        diagnostic.contains("RejectedSampleCompletion") && diagnostic.len() < 128,
        "foreign-completion diagnostics must identify the failure without dumping sample payloads"
    );
    assert_eq!(other.usage(), CatalogUsage::default());
    assert!(matches!(
        catalog.begin(vec![input(&bytes)]),
        Err(SampleError::Busy)
    ));
    let files = catalog.accept(rejected.into_completion()).unwrap().unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(
        other.acquire(files[0].sample.resource_id()).unwrap_err(),
        SampleError::UnknownResource
    );
}

#[test]
fn file_count_encoded_decoded_and_resource_limits_reject_atomically() {
    let bytes = wav();
    let mut catalog = SampleCatalog::new();
    assert!(matches!(catalog.begin(vec![]), Err(SampleError::FileCount)));
    assert!(matches!(
        catalog.begin((0..17).map(|_| input(&bytes)).collect()),
        Err(SampleError::FileCount)
    ));
    let sixteen = finish(&mut catalog, (0..16).map(|_| input(&bytes)).collect());
    assert_eq!(sixteen.len(), 16);
    assert_eq!(catalog.usage().resource_count, 1);
    drop(sixteen);
    catalog.limits.encoded = bytes.len() - 1;
    let work = catalog.begin(vec![input(&bytes)]).unwrap();
    assert_eq!(
        catalog.accept(work.prepare()).unwrap().unwrap_err(),
        SampleError::EncodedLimit
    );
    catalog.limits.encoded = 256 * 1024 * 1024;
    catalog.limits.decoded = 15;
    let work = catalog.begin(vec![input(&bytes)]).unwrap();
    assert_eq!(
        catalog.accept(work.prepare()).unwrap().unwrap_err(),
        SampleError::DecodedLimit
    );
    catalog.limits.decoded = 16;
    catalog.limits.resources = 1;
    let mut changed = bytes.clone();
    changed[44] ^= 1;
    let work = catalog.begin(vec![input(&bytes), input(&changed)]).unwrap();
    assert_eq!(
        catalog.accept(work.prepare()).unwrap().unwrap_err(),
        SampleError::ResourceLimit
    );
    assert_eq!(catalog.usage(), CatalogUsage::default());
    assert_eq!(finish(&mut catalog, vec![input(&bytes)]).len(), 1);
}

#[test]
fn untrusted_file_shapes_names_and_read_errors_never_publish_resources() {
    assert!(matches!(
        NativeSampleFile::new("bad\0name.wav".into(), tempfile::tempfile().unwrap()),
        Err(SampleError::InvalidName)
    ));
    assert!(matches!(
        NativeSampleFile::new("".into(), tempfile::tempfile().unwrap()),
        Err(SampleError::InvalidName)
    ));
    assert!(matches!(
        NativeSampleFile::new("a".repeat(257), tempfile::tempfile().unwrap()),
        Err(SampleError::InvalidName)
    ));
    assert!(matches!(
        NativeSampleFile::new("../sample.wav".into(), tempfile::tempfile().unwrap()),
        Err(SampleError::InvalidName)
    ));
    let directory = tempfile::tempdir().unwrap();
    assert!(matches!(
        NativeSampleFile::new(
            "directory.wav".into(),
            std::fs::File::open(directory.path()).unwrap()
        ),
        Err(SampleError::InvalidFile)
    ));
    let file = std::fs::File::create(directory.path().join("write-only.wav")).unwrap();
    let mut catalog = SampleCatalog::new();
    let work = catalog
        .begin(vec![
            NativeSampleFile::new("write-only.wav".into(), file).unwrap(),
        ])
        .unwrap();
    assert_eq!(
        catalog.accept(work.prepare()).unwrap().unwrap_err(),
        SampleError::Read
    );
    assert_eq!(catalog.usage(), CatalogUsage::default());
    let huge = tempfile::tempfile().unwrap();
    huge.set_len((MAX_ENCODED_SAMPLE_BYTES + 1) as u64).unwrap();
    let work = catalog
        .begin(vec![
            NativeSampleFile::new("too-big.wav".into(), huge).unwrap(),
        ])
        .unwrap();
    assert_eq!(
        catalog.accept(work.prepare()).unwrap().unwrap_err(),
        SampleError::EncodedLimit
    );
    assert_eq!(catalog.usage(), CatalogUsage::default());
}
