use std::path::PathBuf;

use crate::core::TimedInputEvent;
use crate::graph::Graph;
use crate::script::ScriptEvent;

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
        Some("validate") => validate(args.collect()),
        Some("render") => render(args.collect()),
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
    if args.len() != 3 || args[1] != "--output" {
        return error(format!(
            "render requires: <patch> --output <wav>\n\n{}",
            usage()
        ));
    }

    let patch = PathBuf::from(&args[0]);
    let output = PathBuf::from(&args[2]);
    let patch_doc = match crate::patch::load_patch_file(&patch) {
        Ok(patch_doc) => patch_doc,
        Err(load_error) => return error(format!("failed to load patch: {load_error}")),
    };
    if let Err(validation_error) = crate::patch::validate_patch_schema(&patch_doc) {
        return error(format!("patch validation failed: {validation_error}"));
    }

    let graph = Graph::from_patch_declarations(&patch_doc);
    if let Err(validation_error) = graph.validate() {
        return error(format!("graph validation failed: {validation_error}"));
    }

    let base_dir = patch.parent().unwrap_or_else(|| std::path::Path::new("."));
    let sampler_assets = match crate::sample::prepare_sampler_assets(&patch_doc, base_dir) {
        Ok(assets) => assets,
        Err(load_error) => return error(load_error.to_string()),
    };

    let (left, right) = crate::graph_processor::render_offline_with_sampler_assets(
        &graph,
        &patch_doc.render,
        vec![TimedInputEvent::new(
            0,
            ScriptEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
        )],
        &sampler_assets,
    );
    if let Err(write_error) =
        crate::wav::write_wav_file(&output, patch_doc.render.sample_rate_hz, &left, &right)
    {
        return error(format!("failed to write wav: {write_error}"));
    }

    CliResult {
        exit_code: 0,
        stdout: format!(
            "patch: {}\noutput: {}\nrender: ok\n",
            patch.display(),
            output.display()
        ),
        stderr: String::new(),
    }
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

fn not_implemented(stdout: String) -> CliResult {
    CliResult {
        exit_code: 1,
        stdout,
        stderr: String::new(),
    }
}

fn usage() -> String {
    "Usage:\n  dandrum-cli validate <patch.yaml>\n  dandrum-cli render <patch.yaml> --output <output.wav>\n".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

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
    fn render_writes_sampler_example_to_non_empty_wav_file() {
        let dir = unique_temp_dir("render_writes_sampler_example_to_non_empty_wav_file");
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let patch = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/patches/minimal-sampler.yaml");
        let output = dir.join("out.wav");
        let result = run([
            "dandrum-cli",
            "render",
            patch.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ]);

        assert_eq!(result.exit_code, 0, "{}", result.stderr);
        assert!(result.stdout.contains("minimal-sampler.yaml"));
        assert!(result.stdout.contains("render: ok"));
        assert!(result.stderr.is_empty());
        assert!(fs::metadata(&output).unwrap().len() > 44);
    }

    #[test]
    fn invalid_render_arguments_return_usage_error() {
        let result = run(["dandrum-cli", "render", "patches/basic.yaml"]);

        assert_eq!(result.exit_code, 2);
        assert!(result.stdout.is_empty());
        assert!(result.stderr.contains("render requires"));
    }

    #[test]
    fn unknown_command_returns_usage_error() {
        let result = run(["dandrum-cli", "inspect"]);

        assert_eq!(result.exit_code, 2);
        assert!(result.stdout.is_empty());
        assert!(result.stderr.contains("unknown command: inspect"));
        assert!(result.stderr.contains("Usage:"));
    }

    fn unique_temp_dir(test_name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("dandrum-{test_name}-{nanos}"))
    }
}
