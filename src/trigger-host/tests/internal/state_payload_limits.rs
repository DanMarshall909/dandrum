//! Maintained framing/capacity qualification; large audio stays in temporary files.
use super::{fixture, portable_state};
use crate::parameters::ParameterController;
use crate::runtime::{ControlEvent, RuntimeAudio, TimedEvent};
use crate::samples::CatalogUsage;
use crate::state::StateError;
use crate::{ADVANCED_DRUM_KIT_DOCUMENT, ADVANCED_DRUM_KIT_SAMPLE};
use serde_json::{Value, json};
use std::io::{Read, Seek, SeekFrom, Write};

const ASSET_BYTES: usize = 64 * 1024 * 1024;
const PAYLOAD_BYTES: usize = 4 * ASSET_BYTES;
const FRAMES: usize = (ASSET_BYTES - 44) / 4;
const METADATA_BYTES: usize = 1024 * 1024;
// Independently calculated from the literal PCM16 recipe, not the native codec.
const HASHES: [&str; 4] = [
    "5390c312567f61cc4617a13b184a283c0a95619f576f48bc61d691ac99de8a94",
    "511a509b8372689562c86be30108bf9e8877658e93ea1cf3d5b9447585e29a9d",
    "135956c4c0177407e0681248155d2b423f61ae28e39c1a4b8c5ab8f7d43f9ca4",
    "9b5d7aaf1e6144a0ab731caacd2df5a7094169ce8633dc5bd476e0636b86c802",
];
const TAILS: [[[i16; 2]; 2]; 4] = [
    [[-24576, -8192], [-16384, 0]],
    [[-16384, 0], [8192, 8192]],
    [[-8192, 0], [16384, 0]],
    [[-4096, -4096], [-24576, 8192]],
];

fn header(data_bytes: usize, channels: u16) -> Vec<u8> {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend(u32::try_from(36 + data_bytes).unwrap().to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16_u32.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    bytes.extend(channels.to_le_bytes());
    bytes.extend(48_000_u32.to_le_bytes());
    bytes.extend((48_000_u32 * u32::from(channels) * 2).to_le_bytes());
    bytes.extend((channels * 2).to_le_bytes());
    bytes.extend(16_u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(u32::try_from(data_bytes).unwrap().to_le_bytes());
    assert_eq!(bytes.len(), 44);
    bytes
}
fn small_wave(index: usize) -> Vec<u8> {
    let mut bytes = header(2, 1);
    bytes.extend((-16384_i16 + i16::try_from(index).unwrap()).to_le_bytes());
    bytes
}
fn descriptor(bytes: &[u8]) -> Value {
    let path =
        dandrum_engine::sample_snapshot::SampleSnapshot::resource_id_for_encoded(bytes).unwrap();
    let sha = path
        .strip_prefix("selected/")
        .unwrap()
        .strip_suffix(".wav")
        .unwrap();
    json!({"path":path,"sha256":sha,"encoded_bytes":bytes.len()})
}
fn prefix(metadata: &Value, payload: u64) -> Vec<u8> {
    let metadata = serde_json::to_vec(metadata).unwrap();
    let mut bytes = b"DTRGSTAT".to_vec();
    bytes.extend(3_u32.to_le_bytes());
    bytes.extend(u32::try_from(metadata.len()).unwrap().to_le_bytes());
    bytes.extend(payload.to_le_bytes());
    bytes.extend(metadata);
    bytes
}
fn metadata(
    controller: &mut ParameterController,
    resources: &[Value],
    frames: usize,
    zones: &[(usize, u8)],
) -> Value {
    let mut saved = Vec::new();
    controller.save_state(&mut saved).unwrap();
    let mut metadata = portable_state::metadata(&saved);
    let mut document: serde_yaml::Value =
        serde_yaml::from_str(metadata["document"].as_str().unwrap()).unwrap();
    let sources: Vec<_> = resources
        .iter()
        .enumerate()
        .map(|(index, resource)| {
            json!({
                "id":format!("source_{index}"),"path":resource["path"],
                "regions":[{"id":"body","start_frame":frames.saturating_sub(2),"end_frame":frames}]
            })
        })
        .collect();
    let zones: Vec<_> = zones
        .iter()
        .map(|(index, note)| {
            json!({
                "id":format!("hit_{index}"),"region":format!("source_{index}.body"),
                "key_range":[note,note],"velocity_range":[1,127],"control_group":1
            })
        })
        .collect();
    document["assets"] = serde_yaml::to_value(json!({"sample_sources":sources,
        "sample_maps":[{"id":"kit","zones":zones}]}))
    .unwrap();
    metadata["document"] = serde_yaml::to_string(&document).unwrap().into();
    metadata["resources"] = resources.to_vec().into();
    let level = metadata["parameters"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|p| p["id"] == "drums.level")
        .unwrap();
    level["value"] = 2.0.into();
    metadata
}
fn render(audio: &mut RuntimeAudio, note: u8, expected: [f32; 2]) {
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let (mut left, mut right) = ([123.; 16], [123.; 16]);
                let audit = allocation_counter::measure(|| {
                    audio.reset();
                    audio
                        .process_events(
                            &mut left,
                            &mut right,
                            &[TimedEvent {
                                frame_offset: 3,
                                event: ControlEvent::NoteOn {
                                    note,
                                    velocity: 127,
                                },
                            }],
                        )
                        .unwrap();
                });
                assert_eq!(audit, allocation_counter::AllocationInfo::default());
                assert_eq!(&left[..3], &[0.; 3]);
                assert_eq!(left, right);
                assert_eq!([left[3], left[4]], expected);
                assert_eq!(&left[5..], &[0.; 11]);
            })
            .join()
            .unwrap();
    });
}
struct RefuseBody<'a> {
    remaining: &'a [u8],
    consumed: usize,
}
impl Read for RefuseBody<'_> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        assert!(
            !self.remaining.is_empty(),
            "refusal must precede another body read"
        );
        let count = output.len().min(self.remaining.len());
        output[..count].copy_from_slice(&self.remaining[..count]);
        self.remaining = &self.remaining[count..];
        self.consumed += count;
        Ok(count)
    }
}
fn same_bytes(first: &mut std::fs::File, second: &mut std::fs::File) {
    first.rewind().unwrap();
    second.rewind().unwrap();
    let (mut left, mut right) = ([0; 65536], [0; 65536]);
    loop {
        let count = first.read(&mut left).unwrap();
        if count == 0 {
            assert_eq!(second.read(&mut right).unwrap(), 0);
            break;
        }
        second.read_exact(&mut right[..count]).unwrap();
        assert!(left[..count] == right[..count], "exact saved bytes");
    }
}

#[test]
fn exactly_two_hundred_fifty_six_owned_resources_restore_and_release() {
    let (_directory, mut controller, mut audio) = fixture();
    let waves: Vec<_> = (0..256).map(small_wave).collect();
    let resources: Vec<_> = waves.iter().map(|bytes| descriptor(bytes)).collect();
    let ids: std::collections::BTreeSet<_> = resources
        .iter()
        .map(|r| r["path"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), 256);
    assert_eq!(
        resources[0]["sha256"],
        "77c28083dbf5cab8195816070dfcfe78c156a8cb1095dbb511f3f9c363ce1f43"
    );
    assert_eq!(
        resources[255]["sha256"],
        "7840ffd4a576530ec9dcb2f6521e641e8d58190489d969a20b38f07cd2e36d75"
    );
    let metadata = metadata(&mut controller, &resources, 1, &[(255, 60)]);
    let mut frame = prefix(&metadata, 256 * 46);
    for bytes in &waves {
        frame.extend(bytes);
    }
    controller.load_state(&mut frame.as_slice()).unwrap();
    assert_eq!(
        controller.host.sample_catalog.usage(),
        CatalogUsage {
            encoded_bytes: 256 * 46,
            decoded_bytes: 256 * 4,
            resource_count: 256
        }
    );
    let retention = controller
        .host
        .sample_catalog
        .acquire(resources[255]["path"].as_str().unwrap())
        .unwrap()
        .retention();
    render(&mut audio, 60, [-16129.0 / 16384.0, 0.]);
    let accepted = controller.session().owner_snapshot();
    let values = controller.parameter_state();
    let mut extra = resources.clone();
    extra.push(descriptor(&small_wave(256)));
    let rejected = prefix(
        &self::metadata(&mut controller, &extra, 1, &[(256, 60)]),
        257 * 46,
    );
    let mut input = RefuseBody {
        remaining: &rejected,
        consumed: 0,
    };
    assert_eq!(controller.load_state(&mut input), Err(StateError::TooLarge));
    assert_eq!(
        input.consumed,
        rejected.len(),
        "257 entries refuse before body I/O"
    );
    assert_eq!(controller.session().owner_snapshot(), accepted);
    assert_eq!(controller.parameter_state(), values);
    render(&mut audio, 60, [-16129.0 / 16384.0, 0.]);
    assert_eq!(
        controller.session().owner_snapshot().yaml,
        metadata["document"].as_str().unwrap()
    );
    let mut saved = Vec::new();
    controller.save_state(&mut saved).unwrap();
    assert_eq!(
        portable_state::metadata(&saved)["resources"]
            .as_array()
            .unwrap()
            .len(),
        256
    );
    drop(controller);
    assert!(retention.is_live());
    drop(audio);
    assert!(!retention.is_live());
}

#[test]
fn exact_metadata_ceiling_saves_and_restores_but_one_extra_byte_emits_nothing() {
    let (directory, mut controller, audio) = fixture();
    let mut original = Vec::new();
    controller.save_state(&mut original).unwrap();
    let size = u32::from_le_bytes(original[12..16].try_into().unwrap()) as usize;
    let document = controller.session().owner_snapshot().yaml;
    let root = directory.path().join("Trigger.resources");
    drop(audio);
    drop(controller);
    // A final ASCII comment adds its text plus two JSON bytes for its newline.
    let exact = format!("{document}#{}\n", "x".repeat(METADATA_BYTES - size - 3));
    std::fs::write(root.join(ADVANCED_DRUM_KIT_DOCUMENT), &exact).unwrap();
    let config = crate::TriggerHostConfiguration {
        sample_rate_hz: crate::SampleRate::new(48_000.).unwrap(),
        max_block_frames: 16,
    };
    let open = || {
        ParameterController::open(&root, config, std::num::NonZeroU64::new(1).unwrap(), 32, 8)
            .unwrap()
    };
    let (mut source, source_audio) = open();
    let mut saved = Vec::new();
    source.save_state(&mut saved).unwrap();
    assert_eq!(
        u32::from_le_bytes(saved[12..16].try_into().unwrap()) as usize,
        METADATA_BYTES
    );
    drop(source_audio);
    drop(source);
    std::fs::write(root.join(ADVANCED_DRUM_KIT_DOCUMENT), &document).unwrap();
    let (mut fresh, mut fresh_audio) = open();
    std::fs::remove_file(root.join(ADVANCED_DRUM_KIT_SAMPLE)).unwrap();
    fresh.load_state(&mut saved.as_slice()).unwrap();
    assert_eq!(fresh.session().owner_snapshot().yaml, exact);
    // Region is the full factory hit here; inspect its independently known first frame.
    fresh_audio.process(&mut [0.; 16], &mut [0.; 16]).unwrap();
    let mut left = [0.; 16];
    let mut right = [0.; 16];
    fresh_audio.reset();
    fresh_audio
        .process_events(
            &mut left,
            &mut right,
            &[TimedEvent {
                frame_offset: 0,
                event: ControlEvent::NoteOn {
                    note: 36,
                    velocity: 127,
                },
            }],
        )
        .unwrap();
    assert_eq!([left[0], right[0]], [-0.5, -0.5]);
    drop(fresh_audio);
    drop(fresh);
    let repository = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::copy(
        repository.join("examples/patches/assets/advanced-drums.wav"),
        root.join(ADVANCED_DRUM_KIT_SAMPLE),
    )
    .unwrap();
    std::fs::write(root.join(ADVANCED_DRUM_KIT_DOCUMENT), format!("{exact}#")).unwrap();
    let (mut oversized, _audio) = open();
    let before = oversized.session().owner_snapshot();
    let mut output = Vec::new();
    assert_eq!(oversized.save_state(&mut output), Err(StateError::TooLarge));
    assert!(output.is_empty());
    assert_eq!(oversized.session().owner_snapshot(), before);
}

#[test]
#[ignore = "slow: actual 256 MiB encoded/512 MiB minus WAV headers decoded portable state"]
fn maximum_portable_payload_restores_signed_tail_geometry_and_reclaims_every_owner() {
    let (directory, mut source, mut audio) = fixture();
    let resources:Vec<_>=HASHES.iter().map(|sha|json!({"path":format!("selected/{sha}.wav"),"sha256":sha,"encoded_bytes":ASSET_BYTES})).collect();
    let metadata = metadata(
        &mut source,
        &resources,
        FRAMES,
        &[(0, 60), (1, 61), (2, 62), (3, 63)],
    );
    let framing = prefix(&metadata, PAYLOAD_BYTES as u64);
    let mut input = tempfile::tempfile().unwrap();
    input.write_all(&framing).unwrap();
    for (index, tail) in TAILS.iter().enumerate() {
        let start = input.stream_position().unwrap();
        input.write_all(&header(ASSET_BYTES - 44, 2)).unwrap();
        input.write_all(&[0, 64, 0, 64]).unwrap();
        input
            .seek(SeekFrom::Start(start + ASSET_BYTES as u64 - 8))
            .unwrap();
        for frame in tail {
            for channel in frame {
                input.write_all(&channel.to_le_bytes()).unwrap();
            }
        }
        assert_eq!(
            input.stream_position().unwrap(),
            framing.len() as u64 + (index as u64 + 1) * ASSET_BYTES as u64
        );
    }
    input.rewind().unwrap();
    source.load_state(&mut input).unwrap();
    drop(input);
    let used = CatalogUsage {
        encoded_bytes: PAYLOAD_BYTES,
        decoded_bytes: 2 * (PAYLOAD_BYTES - 4 * 44),
        resource_count: 4,
    };
    assert_eq!(used.decoded_bytes, 536_870_560);
    assert_eq!(source.host.sample_catalog.usage(), used);
    let golden = [[-1., -0.5], [-0.5, 0.5], [-0.25, 0.5], [-0.25, -0.5]];
    for (index, expected) in golden.iter().enumerate() {
        render(&mut audio, 60 + index as u8, *expected);
    }
    let retentions: Vec<_> = resources
        .iter()
        .map(|r| {
            source
                .host
                .sample_catalog
                .acquire(r["path"].as_str().unwrap())
                .unwrap()
                .retention()
        })
        .collect();
    let accepted = source.session().owner_snapshot();
    let values = source.parameter_state().parameters;
    let mut saved = tempfile::tempfile().unwrap();
    source.save_state(&mut saved).unwrap();
    assert!(saved.metadata().unwrap().len() > PAYLOAD_BYTES as u64);
    saved.rewind().unwrap();
    let mut framing = [0; 24];
    saved.read_exact(&mut framing).unwrap();
    let mut encoded_metadata =
        vec![0; u32::from_le_bytes(framing[12..16].try_into().unwrap()) as usize];
    saved.read_exact(&mut encoded_metadata).unwrap();
    let canonical: Value = serde_json::from_slice(&encoded_metadata).unwrap();
    assert_eq!(canonical["resources"].as_array().unwrap().len(), 4);

    drop(source);
    assert!(retentions.iter().all(|r| r.is_live()));
    drop(audio);
    assert!(retentions.iter().all(|r| !r.is_live()));
    directory.close().unwrap();
    let (fresh_directory, mut fresh, mut fresh_audio) = fixture();
    std::fs::remove_file(
        fresh_directory
            .path()
            .join("Trigger.resources")
            .join(ADVANCED_DRUM_KIT_SAMPLE),
    )
    .unwrap();
    saved.rewind().unwrap();
    // Match the bounded foreign-stream projection: input ownership is acquired
    // outside the controller borrow, then the native admission is revalidated.
    let state_header = crate::state::read_header(&mut saved).unwrap();
    fresh.preflight_state(&state_header).unwrap();
    let buffered = state_header.buffer_payload(&mut saved).unwrap();
    fresh.load_buffered_state(buffered).unwrap();
    assert_eq!(fresh.host.sample_catalog.usage(), used);
    assert_eq!(fresh.session().owner_snapshot().yaml, accepted.yaml);
    assert_eq!(fresh.parameter_state().parameters, values);
    for (index, expected) in golden.iter().enumerate() {
        render(&mut fresh_audio, 60 + index as u8, *expected);
    }
    let mut again = tempfile::tempfile().unwrap();
    fresh.save_state(&mut again).unwrap();
    same_bytes(&mut saved, &mut again);
    drop(again);
    let before = fresh.session().owner_snapshot();
    let parameters = fresh.parameter_state();
    let mut rejected = canonical.clone();
    rejected["document"] = format!(
        "{}\n# refused metadata must not publish\n",
        metadata["document"].as_str().unwrap()
    )
    .into();
    rejected["parameters"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|p| p["id"] == "drums.level")
        .unwrap()["value"] = 3.0.into();
    // The complete candidate is semantically valid before its final body byte is lost.
    let refused_prefix = prefix(&rejected, PAYLOAD_BYTES as u64);
    fresh
        .preflight_state(&crate::state::read_header(&mut refused_prefix.as_slice()).unwrap())
        .unwrap();
    saved.rewind().unwrap();
    let mut original_header = [0; 24];
    saved.read_exact(&mut original_header).unwrap();
    let original_metadata = u32::from_le_bytes(original_header[12..16].try_into().unwrap());
    saved
        .seek(SeekFrom::Current(i64::from(original_metadata)))
        .unwrap();
    let mut truncated = refused_prefix
        .as_slice()
        .chain(Read::by_ref(&mut saved).take(PAYLOAD_BYTES as u64 - 1));
    assert_eq!(fresh.load_state(&mut truncated), Err(StateError::Invalid));
    assert_eq!(
        saved.stream_position().unwrap(),
        saved.metadata().unwrap().len() - 1,
        "the refusal reaches the missing final body byte, after valid earlier content"
    );
    assert_eq!(fresh.host.sample_catalog.usage(), used);
    assert_eq!(fresh.session().owner_snapshot(), before);
    assert_eq!(fresh.parameter_state(), parameters);
    for (index, expected) in golden.iter().enumerate() {
        render(&mut fresh_audio, 60 + index as u8, *expected);
    }
    for aggregate in [false, true] {
        let mut too_large = rejected.clone();
        if aggregate {
            let mut extra = descriptor(&small_wave(256));
            extra["encoded_bytes"] = 1.into();
            too_large["resources"].as_array_mut().unwrap().push(extra);
        } else {
            too_large["resources"][0]["encoded_bytes"] = (ASSET_BYTES + 1).into();
        }
        let bytes = prefix(&too_large, PAYLOAD_BYTES as u64 + 1);
        let mut input = RefuseBody {
            remaining: &bytes,
            consumed: 0,
        };
        assert_eq!(fresh.load_state(&mut input), Err(StateError::TooLarge));
        assert_eq!(
            input.consumed,
            bytes.len(),
            "metadata bounds precede body I/O"
        );
    }
    let small = small_wave(256);
    let extra = descriptor(&small);
    let mut competing = rejected.clone();
    let mut graph: serde_yaml::Value =
        serde_yaml::from_str(competing["document"].as_str().unwrap()).unwrap();
    graph["assets"]["sample_sources"][0]["path"] = extra["path"].as_str().unwrap().into();
    competing["document"] = serde_yaml::to_string(&graph).unwrap().into();
    let mut manifest = vec![extra];
    manifest.extend(resources.iter().skip(1).cloned());
    competing["resources"] = manifest.into();
    let mut bytes = prefix(&competing, (3 * ASSET_BYTES + small.len()) as u64);
    bytes.extend(&small);
    let mut input = RefuseBody {
        remaining: &bytes,
        consumed: 0,
    };
    assert_eq!(fresh.load_state(&mut input), Err(StateError::TooLarge));
    assert_eq!(
        input.consumed,
        bytes.len(),
        "distinct content refuses after bounded read/hash at full live capacity"
    );
    assert_eq!(fresh.host.sample_catalog.usage(), used);
    assert_eq!(fresh.session().owner_snapshot(), before);
    assert_eq!(fresh.parameter_state(), parameters);
    for (index, expected) in golden.iter().enumerate() {
        render(&mut fresh_audio, 60 + index as u8, *expected);
    }
    // Exact serialized content can be restored again while the live union is full.
    saved.rewind().unwrap();
    fresh.load_state(&mut saved).unwrap();
    assert_eq!(fresh.host.sample_catalog.usage(), used);
    for (index, expected) in golden.iter().enumerate() {
        render(&mut fresh_audio, 60 + index as u8, *expected);
    }
    let retentions: Vec<_> = resources
        .iter()
        .map(|r| {
            fresh
                .host
                .sample_catalog
                .acquire(r["path"].as_str().unwrap())
                .unwrap()
                .retention()
        })
        .collect();
    drop(fresh);
    assert!(retentions.iter().all(|r| r.is_live()));
    drop(fresh_audio);
    assert!(retentions.iter().all(|r| !r.is_live()));
}
