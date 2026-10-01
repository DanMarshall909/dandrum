use std::fs;

use crate::diagnostics::error_codes;
use crate::graph::{PortDirection, SignalType};
use crate::graph_processor::render_kernel_offline_named;
use crate::kernel::{
    ChannelCount, ControlDefault, DefinitionImplementation, GraphDefinition,
    POLY_ALLOCATION_OLDEST_STEAL, POLY_DEFINITION, POLY_NOTE_EVENTS_INPUT, Port, ResourceKind,
    ResourceOrigin, ResourceRef, StaticArg, StaticType, StaticValue,
};
use crate::patch::RenderSettings;
use crate::patch::load_preset_str;
use crate::preparation::{
    PreparationContext, prepare_kernel_patch_with_context, prepare_kernel_patch_with_preset,
    prepare_kernel_patch_with_preset_and_context,
};
use crate::sample::PreparedSamplerAssets;

use super::{load_kernel_definition_str, load_kernel_patch_file, load_kernel_patch_str};

const COMPLETE_PATCH: &str = r#"
metadata:
  name: reusable_voice
  version: "1.2"
  author: Test Author
static_params:
  - name: channels
    type: int
    default: 2
  - name: mode
    type: enum
    default: clean
    allowed_values: [clean, driven]
  - name: source
    type: string
    default: "fn process(ctx) {}"
  - name: impulse
    type: resource
    resource_kind: impulse_response
    default: { kind: impulse_response, path: room_ir.wav }
ports:
  - name: level
    direction: input
    signal: control
    channels: 1
    default: 0.75
    min: 0
    max: 1
    unit: linear
    maps_to: [amp.gain]
  - name: master
    direction: output
    signal: audio
    channels: $channels
    maps_from: [amp.audio_out]
module_definitions:
  - type: amplifier
    static_params:
      - name: channels
        type: int
        default: 1
      - name: mode
        type: enum
        allowed_values: [clean, driven]
      - name: label
        type: string
      - name: impulse
        type: resource
        resource_kind: impulse_response
    ports:
      - name: gain
        direction: input
        signal: control
        channels: 1
        default: 1
        min: 0
        max: 2
        unit: linear
        maps_to: [inner.gain]
      - name: audio_out
        direction: output
        signal: audio
        channels: $channels
        maps_from: [inner.audio_out]
    modules:
      - id: inner
        type: gain
    connections: []
modules:
  - id: amp
    type: amplifier
    static:
      channels: $channels
      mode: driven
      label: main
      impulse: { kind: impulse_response, path: room_ir.wav }
    defaults:
      gain: 0.5
connections:
  - from: amp.audio_out
    to: amp.audio_in
"#;

const SAMPLE_ASSET_PATCH: &str = r#"
metadata: { name: sample_asset_patch }
assets:
  sample_sources:
    - id: break
      path: samples/break.wav
      regions:
        - id: full
          start_frame: 0
          end_frame: 96000
          root_note: 60
          gain_db: -3
          pan: 0.25
          reverse: false
          fade_in_ms: 2
          fade_out_ms: 3
          loop: { mode: forward, start_frame: 24000, end_frame: 48000, crossfade_ms: 5 }
      slices:
        - { id: beat_1, start_frame: 0, end_frame: 24000 }
        - { id: beat_2, start_frame: 24000, end_frame: 48000 }
      cues:
        - { id: downbeat, frame: 0 }
      analysis:
        tempo_bpm: 120
        confidence: 0.95
        beat_grid: { unit: frames, beats: [0, 24000, 48000], downbeats: [0, 48000] }
  sample_maps:
    - id: kit
      selection_seed: 7
      selection_mode: round_robin
      zones:
        - { id: soft, region: break.full, key_range: [36, 36], velocity_range: [1, 70], round_robin_group: kick, choke_group: hats, weight: 2 }
        - { id: loud, region: break.full, key_range: [36, 36], velocity_range: [71, 127], round_robin_group: kick, choke_group: hats, weight: 3 }
ports:
  - { name: audio_out, direction: output, signal: audio, channels: 1, maps_from: osc.audio }
modules:
  - { id: osc, type: oscillator }
"#;

#[test]
fn kernel_patch_accepts_sample_sources_regions_slices_maps_and_choke_metadata() {
    let patch = load_kernel_patch_str(SAMPLE_ASSET_PATCH).expect("sample assets should load");
    assert_eq!(patch.root().name(), "sample_asset_patch");
    let assets = patch.sample_assets();
    assert_eq!(assets.sample_sources.len(), 1);
    let source = &assets.sample_sources[0];
    assert_eq!(source.id, "break");
    assert_eq!(source.resource.kind(), ResourceKind::Sample);
    assert_eq!(source.resource.path().to_str(), Some("samples/break.wav"));
    assert_eq!(source.resource.origin(), &ResourceOrigin::Document);
    assert_eq!(source.regions.len(), 1);
    let region = &source.regions[0];
    assert_eq!(region.id, "full");
    assert_eq!((region.start_frame, region.end_frame), (0, 96_000));
    assert_eq!(region.root_note, Some(60));
    assert_eq!(region.gain_db, Some(-3.0));
    assert_eq!(region.pan, Some(0.25));
    assert!(!region.reverse);
    assert_eq!(
        (region.fade_in_ms, region.fade_out_ms),
        (Some(2.0), Some(3.0))
    );
    let loop_settings = region.loop_settings.as_ref().expect("loop settings");
    assert_eq!(loop_settings.mode, "forward");
    assert_eq!(
        (loop_settings.start_frame, loop_settings.end_frame),
        (24_000, 48_000)
    );
    assert_eq!(loop_settings.crossfade_ms, Some(5.0));
    assert_eq!(
        source
            .slices
            .iter()
            .map(|slice| slice.id.as_str())
            .collect::<Vec<_>>(),
        vec!["beat_1", "beat_2"]
    );
    assert_eq!(
        (source.slices[1].start_frame, source.slices[1].end_frame),
        (24_000, 48_000)
    );
    assert_eq!(
        (source.cues[0].id.as_str(), source.cues[0].frame),
        ("downbeat", 0)
    );
    let analysis = source.analysis.as_ref().expect("analysis metadata");
    assert_eq!(
        (analysis.tempo_bpm, analysis.confidence),
        (Some(120.0), Some(0.95))
    );
    let grid = analysis.beat_grid.as_ref().expect("explicit beat grid");
    assert_eq!(grid.unit.as_deref(), Some("frames"));
    assert_eq!(grid.beats, vec![0, 24_000, 48_000]);
    assert_eq!(grid.downbeats, vec![0, 48_000]);

    assert_eq!(assets.sample_maps.len(), 1);
    let map = &assets.sample_maps[0];
    assert_eq!(map.id, "kit");
    assert_eq!(map.selection_seed, 7);
    assert_eq!(map.selection_mode.as_deref(), Some("round_robin"));
    assert_eq!(map.zones.len(), 2);
    assert_eq!(map.zones[0].id.as_deref(), Some("soft"));
    assert_eq!(map.zones[0].region, "break.full");
    assert_eq!(map.zones[0].key_range, [36, 36]);
    assert_eq!(map.zones[0].velocity_range, [1, 70]);
    assert_eq!(map.zones[0].round_robin_group.as_deref(), Some("kick"));
    assert_eq!(map.zones[0].choke_group.as_deref(), Some("hats"));
    assert_eq!(map.zones[0].weight, Some(2));
    assert_eq!(map.zones[1].velocity_range, [71, 127]);
}

#[test]
fn packaged_sample_source_retains_its_package_version_root() {
    let package_root = std::path::PathBuf::from("/library/1.2.3");
    let patch = load_kernel_definition_str(
        SAMPLE_ASSET_PATCH,
        "packaged_sample",
        ResourceOrigin::Package(package_root.clone()),
    )
    .expect("packaged sample declaration should load");

    assert_eq!(
        patch.sample_assets().sample_sources[0].resource.origin(),
        &ResourceOrigin::Package(package_root)
    );
}

#[test]
fn packaged_sample_source_prepares_from_its_version_root() {
    let package = tempfile::tempdir().expect("package root");
    let document = tempfile::tempdir().expect("separate document root");
    fs::create_dir(package.path().join("samples")).expect("package sample directory");
    let frames = vec![0.375; 96_000];
    crate::wav::write_wav_stereo_i16(
        fs::File::create(package.path().join("samples/break.wav")).expect("package sample"),
        48_000,
        &frames,
        &frames,
    )
    .expect("write package sample");
    let patch = load_kernel_definition_str(
        SAMPLE_ASSET_PATCH,
        "packaged_sample",
        ResourceOrigin::Package(package.path().to_path_buf()),
    )
    .expect("packaged declaration loads");
    let context = PreparationContext::new(document.path(), 48_000);
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 16,
        duration_frames: 16,
    };

    let prepared = prepare_kernel_patch_with_context(&patch, &settings, &context)
        .expect("sample resolves under package root");
    let source = &prepared.sample_assets().sources()[0];
    assert_eq!(source.id(), "break");
    assert!((source.sample().frames()[0] - 0.375).abs() < 0.0001);
}

#[test]
fn unknown_sampling_asset_fields_fail_schema_validation() {
    for (target, replacement) in [
        (
            "path: samples/break.wav",
            "path: samples/break.wav\n      unknown_sampling_field: 1",
        ),
        (
            "root_note: 60",
            "root_note: 60\n          unknown_sampling_field: 1",
        ),
        (
            "{ id: beat_1, start_frame: 0, end_frame: 24000 }",
            "{ id: beat_1, start_frame: 0, end_frame: 24000, unknown_sampling_field: 1 }",
        ),
        (
            "selection_mode: round_robin",
            "selection_mode: round_robin\n      unknown_sampling_field: 1",
        ),
        (
            "id: soft, region: break.full",
            "id: soft, unknown_sampling_field: 1, region: break.full",
        ),
    ] {
        let invalid = SAMPLE_ASSET_PATCH.replacen(target, replacement, 1);
        assert_ne!(
            invalid, SAMPLE_ASSET_PATCH,
            "test fixture substitution must apply"
        );
        let error = load_kernel_patch_str(&invalid).expect_err("unknown asset key must fail");
        let diagnostic = error.errors().next().expect("schema diagnostic");
        assert_eq!(
            diagnostic.error_code(),
            error_codes::KERNEL_DOCUMENT_SCHEMA_FAILED
        );
        assert!(diagnostic.message().contains("unknown_sampling_field"));
    }
}

#[test]
fn streaming_granular_and_workstation_sample_declarations_are_deferred() {
    for (target, replacement) in [
        (
            "path: samples/break.wav",
            "path: samples/break.wav\n      streaming: true",
        ),
        (
            "path: samples/break.wav",
            "path: samples/break.wav\n      grain_size_ms: 25",
        ),
        (
            "path: samples/break.wav",
            "path: samples/break.wav\n      time_stretch: beat_sync",
        ),
        (
            "id: soft, region: break.full",
            "id: soft, keyswitch: 24, region: break.full",
        ),
        (
            "id: soft, region: break.full",
            "id: soft, release_trigger: true, region: break.full",
        ),
    ] {
        let invalid = SAMPLE_ASSET_PATCH.replacen(target, replacement, 1);
        let result = load_kernel_patch_str(&invalid);
        let error = match result {
            Ok(_) => panic!("out-of-scope sampling field must fail schema validation"),
            Err(error) => error,
        };
        assert_eq!(
            error.errors().next().unwrap().error_code(),
            error_codes::KERNEL_DOCUMENT_SCHEMA_FAILED
        );
    }
}

#[test]
fn sample_asset_preparation_reports_invalid_region_loop_and_drum_zone_ranges() {
    let directory = tempfile::tempdir().expect("temporary sample root");
    let context = PreparationContext::new(directory.path(), 48_000);
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 16,
        duration_frames: 16,
    };
    for (target, replacement, code) in [
        (
            "end_frame: 96000",
            "end_frame: 0",
            error_codes::KERNEL_SAMPLE_INVALID_REGION,
        ),
        (
            "end_frame: 48000, crossfade_ms: 5",
            "end_frame: 24000, crossfade_ms: 5",
            error_codes::KERNEL_SAMPLE_INVALID_LOOP,
        ),
        (
            "key_range: [36, 36]",
            "key_range: [37, 36]",
            error_codes::KERNEL_SAMPLE_INVALID_ZONE,
        ),
        (
            "velocity_range: [1, 70]",
            "velocity_range: [70, 1]",
            error_codes::KERNEL_SAMPLE_INVALID_ZONE,
        ),
    ] {
        let invalid = SAMPLE_ASSET_PATCH.replacen(target, replacement, 1);
        let patch = load_kernel_patch_str(&invalid).expect("shape is valid");
        let error = prepare_kernel_patch_with_context(&patch, &settings, &context)
            .expect_err("invalid sample declaration must fail preparation");
        assert_eq!(
            error.diagnostics().errors().next().unwrap().error_code(),
            code
        );
    }
}

#[test]
fn sample_asset_preparation_reports_missing_file_with_source_identity() {
    let directory = tempfile::tempdir().expect("temporary sample root");
    let context = PreparationContext::new(directory.path(), 48_000);
    let patch = load_kernel_patch_str(SAMPLE_ASSET_PATCH).expect("sample declaration loads");
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 16,
        duration_frames: 16,
    };

    let error = prepare_kernel_patch_with_context(&patch, &settings, &context)
        .expect_err("missing sample must fail preparation");
    let diagnostic = error.diagnostics().errors().next().unwrap();
    assert_eq!(
        diagnostic.error_code(),
        error_codes::KERNEL_SAMPLE_MISSING_FILE
    );
    assert!(diagnostic.message().contains("break"));
    assert!(diagnostic.message().contains("samples/break.wav"));
}

#[test]
fn prepared_drum_source_keeps_decoded_frames_regions_and_map_metadata() {
    let directory = tempfile::tempdir().expect("temporary sample root");
    fs::create_dir(directory.path().join("samples")).expect("sample directory");
    let frames = vec![0.25; 96_000];
    crate::wav::write_wav_stereo_i16(
        fs::File::create(directory.path().join("samples/break.wav")).expect("sample file"),
        48_000,
        &frames,
        &frames,
    )
    .expect("write sample");
    let context = PreparationContext::new(directory.path(), 48_000);
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 16,
        duration_frames: 16,
    };
    let patch = load_kernel_patch_str(SAMPLE_ASSET_PATCH).expect("sample declaration loads");

    let prepared = prepare_kernel_patch_with_context(&patch, &settings, &context)
        .expect("valid drum assets prepare");
    let source = &prepared.sample_assets().sources()[0];
    assert_eq!(source.id(), "break");
    assert_eq!(source.sample().sample_rate_hz(), 48_000);
    assert_eq!(source.sample().frames().len(), 96_000);
    assert!((source.sample().frames()[0] - 0.25).abs() < 0.0001);
    assert_eq!(source.declaration().regions[0].end_frame, 96_000);
    assert_eq!(prepared.sample_assets().maps()[0].zones.len(), 2);
}

#[test]
fn sample_source_rejects_unsupported_decode_format_and_out_of_bounds_region() {
    let directory = tempfile::tempdir().expect("temporary sample root");
    fs::create_dir(directory.path().join("samples")).expect("sample directory");
    let path = directory.path().join("samples/break.wav");
    fs::write(&path, b"not a WAV file").expect("write invalid sample");
    let context = PreparationContext::new(directory.path(), 48_000);
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 16,
        duration_frames: 16,
    };
    let patch = load_kernel_patch_str(SAMPLE_ASSET_PATCH).expect("sample declaration loads");

    let error = prepare_kernel_patch_with_context(&patch, &settings, &context)
        .expect_err("unsupported sample must fail preparation");
    assert_eq!(
        error.diagnostics().errors().next().unwrap().error_code(),
        error_codes::KERNEL_SAMPLE_UNSUPPORTED_FORMAT
    );

    let frames = vec![0.25; 95_999];
    crate::wav::write_wav_stereo_i16(fs::File::create(&path).unwrap(), 48_000, &frames, &frames)
        .expect("write valid short sample");
    let error = prepare_kernel_patch_with_context(&patch, &settings, &context)
        .expect_err("region beyond sample must fail preparation");
    assert_eq!(
        error.diagnostics().errors().next().unwrap().error_code(),
        error_codes::KERNEL_SAMPLE_INVALID_REGION
    );
}

#[test]
fn sample_source_reports_sample_rate_mismatch_as_load_failure() {
    let directory = tempfile::tempdir().expect("temporary sample root");
    fs::create_dir(directory.path().join("samples")).expect("sample directory");
    let path = directory.path().join("samples/break.wav");
    crate::wav::write_wav_stereo_i16(fs::File::create(&path).unwrap(), 44_100, &[0.5], &[0.5])
        .expect("write sample at another rate");
    let patch = load_kernel_patch_str(SAMPLE_ASSET_PATCH).expect("sample declaration loads");
    let context = PreparationContext::new(directory.path(), 48_000);
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 16,
        duration_frames: 16,
    };

    let result = prepare_kernel_patch_with_context(&patch, &settings, &context);
    let error = match result {
        Ok(_) => panic!("sample rate mismatch must fail preparation"),
        Err(error) => error,
    };
    let diagnostic = error.diagnostics().errors().next().unwrap();
    assert_eq!(
        diagnostic.error_code(),
        error_codes::KERNEL_RESOURCE_LOAD_FAILED
    );
    assert!(diagnostic.message().contains("break"));
    assert!(diagnostic.message().contains("sample-rate mismatch"));
}

#[test]
fn sample_assets_require_a_resource_context_and_reject_path_escape() {
    let patch = load_kernel_patch_str(SAMPLE_ASSET_PATCH).expect("sample declaration loads");
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 16,
        duration_frames: 16,
    };
    let error = crate::preparation::prepare_kernel_patch(&patch, &settings)
        .expect_err("sample declarations need a document root");
    assert_eq!(
        error.diagnostics().errors().next().unwrap().error_code(),
        error_codes::KERNEL_SAMPLE_CONTEXT_REQUIRED
    );

    let escaped = SAMPLE_ASSET_PATCH.replace("samples/break.wav", "../outside.wav");
    let patch = load_kernel_patch_str(&escaped).expect("path shape loads");
    let directory = tempfile::tempdir().expect("temporary resource root");
    let context = PreparationContext::new(directory.path(), 48_000);
    let error = prepare_kernel_patch_with_context(&patch, &settings, &context)
        .expect_err("escaping path must fail preparation");
    assert_eq!(
        error.diagnostics().errors().next().unwrap().error_code(),
        error_codes::KERNEL_RESOURCE_PATH_ESCAPE
    );
}

#[test]
fn sample_asset_preparation_rejects_unsupported_loop_and_invalid_region_values() {
    let directory = tempfile::tempdir().expect("temporary sample root");
    let context = PreparationContext::new(directory.path(), 48_000);
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 16,
        duration_frames: 16,
    };
    for (target, replacement, code) in [
        (
            "mode: forward",
            "mode: ping_pong",
            error_codes::KERNEL_SAMPLE_UNSUPPORTED_MODE,
        ),
        (
            "pan: 0.25",
            "pan: 2",
            error_codes::KERNEL_SAMPLE_INVALID_REGION,
        ),
        (
            "fade_in_ms: 2",
            "fade_in_ms: -1",
            error_codes::KERNEL_SAMPLE_INVALID_REGION,
        ),
        (
            "crossfade_ms: 5",
            "crossfade_ms: -5",
            error_codes::KERNEL_SAMPLE_INVALID_LOOP,
        ),
    ] {
        let invalid = SAMPLE_ASSET_PATCH.replacen(target, replacement, 1);
        let patch = load_kernel_patch_str(&invalid).expect("shape is valid");
        let result = prepare_kernel_patch_with_context(&patch, &settings, &context);
        let error = match result {
            Ok(_) => panic!("invalid {target} must fail preparation"),
            Err(error) => error,
        };
        assert_eq!(
            error.diagnostics().errors().next().unwrap().error_code(),
            code
        );
    }
}

#[test]
fn drum_map_player_rejects_unbounded_or_zero_voice_limits() {
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 16,
        duration_frames: 16,
    };
    for limit in [0, 4096] {
        let yaml = format!(
            "metadata: {{ name: drum_map }}\nports:\n  - {{ name: audio_out, direction: output, signal: audio, channels: 2, maps_from: player.audio }}\nmodules:\n  - {{ id: player, type: sample_map_player, static: {{ sample_map: kit, max_voices: {limit} }} }}\n"
        );
        let patch = load_kernel_patch_str(&yaml).expect("sampler graph loads");
        let result = crate::preparation::prepare_kernel_patch(&patch, &settings);
        let error = match result {
            Ok(_) => panic!("invalid voice limit {limit} must fail"),
            Err(error) => error,
        };
        assert_eq!(
            error.diagnostics().errors().next().unwrap().error_code(),
            error_codes::KERNEL_SAMPLE_INVALID_VOICE_LIMIT
        );
    }
}

#[test]
fn sample_modules_reject_unsupported_interpolation_and_choke_modes() {
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 16,
        duration_frames: 16,
    };
    for (kind, static_args) in [
        (
            "sample_player",
            "source: break, region: full, interpolation: sinc",
        ),
        ("sample_map_player", "sample_map: kit, choke_mode: teleport"),
    ] {
        let yaml = format!(
            "metadata: {{ name: invalid_sample_mode }}\nports:\n  - {{ name: audio_out, direction: output, signal: audio, channels: 2, maps_from: player.audio }}\nmodules:\n  - {{ id: player, type: {kind}, static: {{ {static_args} }} }}\n"
        );
        let patch = load_kernel_patch_str(&yaml).expect("sampler graph shape loads");
        let result = crate::preparation::prepare_kernel_patch(&patch, &settings);
        let error = match result {
            Ok(_) => panic!("unsupported {kind} mode must fail"),
            Err(error) => error,
        };
        assert_eq!(
            error.diagnostics().errors().next().unwrap().error_code(),
            error_codes::KERNEL_STATIC_ARGUMENT_INVALID_ENUM_VALUE
        );
    }
}

#[test]
fn kernel_cli_overrides_resolve_against_declared_static_and_control_types() {
    let mut patch = load_kernel_patch_str(COMPLETE_PATCH).expect("patch loads");
    for (name, value) in [
        ("channels", "1"),
        ("mode", "clean"),
        ("label", "123"),
        ("impulse", "other.wav"),
        ("gain", "0.8"),
    ] {
        patch
            .apply_cli_override("amp", name, value, None)
            .expect("declared CLI value applies");
    }
    let amp = patch
        .root()
        .nodes()
        .iter()
        .find(|node| node.id().as_str() == "amp")
        .expect("amp node");
    assert_eq!(
        amp.static_args().get("channels"),
        Some(&StaticArg::Literal(StaticValue::Int(1)))
    );
    assert_eq!(
        amp.static_args().get("mode"),
        Some(&StaticArg::Literal(StaticValue::Enum("clean".to_string())))
    );
    assert_eq!(
        amp.static_args().get("label"),
        Some(&StaticArg::Literal(StaticValue::String("123".to_string())))
    );
    assert_eq!(
        amp.static_args().get("impulse"),
        Some(&StaticArg::Literal(StaticValue::Resource(
            ResourceRef::new(
                ResourceKind::ImpulseResponse,
                "other.wav",
                ResourceOrigin::Document,
            )
        )))
    );
    assert_eq!(amp.port_default_overrides().get("gain"), Some(&0.8));
    assert_eq!(
        patch.root().ports()[0].control_default().unwrap().default(),
        0.8
    );
}

#[test]
fn kernel_cli_overrides_reject_unknown_targets_types_and_ranges() {
    let patch = load_kernel_patch_str(COMPLETE_PATCH).expect("patch loads");
    for (module, name, value) in [
        ("missing", "gain", "1"),
        ("amp", "missing", "1"),
        ("amp", "channels", "1.5"),
        ("amp", "channels", "inf"),
        ("amp", "channels", "9223372036854775808"),
        ("amp", "mode", "invalid"),
        ("amp", "mode", "true"),
        ("amp", "gain", "true"),
        ("amp", "gain", "3"),
        ("amp", "gain", "1.5"),
        ("amp", "audio_out", "1"),
    ] {
        let mut candidate = patch.clone();
        let diagnostics = candidate
            .apply_cli_override(module, name, value, None)
            .expect_err("invalid CLI override must fail");
        assert_eq!(diagnostics.errors().count(), 1, "{module}.{name}");
        assert!(
            diagnostics
                .to_string()
                .contains(&format!("{module}.{name}")),
            "{module}.{name}: {diagnostics}"
        );
        assert_eq!(
            candidate.root(),
            patch.root(),
            "failure cannot mutate patch"
        );
    }
}

#[test]
fn kernel_cli_override_rejects_module_without_resolved_definition() {
    let mut patch = load_kernel_patch_str(
        "ports:\n  - { name: out, direction: output, signal: audio, channels: 1 }\nmodules:\n  - { id: future, type: future_module }\nconnections: []\n",
    )
    .expect("unresolved definition is preserved by loading");
    let diagnostics = patch
        .apply_cli_override("future", "count", "2", None)
        .expect_err("override needs a resolved declaration");
    assert!(diagnostics.to_string().contains("future_module"));
    assert!(diagnostics.to_string().contains("future.count"));
}

#[test]
fn kernel_cli_override_uses_only_the_matching_external_definition() {
    let reference = "$USER_LIB/demo/demo.yaml";
    let patch = load_kernel_patch_str(
        "ports:\n  - { name: master, direction: output, signal: audio, channels: 1, maps_from: voice.audio }\nmodules:\n  - { id: voice, type: $USER_LIB/demo/demo.yaml }\nconnections: []\n",
    )
    .expect("external reference loads");
    let level = Port::input("level", SignalType::Control, 1u32)
        .with_control_default(ControlDefault::new(0.5).with_min(0.0).with_max(1.0));
    let matching = GraphDefinition::new(reference).with_port(level.clone());
    let unrelated = GraphDefinition::new("$USER_LIB/other/other.yaml").with_port(level);

    let mut rejected = patch.clone();
    let diagnostics = rejected
        .apply_cli_override("voice", "level", "0.4", Some(&unrelated))
        .expect_err("a declaration from another package is not authoritative");
    assert!(diagnostics.to_string().contains(reference));
    assert_eq!(rejected.root(), patch.root());

    let mut applied = patch;
    applied
        .apply_cli_override("voice", "level", "0.4", Some(&matching))
        .expect("matching package declaration accepts the override");
    assert_eq!(
        applied.root().nodes()[0]
            .port_default_overrides()
            .get("level"),
        Some(&0.4)
    );
}

#[test]
fn kernel_cli_override_validates_integer_choices_and_control_port_kind() {
    let mut reverb = load_kernel_patch_str(
        "ports:\n  - { name: master, direction: output, signal: audio, channels: 2, maps_from: effect.audio_out }\nmodules:\n  - { id: effect, type: reverb }\nconnections: []\n",
    )
    .expect("reverb patch loads");
    reverb
        .apply_cli_override("effect", "channels", "1", None)
        .expect("mono is a declared channel choice");
    assert_eq!(
        reverb.root().nodes()[0].static_args().get("channels"),
        Some(&StaticArg::Literal(StaticValue::Int(1)))
    );
    let before = reverb.root().clone();
    assert!(
        reverb
            .apply_cli_override("effect", "channels", "3", None)
            .is_err()
    );
    assert!(
        reverb
            .apply_cli_override("effect", "audio_in", "1", None)
            .is_err()
    );
    assert_eq!(reverb.root(), &before);
}

#[test]
fn kernel_cli_override_validates_builtin_control_range_boundaries() {
    let patch = load_kernel_patch_str(
        "ports:\n  - { name: master, direction: output, signal: audio, channels: 1, maps_from: amp.audio_out }\nmodules:\n  - { id: amp, type: gain }\nconnections: []\n",
    )
    .expect("gain patch loads");
    for value in ["0", "4"] {
        let mut candidate = patch.clone();
        candidate
            .apply_cli_override("amp", "gain", value, None)
            .expect("inclusive boundary is valid");
        assert_eq!(
            candidate.root().nodes()[0]
                .port_default_overrides()
                .get("gain"),
            Some(&value.parse::<f64>().unwrap())
        );
    }
    for value in ["-0.1", "4.1", "NaN"] {
        let mut candidate = patch.clone();
        assert!(
            candidate
                .apply_cli_override("amp", "gain", value, None)
                .is_err()
        );
        assert_eq!(candidate.root(), patch.root());
    }
}

#[test]
fn kernel_cli_override_validates_mapped_root_control_range_boundaries() {
    let patch = load_kernel_patch_str(
        "ports:\n  - { name: level, direction: input, signal: control, channels: 1, default: 0.75, min: 0.5, max: 1, maps_to: amp.gain }\n  - { name: master, direction: output, signal: audio, channels: 1, maps_from: amp.audio_out }\nmodules:\n  - { id: amp, type: gain }\nconnections: []\n",
    )
    .expect("mapped root control loads");
    for (raw, expected) in [("0.5", 0.5), ("1", 1.0)] {
        let mut candidate = patch.clone();
        candidate
            .apply_cli_override("amp", "gain", raw, None)
            .expect("root boundary is inclusive");
        assert_eq!(
            candidate.root().ports()[0]
                .control_default()
                .unwrap()
                .default(),
            expected
        );
    }
    for raw in ["0.25", "1.25"] {
        let mut candidate = patch.clone();
        assert!(
            candidate
                .apply_cli_override("amp", "gain", raw, None)
                .is_err()
        );
        assert_eq!(candidate.root(), patch.root());
    }
}

#[test]
fn kernel_cli_override_changes_only_the_targeted_root_control_default() {
    let mut patch = load_kernel_patch_str(
        "ports:\n  - { name: left_level, direction: input, signal: control, channels: 1, default: 0.75, min: 0, max: 1, maps_to: left.gain }\n  - { name: right_level, direction: input, signal: control, channels: 1, default: 0.6, min: 0.5, max: 1, maps_to: right.gain }\n  - { name: master, direction: output, signal: audio, channels: 1, maps_from: left.audio_out }\nmodules:\n  - { id: left, type: gain }\n  - { id: right, type: gain }\nconnections: []\n",
    )
    .expect("two independent root controls load");
    patch
        .apply_cli_override("left", "gain", "0.2", None)
        .expect("one destination is unambiguous");
    let defaults = patch
        .root()
        .ports()
        .iter()
        .filter_map(|port| {
            port.control_default()
                .map(|default| (port.name(), default.default()))
        })
        .collect::<Vec<_>>();
    assert_eq!(defaults, [("left_level", 0.2), ("right_level", 0.6)]);
}

#[test]
fn kernel_cli_override_rejects_shared_root_control_without_mutating_patch() {
    let mut patch = load_kernel_patch_str(
        "ports:\n  - { name: level, direction: input, signal: control, channels: 1, default: 0.75, min: 0, max: 1, maps_to: [left.gain, right.gain] }\n  - { name: master, direction: output, signal: audio, channels: 1, maps_from: left.audio_out }\nmodules:\n  - { id: left, type: gain }\n  - { id: right, type: gain }\nconnections: []\n",
    )
    .expect("shared root control loads");
    let before = patch.root().clone();
    let diagnostics = patch
        .apply_cli_override("left", "gain", "0.2", None)
        .expect_err("target-specific override cannot change both destinations");
    assert!(diagnostics.to_string().contains("maps to multiple inputs"));
    assert_eq!(patch.root(), &before);
}

const PRESET_PATCH: &str = r#"
metadata: { name: preset_test }
instrument: { id: test.instrument, preset_schema_version: 2 }
static_params:
  - name: sample
    type: resource
    resource_kind: sample
    default: { kind: sample, path: original.wav }
ports:
  - { name: volume, direction: input, signal: control, channels: 1, default: 0.75, min: 0, max: 1, maps_to: amp.gain }
  - { name: out, direction: output, signal: audio, channels: 1, maps_from: amp.audio_out }
preset_surface:
  parameters:
    - { name: loudness, maps_to: volume }
  assets:
    - { name: hit, maps_to: sample }
modules:
  - { id: osc, type: oscillator }
  - { id: amp, type: gain }
connections:
  - { from: osc.audio, to: amp.audio_in }
"#;

#[test]
fn kernel_preset_surface_preserves_aliased_root_metadata() {
    let patch = load_kernel_patch_str(PRESET_PATCH).expect("preset patch loads");

    let identity = patch.instrument().expect("instrument identity");
    assert_eq!(identity.id, "test.instrument");
    assert_eq!(identity.preset_schema_version, 2);
    let value = &patch.preset_surface().parameters()[0];
    assert_eq!(value.name(), "loudness");
    assert_eq!(value.port_name(), "volume");
    assert_eq!(value.control_default().default(), 0.75);
    assert_eq!(value.control_default().min(), Some(0.0));
    assert_eq!(value.control_default().max(), Some(1.0));
    let asset = &patch.preset_surface().assets()[0];
    assert_eq!(asset.name(), "hit");
    assert_eq!(asset.static_param_name(), "sample");
    assert_eq!(asset.kind(), ResourceKind::Sample);
    assert_eq!(
        asset.default().unwrap().path().to_str(),
        Some("original.wav")
    );
}

#[test]
fn kernel_preset_application_changes_root_defaults_before_flattening() {
    let patch = load_kernel_patch_str(PRESET_PATCH).expect("preset patch loads");
    let preset = load_preset_str(
        "name: loud\ninstrument: { id: test.instrument, preset_schema_version: 2 }\nvalues: { loudness: 0.4 }\nassets: { hit: alternate.wav }\n",
    )
    .expect("preset loads");

    let applied = patch.apply_preset(&preset).expect("preset applies");
    let flat = applied
        .root()
        .flatten(applied.registry())
        .expect("flattens");

    assert_eq!(
        applied.root().ports()[0]
            .control_default()
            .unwrap()
            .default(),
        0.4
    );
    assert_eq!(
        flat.node(&crate::kernel::NodeId::new("amp"))
            .unwrap()
            .port_defaults()["gain"],
        0.4
    );
    assert_eq!(
        applied.root().static_params()[0].default(),
        Some(&StaticValue::Resource(ResourceRef::new(
            ResourceKind::Sample,
            "alternate.wav",
            ResourceOrigin::Document,
        )))
    );
    assert_eq!(
        patch.root().ports()[0].control_default().unwrap().default(),
        0.75
    );
}

#[test]
fn kernel_preset_surface_rejects_duplicate_or_unresolved_aliases() {
    let duplicate = PRESET_PATCH.replace(
        "  assets:\n    - { name: hit, maps_to: sample }",
        "  assets:\n    - { name: loudness, maps_to: sample }",
    );
    let error = load_kernel_patch_str(&duplicate).expect_err("duplicate alias fails");
    assert!(
        error
            .to_string()
            .contains("duplicate preset target loudness")
    );

    let missing = PRESET_PATCH.replace("maps_to: volume }", "maps_to: missing }");
    let error = load_kernel_patch_str(&missing).expect_err("missing destination fails");
    assert!(
        error
            .to_string()
            .contains("unresolved control input missing")
    );

    let unnamed = PRESET_PATCH.replace(
        "name: loudness, maps_to: volume",
        "name: '', maps_to: volume",
    );
    let error = load_kernel_patch_str(&unnamed).expect_err("unnamed alias fails");
    assert_eq!(
        error.errors().next().unwrap().error_code(),
        error_codes::KERNEL_DOCUMENT_SCHEMA_FAILED,
    );
    assert!(
        error
            .to_string()
            .contains("/preset_surface/parameters/0/name")
    );

    let no_default =
        PRESET_PATCH.replace("    default: { kind: sample, path: original.wav }\n", "");
    let error = load_kernel_patch_str(&no_default).expect_err("asset alias needs default");
    assert!(
        error
            .to_string()
            .contains("requires a declared resource default")
    );
}

#[test]
fn kernel_preset_rejects_identity_unknown_targets_and_incompatible_values() {
    let patch = load_kernel_patch_str(PRESET_PATCH).expect("preset patch loads");
    for (yaml, expected) in [
        (
            "name: wrong\ninstrument: { id: other, preset_schema_version: 2 }\n",
            "does not match patch instrument",
        ),
        (
            "name: wrong-version\ninstrument: { id: test.instrument, preset_schema_version: 3 }\n",
            "does not match patch instrument",
        ),
        (
            "name: unknown\ninstrument: { id: test.instrument, preset_schema_version: 2 }\nvalues: { other: 0.5 }\n",
            "unknown preset target other",
        ),
        (
            "name: internal\ninstrument: { id: test.instrument, preset_schema_version: 2 }\nvalues: { 'amp.gain': 0.5 }\n",
            "unknown preset target amp.gain",
        ),
        (
            "name: type\ninstrument: { id: test.instrument, preset_schema_version: 2 }\nvalues: { loudness: loud }\n",
            "incompatible type or range",
        ),
        (
            "name: range\ninstrument: { id: test.instrument, preset_schema_version: 2 }\nvalues: { loudness: 2.0 }\n",
            "incompatible type or range",
        ),
        (
            "name: structural\ninstrument: { id: test.instrument, preset_schema_version: 2 }\nmodules: []\n",
            "preset cannot declare structural field modules",
        ),
        (
            "name: connection\ninstrument: { id: test.instrument, preset_schema_version: 2 }\nconnections: []\n",
            "preset cannot declare structural field connections",
        ),
        (
            "name: unknown-asset\ninstrument: { id: test.instrument, preset_schema_version: 2 }\nassets: { extra: other.wav }\n",
            "unknown preset target extra",
        ),
    ] {
        let preset = load_preset_str(yaml).expect("preset YAML loads");
        let error = patch
            .apply_preset(&preset)
            .expect_err("incompatible preset fails");
        assert!(error.to_string().contains(expected), "{error}");
    }
}

#[test]
fn preset_application_requires_instrument_identity_and_known_asset_target() {
    let preset = load_preset_str(
        "name: missing\ninstrument: { id: test.instrument, preset_schema_version: 2 }\n",
    )
    .expect("preset loads");
    let patch = load_kernel_patch_str(&PRESET_PATCH.replace(
        "instrument: { id: test.instrument, preset_schema_version: 2 }\n",
        "",
    ))
    .expect("patch without preset identity loads");
    let error = patch
        .apply_preset(&preset)
        .expect_err("identity is required");
    assert!(
        error
            .to_string()
            .contains("does not declare instrument preset identity")
    );

    let patch = load_kernel_patch_str(PRESET_PATCH).expect("preset patch loads");
    let unknown_asset = load_preset_str(
        "name: unknown\ninstrument: { id: test.instrument, preset_schema_version: 2 }\nassets: { other: alternate.wav }\n",
    )
    .expect("preset loads");
    let error = patch
        .apply_preset(&unknown_asset)
        .expect_err("unknown asset target fails");
    assert!(error.to_string().contains("unknown preset target other"));
}

#[test]
fn preset_surface_rejects_unknown_asset_destination_and_duplicate_parameter_name() {
    let missing_asset = PRESET_PATCH.replace("maps_to: sample }", "maps_to: missing }");
    let error = load_kernel_patch_str(&missing_asset)
        .expect_err("asset target must name a resource static parameter");
    assert!(
        error
            .to_string()
            .contains("unresolved resource static parameter missing")
    );

    let duplicate_parameter = PRESET_PATCH.replace(
        "    - { name: loudness, maps_to: volume }",
        "    - { name: loudness, maps_to: volume }\n    - { name: loudness, maps_to: out }",
    );
    let error = load_kernel_patch_str(&duplicate_parameter)
        .expect_err("duplicate parameter target must fail");
    assert!(
        error
            .to_string()
            .contains("duplicate preset target loudness")
    );
}

#[test]
fn malformed_yaml_shapes_fail_schema_without_panicking() {
    for yaml in [
        "42",
        "module_definitions: [42]\nports: []\nmodules: []\nconnections: []\n",
        "ports: []\nmodules: [42]\nconnections: []\n",
    ] {
        let error = load_kernel_patch_str(yaml).expect_err("malformed document fails");
        assert_eq!(
            error.errors().next().unwrap().error_code(),
            error_codes::KERNEL_DOCUMENT_SCHEMA_FAILED
        );
    }
}

#[test]
fn non_string_yaml_mapping_keys_fail_before_graph_construction() {
    let yaml = "? [one, two]\n: 1\n";
    let error = load_kernel_patch_str(yaml).expect_err("non-JSON mapping key fails");
    assert_eq!(
        error.errors().next().unwrap().error_code(),
        error_codes::KERNEL_DOCUMENT_SCHEMA_FAILED
    );
    assert!(error.to_string().contains("cannot be represented as JSON"));
}

#[test]
fn unresolved_node_static_literals_keep_inferred_types() {
    let patch = load_kernel_patch_str(
        "ports:\n  - { name: out, direction: output, signal: audio, channels: 1 }\nmodules:\n  - { id: future, type: future_module, static: { count: 3, label: bright, sample: { kind: sample, path: hit.wav } } }\nconnections: []\n",
    )
    .expect("unknown node type can retain its authored static values for later resolution");
    let args = patch.root().nodes()[0].static_args();
    assert_eq!(args["count"], StaticArg::Literal(StaticValue::Int(3)));
    assert_eq!(
        args["label"],
        StaticArg::Literal(StaticValue::String("bright".into()))
    );
    assert_eq!(
        args["sample"],
        StaticArg::Literal(StaticValue::Resource(ResourceRef::new(
            ResourceKind::Sample,
            "hit.wav",
            ResourceOrigin::Document,
        )))
    );
}

#[test]
fn node_static_literal_must_match_the_declared_type() {
    let yaml = "ports:\n  - { name: out, direction: output, signal: audio, channels: 1 }\nmodules:\n  - { id: source, type: control_to_audio, static: { channels: wide } }\nconnections: []\n";
    let error = load_kernel_patch_str(yaml).expect_err("noninteger channel count fails");
    assert_eq!(
        error.errors().next().unwrap().error_code(),
        error_codes::KERNEL_DOCUMENT_PARSE_FAILED
    );
    assert!(
        error
            .to_string()
            .contains("does not match declared type Int")
    );
}

#[test]
fn yaml_control_default_must_parse_as_a_number() {
    let yaml = "ports:\n  - { name: out, direction: output, signal: audio, channels: 1, maps_from: amp.audio_out }\nmodules:\n  - { id: amp, type: gain, defaults: { gain: loud } }\nconnections: []\n";
    let error = load_kernel_patch_str(yaml).expect_err("nonnumeric control default fails");
    assert_eq!(
        error.errors().next().unwrap().error_code(),
        error_codes::KERNEL_DOCUMENT_SCHEMA_FAILED
    );
    assert!(error.to_string().contains("gain"));
}

#[test]
fn yaml_static_expression_is_retained_for_explicit_rejection() {
    let yaml = "ports:\n  - { name: out, direction: output, signal: audio, channels: 1 }\nmodules:\n  - { id: source, type: control_to_audio, static: { channels: '$channels + 1' } }\nconnections: []\n";
    let patch = load_kernel_patch_str(yaml).expect("expression is parsed before graph validation");
    assert_eq!(
        patch.root().nodes()[0].static_args()["channels"],
        StaticArg::Expression("$channels + 1".into())
    );
    let error = patch
        .root()
        .flatten(patch.registry())
        .expect_err("arithmetic is unsupported");
    assert!(error.to_string().contains("expression"));
}

#[test]
fn malformed_port_reference_has_structured_parse_diagnostic() {
    let yaml = "ports:\n  - { name: out, direction: output, signal: audio, channels: 1, maps_from: source }\nmodules:\n  - { id: source, type: control_to_audio }\nconnections: []\n";
    let error = load_kernel_patch_str(yaml).expect_err("port reference must have module and port");
    assert_eq!(
        error.errors().next().unwrap().error_code(),
        error_codes::KERNEL_DOCUMENT_PARSE_FAILED
    );
    assert!(error.to_string().contains("module.port"));
}

#[test]
fn tagged_legacy_binding_is_rejected_before_schema_validation() {
    let yaml = "ports:\n  - { name: out, direction: output, signal: audio, channels: 1 }\nmodules:\n  - id: source\n    type: control_to_audio\n    static: { channels: !tag '${channels}' }\nconnections: []\n";
    let error = load_kernel_patch_str(yaml).expect_err("tagged legacy binding fails");
    assert_eq!(
        error.errors().next().unwrap().error_code(),
        error_codes::KERNEL_DOCUMENT_LEGACY_BINDING
    );
}

#[test]
fn missing_kernel_patch_file_has_read_diagnostic() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("absent.yaml");
    let error = load_kernel_patch_file(&path).expect_err("missing patch fails");
    assert_eq!(
        error.errors().next().unwrap().error_code(),
        error_codes::KERNEL_DOCUMENT_READ_FAILED
    );
    assert!(error.to_string().contains("absent.yaml"));
}

#[test]
fn kernel_preset_compatibility_diagnostics_identify_expected_and_actual_identity() {
    let patch = load_kernel_patch_str(PRESET_PATCH).expect("preset patch loads");
    for (yaml, expected, actual) in [
        (
            "name: wrong-id\ninstrument: { id: other, preset_schema_version: 2 }\n",
            "test.instrument",
            "other",
        ),
        (
            "name: wrong-version\ninstrument: { id: test.instrument, preset_schema_version: 3 }\n",
            "2",
            "3",
        ),
    ] {
        let preset = load_preset_str(yaml).expect("preset loads");
        let error = patch
            .apply_preset(&preset)
            .expect_err("incompatible preset fails");
        let diagnostic = error.errors().next().expect("compatibility diagnostic");
        assert_eq!(diagnostic.expected(), Some(expected));
        assert_eq!(diagnostic.actual(), Some(actual));
    }
}

#[test]
fn omitted_kernel_preset_targets_keep_root_defaults() {
    let patch = load_kernel_patch_str(PRESET_PATCH).expect("preset patch loads");
    let preset = load_preset_str(
        "name: plain\ninstrument: { id: test.instrument, preset_schema_version: 2 }\n",
    )
    .expect("preset loads");

    let applied = patch.apply_preset(&preset).expect("preset applies");
    assert_eq!(
        applied.root().ports()[0]
            .control_default()
            .unwrap()
            .default(),
        0.75
    );
    assert_eq!(
        applied.root().static_params()[0].default(),
        patch.root().static_params()[0].default()
    );
}

#[test]
fn kernel_preset_asset_passes_through_root_static_parameter_to_node() {
    let yaml = PRESET_PATCH.replace(
        "  - { id: amp, type: gain }",
        "  - { id: amp, type: gain }\n  - { id: hit, type: sampler, static: { sample: $sample } }",
    );
    let patch = load_kernel_patch_str(&yaml).expect("resource patch loads");
    let preset = load_preset_str(
        "name: alternate\ninstrument: { id: test.instrument, preset_schema_version: 2 }\nassets: { hit: alternate.wav }\n",
    )
    .expect("preset loads");

    let applied = patch.apply_preset(&preset).expect("preset applies");
    let flat = applied
        .root()
        .flatten(applied.registry())
        .expect("flattens");
    assert_eq!(
        flat.node(&crate::kernel::NodeId::new("hit"))
            .unwrap()
            .static_args()["sample"],
        StaticValue::Resource(ResourceRef::new(
            ResourceKind::Sample,
            "alternate.wav",
            ResourceOrigin::Document,
        ))
    );
}

#[test]
fn kernel_preset_asset_is_resolved_during_context_preparation() {
    let directory = tempfile::tempdir().expect("temporary resource root");
    for (name, sample) in [("original.wav", 0.25), ("alternate.wav", 0.75)] {
        crate::wav::write_wav_stereo_i16(
            fs::File::create(directory.path().join(name)).expect("create sample"),
            48_000,
            &[sample],
            &[sample],
        )
        .expect("write sample");
    }
    let yaml = PRESET_PATCH.replace(
        "  - { id: amp, type: gain }",
        "  - { id: amp, type: gain }\n  - { id: hit, type: sampler, static: { sample: $sample } }",
    );
    let patch = load_kernel_patch_str(&yaml).expect("resource patch loads");
    let preset = load_preset_str(
        "name: alternate\ninstrument: { id: test.instrument, preset_schema_version: 2 }\nassets: { hit: alternate.wav }\n",
    )
    .expect("preset loads");
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 16,
        duration_frames: 16,
    };
    let context = PreparationContext::new(directory.path(), 48_000);

    let prepared =
        prepare_kernel_patch_with_preset_and_context(&patch, &preset, &settings, &context)
            .expect("preset resource prepares");
    let sample = prepared
        .compiled_patch()
        .nodes()
        .iter()
        .find(|node| node.id.as_str() == "hit")
        .and_then(|node| node.resources.sample.as_ref())
        .expect("sampler has resolved sample");
    assert!((sample.frames()[0] - 0.75).abs() < 0.0001);
}

#[test]
fn incoming_control_connection_takes_precedence_over_preset_default() {
    let yaml = PRESET_PATCH
        .replace(
            "  - { id: amp, type: gain }",
            "  - { id: amp, type: gain }\n  - { id: follower, type: envelope_follower }",
        )
        .replace(
            "  - { from: osc.audio, to: amp.audio_in }",
            "  - { from: osc.audio, to: amp.audio_in }\n  - { from: osc.audio, to: follower.audio_in }\n  - { from: follower.value, to: amp.gain }",
        );
    let patch = load_kernel_patch_str(&yaml).expect("patch loads");
    let preset = load_preset_str(
        "name: quiet\ninstrument: { id: test.instrument, preset_schema_version: 2 }\nvalues: { loudness: 0.4 }\n",
    )
    .expect("preset loads");
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 128,
        duration_frames: 512,
    };

    let preset_prepared = prepare_kernel_patch_with_preset(&patch, &preset, &settings)
        .expect("preset patch prepares");
    let default_prepared = crate::preparation::prepare_kernel_patch(&patch, &settings)
        .expect("default patch prepares");
    let render = |prepared: &_| {
        render_kernel_offline_named(prepared, Vec::new(), &PreparedSamplerAssets::empty())
            .expect("named render succeeds")
    };
    let preset_render = render(&preset_prepared);
    assert!(
        preset_render[0].1[0]
            .iter()
            .any(|sample| sample.abs() > 0.001)
    );
    assert_eq!(preset_render, render(&default_prepared));
}

#[test]
fn kernel_preset_render_is_deterministic_and_uses_aliased_default() {
    let patch = load_kernel_patch_str(PRESET_PATCH).expect("preset patch loads");
    let preset = load_preset_str(
        "name: quiet\ninstrument: { id: test.instrument, preset_schema_version: 2 }\nvalues: { loudness: 0.4 }\n",
    )
    .expect("preset loads");
    let settings = RenderSettings {
        sample_rate_hz: 48_000,
        block_size_frames: 128,
        duration_frames: 512,
    };

    let prepared = prepare_kernel_patch_with_preset(&patch, &preset, &settings)
        .expect("preset patch prepares");
    let render = || {
        render_kernel_offline_named(&prepared, Vec::new(), &PreparedSamplerAssets::empty())
            .expect("named render succeeds")
    };
    let first = render();
    let second = render();
    assert_eq!(first, second);
    assert!(first[0].1[0].iter().any(|sample| sample.abs() > 0.01));

    let default = crate::preparation::prepare_kernel_patch(&patch, &settings)
        .expect("default patch prepares");
    let default_render =
        render_kernel_offline_named(&default, Vec::new(), &PreparedSamplerAssets::empty())
            .expect("default named render succeeds");
    assert_ne!(first[0].1[0], default_render[0].1[0]);
}

#[test]
fn complete_kernel_document_produces_root_and_inline_graph_definitions() {
    let patch = load_kernel_patch_str(COMPLETE_PATCH).expect("kernel patch should load");

    assert_eq!(patch.metadata().name(), Some("reusable_voice"));
    assert_eq!(patch.metadata().version(), Some("1.2"));
    assert_eq!(patch.metadata().author(), Some("Test Author"));

    let root = patch.root();
    assert_eq!(root.name(), "reusable_voice");
    assert_eq!(root.static_params().len(), 4);
    assert_eq!(root.static_params()[0].static_type(), StaticType::Int);
    assert_eq!(
        root.static_params()[0].default(),
        Some(&StaticValue::Int(2))
    );
    assert_eq!(
        root.static_params()[1].allowed_values(),
        ["clean", "driven"]
    );
    assert_eq!(
        root.static_params()[2].default(),
        Some(&StaticValue::String("fn process(ctx) {}".into()))
    );
    assert_eq!(
        root.static_params()[3].static_type(),
        StaticType::Resource(ResourceKind::ImpulseResponse)
    );
    assert_eq!(
        root.static_params()[3].default(),
        Some(&StaticValue::Resource(ResourceRef::new(
            ResourceKind::ImpulseResponse,
            "room_ir.wav",
            ResourceOrigin::Document,
        )))
    );

    let level = &root.ports()[0];
    assert_eq!(level.direction(), PortDirection::Input);
    assert_eq!(level.signal_type(), SignalType::Control);
    assert_eq!(level.channels(), &ChannelCount::Literal(1));
    let default = level.control_default().expect("control default");
    assert_eq!(default.default(), 0.75);
    assert_eq!(default.min(), Some(0.0));
    assert_eq!(default.max(), Some(1.0));
    assert_eq!(default.unit(), Some("linear"));
    assert_eq!(level.internal_targets()[0].node().as_str(), "amp");
    assert_eq!(level.internal_targets()[0].port(), "gain");

    let master = &root.ports()[1];
    assert_eq!(master.channels(), &ChannelCount::Param("channels".into()));
    assert_eq!(master.internal_sources()[0].node().as_str(), "amp");
    assert_eq!(
        root.nodes()[0].static_args()["channels"],
        StaticArg::ParamRef("channels".into())
    );
    assert_eq!(
        root.nodes()[0].static_args()["mode"],
        StaticArg::Literal(StaticValue::Enum("driven".into()))
    );
    assert_eq!(
        root.nodes()[0].static_args()["label"],
        StaticArg::Literal(StaticValue::String("main".into()))
    );
    assert_eq!(
        root.nodes()[0].static_args()["impulse"],
        StaticArg::Literal(StaticValue::Resource(ResourceRef::new(
            ResourceKind::ImpulseResponse,
            "room_ir.wav",
            ResourceOrigin::Document,
        )))
    );
    assert_eq!(root.nodes()[0].port_default_overrides()["gain"], 0.5);
    assert_eq!(root.connections().len(), 1);

    let defined_module = patch
        .registry()
        .get("amplifier")
        .expect("inline defined module");
    assert_eq!(defined_module.static_params().len(), 4);
    assert_eq!(defined_module.ports().len(), 2);
    assert_eq!(defined_module.nodes().len(), 1);
}

#[test]
fn resource_static_parameter_requires_a_resource_kind() {
    let error = load_kernel_patch_str(
        "metadata: { name: missing-kind }\nstatic_params:\n  - { name: sample, type: resource }\nports:\n  - { name: out, direction: output, signal: audio, channels: 1 }\nmodules: []\nconnections: []\n",
    )
    .expect_err("resource declarations without a kind must fail");

    assert_eq!(
        error.errors().next().unwrap().error_code(),
        error_codes::KERNEL_DOCUMENT_PARSE_FAILED
    );
}

#[test]
fn graph_declaration_has_patch_and_defined_module_symmetry() {
    let patch = load_kernel_patch_str(COMPLETE_PATCH).expect("kernel patch should load");
    let defined_module = patch
        .registry()
        .get("amplifier")
        .expect("inline defined module");

    assert_eq!(
        patch.root().static_params()[0].name(),
        defined_module.static_params()[0].name()
    );
    assert_eq!(
        patch.root().static_params()[0].static_type(),
        defined_module.static_params()[0].static_type()
    );
    assert!(patch.root().ports()[0].control_default().is_some());
    assert!(defined_module.ports()[0].control_default().is_some());
    assert_eq!(defined_module.nodes()[0].definition_ref(), "gain");
}

#[test]
fn parsed_root_defined_module_script_and_primitive_use_the_same_discovery_schema() {
    let patch = load_kernel_patch_str(COMPLETE_PATCH).expect("kernel patch loads");
    let root = patch.root().metadata();
    let defined_module = patch.registry().discover("amplifier").unwrap();
    let primitive = patch.registry().discover("gain").unwrap();
    let scripted = load_kernel_patch_str(SCRIPT_DEFINITION_PATCH).expect("script patch loads");
    let script = scripted.registry().discover("counter").unwrap();

    assert_eq!(root.name(), "reusable_voice");
    assert_eq!(root.ports()[1].channels(), &ChannelCount::param("channels"));
    assert_eq!(
        root.static_params()[3].static_type(),
        StaticType::Resource(ResourceKind::ImpulseResponse)
    );
    assert_eq!(
        defined_module.ports()[0].control_default().unwrap().unit(),
        Some("linear")
    );
    assert_eq!(
        defined_module.static_params()[1].allowed_values(),
        ["clean", "driven"]
    );
    assert_eq!(primitive.ports()[0].name(), "audio_in");
    assert_eq!(script.ports()[0].signal_type(), SignalType::Event);
    assert_eq!(script.static_params()[1].static_type(), StaticType::String);
}

#[test]
fn standalone_defined_module_shape_loads_as_a_root_patch() {
    let yaml = r#"
metadata: { name: amplifier }
static_params:
  - { name: channels, type: int, default: 1 }
ports:
  - { name: gain, direction: input, signal: control, channels: 1, default: 1, maps_to: inner.gain }
  - { name: audio_out, direction: output, signal: audio, channels: $channels, maps_from: inner.audio_out }
modules:
  - { id: inner, type: gain }
connections: []
"#;
    let patch = load_kernel_patch_str(yaml).expect("defined-module-shaped root should load");

    assert_eq!(patch.root().name(), "amplifier");
    assert_eq!(patch.root().static_params().len(), 1);
    assert_eq!(patch.root().ports().len(), 2);
    assert_eq!(patch.root().nodes().len(), 1);
}

#[test]
fn yaml_and_yml_files_load_through_public_file_surface() {
    let directory = std::env::temp_dir();
    for extension in ["yaml", "yml"] {
        let path = directory.join(format!("dandrum-kernel-document.{extension}"));
        fs::write(&path, COMPLETE_PATCH).expect("write test patch");
        let loaded = load_kernel_patch_file(&path).expect("load test patch");
        fs::remove_file(path).expect("remove test patch");
        assert_eq!(loaded.root().name(), "reusable_voice");
    }
}

#[test]
fn unsupported_file_format_has_structured_diagnostic() {
    let diagnostics = load_kernel_patch_file("patch.json").expect_err("JSON should be rejected");
    assert_eq!(
        diagnostics.all()[0].error_code(),
        error_codes::KERNEL_DOCUMENT_UNSUPPORTED_FORMAT
    );
    assert!(diagnostics.all()[0].message().contains(".json"));
}

#[test]
fn malformed_yaml_has_structured_diagnostic() {
    let diagnostics = load_kernel_patch_str("ports: [").expect_err("malformed YAML should fail");
    assert_eq!(
        diagnostics.all()[0].error_code(),
        error_codes::KERNEL_DOCUMENT_PARSE_FAILED
    );
}

#[test]
fn legacy_document_fields_are_rejected_with_specific_diagnostics() {
    for (field, code) in [
        ("render", error_codes::KERNEL_DOCUMENT_LEGACY_RENDER),
        (
            "voice_allocation",
            error_codes::KERNEL_DOCUMENT_LEGACY_VOICE_ALLOCATION,
        ),
    ] {
        let yaml = format!(
            "metadata: {{ name: test }}\nports: []\nmodules: []\nconnections: []\n{field}: {{}}\n"
        );
        let diagnostics = load_kernel_patch_str(&yaml).expect_err("legacy field should fail");
        assert_eq!(diagnostics.all()[0].error_code(), code);
        assert!(diagnostics.all()[0].message().contains(field));
        if field == "render" {
            assert!(diagnostics.all()[0].message().contains("host"));
        }
    }
}

#[test]
fn legacy_instance_parameters_are_rejected_with_module_context() {
    let yaml = "metadata: { name: test }\nports: []\nmodules:\n  - id: amp\n    type: gain\n    parameters: { gain: 0.5 }\nconnections: []\n";
    let diagnostics = load_kernel_patch_str(yaml).expect_err("parameters should fail");
    let diagnostic = &diagnostics.all()[0];
    assert_eq!(
        diagnostic.error_code(),
        error_codes::KERNEL_DOCUMENT_LEGACY_PARAMETERS
    );
    assert_eq!(diagnostic.module_id(), Some("amp"));
}

#[test]
fn external_module_reference_rejects_unsupported_composite_id_field() {
    let yaml = "ports:\n  - { name: audio, direction: output, signal: audio, channels: 1, maps_from: voice.audio }\nmodules:\n  - id: voice\n    type: $LIB/1.0.0/voice/voice.yaml\n    composite_id: voice\nconnections: []\n";
    let diagnostics = load_kernel_patch_str(yaml)
        .expect_err("composite_id must not become an alternative module reference field");
    assert_eq!(
        diagnostics.all()[0].error_code(),
        error_codes::KERNEL_DOCUMENT_SCHEMA_FAILED
    );
    assert!(diagnostics.all()[0].message().contains("composite_id"));
}

#[test]
fn legacy_parameter_binding_is_rejected_in_static_arguments() {
    let yaml = "metadata: { name: test }\nports: []\nmodule_definitions:\n  - type: child\n    static_params:\n      - { name: channels, type: int }\n    ports: []\n    modules: []\n    connections: []\nmodules:\n  - id: child\n    type: child\n    static: { channels: '${channels}' }\nconnections: []\n";
    let diagnostics = load_kernel_patch_str(yaml).expect_err("legacy binding should fail");
    assert_eq!(
        diagnostics.all()[0].error_code(),
        error_codes::KERNEL_DOCUMENT_LEGACY_BINDING
    );
    assert_eq!(diagnostics.all()[0].module_id(), Some("child"));
}

#[test]
fn legacy_parameter_binding_is_rejected_in_default_overrides() {
    let yaml = "metadata: { name: test }\nports:\n  - { name: out, direction: output, signal: audio, channels: 1 }\nmodules:\n  - id: amp\n    type: gain\n    defaults: { gain: '${level}' }\nconnections: []\n";
    let diagnostics = load_kernel_patch_str(yaml).expect_err("legacy binding should fail");
    assert_eq!(
        diagnostics.all()[0].error_code(),
        error_codes::KERNEL_DOCUMENT_LEGACY_BINDING
    );
    assert_eq!(diagnostics.all()[0].module_id(), Some("amp"));
}

#[test]
fn legacy_composite_asset_bindings_are_rejected_with_definition_context() {
    let yaml = "metadata: { name: test }\nports: []\nmodule_definitions:\n  - type: child\n    asset_bindings: []\n    ports: []\n    modules: []\n    connections: []\nmodules: []\nconnections: []\n";
    let diagnostics = load_kernel_patch_str(yaml).expect_err("asset bindings should fail");
    assert_eq!(
        diagnostics.all()[0].error_code(),
        error_codes::KERNEL_DOCUMENT_LEGACY_ASSET_BINDINGS
    );
    assert_eq!(diagnostics.all()[0].module_id(), Some("child"));
}

#[test]
fn legacy_composite_parameters_are_rejected_with_definition_context() {
    let yaml = "ports: []\nmodule_definitions:\n  - type: child\n    parameters: []\n    ports: []\n    modules: []\n    connections: []\nmodules: []\nconnections: []\n";
    let diagnostics = load_kernel_patch_str(yaml).expect_err("legacy parameters fail");
    assert_eq!(
        diagnostics.all()[0].error_code(),
        error_codes::KERNEL_DOCUMENT_LEGACY_PARAMETERS
    );
    assert_eq!(diagnostics.all()[0].module_id(), Some("child"));
}

#[test]
fn patch_without_root_output_has_structured_diagnostic() {
    let yaml = "metadata: { name: silent }\nports:\n  - { name: level, direction: input, signal: control, channels: 1, default: 0 }\nmodules: []\nconnections: []\n";
    let diagnostics = load_kernel_patch_str(yaml).expect_err("root output is required");
    assert_eq!(
        diagnostics.all()[0].error_code(),
        error_codes::KERNEL_DOCUMENT_NO_OUTPUT
    );
    assert!(
        diagnostics.all()[0]
            .message()
            .contains("no observable output")
    );
}

#[test]
fn unknown_static_and_default_names_are_reported_by_kernel_validation() {
    let yaml = r#"
metadata: { name: invalid_overrides }
ports:
  - { name: out, direction: output, signal: audio, channels: 1, maps_from: amp.audio_out }
module_definitions:
  - type: amplifier
    static_params:
      - { name: channels, type: int, default: 1 }
    ports:
      - { name: audio_out, direction: output, signal: audio, channels: $channels, maps_from: inner.audio_out }
    modules:
      - { id: inner, type: gain }
    connections: []
modules:
  - id: amp
    type: amplifier
    static: { unknown_shape: 2 }
  - id: amp_default
    type: amplifier
    defaults: { unknown_port: 0.5 }
connections: []
"#;
    let patch = load_kernel_patch_str(yaml).expect("document shape should parse");
    let validation = patch.root().validate(patch.registry());
    let codes = validation
        .diagnostics()
        .all()
        .iter()
        .map(|diagnostic| diagnostic.error_code())
        .collect::<Vec<_>>();

    assert!(codes.contains(&error_codes::KERNEL_UNKNOWN_STATIC_ARGUMENT));
    assert!(codes.contains(&error_codes::KERNEL_OVERRIDE_UNKNOWN_PORT));
}

// --- 3.3 Input multiplicity YAML parsing ----------------------------------

#[test]
fn summing_multiplicity_parses_from_yaml() {
    let yaml = r#"
metadata:
  name: summing_test
ports:
  - { name: master, direction: output, signal: audio, channels: 1 }
module_definitions:
  - type: mixer
    ports:
      - { name: inputs, direction: input, signal: audio, channels: 1, multiplicity: summing }
      - { name: mix, direction: output, signal: audio, channels: 1 }
    modules: []
    connections: []
modules:
  - id: m
    type: mixer
connections: []
"#;
    let patch = load_kernel_patch_str(yaml).expect("document should parse");
    let mixer = patch
        .registry()
        .get("mixer")
        .expect("mixer definition registered");
    let inputs = mixer
        .ports()
        .iter()
        .find(|p| p.name() == "inputs")
        .expect("inputs port exists");
    assert_eq!(
        inputs.multiplicity(),
        crate::kernel::Multiplicity::Summing,
        "YAML multiplicity: summing should parse"
    );
}

#[test]
fn omitted_multiplicity_defaults_to_single_source() {
    let yaml = r#"
metadata:
  name: single_test
ports:
  - { name: master, direction: output, signal: audio, channels: 1 }
module_definitions:
  - type: gain_like
    ports:
      - { name: audio_in, direction: input, signal: audio, channels: 1 }
      - { name: audio_out, direction: output, signal: audio, channels: 1 }
    modules: []
    connections: []
modules:
  - id: g
    type: gain_like
connections: []
"#;
    let patch = load_kernel_patch_str(yaml).expect("document should parse");
    let gain_like = patch
        .registry()
        .get("gain_like")
        .expect("gain_like definition registered");
    let audio_in = gain_like
        .ports()
        .iter()
        .find(|p| p.name() == "audio_in")
        .expect("audio_in port exists");
    assert_eq!(
        audio_in.multiplicity(),
        crate::kernel::Multiplicity::SingleSource,
        "omitted multiplicity should default to single_source"
    );
}

#[test]
fn single_source_multiplicity_parses_explicitly() {
    let yaml = r#"
metadata:
  name: explicit_single
ports:
  - { name: master, direction: output, signal: audio, channels: 1 }
module_definitions:
  - type: single_input
    ports:
      - { name: audio_in, direction: input, signal: audio, channels: 1, multiplicity: single_source }
      - { name: audio_out, direction: output, signal: audio, channels: 1 }
    modules: []
    connections: []
modules:
  - id: s
    type: single_input
connections: []
"#;
    let patch = load_kernel_patch_str(yaml).expect("document should parse");
    let single_input = patch
        .registry()
        .get("single_input")
        .expect("single_input definition registered");
    let audio_in = single_input
        .ports()
        .iter()
        .find(|p| p.name() == "audio_in")
        .expect("audio_in port exists");
    assert_eq!(
        audio_in.multiplicity(),
        crate::kernel::Multiplicity::SingleSource,
        "explicit single_source should parse"
    );
}

const SCRIPT_DEFINITION_PATCH: &str = r#"
metadata: { name: scripted }
ports:
  - { name: out, direction: output, signal: audio, channels: 1 }
module_definitions:
  - type: counter
    implementation: script
    static_params:
      - { name: language, type: enum, default: rhai, allowed_values: [rhai] }
      - { name: source, type: string }
    ports:
      - { name: events, direction: input, signal: event, channels: 1 }
      - { name: increment, direction: input, signal: control, channels: 1, default: 1 }
      - { name: count, direction: output, signal: control, channels: 1 }
modules:
  - id: first
    type: counter
    static: { source: "fn process(ctx) {}" }
    defaults: { increment: 2 }
  - id: second
    type: counter
    static: { source: "fn process(ctx) {}" }
connections: []
"#;

#[test]
fn script_backed_definition_uses_the_ordinary_node_shape() {
    let patch = load_kernel_patch_str(SCRIPT_DEFINITION_PATCH).expect("script definition loads");
    let definition = patch.registry().get("counter").expect("named definition");

    assert_eq!(
        definition.implementation(),
        DefinitionImplementation::Script
    );
    assert_eq!(
        definition.static_params()[0].static_type(),
        StaticType::Enum
    );
    assert_eq!(
        definition.static_params()[1].static_type(),
        StaticType::String
    );
    assert_eq!(definition.ports().len(), 3);
    assert_eq!(patch.root().nodes()[0].definition_ref(), "counter");
    assert_eq!(
        patch.root().nodes()[0].port_default_overrides()["increment"],
        2.0
    );
}

#[test]
fn script_instances_reject_ad_hoc_port_fields() {
    let yaml = SCRIPT_DEFINITION_PATCH.replace(
        "    defaults: { increment: 2 }",
        "    defaults: { increment: 2 }\n    inputs: [{ name: invented, signal: control }]",
    );

    let diagnostics = load_kernel_patch_str(&yaml).expect_err("instance ports are not authorable");

    assert_eq!(
        diagnostics.errors().next().unwrap().error_code(),
        error_codes::KERNEL_DOCUMENT_SCHEMA_FAILED
    );
    assert!(diagnostics.all()[0].message().contains("inputs"));
}

#[test]
fn script_definition_rejects_internal_graph_structure_and_audio_ports() {
    for (replacement, expected_fragment) in [
        (
            "    implementation: script\n    modules: [{ id: inner, type: gain }]",
            "internal modules",
        ),
        (
            "    implementation: script\n    connections: [{ from: a.out, to: b.in }]",
            "internal connections",
        ),
        (
            "      - { name: count, direction: output, signal: audio, channels: 1 }",
            "audio port",
        ),
    ] {
        let yaml = if replacement.starts_with("    implementation") {
            SCRIPT_DEFINITION_PATCH.replace("    implementation: script", replacement)
        } else {
            SCRIPT_DEFINITION_PATCH.replace(
                "      - { name: count, direction: output, signal: control, channels: 1 }",
                replacement,
            )
        };

        let diagnostics =
            load_kernel_patch_str(&yaml).expect_err("malformed script definition must fail");
        let diagnostic = diagnostics.errors().next().unwrap();
        assert_eq!(
            diagnostic.error_code(),
            error_codes::KERNEL_SCRIPT_DEFINITION_INVALID
        );
        assert!(diagnostic.message().contains(expected_fragment));
        assert_eq!(diagnostic.module_id(), Some("counter"));
    }
}

#[test]
fn script_definition_requires_typed_language_and_source_declarations() {
    for (yaml, expected_fragment) in [
        (
            SCRIPT_DEFINITION_PATCH.replace(
                "      - { name: language, type: enum, default: rhai, allowed_values: [rhai] }\n",
                "",
            ),
            "language",
        ),
        (
            SCRIPT_DEFINITION_PATCH.replace("      - { name: source, type: string }\n", ""),
            "source",
        ),
        (
            SCRIPT_DEFINITION_PATCH
                .replace("name: language, type: enum", "name: language, type: string"),
            "language",
        ),
        (
            SCRIPT_DEFINITION_PATCH
                .replace("name: source, type: string", "name: source, type: int"),
            "source",
        ),
    ] {
        let diagnostics =
            load_kernel_patch_str(&yaml).expect_err("script construction declarations are fixed");
        let diagnostic = diagnostics.errors().next().unwrap();
        assert_eq!(
            diagnostic.error_code(),
            error_codes::KERNEL_SCRIPT_DEFINITION_INVALID
        );
        assert!(diagnostic.message().contains(expected_fragment));
        assert_eq!(diagnostic.module_id(), Some("counter"));
    }
}

#[test]
fn unsupported_definition_implementation_has_a_structured_diagnostic() {
    let yaml = SCRIPT_DEFINITION_PATCH.replace("implementation: script", "implementation: wasm");

    let diagnostics = load_kernel_patch_str(&yaml).expect_err("unsupported implementation fails");
    let diagnostic = diagnostics.errors().next().unwrap();

    assert_eq!(
        diagnostic.error_code(),
        error_codes::KERNEL_DEFINITION_IMPLEMENTATION_UNSUPPORTED
    );
    assert_eq!(diagnostic.module_id(), Some("counter"));
    assert_eq!(diagnostic.actual(), Some("wasm"));
}

const POLY_PATCH: &str = r#"
metadata: { name: poly_patch }
ports:
  - { name: master, direction: output, signal: audio, channels: 2, maps_from: voices.audio }
module_definitions:
  - type: voice
    ports:
      - { name: level, direction: input, signal: control, channels: 1, default: 0.5 }
      - { name: audio, direction: output, signal: audio, channels: 2 }
    modules: []
    connections: []
modules:
  - { id: midi, type: midi_input }
  - id: voices
    type: poly
    static: { definition: voice, max_voices: 8, allocation: oldest-steal }
connections:
  - { from: midi.events, to: voices.notes }
"#;

#[test]
fn yaml_poly_node_uses_ordinary_node_shape_and_synthesized_interface() {
    let patch = load_kernel_patch_str(POLY_PATCH).expect("poly YAML loads");
    let poly = &patch.root().nodes()[1];

    assert_eq!(poly.definition_ref(), POLY_DEFINITION);
    assert_eq!(
        poly.static_args()["allocation"],
        StaticArg::Literal(StaticValue::Enum(POLY_ALLOCATION_OLDEST_STEAL.to_string()))
    );
    assert!(
        patch.root().validate(patch.registry()).is_ok(),
        "YAML connections validate through the synthesized interface"
    );
    let ports = patch
        .root()
        .resolved_node_ports(patch.registry(), poly.id())
        .expect("poly ports resolve");
    assert!(
        ports
            .iter()
            .any(|port| port.name() == POLY_NOTE_EVENTS_INPUT)
    );
    assert!(ports.iter().any(|port| port.name() == "level"));
    assert!(ports.iter().any(|port| port.name() == "audio"));
}

#[test]
fn yaml_poly_rejects_invalid_allocation_policy_during_validation() {
    let yaml = POLY_PATCH.replace("oldest-steal", "newest-steal");
    let patch = load_kernel_patch_str(&yaml).expect("document shape still loads");
    let validation = patch.root().validate(patch.registry());

    assert_eq!(
        validation
            .diagnostics()
            .errors()
            .next()
            .unwrap()
            .error_code(),
        error_codes::KERNEL_STATIC_ARGUMENT_INVALID_ENUM_VALUE
    );
}

#[test]
fn kernel_loader_checks_external_schema_before_constructing_a_graph() {
    let yaml = r#"
metadata: { name: schema_check }
instrument: { id: "", preset_schema_version: 0 }
ports:
  - { name: out, direction: output, signal: audio, channels: 1, maps_from: osc.audio }
modules:
  - { id: osc, type: oscillator }
"#;

    let error =
        load_kernel_patch_str(yaml).expect_err("empty preset identity fails schema validation");
    assert_eq!(
        error.errors().next().unwrap().error_code(),
        error_codes::KERNEL_DOCUMENT_SCHEMA_FAILED,
    );
    assert!(error.to_string().contains("instrument"));
}

#[test]
fn external_kernel_schema_and_serde_agree_on_document_shape_fixtures() {
    let fixtures = [
        ("complete", COMPLETE_PATCH, true),
        ("preset", PRESET_PATCH, true),
        ("script", SCRIPT_DEFINITION_PATCH, true),
        ("poly", POLY_PATCH, true),
        (
            "summing",
            "ports:\n  - { name: mix, direction: input, signal: audio, channels: 2, multiplicity: summing }\n",
            true,
        ),
        (
            "unknown-root",
            "ports: []\nmodules: []\nunknown: true\n",
            false,
        ),
        (
            "legacy-render",
            "render: { sample_rate_hz: 48000 }\n",
            false,
        ),
        (
            "legacy-voice-allocation",
            "voice_allocation: { max_voices: 8 }\n",
            false,
        ),
        (
            "unknown-node",
            "modules:\n  - { id: osc, type: oscillator, inputs: [] }\n",
            false,
        ),
        (
            "missing-port-channels",
            "ports:\n  - { name: out, direction: output, signal: audio }\n",
            false,
        ),
        (
            "bad-multiplicity",
            "ports:\n  - { name: out, direction: output, signal: audio, channels: 1, multiplicity: many }\n",
            false,
        ),
        (
            "bad-static-type",
            "static_params:\n  - { name: channels, type: float }\n",
            false,
        ),
        (
            "bad-resource-kind",
            "static_params:\n  - { name: sample, type: resource, resource_kind: script }\n",
            false,
        ),
    ];

    for (name, yaml, expected) in fixtures {
        let value: serde_yaml::Value = serde_yaml::from_str(yaml).expect("fixture YAML parses");
        assert_eq!(
            super::validate_kernel_schema(&value).is_ok(),
            expected,
            "schema: {name}",
        );
        assert_eq!(
            serde_yaml::from_value::<super::PatchDocument>(value).is_ok(),
            expected,
            "Serde: {name}",
        );
    }
}

#[test]
fn external_schema_checks_poly_static_argument_types() {
    let yaml = POLY_PATCH.replace("max_voices: 8", "max_voices: many");
    let error = load_kernel_patch_str(&yaml).expect_err("poly shape should fail schema validation");
    assert_eq!(
        error.errors().next().unwrap().error_code(),
        error_codes::KERNEL_DOCUMENT_SCHEMA_FAILED,
    );
    assert!(error.to_string().contains("/modules/1/static/max_voices"));
}
