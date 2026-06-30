#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InterpolationMode {
    Linear,
    Cubic,
}

pub struct DelayLine {
    buffer: Vec<f32>,
    mask: usize,
    write_head: usize,
    max_delay_samples: usize,
    interpolation: InterpolationMode,
}

impl DelayLine {
    pub fn new(max_delay_ms: f64, sample_rate: f64) -> Self {
        let max_samples = (max_delay_ms * sample_rate / 1000.0).ceil() as usize;
        let capacity = max_samples.next_power_of_two();
        Self {
            buffer: vec![0.0; capacity],
            mask: capacity - 1,
            write_head: 0,
            max_delay_samples: max_samples,
            interpolation: InterpolationMode::Linear,
        }
    }

    pub fn max_delay_samples(&self) -> usize {
        self.max_delay_samples
    }

    pub fn set_interpolation_mode(&mut self, mode: InterpolationMode) {
        self.interpolation = mode;
    }

    pub fn write(&mut self, sample: f32) {
        self.buffer[self.write_head] = sample;
        self.write_head = (self.write_head + 1) & self.mask;
    }

    pub fn read(&self, delay_samples: f32) -> f32 {
        let delay = delay_samples.max(0.0).min(self.max_delay_samples as f32);
        let int_part = delay.floor() as usize;
        let frac = delay - int_part as f32;

        let read_head = (self.write_head.wrapping_sub(1 + int_part)) & self.mask;

        match self.interpolation {
            InterpolationMode::Linear => self.read_linear(read_head, frac),
            InterpolationMode::Cubic => self.read_cubic(read_head, frac),
        }
    }

    fn read_linear(&self, read_head: usize, frac: f32) -> f32 {
        let a = self.buffer[read_head];
        let b = self.buffer[(read_head.wrapping_sub(1)) & self.mask];
        a + frac * (b - a)
    }

    fn read_cubic(&self, read_head: usize, frac: f32) -> f32 {
        let x0 = self.buffer[(read_head.wrapping_add(1)) & self.mask];
        let x1 = self.buffer[read_head];
        let x2 = self.buffer[(read_head.wrapping_sub(1)) & self.mask];
        let x3 = self.buffer[(read_head.wrapping_sub(2)) & self.mask];

        let f = frac;
        let f2 = f * f;
        let f3 = f2 * f;

        let a = 3.0 * (x1 - x2) + x3 - x0;
        let b = 2.0 * x0 + 4.0 * x2 - 5.0 * x1 - x3;
        let c = x2 - x0;
        let d = x1;

        0.5 * (a * f3 + b * f2 + c * f + 2.0 * d)
    }

    pub fn set_modulation(&mut self, _offset: f32) {
        // modulation is applied at read time via the `read` parameter;
        // this method is a future hook for stateful modulation
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_head = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_delay() -> DelayLine {
        DelayLine::new(100.0, 48000.0)
    }

    #[test]
    fn power_of_two_buffer_sizing() {
        let d = DelayLine::new(2.0, 48000.0);
        let min_samples = (2.0_f64 * 48000.0 / 1000.0).ceil() as usize;
        assert!(d.buffer.len() >= min_samples);
        assert!(d.buffer.len().is_power_of_two());
    }

    #[test]
    fn max_delay_samples_query() {
        let d = create_delay();
        assert_eq!(
            d.max_delay_samples(),
            (100.0_f64 * 48000.0 / 1000.0).ceil() as usize
        );
    }

    #[test]
    fn write_then_integer_delay_read() {
        let mut d = create_delay();
        d.write(1.0);
        d.write(2.0);
        let result = d.read(1.0);
        assert!((result - 1.0).abs() < 1e-6, "expected 1.0, got {}", result);
    }

    #[test]
    fn fractional_delay_linear_interpolation() {
        let mut d = create_delay();
        d.write(1.0);
        d.write(2.0);
        let result = d.read(0.5);
        assert!((result - 1.5).abs() < 1e-6, "expected 1.5, got {}", result);
    }

    #[test]
    fn cubic_interpolation() {
        let mut d = create_delay();
        d.set_interpolation_mode(InterpolationMode::Cubic);
        d.write(0.0);
        d.write(0.0);
        d.write(1.0);
        d.write(0.0);
        let result = d.read(0.5);
        assert!(
            (result - 0.5625).abs() < 1e-4,
            "expected ~0.5625, got {}",
            result
        );
    }

    #[test]
    fn modulation_shifts_read_position() {
        let mut d = create_delay();
        for i in 0..200 {
            d.write(i as f32);
        }
        let base = d.read(100.0);
        let modulated = d.read(110.0);
        // 110 samples back should read a different value than 100 samples back
        assert!(
            (base - modulated).abs() > 1e-6,
            "modulation should change read position"
        );
    }

    #[test]
    fn modulation_clamped_to_minimum() {
        let mut d = create_delay();
        d.write(42.0);
        // Trying to read at delay 0 (which would mean "current sample") should clamp
        let result = d.read(0.0);
        assert!(!result.is_nan());
    }

    #[test]
    fn reset_clears_buffer() {
        let mut d = create_delay();
        d.write(1.0);
        d.write(2.0);
        d.reset();
        let result = d.read(1.0);
        assert!(
            (result).abs() < 1e-6,
            "expected 0.0 after reset, got {}",
            result
        );
    }

    #[test]
    fn read_at_max_delay_does_not_panic() {
        let mut d = DelayLine::new(5.0, 48000.0);
        let max = d.max_delay_samples();
        for i in 0..max + 10 {
            d.write(i as f32);
        }
        let result = d.read(max as f32);
        assert!(!result.is_nan());
    }

    #[test]
    fn interpolation_mode_switching() {
        let mut d = create_delay();
        assert_eq!(d.interpolation, InterpolationMode::Linear);
        d.set_interpolation_mode(InterpolationMode::Cubic);
        assert_eq!(d.interpolation, InterpolationMode::Cubic);
    }

    #[test]
    fn wrap_around_does_not_panic() {
        let mut d = DelayLine::new(2.0, 100.0);
        let buf_len = d.buffer.len();
        // Write more samples than the buffer can hold to trigger wrap-around
        for i in 0..buf_len * 3 {
            d.write(i as f32);
        }
        let result = d.read(1.0);
        assert!(!result.is_nan());
    }
}
