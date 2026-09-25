use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::core::TimedInputEvent;
use crate::patch::RenderSettings;
use crate::sound_analysis::{AnalysisFrame, AnalysisSettings};

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct SoundFixture {
    pub version: u32,
    pub name: String,
    pub patch: PathBuf,
    pub render: RenderSettings,
    pub analysis: AnalysisSettings,
    pub timeline: SoundTimeline,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct SoundTimeline {
    pub tempo_bpm: f64,
    pub steps_per_bar: u32,
    pub start_frame: u64,
    pub length_frames: u64,
    pub repetitions: u32,
    pub events: Vec<SoundEvent>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SoundEvent {
    NoteOn { frame: u64, note: u8, velocity: u8 },
    NoteOff { frame: u64, note: u8 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct SoundRender {
    pub sample_rate_hz: u32,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
    pub metrics: Vec<AnalysisFrame>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkbenchResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn load_sound_fixture_file(path: impl AsRef<Path>) -> Result<SoundFixture, String> {
    let path = path.as_ref();
    let yaml = fs::read_to_string(path)
        .map_err(|error| format!("failed to read sound fixture {}: {error}", path.display()))?;
    let mut fixture: SoundFixture = serde_yaml::from_str(&yaml)
        .map_err(|error| format!("failed to parse sound fixture {}: {error}", path.display()))?;

    validate_fixture(&fixture)?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    fixture.patch = base_dir
        .join(&fixture.patch)
        .canonicalize()
        .map_err(|error| {
            format!(
                "failed to resolve patch {}: {error}",
                fixture.patch.display()
            )
        })?;
    Ok(fixture)
}

pub fn expand_timeline_events(fixture: &SoundFixture) -> Result<Vec<TimedInputEvent>, String> {
    let mut events =
        Vec::with_capacity(fixture.timeline.events.len() * fixture.timeline.repetitions as usize);
    for repetition in 0..u64::from(fixture.timeline.repetitions) {
        let loop_offset = repetition
            .checked_mul(fixture.timeline.length_frames)
            .and_then(|offset| fixture.timeline.start_frame.checked_add(offset))
            .ok_or_else(|| "sound fixture timeline frame overflow".to_string())?;
        for event in &fixture.timeline.events {
            let frame = loop_offset
                .checked_add(event.frame())
                .ok_or_else(|| "sound fixture event frame overflow".to_string())?;
            let event = match event {
                SoundEvent::NoteOn { note, velocity, .. } => crate::script::ScriptEvent::NoteOn {
                    note: *note,
                    velocity: *velocity,
                },
                SoundEvent::NoteOff { note, .. } => {
                    crate::script::ScriptEvent::NoteOff { note: *note }
                }
            };
            events.push(TimedInputEvent::new(frame, event));
        }
    }
    events.sort_by_key(TimedInputEvent::frame);
    Ok(events)
}

pub fn render_sound_fixture(fixture: &SoundFixture) -> Result<SoundRender, String> {
    let mut patch_doc = crate::patch::load_patch_file(&fixture.patch)
        .map_err(|error| format!("failed to load sound fixture patch: {error}"))?;
    patch_doc.render = fixture.render.clone();
    let patch_root = fixture.patch.parent().unwrap_or_else(|| Path::new("."));
    let prepared = crate::preparation::prepare_instrument_document(patch_doc, patch_root)
        .map_err(|error| format!("failed to prepare sound fixture patch: {error}"))?;
    let events = expand_timeline_events(fixture)?;
    let mut engine = crate::synth::DandrumEngine::new();
    let render = engine.render_prepared_instrument_offline(&prepared, events);
    let mono: Vec<f32> = render
        .left
        .iter()
        .zip(&render.right)
        .map(|(left, right)| (left + right) * 0.5)
        .collect();
    let metrics =
        crate::sound_analysis::analyze_sound(&mono, render.sample_rate_hz, fixture.analysis)?;

    Ok(SoundRender {
        sample_rate_hz: render.sample_rate_hz,
        left: render.left,
        right: render.right,
        metrics,
    })
}

pub fn analyze_external_wav(
    fixture: &SoundFixture,
    wav_path: impl AsRef<Path>,
) -> Result<Vec<AnalysisFrame>, String> {
    let audio =
        crate::audio_loading::load_pcm_wav(wav_path.as_ref(), fixture.render.sample_rate_hz)?;
    crate::sound_analysis::analyze_sound(audio.frames(), audio.sample_rate_hz(), fixture.analysis)
}

pub fn run_sound_workbench<I, S>(args: I) -> WorkbenchResult
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args: Vec<String> = args.into_iter().map(Into::into).collect();
    if !args.is_empty() {
        args.remove(0);
    }
    match args.first().map(String::as_str) {
        Some("render") => run_render_command(&args),
        Some("analyze") => run_analyze_command(&args),
        Some("--help") | Some("-h") | None => workbench_success(workbench_usage()),
        Some(command) => workbench_error(format!(
            "unknown sound workbench command: {command}\n\n{}",
            workbench_usage()
        )),
    }
}

fn run_render_command(args: &[String]) -> WorkbenchResult {
    if args.len() != 6 || args[2] != "--output-wav" || args[4] != "--output-metrics" {
        return workbench_error(format!(
            "render requires a fixture, WAV output, and metrics output\n\n{}",
            workbench_usage()
        ));
    }
    let fixture = match load_sound_fixture_file(&args[1]) {
        Ok(fixture) => fixture,
        Err(error) => return workbench_error(error),
    };
    let render = match render_sound_fixture(&fixture) {
        Ok(render) => render,
        Err(error) => return workbench_error(error),
    };
    if let Err(error) =
        crate::wav::write_wav_file(&args[3], render.sample_rate_hz, &render.left, &render.right)
    {
        return workbench_error(format!("failed to write audition WAV: {error}"));
    }
    if let Err(error) = write_metrics_file(&args[5], &render.metrics) {
        return workbench_error(error);
    }

    workbench_success(format!(
        "fixture: {}\naudio: {}\nmetrics: {}\nrender: ok\n",
        args[1], args[3], args[5]
    ))
}

fn run_analyze_command(args: &[String]) -> WorkbenchResult {
    if args.len() != 5 || args[3] != "--output-metrics" {
        return workbench_error(format!(
            "analyze requires a fixture, aligned PCM WAV, and metrics output\n\n{}",
            workbench_usage()
        ));
    }
    let fixture = match load_sound_fixture_file(&args[1]) {
        Ok(fixture) => fixture,
        Err(error) => return workbench_error(error),
    };
    let metrics = match analyze_external_wav(&fixture, &args[2]) {
        Ok(metrics) => metrics,
        Err(error) => return workbench_error(error),
    };
    if let Err(error) = write_metrics_file(&args[4], &metrics) {
        return workbench_error(error);
    }

    workbench_success(format!(
        "fixture: {}\nreference: {}\nmetrics: {}\nanalyze: ok\n",
        args[1], args[2], args[4]
    ))
}

fn write_metrics_file(path: impl AsRef<Path>, metrics: &[AnalysisFrame]) -> Result<(), String> {
    let path = path.as_ref();
    let file = File::create(path)
        .map_err(|error| format!("failed to create metrics CSV {}: {error}", path.display()))?;
    crate::sound_analysis::write_metrics_csv(file, metrics)
        .map_err(|error| format!("failed to write metrics CSV {}: {error}", path.display()))
}

fn workbench_success(stdout: String) -> WorkbenchResult {
    WorkbenchResult {
        exit_code: 0,
        stdout,
        stderr: String::new(),
    }
}

fn workbench_error(stderr: String) -> WorkbenchResult {
    WorkbenchResult {
        exit_code: 2,
        stdout: String::new(),
        stderr,
    }
}

fn workbench_usage() -> String {
    "Usage:\n  dandrum-sound-workbench render <fixture.yaml> --output-wav <output.wav> --output-metrics <metrics.csv>\n  dandrum-sound-workbench analyze <fixture.yaml> <aligned-reference.wav> --output-metrics <metrics.csv>\n".to_string()
}

impl SoundEvent {
    fn frame(&self) -> u64 {
        match self {
            Self::NoteOn { frame, .. } | Self::NoteOff { frame, .. } => *frame,
        }
    }
}

fn validate_fixture(fixture: &SoundFixture) -> Result<(), String> {
    if fixture.version != 1 {
        return Err(format!(
            "unsupported sound fixture version {}; expected 1",
            fixture.version
        ));
    }
    if fixture.name.trim().is_empty() {
        return Err("sound fixture name must not be empty".to_string());
    }
    if fixture.render.sample_rate_hz == 0
        || fixture.render.block_size_frames == 0
        || fixture.render.duration_frames == 0
    {
        return Err("sound fixture render settings must be positive".to_string());
    }
    let timeline = &fixture.timeline;
    if !timeline.tempo_bpm.is_finite() || timeline.tempo_bpm <= 0.0 {
        return Err("sound fixture tempo must be finite and positive".to_string());
    }
    if timeline.steps_per_bar == 0 || timeline.length_frames == 0 || timeline.repetitions == 0 {
        return Err("sound fixture timeline sizes must be positive".to_string());
    }
    if timeline
        .events
        .iter()
        .any(|event| event.frame() >= timeline.length_frames)
    {
        return Err("sound fixture events must fall within one loop".to_string());
    }
    let timeline_end = u64::from(timeline.repetitions)
        .checked_mul(timeline.length_frames)
        .and_then(|length| timeline.start_frame.checked_add(length))
        .ok_or_else(|| "sound fixture timeline frame overflow".to_string())?;
    if timeline_end > fixture.render.duration_frames {
        return Err("sound fixture timeline exceeds render duration".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::ScriptEvent;
    use std::collections::BTreeSet;
    use std::fs;

    fn acid_fixture_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/sound-design/tb303-acid-poc.yaml")
    }

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("dandrum-{name}-{}", std::process::id()))
    }

    fn write_fixture_variant(name: &str, replace_from: &str, replace_to: &str) -> PathBuf {
        let output_dir = temp_dir("sound-fixture-validation");
        fs::create_dir_all(&output_dir).expect("fixture temp directory should be created");
        let yaml = fs::read_to_string(acid_fixture_path()).expect("source fixture should be read");
        assert!(
            yaml.contains(replace_from),
            "fixture replacement source should exist: {replace_from}"
        );
        let path = output_dir.join(format!("{name}.yaml"));
        fs::write(&path, yaml.replacen(replace_from, replace_to, 1))
            .expect("fixture variant should write");
        path
    }

    fn has_note_on(events: &[TimedInputEvent], frame: u64, note: u8, velocity: u8) -> bool {
        events.iter().any(|event| {
            event.frame() == frame && event.event() == &ScriptEvent::NoteOn { note, velocity }
        })
    }

    fn has_note_off(events: &[TimedInputEvent], frame: u64, note: u8) -> bool {
        events
            .iter()
            .any(|event| event.frame() == frame && event.event() == &ScriptEvent::NoteOff { note })
    }

    #[test]
    fn acid_fixture_reproduces_accents_rests_ties_and_slides() {
        let fixture =
            load_sound_fixture_file(acid_fixture_path()).expect("TB-303 sound fixture should load");
        let events = expand_timeline_events(&fixture).expect("timeline should expand");
        let bar_start = fixture.timeline.start_frame;
        let step = fixture.timeline.length_frames / u64::from(fixture.timeline.steps_per_bar);

        assert_eq!(fixture.version, 1);
        assert!(fixture.patch.ends_with("examples/patches/tb303-acid.yaml"));
        assert_eq!(fixture.render.sample_rate_hz, 48_000);
        assert_eq!(fixture.timeline.tempo_bpm, 120.0);
        assert_eq!(fixture.timeline.steps_per_bar, 16);
        assert_eq!(fixture.timeline.repetitions, 4);

        assert!(has_note_on(&events, bar_start, 36, 64));
        assert!(has_note_on(&events, bar_start + step, 36, 127));

        let note_on_frames: BTreeSet<u64> = events
            .iter()
            .filter_map(|event| match event.event() {
                ScriptEvent::NoteOn { .. } => Some(event.frame()),
                ScriptEvent::NoteOff { .. } => None,
            })
            .collect();

        for rest_step in [3_u64, 8, 11] {
            let rest_frame = bar_start + rest_step * step;
            assert!(!note_on_frames.contains(&rest_frame));
        }

        let tied_note_on = bar_start + 4 * step;
        assert!(has_note_on(&events, tied_note_on, 48, 127));
        assert!(!note_on_frames.contains(&(tied_note_on + step)));
        assert!(has_note_off(&events, tied_note_on + step + 3_000, 48));

        let slide_target = bar_start + 2 * step;
        assert!(has_note_on(&events, slide_target, 43, 64));
        assert!(has_note_off(&events, slide_target + 480, 36));

        let second_bar = bar_start + fixture.timeline.length_frames;
        assert!(has_note_on(&events, second_bar, 36, 64));
        assert_eq!(events.len(), fixture.timeline.events.len() * 4);
    }

    #[test]
    fn acid_fixture_renders_deterministic_finite_non_silent_audio_and_metrics() {
        let fixture =
            load_sound_fixture_file(acid_fixture_path()).expect("TB-303 sound fixture should load");

        let first = render_sound_fixture(&fixture).expect("first fixture render should succeed");
        let second = render_sound_fixture(&fixture).expect("second fixture render should succeed");

        assert_eq!(first.sample_rate_hz, 48_000);
        assert_eq!(first.left, second.left);
        assert_eq!(first.right, second.right);
        assert_eq!(first.metrics, second.metrics);
        assert_eq!(first.left.len(), fixture.render.duration_frames as usize);
        assert!(
            first
                .left
                .iter()
                .chain(&first.right)
                .all(|sample| sample.is_finite())
        );
        assert!(first.left.iter().any(|sample| sample.abs() > 1.0e-6));
        assert!(!first.metrics.is_empty());
        assert!(
            first
                .metrics
                .iter()
                .any(|frame| frame.spectral_centroid_hz.is_some())
        );
    }

    #[test]
    fn workbench_renders_repeatable_wav_and_metrics_artifacts() {
        let output_dir = temp_dir("sound-workbench-render");
        fs::create_dir_all(&output_dir).expect("output directory should be created");
        let first_wav = output_dir.join("first.wav");
        let first_csv = output_dir.join("first.csv");
        let second_wav = output_dir.join("second.wav");
        let second_csv = output_dir.join("second.csv");
        let fixture_path = acid_fixture_path();

        let first = run_sound_workbench([
            "dandrum-sound-workbench".to_string(),
            "render".to_string(),
            fixture_path.display().to_string(),
            "--output-wav".to_string(),
            first_wav.display().to_string(),
            "--output-metrics".to_string(),
            first_csv.display().to_string(),
        ]);
        let second = run_sound_workbench([
            "dandrum-sound-workbench".to_string(),
            "render".to_string(),
            fixture_path.display().to_string(),
            "--output-wav".to_string(),
            second_wav.display().to_string(),
            "--output-metrics".to_string(),
            second_csv.display().to_string(),
        ]);

        assert_eq!(first.exit_code, 0, "{}", first.stderr);
        assert_eq!(second.exit_code, 0, "{}", second.stderr);
        let first_wav_bytes = fs::read(&first_wav).expect("first WAV should exist");
        assert!(first_wav_bytes.len() > 44);
        assert!(first_wav_bytes[44..].iter().any(|byte| *byte != 0));
        assert_eq!(
            first_wav_bytes,
            fs::read(&second_wav).expect("second WAV should exist")
        );
        let first_metrics = fs::read_to_string(&first_csv).expect("first CSV should exist");
        assert!(
            first_metrics.starts_with("start_frame,time_seconds,rms,peak,spectral_centroid_hz\n")
        );
        assert_eq!(
            first_metrics,
            fs::read_to_string(&second_csv).expect("second CSV should exist")
        );
    }

    #[test]
    fn workbench_analyzes_external_wav_and_rejects_sample_rate_mismatch() {
        let output_dir = temp_dir("sound-workbench-analyze");
        fs::create_dir_all(&output_dir).expect("output directory should be created");
        let fixture_path = acid_fixture_path();
        let reference_wav = output_dir.join("reference.wav");
        let reference_csv = output_dir.join("reference.csv");
        let wrong_rate_wav = output_dir.join("wrong-rate.wav");
        crate::wav::write_wav_file(
            &reference_wav,
            48_000,
            &vec![0.25; 2_048],
            &vec![0.25; 2_048],
        )
        .expect("reference WAV should write");
        crate::wav::write_wav_file(
            &wrong_rate_wav,
            44_100,
            &vec![0.25; 2_048],
            &vec![0.25; 2_048],
        )
        .expect("wrong-rate WAV should write");

        let result = run_sound_workbench([
            "dandrum-sound-workbench".to_string(),
            "analyze".to_string(),
            fixture_path.display().to_string(),
            reference_wav.display().to_string(),
            "--output-metrics".to_string(),
            reference_csv.display().to_string(),
        ]);
        let mismatch = run_sound_workbench([
            "dandrum-sound-workbench".to_string(),
            "analyze".to_string(),
            fixture_path.display().to_string(),
            wrong_rate_wav.display().to_string(),
            "--output-metrics".to_string(),
            output_dir.join("wrong-rate.csv").display().to_string(),
        ]);

        assert_eq!(result.exit_code, 0, "{}", result.stderr);
        let metrics = fs::read_to_string(reference_csv).expect("reference CSV should exist");
        assert!(metrics.starts_with("start_frame,time_seconds,rms,peak,spectral_centroid_hz\n"));
        assert_eq!(mismatch.exit_code, 2);
        assert!(mismatch.stderr.contains("sample-rate mismatch"));
    }

    #[test]
    fn fixture_loader_rejects_invalid_reproducibility_contracts() {
        let invalid_cases = [
            ("version", "version: 1", "version: 2", "version"),
            (
                "name",
                "name: TB-303 acid proof of concept",
                "name: ' '",
                "name",
            ),
            (
                "sample-rate",
                "sample_rate_hz: 48000",
                "sample_rate_hz: 0",
                "render settings",
            ),
            (
                "block-size",
                "block_size_frames: 64",
                "block_size_frames: 0",
                "render settings",
            ),
            (
                "duration",
                "duration_frames: 528000",
                "duration_frames: 0",
                "render settings",
            ),
            ("tempo", "tempo_bpm: 120", "tempo_bpm: 0", "tempo"),
            (
                "non-finite-tempo",
                "tempo_bpm: 120",
                "tempo_bpm: .nan",
                "tempo",
            ),
            (
                "steps",
                "steps_per_bar: 16",
                "steps_per_bar: 0",
                "timeline sizes",
            ),
            (
                "loop-length",
                "length_frames: 96000",
                "length_frames: 0",
                "timeline sizes",
            ),
            (
                "repetitions",
                "repetitions: 4",
                "repetitions: 0",
                "timeline sizes",
            ),
            (
                "event-outside-loop",
                "frame: 93000",
                "frame: 96000",
                "within one loop",
            ),
            (
                "timeline-overflow",
                "start_frame: 48000",
                "start_frame: 18446744073709551615",
                "frame overflow",
            ),
            (
                "timeline-too-long",
                "duration_frames: 528000",
                "duration_frames: 100000",
                "exceeds render duration",
            ),
        ];

        for (name, from, to, expected) in invalid_cases {
            let path = write_fixture_variant(name, from, to);
            let error = load_sound_fixture_file(path)
                .expect_err("invalid sound fixture contract should fail");
            assert!(error.contains(expected), "unexpected error: {error}");
        }
    }

    #[test]
    fn fixture_loader_reports_read_parse_and_patch_resolution_errors() {
        let output_dir = temp_dir("sound-fixture-load-errors");
        fs::create_dir_all(&output_dir).expect("fixture temp directory should be created");
        let missing = output_dir.join("missing.yaml");
        let malformed = output_dir.join("malformed.yaml");
        fs::write(&malformed, "version: [").expect("malformed fixture should write");
        let missing_patch = write_fixture_variant(
            "missing-patch",
            "patch: ../patches/tb303-acid.yaml",
            "patch: absent-patch.yaml",
        );

        let read_error = load_sound_fixture_file(missing).expect_err("missing fixture should fail");
        let parse_error =
            load_sound_fixture_file(malformed).expect_err("malformed fixture should fail");
        let patch_error =
            load_sound_fixture_file(missing_patch).expect_err("missing patch should fail");

        assert!(read_error.contains("failed to read sound fixture"));
        assert!(parse_error.contains("failed to parse sound fixture"));
        assert!(patch_error.contains("failed to resolve patch"));
    }

    #[test]
    fn workbench_help_and_invalid_commands_return_actionable_usage() {
        for args in [
            vec!["dandrum-sound-workbench"],
            vec!["dandrum-sound-workbench", "--help"],
            vec!["dandrum-sound-workbench", "-h"],
        ] {
            let result = run_sound_workbench(args);
            assert_eq!(result.exit_code, 0);
            assert!(result.stdout.contains("render <fixture.yaml>"));
        }

        let unknown = run_sound_workbench(["dandrum-sound-workbench", "unknown"]);
        let bad_render = run_sound_workbench(["dandrum-sound-workbench", "render"]);
        let bad_analyze = run_sound_workbench(["dandrum-sound-workbench", "analyze"]);
        let missing_render_fixture = run_sound_workbench([
            "dandrum-sound-workbench",
            "render",
            "missing.yaml",
            "--output-wav",
            "out.wav",
            "--output-metrics",
            "out.csv",
        ]);
        let missing_analyze_fixture = run_sound_workbench([
            "dandrum-sound-workbench",
            "analyze",
            "missing.yaml",
            "reference.wav",
            "--output-metrics",
            "out.csv",
        ]);

        for result in [
            unknown,
            bad_render,
            bad_analyze,
            missing_render_fixture,
            missing_analyze_fixture,
        ] {
            assert_eq!(result.exit_code, 2);
            assert!(result.stdout.is_empty());
            assert!(!result.stderr.is_empty());
        }
    }

    #[test]
    fn workbench_reports_patch_analysis_and_artifact_failures() {
        let fixture_path = acid_fixture_path();
        let mut invalid_analysis =
            load_sound_fixture_file(&fixture_path).expect("source fixture should load");
        invalid_analysis.analysis.hop_size = 0;
        let analysis_error = render_sound_fixture(&invalid_analysis)
            .expect_err("invalid fixture analysis should fail the render workflow");
        assert!(analysis_error.contains("hop size"));

        let invalid_patch_fixture = write_fixture_variant(
            "invalid-patch-fixture",
            "patch: ../patches/tb303-acid.yaml",
            "patch: invalid-patch.yaml",
        );
        let invalid_patch = invalid_patch_fixture
            .parent()
            .expect("fixture has a parent")
            .join("invalid-patch.yaml");
        fs::write(&invalid_patch, "not: a patch").expect("invalid patch should write");
        let patch_result = run_sound_workbench([
            "dandrum-sound-workbench".to_string(),
            "render".to_string(),
            invalid_patch_fixture.display().to_string(),
            "--output-wav".to_string(),
            temp_dir("unused-invalid-patch.wav").display().to_string(),
            "--output-metrics".to_string(),
            temp_dir("unused-invalid-patch.csv").display().to_string(),
        ]);
        assert_eq!(patch_result.exit_code, 2);
        assert!(
            patch_result
                .stderr
                .contains("failed to load sound fixture patch")
        );

        let output_dir = temp_dir("sound-workbench-artifact-errors");
        fs::create_dir_all(&output_dir).expect("output directory should be created");
        let missing_parent = output_dir.join("missing-parent");
        let _ = fs::remove_dir_all(&missing_parent);
        assert!(!missing_parent.exists());
        let wav_error = run_sound_workbench([
            "dandrum-sound-workbench".to_string(),
            "render".to_string(),
            fixture_path.display().to_string(),
            "--output-wav".to_string(),
            missing_parent.join("out.wav").display().to_string(),
            "--output-metrics".to_string(),
            output_dir.join("unused.csv").display().to_string(),
        ]);
        assert_eq!(wav_error.exit_code, 2);
        assert!(wav_error.stderr.contains("failed to write audition WAV"));

        let metrics_error = run_sound_workbench([
            "dandrum-sound-workbench".to_string(),
            "render".to_string(),
            fixture_path.display().to_string(),
            "--output-wav".to_string(),
            output_dir.join("valid.wav").display().to_string(),
            "--output-metrics".to_string(),
            missing_parent.join("out.csv").display().to_string(),
        ]);
        assert_eq!(metrics_error.exit_code, 2);
        assert!(
            metrics_error
                .stderr
                .contains("failed to create metrics CSV")
        );

        let reference_wav = output_dir.join("reference.wav");
        crate::wav::write_wav_file(
            &reference_wav,
            48_000,
            &vec![0.25; 2_048],
            &vec![0.25; 2_048],
        )
        .expect("reference WAV should write");
        let analyze_metrics_error = run_sound_workbench([
            "dandrum-sound-workbench".to_string(),
            "analyze".to_string(),
            fixture_path.display().to_string(),
            reference_wav.display().to_string(),
            "--output-metrics".to_string(),
            missing_parent.join("reference.csv").display().to_string(),
        ]);
        assert_eq!(analyze_metrics_error.exit_code, 2);
        assert!(
            analyze_metrics_error
                .stderr
                .contains("failed to create metrics CSV")
        );
    }
}
