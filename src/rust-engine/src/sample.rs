use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use crate::patch::{AssetKind, ParameterValue, PatchDocument};

#[derive(Clone, Debug, PartialEq)]
pub struct LoadedSample {
    sample_rate_hz: u32,
    frames: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedSamplerAssets {
    samples_by_module: BTreeMap<String, LoadedSample>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SampleLoadError {
    diagnostics: Vec<String>,
}

pub fn prepare_sampler_assets(
    patch: &PatchDocument,
    base_dir: impl AsRef<Path>,
) -> Result<PreparedSamplerAssets, SampleLoadError> {
    let mut diagnostics = Vec::new();
    let mut loaded_by_asset = BTreeMap::new();
    let mut samples_by_module = BTreeMap::new();
    let base_dir = base_dir.as_ref();

    for module in patch
        .modules
        .iter()
        .filter(|module| module.module_type == "sampler")
    {
        let Some(ParameterValue::Text(asset_id)) = module.parameters.get("asset") else {
            continue;
        };
        let Some(asset) = patch.assets.iter().find(|asset| asset.id == *asset_id) else {
            continue;
        };
        if asset.kind != AssetKind::Sample {
            continue;
        }

        if !loaded_by_asset.contains_key(asset_id) {
            let path = base_dir.join(&asset.path);
            match load_pcm_wav(&path, patch.render.sample_rate_hz) {
                Ok(sample) => {
                    loaded_by_asset.insert(asset_id.clone(), sample);
                }
                Err(message) => diagnostics.push(format!(
                    "sample asset {} at {}: {message}",
                    asset.id,
                    path.display()
                )),
            }
        }

        if let Some(sample) = loaded_by_asset.get(asset_id) {
            samples_by_module.insert(module.id.clone(), sample.clone());
        }
    }

    if diagnostics.is_empty() {
        Ok(PreparedSamplerAssets { samples_by_module })
    } else {
        Err(SampleLoadError { diagnostics })
    }
}

impl LoadedSample {
    pub fn new(sample_rate_hz: u32, frames: Vec<f32>) -> Self {
        Self {
            sample_rate_hz,
            frames,
        }
    }

    pub fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    pub fn frames(&self) -> &[f32] {
        &self.frames
    }
}

impl PreparedSamplerAssets {
    pub fn empty() -> Self {
        Self {
            samples_by_module: BTreeMap::new(),
        }
    }

    pub fn from_samples_by_module(samples_by_module: BTreeMap<String, LoadedSample>) -> Self {
        Self { samples_by_module }
    }

    pub fn get(&self, module_id: &str) -> Option<&LoadedSample> {
        self.samples_by_module.get(module_id)
    }
}

impl SampleLoadError {
    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }
}

impl fmt::Display for SampleLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "sample asset loading failed")?;
        for diagnostic in &self.diagnostics {
            write!(formatter, "\n- {diagnostic}")?;
        }
        Ok(())
    }
}

impl std::error::Error for SampleLoadError {}

fn load_pcm_wav(path: &Path, expected_sample_rate_hz: u32) -> Result<LoadedSample, String> {
    let loaded = crate::audio_loading::load_pcm_wav(path, expected_sample_rate_hz)?;
    Ok(LoadedSample::new(
        loaded.sample_rate_hz(),
        loaded.frames().to_vec(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::{
        AssetDeclaration, ModuleDeclaration, PatchMetadata, RenderSettings, VoiceAllocation,
    };
    use crate::wav::write_wav_stereo_i16;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn loads_readable_pcm_wav_sample_asset() {
        let dir = unique_temp_dir("loads_readable_pcm_wav_sample_asset");
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let wav_path = dir.join("hit.wav");
        write_wav_stereo_i16(fs::File::create(&wav_path).unwrap(), 48_000, &[0.5], &[0.5])
            .expect("wav should write");

        let assets = prepare_sampler_assets(&sampler_patch("hit.wav", 48_000), &dir)
            .expect("sample should load");

        let sample = assets.get("sampler").expect("sampler sample should exist");
        assert_eq!(sample.sample_rate_hz(), 48_000);
        assert!((sample.frames()[0] - 0.5).abs() < 0.0001);
    }

    #[test]
    fn reports_missing_sample_file_with_asset_id_and_path() {
        let dir = unique_temp_dir("reports_missing_sample_file");
        let error = prepare_sampler_assets(&sampler_patch("missing.wav", 48_000), &dir)
            .expect_err("missing file should fail");

        assert!(error.diagnostics()[0].contains("sample asset hit"));
        assert!(error.diagnostics()[0].contains("missing.wav"));
        assert!(error.diagnostics()[0].contains("failed to read"));
    }

    #[test]
    fn reports_unsupported_sample_file_with_asset_id_and_path() {
        let dir = unique_temp_dir("reports_unsupported_sample_file");
        fs::create_dir_all(&dir).expect("temp dir should be created");
        fs::write(dir.join("hit.txt"), b"not wave").expect("fixture should write");

        let error = prepare_sampler_assets(&sampler_patch("hit.txt", 48_000), &dir)
            .expect_err("unsupported file should fail");

        assert!(error.diagnostics()[0].contains("sample asset hit"));
        assert!(error.diagnostics()[0].contains("hit.txt"));
        assert!(error.diagnostics()[0].contains("unsupported format"));
    }

    #[test]
    fn reports_sample_rate_mismatch() {
        let dir = unique_temp_dir("reports_sample_rate_mismatch");
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let wav_path = dir.join("hit.wav");
        write_wav_stereo_i16(fs::File::create(&wav_path).unwrap(), 44_100, &[0.5], &[0.5])
            .expect("wav should write");

        let error = prepare_sampler_assets(&sampler_patch("hit.wav", 48_000), &dir)
            .expect_err("rate mismatch should fail");

        assert!(error.diagnostics()[0].contains("sample-rate mismatch"));
        assert!(error.diagnostics()[0].contains("44100"));
        assert!(error.diagnostics()[0].contains("48000"));
    }

    fn sampler_patch(path: &str, sample_rate_hz: u32) -> PatchDocument {
        PatchDocument {
            metadata: PatchMetadata {
                name: "Sampler".to_string(),
                version: None,
                author: None,
            },
            render: RenderSettings {
                sample_rate_hz,
                block_size_frames: 1,
                duration_frames: 4,
            },
            assets: vec![AssetDeclaration {
                id: "hit".to_string(),
                kind: AssetKind::Sample,
                path: path.to_string(),
            }],
            module_definitions: vec![],
            modules: vec![ModuleDeclaration {
                id: "sampler".to_string(),
                module_type: "sampler".to_string(),
                inputs: vec![],
                outputs: vec![],
                parameters: BTreeMap::from([(
                    "asset".to_string(),
                    ParameterValue::Text("hit".to_string()),
                )]),
            }],
            connections: vec![],
            voice_allocation: VoiceAllocation::default(),
        }
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("dandrum-{name}-{}", std::process::id()))
    }
}
