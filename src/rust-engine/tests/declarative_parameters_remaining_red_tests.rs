use dandrum_engine::core::TimedInputEvent;
use dandrum_engine::graph::Graph;
use dandrum_engine::patch::{
    AssetKind, ParameterValue, PresetTargetType, apply_preset, load_patch_str, load_preset_file,
    load_preset_str, resolve_module_parameters, validate_patch_schema, validate_preset,
    validate_preset_compatibility,
};
use dandrum_engine::script::ScriptEvent;
use std::path::PathBuf;

fn validation_messages(yaml: &str) -> Vec<String> {
    let patch = load_patch_str(yaml).expect("test patch YAML should parse");
    validate_patch_schema(&patch)
        .expect_err("patch should fail validation")
        .to_diagnostics()
        .all()
        .iter()
        .map(|diagnostic| diagnostic.to_string())
        .collect()
}

fn assert_any_message_contains(messages: &[String], expected: &str) {
    assert!(
        messages.iter().any(|message| message.contains(expected)),
        "expected diagnostic containing {expected:?}, got: {messages:#?}"
    );
}

#[test]
fn patch_yaml_preserves_instrument_identity_and_public_preset_surface() {
    let patch = load_patch_str(
        r#"
metadata:
  name: Presettable Patch
instrument:
  id: dandrum.synthetic-kick
  preset_schema_version: 1
preset_surface:
  parameters:
    - name: tone.decay
      type: number
      default: 0.5
      min: 0
      max: 1
      maps_to: voice.seed
  assets:
    - name: body.sample
      kind: sample
      default: kick_body
      maps_to: sampler.asset
assets:
  - id: kick_body
    kind: sample
    path: samples/kick.wav
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: voice
    type: noise
  - id: sampler
    type: sampler
    parameters:
      asset: kick_body
"#,
    )
    .expect("patch should parse");

    let instrument = patch
        .instrument
        .as_ref()
        .expect("instrument identity should be present");
    assert_eq!(instrument.id, "dandrum.synthetic-kick");
    assert_eq!(instrument.preset_schema_version, 1);

    assert_eq!(patch.preset_surface.parameters.len(), 1);
    let parameter = &patch.preset_surface.parameters[0];
    assert_eq!(parameter.name, "tone.decay");
    assert_eq!(parameter.value_type, PresetTargetType::Number);
    assert_eq!(parameter.default, ParameterValue::Number(0.5));
    assert_eq!(parameter.min, Some(0.0));
    assert_eq!(parameter.max, Some(1.0));
    assert_eq!(parameter.maps_to.module_id, "voice");
    assert_eq!(parameter.maps_to.port_name, "seed");

    assert_eq!(patch.preset_surface.assets.len(), 1);
    let asset = &patch.preset_surface.assets[0];
    assert_eq!(asset.name, "body.sample");
    assert_eq!(asset.kind, AssetKind::Sample);
    assert_eq!(asset.default, "kick_body");
    assert_eq!(asset.maps_to.module_id, "sampler");
    assert_eq!(asset.maps_to.port_name, "asset");
}

#[test]
fn patch_preset_surface_rejects_duplicate_targets() {
    let messages = validation_messages(
        r#"
metadata:
  name: Duplicate Preset Targets
instrument:
  id: dandrum.duplicate-targets
  preset_schema_version: 1
preset_surface:
  parameters:
    - name: tone.decay
      type: number
      default: 0.5
      maps_to: voice.seed
  assets:
    - name: tone.decay
      kind: sample
      default: kick_body
      maps_to: sampler.asset
assets:
  - id: kick_body
    kind: sample
    path: samples/kick.wav
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: voice
    type: noise
  - id: sampler
    type: sampler
    parameters:
      asset: kick_body
"#,
    );

    assert_any_message_contains(&messages, "tone.decay");
    assert_any_message_contains(&messages, "duplicate");
}

#[test]
fn patch_preset_surface_rejects_unresolved_target_destinations() {
    let messages = validation_messages(
        r#"
metadata:
  name: Missing Preset Target Destination
instrument:
  id: dandrum.missing-target
  preset_schema_version: 1
preset_surface:
  parameters:
    - name: tone.decay
      type: number
      default: 0.5
      maps_to: missing.decay
  assets:
    - name: body.sample
      kind: sample
      default: missing_asset
      maps_to: sampler.asset
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: sampler
    type: sampler
    parameters:
      asset: missing_asset
"#,
    );

    assert_any_message_contains(&messages, "tone.decay");
    assert_any_message_contains(&messages, "missing.decay");
    assert_any_message_contains(&messages, "body.sample");
    assert_any_message_contains(&messages, "missing_asset");
}

#[test]
fn preset_yaml_document_parses_values_assets_and_metadata() {
    let preset = load_preset_str(
        r#"
name: Tight Kick
instrument:
  id: dandrum.synthetic-kick
  preset_schema_version: 1
values:
  tone.seed: 99
assets:
  body.sample: tight_body
metadata:
  author: Dan
  category: kicks
  tags: [tight, electronic]
  description: Short tight electronic kick.
"#,
    )
    .expect("preset should parse");

    assert_eq!(preset.name, "Tight Kick");
    assert_eq!(preset.instrument.id, "dandrum.synthetic-kick");
    assert_eq!(preset.instrument.preset_schema_version, 1);
    assert_eq!(
        preset.values.get("tone.seed"),
        Some(&ParameterValue::Number(99.0))
    );
    assert_eq!(
        preset.assets.get("body.sample"),
        Some(&"tight_body".to_string())
    );
    assert_eq!(
        preset
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.author.as_ref()),
        Some(&"Dan".to_string())
    );
}

#[test]
fn non_yaml_preset_file_is_rejected_before_reading() {
    let error = load_preset_file(PathBuf::from("kick.preset.json"))
        .expect_err("unsupported preset format should be rejected");

    assert!(error.to_string().contains("unsupported preset format"));
    assert!(error.to_string().contains("kick.preset.json"));
}

#[test]
fn preset_compatibility_requires_matching_instrument_id_and_schema_version() {
    let patch = load_patch_str(
        r#"
metadata:
  name: Presettable Patch
instrument:
  id: dandrum.synthetic-kick
  preset_schema_version: 2
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: tone
    type: noise
"#,
    )
    .expect("patch should parse");
    let wrong_id = load_preset_str(
        r#"
name: Wrong Instrument
instrument:
  id: dandrum.other
  preset_schema_version: 2
"#,
    )
    .expect("preset should parse");
    let wrong_version = load_preset_str(
        r#"
name: Wrong Version
instrument:
  id: dandrum.synthetic-kick
  preset_schema_version: 1
"#,
    )
    .expect("preset should parse");

    let wrong_id_messages = validate_preset_compatibility(&patch, &wrong_id)
        .expect_err("wrong instrument should fail")
        .to_diagnostics()
        .to_string();
    assert!(wrong_id_messages.contains("dandrum.synthetic-kick"));
    assert!(wrong_id_messages.contains("dandrum.other"));

    let wrong_version_messages = validate_preset_compatibility(&patch, &wrong_version)
        .expect_err("wrong schema version should fail")
        .to_diagnostics()
        .to_string();
    assert!(wrong_version_messages.contains("2"));
    assert!(wrong_version_messages.contains("1"));
}

fn presettable_patch_yaml() -> &'static str {
    r#"
metadata:
  name: Presettable Patch
instrument:
  id: dandrum.synthetic-kick
  preset_schema_version: 1
preset_surface:
  parameters:
    - name: tone.seed
      type: integer
      default: 42
      min: 1
      max: 1000
      maps_to: tone.seed
  assets:
    - name: body.sample
      kind: sample
      default: kick_body
      maps_to: sampler.asset
assets:
  - id: kick_body
    kind: sample
    path: samples/kick.wav
  - id: tight_body
    kind: sample
    path: samples/tight.wav
  - id: velocity_script
    kind: script
    path: scripts/velocity.rhai
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: tone
    type: noise
  - id: sampler
    type: sampler
    parameters:
      asset: kick_body
"#
}

#[test]
fn preset_validation_accepts_declared_targets_and_rejects_unknown_or_incompatible_values() {
    let patch = load_patch_str(presettable_patch_yaml()).expect("patch should parse");
    let accepted = load_preset_str(
        r#"
name: Tight Kick
instrument:
  id: dandrum.synthetic-kick
  preset_schema_version: 1
values:
  tone.seed: 99
assets:
  body.sample: tight_body
"#,
    )
    .expect("preset should parse");
    validate_preset(&patch, &accepted).expect("declared targets should validate");

    let unknown = load_preset_str(
        r#"
name: Unknown Target
instrument:
  id: dandrum.synthetic-kick
  preset_schema_version: 1
values:
  tone.hidden: 99
"#,
    )
    .expect("preset should parse");
    let unknown_messages = validate_preset(&patch, &unknown)
        .expect_err("unknown target should fail")
        .to_diagnostics()
        .to_string();
    assert!(unknown_messages.contains("tone.hidden"));
    assert!(unknown_messages.contains("unknown preset target"));

    let incompatible = load_preset_str(
        r#"
name: Bad Value
instrument:
  id: dandrum.synthetic-kick
  preset_schema_version: 1
values:
  tone.seed: loud
assets:
  body.sample: velocity_script
"#,
    )
    .expect("preset should parse");
    let incompatible_messages = validate_preset(&patch, &incompatible)
        .expect_err("incompatible target values should fail")
        .to_diagnostics()
        .to_string();
    assert!(incompatible_messages.contains("tone.seed"));
    assert!(incompatible_messages.contains("integer"));
    assert!(incompatible_messages.contains("body.sample"));
    assert!(incompatible_messages.contains("sample"));
}

#[test]
fn preset_documents_reject_graph_routing_render_event_script_and_scheduling_fields() {
    let patch = load_patch_str(presettable_patch_yaml()).expect("patch should parse");
    let preset = load_preset_str(
        r#"
name: Structural Preset
instrument:
  id: dandrum.synthetic-kick
  preset_schema_version: 1
modules: []
connections: []
render:
  duration_frames: 64
events: []
scripts: []
scheduling:
  lookahead_frames: 128
"#,
    )
    .expect("preset should parse");

    let messages = validate_preset(&patch, &preset)
        .expect_err("structural preset fields should fail")
        .to_diagnostics()
        .to_string();
    for field in [
        "modules",
        "connections",
        "render",
        "events",
        "scripts",
        "scheduling",
    ] {
        assert!(messages.contains(field), "expected {field} in {messages}");
    }
}

#[test]
fn composite_public_parameter_declarations_reject_duplicate_names() {
    let messages = validation_messages(
        r#"
metadata:
  name: Duplicate Composite Parameters
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
module_definitions:
  - type: kick_voice
    parameters:
      - name: tune_hz
        maps_to: []
      - name: tune_hz
        maps_to: []
    modules:
      - id: filt
        type: filter
modules:
  - id: kick
    type: kick_voice
"#,
    );

    assert_any_message_contains(&messages, "tune_hz");
    assert_any_message_contains(&messages, "duplicate");
}

#[test]
fn composite_public_parameter_declarations_validate_constraints_and_defaults() {
    let messages = validation_messages(
        r#"
metadata:
  name: Invalid Composite Parameter Constraint
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
module_definitions:
  - type: kick_voice
    parameters:
      - name: tune_hz
        type: number
        default: 5
        min: 20
        max: 200
        unit: Hz
        description: fundamental tuning
        maps_to:
          - filt.cutoff_hz
    modules:
      - id: filt
        type: filter
modules:
  - id: kick
    type: kick_voice
"#,
    );

    assert_any_message_contains(&messages, "tune_hz");
    assert_any_message_contains(&messages, "default");
}

#[test]
fn composite_instance_parameter_type_is_validated_against_public_declaration() {
    let messages = validation_messages(
        r#"
metadata:
  name: Composite Instance Wrong Type
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
module_definitions:
  - type: kick_voice
    parameters:
      - name: tune_hz
        type: number
        default: 55
        min: 20
        max: 200
        maps_to:
          - filt.cutoff_hz
    modules:
      - id: filt
        type: filter
modules:
  - id: kick
    type: kick_voice
    parameters:
      tune_hz: loud
"#,
    );

    assert_any_message_contains(&messages, "kick");
    assert_any_message_contains(&messages, "tune_hz");
    assert_any_message_contains(&messages, "number");
}

#[test]
fn composite_parameter_direct_reference_resolves_to_internal_module_parameter_before_graph_build() {
    let patch = load_patch_str(
        r#"
metadata:
  name: Composite Binding Resolution
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
module_definitions:
  - type: filter_voice
    parameters:
      - name: algorithm
        maps_to:
          - filt.algorithm
    modules:
      - id: filt
        type: filter
modules:
  - id: voice
    type: filter_voice
    parameters:
      algorithm: biquad
"#,
    )
    .expect("patch should parse");

    validate_patch_schema(&patch).expect("composite parameter binding should validate");
    let graph = Graph::from_patch_declarations(&patch);
    let filt = graph
        .modules()
        .iter()
        .find(|module| module.id().as_str() == "voice::filt")
        .expect("internal filter should be expanded");

    assert_eq!(filt.params().get("algorithm"), Some(&"biquad".to_string()));
}

#[test]
fn composite_parameter_literal_bindings_resolve_to_internal_module_parameters() {
    let patch = load_patch_str(
        r#"
metadata:
  name: Composite Literal Binding Resolution
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
module_definitions:
  - type: filter_voice
    parameters:
      - name: algorithm
        value: biquad
        maps_to:
          - filt.algorithm
      - name: mode
        value: highpass
        maps_to:
          - filt.mode
    modules:
      - id: filt
        type: filter
modules:
  - id: voice
    type: filter_voice
"#,
    )
    .expect("patch should parse");

    validate_patch_schema(&patch).expect("literal parameter bindings should validate");
    let graph = Graph::from_patch_declarations(&patch);
    let filt = graph
        .modules()
        .iter()
        .find(|module| module.id().as_str() == "voice::filt")
        .expect("internal filter should be expanded");

    assert_eq!(filt.params().get("algorithm"), Some(&"biquad".to_string()));
    assert_eq!(filt.params().get("mode"), Some(&"highpass".to_string()));
}

#[test]
fn composite_parameter_binding_rejects_unsupported_expression_syntax() {
    let messages = validation_messages(
        r#"
metadata:
  name: Unsupported Binding Expression
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
module_definitions:
  - type: filter_voice
    parameters:
      - name: tune_hz
        type: number
        default: 55
        expression: ${tune_hz} * 2
        maps_to:
          - filt.cutoff_hz
    modules:
      - id: filt
        type: filter
modules:
  - id: voice
    type: filter_voice
"#,
    );

    assert_any_message_contains(&messages, "expression");
    assert_any_message_contains(&messages, "unsupported");
}

#[test]
fn unselected_presets_do_not_affect_resolved_parameters() {
    let patch = load_patch_str(
        r#"
metadata:
  name: Unselected Preset Resolution
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
presets:
  bright:
    filt:
      algorithm: biquad
      mode: highpass
modules:
  - id: filt
    type: filter
    parameters:
      algorithm: moog
"#,
    )
    .expect("patch should parse");

    validate_patch_schema(&patch).expect("preset should validate");
    let resolved = resolve_module_parameters(&patch).expect("parameters should resolve");
    let filt = resolved.get("filt").expect("filter params should resolve");

    assert_eq!(
        filt.get("algorithm"),
        Some(&ParameterValue::Text("moog".into()))
    );
    assert_eq!(
        filt.get("mode"),
        Some(&ParameterValue::Text("lowpass".into()))
    );
}

#[test]
fn selected_preset_values_apply_after_module_values_and_before_patch_parameters() {
    let patch = load_patch_str(
        r#"
metadata:
  name: Selected Preset Resolution
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
selected_preset: bright
presets:
  bright:
    filt:
      algorithm: biquad
      mode: highpass
parameters:
  filt:
    mode: lowpass
modules:
  - id: filt
    type: filter
    parameters:
      algorithm: moog
"#,
    )
    .expect("patch should parse");

    validate_patch_schema(&patch).expect("selected preset should validate");
    let resolved = resolve_module_parameters(&patch).expect("parameters should resolve");
    let filt = resolved.get("filt").expect("filter params should resolve");

    assert_eq!(
        filt.get("algorithm"),
        Some(&ParameterValue::Text("biquad".into()))
    );
    assert_eq!(
        filt.get("mode"),
        Some(&ParameterValue::Text("lowpass".into()))
    );
}

#[test]
fn unknown_selected_preset_is_rejected() {
    let messages = validation_messages(
        r#"
metadata:
  name: Unknown Selected Preset
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
selected_preset: missing
presets:
  bright:
    filt:
      algorithm: biquad
modules:
  - id: filt
    type: filter
"#,
    );

    assert_any_message_contains(&messages, "selected_preset");
    assert_any_message_contains(&messages, "missing");
}

#[test]
fn preset_values_cannot_target_internal_module_parameters_unless_exposed() {
    let messages = validation_messages(
        r#"
metadata:
  name: Preset Internal Leak
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
module_definitions:
  - type: filter_voice
    parameters:
      - name: exposed_algorithm
        maps_to:
          - filt.algorithm
    modules:
      - id: filt
        type: filter
presets:
  leak:
    voice::filt:
      algorithm: biquad
modules:
  - id: voice
    type: filter_voice
"#,
    );

    assert_any_message_contains(&messages, "voice::filt");
}

#[test]
fn synthetic_808_kick_invalid_tuning_reports_structured_diagnostic_before_rendering() {
    let messages = validation_messages(
        r#"
metadata:
  name: Invalid Kick
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 2048
module_definitions:
  - type: synthetic_808_kick
    parameters:
      - name: tune_hz
        type: number
        default: 55
        min: 20
        max: 120
        unit: Hz
        maps_to:
          - body.frequency_hz
      - name: decay_ms
        type: number
        default: 500
        min: 50
        max: 2000
        unit: ms
        maps_to:
          - amp.decay_ms
    inputs:
      - name: trigger
        signal_type: event
        maps_to:
          - amp.gate
    outputs:
      - name: audio
        signal_type: audio
        maps_from:
          - body.audio
    modules:
      - id: body
        type: oscillator
      - id: amp
        type: adsr
modules:
  - id: kick
    type: synthetic_808_kick
    parameters:
      tune_hz: 5
"#,
    );

    assert_any_message_contains(&messages, "tune_hz");
    assert_any_message_contains(&messages, "20");
}

#[test]
fn synthetic_808_kick_example_declares_public_controls_and_renders() {
    let patch_path = example_patch_path("synthetic-808-kick.yaml");
    let patch = dandrum_engine::patch::load_patch_file(&patch_path).expect("kick example parses");

    validate_patch_schema(&patch).expect("kick example should validate");
    let graph = Graph::from_patch_declarations(&patch);
    graph.validate().expect("kick example graph validates");
    let events = vec![TimedInputEvent::new(
        0,
        ScriptEvent::NoteOn {
            note: 36,
            velocity: 110,
        },
    )];
    let (left, _right) =
        dandrum_engine::graph_processor::render_offline(&graph, &patch.render, events);

    assert!(left.iter().any(|sample| sample.abs() > 0.0));
}

#[test]
fn synthetic_808_kick_public_controls_are_discoverable_without_internal_leakage() {
    let patch_path = example_patch_path("synthetic-808-kick.yaml");
    let patch = dandrum_engine::patch::load_patch_file(&patch_path).expect("kick example parses");
    let kick = patch
        .module_definitions
        .iter()
        .find(|definition| definition.module_type == "synthetic_808_kick")
        .expect("kick composite should be declared");
    let public_names = kick
        .parameters
        .iter()
        .map(|parameter| parameter.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(public_names, ["tune_hz", "decay_ms", "punch", "click", "sub_decay_ms", "sub_level"]);
    assert!(!public_names.contains(&"seed"));
    assert!(!public_names.contains(&"fft_size"));
    assert!(
        kick.parameters
            .iter()
            .all(|parameter| parameter.description.is_some())
    );
}

#[test]
fn synthetic_808_kick_yaml_decay_control_changes_render_deterministically() {
    let patch_path = example_patch_path("synthetic-808-kick.yaml");
    let patch = dandrum_engine::patch::load_patch_file(&patch_path).expect("kick example parses");
    validate_patch_schema(&patch).expect("kick example should validate");
    let first_preset = load_preset_str(
        r#"
name: Kick Long
instrument:
  id: dandrum.synthetic-808-kick
  preset_schema_version: 1
values:
  kick.decay_ms: 1400
"#,
    )
    .expect("first preset should parse");
    let second_preset = load_preset_str(
        r#"
name: Kick Short
instrument:
  id: dandrum.synthetic-808-kick
  preset_schema_version: 1
values:
  kick.decay_ms: 250
"#,
    )
    .expect("second preset should parse");
    validate_preset(&patch, &first_preset).expect("first preset should validate");
    validate_preset(&patch, &second_preset).expect("second preset should validate");
    let first_patch = apply_preset(&patch, &first_preset).expect("first preset should apply");
    let second_patch = apply_preset(&patch, &second_preset).expect("second preset should apply");
    let first_graph = Graph::from_patch_declarations(&first_patch);
    let second_graph = Graph::from_patch_declarations(&second_patch);
    first_graph.validate().expect("first graph validates");
    second_graph.validate().expect("second graph validates");
    let events = vec![
        TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note: 36,
                velocity: 110,
            },
        ),
        TimedInputEvent::new(96_000, ScriptEvent::NoteOff { note: 36 }),
    ];

    let (first_left, _) = dandrum_engine::graph_processor::render_offline(
        &first_graph,
        &first_patch.render,
        events.clone(),
    );
    let (first_left_again, _) = dandrum_engine::graph_processor::render_offline(
        &first_graph,
        &first_patch.render,
        events.clone(),
    );
    let (second_left, _) = dandrum_engine::graph_processor::render_offline(
        &second_graph,
        &second_patch.render,
        events,
    );

    assert_eq!(first_left, first_left_again);
    assert_ne!(first_left, second_left);
}

#[test]
fn example_preset_loads_against_matching_patch_and_applies_public_values() {
    let patch =
        dandrum_engine::patch::load_patch_file(example_patch_path("synthetic-808-kick.yaml"))
            .expect("kick example parses");
    let preset = load_preset_file(example_preset_path("tight-808-kick.yaml"))
        .expect("preset example parses");

    validate_patch_schema(&patch).expect("kick example should validate");
    validate_preset(&patch, &preset).expect("preset should validate against matching patch");
    let patched = apply_preset(&patch, &preset).expect("preset should apply");
    let kick = patched
        .modules
        .iter()
        .find(|module| module.id == "kick")
        .expect("kick module should exist");

    assert_eq!(
        kick.parameters.get("tune_hz"),
        Some(&ParameterValue::Number(52.0))
    );
    assert_eq!(
        kick.parameters.get("decay_ms"),
        Some(&ParameterValue::Number(420.0))
    );
    assert_eq!(
        kick.parameters.get("punch"),
        Some(&ParameterValue::Number(0.9))
    );
    assert_eq!(
        kick.parameters.get("click"),
        Some(&ParameterValue::Number(0.8))
    );
}

fn example_patch_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("examples")
        .join("patches")
        .join(name)
}

fn example_preset_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("examples")
        .join("presets")
        .join(name)
}
