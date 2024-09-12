// GUI
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

// Audio
use cpal::traits::DeviceTrait;
use rodio::source::SineWave;

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::spawn;
use std::time::Instant;
use std::vec::Vec;

// Constants
const A4_FREQ: f32 = 440.0;
const SAMPLE_RATE: f32 = 44100.0; // Standard audio sample rate
const DURATION: f32 = 5.0; // 15 seconds for the A4 note
                           //
mod audio;
mod chirp;
mod freq;
use chirp::Chirp;

struct MyEguiApp {
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
    last_for: f32,
    input_device_name: String,
    output_device_name: String,
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
        let (for_tx, for_rx): (Sender<f32>, Receiver<f32>) = mpsc::channel();
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
        let input_device_name = self.input_device_name.clone();
        let output_device_name = self.output_device_name.clone();

        // Start the wave playing thread.
        spawn(move || {
            let sound = Chirp::new(SAMPLE_RATE, 1000.0, 20000.0, duration);
            audio::play_output(output_device_name, sound);
        });

        // Start the wave capturing thread.
        spawn(move || {
            audio::capture_input(input_device_name, SAMPLE_RATE, duration, for_tx);
        });
    }
}

impl MyEguiApp {
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
            Plot::new("Sine Wave")
                .view_aspect(2.0) // Aspect ratio of the plot
                .data_aspect(self.zoom_factor)
                .allow_drag(true)
                .show(ui, |plot_ui| {
                    plot_ui.line(Line::new(PlotPoints::new(self.points_vector.clone())));
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
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
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

            if let Ok(freq) = self.for_rx.try_recv() {
                self.last_for = freq;
            }
            if !self.is_playing {
                self.paint_frequency_of_resonance(ui);
            }
            // Request a repaint to keep the animation running
            ctx.request_repaint();
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
    use super::chirp::Chirp;
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
        // Generate a chirp that lasts for 2 seconds.
        // Starting frequency = 100.0 HZ.
        // End frequency = 1000.0 HZ.
        // Sample rate = 44100.0 rate/second. Which is standard for digital audio.
        let chirp = Chirp::new(SAMPLE_RATE, 100.0, 1000.0, 2.0);

        // // Get an output stream handle to the default physical sound device
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();

        // Make a sync to append the audio to.
        let sink = Sink::try_new(&stream_handle).unwrap();

        // Load a sound from a file, using a path relative to Cargo.toml
        // Play the sound directly on the device
        sink.append(chirp);

        // Sleep until the audio is done playing.
        // Giving up ownership of the sink would close it and the audio will stop.
        sink.sleep_until_end();
    }
}
