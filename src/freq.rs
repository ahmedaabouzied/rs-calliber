use rustfft::{
    num_complex::Complex, FftPlanner, FftPlannerAvx, FftPlannerNeon, FftPlannerScalar,
    FftPlannerSse,
};

#[derive(Clone, Debug)]
pub enum Planner {
    FftPlanner,
    // FftPlannerAvx,
    FftPlannerNeon,
    FftPlannerScalar,
    // FftPlannerSse,
}

pub fn freq_of_resonance(samples: Vec<f32>, sample_rate: f32, planner: Option<Planner>) -> f32 {
    let num_samples = samples.len();
    println!("Num samples = {}", num_samples);

    let mut fft_input: Vec<Complex<f32>> = samples.iter().map(|&x| Complex::new(x, 0.0)).collect();

    let mut generic_planner = FftPlanner::new();
    // let mut avx_planner = FftPlannerAvx::new().unwrap();
    let mut neon_planner = FftPlannerNeon::new().unwrap();
    let mut scalar_planner = FftPlannerScalar::new();
    // let mut sse_planner = FftPlannerSse::new().unwrap();

    let fft = match planner {
        Some(p) => match p {
            // Planner::FftPlannerAvx => avx_planner.plan_fft_forward(num_samples),
            Planner::FftPlannerNeon => neon_planner.plan_fft_forward(num_samples),
            Planner::FftPlannerScalar => scalar_planner.plan_fft_forward(num_samples),
            // Planner::FftPlannerSse => sse_planner.plan_fft_forward(num_samples),
            Planner::FftPlanner => generic_planner.plan_fft_forward(num_samples),
        },
        None => generic_planner.plan_fft_forward(num_samples),
    };

    fft.process(&mut fft_input);
    println!("FFT processed {}", fft_input.len());

    let magnitudes: Vec<f32> = fft_input[0..num_samples / 2]
        .iter()
        .map(|c| c.norm())
        .collect();
    if magnitudes.len() > 10 {
        println!("Magnitudes = {:?}", &magnitudes[0..10]);
    }

    let (max_index, _) = magnitudes
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .unwrap();

    let freq_of_resolution = sample_rate / num_samples as f32;
    let freq_of_resonance = max_index as f32 * freq_of_resolution;

    freq_of_resonance
}

#[cfg(test)]
mod tests {
    use super::*;
    use hound;

    use rustfft::{num_complex::Complex, FftPlanner};
    use std::f32::consts::PI;

    // Generate a sine wave at a given frequency, sample rate, and duration
    fn generate_sine_wave(frequency: f32, sample_rate: f32, duration: f32) -> Vec<f32> {
        let sample_count = (sample_rate * duration) as usize;
        (0..sample_count)
            .map(|i| (2.0 * PI * frequency * i as f32 / sample_rate).sin())
            .collect()
    }

    // Calculate the FFT of the signal
    fn calculate_fft(samples: &[f32]) -> Vec<Complex<f32>> {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(samples.len());

        let mut buf: Vec<Complex<f32>> = samples
            .iter()
            .map(|&x| Complex { re: x, im: 0.0 })
            .collect();

        let mut output = buf.clone();
        fft.process_with_scratch(&mut buf, &mut output);
        output
    }

    // Calculate the IFFT from the FFT output
    fn calculate_ifft(fft_output: &[Complex<f32>]) -> Vec<f32> {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_inverse(fft_output.len());

        let mut buf = fft_output.to_vec();

        fft.process(&mut buf);
        buf.iter().map(|c| c.re).collect() // Return the real part of the IFFT result
    }

    #[test]
    fn test_freq_of_resonance_of_wav_file() {
        let mut reader =
            hound::WavReader::open("/Users/aabouzied/Downloads/input_excitation.wav").unwrap();
        let spec = reader.spec();
        println!("Sample rate: {}", spec.sample_rate);
        println!("Channels: {}", spec.channels);
        println!("Bits per sample: {}", spec.bits_per_sample);
        println!("Duration: {}", reader.duration());
        let samples: Vec<f32> = reader
            .samples::<f32>() // Assume the WAV file has 16-bit samples
            .map(|s| s.unwrap() as f32 / f32::MAX as f32) // Convert samples to f32 in range -1.0 to 1.0
            .collect();
        for alg in vec![
            Planner::FftPlanner,
            Planner::FftPlannerNeon,
            // Planner::FftPlannerAvx,
            Planner::FftPlannerScalar,
            // Planner::FftPlannerSse,
        ]
        .into_iter()
        {
            let res = freq_of_resonance(samples.clone(), 192000.00, Some(alg.clone()));
            if (res - 1348.00).abs() > 1.0 {
                println!(
                    "{:?}: Expected freq of resonance = 1348, but got {}",
                    alg, res
                );
            }
            println!("{:?}: Result freq of resonance = {}", alg, res);
        }
    }

    #[test]
    fn test_sine_wave_fft_peak() {
        let sample_rate = 44100.0;
        let frequency = 440.0;
        let duration = 1.0; // 1 second

        let samples = generate_sine_wave(frequency, sample_rate, duration);
        let calculated_frequency = freq_of_resonance(samples, sample_rate, None);
        // Assert that the calculated frequency is close to 440 Hz
        assert!(
            (calculated_frequency - frequency).abs() < 1.0,
            "Expected frequency: {}, but got: {}",
            frequency,
            calculated_frequency
        );
    }

    #[test]
    fn test_fft_symmetry() {
        let sample_rate = 44100.0;
        let frequency = 440.0;
        let duration = 1.0;

        let samples = generate_sine_wave(frequency, sample_rate, duration);
        let fft_output = calculate_fft(&samples);

        let n = fft_output.len();

        for i in 0..(n / 2) {
            let left = fft_output[i];
            let right = fft_output[n - i - 1];

            // Verify that the FFT result is symmetric
            assert!(
                (left.re - right.re).abs() < 1e-5,
                "Real parts are not symmetric"
            );
            assert!(
                (left.im + right.im).abs() < 1e-5,
                "Imaginary parts are not symmetric"
            );
        }
    }
    #[test]
    fn test_ifft_reconstruction() {
        let sample_rate = 44100.0;
        let frequency = 440.0;
        let duration = 1.0;

        let samples = generate_sine_wave(frequency, sample_rate, duration);
        let fft_output = calculate_fft(&samples);
        let ifft_output = calculate_ifft(&fft_output);

        // Compare the IFFT output with the original signal
        let tolerance = 1e-5;
        for (original, reconstructed) in samples.iter().zip(ifft_output.iter()) {
            assert!(
                (original - reconstructed).abs() < tolerance,
                "Original and IFFT-reconstructed signals differ"
            );
        }
    }
    #[test]
    fn test_frequency_bin_resolution() {
        let sample_rate = 44100.0;
        let fft_length = 1024;
        let freq_bin_size = sample_rate / fft_length as f32;

        assert!(
            (freq_bin_size - 43.07).abs() < 0.01,
            "Frequency bin size should be approximately 43.07 Hz, but got: {}",
            freq_bin_size
        );
    }
}
