use dandrum_engine::graph::Graph;
use dandrum_engine::patch::{load_patch_str, resolve_module_parameters, validate_patch_schema, ParameterValue};

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

    assert_any_message_contains(&messages, "kick.tune_hz");
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
fn preset_values_apply_after_module_values_and_before_resolution() {
    let patch = load_patch_str(
        r#"
metadata:
  name: Preset Resolution
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

    assert_eq!(filt.get("algorithm"), Some(&ParameterValue::Text("biquad".into())));
    assert_eq!(filt.get("mode"), Some(&ParameterValue::Text("highpass".into())));
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
fn capability_metadata_lists_builtin_parameter_schema_without_building_graph() {
    let patch = load_patch_str(
        r#"
metadata:
  name: Capability Placeholder
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: filt
    type: filter
"#,
    )
    .expect("patch should parse");

    let capability = dandrum_engine::patch::describe_module_parameters(&patch, "filter")
        .expect("filter capability metadata should be available");

    assert!(capability.iter().any(|param| param.name == "algorithm"));
    assert!(capability.iter().any(|param| param.name == "mode"));
    assert!(capability.iter().any(|param| param.default.as_deref() == Some("moog")));
}

#[test]
fn capability_metadata_lists_composite_public_controls_only() {
    let patch = load_patch_str(
        r#"
metadata:
  name: Composite Capability
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
module_definitions:
  - type: filter_voice
    parameters:
      - name: tone
        type: string
        default: moog
        enum: [moog, biquad]
        maps_to:
          - filt.algorithm
    modules:
      - id: filt
        type: filter
modules:
  - id: voice
    type: filter_voice
"#,
    )
    .expect("patch should parse");

    let capability = dandrum_engine::patch::describe_module_parameters(&patch, "filter_voice")
        .expect("composite capability metadata should be available");

    assert!(capability.iter().any(|param| param.name == "tone"));
    assert!(!capability.iter().any(|param| param.name == "algorithm"));
}

#[test]
fn cli_override_parser_accepts_module_parameter_values_without_mutating_yaml() {
    let patch = load_patch_str(
        r#"
metadata:
  name: CLI Override
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: filt
    type: filter
"#,
    )
    .expect("patch should parse");

    let overrides = dandrum_engine::cli::parse_parameter_overrides([
        "--set",
        "filt.algorithm=biquad",
        "--set",
        "filt.mode=highpass",
    ])
    .expect("CLI overrides should parse");

    let resolved = dandrum_engine::patch::resolve_module_parameters_with_overrides(&patch, &overrides)
        .expect("CLI overrides should resolve");
    let filt = resolved.get("filt").expect("filter parameters should resolve");

    assert_eq!(filt.get("algorithm"), Some(&ParameterValue::Text("biquad".into())));
    assert_eq!(filt.get("mode"), Some(&ParameterValue::Text("highpass".into())));
    assert!(patch.parameters.is_empty(), "source YAML model must not be mutated");
}

#[test]
fn cli_override_parser_uses_last_value_for_duplicate_overrides() {
    let overrides = dandrum_engine::cli::parse_parameter_overrides([
        "--set",
        "filt.algorithm=moog",
        "--set",
        "filt.algorithm=biquad",
    ])
    .expect("CLI overrides should parse");

    assert_eq!(overrides.get("filt").and_then(|p| p.get("algorithm")), Some(&ParameterValue::Text("biquad".into())));
}

#[test]
fn cli_override_parser_rejects_malformed_override_targets() {
    let error = dandrum_engine::cli::parse_parameter_overrides(["--set", "filt=biquad"])
        .expect_err("missing parameter name should be rejected");

    assert!(error.to_string().contains("module_id.parameter"));
}

#[test]
fn synthetic_808_kick_composite_exposes_expected_public_controls() {
    let patch = load_patch_str(include_str!("../../examples/synthetic_808_kick.yaml"))
        .expect("synthetic 808 kick example should parse");

    validate_patch_schema(&patch).expect("synthetic 808 kick should validate");
    let capability = dandrum_engine::patch::describe_module_parameters(&patch, "synthetic_808_kick")
        .expect("kick public controls should be discoverable");

    for expected in ["tune_hz", "decay_ms", "punch", "click"] {
        assert!(
            capability.iter().any(|param| param.name == expected),
            "missing kick public control {expected}"
        );
    }
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

    assert_any_message_contains(&messages, "kick.tune_hz");
    assert_any_message_contains(&messages, "20");
}
