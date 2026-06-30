use crate::delay_line::DelayLine;
use crate::filter::{FilterAlgorithm, OnePoleFilter};

pub struct Echo {
    delay_l: DelayLine,
    delay_r: DelayLine,
    damp_l: OnePoleFilter,
    damp_r: OnePoleFilter,
    delay_ms_l: f64,
    delay_ms_r: f64,
    feedback: f32,
    wet: f32,
    dry: f32,
    ping_pong: bool,
    sample_rate: f64,
    sync_division: Option<SyncDivision>,
    bpm: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SyncDivision {
    Whole,
    Half,
    Quarter,
    QuarterTriplet,
    Eighth,
    EighthTriplet,
    Sixteenth,
    SixteenthTriplet,
    ThirtySecond,
}

impl SyncDivision {
    pub fn beat_fraction(&self) -> f64 {
        match self {
            SyncDivision::Whole => 4.0,
            SyncDivision::Half => 2.0,
            SyncDivision::Quarter => 1.0,
            SyncDivision::QuarterTriplet => 1.0 / 3.0,
            SyncDivision::Eighth => 0.5,
            SyncDivision::EighthTriplet => 0.5 / 3.0,
            SyncDivision::Sixteenth => 0.25,
            SyncDivision::SixteenthTriplet => 0.25 / 3.0,
            SyncDivision::ThirtySecond => 0.125,
        }
    }
}

impl Echo {
    pub fn new(sample_rate: f64) -> Self {
        let max_delay_ms = 2000.0;
        Self {
            delay_l: DelayLine::new(max_delay_ms, sample_rate),
            delay_r: DelayLine::new(max_delay_ms, sample_rate),
            damp_l: OnePoleFilter::new(sample_rate),
            damp_r: OnePoleFilter::new(sample_rate),
            delay_ms_l: 200.0,
            delay_ms_r: 200.0,
            feedback: 0.5,
            wet: 0.5,
            dry: 1.0,
            ping_pong: false,
            sample_rate,
            sync_division: None,
            bpm: 120.0,
        }
    }

    pub fn set_delay_ms(&mut self, left_ms: f64, right_ms: f64) {
        self.delay_ms_l = left_ms.max(1.0).min(2000.0);
        self.delay_ms_r = right_ms.max(1.0).min(2000.0);
    }

    pub fn set_feedback(&mut self, fb: f32) {
        self.feedback = fb.clamp(0.0, 0.99);
    }

    pub fn set_damping_cutoff(&mut self, hz: f64) {
        self.damp_l.set_cutoff(hz);
        self.damp_r.set_cutoff(hz);
    }

    pub fn set_wet_dry(&mut self, wet: f32, dry: f32) {
        self.wet = wet.clamp(0.0, 1.0);
        self.dry = dry.clamp(0.0, 1.0);
    }

    pub fn set_ping_pong(&mut self, enabled: bool) {
        self.ping_pong = enabled;
    }

    pub fn set_sync(&mut self, division: Option<SyncDivision>) {
        self.sync_division = division;
    }

    pub fn set_bpm(&mut self, bpm: f64) {
        self.bpm = bpm.max(1.0);
    }

    fn delay_samples_from_ms(&self, ms: f64) -> f32 {
        (ms * self.sample_rate / 1000.0) as f32
    }

    fn delay_samples_from_sync(&self) -> f32 {
        let Some(div) = self.sync_division else {
            return 0.0;
        };
        let beats = div.beat_fraction();
        let seconds_per_beat = 60.0 / self.bpm;
        let delay_s = seconds_per_beat * beats;
        (delay_s * self.sample_rate) as f32
    }

    pub fn process(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        let delay_samples_l = self
            .sync_division
            .map(|_| self.delay_samples_from_sync())
            .unwrap_or_else(|| self.delay_samples_from_ms(self.delay_ms_l));

        let delay_samples_r = self
            .sync_division
            .map(|_| self.delay_samples_from_sync())
            .unwrap_or_else(|| self.delay_samples_from_ms(self.delay_ms_r));

        let tap_l = self.delay_l.read(delay_samples_l);
        let tap_r = self.delay_r.read(delay_samples_r);

        let damped_l = self.damp_l.process(tap_l);
        let damped_r = self.damp_r.process(tap_r);

        if self.ping_pong {
            self.delay_l.write(in_l + damped_r * self.feedback);
            self.delay_r.write(in_r + damped_l * self.feedback);
        } else {
            self.delay_l.write(in_l + damped_l * self.feedback);
            self.delay_r.write(in_r + damped_r * self.feedback);
        }

        let out_l = self.dry * in_l + self.wet * if self.ping_pong { tap_r } else { tap_l };
        let out_r = self.dry * in_r + self.wet * if self.ping_pong { tap_l } else { tap_r };

        (out_l, out_r)
    }

    pub fn reset(&mut self) {
        self.delay_l.reset();
        self.delay_r.reset();
        self.damp_l.reset();
        self.damp_r.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impulse_produces_repeat_at_delay_time() {
        let mut e = Echo::new(48000.0);
        e.set_delay_ms(10.0, 10.0);
        e.set_damping_cutoff(24000.0);
        e.set_wet_dry(1.0, 0.0);
        e.set_feedback(0.0);
        let delay_samples = (10.0 * 48000.0 / 1000.0) as usize;

        // Write impulse
        let (_, _) = e.process(1.0, 1.0);

        // Read before delay — should be silent
        for _ in 0..delay_samples {
            let (l, r) = e.process(0.0, 0.0);
            assert!((l).abs() < 1e-6, "expected silence before delay, got {l}");
            assert!((r).abs() < 1e-6, "expected silence before delay, got {r}");
        }

        // Read at delay — should get the impulse (attenuated by one-pole damping filter)
        let (l, r) = e.process(0.0, 0.0);
        assert!(l.abs() > 0.5, "expected signal at delay, got {l}");
        assert!(r.abs() > 0.5, "expected signal at delay, got {r}");
    }

    #[test]
    fn feedback_produces_decaying_repeats() {
        let mut e = Echo::new(48000.0);
        e.set_delay_ms(5.0, 5.0);
        e.set_damping_cutoff(24000.0);
        e.set_wet_dry(1.0, 0.0);
        e.set_feedback(0.5);
        let delay_samples = (5.0 * 48000.0 / 1000.0) as usize;

        let (_, _) = e.process(1.0, 1.0);

        // Advance to first repeat
        for _ in 0..delay_samples {
            let (_, _) = e.process(0.0, 0.0);
        }
        let (first_l, _) = e.process(0.0, 0.0);
        assert!(
            first_l.abs() > 0.01,
            "expected non-zero first repeat, got {first_l}"
        );

        // Advance to second repeat
        for _ in 0..delay_samples {
            let (_, _) = e.process(0.0, 0.0);
        }
        let (second_l, _) = e.process(0.0, 0.0);
        assert!(
            second_l.abs() < first_l.abs() * 0.99,
            "second repeat {second_l} should be quieter than first {first_l}"
        );
    }

    #[test]
    fn feedback_zero_produces_single_repeat() {
        let mut e = Echo::new(48000.0);
        e.set_delay_ms(5.0, 5.0);
        e.set_damping_cutoff(24000.0);
        e.set_wet_dry(1.0, 0.0);
        e.set_feedback(0.0);
        let delay_samples = (5.0 * 48000.0 / 1000.0) as usize;

        let (_, _) = e.process(1.0, 1.0);

        // Advance past delay
        for _ in 0..delay_samples {
            let (_, _) = e.process(0.0, 0.0);
        }
        // Read and discard first repeat
        let (first_l, _) = e.process(0.0, 0.0);
        assert!(
            first_l.abs() > 0.01,
            "expected non-zero first repeat, got {first_l}"
        );

        // Second repeat should be zero (feedback=0 means only one echo)
        for _ in 0..delay_samples * 2 {
            let (l, r) = e.process(0.0, 0.0);
            assert!(
                (l).abs() < 1e-6,
                "expected silence after single repeat, got {l}"
            );
            assert!(
                (r).abs() < 1e-6,
                "expected silence after single repeat, got {r}"
            );
        }
    }

    #[test]
    fn ping_pong_alternates_channels() {
        let mut e = Echo::new(48000.0);
        e.set_delay_ms(5.0, 5.0);
        e.set_wet_dry(1.0, 0.0);
        e.set_feedback(1.0);
        e.set_ping_pong(true);
        let delay_samples = (5.0 * 48000.0 / 1000.0) as usize;

        // Send impulse to left only
        let (_, _) = e.process(1.0, 0.0);

        // Advance past delay to the first repeat
        for _ in 0..delay_samples {
            let (_, _) = e.process(0.0, 0.0);
        }
        let (l, r) = e.process(0.0, 0.0);
        // First repeat should appear on right channel (ping-pong cross)
        assert!(
            (l).abs() < 1e-6,
            "first ping-pong repeat should be on R, got L={l}"
        );
        assert!(
            r.abs() > 0.5,
            "first ping-pong repeat expected on R, got {r}"
        );
    }

    #[test]
    fn damping_filter_darkens_repeats() {
        let mut e = Echo::new(48000.0);
        e.set_delay_ms(5.0, 5.0);
        e.set_wet_dry(1.0, 0.0);
        e.set_feedback(0.8);
        e.set_damping_cutoff(500.0);

        let (_, _) = e.process(1.0, 1.0);

        // Run a few repeats — should not blow up
        for _ in 0..500 {
            let (l, r) = e.process(0.0, 0.0);
            assert!(l.is_finite(), "damped echo should remain stable");
            assert!(r.is_finite(), "damped echo should remain stable");
        }
    }

    #[test]
    fn tempo_sync_computes_correct_delay() {
        let mut e = Echo::new(48000.0);
        e.set_bpm(480.0);
        e.set_sync(Some(SyncDivision::ThirtySecond));
        e.set_damping_cutoff(24000.0);
        e.set_wet_dry(1.0, 0.0);
        e.set_feedback(0.0);

        // 480 BPM, 32nd note: 60/480 * 0.125 = 0.015625s → 750 samples at 48kHz
        let expected_delay = 750;

        let (_, _) = e.process(1.0, 1.0);

        for i in 0..expected_delay {
            let (l, r) = e.process(0.0, 0.0);
            assert!(
                (l).abs() < 1e-6,
                "expected silence before sync delay at sample {i}, got {l}"
            );
            assert!(
                (r).abs() < 1e-6,
                "expected silence before sync delay at sample {i}, got {r}"
            );
        }

        let (l, r) = e.process(0.0, 0.0);
        assert!(l.abs() > 0.5, "expected signal at sync delay, got {l}");
        assert!(r.abs() > 0.5, "expected signal at sync delay, got {r}");
    }

    #[test]
    fn wet_dry_mix() {
        let mut e = Echo::new(48000.0);
        e.set_delay_ms(10.0, 10.0);
        e.set_feedback(0.0);
        e.set_wet_dry(0.0, 1.0);

        // Full dry: output should be input, no delayed signal
        let (l, r) = e.process(0.5, 0.5);
        assert!(
            (l - 0.5).abs() < 1e-6,
            "dry signal should pass through, got {l}"
        );
        assert!(
            (r - 0.5).abs() < 1e-6,
            "dry signal should pass through, got {r}"
        );
    }

    #[test]
    fn zero_delay_does_not_panic() {
        let mut e = Echo::new(48000.0);
        e.set_delay_ms(1.0, 1.0); // minimum 1ms
        for _ in 0..100 {
            let (l, r) = e.process(0.5, 0.5);
            assert!(l.is_finite(), "echo should be stable at min delay");
            assert!(r.is_finite(), "echo should be stable at min delay");
        }
    }

    #[test]
    fn max_feedback_stable() {
        let mut e = Echo::new(48000.0);
        e.set_delay_ms(10.0, 10.0);
        e.set_feedback(0.99);
        for _ in 0..1000 {
            let (l, r) = e.process(0.0, 0.0);
            assert!(l.is_finite(), "echo should be stable at max feedback");
            assert!(r.is_finite(), "echo should be stable at max feedback");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut e = Echo::new(48000.0);
        e.set_delay_ms(5.0, 5.0);
        e.set_wet_dry(1.0, 0.0);
        e.set_feedback(0.9);

        let (_, _) = e.process(1.0, 1.0);
        e.reset();

        // After reset, should be silent regardless of feedback
        for _ in 0..100 {
            let (l, r) = e.process(0.0, 0.0);
            assert!((l).abs() < 1e-6, "after reset expected silence, got {l}");
            assert!((r).abs() < 1e-6, "after reset expected silence, got {r}");
        }
    }

    #[test]
    fn sync_division_beat_fractions() {
        assert!((SyncDivision::Quarter.beat_fraction() - 1.0).abs() < 1e-6);
        assert!((SyncDivision::Eighth.beat_fraction() - 0.5).abs() < 1e-6);
        assert!((SyncDivision::SixteenthTriplet.beat_fraction() - 0.25 / 3.0).abs() < 1e-6);
    }
}
