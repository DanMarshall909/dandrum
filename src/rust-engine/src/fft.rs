use num_complex::Complex;
use rustfft::FftPlanner;

pub struct MagnitudeResponse {
    pub bins: Vec<(f64, f64)>,
}

pub fn compute_magnitude_response(
    signal: &[f32],
    sample_rate: f64,
) -> MagnitudeResponse {
    let fft_size = signal.len().next_power_of_two().max(2);
    let mut buffer: Vec<Complex<f32>> = signal
        .iter()
        .copied()
        .map(|s| Complex::new(s, 0.0))
        .chain(std::iter::repeat(Complex::new(0.0, 0.0)))
        .take(fft_size)
        .collect();

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    fft.process(&mut buffer);

    let nyquist = fft_size / 2 + 1;
    let bins: Vec<(f64, f64)> = (0..nyquist)
        .map(|i| {
            let magnitude = (buffer[i].norm() as f64).max(1e-12);
            let magnitude_db = 20.0 * magnitude.log10();
            let freq = i as f64 * sample_rate / fft_size as f64;
            (freq, magnitude_db)
        })
        .collect();

    MagnitudeResponse { bins }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_impulse_produces_flat_spectrum() {
        let len = 4096;
        let mut signal = vec![0.0f32; len];
        signal[len / 2] = 1.0;
        let response = compute_magnitude_response(&signal, 48000.0);
        for &(_freq, db) in &response.bins {
            assert!(
                (db - 0.0).abs() < 1.0,
                "expected near 0 dB, got {db} dB"
            );
        }
    }

    #[test]
    fn sinc_functional_frequency_bin_centers() {
        let signal = vec![1.0f32; 512];
        let response = compute_magnitude_response(&signal, 1000.0);
        let bin_spacing = 1000.0 / 512.0;
        for (i, &(freq, _db)) in response.bins.iter().enumerate() {
            let expected = i as f64 * bin_spacing;
            assert!(
                (freq - expected).abs() < 1e-6,
                "bin {i}: expected {expected} Hz, got {freq} Hz"
            );
        }
    }

    #[test]
    fn zero_pads_to_next_power_of_two() {
        let signal = vec![1.0f32; 1000];
        let response = compute_magnitude_response(&signal, 48000.0);
        let expected_bins = 1024 / 2 + 1;
        assert_eq!(response.bins.len(), expected_bins);
    }
}
