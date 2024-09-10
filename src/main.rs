// GUI
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

// Audio
use rodio::{source::SineWave, source::Source, OutputStream, Sink};
use std::f64::consts::PI;
use std::thread;
use std::time::{Duration, Instant};
use std::vec::Vec;

// Constants
const A4_FREQ: f32 = 440.0;
const SAMPLE_RATE: f32 = 44100.0; // Standard audio sample rate
const DURATION: f32 = 5.0; // 15 seconds for the A4 note

fn generate_sine_wave_vector(frequency: f32) -> Vec<f32> {
    Vec::<f32>::new()
}

struct Chirp {
    start_freq: f32,
    end_freq: f32,
    duration: f32,
    sample_rate: f32,
    index: usize,
    samples: Vec<f32>,
}

impl Chirp {
    fn new(sample_rate: f32, start_freq: f32, end_freq: f32, duration: f32) -> Self {
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

struct MyEguiApp {
    sine_wave: SineWave,
    duration: f32,
    is_playing: bool,
    started_sound: bool,
    start_time: Instant,
    zoom_factor: f32,
    x_offset: f32,
    points_vector: Vec<[f64; 2]>,
}

impl MyEguiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let sine_wave = SineWave::new(A4_FREQ);
        let start_time = Instant::now();
        let is_playing = false;
        let started_sound = false;
        let zoom_factor = 1.0;
        let x_offset = 0.0;
        let points_vector = vec![];
        Self {
            sine_wave,
            duration: DURATION,
            is_playing,
            started_sound,
            start_time,
            zoom_factor,
            x_offset,
            points_vector,
        }
    }

    fn plot(&mut self) {
        self.is_playing = true;
        self.start_time = Instant::now();
    }

    fn stop(&mut self) {
        self.is_playing = false;
    }

    fn start_sound(&mut self) {
        if self.started_sound {
            return;
        }
        self.started_sound = true;
        // Play the sound in a separate thread
        let duration = self.duration;
        thread::spawn(move || {
            let sine_wave = SineWave::new(A4_FREQ);
            // Play the sound for 2 seconds through the speakers
            // Get an output stream handle to the default physical sound device
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let sink = Sink::try_new(&stream_handle).unwrap();
            // Play the sound directly on the device
            sink.append(sine_wave.take_duration(std::time::Duration::from_secs(duration as u64)));
            sink.sleep_until_end();
        });
    }
}
impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Caliber");

            ui.horizontal(|ui| {
                if ui.button("Start").clicked() {
                    self.plot();
                };
                if ui.button("Stop").clicked() {
                    self.stop();
                };
            });

            ui.horizontal(|ui| {
                if ui.button("Zoom In").clicked() {
                    self.zoom_factor *= 1.2; // Increase zoom factor
                }
                if ui.button("Zoom Out").clicked() {
                    self.zoom_factor /= 1.2; // Decrease zoom factor
                }
            });

            // Create a scroll area with a scrollbar
            egui::ScrollArea::horizontal().show(ui, |ui| {
                Plot::new("Sine Wave")
                    .view_aspect(2.0) // Aspect ratio of the plot
                    .data_aspect(self.zoom_factor)
                    .allow_drag(true)
                    .show(ui, |plot_ui| {
                        plot_ui.line(Line::new(PlotPoints::new(self.points_vector.clone())));
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Scroll X:");
                if ui.button("<< Left").clicked() {
                    self.x_offset -= 0.1; // Pan left
                }
                if ui.button("Right >>").clicked() {
                    self.x_offset += 0.1; // Pan right
                }
            });

            if self.is_playing {
                self.start_sound();
                let elapsed = self.start_time.elapsed().as_secs_f32();
                let max_time = elapsed.min(DURATION);

                // Plot the sine wave over time
                let samples_to_show = (max_time * SAMPLE_RATE) as usize;
                let sine_wave_segment = self
                    .sine_wave
                    .clone()
                    .take(samples_to_show)
                    .collect::<Vec<f32>>();

                let points: Vec<[f64; 2]> = sine_wave_segment
                    .iter()
                    .enumerate()
                    .map(|(i, &val)| {
                        let time = i as f32 / SAMPLE_RATE;
                        [time as f64, val as f64]
                    })
                    .collect();
                self.points_vector = points;

                // Stop playing after 15 seconds
                if elapsed >= DURATION {
                    self.is_playing = false;
                    self.started_sound = false;
                }
            }

            // Request a repaint to keep the animation running
            if self.is_playing {
                ctx.request_repaint();
            }
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "Caliber",
        native_options,
        Box::new(|cc| Ok(Box::new(MyEguiApp::new(cc)))),
    );
}

// WebAssembly entry point
#[cfg(target_arch = "wasm32")]
fn wasm_main() {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    pub fn start() -> Result<(), JsValue> {
        let mut app = MyEguiApp::new(&eframe::CreationContext {
            web_info: eframe::WebInfo {
                canvas_id: "the_canvas_id".to_owned(),
            },
        });
        egui_web::start("the_canvas_id", Box::new(app))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::{source::SineWave, source::Source, OutputStream, Sink};

    #[test]
    fn test_make_a4_sound() {
        let sine_wave = SineWave::new(A4_FREQ).take_duration(std::time::Duration::from_secs(2));
        // Play the sound for 2 seconds through the speakers
        // // Get an output stream handle to the default physical sound device
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        // Load a sound from a file, using a path relative to Cargo.toml
        // Play the sound directly on the device
        sink.append(sine_wave);
        sink.sleep_until_end();
    }

    #[test]
    fn test_make_chirp() {
        let chirp = Chirp::new(SAMPLE_RATE, 100.0, 1000.0, 14.0);
        // Play the sound for 2 seconds through the speakers
        // // Get an output stream handle to the default physical sound device
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        // Load a sound from a file, using a path relative to Cargo.toml
        // Play the sound directly on the device
        sink.append(chirp);
        sink.sleep_until_end();
    }
}
