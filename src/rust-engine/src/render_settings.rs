//! Validated realtime clocks, separate from integer authored/offline inputs.

use crate::patch::RenderSettings;

/// Exact accepted host clock. DSP conversion happens only at an explicit edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SampleRate(f64);

// Construction excludes NaN and both zero representations, so equality is total.
impl Eq for SampleRate {}

impl SampleRate {
    /// Accept finite clocks in `1..=u32::MAX` Hz, retaining the requested f64 value.
    pub fn new(hz: f64) -> Result<Self, UnsupportedSampleRate> {
        if !hz.is_finite() || !(1.0..=f64::from(u32::MAX)).contains(&hz) {
            return Err(UnsupportedSampleRate);
        }
        Ok(Self(hz))
    }

    pub const fn hz(self) -> f64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnsupportedSampleRate;

impl std::fmt::Display for UnsupportedSampleRate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("sample rate must be finite and between 1 and 4294967295 Hz")
    }
}

impl std::error::Error for UnsupportedSampleRate {}

/// One effective configuration after input admission; never integer-rounded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RealtimeRenderSettings {
    pub sample_rate_hz: SampleRate,
    pub block_size_frames: u32,
    pub duration_frames: u64,
}

impl TryFrom<&RenderSettings> for RealtimeRenderSettings {
    type Error = UnsupportedSampleRate;

    fn try_from(settings: &RenderSettings) -> Result<Self, Self::Error> {
        Ok(Self {
            sample_rate_hz: SampleRate::new(f64::from(settings.sample_rate_hz))?,
            block_size_frames: settings.block_size_frames,
            duration_frames: settings.duration_frames,
        })
    }
}
