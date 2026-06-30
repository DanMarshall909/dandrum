use num_complex::Complex;
use rustfft::FftPlanner;

pub struct SpectralProcessor {
    frame_size: usize,
    hop_size: usize,
    window: Vec<f64>,
    input_buf: Vec<f64>,
    output_buf: Vec<f64>,
    write_pos: usize,
    read_pos: usize,
    frames_processed: usize,
    mode: SpectralMode,
    threshold_db: f64,
}

pub enum SpectralMode {
    Passthrough,
    Gate,
}

impl SpectralProcessor {
    pub fn new(frame_size: usize, mode: SpectralMode) -> Self {
        let hop_size = frame_size / 2;
        let window = Self::hann_window(frame_size);
        Self {
            frame_size,
            hop_size,
            window,
            input_buf: vec![0.0; frame_size],
            output_buf: vec![0.0; frame_size * 2],
            write_pos: 0,
            read_pos: 0,
            frames_processed: 0,
            mode,
            threshold_db: -40.0,
        }
    }

    pub fn set_threshold(&mut self, threshold_db: f64) {
        self.threshold_db = threshold_db;
    }

    fn hann_window(size: usize) -> Vec<f64> {
        (0..size)
            .map(|i| 0.5 * (1.0 - (std::f64::consts::TAU * i as f64 / size as f64).cos()))
            .collect()
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let idx = self.write_pos % self.frame_size;
        self.input_buf[idx] = input as f64;
        self.write_pos += 1;

        if self.write_pos >= self.frame_size
            && (self.write_pos - self.frame_size) % self.hop_size == 0
        {
            self.process_frame();
            self.frames_processed += 1;
        }

        if self.read_pos < self.write_pos && self.frames_processed > 0 {
            let out_idx = self.read_pos % (self.frame_size * 2);
            let sample = self.output_buf[out_idx];
            self.output_buf[out_idx] = 0.0;
            self.read_pos += 1;
            sample as f32
        } else {
            0.0
        }
    }

    fn process_frame(&mut self) {
        let start = self.write_pos % self.frame_size;
        let mut buffer: Vec<Complex<f64>> = (0..self.frame_size)
            .map(|i| {
                let idx = (start + i) % self.frame_size;
                Complex::new(self.input_buf[idx] * self.window[i], 0.0)
            })
            .collect();

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(self.frame_size);
        fft.process(&mut buffer);

        match self.mode {
            SpectralMode::Passthrough => {}
            SpectralMode::Gate => {
                let threshold_linear = 10.0_f64.powf(self.threshold_db / 20.0);
                for bin in buffer.iter_mut() {
                    let mag = bin.norm();
                    if mag < threshold_linear {
                        *bin = Complex::new(0.0, 0.0);
                    }
                }
            }
        }

        let ifft = planner.plan_fft_inverse(self.frame_size);
        ifft.process(&mut buffer);

        let out_offset = (self.write_pos - self.frame_size) % (self.frame_size * 2);

        for i in 0..self.frame_size {
            let idx = (out_offset + i) % (self.frame_size * 2);
            self.output_buf[idx] += buffer[i].re / self.frame_size as f64;
        }
    }

    pub fn reset(&mut self) {
        self.input_buf.fill(0.0);
        self.output_buf.fill(0.0);
        self.write_pos = 0;
        self.read_pos = 0;
        self.frames_processed = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms_error(a: &[f32], b: &[f32]) -> f64 {
        let sum_sq: f64 = a
            .iter()
            .zip(b.iter())
            .map(|(x, y)| ((x - y) as f64).powi(2))
            .sum();
        (sum_sq / a.len() as f64).sqrt()
    }

    #[test]
    fn passthrough_approximates_input() {
        let mut proc = SpectralProcessor::new(256, SpectralMode::Passthrough);
        let frame_size = 256;
        let signal_len = 4096;
        let total_len = signal_len + frame_size;
        let freq = 2.0 * std::f64::consts::PI * 40.0 / 256.0;
        let input: Vec<f32> = (0..total_len)
            .map(|i| 0.5_f32 * (i as f32 * freq as f32).sin())
            .collect();
        let mut output = vec![0.0f32; total_len];

        for (i, &sample) in input.iter().enumerate() {
            output[i] = proc.process(sample);
        }

        // Compare overlapped steady-state region (skip frame latency + frame-size transient)
        let aligned_input = &input[frame_size..signal_len];
        let aligned_output = &output[frame_size - 1 + frame_size..frame_size - 1 + signal_len];
        let error = rms_error(aligned_input, aligned_output);
        let input_rms = rms_error(aligned_input, &vec![0.0f32; aligned_input.len()]);
        let db = 20.0 * (error / input_rms.max(1e-12)).log10();
        assert!(
            db < -60.0,
            "passthrough error {db:.1} dB (expected < -60 dB)"
        );
    }

    #[test]
    fn spectral_gate_attenuates_quiet_bins() {
        let mut gate = SpectralProcessor::new(256, SpectralMode::Gate);
        gate.set_threshold(-20.0);
        let frame_size = 256;
        let signal_len = 4096;
        let total_len = signal_len + frame_size;
        // Amplitude 0.0005 gives peak FFT bin ≈ 0.0005 * 128 = 0.064, below -20 dB (0.1) threshold
        let amp = 0.0005_f32;
        let freq = 2.0 * std::f64::consts::PI * 20.0 / 256.0;
        let input: Vec<f32> = (0..total_len)
            .map(|i| amp * (i as f32 * freq as f32).sin())
            .collect();
        let mut output = vec![0.0f32; total_len];

        for (i, &sample) in input.iter().enumerate() {
            output[i] = gate.process(sample);
        }

        // Skip frame-size latency + one frame for steady state
        let aligned_output = &output[frame_size - 1 + frame_size..frame_size - 1 + signal_len];
        let output_energy: f64 = aligned_output.iter().map(|s| (*s as f64).powi(2)).sum();
        assert!(
            output_energy < 0.1,
            "gate below threshold should attenuate (energy {output_energy})"
        );
    }

    #[test]
    fn all_zero_in_all_zero_out() {
        let mut proc = SpectralProcessor::new(256, SpectralMode::Passthrough);
        for _ in 0..2048 {
            let out = proc.process(0.0);
            assert!(out == 0.0, "zero in should give zero out");
        }
    }
}
