use dandrum_engine::graph::Graph;
use dandrum_engine::patch::{
    ParameterValue, load_patch_str, resolve_module_parameters, validate_patch_schema,
};
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
    let (left, _right) =
        dandrum_engine::graph_processor::render_offline(&graph, &patch.render, Vec::new());

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

    assert_eq!(public_names, ["tune_hz", "decay_ms", "punch", "click"]);
    assert!(!public_names.contains(&"seed"));
    assert!(!public_names.contains(&"fft_size"));
    assert!(
        kick.parameters
            .iter()
            .all(|parameter| parameter.description.is_some())
    );
}

#[test]
fn synthetic_808_kick_yaml_tuning_changes_output_deterministically() {
    let patch_path = example_patch_path("synthetic-808-kick.yaml");
    let first_patch =
        dandrum_engine::patch::load_patch_file(&patch_path).expect("kick example parses");
    let mut second_patch = first_patch.clone();
    second_patch.modules[0]
        .parameters
        .insert("tune_hz".to_string(), ParameterValue::Number(72.0));

    let first_graph = Graph::from_patch_declarations(&first_patch);
    let second_graph = Graph::from_patch_declarations(&second_patch);
    first_graph.validate().expect("first graph validates");
    second_graph.validate().expect("second graph validates");

    let (first_left, _) = dandrum_engine::graph_processor::render_offline(
        &first_graph,
        &first_patch.render,
        Vec::new(),
    );
    let (first_left_again, _) = dandrum_engine::graph_processor::render_offline(
        &first_graph,
        &first_patch.render,
        Vec::new(),
    );
    let (second_left, _) = dandrum_engine::graph_processor::render_offline(
        &second_graph,
        &second_patch.render,
        Vec::new(),
    );

    assert_eq!(first_left, first_left_again);
    assert_ne!(first_left, second_left);
}

fn example_patch_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("examples")
        .join("patches")
        .join(name)
}
