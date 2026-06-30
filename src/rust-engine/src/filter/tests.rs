use super::{
    BiquadFilter, CombFilter, CombType, FilterAlgorithm, MoogLadder, OnePoleFilter, OnePoleMode,
};

fn run_filter_response<F>(filter: &mut F, sample_rate: f64, impulse_len: usize) -> Vec<(f64, f64)>
where
    F: FilterAlgorithm,
{
    let mut out = vec![0.0f32; impulse_len];
    for i in 0..impulse_len {
        let impulse = if i == 0 { 1.0 } else { 0.0 };
        out[i] = filter.process(impulse);
    }
    filter.reset();
    crate::fft::compute_magnitude_response(&out, sample_rate).bins
}

fn magnitude_at(bins: &[(f64, f64)], target_norm: f64, sample_rate: f64) -> f64 {
    let target_hz = target_norm * sample_rate;
    bins.iter()
        .min_by(|(a, _), (b, _)| {
            (a - target_hz)
                .abs()
                .partial_cmp(&(b - target_hz).abs())
                .unwrap()
        })
        .map(|&(_, db)| db)
        .unwrap_or(-100.0)
}

#[test]
fn biquad_lowpass_attenuates_high_frequencies() {
    let mut filter = BiquadFilter::new_lowpass(0.1, 0.707);
    let response = run_filter_response(&mut filter, 48000.0, 8192);
    let passband_db = magnitude_at(&response, 0.1, 48000.0);
    let stopband_db = magnitude_at(&response, 0.4, 48000.0);
    assert!(
        stopband_db < passband_db - 12.0,
        "lowpass: passband {passband_db:.1} dB, stopband {stopband_db:.1} dB (expected ≥12 dB difference)"
    );
}

#[test]
fn biquad_highpass_attenuates_low_frequencies() {
    let mut filter = BiquadFilter::new_highpass(0.3, 0.707);
    let response = run_filter_response(&mut filter, 48000.0, 8192);
    let passband_db = magnitude_at(&response, 0.4, 48000.0);
    let stopband_db = magnitude_at(&response, 0.05, 48000.0);
    assert!(
        stopband_db < passband_db - 12.0,
        "highpass: passband {passband_db:.1} dB, stopband {stopband_db:.1} dB"
    );
}

#[test]
fn biquad_peaking_boosts_at_center_frequency() {
    let mut filter = BiquadFilter::new_peaking(0.1, 2.0, 12.0);
    let response = run_filter_response(&mut filter, 48000.0, 8192);
    let peak_db = magnitude_at(&response, 0.1, 48000.0);
    assert!(
        peak_db > 6.0,
        "peaking boost: expected >6 dB, got {peak_db:.1} dB"
    );
}

#[test]
fn biquad_peaking_cuts_at_center_frequency() {
    let mut filter = BiquadFilter::new_peaking(0.1, 2.0, -12.0);
    let response = run_filter_response(&mut filter, 48000.0, 8192);
    let cut_db = magnitude_at(&response, 0.1, 48000.0);
    assert!(
        cut_db < -6.0,
        "peaking cut: expected < -6 dB, got {cut_db:.1} dB"
    );
}

#[test]
fn biquad_resonance_creates_peak() {
    let mut low_q = BiquadFilter::new_lowpass(0.1, 0.707);
    let mut high_q = BiquadFilter::new_lowpass(0.1, 5.0);
    let low_resp = run_filter_response(&mut low_q, 48000.0, 8192);
    let high_resp = run_filter_response(&mut high_q, 48000.0, 8192);
    let low_db = magnitude_at(&low_resp, 0.1, 48000.0);
    let high_db = magnitude_at(&high_resp, 0.1, 48000.0);
    assert!(
        high_db > low_db + 3.0,
        "resonance peak: high Q {high_db:.1} dB should be > low Q {low_db:.1} dB + 3 dB"
    );
}

#[test]
fn biquad_no_nan_at_extreme_cutoff() {
    let mut filter = BiquadFilter::new_lowpass(0.0, 0.707);
    for _ in 0..100 {
        let out = filter.process(1.0);
        assert!(!out.is_nan(), "output should not be NaN at cutoff=0");
    }
}

#[test]
fn moog_resonance_emphasizes_cutoff() {
    let mut low_res = MoogLadder::new(48000.0);
    low_res.set_cutoff(1000.0);
    low_res.set_resonance(0.0);
    let mut high_res = MoogLadder::new(48000.0);
    high_res.set_cutoff(1000.0);
    high_res.set_resonance(0.8);

    let low_response = run_filter_response(&mut low_res, 48000.0, 8192);
    let high_response = run_filter_response(&mut high_res, 48000.0, 8192);

    let low_db = magnitude_at(&low_response, 1000.0 / 48000.0, 48000.0);
    let high_db = magnitude_at(&high_response, 1000.0 / 48000.0, 48000.0);
    assert!(
        high_db > low_db + 2.0,
        "moog resonance: high res {high_db:.1} dB vs low res {low_db:.1} dB"
    );
}

#[test]
fn moog_no_nan_at_extreme_resonance() {
    let mut filter = MoogLadder::new(48000.0);
    filter.set_cutoff(1000.0);
    filter.set_resonance(0.99);
    for _ in 0..1000 {
        let out = filter.process(1.0);
        assert!(out.is_finite(), "output should be finite at high resonance");
    }
}

#[test]
fn comb_feedforward_produces_notches() {
    let delay_samples = 96;
    let mut filter = CombFilter::new(delay_samples, 0.7, CombType::Feedforward);
    let response = run_filter_response(&mut filter, 48000.0, 8192);
    let peak_norm = 1.0 / delay_samples as f64;
    let notch_norm = 0.5 / delay_samples as f64;

    let peak_db = magnitude_at(&response, peak_norm, 48000.0);
    let notch_db = magnitude_at(&response, notch_norm, 48000.0);
    assert!(
        peak_db > notch_db + 3.0,
        "comb feedforward: expected peak > notch"
    );
}

#[test]
fn comb_feedback_produces_peaks() {
    let delay_samples = 96;
    let mut filter = CombFilter::new(delay_samples, 0.7, CombType::Feedback);
    let response = run_filter_response(&mut filter, 48000.0, 8192);
    let peak_norm = 1.0 / delay_samples as f64;
    let notch_norm = 0.5 / delay_samples as f64;

    let peak_db = magnitude_at(&response, peak_norm, 48000.0);
    let notch_db = magnitude_at(&response, notch_norm, 48000.0);
    assert!(
        peak_db > notch_db + 3.0,
        "comb feedback: expected peak > notch"
    );
}

#[test]
fn comb_gain_stability() {
    let mut filter = CombFilter::new(100, 0.99, CombType::Feedback);
    for _ in 0..10000 {
        let out = filter.process(1.0);
        assert!(
            out.is_finite(),
            "comb feedback should be bounded at max gain"
        );
    }
}

#[test]
fn comb_no_nan() {
    let mut filter = CombFilter::new(100, 0.0, CombType::Feedback);
    for _ in 0..1000 {
        let out = filter.process(0.0);
        assert!(!out.is_nan(), "comb output should not be NaN");
    }
}

#[test]
fn one_pole_lowpass_dc_gain_is_unity() {
    let mut filter = OnePoleFilter::new(48000.0);
    filter.set_cutoff(1000.0);
    filter.process(1.0);
    let mut y = 0.0f32;
    for _ in 0..1000 {
        y = filter.process(1.0);
    }
    assert!((y - 1.0).abs() < 0.01, "DC gain should be ~1.0, got {y}");
}

#[test]
fn one_pole_lowpass_attenuates_high_frequencies() {
    let mut filter = OnePoleFilter::new(48000.0);
    filter.set_cutoff(1000.0);
    let response = run_filter_response(&mut filter, 48000.0, 8192);
    let passband_db = magnitude_at(&response, 0.02, 48000.0);
    let stopband_db = magnitude_at(&response, 0.4, 48000.0);
    assert!(
        stopband_db < passband_db - 3.0,
        "one-pole LP: passband {passband_db:.1} dB, stopband {stopband_db:.1} dB"
    );
}

#[test]
fn one_pole_highpass_attenuates_low_frequencies() {
    let mut filter = OnePoleFilter::new(48000.0);
    filter.set_mode(OnePoleMode::Highpass);
    filter.set_cutoff(1000.0);
    let response = run_filter_response(&mut filter, 48000.0, 8192);
    let passband_db = magnitude_at(&response, 0.4, 48000.0);
    let stopband_db = magnitude_at(&response, 0.01, 48000.0);
    assert!(
        stopband_db < passband_db - 6.0,
        "one-pole HP: passband {passband_db:.1} dB, stopband {stopband_db:.1} dB"
    );
}

#[test]
fn one_pole_no_nan_at_extreme_cutoff() {
    let mut filter = OnePoleFilter::new(48000.0);
    filter.set_cutoff(20.0);
    for _ in 0..100 {
        let out = filter.process(1.0);
        assert!(out.is_finite(), "output should be finite at 20 Hz cutoff");
    }
    filter.set_cutoff(20000.0);
    for _ in 0..100 {
        let out = filter.process(1.0);
        assert!(out.is_finite(), "output should be finite at 20 kHz cutoff");
    }
}

#[test]
fn one_pole_reset_clears_state() {
    let mut filter = OnePoleFilter::new(48000.0);
    filter.process(1.0);
    filter.process(1.0);
    filter.reset();
    let out = filter.process(0.0);
    assert!(
        out.abs() < 1e-6,
        "after reset with zero input, expected 0, got {out}"
    );
}
