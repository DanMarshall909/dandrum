use dandrum_engine::diagnostics::{Diagnostic, error_codes};
use dandrum_engine::patch::{
    ParameterValue, apply_preset, load_patch_str, load_preset_str, validate_patch_schema,
    validate_preset,
};

fn diagnostics_for_invalid_patch(yaml: &str) -> Vec<Diagnostic> {
    let patch = load_patch_str(yaml).expect("patch YAML should parse before validation");
    validate_patch_schema(&patch)
        .expect_err("patch should fail behavioural validation")
        .to_diagnostics()
        .all()
        .to_vec()
}

fn assert_diagnostic_contains(diagnostics: &[Diagnostic], code: &str, text: &str) {
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.error_code() == code && diagnostic.message().contains(text)),
        "expected diagnostic {code} containing {text:?}; actual diagnostics: {diagnostics:#?}"
    );
}

fn composite_patch_with_instance(parameter_value: &str) -> String {
    format!(
        r#"
metadata:
  name: Composite Instance
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
module_definitions:
  - type: test.stereo_gain
    inputs:
      - name: in
        signal_type: audio
        maps_to:
          - body.audio_in
    outputs:
      - name: out
        signal_type: audio
        maps_from:
          - body.audio_out
    parameters:
      - name: amount
        type: number
        default: 0.5
        min: 0
        max: 1
        maps_to:
          - body.mix
    modules:
      - id: body
        type: test.body
        inputs:
          - name: audio_in
            signal_type: audio
          - name: mix
            signal_type: control
        outputs:
          - name: audio_out
            signal_type: audio
modules:
  - id: macro
    type: test.stereo_gain
    parameters:
      amount: {parameter_value}
"#
    )
}

#[test]
fn composite_definition_accepts_public_ports_and_instance_bindings() {
    let patch = load_patch_str(&composite_patch_with_instance("0.75"))
        .expect("patch YAML should parse");

    validate_patch_schema(&patch).expect("composite public surface should validate");
}

#[test]
fn composite_public_port_contract_rejects_wrong_direction_and_signal_mismatches() {
    let diagnostics = diagnostics_for_invalid_patch(
        r#"
metadata:
  name: Bad Composite Ports
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
module_definitions:
  - type: test.bad_ports
    inputs:
      - name: in
        signal_type: audio
        maps_to:
          - body.audio_out
      - name: control_in
        signal_type: audio
        maps_to:
          - body.mix
    outputs:
      - name: out
        signal_type: audio
        maps_from:
          - body.audio_in
      - name: meter
        signal_type: audio
        maps_from:
          - body.cv_out
    modules:
      - id: body
        type: test.body
        inputs:
          - name: audio_in
            signal_type: audio
          - name: mix
            signal_type: control
        outputs:
          - name: audio_out
            signal_type: audio
          - name: cv_out
            signal_type: control
modules:
  - id: macro
    type: test.bad_ports
"#,
    );

    assert_diagnostic_contains(
        &diagnostics,
        error_codes::VALIDATION_INVALID_VALUE,
        "input in maps_to body.audio_out must reference an internal input port",
    );
    assert_diagnostic_contains(
        &diagnostics,
        error_codes::VALIDATION_TYPE_MISMATCH,
        "input control_in maps_to body.mix has incompatible signal types",
    );
    assert_diagnostic_contains(
        &diagnostics,
        error_codes::VALIDATION_INVALID_VALUE,
        "output out maps_from body.audio_in must reference an internal output port",
    );
    assert_diagnostic_contains(
        &diagnostics,
        error_codes::VALIDATION_TYPE_MISMATCH,
        "output meter maps_from body.cv_out has incompatible signal types",
    );
}

#[test]
fn composite_parameter_contract_rejects_bad_definitions_and_instance_values() {
    let diagnostics = diagnostics_for_invalid_patch(
        r#"
metadata:
  name: Bad Composite Parameters
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
module_definitions:
  - type: test.bad_parameters
    parameters:
      - name: ""
        type: number
        default: 0.5
      - name: amount
        type: number
        default: loud
      - name: amount
        type: number
        default: 0.25
      - name: literal
        type: number
        value: high
      - name: inverted
        type: number
        default: 0.5
        min: 1
        max: 0
      - name: texty
        type: string
        default: ok
        min: 0
      - name: expr
        type: number
        expression: amount * 2
      - name: clipped
        type: number
        default: 0.5
        min: 0
        max: 1
    modules: []
modules:
  - id: macro
    type: test.bad_parameters
    parameters:
      amount: loud
      clipped: 1.5
      extra: 1
"#,
    );

    assert_diagnostic_contains(
        &diagnostics,
        error_codes::VALIDATION_MISSING_FIELD,
        "parameter name is required",
    );
    assert_diagnostic_contains(
        &diagnostics,
        error_codes::VALIDATION_INVALID_VALUE,
        "duplicate parameter name amount",
    );
    assert_diagnostic_contains(
        &diagnostics,
        error_codes::VALIDATION_TYPE_MISMATCH,
        "parameter amount default has wrong type",
    );
    assert_diagnostic_contains(
        &diagnostics,
        error_codes::VALIDATION_TYPE_MISMATCH,
        "parameter literal literal value has wrong type",
    );
    assert_diagnostic_contains(
        &diagnostics,
        error_codes::VALIDATION_INVALID_VALUE,
        "parameter inverted has invalid range",
    );
    assert_diagnostic_contains(
        &diagnostics,
        error_codes::VALIDATION_INVALID_VALUE,
        "parameter texty has numeric constraints on a string parameter",
    );
    assert_diagnostic_contains(
        &diagnostics,
        error_codes::VALIDATION_INVALID_VALUE,
        "parameter expr uses unsupported expression syntax",
    );
    assert_diagnostic_contains(
        &diagnostics,
        error_codes::VALIDATION_INVALID_VALUE,
        "instance macro sets undeclared parameter extra",
    );
    assert_diagnostic_contains(
        &diagnostics,
        error_codes::VALIDATION_TYPE_MISMATCH,
        "instance macro parameter amount has wrong type",
    );
    assert_diagnostic_contains(
        &diagnostics,
        error_codes::VALIDATION_INVALID_VALUE,
        "parameter clipped value is above maximum 1: 1.5",
    );
}

#[test]
fn composite_asset_bindings_accept_only_existing_sample_assets() {
    let diagnostics = diagnostics_for_invalid_patch(
        r#"
metadata:
  name: Composite Asset Bindings
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 64
assets:
  - id: kick
    kind: sample
    path: kick.wav
  - id: script_asset
    kind: script
    path: map.rhai
module_definitions:
  - type: test.sample_wrapper
    asset_bindings:
      - name: hit
    modules: []
modules:
  - id: numeric_asset
    type: test.sample_wrapper
    parameters:
      hit: 123
  - id: missing_asset
    type: test.sample_wrapper
    parameters:
      hit: missing
  - id: wrong_kind
    type: test.sample_wrapper
    parameters:
      hit: script_asset
  - id: ok
    type: test.sample_wrapper
    parameters:
      hit: kick
"#,
    );

    assert_diagnostic_contains(
        &diagnostics,
        error_codes::VALIDATION_TYPE_MISMATCH,
        "asset binding hit must be a text asset ID",
    );
    assert_diagnostic_contains(
        &diagnostics,
        error_codes::VALIDATION_UNKNOWN_MODULE,
        "asset binding hit references missing asset missing",
    );
    assert_diagnostic_contains(
        &diagnostics,
        error_codes::VALIDATION_TYPE_MISMATCH,
        "asset binding hit references asset script_asset with kind Script; expected sample",
    );
}

#[test]
fn external_preset_application_preserves_graph_structure_and_replaces_surface_values() {
    let patch = load_patch_str(&composite_patch_with_instance("0.1"))
        .expect("patch YAML should parse");
    let mut patch = patch.clone();
    patch.instrument = Some(dandrum_engine::patch::InstrumentIdentity {
        id: "test.instrument".to_string(),
        preset_schema_version: 1,
    });
    patch.preset_surface.parameters.push(
        dandrum_engine::patch::PresetParameterTargetDeclaration {
            name: "amount".to_string(),
            value_type: dandrum_engine::patch::PresetTargetType::Number,
            default: ParameterValue::Number(0.5),
            min: Some(0.0),
            max: Some(1.0),
            maps_to: dandrum_engine::patch::PortReference {
                module_id: "macro".to_string(),
                port_name: "amount".to_string(),
            },
        },
    );
    validate_patch_schema(&patch).expect("patch should be preset-capable");

    let preset = load_preset_str(
        r#"
name: Brighter
instrument:
  id: test.instrument
  preset_schema_version: 1
values:
  amount: 0.8
"#,
    )
    .expect("preset YAML should parse");

    validate_preset(&patch, &preset).expect("preset should match the patch surface");
    let patched = apply_preset(&patch, &preset).expect("preset should apply");

    assert_eq!(patched.module_definitions, patch.module_definitions);
    assert_eq!(patched.modules.len(), patch.modules.len());
    assert_eq!(
        patch.modules[0].parameters.get("amount"),
        Some(&ParameterValue::Number(0.1)),
        "the source patch remains immutable"
    );
    assert_eq!(
        patched.modules[0].parameters.get("amount"),
        Some(&ParameterValue::Number(0.8))
    );
}

#[test]
fn external_preset_validation_rejects_identity_schema_unknown_targets_and_structure() {
    let patch = load_patch_str(&composite_patch_with_instance("0.1"))
        .expect("patch YAML should parse");
    let mut patch = patch.clone();
    patch.instrument = Some(dandrum_engine::patch::InstrumentIdentity {
        id: "test.instrument".to_string(),
        preset_schema_version: 1,
    });

    let preset = load_preset_str(
        r#"
name: Structural Preset
instrument:
  id: other.instrument
  preset_schema_version: 2
values:
  ghost: 0.8
modules: []
"#,
    )
    .expect("preset YAML should parse");

    let diagnostics = validate_preset(&patch, &preset)
        .expect_err("preset should fail behavioural validation")
        .to_diagnostics();
    let diagnostics = diagnostics.all();

    assert_diagnostic_contains(
        diagnostics,
        error_codes::VALIDATION_INVALID_VALUE,
        "preset targets instrument other.instrument",
    );
    assert_diagnostic_contains(
        diagnostics,
        error_codes::VALIDATION_INVALID_VALUE,
        "preset schema version 2 does not match patch preset schema version 1",
    );
    assert_diagnostic_contains(
        diagnostics,
        error_codes::VALIDATION_INVALID_VALUE,
        "unknown preset target ghost",
    );
    assert_diagnostic_contains(
        diagnostics,
        error_codes::VALIDATION_INVALID_VALUE,
        "preset document cannot declare structural field modules",
    );
}
