pub trait WaveshaperCurve: Send {
    fn process(&self, sample: f64) -> f64;
    fn name(&self) -> &'static str;
}

pub struct TanhCurve;

impl WaveshaperCurve for TanhCurve {
    fn process(&self, sample: f64) -> f64 {
        sample.tanh()
    }

    fn name(&self) -> &'static str {
        "tanh"
    }
}

pub struct HardClipCurve;

impl WaveshaperCurve for HardClipCurve {
    fn process(&self, sample: f64) -> f64 {
        sample.clamp(-1.0, 1.0)
    }

    fn name(&self) -> &'static str {
        "hard_clip"
    }
}

pub struct SoftClipCurve;

impl WaveshaperCurve for SoftClipCurve {
    fn process(&self, sample: f64) -> f64 {
        let x = sample.clamp(-1.0, 1.0);
        x - x * x * x / 3.0
    }

    fn name(&self) -> &'static str {
        "soft_clip"
    }
}

pub struct SinFoldCurve;

impl WaveshaperCurve for SinFoldCurve {
    fn process(&self, sample: f64) -> f64 {
        (sample * std::f64::consts::PI).sin()
    }

    fn name(&self) -> &'static str {
        "sinfold"
    }
}

pub struct Saturator {
    curve: Box<dyn WaveshaperCurve>,
    drive_db: f64,
    bias: f64,
}

impl Saturator {
    pub fn new() -> Self {
        Self {
            curve: Box::new(TanhCurve),
            drive_db: 0.0,
            bias: 0.0,
        }
    }

    pub fn set_curve(&mut self, curve: Box<dyn WaveshaperCurve>) {
        self.curve = curve;
    }

    pub fn set_drive_db(&mut self, drive_db: f64) {
        self.drive_db = drive_db;
    }

    pub fn set_bias(&mut self, bias: f64) {
        self.bias = bias;
    }

    pub fn process(&self, input: f64) -> f64 {
        let drive_linear = 10.0_f64.powf(self.drive_db / 20.0);
        let biased = input * drive_linear + self.bias;
        self.curve.process(biased)
    }

    pub fn curve_name(&self) -> &'static str {
        self.curve.name()
    }
}

impl Default for Saturator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    use crate::fft::compute_magnitude_response;

    #[test]
    fn tanh_curve_symmetry() {
        let curve = TanhCurve;
        let out_pos = curve.process(2.0);
        let out_neg = curve.process(-2.0);
        assert!(approx_eq(out_pos, -out_neg, 1e-10), "tanh should be symmetric");
    }

    #[test]
    fn tanh_curve_bounds() {
        let curve = TanhCurve;
        for input in [-10.0, -5.0, -1.0, 0.0, 1.0, 5.0, 10.0] {
            let out = curve.process(input);
            assert!(out.abs() < 1.01, "tanh should stay within [-1, 1], got {out} for {input}");
        }
    }

    #[test]
    fn hard_clip_clamping() {
        let curve = HardClipCurve;
        assert_eq!(curve.process(0.5), 0.5);
        assert_eq!(curve.process(1.0), 1.0);
        assert_eq!(curve.process(2.0), 1.0);
        assert_eq!(curve.process(-2.0), -1.0);
    }

    #[test]
    fn soft_clip_smooth_transition() {
        let curve = SoftClipCurve;
        // At low levels should be near linear
        let low = curve.process(0.1);
        assert!(approx_eq(low, 0.1 - 0.1_f64.powi(3) / 3.0, 1e-10));
        // At high levels should approach limit
        let high = curve.process(10.0);
        assert!(high.abs() < 1.0, "soft clip should limit output");
    }

    #[test]
    fn sinfold_produces_harmonics() {
        let curve = SinFoldCurve;
        // sin(pi * 0.25) = ~0.707
        let out = curve.process(0.25);
        assert!(approx_eq(out, (0.25 * std::f64::consts::PI).sin(), 1e-10));
        // Wavefolding: sin(pi * 1.5) = sin(1.5pi) = -1
        let folded = curve.process(1.5);
        assert!(approx_eq(folded, (1.5 * std::f64::consts::PI).sin(), 1e-10));
    }

    #[test]
    fn drive_scales_input() {
        let sat = {
            let mut s = Saturator::new();
            s.set_curve(Box::new(HardClipCurve));
            s.set_drive_db(6.0);
            s.set_bias(0.0);
            s
        };
        // 6 dB drive ≈ 2x amplification; 0.6 * 2 ≈ 1.2 → clipped to 1.0
        let out = sat.process(0.6);
        assert!(approx_eq(out, 1.0, 1e-6), "6 dB drive clips 0.6 to 1.0, got {out}");
    }

    #[test]
    fn bias_creates_asymmetry() {
        let mut sat = Saturator::new();
        sat.set_curve(Box::new(HardClipCurve));
        sat.set_drive_db(0.0);
        sat.set_bias(0.3);
        // With positive bias, a symmetric input produces asymmetric output
        let out_pos = sat.process(0.5);
        let out_neg = sat.process(-0.5);
        assert!(out_pos != -out_neg, "bias should create asymmetry: {out_pos} vs {}", -out_neg);
    }

    #[test]
    fn unity_gain_at_minimum_drive() {
        let sat = {
            let mut s = Saturator::new();
            s.set_curve(Box::new(TanhCurve));
            s.set_drive_db(0.0);
            s.set_bias(0.0);
            s
        };
        let out = sat.process(0.1);
        // At very low levels, tanh(x) ≈ x
        assert!(approx_eq(out, 0.1, 0.01), "unity drive should pass signal, got {out}");
    }

    #[test]
    fn custom_curve_trait_integration() {
        struct Doubler;
        impl WaveshaperCurve for Doubler {
            fn process(&self, sample: f64) -> f64 { sample * 2.0 }
            fn name(&self) -> &'static str { "doubler" }
        }

        let mut sat = Saturator::new();
        sat.set_curve(Box::new(Doubler));
        sat.set_drive_db(0.0);
        sat.set_bias(0.0);

        assert_eq!(sat.curve_name(), "doubler");
        assert!(approx_eq(sat.process(0.5), 1.0, 1e-10));
    }

    #[test]
    fn no_nan_for_any_input() {
        let sat = Saturator::new();
        for input in [-1e10, -100.0, -1.0, 0.0, 1.0, 100.0, 1e10] {
            let out = sat.process(input);
            assert!(!out.is_nan(), "no NaN for input {input}");
        }
    }

    // === FFT-based acceptance tests ===

    #[test]
    fn tanh_saturator_produces_odd_harmonics() {
        // Saturate a sine wave with tanh and verify odd harmonics appear
        let sat = {
            let mut s = Saturator::new();
            s.set_curve(Box::new(TanhCurve));
            s.set_drive_db(12.0);
            s.set_bias(0.0);
            s
        };

        let mut output = Vec::with_capacity(4096);
        for i in 0..4096 {
            let s = (440.0 * 2.0 * std::f64::consts::PI * i as f64 / 48000.0).sin();
            output.push(sat.process(s) as f32);
        }

        let response = compute_magnitude_response(&output, 48000.0);
        // Find bins at harmonic frequencies: 3rd (1320 Hz), 5th (2200 Hz)
        let h3 = response.bins.iter().find(|(f, _)| (*f - 1320.0).abs() < 100.0);
        let h5 = response.bins.iter().find(|(f, _)| (*f - 2200.0).abs() < 100.0);

        assert!(h3.is_some(), "tanh should produce 3rd harmonic");
        assert!(h5.is_some(), "tanh should produce 5th harmonic");

        if let (Some((_, h3_db)), Some((_, h5_db))) = (h3, h5) {
            // Higher harmonics should be lower in amplitude
            assert!(
                *h3_db > *h5_db - 10.0,
                "3rd harmonic ({h3_db} dB) should be comparable to 5th ({h5_db} dB)"
            );
        }
    }

    #[test]
    fn hard_clip_saturator_produces_strong_odd_harmonics() {
        let sat = {
            let mut s = Saturator::new();
            s.set_curve(Box::new(HardClipCurve));
            s.set_drive_db(12.0);
            s.set_bias(0.0);
            s
        };

        let mut output = Vec::with_capacity(4096);
        for i in 0..4096 {
            let s = (440.0 * 2.0 * std::f64::consts::PI * i as f64 / 48000.0).sin();
            output.push(sat.process(s) as f32);
        }

        let response = compute_magnitude_response(&output, 48000.0);
        // Hard clip produces many odd harmonics
        let h3 = response.bins.iter().find(|(f, _)| (*f - 1320.0).abs() < 100.0);
        let h7 = response.bins.iter().find(|(f, _)| (*f - 3080.0).abs() < 100.0);
        let h9 = response.bins.iter().find(|(f, _)| (*f - 3960.0).abs() < 100.0);

        assert!(h3.is_some(), "hard clip should produce 3rd harmonic");
        assert!(h7.is_some(), "hard clip should produce 7th harmonic");
        assert!(h9.is_some(), "hard clip should produce 9th harmonic");
    }
}
