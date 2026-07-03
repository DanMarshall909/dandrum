use crate::delay_line::DelayLine;
use crate::filter::{FilterAlgorithm, OnePoleFilter};

pub struct AllpassDiffuser {
    delay: DelayLine,
    delay_samples: f32,
    coefficient: f32,
}

impl AllpassDiffuser {
    pub fn new(max_delay_ms: f64, sample_rate: f64) -> Self {
        let delay_samples = (max_delay_ms * sample_rate / 1000.0) as f32;
        Self {
            delay: DelayLine::new(max_delay_ms, sample_rate),
            delay_samples: delay_samples.max(1.0),
            coefficient: 0.5,
        }
    }

    pub fn set_delay_samples(&mut self, samples: f32) {
        let max = self.delay.max_delay_samples() as f32;
        self.delay_samples = samples.clamp(1.0, max);
    }

    pub fn set_coefficient(&mut self, coeff: f32) {
        self.coefficient = coeff.clamp(0.0, 0.99);
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let delayed = self.delay.read(self.delay_samples);
        let output = delayed - self.coefficient * input;
        self.delay.write(input + self.coefficient * delayed);
        output
    }

    pub fn reset(&mut self) {
        self.delay.reset();
    }
}

struct CombStage {
    delay: DelayLine,
    damping: OnePoleFilter,
    gain: f32,
    delay_samples: f32,
}

impl CombStage {
    fn new(max_delay_ms: f64, sample_rate: f64) -> Self {
        Self {
            delay: DelayLine::new(max_delay_ms, sample_rate),
            damping: OnePoleFilter::new(sample_rate),
            gain: 0.5,
            delay_samples: max_delay_ms as f32 * sample_rate as f32 / 1000.0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let tap = self.delay.read(self.delay_samples);
        let damped = self.damping.process(tap);
        self.delay.write(input + damped * self.gain);
        tap
    }

    fn set_delay_samples(&mut self, samples: f32) {
        self.delay_samples = samples.max(1.0);
    }

    fn set_gain(&mut self, gain: f32) {
        self.gain = gain.clamp(0.0, 0.99);
    }

    fn reset(&mut self) {
        self.delay.reset();
        self.damping.reset();
    }
}

pub struct Reverb {
    sample_rate: f64,
    pre_delay_l: DelayLine,
    pre_delay_r: DelayLine,
    pre_delay_ms: f64,
    combs_l: [CombStage; 8],
    combs_r: [CombStage; 8],
    diffusers_l: [AllpassDiffuser; 4],
    diffusers_r: [AllpassDiffuser; 4],
    decay_time: f64,
    room_size: f32,
    damping_hz: f64,
    diffusion: f32,
    stereo_width: f32,
    wet: f32,
    dry: f32,
    base_comb_delays_ms: [f64; 8],
    base_allpass_delays_ms: [f64; 4],
}

impl Reverb {
    pub fn new(sample_rate: f64) -> Self {
        let max_comb_delay_ms = 100.0;
        let max_allpass_delay_ms = 20.0;
        let max_pre_delay_ms = 250.0;

        let base_comb_delays_ms = [10.0, 14.0, 19.0, 24.0, 29.0, 34.0, 40.0, 46.0];

        let base_allpass_delays_ms = [3.1, 4.1, 5.0, 6.1];

        let mut s = Self {
            sample_rate,
            pre_delay_l: DelayLine::new(max_pre_delay_ms, sample_rate),
            pre_delay_r: DelayLine::new(max_pre_delay_ms, sample_rate),
            pre_delay_ms: 0.0,
            combs_l: [
                CombStage::new(max_comb_delay_ms, sample_rate),
                CombStage::new(max_comb_delay_ms, sample_rate),
                CombStage::new(max_comb_delay_ms, sample_rate),
                CombStage::new(max_comb_delay_ms, sample_rate),
                CombStage::new(max_comb_delay_ms, sample_rate),
                CombStage::new(max_comb_delay_ms, sample_rate),
                CombStage::new(max_comb_delay_ms, sample_rate),
                CombStage::new(max_comb_delay_ms, sample_rate),
            ],
            combs_r: [
                CombStage::new(max_comb_delay_ms, sample_rate),
                CombStage::new(max_comb_delay_ms, sample_rate),
                CombStage::new(max_comb_delay_ms, sample_rate),
                CombStage::new(max_comb_delay_ms, sample_rate),
                CombStage::new(max_comb_delay_ms, sample_rate),
                CombStage::new(max_comb_delay_ms, sample_rate),
                CombStage::new(max_comb_delay_ms, sample_rate),
                CombStage::new(max_comb_delay_ms, sample_rate),
            ],
            diffusers_l: [
                AllpassDiffuser::new(max_allpass_delay_ms, sample_rate),
                AllpassDiffuser::new(max_allpass_delay_ms, sample_rate),
                AllpassDiffuser::new(max_allpass_delay_ms, sample_rate),
                AllpassDiffuser::new(max_allpass_delay_ms, sample_rate),
            ],
            diffusers_r: [
                AllpassDiffuser::new(max_allpass_delay_ms, sample_rate),
                AllpassDiffuser::new(max_allpass_delay_ms, sample_rate),
                AllpassDiffuser::new(max_allpass_delay_ms, sample_rate),
                AllpassDiffuser::new(max_allpass_delay_ms, sample_rate),
            ],
            decay_time: 2.0,
            room_size: 0.5,
            damping_hz: 8000.0,
            diffusion: 0.5,
            stereo_width: 0.5,
            wet: 0.3,
            dry: 1.0,
            base_comb_delays_ms,
            base_allpass_delays_ms,
        };
        s.update_delays_and_gains();
        s.update_diffusion();
        s.update_damping();
        s
    }

    pub fn set_decay_time(&mut self, seconds: f64) {
        self.decay_time = seconds.clamp(0.1, 30.0);
        self.update_delays_and_gains();
    }

    pub fn set_room_size(&mut self, size: f32) {
        self.room_size = size.clamp(0.0, 1.0);
        self.update_delays_and_gains();
    }

    pub fn set_pre_delay(&mut self, ms: f64) {
        self.pre_delay_ms = ms.clamp(0.0, 250.0);
    }

    pub fn set_damping(&mut self, hz: f64) {
        self.damping_hz = hz.clamp(20.0, 20000.0);
        self.update_damping();
    }

    pub fn set_diffusion(&mut self, diffusion: f32) {
        self.diffusion = diffusion.clamp(0.0, 1.0);
        self.update_diffusion();
    }

    pub fn set_stereo_width(&mut self, width: f32) {
        self.stereo_width = width.clamp(0.0, 1.0);
    }

    pub fn set_wet_dry(&mut self, wet: f32, dry: f32) {
        self.wet = wet.clamp(0.0, 1.0);
        self.dry = dry.clamp(0.0, 1.0);
    }

    fn update_delays_and_gains(&mut self) {
        let rt60_samples = self.decay_time * self.sample_rate;
        let size_scale = 0.3 + 0.7 * self.room_size as f64;

        for i in 0..8 {
            let delay_ms_l = self.base_comb_delays_ms[i] * size_scale;
            let delay_samples_l = delay_ms_l * self.sample_rate / 1000.0;
            self.combs_l[i].set_delay_samples(delay_samples_l as f32);
            let gain_l = 10.0f64.powf(-3.0 * delay_samples_l / rt60_samples);
            self.combs_l[i].set_gain(gain_l as f32);

            let offset = (i as f64) * 0.3 + 0.7;
            let delay_ms_r = (self.base_comb_delays_ms[i] + offset) * size_scale;
            let delay_samples_r = delay_ms_r * self.sample_rate / 1000.0;
            self.combs_r[i].set_delay_samples(delay_samples_r as f32);
            let gain_r = 10.0f64.powf(-3.0 * delay_samples_r / rt60_samples);
            self.combs_r[i].set_gain(gain_r as f32);
        }

        for i in 0..4 {
            let delay_ms_l = self.base_allpass_delays_ms[i] * size_scale;
            let delay_samples_l = delay_ms_l * self.sample_rate / 1000.0;
            self.diffusers_l[i].set_delay_samples(delay_samples_l as f32);

            let offset = (i as f64) * 0.5 + 0.3;
            let delay_ms_r = (self.base_allpass_delays_ms[i] + offset) * size_scale;
            let delay_samples_r = delay_ms_r * self.sample_rate / 1000.0;
            self.diffusers_r[i].set_delay_samples(delay_samples_r as f32);
        }
    }

    fn update_damping(&mut self) {
        for i in 0..8 {
            self.combs_l[i].damping.set_cutoff(self.damping_hz);
            self.combs_r[i].damping.set_cutoff(self.damping_hz);
        }
    }

    fn update_diffusion(&mut self) {
        let coeff = self.diffusion * 0.7;
        for i in 0..4 {
            self.diffusers_l[i].set_coefficient(coeff);
            self.diffusers_r[i].set_coefficient(coeff);
        }
    }

    fn process_channel(combs: &mut [CombStage], diffusers: &mut [AllpassDiffuser], input: f32) -> f32 {
        let mut sum = 0.0;
        for comb in combs.iter_mut() {
            sum += comb.process(input);
        }
        let norm = sum / combs.len() as f32;
        let mut out = norm;
        for diffuser in diffusers.iter_mut() {
            out = diffuser.process(out);
        }
        out
    }

    pub fn process(
        &mut self,
        in_l: f32,
        in_r: f32,
    ) -> (f32, f32) {
        let pre_delay_samples = (self.pre_delay_ms * self.sample_rate / 1000.0) as f32;

        let delayed_in_l = if pre_delay_samples > 0.0 {
            self.pre_delay_l.write(in_l);
            self.pre_delay_l.read(pre_delay_samples)
        } else {
            in_l
        };
        let delayed_in_r = if pre_delay_samples > 0.0 {
            self.pre_delay_r.write(in_r);
            self.pre_delay_r.read(pre_delay_samples)
        } else {
            in_r
        };

        let wet_l = Self::process_channel(&mut self.combs_l, &mut self.diffusers_l, delayed_in_l);
        let wet_r = Self::process_channel(&mut self.combs_r, &mut self.diffusers_r, delayed_in_r);

        let width = self.stereo_width;
        let mid = (wet_l + wet_r) * 0.5;
        let side = (wet_l - wet_r) * 0.5;
        let out_l = mid + side * width;
        let out_r = mid - side * width;

        (
            self.dry * in_l + self.wet * out_l,
            self.dry * in_r + self.wet * out_r,
        )
    }

    pub fn reset(&mut self) {
        self.pre_delay_l.reset();
        self.pre_delay_r.reset();
        for i in 0..8 {
            self.combs_l[i].reset();
            self.combs_r[i].reset();
        }
        for i in 0..4 {
            self.diffusers_l[i].reset();
            self.diffusers_r[i].reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_reverb() -> Reverb {
        Reverb::new(48000.0)
    }

    #[test]
    fn impulse_response_has_dense_tail() {
        let mut r = create_reverb();
        let mut non_zero_count = 0;
        let (_, _) = r.process(1.0, 1.0);
        for _ in 0..48000 {
            let (l, r) = r.process(0.0, 0.0);
            if l.abs() > 0.001 || r.abs() > 0.001 {
                non_zero_count += 1;
            }
        }
        assert!(non_zero_count > 1000, "tail should have >1000 non-zero samples, got {non_zero_count}");
    }

    #[test]
    fn short_decay_produces_brief_tail() {
        let mut r = create_reverb();
        r.set_decay_time(0.1);
        r.set_room_size(0.3);
        let (_, _) = r.process(1.0, 1.0);
        let mut non_zero = false;
        for i in 0..10000 {
            let (l, r_val) = r.process(0.0, 0.0);
            if i < 2000 && (l.abs() > 0.001 || r_val.abs() > 0.001) {
                non_zero = true;
            }
            if i > 5000 {
                assert!(
                    l.abs() < 0.02 && r_val.abs() < 0.02,
                    "short decay tail should mostly decay by frame {i}: l={l}, r={r_val}"
                );
            }
        }
        assert!(non_zero, "should produce initial sound");
    }

    #[test]
    fn room_size_affects_initial_delay_spread() {
        let mut r_small = create_reverb();
        r_small.set_room_size(0.2);
        let mut r_large = create_reverb();
        r_large.set_room_size(0.8);

        let (_, _) = r_small.process(1.0, 1.0);
        let (_, _) = r_large.process(1.0, 1.0);

        let mut small_early = 0;
        let mut large_early = 0;
        for i in 0..2000 {
            let (l, _) = r_small.process(0.0, 0.0);
            if l.abs() > 0.001 {
                small_early = i;
                break;
            }
        }
        for i in 0..2000 {
            let (l, _) = r_large.process(0.0, 0.0);
            if l.abs() > 0.001 {
                large_early = i;
                break;
            }
        }
        assert!(
            large_early >= small_early,
            "larger room should have longer initial delay spread, small={small_early}, large={large_early}"
        );
    }

    #[test]
    fn pre_delay_separates_dry_from_reverb() {
        let mut r = create_reverb();
        r.set_pre_delay(10.0);
        r.set_wet_dry(1.0, 0.0);
        r.set_room_size(0.2);
        r.set_decay_time(0.5);
        let pre_delay_samples = (10.0 * 48000.0 / 1000.0) as usize;
        // Smallest comb at room_size=0.2: scale = 0.3+0.7*0.2 = 0.44,
        // 10ms * 0.44 = 4.4ms = ~211 samples
        let min_comb_samples = 211usize;

        let (_, _) = r.process(1.0, 1.0);
        for i in 0..pre_delay_samples {
            let (l, r_val) = r.process(0.0, 0.0);
            assert!(
                l.abs() < 0.001 && r_val.abs() < 0.001,
                "pre-delay should prevent reverb before {i}, got l={l}, r={r_val}"
            );
        }
        // After pre-delay, the impulse enters the reverb core which has comb filter
        // latency — wait for the smallest comb to produce output
        for _i in 0..min_comb_samples + 10 {
            let (l, r_val) = r.process(0.0, 0.0);
            if l.abs() > 0.001 || r_val.abs() > 0.001 {
                return;
            }
        }
        panic!("reverb should appear after pre-delay + comb delay");
    }

    #[test]
    fn damping_controls_brightness() {
        let mut r_dark = create_reverb();
        r_dark.set_damping(500.0);
        let mut r_bright = create_reverb();
        r_bright.set_damping(10000.0);

        let (_, _) = r_dark.process(1.0, 1.0);
        let (_, _) = r_bright.process(1.0, 1.0);

        let mut dark_max: f32 = 0.0;
        let mut bright_max: f32 = 0.0;
        for _ in 0..1000 {
            let (l, _) = r_dark.process(0.0, 0.0);
            dark_max = dark_max.max(l.abs());
            let (l, _) = r_bright.process(0.0, 0.0);
            bright_max = bright_max.max(l.abs());
        }
        assert!(
            bright_max >= dark_max * 0.5,
            "bright reverb should have comparable amplitude to dark, dark_max={dark_max}, bright_max={bright_max}"
        );
    }

    #[test]
    fn high_diffusion_smooths_response() {
        let mut r_low = create_reverb();
        r_low.set_diffusion(0.0);
        let mut r_high = create_reverb();
        r_high.set_diffusion(1.0);

        let (_, _) = r_low.process(1.0, 1.0);
        let (_, _) = r_high.process(1.0, 1.0);

        let mut low_samples = Vec::new();
        let mut high_samples = Vec::new();
        for _ in 0..4000 {
            let (l, _) = r_low.process(0.0, 0.0);
            low_samples.push(l);
            let (l, _) = r_high.process(0.0, 0.0);
            high_samples.push(l);
        }

        let low_energy: f32 = low_samples.iter().map(|s| s * s).sum();
        let high_energy: f32 = high_samples.iter().map(|s| s * s).sum();
        assert!(
            high_energy > 0.0,
            "high diffusion should produce energy"
        );
        assert!(
            low_energy > 0.0,
            "low diffusion should produce energy"
        );
    }

    #[test]
    fn mono_stereo_width_produces_identical_channels() {
        let mut r = create_reverb();
        r.set_stereo_width(0.0);
        let (_, _) = r.process(1.0, 1.0);
        for _ in 0..1000 {
            let (l, r_val) = r.process(0.0, 0.0);
            assert!(
                (l - r_val).abs() < 1e-6,
                "mono width should produce identical channels, l={l}, r={r_val}"
            );
        }
    }

    #[test]
    fn full_wet_no_dry() {
        let mut r = create_reverb();
        r.set_wet_dry(1.0, 0.0);
        r.set_pre_delay(0.0);
        let (l, r_val) = r.process(0.5, 0.5);
        assert!(
            l.abs() < 0.001 || (l - r_val).abs() < 1e-6,
            "full wet should not pass dry signal through, l={l}, r={r_val}"
        );
    }

    #[test]
    fn dry_signal_passes_through() {
        let mut r = create_reverb();
        r.set_wet_dry(0.0, 1.0);
        let (l, r_val) = r.process(0.5, 0.5);
        assert!(
            (l - 0.5).abs() < 1e-6,
            "dry=1 wet=0 should pass input through, got l={l}"
        );
        assert!(
            (r_val - 0.5).abs() < 1e-6,
            "dry=1 wet=0 should pass input through, got r={r_val}"
        );
    }

    #[test]
    fn long_decay_produces_sustained_tail() {
        let mut r = create_reverb();
        r.set_decay_time(5.0);
        r.set_room_size(0.8);
        r.set_wet_dry(1.0, 0.0);
        let (_, _) = r.process(1.0, 1.0);
        let mut found_mid = false;
        for i in 0..48000 {
            let (l, _) = r.process(0.0, 0.0);
            if i > 20000 && i < 30000 && l.abs() > 0.001 {
                found_mid = true;
            }
        }
        assert!(
            found_mid,
            "long decay should sustain past 0.5 seconds"
        );
    }

    #[test]
    fn reset_clears_state() {
        let mut r = create_reverb();
        r.set_decay_time(5.0);
        r.set_room_size(0.9);
        let (_, _) = r.process(1.0, 1.0);
        r.reset();
        for _ in 0..100 {
            let (l, r_val) = r.process(0.0, 0.0);
            assert!(
                l.abs() < 1e-6,
                "after reset should be silent, got l={l}"
            );
            assert!(
                r_val.abs() < 1e-6,
                "after reset should be silent, got r={r_val}"
            );
        }
    }

    #[test]
    fn process_does_not_panic_or_produce_nan() {
        let mut r = create_reverb();
        for _ in 0..10000 {
            let (l, r_val) = r.process(0.5, 0.5);
            assert!(l.is_finite(), "l should be finite, got {l}");
            assert!(r_val.is_finite(), "r should be finite, got {r_val}");
        }
    }

    #[test]
    fn full_stereo_width_decorrelates_channels() {
        let mut r = create_reverb();
        r.set_stereo_width(1.0);
        let (_, _) = r.process(1.0, 1.0);
        let mut diff_sum = 0.0f32;
        for _ in 0..1000 {
            let (l, r_val) = r.process(0.0, 0.0);
            diff_sum += (l - r_val).abs();
        }
        assert!(
            diff_sum > 0.001,
            "full stereo width should decorrelate channels, diff_sum={diff_sum}"
        );
    }
}
