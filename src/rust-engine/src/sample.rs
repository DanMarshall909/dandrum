use std::collections::BTreeMap;
#[cfg(test)]
use std::fmt;
#[cfg(test)]
use std::path::Path;

#[cfg(test)]
use crate::patch::{AssetKind, ParameterValue, PatchDocument};

#[derive(Clone, Debug, PartialEq)]
pub struct LoadedSample {
    sample_rate_hz: u32,
    source_channel_count: u16,
    frames: Vec<f32>,
    source_pcm: Option<Vec<i16>>,
    content_revision: [u8; 32],
}

pub const MAX_WAVEFORM_BUCKETS: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveformBucket {
    pub start_frame: u64,
    pub end_frame: u64,
    pub minimum: f32,
    pub maximum: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedSamplerAssets {
    samples_by_module: BTreeMap<String, LoadedSample>,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SampleLoadError {
    diagnostics: Vec<String>,
}

#[cfg(test)]
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
        Self::from_loaded_audio(crate::audio_loading::LoadedAudio::new(
            sample_rate_hz,
            frames,
        ))
    }

    pub fn with_source_channels(
        sample_rate_hz: u32,
        source_channel_count: u16,
        frames: Vec<f32>,
    ) -> Self {
        Self::from_loaded_audio(crate::audio_loading::LoadedAudio::with_source_channels(
            sample_rate_hz,
            source_channel_count,
            frames,
        ))
    }

    pub(crate) fn from_loaded_audio(audio: crate::audio_loading::LoadedAudio) -> Self {
        Self {
            sample_rate_hz: audio.sample_rate_hz,
            source_channel_count: audio.source_channel_count,
            frames: audio.frames,
            source_pcm: audio.source_pcm,
            content_revision: audio.content_revision,
        }
    }

    pub fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    pub fn source_channel_count(&self) -> u16 {
        self.source_channel_count
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn frames(&self) -> &[f32] {
        &self.frames
    }

    pub fn source_sample(&self, channel: u16, frame: usize) -> Option<f32> {
        crate::audio_loading::source_sample(
            &self.frames,
            self.source_channel_count,
            self.source_pcm.as_deref(),
            channel,
            frame,
        )
    }

    pub fn content_revision(&self) -> &[u8; 32] {
        &self.content_revision
    }

    pub fn reduce_waveform(
        &self,
        channel: u16,
        start_frame: u64,
        end_frame: u64,
        bucket_count: usize,
    ) -> Option<Vec<WaveformBucket>> {
        let start = usize::try_from(start_frame).ok()?;
        let end = usize::try_from(end_frame).ok()?;
        if channel >= self.source_channel_count
            || start >= end
            || end > self.frames.len()
            || bucket_count == 0
            || bucket_count > MAX_WAVEFORM_BUCKETS
            || bucket_count > end - start
        {
            return None;
        }
        let span = end - start;
        let mut buckets = Vec::with_capacity(bucket_count);
        for index in 0..bucket_count {
            let begin = start + (index as u128 * span as u128 / bucket_count as u128) as usize;
            let finish =
                start + ((index + 1) as u128 * span as u128 / bucket_count as u128) as usize;
            let mut minimum = f32::INFINITY;
            let mut maximum = f32::NEG_INFINITY;
            for frame in begin..finish {
                let value = self.source_sample(channel, frame)?;
                if !value.is_finite() {
                    return None;
                }
                minimum = minimum.min(value);
                maximum = maximum.max(value);
            }
            buckets.push(WaveformBucket {
                start_frame: begin as u64,
                end_frame: finish as u64,
                minimum,
                maximum,
            });
        }
        Some(buckets)
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

#[cfg(test)]
impl SampleLoadError {
    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }
}

#[cfg(test)]
impl fmt::Display for SampleLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "sample asset loading failed")?;
        for diagnostic in &self.diagnostics {
            write!(formatter, "\n- {diagnostic}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
impl std::error::Error for SampleLoadError {}

#[cfg(test)]
fn load_pcm_wav(path: &Path, expected_sample_rate_hz: u32) -> Result<LoadedSample, String> {
    let loaded = crate::audio_loading::load_pcm_wav(path, expected_sample_rate_hz)?;
    Ok(LoadedSample::from_loaded_audio(loaded))
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
    fn waveform_reduction_preserves_narrow_signed_transients_and_source_offsets() {
        let sample = LoadedSample::new(48_000, vec![0.0, -0.75, 0.5, 0.0, 0.25]);
        let whole = sample.reduce_waveform(0, 0, 4, 1).unwrap();
        assert_eq!((whole[0].start_frame, whole[0].end_frame), (0, 4));
        assert_eq!((whole[0].minimum, whole[0].maximum), (-0.75, 0.5));

        let selected = sample.reduce_waveform(0, 1, 5, 2).unwrap();
        assert_eq!((selected[0].start_frame, selected[0].end_frame), (1, 3));
        assert_eq!((selected[0].minimum, selected[0].maximum), (-0.75, 0.5));
        assert_eq!((selected[1].start_frame, selected[1].end_frame), (3, 5));
        assert_eq!((selected[1].minimum, selected[1].maximum), (0.0, 0.25));
    }

    #[test]
    fn waveform_reduction_rejects_invalid_or_nonfinite_requests() {
        let sample = LoadedSample::new(48_000, vec![0.0, f32::NAN]);
        assert!(sample.reduce_waveform(0, 0, 2, 1).is_none());
        assert!(sample.reduce_waveform(0, 0, 1, 0).is_none());
        assert!(sample.reduce_waveform(0, 0, 1, 2).is_none());
        assert!(sample.reduce_waveform(1, 0, 1, 1).is_none());
        assert!(sample.reduce_waveform(0, 1, 1, 1).is_none());
        assert!(sample.reduce_waveform(0, 0, 3, 1).is_none());
        let long = LoadedSample::new(48_000, vec![0.0; MAX_WAVEFORM_BUCKETS + 1]);
        assert!(
            long.reduce_waveform(
                0,
                0,
                (MAX_WAVEFORM_BUCKETS + 1) as u64,
                MAX_WAVEFORM_BUCKETS + 1
            )
            .is_none()
        );
    }

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
        assert!(
            error
                .to_string()
                .starts_with("sample asset loading failed\n- sample asset hit")
        );
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
            instrument: None,
            preset_surface: Default::default(),
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
                extra_fields: BTreeMap::new(),
            }],
            connections: vec![],
            voice_allocation: VoiceAllocation::default(),
            parameters: BTreeMap::new(),
            presets: BTreeMap::new(),
            selected_preset: None,
        }
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("dandrum-{name}-{}", std::process::id()))
    }
}
