use dandrum_engine::diagnostics::{Diagnostic, error_codes};
use dandrum_engine::patch::{
    ParameterValue, load_patch_str, resolve_module_parameters, validate_patch_schema,
};

fn validation_diagnostics(yaml: &str) -> Vec<Diagnostic> {
    let patch = load_patch_str(yaml).expect("test patch YAML should parse");
    validate_patch_schema(&patch)
        .expect_err("patch should fail declaration-driven parameter validation")
        .to_diagnostics()
        .all()
        .to_vec()
}

fn assert_has_diagnostic_for_parameter(
    diagnostics: &[Diagnostic],
    module_id: &str,
    parameter_name: &str,
) {
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.module_id() == Some(module_id)
                && diagnostic.message().contains(parameter_name)
        }),
        "expected diagnostic for {module_id}.{parameter_name}, got: {diagnostics:#?}"
    );
}

#[test]
fn module_instance_unknown_builtin_parameter_is_rejected_before_graph_preparation() {
    let diagnostics = validation_diagnostics(
        r#"
metadata:
  name: Unknown Builtin Parameter
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: filt
    type: filter
    parameters:
      not_declared: 1.0
"#,
    );

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.error_code() == error_codes::VALIDATION_INVALID_VALUE),
        "expected stable validation error code, got: {diagnostics:#?}"
    );
    assert_has_diagnostic_for_parameter(&diagnostics, "filt", "not_declared");
}

#[test]
fn module_instance_parameter_is_rejected_when_builtin_declares_no_static_parameters() {
    let diagnostics = validation_diagnostics(
        r#"
metadata:
  name: No Static Parameters
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: osc
    type: oscillator
    parameters:
      frequency_hz: 55
"#,
    );

    assert_has_diagnostic_for_parameter(&diagnostics, "osc", "frequency_hz");
}

#[test]
fn patch_level_parameter_is_rejected_when_builtin_declares_no_static_parameters() {
    let diagnostics = validation_diagnostics(
        r#"
metadata:
  name: Patch No Static Parameters
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
parameters:
  osc:
    frequency_hz: 55
modules:
  - id: osc
    type: oscillator
"#,
    );

    assert_has_diagnostic_for_parameter(&diagnostics, "osc", "frequency_hz");
}

#[test]
fn preset_parameter_is_rejected_when_builtin_declares_no_static_parameters() {
    let diagnostics = validation_diagnostics(
        r#"
metadata:
  name: Preset No Static Parameters
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
presets:
  tuned:
    osc:
      frequency_hz: 55
modules:
  - id: osc
    type: oscillator
"#,
    );

    assert_has_diagnostic_for_parameter(&diagnostics, "osc", "frequency_hz");
}

#[test]
fn module_instance_invalid_enum_parameter_is_rejected_before_graph_preparation() {
    let diagnostics = validation_diagnostics(
        r#"
metadata:
  name: Invalid Enum Parameter
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: filt
    type: filter
    parameters:
      algorithm: banana
"#,
    );

    assert_has_diagnostic_for_parameter(&diagnostics, "filt", "algorithm");
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic
                .actual()
                .is_some_and(|actual| actual.contains("banana"))
                && diagnostic
                    .expected()
                    .is_some_and(|expected| expected.contains("moog"))
        }),
        "expected enum diagnostic with actual value and allowed values, got: {diagnostics:#?}"
    );
}

#[test]
fn module_instance_number_below_declared_minimum_is_rejected_before_graph_preparation() {
    let diagnostics = validation_diagnostics(
        r#"
metadata:
  name: Number Below Minimum
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: spectral
    type: spectral_processor
    parameters:
      fft_size: 64
"#,
    );

    assert_has_diagnostic_for_parameter(&diagnostics, "spectral", "fft_size");
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic
                .expected()
                .is_some_and(|expected| expected.contains("256"))
                && diagnostic
                    .actual()
                    .is_some_and(|actual| actual.contains("64"))
        }),
        "expected range diagnostic with minimum and actual value, got: {diagnostics:#?}"
    );
}

#[test]
fn module_instance_wrong_parameter_type_is_rejected_before_graph_preparation() {
    let diagnostics = validation_diagnostics(
        r#"
metadata:
  name: Wrong Parameter Type
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: spectral
    type: spectral_processor
    parameters:
      fft_size: fast
"#,
    );

    assert_has_diagnostic_for_parameter(&diagnostics, "spectral", "fft_size");
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic
                .expected()
                .is_some_and(|expected| expected.contains("number"))
                && diagnostic
                    .actual()
                    .is_some_and(|actual| actual.contains("string"))
        }),
        "expected type diagnostic with expected and actual scalar types, got: {diagnostics:#?}"
    );
}

#[test]
fn patch_level_parameter_targeting_known_module_must_be_declared_by_that_module_type() {
    let diagnostics = validation_diagnostics(
        r#"
metadata:
  name: Patch Parameter Unknown Target
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
parameters:
  filt:
    not_declared: 0.5
modules:
  - id: filt
    type: filter
"#,
    );

    assert_has_diagnostic_for_parameter(&diagnostics, "filt", "not_declared");
}

#[test]
fn preset_parameter_targeting_known_module_must_be_declared_by_that_module_type() {
    let diagnostics = validation_diagnostics(
        r#"
metadata:
  name: Preset Unknown Target
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
presets:
  punchy:
    filt:
      not_declared: 0.5
modules:
  - id: filt
    type: filter
"#,
    );

    assert_has_diagnostic_for_parameter(&diagnostics, "filt", "not_declared");
}

#[test]
fn omitted_optional_module_parameters_resolve_to_declared_defaults() {
    let patch = load_patch_str(
        r#"
metadata:
  name: Defaults Resolve
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

    let resolved = resolve_module_parameters(&patch).expect("parameters should resolve");
    let filt = resolved
        .get("filt")
        .expect("filter parameters should resolve");

    assert_eq!(
        filt.get("algorithm"),
        Some(&ParameterValue::Text("moog".into()))
    );
    assert_eq!(
        filt.get("mode"),
        Some(&ParameterValue::Text("lowpass".into()))
    );
    assert_eq!(
        filt.get("comb_type"),
        Some(&ParameterValue::Text("feedback".into()))
    );
}

#[test]
fn module_and_patch_values_override_declared_defaults_before_graph_preparation() {
    let patch = load_patch_str(
        r#"
metadata:
  name: Overrides Resolve
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
parameters:
  filt:
    mode: highpass
modules:
  - id: filt
    type: filter
    parameters:
      algorithm: biquad
"#,
    )
    .expect("patch should parse");

    let resolved = resolve_module_parameters(&patch).expect("parameters should resolve");
    let filt = resolved
        .get("filt")
        .expect("filter parameters should resolve");

    assert_eq!(
        filt.get("algorithm"),
        Some(&ParameterValue::Text("biquad".into()))
    );
    assert_eq!(
        filt.get("mode"),
        Some(&ParameterValue::Text("highpass".into()))
    );
    assert_eq!(
        filt.get("comb_type"),
        Some(&ParameterValue::Text("feedback".into()))
    );
}

#[test]
fn missing_required_module_parameter_without_default_fails_resolution() {
    let patch = load_patch_str(
        r#"
metadata:
  name: Missing Required Parameter
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: sampler
    type: sampler
"#,
    )
    .expect("patch should parse");

    let diagnostics = resolve_module_parameters(&patch)
        .expect_err("sampler asset is required")
        .to_diagnostics()
        .all()
        .to_vec();

    assert_has_diagnostic_for_parameter(&diagnostics, "sampler", "asset");
}

#[test]
fn equivalent_patches_resolve_to_identical_parameter_maps_across_repeated_loads() {
    let yaml = r#"
metadata:
  name: Repeatable Defaults
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: spectral
    type: spectral_processor
"#;

    let first = resolve_module_parameters(&load_patch_str(yaml).expect("first patch should parse"))
        .expect("first patch should resolve");
    let second =
        resolve_module_parameters(&load_patch_str(yaml).expect("second patch should parse"))
            .expect("second patch should resolve");

    assert_eq!(first, second);
}
