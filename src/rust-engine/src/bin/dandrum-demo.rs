use std::path::Path;

use dandrum_engine::filter::{BiquadFilter, FilterAlgorithm};
use dandrum_engine::spectral::{SpectralMode, SpectralProcessor};
use dandrum_engine::wav::write_wav_file;

const FS: u32 = 48000;
const TOTAL: usize = (FS as f64 * 14.0) as usize;

struct Saw {
    phase: f64,
}
impl Saw {
    fn new() -> Self { Self { phase: 0.0 } }
    fn process(&mut self, freq_hz: f64) -> f32 {
        self.phase += freq_hz / FS as f64;
        if self.phase >= 1.0 { self.phase -= 1.0; }
        (self.phase * 2.0 - 1.0) as f32
    }
}

fn main() {
    let mut osc = Saw::new();
    let mut lp = BiquadFilter::new_lowpass(0.5, 0.707);
    let mut hp = BiquadFilter::new_highpass(0.001, 0.707);
    let mut pk = BiquadFilter::new_peaking(0.5, 3.0, 0.0);
    let mut gate = SpectralProcessor::new(256, SpectralMode::Gate);

    let mut left = vec![0.0f32; TOTAL];
    let mut right = vec![0.0f32; TOTAL];

    for i in 0..TOTAL {
        let t = i as f64 / FS as f64;
        let raw = 0.3 * osc.process(440.0); // A4 saw wave

        let out = if t < 2.0 {
            // 0-2s: Raw saw wave (reference)
            raw
        } else if t < 4.5 {
            // 2-4.5s: LP exponential sweep 20kHz → 100Hz
            let frac = ((t - 2.0) / 2.5).min(1.0);
            let hz = 20000.0 * (100.0_f64 / 20000.0_f64).powf(frac);
            lp.set_coefficients_lowpass(hz / FS as f64, 0.707);
            lp.process(raw)
        } else if t < 5.5 {
            // 4.5-5.5s: silence gap
            0.0
        } else if t < 8.0 {
            // 5.5-8s: HP exponential sweep 20Hz → 2kHz
            let frac = ((t - 5.5) / 2.5).min(1.0);
            let hz = 20.0 * (2000.0_f64 / 20.0_f64).powf(frac);
            hp.set_coefficients_highpass(hz / FS as f64, 0.707);
            hp.process(raw)
        } else if t < 9.0 {
            // 8-9s: silence gap
            0.0
        } else if t < 11.5 {
            // 9-11.5s: Peaking +18dB sweep 100Hz → 8kHz
            let frac = ((t - 9.0) / 2.5).min(1.0);
            let hz = 100.0 * (8000.0_f64 / 100.0_f64).powf(frac);
            pk.set_coefficients_peaking(hz / FS as f64, 5.0, 18.0);
            pk.process(raw)
        } else if t < 12.0 {
            // 11.5-12s: silence gap
            0.0
        } else {
            // 12-14s: Spectral gate opening
            let frac = (t - 12.0) / 2.0;
            let db = if frac < 0.5 { -80.0 + 80.0 * frac * 2.0 } else { 0.0 };
            gate.set_threshold(db);
            gate.process(raw)
        };

        left[i] = out;
        right[i] = out;
    }

    write_wav_file(Path::new("/tmp/dandrum-demo.wav"), FS, &left, &right).expect("write wav");
    println!("Wrote 14s demo to /tmp/dandrum-demo.wav");
}
