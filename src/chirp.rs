use rodio::source::Source;
use std::time::Duration;

/// Chirp is a linear sound wave which frequency increases linearly over time.
#[derive(Debug, Clone)]
pub struct Chirp {
    pub start_freq: f32,
    pub end_freq: f32,
    pub duration: f32,
    pub sample_rate: f32,
    index: usize,
    pub samples: Vec<f32>,
}

impl From<hound::WavReader<std::io::BufReader<std::fs::File>>> for Chirp {
    fn from(mut r: hound::WavReader<std::io::BufReader<std::fs::File>>) -> Self {
        let spec = r.spec();
        let samples: Vec<f32> = r.samples::<i32>().map(|s| s.unwrap() as f32).collect();
        let duration = r.duration() / spec.sample_rate;
        let sample_rate = spec.sample_rate;
        let start_freq = samples[0];
        let end_freq = samples.last().unwrap().clone();
        Self {
            samples,
            sample_rate: sample_rate as f32,
            duration: duration as f32,
            start_freq: start_freq,
            end_freq: end_freq.to_owned(),
            index: 0,
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
