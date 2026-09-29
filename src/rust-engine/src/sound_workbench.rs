use std::collections::BTreeSet;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::TimedInputEvent;
use crate::patch::RenderSettings;
use crate::sound_analysis::{AnalysisFrame, AnalysisSettings};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct SoundFixture {
    pub version: u32,
    pub name: String,
    pub patch: PathBuf,
    pub render: RenderSettings,
    pub analysis: AnalysisSettings,
    #[serde(default)]
    pub matching: Option<SoundMatchSettings>,
    pub timeline: SoundTimeline,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct SoundMatchSettings {
    pub seed: u64,
    pub max_evaluations: usize,
    pub region: SoundMatchRegion,
    pub spectral_windows: Vec<usize>,
    pub weights: SoundMatchWeights,
    pub parameters: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SoundMatchRegion {
    pub start_frame: u64,
    pub length_frames: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct SoundMatchWeights {
    pub spectral: f64,
    pub rms: f64,
    pub centroid: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct SoundTimeline {
    pub tempo_bpm: f64,
    pub steps_per_bar: u32,
    pub start_frame: u64,
    pub length_frames: u64,
    pub repetitions: u32,
    pub events: Vec<SoundEvent>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
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

pub(crate) enum SoundPatch {
    Kernel(crate::kernel::document::KernelPatch),
    #[cfg(test)]
    Legacy(crate::patch::PatchDocument),
}

pub(crate) struct PublicNumericTarget {
    pub default: f64,
    pub min: f64,
    pub max: f64,
}

impl SoundPatch {
    pub(crate) fn load(path: &Path) -> Result<Self, String> {
        crate::kernel::document::load_kernel_patch_file(path)
            .map(Self::Kernel)
            .map_err(|error| format!("failed to load sound fixture patch: {error}"))
    }

    pub(crate) fn numeric_target(&self, parameter_id: &str) -> Result<PublicNumericTarget, String> {
        let invalid = || {
            format!(
                "public numeric parameter {parameter_id} must have a finite continuous numeric default and ordered bounds"
            )
        };
        let (default, min, max) = match self {
            Self::Kernel(patch) => {
                let target = patch
                    .preset_surface()
                    .parameters()
                    .iter()
                    .find(|target| target.name() == parameter_id)
                    .ok_or_else(|| format!("unknown public numeric parameter {parameter_id}"))?;
                let control = target.control_default();
                (control.default(), control.min(), control.max())
            }
            #[cfg(test)]
            Self::Legacy(patch) => {
                let target = patch
                    .preset_surface
                    .parameters
                    .iter()
                    .find(|target| target.name == parameter_id)
                    .ok_or_else(|| format!("unknown public numeric parameter {parameter_id}"))?;
                if target.value_type != crate::patch::PresetTargetType::Number {
                    return Err(invalid());
                }
                let crate::patch::ParameterValue::Number(default) = &target.default else {
                    return Err(invalid());
                };
                (*default, target.min, target.max)
            }
        };
        let (Some(min), Some(max)) = (min, max) else {
            return Err(invalid());
        };
        if !default.is_finite() || !min.is_finite() || !max.is_finite() || min >= max {
            return Err(invalid());
        }
        Ok(PublicNumericTarget { default, min, max })
    }

    pub(crate) fn render(
        &self,
        fixture: &SoundFixture,
        patch_root: &Path,
        values: &std::collections::BTreeMap<String, f64>,
    ) -> Result<SoundRender, String> {
        match self {
            Self::Kernel(patch) => {
                render_sound_fixture_with_kernel_patch_and_public_numeric_values(
                    fixture, patch, patch_root, values,
                )
            }
            #[cfg(test)]
            Self::Legacy(patch) => render_sound_fixture_with_patch_and_public_numeric_values(
                fixture, patch, patch_root, values,
            ),
        }
    }
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
    validate_matching_declaration(&fixture)?;
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
    render_sound_fixture_with_public_numeric_values(fixture, &std::collections::BTreeMap::new())
}

pub fn render_sound_fixture_with_public_numeric_values(
    fixture: &SoundFixture,
    values: &std::collections::BTreeMap<String, f64>,
) -> Result<SoundRender, String> {
    let patch_doc = SoundPatch::load(&fixture.patch)?;
    let patch_root = fixture.patch.parent().unwrap_or_else(|| Path::new("."));
    patch_doc.render(fixture, patch_root, values)
}

fn render_sound_fixture_with_kernel_patch_and_public_numeric_values(
    fixture: &SoundFixture,
    patch: &crate::kernel::document::KernelPatch,
    patch_root: &Path,
    values: &std::collections::BTreeMap<String, f64>,
) -> Result<SoundRender, String> {
    let mut patch = patch.clone();
    if !values.is_empty() {
        let identity = patch
            .instrument()
            .cloned()
            .ok_or_else(|| "sound fixture patch has no instrument identity".to_string())?;
        for (parameter_id, value) in values {
            let target = SoundPatch::Kernel(patch.clone()).numeric_target(parameter_id)?;
            if !value.is_finite() || *value < target.min || *value > target.max {
                return Err(format!(
                    "public numeric parameter {parameter_id} value {value} is outside {}..={}",
                    target.min, target.max
                ));
            }
        }
        let preset = crate::patch::PresetDocument {
            name: "Sound Lab controls".to_string(),
            instrument: identity,
            values: values
                .iter()
                .map(|(name, value)| (name.clone(), crate::patch::ParameterValue::Number(*value)))
                .collect(),
            assets: Default::default(),
            metadata: None,
            extra_fields: Default::default(),
        };
        patch = patch
            .apply_preset(&preset)
            .map_err(|error| format!("failed to apply sound fixture values: {error}"))?;
    }
    let context =
        crate::preparation::PreparationContext::new(patch_root, fixture.render.sample_rate_hz);
    let prepared =
        crate::preparation::prepare_kernel_patch_with_context(&patch, &fixture.render, &context)
            .map_err(|error| format!("failed to prepare sound fixture patch: {error}"))?;
    let buses = crate::graph_processor::render_kernel_offline_named(
        &prepared,
        expand_timeline_events(fixture)?,
        &crate::sample::PreparedSamplerAssets::empty(),
    )
    .map_err(|error| format!("failed to render sound fixture patch: {error}"))?;
    let (_, channels) = buses
        .into_iter()
        .find(|(name, _)| name == "master")
        .ok_or_else(|| "sound fixture patch has no master output bus".to_string())?;
    let [left, right]: [Vec<f32>; 2] = channels
        .try_into()
        .map_err(|_| "sound fixture master bus must have two channels".to_string())?;
    let mono: Vec<f32> = left
        .iter()
        .zip(&right)
        .map(|(left, right)| (left + right) * 0.5)
        .collect();
    let metrics = crate::sound_analysis::analyze_sound(
        &mono,
        fixture.render.sample_rate_hz,
        fixture.analysis,
    )?;
    Ok(SoundRender {
        sample_rate_hz: fixture.render.sample_rate_hz,
        left,
        right,
        metrics,
    })
}

#[cfg(test)]
pub(crate) fn render_sound_fixture_with_patch_and_public_numeric_values(
    fixture: &SoundFixture,
    patch_doc: &crate::patch::PatchDocument,
    patch_root: &Path,
    values: &std::collections::BTreeMap<String, f64>,
) -> Result<SoundRender, String> {
    let mut patch_doc = patch_doc.clone();
    for (parameter_id, value) in values {
        let target = patch_doc
            .preset_surface
            .parameters
            .iter()
            .find(|target| target.name == *parameter_id)
            .ok_or_else(|| format!("unknown public numeric parameter {parameter_id}"))?;
        let (Some(min), Some(max)) = (target.min, target.max) else {
            return Err(format!(
                "public numeric parameter {parameter_id} does not declare matching bounds"
            ));
        };
        if !value.is_finite() || *value < min || *value > max {
            return Err(format!(
                "public numeric parameter {parameter_id} value {value} is outside {min}..={max}"
            ));
        }
        if !matches!(target.default, crate::patch::ParameterValue::Number(_)) {
            return Err(format!("public parameter {parameter_id} is not numeric"));
        }
        let destination = target.maps_to.clone();
        let module = patch_doc
            .modules
            .iter_mut()
            .find(|module| module.id == destination.module_id)
            .ok_or_else(|| {
                format!(
                    "public numeric parameter {parameter_id} targets missing module {}",
                    destination.module_id
                )
            })?;
        module.parameters.insert(
            destination.port_name,
            crate::patch::ParameterValue::Number(*value),
        );
    }
    patch_doc.render = fixture.render.clone();
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
        Some("match") => run_match_command(&args),
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

fn run_match_command(args: &[String]) -> WorkbenchResult {
    if args.len() != 7 || args[3] != "--output-wav" || args[5] != "--output-manifest" {
        return workbench_error(format!(
            "match requires a fixture, aligned PCM WAV, candidate WAV output, and manifest output\n\n{}",
            workbench_usage()
        ));
    }
    let fixture = match load_sound_fixture_file(&args[1]) {
        Ok(fixture) => fixture,
        Err(error) => return workbench_error(error),
    };
    let artifact = match crate::sound_matching::match_sound_fixture(&fixture, &args[2], |_| true) {
        Ok(artifact) => artifact,
        Err(error) => return workbench_error(error),
    };
    if let Err(error) = fs::write(&args[4], &artifact.candidate_wav_bytes) {
        return workbench_error(format!("failed to write matched candidate WAV: {error}"));
    }
    let manifest = match serde_json::to_vec_pretty(&artifact.manifest) {
        Ok(manifest) => manifest,
        Err(error) => {
            return workbench_error(format!("failed to serialize match manifest: {error}"));
        }
    };
    if let Err(error) = fs::write(&args[6], manifest) {
        return workbench_error(format!("failed to write match manifest: {error}"));
    }

    workbench_success(format!(
        "fixture: {}\nreference: {}\ncandidate: {}\nmanifest: {}\nmatch: ok\n",
        args[1], args[2], args[4], args[6]
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
    "Usage:\n  dandrum-sound-workbench render <fixture.yaml> --output-wav <output.wav> --output-metrics <metrics.csv>\n  dandrum-sound-workbench analyze <fixture.yaml> <aligned-reference.wav> --output-metrics <metrics.csv>\n  dandrum-sound-workbench match <fixture.yaml> <aligned-reference.wav> --output-wav <candidate.wav> --output-manifest <match.json>\n".to_string()
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

fn validate_matching_declaration(fixture: &SoundFixture) -> Result<(), String> {
    let Some(matching) = &fixture.matching else {
        return Ok(());
    };

    if matching.seed == 0 {
        return Err("sound matching seed must be positive".to_string());
    }
    if !fixture.analysis.min_frequency_hz.is_finite() || fixture.analysis.min_frequency_hz <= 0.0 {
        return Err(
            "sound matching log-spectral minimum frequency must be finite and positive".to_string(),
        );
    }
    crate::sound_analysis::validate_settings(fixture.render.sample_rate_hz, fixture.analysis)
        .map_err(|error| format!("invalid sound matching analysis settings: {error}"))?;
    if matching.max_evaluations == 0 {
        return Err("sound matching evaluation count must be positive".to_string());
    }
    if matching.region.length_frames == 0
        || matching
            .region
            .start_frame
            .checked_add(matching.region.length_frames)
            .is_none_or(|end| end > fixture.render.duration_frames)
    {
        return Err(
            "sound matching region must be non-empty and fall within the render".to_string(),
        );
    }
    if matching.spectral_windows.is_empty()
        || matching.spectral_windows.iter().any(|window| {
            *window < 16
                || !window.is_power_of_two()
                || u64::try_from(*window)
                    .map_or(true, |window| window > matching.region.length_frames)
        })
    {
        return Err(
            "sound matching spectral windows must be powers of two within the matching region"
                .to_string(),
        );
    }
    let weights = [
        matching.weights.spectral,
        matching.weights.rms,
        matching.weights.centroid,
    ];
    if weights
        .iter()
        .any(|weight| !weight.is_finite() || *weight < 0.0)
        || weights.iter().sum::<f64>() <= 0.0
    {
        return Err(
            "sound matching weights must be finite, non-negative, and non-zero".to_string(),
        );
    }
    if matching.parameters.is_empty() {
        return Err("sound matching must declare at least one public parameter".to_string());
    }
    let mut unique = BTreeSet::new();
    if matching
        .parameters
        .iter()
        .any(|parameter| !unique.insert(parameter))
    {
        return Err("sound matching public parameters must be unique".to_string());
    }

    let patch = SoundPatch::load(&fixture.patch)
        .map_err(|error| format!("failed to load matching patch: {error}"))?;
    for parameter_id in &matching.parameters {
        patch.numeric_target(parameter_id).map_err(|error| {
            if error.starts_with("unknown public numeric parameter") {
                format!("sound matching references unknown public numeric parameter {parameter_id}")
            } else {
                format!("sound matching {error}")
            }
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::ScriptEvent;
    use std::collections::{BTreeMap, BTreeSet};
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
        let patch_path = acid_fixture_path()
            .parent()
            .expect("fixture should have a parent")
            .join("../patches/tb303-acid.yaml")
            .canonicalize()
            .expect("TB-303 patch should resolve");
        let yaml = fs::read_to_string(acid_fixture_path()).expect("source fixture should be read");
        assert!(
            yaml.contains(replace_from),
            "fixture replacement source should exist: {replace_from}"
        );
        let yaml = yaml.replacen(replace_from, replace_to, 1).replace(
            "patch: ../patches/tb303-acid.yaml",
            &format!("patch: {}", patch_path.display()),
        );
        let path = output_dir.join(format!("{name}.yaml"));
        fs::write(&path, yaml).expect("fixture variant should write");
        path
    }

    fn write_matching_fixture_variant(name: &str, replace_from: &str, replace_to: &str) -> PathBuf {
        let output_dir = temp_dir("sound-matching-fixture-validation");
        fs::create_dir_all(&output_dir).expect("fixture temp directory should be created");
        let patch_path = acid_fixture_path()
            .parent()
            .expect("fixture should have a parent")
            .join("../patches/tb303-acid.yaml")
            .canonicalize()
            .expect("TB-303 patch should resolve");
        let yaml = fs::read_to_string(acid_fixture_path())
            .expect("source fixture should be read")
            .replacen(
                "patch: ../patches/tb303-acid.yaml",
                &format!("patch: {}", patch_path.display()),
                1,
            );
        assert!(
            yaml.contains(replace_from),
            "fixture replacement source should exist: {replace_from}"
        );
        let path = output_dir.join(format!("{name}.yaml"));
        fs::write(&path, yaml.replacen(replace_from, replace_to, 1))
            .expect("fixture variant should write");
        path
    }

    fn write_matching_patch_variant(name: &str, replace_from: &str, replace_to: &str) -> PathBuf {
        let output_dir = temp_dir("sound-matching-patch-validation");
        fs::create_dir_all(&output_dir).expect("patch temp directory should be created");
        let source_patch = acid_fixture_path()
            .parent()
            .expect("fixture should have a parent")
            .join("../patches/tb303-acid.yaml")
            .canonicalize()
            .expect("TB-303 patch should resolve");
        let patch = fs::read_to_string(&source_patch).expect("source patch should be read");
        assert!(
            patch.contains(replace_from),
            "patch replacement source should exist"
        );
        let patch_path = output_dir.join(format!("{name}-patch.yaml"));
        fs::write(&patch_path, patch.replacen(replace_from, replace_to, 1))
            .expect("patch variant should write");
        let fixture = fs::read_to_string(acid_fixture_path())
            .expect("source fixture should be read")
            .replacen(
                "patch: ../patches/tb303-acid.yaml",
                &format!("patch: {}", patch_path.display()),
                1,
            );
        let fixture_path = output_dir.join(format!("{name}.yaml"));
        fs::write(&fixture_path, fixture).expect("fixture variant should write");
        fixture_path
    }

    fn write_short_matching_fixture(name: &str) -> PathBuf {
        let output_dir = temp_dir("sound-matching-cli");
        fs::create_dir_all(&output_dir).expect("fixture temp directory should be created");
        let patch_path = acid_fixture_path()
            .parent()
            .expect("fixture should have a parent")
            .join("../patches/tb303-acid.yaml")
            .canonicalize()
            .expect("TB-303 patch should resolve");
        let yaml = fs::read_to_string(acid_fixture_path())
            .expect("source fixture should be read")
            .replace(
                "patch: ../patches/tb303-acid.yaml",
                &format!("patch: {}", patch_path.display()),
            )
            .replace("duration_frames: 528000", "duration_frames: 144000")
            .replace("max_evaluations: 16", "max_evaluations: 2")
            .replace("repetitions: 4", "repetitions: 1");
        let path = output_dir.join(format!("{name}.yaml"));
        fs::write(&path, yaml).expect("short matching fixture should write");
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
    fn acid_fixture_declares_a_bounded_public_matching_problem() {
        let fixture =
            load_sound_fixture_file(acid_fixture_path()).expect("TB-303 sound fixture should load");
        let matching = fixture
            .matching
            .as_ref()
            .expect("TB-303 fixture should declare matching settings");
        let patch = SoundPatch::load(&fixture.patch)
            .expect("TB-303 patch should expose matching parameters");

        assert_eq!(matching.seed, 303);
        assert_eq!(matching.max_evaluations, 16);
        assert_eq!(matching.region.start_frame, 48_000);
        assert_eq!(matching.region.length_frames, 96_000);
        assert_eq!(matching.spectral_windows, vec![256, 1_024, 4_096]);
        assert!(matching.weights.spectral > 0.0);
        assert!(matching.weights.rms > 0.0);
        assert!(matching.weights.centroid > 0.0);
        assert_eq!(
            matching.parameters,
            [
                "filter.cutoff",
                "filter.resonance",
                "filter.envelope_modulation",
                "filter.decay_ms",
            ]
        );

        for parameter_id in &matching.parameters {
            let target = patch
                .numeric_target(parameter_id)
                .unwrap_or_else(|error| panic!("missing public matching parameter: {error}"));
            assert!(target.min.is_finite() && target.max.is_finite() && target.min < target.max);
            assert!(target.default.is_finite());
        }
    }

    #[test]
    fn fixture_loader_rejects_invalid_matching_declarations_before_rendering() {
        let invalid_cases = [
            ("zero-seed", "seed: 303", "seed: 0", "seed"),
            (
                "zero-evaluations",
                "max_evaluations: 16",
                "max_evaluations: 0",
                "evaluation",
            ),
            (
                "region-outside-render",
                "length_frames: 96000\n  spectral_windows:",
                "length_frames: 999999\n  spectral_windows:",
                "matching region",
            ),
            (
                "invalid-window",
                "spectral_windows: [256, 1024, 4096]",
                "spectral_windows: [255]",
                "spectral window",
            ),
            ("invalid-weight", "spectral: 0.70", "spectral: -1", "weight"),
            (
                "zero-log-frequency",
                "min_frequency_hz: 30",
                "min_frequency_hz: 0",
                "log-spectral minimum frequency",
            ),
            (
                "invalid-analysis-frame",
                "frame_size: 1024",
                "frame_size: 1",
                "frame size",
            ),
            (
                "invalid-analysis-hop",
                "hop_size: 128",
                "hop_size: 0",
                "hop size",
            ),
            (
                "invalid-analysis-maximum",
                "max_frequency_hz: 20000",
                "max_frequency_hz: 30000",
                "frequency band",
            ),
            (
                "invalid-analysis-silence",
                "silence_rms: 0.00001",
                "silence_rms: -1",
                "silence RMS",
            ),
            (
                "unknown-parameter",
                "- filter.cutoff",
                "- filter.absent",
                "unknown public numeric parameter",
            ),
            (
                "empty-parameters",
                "parameters:\n    - filter.cutoff\n    - filter.resonance\n    - filter.envelope_modulation\n    - filter.decay_ms",
                "parameters: []",
                "at least one public parameter",
            ),
            (
                "duplicate-parameter",
                "- filter.resonance",
                "- filter.cutoff",
                "must be unique",
            ),
        ];

        for (name, from, to, expected) in invalid_cases {
            let path = write_matching_fixture_variant(name, from, to);
            let error = load_sound_fixture_file(path)
                .expect_err("invalid matching declaration should fail before rendering");
            assert!(error.contains(expected), "unexpected error: {error}");
        }

        let patch_cases = [(
            "unbounded-parameter",
            "default: 0.4, min: 0.02, max: 0.9",
            "default: 0.4, min: 0.02",
            "ordered bounds",
        )];
        for (name, from, to, expected) in patch_cases {
            let path = write_matching_patch_variant(name, from, to);
            let error = load_sound_fixture_file(path)
                .expect_err("invalid patch matching declaration should fail before rendering");
            assert!(error.contains(expected), "unexpected error: {error}");
        }
    }

    #[test]
    fn fixture_without_matching_declaration_remains_valid_for_render_only_workflows() {
        let mut fixture =
            load_sound_fixture_file(acid_fixture_path()).expect("TB-303 fixture should load");
        fixture.matching = None;

        assert_eq!(validate_matching_declaration(&fixture), Ok(()));
    }

    #[test]
    fn matching_declaration_validates_window_and_weight_boundaries_independently() {
        let mut fixture =
            load_sound_fixture_file(acid_fixture_path()).expect("TB-303 fixture should load");
        let matching = fixture.matching.as_mut().unwrap();

        matching.spectral_windows = vec![16];
        assert_eq!(validate_matching_declaration(&fixture), Ok(()));
        fixture.matching.as_mut().unwrap().spectral_windows = vec![8];
        assert!(validate_matching_declaration(&fixture).is_err());

        let matching = fixture.matching.as_mut().unwrap();
        matching.region.length_frames = 65_536;
        matching.spectral_windows = vec![65_536];
        assert_eq!(validate_matching_declaration(&fixture), Ok(()));
        fixture.matching.as_mut().unwrap().spectral_windows = vec![131_072];
        assert!(validate_matching_declaration(&fixture).is_err());

        let matching = fixture.matching.as_mut().unwrap();
        matching.spectral_windows = vec![16];
        matching.weights = SoundMatchWeights {
            spectral: 0.0,
            rms: 0.5,
            centroid: 0.5,
        };
        assert_eq!(validate_matching_declaration(&fixture), Ok(()));
        for weights in [
            SoundMatchWeights {
                spectral: -0.1,
                rms: 0.2,
                centroid: 0.1,
            },
            SoundMatchWeights {
                spectral: f64::NAN,
                rms: 0.5,
                centroid: 0.5,
            },
            SoundMatchWeights {
                spectral: 0.0,
                rms: 0.0,
                centroid: 0.0,
            },
        ] {
            fixture.matching.as_mut().unwrap().weights = weights;
            assert!(validate_matching_declaration(&fixture).is_err());
        }
    }

    #[test]
    fn snapshot_renderer_rejects_invalid_public_parameter_applications_before_rendering() {
        let fixture =
            load_sound_fixture_file(acid_fixture_path()).expect("TB-303 fixture should load");
        let patch = SoundPatch::load(&fixture.patch).expect("patch should load");
        let patch_root = fixture.patch.parent().unwrap();
        let render_result = |patch: &SoundPatch, id: &str, value: f64| {
            patch.render(
                &fixture,
                patch_root,
                &std::collections::BTreeMap::from([(id.to_string(), value)]),
            )
        };

        assert!(
            render_result(&patch, "filter.absent", 0.5)
                .unwrap_err()
                .contains("unknown")
        );
        for value in [f64::NAN, 0.01, 0.91] {
            assert!(
                render_result(&patch, "filter.cutoff", value)
                    .unwrap_err()
                    .contains("outside")
            );
        }
        assert!(render_result(&patch, "filter.cutoff", 0.02).is_ok());
        assert!(render_result(&patch, "filter.cutoff", 0.9).is_ok());

        let yaml = fs::read_to_string(&fixture.patch).expect("acid YAML should read");
        let unbounded = yaml.replacen(
            "default: 0.4, min: 0.02, max: 0.9",
            "default: 0.4, min: 0.02",
            1,
        );
        let unbounded = SoundPatch::Kernel(
            crate::kernel::document::load_kernel_patch_str(&unbounded)
                .expect("unbounded control patch should parse"),
        );
        assert!(
            render_result(&unbounded, "filter.cutoff", 0.5)
                .unwrap_err()
                .contains("ordered bounds")
        );

        assert!(yaml.contains("maps_to: accent_bright.offset"));
        let missing_module =
            yaml.replacen("maps_to: accent_bright.offset", "maps_to: absent.offset", 1);
        let missing_module = SoundPatch::Kernel(
            crate::kernel::document::load_kernel_patch_str(&missing_module)
                .expect("kernel document syntax is valid"),
        );
        assert!(render_result(&missing_module, "filter.cutoff", 0.5).is_err());
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
        let mono = first
            .left
            .iter()
            .zip(&first.right)
            .map(|(left, right)| (left + right) * 0.5)
            .collect::<Vec<_>>();
        assert_eq!(
            first.metrics,
            crate::sound_analysis::analyze_sound(&mono, first.sample_rate_hz, fixture.analysis,)
                .unwrap()
        );
    }

    #[test]
    fn acid_kernel_variant_loads_as_a_kernel_document() {
        let fixture =
            load_sound_fixture_file(acid_fixture_path()).expect("TB-303 sound fixture should load");
        let kernel = crate::kernel::document::load_kernel_patch_file(&fixture.patch)
            .expect("TB-303 patch should use the kernel document shape");
        let legacy = crate::patch::load_patch_str(include_str!(
            "../tests/fixtures/unify-graph-kernel/legacy/tb303-acid.yaml"
        ))
        .expect("pre-migration acid patch should parse");
        let kernel = SoundPatch::Kernel(kernel);
        let legacy = SoundPatch::Legacy(legacy);
        for name in [
            "filter.cutoff",
            "filter.resonance",
            "filter.envelope_modulation",
            "filter.decay_ms",
            "accent.brightness",
            "amp.release_ms",
            "slide.time_ms",
        ] {
            let expected = legacy.numeric_target(name).expect("legacy control exists");
            let actual = kernel.numeric_target(name).expect("kernel control exists");
            assert_eq!(actual.default, expected.default, "{name} default changed");
            assert_eq!(actual.min, expected.min, "{name} minimum changed");
            assert_eq!(actual.max, expected.max, "{name} maximum changed");
        }
    }

    #[test]
    fn sound_fixture_loader_rejects_archived_legacy_patch_documents() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/unify-graph-kernel/legacy/tb303-acid.yaml");
        assert!(SoundPatch::load(&path).is_err());
    }

    #[test]
    fn acid_kernel_render_preserves_legacy_calibration() {
        let fixture =
            load_sound_fixture_file(acid_fixture_path()).expect("TB-303 sound fixture should load");
        let legacy = crate::patch::load_patch_str(include_str!(
            "../tests/fixtures/unify-graph-kernel/legacy/tb303-acid.yaml"
        ))
        .expect("pre-migration acid patch should parse");
        let patch_root = fixture.patch.parent().expect("acid patch has a directory");

        for values in [
            BTreeMap::new(),
            BTreeMap::from([("filter.cutoff".to_string(), 0.7)]),
        ] {
            let expected = render_sound_fixture_with_patch_and_public_numeric_values(
                &fixture, &legacy, patch_root, &values,
            )
            .expect("legacy calibration should render");
            let actual = render_sound_fixture_with_public_numeric_values(&fixture, &values)
                .expect("kernel acid patch should render");
            if values.is_empty() {
                assert!(actual.left[49_000] < -0.04);
            }
            for (name, actual, expected) in [
                ("left", &actual.left, &expected.left),
                ("right", &actual.right, &expected.right),
            ] {
                assert_eq!(actual.len(), expected.len());
                let max_difference = actual
                    .iter()
                    .zip(expected)
                    .map(|(actual, expected)| (actual - expected).abs())
                    .fold(0.0_f32, f32::max);
                let rms_difference = (actual
                    .iter()
                    .zip(expected)
                    .map(|(actual, expected)| f64::from(actual - expected).powi(2))
                    .sum::<f64>()
                    / actual.len() as f64)
                    .sqrt();
                // The legacy offline renderer skips inactive voice processing;
                // the kernel root runs continuously, changing only onset transients.
                assert!(
                    max_difference < 0.01,
                    "{name} peak difference {max_difference} for {values:?}"
                );
                assert!(
                    rms_difference < 0.0001,
                    "{name} RMS difference {rms_difference} for {values:?}"
                );
            }
        }
    }

    #[test]
    fn synthetic_kick_sound_fixture_renders_kernel_patch_and_public_decay_values() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/sound-design/synthetic-808-kick-poc.yaml");
        let fixture = load_sound_fixture_file(path).expect("synthetic kick fixture loads");
        let default = render_sound_fixture(&fixture).expect("kernel kick fixture renders");
        assert_eq!(default.left.len(), fixture.render.duration_frames as usize);
        assert!(default.left.iter().any(|sample| sample.abs() > 0.01));
        assert_eq!(default.left, default.right);
        let short = render_sound_fixture_with_public_numeric_values(
            &fixture,
            &std::collections::BTreeMap::from([("kick.decay_ms".to_string(), 250.0)]),
        )
        .expect("short decay renders");
        let long = render_sound_fixture_with_public_numeric_values(
            &fixture,
            &std::collections::BTreeMap::from([("kick.decay_ms".to_string(), 1_400.0)]),
        )
        .expect("long decay renders");
        assert_ne!(short.left, long.left);
    }

    #[test]
    fn synthetic_kick_sound_matching_uses_kernel_public_controls() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/sound-design/synthetic-808-kick-poc.yaml");
        let fixture = load_sound_fixture_file(path).expect("synthetic kick fixture loads");
        let reference = render_sound_fixture(&fixture).expect("reference renders");
        let wav = tempfile::Builder::new()
            .suffix(".wav")
            .tempfile()
            .expect("temporary WAV opens");
        crate::wav::write_wav_file(
            wav.path(),
            reference.sample_rate_hz,
            &reference.left,
            &reference.right,
        )
        .expect("reference WAV writes");
        let artifact = crate::sound_matching::match_sound_fixture(&fixture, wav.path(), |_| true)
            .expect("kernel patch matches against reference audio");
        assert_eq!(artifact.manifest.best_parameters.len(), 2);
        assert_eq!(artifact.manifest.best_parameters[0].id, "kick.tune_hz");
        assert_eq!(artifact.manifest.best_parameters[1].id, "kick.decay_ms");
        assert!(!artifact.candidate_wav_bytes.is_empty());
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
    fn workbench_match_writes_candidate_wav_and_reproducible_manifest() {
        let output_dir = temp_dir("sound-workbench-match");
        fs::create_dir_all(&output_dir).expect("output directory should be created");
        let fixture_path = write_short_matching_fixture("cli-match");
        let fixture = load_sound_fixture_file(&fixture_path).expect("short fixture should load");
        let reference = render_sound_fixture(&fixture).expect("reference should render");
        let reference_wav = output_dir.join("reference.wav");
        let candidate_wav = output_dir.join("candidate.wav");
        let manifest_path = output_dir.join("match.json");
        crate::wav::write_wav_file(
            &reference_wav,
            reference.sample_rate_hz,
            &reference.left,
            &reference.right,
        )
        .expect("reference WAV should write");

        let result = run_sound_workbench([
            "dandrum-sound-workbench".to_string(),
            "match".to_string(),
            fixture_path.display().to_string(),
            reference_wav.display().to_string(),
            "--output-wav".to_string(),
            candidate_wav.display().to_string(),
            "--output-manifest".to_string(),
            manifest_path.display().to_string(),
        ]);

        assert_eq!(result.exit_code, 0, "{}", result.stderr);
        assert!(result.stdout.contains("match: ok"));
        assert!(fs::read(candidate_wav).unwrap().starts_with(b"RIFF"));
        let manifest: crate::sound_matching::SoundMatchManifest =
            serde_json::from_slice(&fs::read(manifest_path).expect("manifest should exist"))
                .expect("manifest should be valid JSON");
        assert_eq!(manifest.fixture_name, fixture.name);
        assert_eq!(manifest.seed, 303);
        assert_eq!(manifest.completed_evaluations, 2);
        assert_eq!(manifest.history.len(), 2);
        assert_eq!(manifest.reference_sha256.len(), 64);
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
                "  length_frames: 96000\n  repetitions: 4",
                "  length_frames: 0\n  repetitions: 4",
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
                "timeline:\n  tempo_bpm: 120\n  steps_per_bar: 16\n  start_frame: 48000",
                "timeline:\n  tempo_bpm: 120\n  steps_per_bar: 16\n  start_frame: 18446744073709551615",
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
        let bad_match = run_sound_workbench(["dandrum-sound-workbench", "match"]);
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
            bad_match,
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
                .contains("failed to load matching patch")
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
