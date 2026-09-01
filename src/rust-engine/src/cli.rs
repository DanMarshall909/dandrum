use std::path::PathBuf;

use crate::core::TimedInputEvent;
use crate::diagnostics::error_codes;
use crate::patch::{self, ParameterValue};
use crate::sample::PreparedSamplerAssets;
use crate::script::ScriptEvent;
use crate::synth::DandrumEngine;

const OUTPUT_FLAG: &str = "--output";
const PRESET_FLAG: &str = "--preset";
const SET_FLAG: &str = "--set";
const SAMPLE_RATE_FLAG: &str = "--sample-rate";
const BLOCK_SIZE_FLAG: &str = "--block-size";
const DURATION_FRAMES_FLAG: &str = "--duration-frames";
const RENDER_COMMAND: &str = "render";
const RENDER_CHORDS_COMMAND: &str = "render-chords";
const VALIDATE_COMMAND: &str = "validate";
const DEFAULT_SAMPLE_RATE_HZ: u32 = 48_000;
const DEFAULT_BLOCK_SIZE_FRAMES: u32 = 128;

#[derive(Debug, PartialEq, Eq)]
pub struct CliResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn run<I, S>(args: I) -> CliResult
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);
    let _program = args.next();

    match args.next().as_deref() {
        Some(VALIDATE_COMMAND) => validate(args.collect()),
        Some(RENDER_COMMAND) => render(args.collect()),
        Some(RENDER_CHORDS_COMMAND) => render_chords(args.collect()),
        Some("--help") | Some("-h") | None => help(),
        Some(command) => error(format!("unknown command: {command}\n\n{}", usage())),
    }
}

fn validate(args: Vec<String>) -> CliResult {
    if args.len() != 1 {
        return error(format!(
            "validate requires exactly one patch path\n\n{}",
            usage()
        ));
    }

    let patch = PathBuf::from(&args[0]);
    not_implemented(format!(
        "patch: {}\nvalidation: not implemented yet\n",
        patch.display()
    ))
}

fn render(args: Vec<String>) -> CliResult {
    let render_args = match parse_render_args(args) {
        Ok(args) => args,
        Err(message) => return error(format!("{message}\n\n{}", usage())),
    };

    render_with_events(render_args, |settings| {
        single_note_sequence(settings.sample_rate_hz)
    })
}

fn render_with_events(
    render_args: RenderArgs,
    events: impl FnOnce(&patch::RenderSettings) -> Vec<TimedInputEvent>,
) -> CliResult {
    let settings = patch::RenderSettings {
        sample_rate_hz: render_args.sample_rate_hz.unwrap_or(DEFAULT_SAMPLE_RATE_HZ),
        block_size_frames: render_args
            .block_size_frames
            .unwrap_or(DEFAULT_BLOCK_SIZE_FRAMES),
        duration_frames: render_args
            .duration_frames
            .expect("parsed render arguments always include an offline duration"),
    };

    let (sample_rate_hz, left, right) = match crate::kernel::document::load_kernel_patch_file(
        &render_args.patch,
    ) {
        Ok(kernel_patch) => {
            if render_args.preset.is_some() || !render_args.overrides.is_empty() {
                return error(
                        "failed to render patch: --preset and --set are not yet supported for kernel patch documents"
                            .to_string(),
                    );
            }

            let prepared = match crate::preparation::prepare_kernel_patch(&kernel_patch, &settings)
            {
                Ok(prepared) => prepared,
                Err(prepare_error) => {
                    return error(format!("failed to render patch: {prepare_error}"));
                }
            };
            let events = events(&settings);
            let (left, right) = crate::graph_processor::render_offline_compiled(
                prepared.compiled_patch(),
                events,
                &PreparedSamplerAssets::empty(),
            );
            (settings.sample_rate_hz, left, right)
        }
        Err(diagnostics)
            if diagnostics.all().iter().any(|diagnostic| {
                diagnostic.error_code() == error_codes::KERNEL_DOCUMENT_LEGACY_RENDER
            }) =>
        {
            match render_legacy_patch(&render_args, &settings, events) {
                Ok(rendered) => rendered,
                Err(message) => return error(message),
            }
        }
        Err(diagnostics) => {
            return error(format!("failed to render patch: {diagnostics}"));
        }
    };

    if let Err(write_error) =
        crate::wav::write_wav_file(&render_args.output, sample_rate_hz, &left, &right)
    {
        return error(format!("failed to write wav: {write_error}"));
    }

    CliResult {
        exit_code: 0,
        stdout: format!(
            "patch: {}\noutput: {}\nrender: ok\n",
            render_args.patch.display(),
            render_args.output.display()
        ),
        stderr: String::new(),
    }
}

fn render_legacy_patch(
    render_args: &RenderArgs,
    settings: &patch::RenderSettings,
    events: impl FnOnce(&patch::RenderSettings) -> Vec<TimedInputEvent>,
) -> Result<(u32, Vec<f32>, Vec<f32>), String> {
    let mut patch_doc = match patch::load_patch_file(&render_args.patch) {
        Ok(patch_doc) => patch_doc,
        Err(load_error) => return Err(format!("failed to render patch: {load_error}")),
    };
    if let Some(preset_path) = &render_args.preset {
        let preset_doc = match patch::load_preset_file(preset_path) {
            Ok(preset_doc) => preset_doc,
            Err(load_error) => return Err(format!("failed to render patch: {load_error}")),
        };
        patch_doc = match patch::apply_preset(&patch_doc, &preset_doc) {
            Ok(patch_doc) => patch_doc,
            Err(validation_error) => {
                return Err(format!("failed to render patch: {validation_error}"));
            }
        };
    }
    apply_cli_overrides(&mut patch_doc, &render_args.overrides);
    apply_legacy_host_settings(&mut patch_doc, settings);
    let base_dir = render_args
        .patch
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let prepared = match crate::preparation::prepare_instrument_document(patch_doc, base_dir) {
        Ok(prepared) => prepared,
        Err(prepare_error) => return Err(format!("failed to render patch: {prepare_error}")),
    };
    let events = events(&prepared.patch_doc().render);
    let mut engine = DandrumEngine::new();
    let render = engine.render_prepared_instrument_offline(&prepared, events);
    Ok((render.sample_rate_hz, render.left, render.right))
}

fn parse_render_args(args: Vec<String>) -> Result<RenderArgs, String> {
    if args.len() < 3 || args[1] != OUTPUT_FLAG {
        return Err("render requires: <patch> --output <wav> [--preset <preset.yaml>] [--set module.parameter=value] [--sample-rate Hz] [--block-size frames] [--duration-frames frames]".to_string());
    }

    let mut overrides = Vec::new();
    let mut preset = None;
    let mut sample_rate_hz = Some(DEFAULT_SAMPLE_RATE_HZ);
    let mut block_size_frames = Some(DEFAULT_BLOCK_SIZE_FRAMES);
    let mut duration_frames = None;
    let mut index = 3;
    while index < args.len() {
        match args[index].as_str() {
            PRESET_FLAG => {
                let Some(value) = args.get(index + 1) else {
                    return Err(format!("{PRESET_FLAG} requires preset path"));
                };
                preset = Some(PathBuf::from(value));
                index += 2;
            }
            SET_FLAG => {
                let Some(value) = args.get(index + 1) else {
                    return Err(format!("{SET_FLAG} requires module.parameter=value"));
                };
                overrides.push(parse_cli_override(value)?);
                index += 2;
            }
            SAMPLE_RATE_FLAG => {
                let Some(value) = args.get(index + 1) else {
                    return Err(format!("{SAMPLE_RATE_FLAG} requires sample rate in Hz"));
                };
                sample_rate_hz = Some(
                    value
                        .parse()
                        .map_err(|_| format!("{SAMPLE_RATE_FLAG} must be a positive integer"))?,
                );
                if sample_rate_hz == Some(0) {
                    return Err(format!("{SAMPLE_RATE_FLAG} must be a positive integer"));
                }
                index += 2;
            }
            BLOCK_SIZE_FLAG => {
                let Some(value) = args.get(index + 1) else {
                    return Err(format!("{BLOCK_SIZE_FLAG} requires block size in frames"));
                };
                block_size_frames = Some(
                    value
                        .parse()
                        .map_err(|_| format!("{BLOCK_SIZE_FLAG} must be a positive integer"))?,
                );
                if block_size_frames == Some(0) {
                    return Err(format!("{BLOCK_SIZE_FLAG} must be a positive integer"));
                }
                index += 2;
            }
            DURATION_FRAMES_FLAG => {
                let Some(value) = args.get(index + 1) else {
                    return Err(format!(
                        "{DURATION_FRAMES_FLAG} requires duration in frames"
                    ));
                };
                duration_frames =
                    Some(value.parse().map_err(|_| {
                        format!("{DURATION_FRAMES_FLAG} must be a positive integer")
                    })?);
                if duration_frames == Some(0) {
                    return Err(format!("{DURATION_FRAMES_FLAG} must be a positive integer"));
                }
                index += 2;
            }
            _ => return Err(format!("unexpected render argument: {}", args[index])),
        }
    }

    if duration_frames.is_none() {
        return Err(format!(
            "{DURATION_FRAMES_FLAG} is required for offline rendering"
        ));
    }

    Ok(RenderArgs {
        patch: PathBuf::from(&args[0]),
        output: PathBuf::from(&args[2]),
        preset,
        overrides,
        sample_rate_hz,
        block_size_frames,
        duration_frames,
    })
}

fn help() -> CliResult {
    CliResult {
        exit_code: 0,
        stdout: usage(),
        stderr: String::new(),
    }
}

fn error(message: String) -> CliResult {
    CliResult {
        exit_code: 2,
        stdout: String::new(),
        stderr: message,
    }
}

fn render_chords(args: Vec<String>) -> CliResult {
    let render_args = match parse_render_args(args) {
        Ok(args) => args,
        Err(message) => return error(format!("{message}\n\n{}", usage())),
    };

    let mut result = render_with_events(render_args, |settings| {
        chord_sequence(settings.sample_rate_hz)
    });
    result.stdout = result.stdout.replace("render: ok", "render-chords: ok");
    result
}

#[derive(Debug, PartialEq)]
struct RenderArgs {
    patch: PathBuf,
    output: PathBuf,
    preset: Option<PathBuf>,
    overrides: Vec<CliParameterOverride>,
    sample_rate_hz: Option<u32>,
    block_size_frames: Option<u32>,
    duration_frames: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
struct CliParameterOverride {
    module_id: String,
    parameter_name: String,
    value: ParameterValue,
}

fn parse_cli_override(input: &str) -> Result<CliParameterOverride, String> {
    let Some((target, raw_value)) = input.split_once('=') else {
        return Err(format!("{SET_FLAG} requires module.parameter=value"));
    };
    let Some((module_id, parameter_name)) = target.split_once('.') else {
        return Err(format!("{SET_FLAG} target must use module.parameter"));
    };
    if module_id.is_empty() || parameter_name.is_empty() || parameter_name.contains('.') {
        return Err(format!("{SET_FLAG} target must use module.parameter"));
    }

    Ok(CliParameterOverride {
        module_id: module_id.to_string(),
        parameter_name: parameter_name.to_string(),
        value: parse_cli_parameter_value(raw_value),
    })
}

fn parse_cli_parameter_value(raw_value: &str) -> ParameterValue {
    match raw_value {
        "true" => ParameterValue::Boolean(true),
        "false" => ParameterValue::Boolean(false),
        _ => raw_value
            .parse::<f64>()
            .map(ParameterValue::Number)
            .unwrap_or_else(|_| ParameterValue::Text(raw_value.to_string())),
    }
}

fn apply_cli_overrides(patch_doc: &mut patch::PatchDocument, overrides: &[CliParameterOverride]) {
    for parameter_override in overrides {
        if let Some(module) = patch_doc
            .modules
            .iter_mut()
            .find(|module| module.id == parameter_override.module_id)
        {
            module.parameters.insert(
                parameter_override.parameter_name.clone(),
                parameter_override.value.clone(),
            );
        } else {
            patch_doc
                .parameters
                .entry(parameter_override.module_id.clone())
                .or_default()
                .insert(
                    parameter_override.parameter_name.clone(),
                    parameter_override.value.clone(),
                );
        }
    }
}

fn apply_legacy_host_settings(
    patch_doc: &mut patch::PatchDocument,
    settings: &patch::RenderSettings,
) {
    patch_doc.render = settings.clone();
}

fn single_note_sequence(sample_rate: u32) -> Vec<TimedInputEvent> {
    let note_off_frame = (sample_rate as u64 / 50).max(1);
    vec![
        TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
        ),
        TimedInputEvent::new(note_off_frame, ScriptEvent::NoteOff { note: 60 }),
    ]
}

fn chord_sequence(sample_rate: u32) -> Vec<TimedInputEvent> {
    let sample_rate_hz = sample_rate as u64;
    let mut events = Vec::new();

    // Helper to add a chord with note-offs for the previous chord
    let mut prev_notes: Vec<u8> = Vec::new();

    let chords: Vec<(u64, Vec<u8>)> = vec![
        (0, vec![60, 64, 67]),                  // C major
        (sample_rate_hz, vec![65, 69, 72]),     // F major
        (2 * sample_rate_hz, vec![67, 71, 74]), // G major
        (3 * sample_rate_hz, vec![60, 64, 67]), // C major
    ];

    for (frame, notes) in &chords {
        // Note-off previous chord
        for prev in &prev_notes {
            events.push(TimedInputEvent::new(
                *frame,
                ScriptEvent::NoteOff { note: *prev },
            ));
        }
        // Note-on current chord
        for note in notes {
            events.push(TimedInputEvent::new(
                *frame,
                ScriptEvent::NoteOn {
                    note: *note,
                    velocity: 100,
                },
            ));
        }
        prev_notes = notes.clone();
    }

    // Note-off final chord
    let end = 4 * sample_rate_hz + sample_rate_hz / 4;
    for note in &prev_notes {
        events.push(TimedInputEvent::new(
            end,
            ScriptEvent::NoteOff { note: *note },
        ));
    }

    events
}

fn not_implemented(stdout: String) -> CliResult {
    CliResult {
        exit_code: 1,
        stdout,
        stderr: String::new(),
    }
}

fn usage() -> String {
    "Usage:\n  dandrum-cli validate <patch.yaml>\n  dandrum-cli render <patch.yaml> --output <output.wav> --duration-frames <frames> [--preset <preset.yaml>] [--set module.parameter=value] [--sample-rate Hz] [--block-size frames]\n  dandrum-cli render-chords <patch.yaml> --output <output.wav> --duration-frames <frames> [--preset <preset.yaml>] [--set module.parameter=value] [--sample-rate Hz] [--block-size frames]\n\nDefaults: --sample-rate 48000 --block-size 128\n".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const WAV_HEADER_BYTES: usize = 44;

    #[test]
    fn help_lists_patch_validation_and_render_commands() {
        let result = run(["dandrum-cli", "--help"]);

        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("validate <patch.yaml>"));
        assert!(
            result
                .stdout
                .contains("render <patch.yaml> --output <output.wav>")
        );
        assert!(result.stderr.is_empty());
    }

    #[test]
    fn validate_accepts_patch_path_for_future_validation() {
        let result = run(["dandrum-cli", "validate", "patches/basic.yaml"]);

        assert_eq!(result.exit_code, 1);
        assert!(result.stdout.contains("patch: patches/basic.yaml"));
        assert!(result.stdout.contains("validation: not implemented yet"));
        assert!(result.stderr.is_empty());
    }

    #[test]
    fn validate_without_exactly_one_patch_path_returns_usage_error() {
        let result = run(["dandrum-cli", "validate"]);

        assert_eq!(result.exit_code, 2);
        assert!(result.stdout.is_empty());
        assert!(
            result
                .stderr
                .contains("validate requires exactly one patch path")
        );
        assert!(result.stderr.contains("validate <patch.yaml>"));
    }

    #[test]
    fn invalid_render_arguments_return_usage_error() {
        let result = run(["dandrum-cli", "render", "patches/basic.yaml"]);

        assert_eq!(result.exit_code, 2);
        assert!(result.stdout.is_empty());
        assert!(result.stderr.contains("render requires"));
    }

    #[test]
    fn cli_set_parser_accepts_module_parameter_value_syntax() {
        let parsed = parse_cli_override("kick.tune_hz=48").expect("override should parse");

        assert_eq!(parsed.module_id, "kick");
        assert_eq!(parsed.parameter_name, "tune_hz");
        assert_eq!(parsed.value, ParameterValue::Number(48.0));
    }

    #[test]
    fn cli_set_parser_preserves_boolean_and_string_values() {
        let boolean = parse_cli_override("kick.click=true").expect("bool override should parse");
        let text = parse_cli_override("filt.algorithm=biquad").expect("text override should parse");

        assert_eq!(boolean.value, ParameterValue::Boolean(true));
        assert_eq!(text.value, ParameterValue::Text("biquad".to_string()));
    }

    #[test]
    fn cli_set_parser_rejects_targets_without_module_and_parameter() {
        assert!(parse_cli_override("kick=48").is_err());
        assert!(parse_cli_override("kick.=48").is_err());
        assert!(parse_cli_override("kick.tune.hz=48").is_err());
        assert!(parse_cli_override("kick.tune_hz").is_err());
    }

    #[test]
    fn parse_render_args_accepts_repeated_set_overrides_in_order() {
        let args = parse_render_args(vec![
            "patch.yaml".to_string(),
            OUTPUT_FLAG.to_string(),
            "out.wav".to_string(),
            DURATION_FRAMES_FLAG.to_string(),
            "48000".to_string(),
            SET_FLAG.to_string(),
            "kick.tune_hz=48".to_string(),
            SET_FLAG.to_string(),
            "kick.tune_hz=52".to_string(),
        ])
        .expect("render args should parse");

        assert_eq!(args.overrides.len(), 2);
        assert_eq!(args.preset, None);
        assert_eq!(args.overrides[0].value, ParameterValue::Number(48.0));
        assert_eq!(args.overrides[1].value, ParameterValue::Number(52.0));
    }

    #[test]
    fn parse_render_args_accepts_external_preset_path() {
        let args = parse_render_args(vec![
            "patch.yaml".to_string(),
            OUTPUT_FLAG.to_string(),
            "out.wav".to_string(),
            DURATION_FRAMES_FLAG.to_string(),
            "48000".to_string(),
            PRESET_FLAG.to_string(),
            "tight.yaml".to_string(),
        ])
        .expect("render args should parse");

        assert_eq!(args.preset, Some(PathBuf::from("tight.yaml")));
        assert!(args.overrides.is_empty());
    }

    #[test]
    fn parse_render_args_accepts_sample_rate_block_size_and_duration_flags() {
        let args = parse_render_args(vec![
            "patch.yaml".to_string(),
            OUTPUT_FLAG.to_string(),
            "out.wav".to_string(),
            SAMPLE_RATE_FLAG.to_string(),
            "22050".to_string(),
            BLOCK_SIZE_FLAG.to_string(),
            "256".to_string(),
            DURATION_FRAMES_FLAG.to_string(),
            "96000".to_string(),
        ])
        .expect("render args should parse");

        assert_eq!(args.sample_rate_hz, Some(22050));
        assert_eq!(args.block_size_frames, Some(256));
        assert_eq!(args.duration_frames, Some(96000));
    }

    #[test]
    fn parse_render_args_defaults_preparation_settings_when_duration_is_supplied() {
        let args = parse_render_args(vec![
            "patch.yaml".to_string(),
            OUTPUT_FLAG.to_string(),
            "out.wav".to_string(),
            DURATION_FRAMES_FLAG.to_string(),
            "96000".to_string(),
        ])
        .expect("render args should parse");

        assert_eq!(args.sample_rate_hz, Some(48_000));
        assert_eq!(args.block_size_frames, Some(128));
        assert_eq!(args.duration_frames, Some(96_000));
    }

    #[test]
    fn parse_render_args_requires_duration_frames() {
        let result = parse_render_args(vec![
            "patch.yaml".to_string(),
            OUTPUT_FLAG.to_string(),
            "out.wav".to_string(),
        ]);

        let message = result.expect_err("offline duration must be explicit");
        assert!(message.contains(DURATION_FRAMES_FLAG));
    }

    #[test]
    fn parse_render_args_rejects_non_integer_sample_rate() {
        let result = parse_render_args(vec![
            "patch.yaml".to_string(),
            OUTPUT_FLAG.to_string(),
            "out.wav".to_string(),
            SAMPLE_RATE_FLAG.to_string(),
            "abc".to_string(),
            DURATION_FRAMES_FLAG.to_string(),
            "48000".to_string(),
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn parse_render_args_rejects_zero_host_settings() {
        for (flag, value) in [
            (SAMPLE_RATE_FLAG, "0"),
            (BLOCK_SIZE_FLAG, "0"),
            (DURATION_FRAMES_FLAG, "0"),
        ] {
            let mut args = vec![
                "patch.yaml".to_string(),
                OUTPUT_FLAG.to_string(),
                "out.wav".to_string(),
                DURATION_FRAMES_FLAG.to_string(),
                "48000".to_string(),
            ];
            args.extend([flag.to_string(), value.to_string()]);

            assert!(parse_render_args(args).is_err(), "{flag} must reject zero");
        }
    }

    #[test]
    fn apply_legacy_host_settings_sets_patch_render_fields() {
        let mut patch = patch::load_patch_str(
            r#"
metadata:
  name: Render Override Test
render:
  sample_rate_hz: 48000
  block_size_frames: 128
modules:
  - id: osc
    type: oscillator
"#,
        )
        .expect("patch should parse");
        let args = RenderArgs {
            patch: PathBuf::from("test.yaml"),
            output: PathBuf::from("out.wav"),
            preset: None,
            overrides: Vec::new(),
            sample_rate_hz: Some(22050),
            block_size_frames: Some(256),
            duration_frames: Some(96000),
        };

        let settings = patch::RenderSettings {
            sample_rate_hz: args.sample_rate_hz.unwrap(),
            block_size_frames: args.block_size_frames.unwrap(),
            duration_frames: args.duration_frames.unwrap(),
        };
        apply_legacy_host_settings(&mut patch, &settings);

        assert_eq!(patch.render.sample_rate_hz, 22050);
        assert_eq!(patch.render.block_size_frames, 256);
        assert_eq!(patch.render.duration_frames, 96000);
    }

    #[test]
    fn yaml_without_duration_frames_uses_default() {
        let patch = patch::load_patch_str(
            r#"
metadata:
  name: No Duration
render:
  sample_rate_hz: 48000
  block_size_frames: 128
modules:
  - id: osc
    type: oscillator
"#,
        )
        .expect("patch should parse");

        assert_eq!(patch.render.duration_frames, patch::DEFAULT_DURATION_FRAMES);
    }

    #[test]
    fn cli_overrides_apply_after_yaml_values_and_last_repeated_value_wins() {
        let mut patch = patch::load_patch_str(
            r#"
metadata:
  name: CLI Override Apply
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: filt
    type: filter
    parameters:
      algorithm: moog
"#,
        )
        .expect("patch should parse");
        let overrides = vec![
            parse_cli_override("filt.algorithm=biquad").expect("override should parse"),
            parse_cli_override("filt.algorithm=comb").expect("override should parse"),
        ];

        apply_cli_overrides(&mut patch, &overrides);

        assert_eq!(
            patch.modules[0].parameters.get("algorithm"),
            Some(&ParameterValue::Text("comb".to_string()))
        );
    }

    #[test]
    fn cli_override_validation_rejects_unknown_module() {
        let mut patch = patch::load_patch_str(
            r#"
metadata:
  name: CLI Unknown Module
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
        let overrides =
            vec![parse_cli_override("missing.algorithm=moog").expect("override parses")];

        apply_cli_overrides(&mut patch, &overrides);
        let diagnostics = patch::validate_patch_schema(&patch)
            .expect_err("unknown module override should fail")
            .to_diagnostics();

        assert!(
            diagnostics
                .all()
                .iter()
                .any(|diagnostic| diagnostic.module_id() == Some("missing"))
        );
    }

    #[test]
    fn cli_override_validation_uses_declaration_type_range_and_enum_checks() {
        let mut patch = patch::load_patch_str(
            r#"
metadata:
  name: CLI Invalid Values
render:
  sample_rate_hz: 48000
  block_size_frames: 128
  duration_frames: 128
modules:
  - id: filt
    type: filter
  - id: spectral
    type: spectral_processor
"#,
        )
        .expect("patch should parse");
        let overrides = vec![
            parse_cli_override("filt.algorithm=banana").expect("enum override parses"),
            parse_cli_override("spectral.fft_size=64").expect("range override parses"),
            parse_cli_override("spectral.mix=wide").expect("type override parses"),
        ];

        apply_cli_overrides(&mut patch, &overrides);
        let diagnostics = patch::validate_patch_schema(&patch)
            .expect_err("invalid overrides should fail")
            .to_diagnostics();

        assert!(
            diagnostics
                .all()
                .iter()
                .any(|diagnostic| diagnostic.message().contains("algorithm"))
        );
        assert!(
            diagnostics
                .all()
                .iter()
                .any(|diagnostic| diagnostic.message().contains("fft_size"))
        );
        assert!(
            diagnostics
                .all()
                .iter()
                .any(|diagnostic| diagnostic.message().contains("mix"))
        );
    }

    #[test]
    fn unknown_command_returns_usage_error() {
        let result = run(["dandrum-cli", "inspect"]);

        assert_eq!(result.exit_code, 2);
        assert!(result.stdout.is_empty());
        assert!(result.stderr.contains("unknown command: inspect"));
        assert!(result.stderr.contains("Usage:"));
    }

    #[test]
    fn render_command_writes_deterministic_non_empty_wav_for_event_routing_dogfood_examples() {
        for patch_name in [
            "event-routing-drum-machine.yaml",
            "event-routing-simple-poly-synth.yaml",
        ] {
            let patch_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("examples")
                .join("patches")
                .join(patch_name);
            let first_output = temp_wav_path(patch_name, "first");
            let second_output = temp_wav_path(patch_name, "second");

            let first = run([
                "dandrum-cli".to_string(),
                "render".to_string(),
                patch_path.to_string_lossy().to_string(),
                "--output".to_string(),
                first_output.to_string_lossy().to_string(),
                DURATION_FRAMES_FLAG.to_string(),
                "48000".to_string(),
            ]);
            let second = run([
                "dandrum-cli".to_string(),
                "render".to_string(),
                patch_path.to_string_lossy().to_string(),
                "--output".to_string(),
                second_output.to_string_lossy().to_string(),
                DURATION_FRAMES_FLAG.to_string(),
                "48000".to_string(),
            ]);

            assert_eq!(first.exit_code, 0, "{}", first.stderr);
            assert_eq!(second.exit_code, 0, "{}", second.stderr);

            let first_bytes = fs::read(&first_output).expect("first WAV should be readable");
            let second_bytes = fs::read(&second_output).expect("second WAV should be readable");
            assert_eq!(first_bytes, second_bytes);
            assert!(first_bytes.len() > WAV_HEADER_BYTES);
            assert!(
                first_bytes[WAV_HEADER_BYTES..]
                    .iter()
                    .any(|byte| *byte != 0),
                "{patch_name} should render non-empty WAV audio"
            );

            let _ = fs::remove_file(first_output);
            let _ = fs::remove_file(second_output);
        }
    }

    #[test]
    fn render_command_accepts_kernel_patch_with_host_owned_settings() {
        let patch_path = example_path("patches", "synthetic-snare.yaml");
        let output = temp_wav_path("synthetic-snare", "kernel");

        let result = run([
            "dandrum-cli".to_string(),
            "render".to_string(),
            patch_path.to_string_lossy().to_string(),
            OUTPUT_FLAG.to_string(),
            output.to_string_lossy().to_string(),
            DURATION_FRAMES_FLAG.to_string(),
            "2048".to_string(),
        ]);

        assert_eq!(result.exit_code, 0, "{}", result.stderr);
        let bytes = fs::read(&output).expect("kernel render WAV should be readable");
        assert_eq!(bytes.len(), WAV_HEADER_BYTES + 2048 * 2 * 2);
        assert_eq!(
            u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
            DEFAULT_SAMPLE_RATE_HZ
        );
        assert!(bytes[WAV_HEADER_BYTES..].iter().any(|byte| *byte != 0));

        let _ = fs::remove_file(output);
    }

    #[test]
    fn render_command_loads_patch_with_external_preset_file() {
        let patch_path = example_path("patches", "synthetic-808-kick.yaml");
        let preset_path = example_path("presets", "tight-808-kick.yaml");
        let output = temp_wav_path("tight-808-kick", "preset");

        let result = run([
            "dandrum-cli".to_string(),
            "render".to_string(),
            patch_path.to_string_lossy().to_string(),
            OUTPUT_FLAG.to_string(),
            output.to_string_lossy().to_string(),
            PRESET_FLAG.to_string(),
            preset_path.to_string_lossy().to_string(),
            DURATION_FRAMES_FLAG.to_string(),
            "48000".to_string(),
        ]);

        assert_eq!(result.exit_code, 0, "{}", result.stderr);
        assert!(result.stdout.contains("render: ok"));
        assert!(fs::metadata(&output).expect("WAV should exist").len() > WAV_HEADER_BYTES as u64);

        let _ = fs::remove_file(output);
    }

    fn example_path(kind: &str, name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples")
            .join(kind)
            .join(name)
    }

    fn temp_wav_path(patch_name: &str, label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "dandrum-{patch_name}-{label}-{}.wav",
            std::process::id()
        ))
    }
}
