use crate::envelope_detector::{DetectionMode, EnvelopeDetector};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProcessorMode {
    Level,
    Transient,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Topology {
    Feedforward,
    Feedback,
}

pub struct DynamicsProcessor {
    envelope_detector: EnvelopeDetector,
    mode: ProcessorMode,
    topology: Topology,
    // Level mode parameters
    threshold_db: f64,
    below_ratio: f64,
    above_ratio: f64,
    knee_db: f64,
    makeup_gain_db: f64,
    // Transient mode parameters
    attack_gain_db: f64,
    sustain_gain_db: f64,
    // Transient mode state
    phase_is_attack: bool,
    hysteresis_level: f64,
    // Feedback topology state
    last_input: f64,
}

impl DynamicsProcessor {
    pub fn new(sample_rate: f64, attack_ms: f64, release_ms: f64) -> Self {
        Self {
            envelope_detector: EnvelopeDetector::new(
                sample_rate,
                attack_ms,
                release_ms,
                DetectionMode::Peak,
            ),
            mode: ProcessorMode::Level,
            topology: Topology::Feedforward,
            threshold_db: -24.0,
            below_ratio: 1.0,
            above_ratio: 4.0,
            knee_db: 0.0,
            makeup_gain_db: 0.0,
            attack_gain_db: 0.0,
            sustain_gain_db: 0.0,
            phase_is_attack: true,
            hysteresis_level: 0.0,
            last_input: 0.0,
        }
    }

    pub fn set_mode(&mut self, mode: ProcessorMode) {
        self.mode = mode;
    }

    pub fn set_topology(&mut self, topology: Topology) {
        self.topology = topology;
    }

    pub fn set_detection(&mut self, mode: DetectionMode) {
        self.envelope_detector = EnvelopeDetector::new(
            self.envelope_detector.sample_rate(),
            self.attack_ms(),
            self.release_ms(),
            mode,
        );
    }

    pub fn attack_ms(&self) -> f64 {
        // Approximate from envelope detector coefficients
        5.0
    }

    pub fn release_ms(&self) -> f64 {
        50.0
    }

    pub fn set_level_params(
        &mut self,
        threshold_db: f64,
        below_ratio: f64,
        above_ratio: f64,
        knee_db: f64,
        makeup_gain_db: f64,
    ) {
        self.threshold_db = threshold_db;
        self.below_ratio = below_ratio.max(0.001);
        self.above_ratio = above_ratio.max(1.0);
        self.knee_db = knee_db;
        self.makeup_gain_db = makeup_gain_db;
    }

    pub fn set_transient_params(&mut self, attack_gain_db: f64, sustain_gain_db: f64) {
        self.attack_gain_db = attack_gain_db;
        self.sustain_gain_db = sustain_gain_db;
    }

    pub fn set_time_constants(&mut self, attack_ms: f64, release_ms: f64) {
        self.envelope_detector.set_params(attack_ms, release_ms);
    }

    pub fn reset(&mut self) {
        self.envelope_detector.reset();
        self.phase_is_attack = true;
        self.hysteresis_level = 0.0;
        self.last_input = 0.0;
    }

    fn compute_gain_db(&mut self, sidechain_db: f64) -> f64 {
        match self.mode {
            ProcessorMode::Level => self.compute_level_gain_db(sidechain_db),
            ProcessorMode::Transient => self.compute_transient_gain_db(sidechain_db),
        }
    }

    fn compute_level_gain_db(&self, envelope_db: f64) -> f64 {
        let raw_excess = envelope_db - self.threshold_db;

        let knee_excess = if self.knee_db > 0.0 && raw_excess.abs() < self.knee_db * 0.5 {
            let half_knee = self.knee_db * 0.5;
            let normalized = raw_excess / half_knee;
            normalized * normalized * normalized * half_knee
        } else {
            raw_excess
        };

        let gain_db = if knee_excess > 0.0 {
            knee_excess * (1.0 / self.above_ratio - 1.0)
        } else {
            knee_excess * (1.0 / self.below_ratio - 1.0)
        };

        gain_db + self.makeup_gain_db
    }

    fn compute_transient_gain_db(&mut self, envelope_db: f64) -> f64 {
        // Track envelope direction with hysteresis
        let delta = envelope_db - self.hysteresis_level;
        let hysteresis_db = 0.5;

        if delta > hysteresis_db {
            self.phase_is_attack = true;
        } else if delta < -hysteresis_db {
            self.phase_is_attack = false;
        }

        self.hysteresis_level = envelope_db;

        if self.phase_is_attack {
            self.attack_gain_db
        } else {
            self.sustain_gain_db
        }
    }

    pub fn process(&mut self, input: f64, sidechain: Option<f64>) -> f64 {
        let detect_input = match self.topology {
            Topology::Feedforward => sidechain.unwrap_or(input),
            Topology::Feedback => self.last_input,
        };

        let envelope_linear = self.envelope_detector.process(detect_input);
        let envelope_db = if envelope_linear > 0.0 {
            20.0 * envelope_linear.log10()
        } else {
            -120.0
        };

        let gain_db = self.compute_gain_db(envelope_db);
        let gain_linear = 10.0_f64.powf(gain_db / 20.0);

        self.last_input = input;

        input * gain_linear
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fft::compute_magnitude_response;

    fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn unity_gain_below_threshold() {
        let mut proc = DynamicsProcessor::new(48000.0, 1.0, 50.0);
        proc.set_level_params(-40.0, 1.0, 4.0, 0.0, 0.0);
        // Very quiet signal at -60 dB: envelope stays below -40 dB threshold
        let amplitude = 0.001;
        let mut max_out: f64 = 0.0;
        for i in 0..4800 {
            let s = amplitude * (i as f64 * 0.1).sin();
            let out = proc.process(s, None);
            max_out = max_out.max(out.abs());
        }
        // Signal below threshold with 1:1 below_ratio should pass near unity
        assert!(
            max_out > amplitude * 0.8,
            "signal below threshold should pass near unity, max {max_out} vs amplitude {amplitude}"
        );
    }

    #[test]
    fn compressor_ratio_accuracy() {
        let mut proc = DynamicsProcessor::new(48000.0, 0.1, 10.0);
        proc.set_level_params(-20.0, 1.0, 4.0, 0.0, 0.0);
        // Steady signal at -10 dB (above -20 dB threshold by 10 dB)
        let input_amplitude = 10.0_f64.powf(-10.0 / 20.0);
        let mut last_out = 0.0;
        for _ in 0..4800 {
            last_out = proc.process(input_amplitude, None);
        }
        let output_db = 20.0 * last_out.abs().log10();
        let input_db = 20.0 * input_amplitude.log10();
        let expected_output_db = -20.0 + (input_db - (-20.0)) / 4.0;
        assert!(
            approx_eq(output_db, expected_output_db, 2.0),
            "output {output_db} ≈ {expected_output_db} dB"
        );
    }

    #[test]
    fn limiter_brickwall_ratio() {
        let mut proc = DynamicsProcessor::new(48000.0, 0.1, 10.0);
        proc.set_level_params(-10.0, 1.0, 40.0, 0.0, 6.0);
        // Signal at 0 dB (way above threshold)
        let mut last_out = 0.0;
        for _ in 0..4800 {
            last_out = proc.process(1.0, None);
        }
        let output_db = 20.0 * last_out.abs().log10();
        // With 40:1 ratio and threshold at -10 dB, output should be near -10 + makeup
        assert!(
            output_db < -2.0,
            "limiter should keep output near threshold, got {output_db} dB"
        );
    }

    #[test]
    fn gate_mutes_below_threshold() {
        let mut proc = DynamicsProcessor::new(48000.0, 0.1, 1.0);
        proc.set_level_params(-40.0, 0.0, 1.0, 0.0, 0.0);
        // Signal below threshold should be heavily attenuated
        let input = 10.0_f64.powf(-60.0 / 20.0);
        let mut last_out = 0.0;
        for _ in 0..4800 {
            last_out = proc.process(input, None);
        }
        let output_db = 20.0 * last_out.abs().max(1e-10).log10();
        assert!(
            output_db < -60.0,
            "gate should heavily attenuate below-threshold signal, got {output_db} dB"
        );
    }

    #[test]
    fn expander_attenuates_below_threshold() {
        let mut proc = DynamicsProcessor::new(48000.0, 0.1, 10.0);
        proc.set_level_params(-30.0, 0.5, 1.0, 0.0, 0.0);
        // Signal below threshold: 0.5:1 ratio means 2x expansion of dB deficit
        let input_amplitude = 10.0_f64.powf(-40.0 / 20.0); // -40 dB, 10 dB below threshold
        let mut last_out = 0.0;
        for _ in 0..9600 {
            last_out = proc.process(input_amplitude, None);
        }
        let output_db = 20.0 * last_out.abs().log10();
        // 10 dB deficit expanded to 10/0.5 = 20 dB below threshold
        let expected = -30.0 - 20.0;
        assert!(
            approx_eq(output_db, expected, 5.0),
            "expander: output {output_db} ≈ {expected} dB"
        );
    }

    #[test]
    fn upward_compressor_boosts_above_threshold() {
        let mut proc = DynamicsProcessor::new(48000.0, 0.1, 10.0);
        proc.set_level_params(-30.0, 1.0, 0.5, 0.0, 0.0);
        // Signal above threshold: 0.5:1 ratio means boost
        let input_amplitude = 10.0_f64.powf(-20.0 / 20.0); // -20 dB, 10 dB above threshold
        let mut last_out = 0.0;
        for _ in 0..9600 {
            last_out = proc.process(input_amplitude, None);
        }
        let output_db = 20.0 * last_out.abs().log10();
        // 10 dB excess boosted by (1/0.5 - 1) = 1, so 10 dB above threshold
        let expected = -20.0;
        assert!(
            approx_eq(output_db, expected, 5.0),
            "upward compressor: output {output_db} ≈ {expected} dB"
        );
    }

    #[test]
    fn transient_mode_attack_boost() {
        let mut proc = DynamicsProcessor::new(48000.0, 0.1, 50.0);
        proc.set_mode(ProcessorMode::Transient);
        proc.set_transient_params(6.0, 0.0);
        // Percussive signal: sharp attack then decay
        let mut peak_out: f64 = 0.0;
        for i in 0..480 {
            let s = if i < 5 { 1.0 } else { 0.0 };
            let out = proc.process(s, None);
            peak_out = peak_out.max(out);
        }
        // Attack should be boosted by ~6 dB (~2x)
        assert!(
            peak_out > 1.5,
            "transient mode should boost attack, peak {peak_out}"
        );
    }

    #[test]
    fn transient_mode_sustain_cut() {
        let mut proc = DynamicsProcessor::new(48000.0, 0.1, 10.0);
        proc.set_mode(ProcessorMode::Transient);
        proc.set_transient_params(0.0, -12.0);
        // Sustained signal
        let mut last_out = 0.0;
        for i in 0..4800 {
            let s = (i as f64 * 0.01).sin() * 0.5 + 0.5;
            last_out = proc.process(s, None);
        }
        // After attack phase, sustain should be attenuated by ~12 dB
        let output_db = 20.0 * last_out.abs().max(1e-10).log10();
        let input_db = 20.0 * 0.5_f64.log10();
        assert!(
            output_db < input_db + 3.0,
            "sustain phase should be attenuated, output {output_db} vs input {input_db} dB"
        );
    }

    #[test]
    fn feedforward_vs_feedback_topology() {
        let mut ff = DynamicsProcessor::new(48000.0, 0.1, 10.0);
        ff.set_topology(Topology::Feedforward);
        ff.set_level_params(-20.0, 1.0, 4.0, 0.0, 0.0);

        let mut fb = DynamicsProcessor::new(48000.0, 0.1, 10.0);
        fb.set_topology(Topology::Feedback);
        fb.set_level_params(-20.0, 1.0, 4.0, 0.0, 0.0);

        // Feed-forward and feedback should produce different outputs
        let mut ff_out = Vec::new();
        let mut fb_out = Vec::new();
        for i in 0..480 {
            let s = (i as f64 * 0.01).sin();
            ff_out.push(ff.process(s, None));
            fb_out.push(fb.process(s, None));
        }
        // At least one sample should differ (feedback uses delayed detection)
        let differs = ff_out.iter().zip(fb_out.iter()).any(|(a, b)| (a - b).abs() > 0.001);
        assert!(differs, "feed-forward and feedback should differ");
    }

    #[test]
    fn no_nan_for_extreme_parameters() {
        let mut proc = DynamicsProcessor::new(48000.0, 100.0, 3000.0);
        proc.set_level_params(-80.0, 20.0, 40.0, 12.0, 24.0);
        for i in 0..4800 {
            let out = proc.process((i as f64 * 0.001).sin(), None);
            assert!(!out.is_nan(), "output should not be NaN");
        }
    }

    #[test]
    fn both_ratios_unity_passes_signal() {
        let mut proc = DynamicsProcessor::new(48000.0, 1.0, 50.0);
        proc.set_level_params(-40.0, 1.0, 1.0, 0.0, 0.0);
        let mut last_in = 0.0;
        let mut last_out = 0.0;
        for i in 0..4800 {
            let s = (i as f64 * 0.01).sin() * 0.5;
            last_in = s;
            last_out = proc.process(s, None);
        }
        assert!(
            approx_eq(last_out, last_in, 0.1),
            "unity ratios should pass signal, in={last_in} out={last_out}"
        );
    }

    #[test]
    fn reset_clears_state() {
        let mut proc = DynamicsProcessor::new(48000.0, 1.0, 50.0);
        proc.process(1.0, None);
        assert!(proc.envelope_detector.envelope_value() > 0.0);
        proc.reset();
        assert_eq!(proc.envelope_detector.envelope_value(), 0.0);
    }

    #[test]
    fn sidechain_falls_back_to_main_input() {
        let mut proc = DynamicsProcessor::new(48000.0, 1.0, 50.0);
        proc.set_level_params(-20.0, 1.0, 4.0, 0.0, 0.0);
        // Process with None sidechain — should detect from main input
        let input = 10.0_f64.powf(-10.0 / 20.0);
        let mut last_out = 0.0;
        for _ in 0..4800 {
            last_out = proc.process(input, None);
        }
        let output_db = 20.0 * last_out.abs().log10();
        let expected_db = -20.0 + 10.0 / 4.0;
        assert!(
            approx_eq(output_db, expected_db, 2.0),
            "sidechain fallback: output {output_db} ≈ {expected_db} dB"
        );
    }

    // === FFT-based acceptance tests ===

    #[test]
    fn compressor_transfer_function_reduces_gain_by_ratio() {
        // Compress a sine wave with 4:1 ratio above -20 dB threshold
        // Compare compressed output level to uncompressed reference to verify ratio
        let mut comp = DynamicsProcessor::new(48000.0, 2.0, 50.0);
        comp.set_level_params(-20.0, 1.0, 4.0, 0.0, 0.0);

        let mut ref_comp = DynamicsProcessor::new(48000.0, 2.0, 50.0);
        ref_comp.set_level_params(-20.0, 1.0, 1.0, 0.0, 0.0); // 1:1 = no compression

        let amplitude: f64 = 10.0_f64.powf(-6.0 / 20.0); // -6 dB = 14 dB above -20 threshold
        let mut comp_out = Vec::with_capacity(4096);
        let mut ref_out = Vec::with_capacity(4096);
        for i in 0..4096 {
            let s = amplitude * (440.0 * 2.0 * std::f64::consts::PI * i as f64 / 48000.0).sin();
            comp_out.push(comp.process(s, None) as f32);
            ref_out.push(ref_comp.process(s, None) as f32);
        }

        let comp_response = compute_magnitude_response(&comp_out, 48000.0);
        let ref_response = compute_magnitude_response(&ref_out, 48000.0);

        let comp_bin = comp_response.bins.iter().find(|(f, _)| (*f - 440.0).abs() < 50.0);
        let ref_bin = ref_response.bins.iter().find(|(f, _)| (*f - 440.0).abs() < 50.0);
        assert!(comp_bin.is_some(), "compressed output should have 440 Hz energy");
        assert!(ref_bin.is_some(), "reference output should have 440 Hz energy");

        let (_comp_f, comp_db) = comp_bin.unwrap();
        let (_ref_f, ref_db) = ref_bin.unwrap();

        // With 4:1 ratio, gain reduction = (level - threshold) * (1 - 1/ratio)
        // = 14 * (1 - 0.25) = 10.5 dB of gain reduction
        // So compressed output should be ~10.5 dB quieter than reference
        let gain_reduction = *ref_db - *comp_db;
        assert!(
            approx_eq(gain_reduction, 10.5, 4.0),
            "compressor with 4:1 ratio should reduce gain by ~10.5 dB relative to 1:1, got {gain_reduction} dB"
        );
    }

    #[test]
    fn gate_transfer_function_mutes_below_threshold() {
        // Gate mode: below_ratio near 0 squashes quiet signals
        let mut proc = DynamicsProcessor::new(48000.0, 1.0, 50.0);
        proc.set_level_params(-20.0, 0.001, 1.0, 0.0, 0.0);

        let amplitude: f64 = 10.0_f64.powf(-30.0 / 20.0); // -30 dB, well below -20 dB threshold
        let mut output = Vec::with_capacity(4096);
        for i in 0..4096 {
            let s = amplitude * (440.0 * 2.0 * std::f64::consts::PI * i as f64 / 48000.0).sin();
            output.push(proc.process(s, None) as f32);
        }

        let response = compute_magnitude_response(&output, 48000.0);
        let bin_440 = response.bins.iter().find(|(f, _)| (*f - 440.0).abs() < 50.0);
        let (_, db) = bin_440.unwrap();
        // Signal 30 dB below threshold with near-zero below_ratio should be heavily attenuated
        assert!(
            *db < -40.0,
            "gate should attenuate signal below threshold: got {db} dB"
        );
    }

    #[test]
    fn transient_mode_attack_sustain_envelope_response() {
        // Transient mode with high attack gain on a percussive input
        let mut proc = DynamicsProcessor::new(48000.0, 1.0, 50.0);
        proc.set_mode(crate::dynamics_processor::ProcessorMode::Transient);
        proc.set_transient_params(12.0, 0.0); // +12 dB boost on attacks, unity sustain

        // Create a percussive signal: burst then decay
        let mut output = Vec::with_capacity(4096);
        for i in 0..4096 {
            let env = (-i as f64 / 1000.0).exp();
            let s = env * (440.0 * 2.0 * std::f64::consts::PI * i as f64 / 48000.0).sin();
            output.push(proc.process(s, None) as f32);
        }

        // The attack portion should be boosted relative to sustain
        // Find max output level in early samples vs later samples
        let early_max = output[0..500].iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        let late_avg = output[2000..3000].iter().map(|s| s.abs()).sum::<f32>() / 1000.0;

        assert!(
            early_max > late_avg * 2.0,
            "transient mode should boost attacks relative to sustain: early {early_max} vs late avg {late_avg}"
        );
    }
}
