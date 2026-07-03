use dandrum_engine::diagnostics::{error_codes, Diagnostic};
use dandrum_engine::patch::{load_patch_str, validate_patch_schema};

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
            diagnostic.actual().is_some_and(|actual| actual.contains("banana"))
                && diagnostic.expected().is_some_and(|expected| expected.contains("moog"))
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
            diagnostic.expected().is_some_and(|expected| expected.contains("256"))
                && diagnostic.actual().is_some_and(|actual| actual.contains("64"))
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
            diagnostic.expected().is_some_and(|expected| expected.contains("number"))
                && diagnostic.actual().is_some_and(|actual| actual.contains("string"))
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
