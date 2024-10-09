// GUI
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

// Audio
use cpal::traits::DeviceTrait;
use rodio::source::SineWave;

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::vec::Vec;
use tokio::spawn;

// Constants
const A4_FREQ: f32 = 440.0;
const SAMPLE_RATE: f32 = 192000.0; // Standard audio sample rate
const DOWNSAMPLE_FACTOR: f32 = 1000.0;
const DURATION: f32 = 30.0; // 15 seconds for the A4 note
mod audio;
mod chirp;
mod freq;
use chirp::Chirp;

struct MainUI {
    sine_wave: SineWave,
    duration: f32,
    is_playing: bool,
    started_sound: bool,
    start_time: Instant,
    zoom_factor: f32,
    x_offset: f32,
    points_vector: Vec<[f64; 2]>,
    for_tx: Sender<f32>,
    for_rx: Receiver<f32>,
    captured_buffer: Arc<Mutex<Vec<f32>>>,
    last_for: f32,
    input_device_name: String,
    output_device_name: String,
}

impl MainUI {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let sine_wave = SineWave::new(A4_FREQ);
        let start_time = Instant::now();
        let is_playing = false;
        let started_sound = false;
        let zoom_factor = 0.0;
        let x_offset = 0.0;
        let points_vector = vec![];
        let (for_tx, for_rx): (Sender<f32>, Receiver<f32>) = mpsc::channel();
        let captured_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
        Self {
            sine_wave,
            duration: DURATION,
            is_playing,
            started_sound,
            start_time,
            zoom_factor,
            x_offset,
            points_vector,
            for_tx,
            for_rx,
            captured_buffer,
            last_for: 0.0,
            input_device_name: "Default".to_string(),
            output_device_name: "Default".to_string(),
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
        let for_tx = self.for_tx.clone();
        let captured_buffer = self.captured_buffer.clone();
        let input_device_name = self.input_device_name.clone();
        let output_device_name = self.output_device_name.clone();

        // Start the wave playing thread.
        spawn(async move {
            let sound = Chirp::new(SAMPLE_RATE, 500.0, 2000.0, duration);
            audio::play_output(output_device_name, sound);
        });

        // Start the wave capturing thread.
        spawn(async move {
            audio::capture_input(
                input_device_name,
                SAMPLE_RATE,
                duration,
                captured_buffer,
                for_tx,
            );
        });
    }
}

impl MainUI {
    fn paint_window_title(&self, ui: &mut egui::Ui) {
        ui.heading("Calliber");
    }

    fn paint_start_and_stop_buttons(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Start").clicked() {
                self.plot();
            };
            if ui.button("Stop").clicked() {
                self.stop();
            };
        });
    }

    fn paint_sound_devices_dropdown(&mut self, ui: &mut egui::Ui) {
        let input_devices = audio::get_input_devices().unwrap();
        let output_devices = audio::get_output_devices().unwrap();

        ui.horizontal(|ui| {
            ui.label("Input device:");
            egui::ComboBox::new("input_device", "")
                .selected_text(self.input_device_name.to_string())
                .show_ui(ui, |ui| {
                    for kind in input_devices {
                        ui.selectable_value(
                            &mut self.input_device_name,
                            kind.name().unwrap(),
                            kind.name().unwrap(),
                        );
                    }
                });
            ui.label("Output device:");
            egui::ComboBox::new("output_device", "")
                .selected_text(self.output_device_name.to_string())
                .show_ui(ui, |ui| {
                    for kind in output_devices {
                        ui.selectable_value(
                            &mut self.output_device_name,
                            kind.name().unwrap(),
                            kind.name().unwrap(),
                        );
                    }
                });
        });
    }

    fn paint_zoom_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Zoom In").clicked() {
                self.zoom_factor *= 1.2; // Increase zoom factor
            }
            if ui.button("Zoom Out").clicked() {
                self.zoom_factor /= 1.2; // Decrease zoom factor
            }
        });
    }

    fn paint_output_wave(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::horizontal().show(ui, |ui| {
            let mut points_to_plot = self.points_vector.clone();
            let points_to_plot_len = points_to_plot.len();
            let downsampled_sample_rate = (SAMPLE_RATE / DOWNSAMPLE_FACTOR) as usize;
            if points_to_plot.len() > downsampled_sample_rate * 5 {
                points_to_plot.drain(0..points_to_plot_len - downsampled_sample_rate * 5);
            }
            Plot::new("Sine Wave")
                .height(240.0)
                .allow_drag(true)
                .show(ui, |plot_ui| {
                    plot_ui.line(Line::new(PlotPoints::new(points_to_plot)));
                });
        });
    }

    fn paint_scroll_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Scroll X:");
            if ui.button("<< Left").clicked() {
                self.x_offset -= 0.1; // Pan left
            }
            if ui.button("Right >>").clicked() {
                self.x_offset += 0.1; // Pan right
            }
        });
    }

    fn paint_frequency_of_resonance(&self, ui: &mut egui::Ui) {
        ui.label(format!("Frequency of resonance: {:.2} Hz", self.last_for));
    }

    fn update_outgoing_wave_graph(&mut self) {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let max_time = elapsed.min(DURATION);

        // Plot the sine wave over time
        let samples_to_show = (max_time * SAMPLE_RATE) as usize;

        let downsample_factor = DOWNSAMPLE_FACTOR as usize;
        let sine_wave_segment = self
            .sine_wave
            .clone()
            .enumerate()
            .filter(|(i, _)| i % downsample_factor == 0)
            .take(samples_to_show / downsample_factor)
            .map(|(_, val)| val)
            .collect::<Vec<f32>>();

        let points: Vec<[f64; 2]> = sine_wave_segment
            .iter()
            .enumerate()
            .map(|(i, &val)| {
                // Multiply by tghe downsample_factor to restore the units to seconds.
                let time = (i * downsample_factor) as f32 / SAMPLE_RATE;
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
}

impl eframe::App for MainUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::scroll_area::ScrollArea::vertical().show(ui, |ui| {
                self.paint_window_title(ui);
                self.paint_start_and_stop_buttons(ui);
                self.paint_sound_devices_dropdown(ui);
                self.paint_zoom_controls(ui);
                self.paint_output_wave(ui);
                self.paint_scroll_controls(ui);

                if self.is_playing {
                    self.start_sound();
                    self.update_outgoing_wave_graph();
                    ui.label("Calculating frequency of resonance...");
                }

                let captured_buffer = self.captured_buffer.lock().unwrap();
                ui.label(format!("Captured buffer len {}", captured_buffer.len()));
                if let Ok(freq) = self.for_rx.try_recv() {
                    self.last_for = freq;
                }
                if !self.is_playing {
                    self.paint_frequency_of_resonance(ui);
                }

                let buffer_to_plot = captured_buffer.clone();
                let buf_len = buffer_to_plot.len();

                let mut points: Vec<[f64; 2]> = buffer_to_plot
                    .into_iter()
                    .enumerate()
                    .map(|(i, x)| [(i as f32 / 44100.0) as f64, x as f64])
                    .collect();

                if buf_len > 44100 * 5 {
                    points.drain(0..buf_len - 44100 * 5);
                }

                let line = Line::new(PlotPoints::new(points));
                let plot = Plot::new("Received audio").height(240.0);
                plot.show(ui, |plot_ui| {
                    plot_ui.line(line);
                });
                // Request a repaint to keep the animation running
                ctx.request_repaint();
            });
        });
    }
}

#[tokio::main]
async fn main() {
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "Caliber",
        native_options,
        Box::new(|cc| Ok(Box::new(MainUI::new(cc)))),
    );
}

#[cfg(test)]
mod tests {
    use super::chirp::Chirp;
    use super::*;
    use rodio::{source::SineWave, source::Source, OutputStream, Sink};

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

    fn test_make_chirp() {
        // Generate a chirp that lasts for 2 seconds.
        // Starting frequency = 100.0 HZ.
        // End frequency = 1000.0 HZ.
        // Sample rate = 44100.0 rate/second. Which is standard for digital audio.
        let chirp = Chirp::new(SAMPLE_RATE, 100.0, 1000.0, 2.0);

        // // Get an output stream handle to the default physical sound device
        // let (_stream, stream_handle) = OutputStream::try_default().unwrap();

        // // Make a sync to append the audio to.
        // let sink = Sink::try_new(&stream_handle).unwrap();

        // // Load a sound from a file, using a path relative to Cargo.toml
        // // Play the sound directly on the device
        // sink.append(chirp);

        // // Sleep until the audio is done playing.
        // // Giving up ownership of the sink would close it and the audio will stop.
        // sink.sleep_until_end();
    }
}
