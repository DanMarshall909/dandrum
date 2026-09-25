use std::path::Path;

use rustfft::FftPlanner;
use rustfft::num_complex::Complex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::sound_analysis::AnalysisSettings;
use crate::sound_workbench::SoundMatchWeights;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoundMatchObjectiveSettings<'a> {
    pub sample_rate_hz: u32,
    pub spectral_windows: &'a [usize],
    pub analysis: AnalysisSettings,
    pub weights: SoundMatchWeights,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SoundMatchScore {
    pub spectral: f64,
    pub rms: f64,
    pub centroid: f64,
    pub total: f64,
    pub candidate_gain: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundedSearchParameter {
    pub id: String,
    pub min: f64,
    pub max: f64,
    pub initial: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundedSearchProgress {
    pub completed_evaluations: usize,
    pub max_evaluations: usize,
    pub best_total: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundedSearchEvaluation {
    pub values: Vec<f64>,
    pub score: SoundMatchScore,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundedSearchResult {
    pub cancelled: bool,
    pub best: BoundedSearchEvaluation,
    pub history: Vec<BoundedSearchEvaluation>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SoundMatchStatus {
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SoundMatchParameterResult {
    pub id: String,
    pub min: f64,
    pub max: f64,
    pub initial: f64,
    pub best: f64,
    pub normalized: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SoundMatchEvaluationRecord {
    pub index: usize,
    pub values: Vec<f64>,
    pub score: SoundMatchScore,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SoundMatchManifest {
    pub version: u32,
    pub fixture_name: String,
    pub fixture_sha256: String,
    pub patch_sha256: String,
    pub reference_name: String,
    pub reference_sha256: String,
    pub optimizer: String,
    pub seed: u64,
    pub max_evaluations: usize,
    pub completed_evaluations: usize,
    pub status: SoundMatchStatus,
    pub sample_rate_hz: u32,
    pub block_size_frames: u32,
    pub render_duration_frames: u64,
    pub analysis: AnalysisSettings,
    pub region_start_frame: u64,
    pub region_length_frames: u64,
    pub spectral_windows: Vec<usize>,
    pub weights: crate::sound_workbench::SoundMatchWeights,
    pub best_score: SoundMatchScore,
    pub best_parameters: Vec<SoundMatchParameterResult>,
    pub history: Vec<SoundMatchEvaluationRecord>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SoundMatchArtifact {
    pub manifest: SoundMatchManifest,
    pub patch_yaml: String,
    pub sample_rate_hz: u32,
    pub duration_frames: u64,
    pub candidate_metrics: Vec<crate::sound_analysis::AnalysisFrame>,
    pub reference_metrics: Vec<crate::sound_analysis::AnalysisFrame>,
    pub candidate_wav_bytes: Vec<u8>,
    pub reference_wav_bytes: Vec<u8>,
}

pub fn match_sound_fixture<P>(
    fixture: &crate::sound_workbench::SoundFixture,
    reference_path: impl AsRef<Path>,
    on_progress: P,
) -> Result<SoundMatchArtifact, String>
where
    P: FnMut(BoundedSearchProgress) -> bool,
{
    let matching = fixture
        .matching
        .as_ref()
        .ok_or_else(|| "sound fixture does not declare matching settings".to_string())?;
    let reference_path = reference_path.as_ref();
    let reference_wav_bytes = std::fs::read(reference_path)
        .map_err(|error| format!("failed to read reference WAV: {error}"))?;
    let reference_audio =
        crate::audio_loading::decode_pcm_wav(&reference_wav_bytes, fixture.render.sample_rate_hz)?;
    let region_start = usize::try_from(matching.region.start_frame)
        .map_err(|_| "sound matching region start does not fit this platform".to_string())?;
    let region_length = usize::try_from(matching.region.length_frames)
        .map_err(|_| "sound matching region length does not fit this platform".to_string())?;
    let region_end = region_start
        .checked_add(region_length)
        .ok_or_else(|| "sound matching region overflow".to_string())?;
    if reference_audio.frames().len() < region_end {
        return Err(format!(
            "reference WAV is shorter than the matching region ending at frame {region_end}"
        ));
    }
    let reference_region = &reference_audio.frames()[region_start..region_end];
    let reference_energy = reference_region
        .iter()
        .map(|sample| f64::from(*sample).powi(2))
        .sum::<f64>();
    if reference_energy <= fixture.analysis.silence_rms.powi(2) * reference_region.len() as f64 {
        return Err("sound matching requires a non-silent reference region".to_string());
    }

    let patch_bytes = std::fs::read(&fixture.patch)
        .map_err(|error| format!("failed to load sound matching patch: {error}"))?;
    let patch_yaml = std::str::from_utf8(&patch_bytes)
        .map_err(|error| format!("failed to load sound matching patch as UTF-8: {error}"))?;
    let patch = crate::patch::load_patch_str(patch_yaml)
        .map_err(|error| format!("failed to load sound matching patch: {error}"))?;
    let patch_root = fixture.patch.parent().unwrap_or_else(|| Path::new("."));
    let mut parameters = Vec::with_capacity(matching.parameters.len());
    for parameter_id in &matching.parameters {
        let target = patch
            .preset_surface
            .parameters
            .iter()
            .find(|target| target.name == *parameter_id)
            .ok_or_else(|| format!("unknown public numeric parameter {parameter_id}"))?;
        let (
            crate::patch::PresetTargetType::Number,
            crate::patch::ParameterValue::Number(initial),
            Some(min),
            Some(max),
        ) = (&target.value_type, &target.default, target.min, target.max)
        else {
            return Err(format!(
                "public matching parameter {parameter_id} must have a finite continuous numeric default and ordered bounds"
            ));
        };
        if !initial.is_finite() || !min.is_finite() || !max.is_finite() || min >= max {
            return Err(format!(
                "public matching parameter {parameter_id} must have a finite continuous numeric default and ordered bounds"
            ));
        }
        parameters.push(BoundedSearchParameter {
            id: parameter_id.clone(),
            min,
            max,
            initial: *initial,
        });
    }

    let objective_settings = SoundMatchObjectiveSettings {
        sample_rate_hz: fixture.render.sample_rate_hz,
        spectral_windows: &matching.spectral_windows,
        analysis: fixture.analysis,
        weights: matching.weights,
    };
    let search = deterministic_bounded_search(
        &parameters,
        matching.seed,
        matching.max_evaluations,
        |values| {
            let values = parameters
                .iter()
                .zip(values)
                .map(|(parameter, value)| (parameter.id.clone(), *value))
                .collect();
            let render =
                crate::sound_workbench::render_sound_fixture_with_patch_and_public_numeric_values(
                    fixture, &patch, patch_root, &values,
                )?;
            let mono = mono_audio(&render.left, &render.right);
            compare_aligned_audio(
                &mono[region_start..region_end],
                reference_region,
                objective_settings,
            )
        },
        on_progress,
    )?;

    let best_values = parameters
        .iter()
        .zip(&search.best.values)
        .map(|(parameter, value)| (parameter.id.clone(), *value))
        .collect();
    let mut candidate =
        crate::sound_workbench::render_sound_fixture_with_patch_and_public_numeric_values(
            fixture,
            &patch,
            patch_root,
            &best_values,
        )
        .expect("the best candidate was rendered successfully during the search");
    for sample in candidate.left.iter_mut().chain(&mut candidate.right) {
        *sample = (f64::from(*sample) * search.best.score.candidate_gain) as f32;
    }
    let candidate_mono = mono_audio(&candidate.left, &candidate.right);
    let candidate_metrics = crate::sound_analysis::analyze_sound(
        &candidate_mono[region_start..region_end],
        fixture.render.sample_rate_hz,
        fixture.analysis,
    )
    .expect("objective validation already accepted candidate analysis settings");
    let reference_metrics = crate::sound_analysis::analyze_sound(
        reference_region,
        fixture.render.sample_rate_hz,
        fixture.analysis,
    )
    .expect("objective validation already accepted reference analysis settings");
    let mut candidate_wav_bytes = Vec::new();
    crate::wav::write_wav_stereo_i16(
        &mut candidate_wav_bytes,
        candidate.sample_rate_hz,
        &candidate.left,
        &candidate.right,
    )
    .expect("writing a WAV to an in-memory byte vector cannot fail");

    let reference_sha256 = sha256_hex(&reference_wav_bytes);
    let patch_sha256 = sha256_hex(&patch_bytes);
    let fixture_identity = serde_json::to_vec(&SoundMatchFixtureIdentity {
        version: fixture.version,
        name: &fixture.name,
        patch_sha256: &patch_sha256,
        render: &fixture.render,
        analysis: fixture.analysis,
        matching,
        timeline: &fixture.timeline,
    })
    .expect("sound fixture identity contains only serializable values");
    let fixture_sha256 = sha256_hex(&fixture_identity);
    let best_parameters = parameters
        .iter()
        .zip(&search.best.values)
        .map(|(parameter, best)| SoundMatchParameterResult {
            id: parameter.id.clone(),
            min: parameter.min,
            max: parameter.max,
            initial: parameter.initial,
            best: *best,
            normalized: (*best - parameter.min) / (parameter.max - parameter.min),
        })
        .collect();
    let history = search
        .history
        .iter()
        .enumerate()
        .map(|(index, evaluation)| SoundMatchEvaluationRecord {
            index: index + 1,
            values: evaluation.values.clone(),
            score: evaluation.score,
        })
        .collect::<Vec<_>>();
    let completed_evaluations = history.len();
    let status = if search.cancelled {
        SoundMatchStatus::Cancelled
    } else {
        SoundMatchStatus::Completed
    };
    let reference_name = reference_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .expect("a successfully loaded reference file has a file name");

    Ok(SoundMatchArtifact {
        manifest: SoundMatchManifest {
            version: 2,
            fixture_name: fixture.name.clone(),
            fixture_sha256,
            patch_sha256,
            reference_name,
            reference_sha256,
            optimizer: "dandrum.seeded_evolution.v1".to_string(),
            seed: matching.seed,
            max_evaluations: matching.max_evaluations,
            completed_evaluations,
            status,
            sample_rate_hz: fixture.render.sample_rate_hz,
            block_size_frames: fixture.render.block_size_frames,
            render_duration_frames: fixture.render.duration_frames,
            analysis: fixture.analysis,
            region_start_frame: matching.region.start_frame,
            region_length_frames: matching.region.length_frames,
            spectral_windows: matching.spectral_windows.clone(),
            weights: matching.weights,
            best_score: search.best.score,
            best_parameters,
            history,
        },
        patch_yaml: patch_yaml.to_string(),
        sample_rate_hz: candidate.sample_rate_hz,
        duration_frames: candidate.left.len() as u64,
        candidate_metrics,
        reference_metrics,
        candidate_wav_bytes,
        reference_wav_bytes,
    })
}

#[derive(Serialize)]
struct SoundMatchFixtureIdentity<'a> {
    version: u32,
    name: &'a str,
    patch_sha256: &'a str,
    render: &'a crate::patch::RenderSettings,
    analysis: AnalysisSettings,
    matching: &'a crate::sound_workbench::SoundMatchSettings,
    timeline: &'a crate::sound_workbench::SoundTimeline,
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn mono_audio(left: &[f32], right: &[f32]) -> Vec<f32> {
    left.iter()
        .zip(right)
        .map(|(left, right)| (left + right) * 0.5)
        .collect()
}

pub fn deterministic_bounded_search<E, P>(
    parameters: &[BoundedSearchParameter],
    seed: u64,
    max_evaluations: usize,
    mut evaluate: E,
    mut on_progress: P,
) -> Result<BoundedSearchResult, String>
where
    E: FnMut(&[f64]) -> Result<SoundMatchScore, String>,
    P: FnMut(BoundedSearchProgress) -> bool,
{
    validate_search_contract(parameters, seed, max_evaluations)?;

    let initial: Vec<f64> = parameters
        .iter()
        .map(|parameter| parameter.initial)
        .collect();
    let initial_score = evaluate(&initial)?;
    validate_evaluated_score(initial_score)?;
    let initial_evaluation = BoundedSearchEvaluation {
        values: initial,
        score: initial_score,
    };
    let mut history = vec![initial_evaluation.clone()];
    let mut best = initial_evaluation;
    let mut cancelled = !on_progress(BoundedSearchProgress {
        completed_evaluations: 1,
        max_evaluations,
        best_total: best.score.total,
    });

    let cycle_length = parameters.len() * 2 + 2;
    let mut radius = 0.4_f64;
    let mut random = SplitMix64::new(seed);
    while history.len() < max_evaluations && !cancelled {
        let phase = (history.len() - 1) % cycle_length;
        let mut normalized: Vec<f64> = best
            .values
            .iter()
            .zip(parameters)
            .map(|(value, parameter)| (value - parameter.min) / (parameter.max - parameter.min))
            .collect();
        if phase < parameters.len() * 2 {
            let parameter = phase / 2;
            let direction = if phase % 2 == 0 { 1.0 } else { -1.0 };
            normalized[parameter] = (normalized[parameter] + direction * radius).clamp(0.0, 1.0);
        } else {
            for value in &mut normalized {
                let offset = (random.next_unit() * 2.0 - 1.0) * radius;
                *value = (*value + offset).clamp(0.0, 1.0);
            }
        }
        let values: Vec<f64> = normalized
            .iter()
            .zip(parameters)
            .map(|(normalized, parameter)| {
                parameter.min + normalized * (parameter.max - parameter.min)
            })
            .collect();
        let score = evaluate(&values)?;
        validate_evaluated_score(score)?;
        let evaluation = BoundedSearchEvaluation { values, score };
        if evaluation.score.total < best.score.total {
            best = evaluation.clone();
        }
        history.push(evaluation);
        cancelled = !on_progress(BoundedSearchProgress {
            completed_evaluations: history.len(),
            max_evaluations,
            best_total: best.score.total,
        });
        if phase + 1 == cycle_length {
            radius = (radius * 0.65).max(0.01);
        }
    }

    Ok(BoundedSearchResult {
        cancelled,
        best,
        history,
    })
}

fn validate_search_contract(
    parameters: &[BoundedSearchParameter],
    seed: u64,
    max_evaluations: usize,
) -> Result<(), String> {
    if seed == 0 {
        return Err("bounded search seed must be positive".to_string());
    }
    if max_evaluations == 0 {
        return Err("bounded search evaluation count must be positive".to_string());
    }
    if parameters.is_empty() {
        return Err("bounded search requires at least one parameter".to_string());
    }
    let mut ids = std::collections::BTreeSet::new();
    for parameter in parameters {
        if parameter.id.trim().is_empty()
            || !ids.insert(&parameter.id)
            || !parameter.min.is_finite()
            || !parameter.max.is_finite()
            || !parameter.initial.is_finite()
            || parameter.min >= parameter.max
            || parameter.initial < parameter.min
            || parameter.initial > parameter.max
        {
            return Err(format!(
                "bounded search parameter {} must have a unique id, ordered finite bounds, and an in-range initial value",
                parameter.id
            ));
        }
    }
    Ok(())
}

fn validate_evaluated_score(score: SoundMatchScore) -> Result<(), String> {
    if [
        score.spectral,
        score.rms,
        score.centroid,
        score.total,
        score.candidate_gain,
    ]
    .iter()
    .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err(
            "bounded search objective returned an invalid non-finite or negative score".to_string(),
        );
    }
    Ok(())
}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_unit(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^= value >> 31;
        value as f64 / u64::MAX as f64
    }
}

pub fn compare_aligned_audio(
    candidate: &[f32],
    reference: &[f32],
    settings: SoundMatchObjectiveSettings,
) -> Result<SoundMatchScore, String> {
    validate_objective_input(candidate, reference, settings)?;

    let reference_energy = reference
        .iter()
        .map(|sample| f64::from(*sample).powi(2))
        .sum::<f64>();
    let silence_energy = settings.analysis.silence_rms.powi(2) * reference.len() as f64;
    if reference_energy <= silence_energy {
        return Err("sound matching requires a non-silent reference region".to_string());
    }

    let candidate_energy = candidate
        .iter()
        .map(|sample| f64::from(*sample).powi(2))
        .sum::<f64>();
    let correlation = candidate
        .iter()
        .zip(reference)
        .map(|(candidate, reference)| f64::from(*candidate) * f64::from(*reference))
        .sum::<f64>();
    let candidate_gain = if candidate_energy > f64::EPSILON {
        (correlation / candidate_energy).max(0.0)
    } else {
        1.0
    };
    let gain_aligned: Vec<f32> = candidate
        .iter()
        .map(|sample| (f64::from(*sample) * candidate_gain) as f32)
        .collect();
    let spectral = multi_resolution_spectral_loss(
        &gain_aligned,
        reference,
        settings.sample_rate_hz,
        settings.spectral_windows,
        settings.analysis.min_frequency_hz,
        settings.analysis.max_frequency_hz,
    );
    let candidate_frames = crate::sound_analysis::analyze_sound(
        &gain_aligned,
        settings.sample_rate_hz,
        settings.analysis,
    )?;
    let reference_frames =
        crate::sound_analysis::analyze_sound(reference, settings.sample_rate_hz, settings.analysis)
            .expect("candidate and reference share validated analysis settings and aligned length");
    let (rms, centroid) = trajectory_losses(
        &candidate_frames,
        &reference_frames,
        settings.analysis.silence_rms,
    );
    let total = spectral * settings.weights.spectral
        + rms * settings.weights.rms
        + centroid * settings.weights.centroid;

    Ok(SoundMatchScore {
        spectral,
        rms,
        centroid,
        total,
        candidate_gain,
    })
}

fn validate_objective_input(
    candidate: &[f32],
    reference: &[f32],
    settings: SoundMatchObjectiveSettings<'_>,
) -> Result<(), String> {
    if settings.sample_rate_hz == 0 {
        return Err("sound matching sample rate must be positive".to_string());
    }
    let analysis = settings.analysis;
    let nyquist_hz = f64::from(settings.sample_rate_hz) / 2.0;
    if !analysis.min_frequency_hz.is_finite()
        || !analysis.max_frequency_hz.is_finite()
        || analysis.min_frequency_hz <= 0.0
        || analysis.max_frequency_hz <= analysis.min_frequency_hz
        || analysis.max_frequency_hz > nyquist_hz
    {
        return Err(format!(
            "sound matching log-spectral minimum frequency and band must satisfy 0 < min < max <= {nyquist_hz} Hz"
        ));
    }
    crate::sound_analysis::validate_settings(settings.sample_rate_hz, analysis)
        .map_err(|error| format!("invalid sound matching analysis settings: {error}"))?;
    if candidate.len() != reference.len() || candidate.is_empty() {
        return Err(
            "sound matching requires candidate and reference to have the same aligned length"
                .to_string(),
        );
    }
    if candidate
        .iter()
        .chain(reference)
        .any(|sample| !sample.is_finite())
    {
        return Err("sound matching requires finite candidate and reference audio".to_string());
    }
    if settings.spectral_windows.is_empty()
        || settings
            .spectral_windows
            .iter()
            .any(|window| *window < 16 || !window.is_power_of_two() || *window > candidate.len())
    {
        return Err(
            "sound matching spectral windows must be powers of two within the aligned region"
                .to_string(),
        );
    }
    let weights = [
        settings.weights.spectral,
        settings.weights.rms,
        settings.weights.centroid,
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
    Ok(())
}

fn multi_resolution_spectral_loss(
    candidate: &[f32],
    reference: &[f32],
    sample_rate_hz: u32,
    windows: &[usize],
    min_frequency_hz: f64,
    max_frequency_hz: f64,
) -> f64 {
    const BAND_COUNT: usize = 24;
    const POWER_FLOOR: f64 = 1.0e-12;

    let mut planner = FftPlanner::new();
    let mut squared_error = 0.0;
    let mut compared = 0usize;
    let log_min = min_frequency_hz.ln();
    let log_range = max_frequency_hz.ln() - log_min;

    for &window_size in windows {
        let fft = planner.plan_fft_forward(window_size);
        let hop_size = window_size / 4;
        let hann: Vec<f32> = (0..window_size)
            .map(|index| {
                (0.5 * (1.0 - (std::f64::consts::TAU * index as f64 / window_size as f64).cos()))
                    as f32
            })
            .collect();
        let mut candidate_fft = vec![Complex::new(0.0_f32, 0.0); window_size];
        let mut reference_fft = vec![Complex::new(0.0_f32, 0.0); window_size];

        for start in (0..=candidate.len() - window_size).step_by(hop_size) {
            for index in 0..window_size {
                candidate_fft[index].re = candidate[start + index] * hann[index];
                candidate_fft[index].im = 0.0;
                reference_fft[index].re = reference[start + index] * hann[index];
                reference_fft[index].im = 0.0;
            }
            fft.process(&mut candidate_fft);
            fft.process(&mut reference_fft);

            let mut candidate_bands = [0.0_f64; BAND_COUNT];
            let mut reference_bands = [0.0_f64; BAND_COUNT];
            let mut band_bins = [0_u32; BAND_COUNT];
            for bin in 1..=window_size / 2 {
                let frequency_hz = bin as f64 * f64::from(sample_rate_hz) / window_size as f64;
                if frequency_hz < min_frequency_hz || frequency_hz > max_frequency_hz {
                    continue;
                }
                let position = ((frequency_hz.ln() - log_min) / log_range).clamp(0.0, 1.0);
                let band = ((position * BAND_COUNT as f64) as usize).min(BAND_COUNT - 1);
                candidate_bands[band] += f64::from(candidate_fft[bin].norm_sqr());
                reference_bands[band] += f64::from(reference_fft[bin].norm_sqr());
                band_bins[band] += 1;
            }

            for band in 0..BAND_COUNT {
                if band_bins[band] == 0 {
                    continue;
                }
                let divisor = f64::from(band_bins[band]);
                let candidate_log = (candidate_bands[band] / divisor + POWER_FLOOR).ln();
                let reference_log = (reference_bands[band] / divisor + POWER_FLOOR).ln();
                squared_error += (candidate_log - reference_log).powi(2);
                compared += 1;
            }
        }
    }

    if compared == 0 {
        0.0
    } else {
        squared_error / compared as f64
    }
}

fn trajectory_losses(
    candidate: &[crate::sound_analysis::AnalysisFrame],
    reference: &[crate::sound_analysis::AnalysisFrame],
    silence_rms: f64,
) -> (f64, f64) {
    let mut rms_squared_error = 0.0;
    let mut centroid_squared_error = 0.0;
    let mut compared = 0usize;

    for (candidate, reference) in candidate.iter().zip(reference) {
        let candidate_db = 20.0 * candidate.rms.max(silence_rms.max(1.0e-12)).log10();
        let reference_db = 20.0 * reference.rms.max(silence_rms.max(1.0e-12)).log10();
        rms_squared_error += ((candidate_db - reference_db) / 20.0).powi(2);
        centroid_squared_error += match (
            candidate.spectral_centroid_hz,
            reference.spectral_centroid_hz,
        ) {
            (Some(candidate), Some(reference)) if candidate > 0.0 && reference > 0.0 => {
                (candidate / reference).log2().powi(2)
            }
            (None, None) => 0.0,
            _ => 1.0,
        };
        compared += 1;
    }

    if compared == 0 {
        (0.0, 0.0)
    } else {
        (
            rms_squared_error / compared as f64,
            centroid_squared_error / compared as f64,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOWS: &[usize] = &[256, 1_024];
    const SETTINGS: SoundMatchObjectiveSettings = SoundMatchObjectiveSettings {
        sample_rate_hz: 48_000,
        spectral_windows: WINDOWS,
        analysis: AnalysisSettings {
            frame_size: 1_024,
            hop_size: 128,
            min_frequency_hz: 30.0,
            max_frequency_hz: 20_000.0,
            silence_rms: 1.0e-6,
        },
        weights: SoundMatchWeights {
            spectral: 0.7,
            rms: 0.2,
            centroid: 0.1,
        },
    };

    fn sine(frequency_hz: f64, amplitude: impl Fn(usize) -> f64) -> Vec<f32> {
        (0..4_096)
            .map(|frame| {
                (std::f64::consts::TAU * frequency_hz * frame as f64 / 48_000.0).sin() as f32
                    * amplitude(frame) as f32
            })
            .collect()
    }

    #[test]
    fn objective_identical_audio_has_zero_loss_and_whole_region_gain_alignment_is_fixed() {
        let reference = sine(440.0, |_| 0.4);
        let identical = compare_aligned_audio(&reference, &reference, SETTINGS)
            .expect("identical audio should compare");
        let quieter: Vec<f32> = reference.iter().map(|sample| sample * 0.5).collect();
        let gain_aligned = compare_aligned_audio(&quieter, &reference, SETTINGS)
            .expect("globally scaled audio should compare");

        assert!(identical.total < 1.0e-12, "{identical:?}");
        assert!(identical.spectral < 1.0e-12);
        assert!(identical.rms < 1.0e-12);
        assert!(identical.centroid < 1.0e-12);
        assert!((identical.candidate_gain - 1.0).abs() < 1.0e-12);
        assert!(gain_aligned.total < 1.0e-10, "{gain_aligned:?}");
        assert!((gain_aligned.candidate_gain - 2.0).abs() < 1.0e-6);
    }

    #[test]
    fn objective_spectral_level_and_centroid_differences_raise_their_owning_terms() {
        let reference = sine(440.0, |frame| if frame < 2_048 { 0.2 } else { 0.7 });
        let harmonic: Vec<f32> = reference
            .iter()
            .enumerate()
            .map(|(frame, sample)| {
                sample
                    + (std::f64::consts::TAU * 4_000.0 * frame as f64 / 48_000.0).sin() as f32
                        * 0.15
            })
            .collect();
        let reversed_envelope = sine(440.0, |frame| if frame < 2_048 { 0.7 } else { 0.2 });

        let baseline = compare_aligned_audio(&reference, &reference, SETTINGS).unwrap();
        let spectral = compare_aligned_audio(&harmonic, &reference, SETTINGS).unwrap();
        let temporal = compare_aligned_audio(&reversed_envelope, &reference, SETTINGS).unwrap();

        assert!(
            spectral.spectral > baseline.spectral + 1.0e-4,
            "{spectral:?}"
        );
        assert!(
            spectral.centroid > baseline.centroid + 1.0e-4,
            "{spectral:?}"
        );
        assert!(temporal.rms > baseline.rms + 1.0e-4, "{temporal:?}");
        for score in [spectral, temporal] {
            assert!(score.spectral.is_finite());
            assert!(score.rms.is_finite());
            assert!(score.centroid.is_finite());
            assert!(score.total.is_finite());
            assert!(score.total > 0.0);
            assert!(
                (score.total
                    - (score.spectral * SETTINGS.weights.spectral
                        + score.rms * SETTINGS.weights.rms
                        + score.centroid * SETTINGS.weights.centroid))
                    .abs()
                    < 1.0e-12
            );
        }
    }

    #[test]
    fn objective_rejects_malformed_or_unusable_audio_and_settings() {
        let reference = sine(440.0, |_| 0.4);
        let cases = [
            (
                compare_aligned_audio(&reference[..2_000], &reference, SETTINGS),
                "same aligned length",
            ),
            (
                compare_aligned_audio(
                    &vec![0.0; reference.len()],
                    &vec![0.0; reference.len()],
                    SETTINGS,
                ),
                "non-silent reference",
            ),
            (
                compare_aligned_audio(
                    &reference,
                    &reference,
                    SoundMatchObjectiveSettings {
                        sample_rate_hz: 0,
                        ..SETTINGS
                    },
                ),
                "sample rate",
            ),
            (
                compare_aligned_audio(
                    &reference,
                    &reference,
                    SoundMatchObjectiveSettings {
                        spectral_windows: &[255],
                        ..SETTINGS
                    },
                ),
                "spectral window",
            ),
            (
                compare_aligned_audio(
                    &reference,
                    &reference,
                    SoundMatchObjectiveSettings {
                        weights: SoundMatchWeights {
                            spectral: -1.0,
                            ..SETTINGS.weights
                        },
                        ..SETTINGS
                    },
                ),
                "weights",
            ),
            (
                compare_aligned_audio(
                    &reference,
                    &reference,
                    SoundMatchObjectiveSettings {
                        analysis: AnalysisSettings {
                            hop_size: 0,
                            ..SETTINGS.analysis
                        },
                        ..SETTINGS
                    },
                ),
                "hop size",
            ),
            (
                compare_aligned_audio(
                    &reference,
                    &reference,
                    SoundMatchObjectiveSettings {
                        analysis: AnalysisSettings {
                            min_frequency_hz: 0.0,
                            ..SETTINGS.analysis
                        },
                        ..SETTINGS
                    },
                ),
                "minimum frequency",
            ),
        ];

        for (result, expected) in cases {
            let error = result.expect_err("invalid objective input should fail");
            assert!(error.contains(expected), "unexpected error: {error}");
        }

        let mut non_finite = reference.clone();
        non_finite[42] = f32::NAN;
        assert!(
            compare_aligned_audio(&non_finite, &reference, SETTINGS)
                .expect_err("non-finite candidate should fail")
                .contains("finite")
        );

        for weights in [
            SoundMatchWeights {
                spectral: -0.01,
                rms: 0.2,
                centroid: 0.1,
            },
            SoundMatchWeights {
                spectral: f64::NAN,
                rms: 0.2,
                centroid: 0.1,
            },
            SoundMatchWeights {
                spectral: 0.0,
                rms: 0.0,
                centroid: 0.0,
            },
        ] {
            assert!(
                compare_aligned_audio(
                    &reference,
                    &reference,
                    SoundMatchObjectiveSettings {
                        weights,
                        ..SETTINGS
                    },
                )
                .expect_err("each invalid weight contract should fail independently")
                .contains("weights")
            );
        }
    }

    #[test]
    fn objective_handles_silent_candidates_empty_trajectories_and_empty_spectral_bands() {
        let reference = sine(440.0, |_| 0.4);
        let silence = vec![0.0; reference.len()];
        let silent_candidate = compare_aligned_audio(&silence, &reference, SETTINGS)
            .expect("a silent candidate is a poor but valid candidate");
        assert_eq!(silent_candidate.candidate_gain, 1.0);
        assert!(silent_candidate.total.is_finite() && silent_candidate.total > 0.0);
        assert!(
            compare_aligned_audio(
                &reference,
                &reference,
                SoundMatchObjectiveSettings {
                    weights: SoundMatchWeights {
                        spectral: -0.0,
                        rms: 0.2,
                        centroid: 0.1,
                    },
                    ..SETTINGS
                },
            )
            .is_ok(),
            "negative zero is a valid non-negative weight"
        );

        let no_spectral_bins = compare_aligned_audio(
            &reference,
            &reference,
            SoundMatchObjectiveSettings {
                spectral_windows: &[16],
                analysis: AnalysisSettings {
                    min_frequency_hz: 20_001.0,
                    max_frequency_hz: 20_002.0,
                    ..SETTINGS.analysis
                },
                ..SETTINGS
            },
        )
        .expect("a valid narrow band without FFT bins should remain finite");
        assert_eq!(no_spectral_bins.spectral, 0.0);

        let short = &reference[..256];
        let no_trajectory_frames = compare_aligned_audio(
            short,
            short,
            SoundMatchObjectiveSettings {
                spectral_windows: &[256],
                ..SETTINGS
            },
        )
        .expect("audio shorter than the trajectory window should still compare spectrally");
        assert_eq!(no_trajectory_frames.rms, 0.0);
        assert_eq!(no_trajectory_frames.centroid, 0.0);

        let mut epsilon_candidate = vec![0.0; 16];
        epsilon_candidate[0] = f32::from_bits((127 - 26) << 23);
        let mut impulse_reference = vec![0.0; 16];
        impulse_reference[0] = 1.0;
        let epsilon_energy = compare_aligned_audio(
            &epsilon_candidate,
            &impulse_reference,
            SoundMatchObjectiveSettings {
                spectral_windows: &[16],
                analysis: AnalysisSettings {
                    frame_size: 16,
                    hop_size: 16,
                    ..SETTINGS.analysis
                },
                ..SETTINGS
            },
        )
        .expect("epsilon-energy candidate should compare");
        assert_eq!(epsilon_energy.candidate_gain, 1.0);

        let exact_floor = 2.0_f32.powi(-10);
        let floor_reference = vec![exact_floor; 16];
        let floor_error = compare_aligned_audio(
            &floor_reference,
            &floor_reference,
            SoundMatchObjectiveSettings {
                spectral_windows: &[16],
                analysis: AnalysisSettings {
                    frame_size: 16,
                    hop_size: 16,
                    silence_rms: f64::from(exact_floor),
                    ..SETTINGS.analysis
                },
                ..SETTINGS
            },
        )
        .expect_err("a reference exactly on the silence floor should be rejected");
        assert!(floor_error.contains("non-silent"));
    }

    #[test]
    fn objective_spectral_and_trajectory_formulas_match_numeric_fixtures() {
        let reference: Vec<f32> = (0..96)
            .map(|index| {
                let amplitude = if index < 48 { 0.8 } else { 0.2 };
                (std::f64::consts::TAU * 6_000.0 * index as f64 / 48_000.0).sin() as f32 * amplitude
            })
            .collect();
        let candidate: Vec<f32> = (0..96)
            .map(|index| {
                let amplitude = if index < 48 { 0.15 } else { 0.7 };
                ((std::f64::consts::TAU * 9_000.0 * index as f64 / 48_000.0).sin()
                    + 0.25 * (std::f64::consts::TAU * 3_000.0 * index as f64 / 48_000.0).sin())
                    as f32
                    * amplitude
            })
            .collect();
        let spectral = multi_resolution_spectral_loss(
            &candidate,
            &reference,
            48_000,
            &[16, 32],
            3_000.0,
            12_000.0,
        );

        let frame = |rms, centroid| crate::sound_analysis::AnalysisFrame {
            start_frame: 0,
            time_seconds: 0.0,
            rms,
            peak: rms,
            spectral_centroid_hz: centroid,
        };
        let (rms, centroid) = trajectory_losses(
            &[
                frame(0.1, Some(200.0)),
                frame(0.2, None),
                frame(0.0, Some(0.0)),
                frame(0.3, Some(100.0)),
            ],
            &[
                frame(0.2, Some(100.0)),
                frame(0.1, None),
                frame(0.0, Some(100.0)),
                frame(0.3, Some(0.0)),
            ],
            1.0e-6,
        );

        assert!((spectral - 229.30984419479594).abs() < 1.0e-9, "{spectral}");
        assert!((rms - 0.045309529144728286).abs() < 1.0e-12, "{rms}");
        assert!((centroid - 0.75).abs() < 1.0e-12, "{centroid}");
    }

    fn quadratic_score(values: &[f64]) -> Result<SoundMatchScore, String> {
        let total = (values[0] - 0.8).powi(2) + (values[1] - 0.2).powi(2);
        Ok(SoundMatchScore {
            total,
            ..SoundMatchScore::default()
        })
    }

    fn continue_search(_: BoundedSearchProgress) -> bool {
        true
    }

    fn search_parameters() -> Vec<BoundedSearchParameter> {
        vec![
            BoundedSearchParameter {
                id: "x".to_string(),
                min: 0.0,
                max: 1.0,
                initial: 0.1,
            },
            BoundedSearchParameter {
                id: "y".to_string(),
                min: 0.0,
                max: 1.0,
                initial: 0.9,
            },
        ]
    }

    #[test]
    fn bounded_search_is_seeded_reproducible_and_improves_the_initial_candidate() {
        let first = deterministic_bounded_search(
            &search_parameters(),
            303,
            32,
            quadratic_score,
            continue_search,
        )
        .expect("first search should complete");
        let second = deterministic_bounded_search(
            &search_parameters(),
            303,
            32,
            quadratic_score,
            continue_search,
        )
        .expect("second search should complete");

        assert_eq!(first, second);
        assert!(!first.cancelled);
        assert_eq!(first.history.len(), 32);
        assert!(first.best.score.total < first.history[0].score.total * 0.25);
        assert!(first.best.values[0] >= 0.0 && first.best.values[0] <= 1.0);
        assert!(first.best.values[1] >= 0.0 && first.best.values[1] <= 1.0);
    }

    #[test]
    fn bounded_search_reports_monotonic_progress_and_preserves_best_on_cancellation() {
        let mut progress = Vec::new();
        let result = deterministic_bounded_search(
            &search_parameters(),
            303,
            32,
            quadratic_score,
            |update| {
                progress.push(update);
                update.completed_evaluations < 5
            },
        )
        .expect("cancelled search should retain its best result");

        assert!(result.cancelled);
        assert_eq!(result.history.len(), 5);
        assert_eq!(progress.len(), 5);
        assert_eq!(progress.last().unwrap().completed_evaluations, 5);
        assert!(
            progress
                .windows(2)
                .all(|pair| pair[1].best_total <= pair[0].best_total)
        );
        assert_eq!(result.best.score.total, progress.last().unwrap().best_total);
    }

    #[test]
    fn bounded_search_uses_the_declared_coordinate_and_radius_schedule() {
        let parameters = vec![
            BoundedSearchParameter {
                id: "a".to_string(),
                min: -2.0,
                max: 2.0,
                initial: -1.0,
            },
            BoundedSearchParameter {
                id: "b".to_string(),
                min: 10.0,
                max: 30.0,
                initial: 28.0,
            },
            BoundedSearchParameter {
                id: "c".to_string(),
                min: 100.0,
                max: 200.0,
                initial: 150.0,
            },
        ];
        let result = deterministic_bounded_search(
            &parameters,
            303,
            10,
            |_| Ok(SoundMatchScore::default()),
            continue_search,
        )
        .expect("constant-score search should complete");

        let expected = [
            vec![0.6, 28.0, 150.0],
            vec![-2.0, 28.0, 150.0],
            vec![-1.0, 30.0, 150.0],
            vec![-1.0, 20.0, 150.0],
            vec![-1.0, 28.0, 190.0],
            vec![-1.0, 28.0, 110.0],
            vec![-1.8967948869879296, 24.08889416827367, 145.41339479738593],
            vec![-1.7398431889674546, 26.655178228623146, 168.81270495872667],
            vec![0.04, 28.0, 150.0],
        ];
        for (actual, expected) in result.history.iter().skip(1).zip(expected) {
            assert!(
                actual
                    .values
                    .iter()
                    .zip(expected)
                    .all(|(actual, expected)| (actual - expected).abs() < 1e-12),
                "unexpected schedule value: {:?}",
                actual.values
            );
        }
    }

    #[test]
    fn stereo_matching_analysis_uses_the_arithmetic_mean() {
        assert_eq!(mono_audio(&[1.0, -0.5], &[-0.5, 1.0]), vec![0.25, 0.25]);
    }

    #[test]
    fn bounded_search_rejects_invalid_contracts_before_evaluation() {
        let invalid_parameters = [
            vec![],
            vec![BoundedSearchParameter {
                id: "x".to_string(),
                min: 1.0,
                max: 0.0,
                initial: 0.5,
            }],
            vec![BoundedSearchParameter {
                id: "x".to_string(),
                min: 0.0,
                max: 1.0,
                initial: 2.0,
            }],
        ];

        for parameters in invalid_parameters {
            let error =
                deterministic_bounded_search(&parameters, 303, 4, quadratic_score, continue_search)
                    .expect_err("invalid bounded search should fail");
            assert!(error.contains("parameter"), "unexpected error: {error}");
        }
        let error =
            deterministic_bounded_search(&search_parameters(), 0, 4, quadratic_score, |_| true)
                .expect_err("zero seed should fail");
        assert!(error.contains("seed"));
        let error =
            deterministic_bounded_search(&search_parameters(), 303, 0, quadratic_score, |_| true)
                .expect_err("zero evaluation budget should fail");
        assert!(error.contains("evaluation"));
        let invalid_score = deterministic_bounded_search(
            &search_parameters(),
            303,
            1,
            |_| {
                Ok(SoundMatchScore {
                    total: f64::NAN,
                    ..SoundMatchScore::default()
                })
            },
            |_| true,
        )
        .expect_err("a non-finite objective score should fail");
        assert!(invalid_score.contains("invalid"));
    }

    fn short_acid_fixture() -> crate::sound_workbench::SoundFixture {
        let mut fixture = crate::sound_workbench::load_sound_fixture_file(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("examples/sound-design/tb303-acid-poc.yaml"),
        )
        .expect("TB-303 fixture should load");
        fixture.render.duration_frames = 144_000;
        fixture.timeline.repetitions = 1;
        let matching = fixture.matching.as_mut().expect("fixture should match");
        matching.max_evaluations = 8;
        matching.parameters = vec!["filter.cutoff".to_string()];
        fixture
    }

    fn temp_wav(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("dandrum-match-{name}-{}.wav", std::process::id()))
    }

    fn write_reference(
        fixture: &crate::sound_workbench::SoundFixture,
        cutoff: f64,
        name: &str,
    ) -> std::path::PathBuf {
        write_scaled_reference(fixture, cutoff, 1.0, name)
    }

    fn write_scaled_reference(
        fixture: &crate::sound_workbench::SoundFixture,
        cutoff: f64,
        gain: f32,
        name: &str,
    ) -> std::path::PathBuf {
        let values = std::collections::BTreeMap::from([("filter.cutoff".to_string(), cutoff)]);
        let mut render = crate::sound_workbench::render_sound_fixture_with_public_numeric_values(
            fixture, &values,
        )
        .expect("reference should render");
        for sample in render.left.iter_mut().chain(&mut render.right) {
            *sample *= gain;
        }
        let path = temp_wav(name);
        crate::wav::write_wav_file(&path, render.sample_rate_hz, &render.left, &render.right)
            .expect("reference WAV should write");
        path
    }

    #[test]
    fn fixture_match_improves_a_self_reference_and_returns_coherent_provenance() {
        let fixture = short_acid_fixture();
        let reference_path = write_reference(&fixture, 0.75, "self-reference");
        let mut progress = Vec::new();

        let artifact = match_sound_fixture(&fixture, &reference_path, |update| {
            progress.push(update);
            true
        })
        .expect("self-reference should match");

        assert_eq!(artifact.manifest.status, SoundMatchStatus::Completed);
        assert_eq!(artifact.manifest.seed, 303);
        assert_eq!(artifact.manifest.completed_evaluations, 8);
        assert_eq!(artifact.manifest.history.len(), 8);
        assert_eq!(artifact.manifest.reference_sha256.len(), 64);
        assert_eq!(artifact.manifest.fixture_sha256.len(), 64);
        assert_eq!(artifact.manifest.patch_sha256.len(), 64);
        assert_eq!(artifact.manifest.optimizer, "dandrum.seeded_evolution.v1");
        assert_eq!(
            artifact.manifest.sample_rate_hz,
            fixture.render.sample_rate_hz
        );
        assert_eq!(
            artifact.manifest.block_size_frames,
            fixture.render.block_size_frames
        );
        assert_eq!(
            artifact.manifest.render_duration_frames,
            fixture.render.duration_frames
        );
        assert_eq!(artifact.manifest.analysis, fixture.analysis);
        assert!(
            artifact.manifest.best_score.total < artifact.manifest.history[0].score.total,
            "{:?}",
            artifact.manifest
        );
        let cutoff = &artifact.manifest.best_parameters[0];
        assert_eq!(cutoff.id, "filter.cutoff");
        assert!(cutoff.best >= cutoff.min && cutoff.best <= cutoff.max);
        assert!((0.0..=1.0).contains(&cutoff.normalized));
        assert!(
            (cutoff.normalized - (cutoff.best - cutoff.min) / (cutoff.max - cutoff.min)).abs()
                < 1e-12
        );
        assert!(
            artifact
                .manifest
                .history
                .iter()
                .enumerate()
                .all(|(index, record)| record.index == index + 1)
        );
        assert_eq!(artifact.sample_rate_hz, 48_000);
        assert_eq!(artifact.duration_frames, fixture.render.duration_frames);
        assert!(!artifact.candidate_metrics.is_empty());
        assert_eq!(
            artifact.candidate_metrics.len(),
            artifact.reference_metrics.len()
        );
        assert!(artifact.candidate_wav_bytes.starts_with(b"RIFF"));
        assert!(artifact.reference_wav_bytes.starts_with(b"RIFF"));
        assert_eq!(progress.len(), 8);
    }

    #[test]
    fn fixture_match_applies_the_measured_gain_to_the_returned_candidate() {
        let mut fixture = short_acid_fixture();
        fixture.matching.as_mut().unwrap().max_evaluations = 1;
        let reference_path = write_scaled_reference(&fixture, 0.4, 0.25, "scaled-reference");

        let artifact = match_sound_fixture(&fixture, reference_path, continue_search)
            .expect("scaled self-reference should match");
        let candidate_rms = artifact
            .candidate_metrics
            .iter()
            .map(|frame| frame.rms)
            .sum::<f64>()
            / artifact.candidate_metrics.len() as f64;
        let reference_rms = artifact
            .reference_metrics
            .iter()
            .map(|frame| frame.rms)
            .sum::<f64>()
            / artifact.reference_metrics.len() as f64;

        assert!((artifact.manifest.best_score.candidate_gain - 0.25).abs() < 0.01);
        assert!((candidate_rms / reference_rms - 1.0).abs() < 0.01);
    }

    #[test]
    fn fixture_match_returns_the_best_completed_candidate_when_cancelled() {
        let fixture = short_acid_fixture();
        let reference_path = write_reference(&fixture, 0.75, "cancelled");

        let artifact = match_sound_fixture(&fixture, reference_path, |progress| {
            progress.completed_evaluations < 2
        })
        .expect("cancelled fixture match should retain its best candidate");

        assert_eq!(artifact.manifest.status, SoundMatchStatus::Cancelled);
        assert_eq!(artifact.manifest.completed_evaluations, 2);
        assert_eq!(artifact.manifest.history.len(), 2);
        assert_eq!(artifact.manifest.best_parameters.len(), 1);
    }

    #[test]
    fn fixture_match_uses_immutable_patch_and_reference_snapshots_for_every_evaluation() {
        let mut fixture = short_acid_fixture();
        fixture.matching.as_mut().unwrap().max_evaluations = 2;
        let directory =
            std::env::temp_dir().join(format!("dandrum-match-snapshot-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let patch_path = directory.join("acid.yaml");
        let original_patch = std::fs::read(&fixture.patch).unwrap();
        std::fs::write(&patch_path, &original_patch).unwrap();
        fixture.patch = patch_path.clone();
        let reference_path = write_reference(&fixture, 0.75, "immutable-snapshot");
        let original_reference = std::fs::read(&reference_path).unwrap();

        let baseline = match_sound_fixture(&fixture, &reference_path, continue_search)
            .expect("baseline snapshot match should complete");
        std::fs::write(&patch_path, &original_patch).unwrap();
        std::fs::write(&reference_path, &original_reference).unwrap();
        let observed = match_sound_fixture(&fixture, &reference_path, |progress| {
            if progress.completed_evaluations == 1 {
                std::fs::write(&patch_path, b"not a patch").unwrap();
                std::fs::write(&reference_path, b"not a wave").unwrap();
            }
            true
        })
        .expect("owned snapshots should isolate the running match from file edits");

        assert_eq!(observed.manifest.history, baseline.manifest.history);
        assert_eq!(
            observed.manifest.best_parameters,
            baseline.manifest.best_parameters
        );
        assert_eq!(observed.candidate_wav_bytes, baseline.candidate_wav_bytes);
        assert_eq!(observed.reference_wav_bytes, original_reference);
        assert_eq!(observed.patch_yaml.as_bytes(), original_patch);
        assert_eq!(observed.patch_yaml, baseline.patch_yaml);
        assert_eq!(
            observed.manifest.reference_sha256,
            baseline.manifest.reference_sha256
        );
        std::fs::remove_dir_all(directory).ok();
        std::fs::remove_file(reference_path).ok();
    }

    #[test]
    fn fixture_match_rejects_short_silent_and_wrong_rate_references_before_search() {
        let fixture = short_acid_fixture();
        let short = temp_wav("short");
        let silent = temp_wav("silent");
        let wrong_rate = temp_wav("wrong-rate");
        let unsupported = temp_wav("unsupported");
        crate::wav::write_wav_file(&short, 48_000, &vec![0.0; 96_000], &vec![0.0; 96_000]).unwrap();
        crate::wav::write_wav_file(&silent, 48_000, &vec![0.0; 144_000], &vec![0.0; 144_000])
            .unwrap();
        crate::wav::write_wav_file(
            &wrong_rate,
            44_100,
            &vec![0.2; 144_000],
            &vec![0.2; 144_000],
        )
        .unwrap();
        std::fs::write(&unsupported, b"not a wave").unwrap();

        for (path, expected) in [
            (short, "shorter than the matching region"),
            (silent, "non-silent reference"),
            (wrong_rate, "sample-rate mismatch"),
            (unsupported, "unsupported format"),
        ] {
            let mut progress_calls = 0;
            let error = match_sound_fixture(&fixture, path, |_| {
                progress_calls += 1;
                true
            })
            .expect_err("invalid reference should fail before search");
            assert!(error.contains(expected), "unexpected error: {error}");
            assert_eq!(progress_calls, 0, "optimizer must not start for {expected}");
        }

        let near_silent = temp_wav("near-silent");
        crate::wav::write_wav_file(
            &near_silent,
            48_000,
            &vec![0.0005; 144_000],
            &vec![0.0005; 144_000],
        )
        .unwrap();
        let mut elevated_floor = fixture;
        elevated_floor.analysis.silence_rms = 0.001;
        elevated_floor.patch =
            Path::new("/definitely/missing/near-silent-patch.yaml").to_path_buf();
        let mut progress_calls = 0;
        assert!(
            match_sound_fixture(&elevated_floor, near_silent, |_| {
                progress_calls += 1;
                true
            })
            .expect_err("reference below the configured RMS floor should fail")
            .contains("non-silent reference")
        );
        assert_eq!(progress_calls, 0);
    }

    #[test]
    fn fixture_match_reports_missing_contract_reference_patch_and_candidate_failures() {
        let fixture = short_acid_fixture();
        let reference_path = write_reference(&fixture, 0.75, "errors");

        let mut no_matching = fixture.clone();
        no_matching.matching = None;
        assert!(
            match_sound_fixture(&no_matching, &reference_path, |_| true)
                .expect_err("missing matching declaration should fail")
                .contains("does not declare")
        );
        assert!(
            match_sound_fixture(&fixture, temp_wav("missing"), |_| true)
                .expect_err("missing reference should fail")
                .contains("failed to read reference")
        );

        let mut missing_patch = fixture.clone();
        missing_patch.patch =
            Path::new("/definitely/missing/dandrum-match-patch.yaml").to_path_buf();
        assert!(
            match_sound_fixture(&missing_patch, &reference_path, |_| true)
                .expect_err("missing patch should fail")
                .contains("failed to load sound matching patch")
        );

        let mut unknown_parameter = fixture.clone();
        unknown_parameter.matching.as_mut().unwrap().parameters = vec!["filter.absent".to_string()];
        assert!(
            match_sound_fixture(&unknown_parameter, &reference_path, |_| true)
                .expect_err("unknown parameter should fail")
                .contains("unknown public numeric parameter")
        );

        let non_numeric_patch = std::env::temp_dir().join(format!(
            "dandrum-match-non-numeric-{}.yaml",
            std::process::id()
        ));
        std::fs::write(
            &non_numeric_patch,
            r#"
metadata:
  name: Non-numeric matching surface
instrument:
  id: dandrum.non-numeric-match
  preset_schema_version: 1
preset_surface:
  parameters:
    - name: filter.cutoff
      type: text
      default: saw
      maps_to: osc.waveform
render:
  sample_rate_hz: 48000
  block_size_frames: 64
  duration_frames: 144000
modules:
  - id: osc
    type: oscillator
  - id: mixer
    type: audio_mixer
  - id: out
    type: audio_output
    inputs:
      - { name: left, signal_type: audio }
      - { name: right, signal_type: audio }
connections:
  - { from: osc.audio, to: mixer.inputs }
  - { from: mixer.mix, to: out.left }
  - { from: mixer.mix, to: out.right }
"#,
        )
        .unwrap();
        let mut non_numeric = fixture.clone();
        non_numeric.patch = non_numeric_patch;
        assert!(
            match_sound_fixture(&non_numeric, &reference_path, continue_search)
                .expect_err("non-numeric matching parameter should fail")
                .contains("continuous numeric")
        );

        let mut invalid_analysis = fixture;
        invalid_analysis.analysis.hop_size = 0;
        assert!(
            match_sound_fixture(&invalid_analysis, reference_path, |_| true)
                .expect_err("candidate analysis failure should propagate")
                .contains("hop size")
        );
    }
}
