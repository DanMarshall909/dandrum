use num_complex::Complex;
use rustfft::FftPlanner;

pub struct Convolution {
    ir: Vec<f32>,
    block_size: usize,
    partitions: Vec<Vec<Complex<f32>>>,
    input_buffer: Vec<f32>,
    input_pos: usize,
    output_buffer: Vec<f32>,
    output_pos: usize,
    wet: f32,
    fft_planner: FftPlanner<f32>,
}

impl Convolution {
    pub fn new() -> Self {
        Self {
            ir: Vec::new(),
            block_size: 512,
            partitions: Vec::new(),
            input_buffer: vec![0.0; 512],
            input_pos: 0,
            output_buffer: vec![0.0; 1024],
            output_pos: 0,
            wet: 1.0,
            fft_planner: FftPlanner::new(),
        }
    }

    pub fn load_ir(&mut self, ir: Vec<f32>) {
        self.ir = ir;
        self.partition_ir();
        self.reset();
    }

    pub fn set_wet(&mut self, wet: f32) {
        self.wet = wet.clamp(0.0, 1.0);
    }

    pub fn reset(&mut self) {
        self.input_buffer.fill(0.0);
        self.input_pos = 0;
        self.output_buffer.fill(0.0);
        self.output_pos = 0;
    }

    fn partition_ir(&mut self) {
        self.partitions.clear();
        if self.ir.is_empty() {
            return;
        }

        let fft_size = self.block_size * 2;
        let mut planner = FftPlanner::new();

        for chunk in self.ir.chunks(self.block_size) {
            let mut buffer: Vec<Complex<f32>> = chunk
                .iter()
                .copied()
                .map(|s| Complex::new(s, 0.0))
                .chain(std::iter::repeat(Complex::new(0.0, 0.0)))
                .take(fft_size)
                .collect();

            let fft = planner.plan_fft_forward(fft_size);
            fft.process(&mut buffer);
            self.partitions.push(buffer);
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        if self.partitions.is_empty() {
            return input * (1.0 - self.wet) + input * self.wet;
        }

        self.input_buffer[self.input_pos] = input;
        self.input_pos += 1;

        // Get dry sample from output buffer (overlap-add)
        let dry = input;
        let wet_sum = self.output_buffer[self.output_pos];
        self.output_buffer[self.output_pos] = 0.0;
        self.output_pos += 1;

        // When input block is full, process a convolution step
        if self.input_pos == self.block_size {
            self.process_block();
            self.input_pos = 0;
            self.output_pos = 0;
        }

        let out = dry * (1.0 - self.wet) + wet_sum * self.wet;
        out
    }

    fn process_block(&mut self) {
        let fft_size = self.block_size * 2;

        // FFT the input block
        let mut buffer: Vec<Complex<f32>> = self
            .input_buffer
            .iter()
            .copied()
            .map(|s| Complex::new(s, 0.0))
            .chain(std::iter::repeat(Complex::new(0.0, 0.0)))
            .take(fft_size)
            .collect();

        let fft = self.fft_planner.plan_fft_forward(fft_size);
        fft.process(&mut buffer);

        // Multiply with each partition and accumulate into output buffer
        for (partition_idx, partition_fft) in self.partitions.iter().enumerate() {
            let mut product: Vec<Complex<f32>> = buffer
                .iter()
                .zip(partition_fft.iter())
                .map(|(a, b)| a * b)
                .collect();

            let ifft = self.fft_planner.plan_fft_inverse(fft_size);
            ifft.process(&mut product);

            // Scale by 1/fft_size (rustfft doesn't normalize)
            let scale = 1.0 / fft_size as f32;

            // Overlap-add into output buffer at the appropriate offset
            let offset = partition_idx * self.block_size;
            if offset < self.output_buffer.len() {
                for j in 0..fft_size {
                    let out_idx = offset + j;
                    if out_idx < self.output_buffer.len() {
                        self.output_buffer[out_idx] += product[j].re * scale;
                    }
                }
            }
        }

        // Clear input buffer for next block
        self.input_buffer.fill(0.0);
    }
}

impl Default for Convolution {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
use super::*;

fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn empty_ir_is_passthrough() {
        let mut conv = Convolution::new();
        conv.set_wet(0.5);
        let out = conv.process(0.5);
        assert_eq!(out, 0.5, "no IR should pass signal through");
    }

    #[test]
    fn zero_wet_is_dry_passthrough() {
        let mut conv = Convolution::new();
        conv.set_wet(0.0);
        let out = conv.process(0.75);
        assert_eq!(out, 0.75, "zero wet should pass dry signal");
    }

    #[test]
    fn full_wet_with_unit_impulse_ir_passes_signal() {
        // Unit impulse IR: convolution with unit impulse is identity
        // Overlap-add convolution has a latency of one block_size
        let block_size = 512;
        let mut conv = Convolution::new();
        conv.load_ir(vec![1.0]);
        conv.set_wet(1.0);

        let mut output = Vec::new();
        for i in 0..2048 {
            let s = (i as f32 * 0.01).sin();
            output.push(conv.process(s));
        }
        // Output at (n + block_size) should match input at n (latency = block_size)
        let input_n = 100;
        let expected = (input_n as f32 * 0.01).sin();
        let actual = output[input_n + block_size];
        assert!(
            approx_eq(actual, expected, 1e-3),
            "unit impulse IR should preserve signal with block latency: at n+block got {actual}, expected {expected}"
        );
    }

    #[test]
    fn ir_length_shorter_than_block_size() {
        // Short IR should still produce output
        let mut conv = Convolution::new();
        conv.load_ir(vec![0.5, 0.25]);
        conv.set_wet(1.0);

        let output: Vec<f32> = (0..1024).map(|_| conv.process(1.0)).collect();
        // After block_size samples, some output should be produced
        let has_output = output.iter().skip(600).any(|&s| s.abs() > 0.001);
        assert!(has_output, "short IR should produce output");
    }

    #[test]
    fn ir_length_longer_than_block_size() {
        let mut conv = Convolution::new();
        let long_ir: Vec<f32> = (0..1024).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();
        conv.load_ir(long_ir);
        conv.set_wet(1.0);

        let output: Vec<f32> = (0..2048).map(|_| conv.process(0.5)).collect();
        let has_output = output.iter().skip(600).any(|&s| s.abs() > 0.001);
        assert!(has_output, "long IR should produce output");
    }

    #[test]
    fn reloading_ir_resets_state() {
        let mut conv = Convolution::new();
        conv.load_ir(vec![0.5]);
        conv.load_ir(vec![1.0]);
        conv.set_wet(1.0);

        conv.process(0.5);
        // After flushing block, unit impulse IR should recover signal
        let mut output = Vec::new();
        for _ in 0..600 {
            output.push(conv.process(0.3));
        }
        let val = output[599];
        assert!(val > 0.0, "second IR load should work: got {val}");
    }

    #[test]
    fn wet_dry_mix_ratio() {
        let mut conv = Convolution::new();
        conv.set_wet(0.5);
        conv.load_ir(vec![1.0]);

        let output: Vec<f32> = (0..2048).map(|_| conv.process(1.0)).collect();
        // dry = 0.5, wet = 0.5 * convolution_result
        // After latency, with unit IR the wet path tracks input
        let val = output[600 + 512];
        assert!(
            val > 0.0 && val < 1.05,
            "wet/dry mix should be between 0 and 1, got {val}"
        );
    }

    #[test]
    fn no_nan_for_any_input() {
        let mut conv = Convolution::new();
        conv.load_ir(vec![0.5, -0.25, 0.1]);
        conv.set_wet(1.0);

        for input in [-1e5, -100.0, -1.0, 0.0, 1.0, 100.0, 1e5] {
            let out = conv.process(input);
            assert!(!out.is_nan(), "no NaN for input {input}, got {out}");
        }
    }

    // === FFT-based acceptance tests ===

    #[test]
    fn convolution_impulse_response_accuracy() {
        // Convolve a unit impulse through an IR and verify the output matches the IR
        let ir: Vec<f32> = vec![0.5, -0.25, 0.1, -0.05];
        let mut conv = Convolution::new();
        conv.load_ir(ir.clone());
        conv.set_wet(1.0);

        let mut output = Vec::with_capacity(4096);
        for i in 0..4096 {
            // Single impulse at sample 0, silence rest
            let input = if i == 0 { 1.0 } else { 0.0 };
            output.push(conv.process(input));
        }

        // After block latency, convolution output should match IR
        // Output at (offset + block_size) should approximate IR at offset
        for (j, &expected) in ir.iter().enumerate() {
            let actual = output[j + 512]; // account for block latency
            assert!(
                approx_eq(actual, expected, 0.01),
                "IR sample {j}: expected {expected}, got {actual}"
            );
        }
    }
}
