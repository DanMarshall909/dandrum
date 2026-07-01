use std::path::PathBuf;

use crate::core::TimedInputEvent;
use crate::script::ScriptEvent;
use crate::synth::DandrumEngine;

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
        Some("render-chords") => render_chords(args.collect()),
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
    let mut engine = DandrumEngine::new();
    let render = match engine
        .render_patch_file_offline_with_events(&patch, |settings| single_note_sequence(settings.sample_rate_hz))
    {
        Ok(render) => render,
        Err(load_error) => return error(format!("failed to render patch: {load_error}")),
    };
    if let Err(write_error) =
        crate::wav::write_wav_file(&output, render.sample_rate_hz, &render.left, &render.right)
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

fn render_chords(args: Vec<String>) -> CliResult {
    if args.len() != 3 || args[1] != "--output" {
        return error(format!(
            "render-chords requires: <patch.yaml> --output <wav>\n\n{}",
            usage()
        ));
    }

    let patch = PathBuf::from(&args[0]);
    let output = PathBuf::from(&args[2]);
    let mut engine = DandrumEngine::new();
    let render = match engine
        .render_patch_file_offline_with_events(&patch, |settings| chord_sequence(settings.sample_rate_hz))
    {
        Ok(render) => render,
        Err(load_error) => return error(format!("failed to render patch: {load_error}")),
    };
    if let Err(write_error) =
        crate::wav::write_wav_file(&output, render.sample_rate_hz, &render.left, &render.right)
    {
        return error(format!("failed to write wav: {write_error}"));
    }

    CliResult {
        exit_code: 0,
        stdout: format!(
            "patch: {}\noutput: {}\nrender-chords: ok\n",
            patch.display(),
            output.display()
        ),
        stderr: String::new(),
    }
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
        (0, vec![60, 64, 67]),      // C major
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
    "Usage:\n  dandrum-cli validate <patch.yaml>\n  dandrum-cli render <patch.yaml> --output <output.wav>\n  dandrum-cli render-chords <patch.yaml> --output <output.wav>\n".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn unknown_command_returns_usage_error() {
        let result = run(["dandrum-cli", "inspect"]);

        assert_eq!(result.exit_code, 2);
        assert!(result.stdout.is_empty());
        assert!(result.stderr.contains("unknown command: inspect"));
        assert!(result.stderr.contains("Usage:"));
    }
}
