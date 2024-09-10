use rodio::source::Source;
use std::f64::consts::PI;
use std::time::Duration;

pub struct Chirp {
    start_freq: f32,
    end_freq: f32,
    duration: f32,
    sample_rate: f32,
    index: usize,
    samples: Vec<f32>,
}

impl Chirp {
    pub fn new(sample_rate: f32, start_freq: f32, end_freq: f32, duration: f32) -> Self {
        let samples = generate_chirp_wave(sample_rate, duration, start_freq, end_freq);
        Self {
            start_freq,
            end_freq,
            duration,
            sample_rate,
            index: 0,
            samples,
        }
    }
}

impl Iterator for Chirp {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let index = self.index as usize;
        if index < self.samples.len() {
            let sample = self.samples[index];
            self.index += 1;
            Some(sample)
        } else {
            None
        }
    }
}

impl Source for Chirp {
    fn current_frame_len(&self) -> Option<usize> {
        // Number of remaining samples (frame length)
        Some(self.samples.len() - self.index)
    }

    fn channels(&self) -> u16 {
        // We are generating a mono signal (one channel)
        1
    }

    fn sample_rate(&self) -> u32 {
        // Return the sample rate
        self.sample_rate as u32
    }

    fn total_duration(&self) -> Option<Duration> {
        // Total duration of the chirp wave
        Some(Duration::from_secs_f64(
            self.samples.len() as f64 / self.sample_rate as f64,
        ))
    }
}

// Function to generate a linear chirp wave
fn generate_chirp_wave(
    sample_rate: f32,
    duration: f32,
    start_freq: f32,
    end_freq: f32,
) -> Vec<f32> {
    let num_samples = (duration * sample_rate as f32) as usize;
    let mut waveform = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64; // current time
        let freq =
            start_freq as f64 + (end_freq as f64 - start_freq as f64) * (t / duration as f64);
        let sample = (2.0 * PI * freq * t).sin(); // sine wave sample
        waveform.push(sample as f32);
    }

    waveform
}
