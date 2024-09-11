use rustfft::{num_complex::Complex, FftPlanner};

pub fn freq_of_resonance(samples: Vec<f32>, sample_rate: f32) -> f32 {
    let num_samples = samples.len();

    let mut fft_input: Vec<Complex<f32>> = samples.iter().map(|&x| Complex::new(x, 0.0)).collect();

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(num_samples);

    fft.process(&mut fft_input);

    let magnitudes: Vec<f32> = fft_input
        .iter()
        .take(num_samples / 2)
        .map(|c| c.norm_sqr())
        .collect();

    let (max_index, _) = magnitudes
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .unwrap();

    let freq_of_resolution = sample_rate / num_samples as f32;
    let freq_of_resonance = max_index as f32 * freq_of_resolution;

    freq_of_resonance
}
