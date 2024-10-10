use rodio::source::Source;
use std::f32::consts::PI;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Wave {
    samples: Vec<f32>,
    sample_rate: f32,
    duration: f32,
    index: usize,
    frequency: f32,
}

impl Wave {
    pub fn new(sample_rate: f32, frequency: f32, duration: f32) -> Self {
        let mut wave = Wave {
            sample_rate,
            frequency,
            duration,
            index: 0,
            samples: Vec::new(),
        };
        wave.samples = wave.build_sine_wave();
        wave
    }

    fn build_sine_wave(&mut self) -> Vec<f32> {
        let duration = self.duration;
        let sample_rate = self.sample_rate;
        let frequency = self.frequency;

        // Calculate the total number of samples needed
        let total_samples = (sample_rate * duration) as usize;

        // Prepare the sine wave generation parameters
        let two_pi_f = 2.0 * PI * frequency;
        let mut sample_clock = 0f32;

        // Create a vector to store the generated sine wave samples
        let mut sine_wave: Vec<f32> = Vec::with_capacity(total_samples);

        // Generate the sine wave and store it in the Vec<[f32; 2]>
        for _ in 0..total_samples {
            let sample_value = (two_pi_f * sample_clock / sample_rate as f32).sin();

            // Store the same value for both left and right channels (mono output in stereo format)
            sine_wave.push(sample_value);

            // Increment the sample clock
            sample_clock += 1.0;
            if sample_clock > sample_rate as f32 {
                sample_clock = 0.0;
            }
        }
        sine_wave
    }
}

impl Iterator for Wave {
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

impl Source for Wave {
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
