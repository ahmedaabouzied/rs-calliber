// GUI
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

// Audio
use cpal::traits::DeviceTrait;
use rodio::source::SineWave;

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::spawn;
use std::time::Instant;
use std::vec::Vec;

// Constants
const DEFAULT_SAMPLE_RATE: f32 = 192000.0;
const DEFAULT_DOWNSAMPLE_FACTOR: f32 = 1000.0;
const DEFAULT_DURATION: f32 = 5.0;

mod audio;
mod chirp;
mod freq;
use chirp::Chirp;

struct MainUI {
    current_chirp: Chirp,
    duration: f32, // Default is 5.0;
    is_playing: Arc<AtomicBool>,
    started_sound: bool,
    start_time: Instant,
    points_vector: Vec<[f64; 2]>,
    for_tx: Sender<f32>,
    for_rx: Receiver<f32>,
    captured_buffer: Arc<Mutex<Vec<f32>>>,
    last_for: f32,
    input_device_name: String,
    output_device_name: String,
    drain_graphs: bool,
}

impl MainUI {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let current_chirp = Chirp::new(DEFAULT_SAMPLE_RATE, 0.0, 20000.0, DEFAULT_DURATION);
        let start_time = Instant::now();
        let is_playing = Arc::new(AtomicBool::new(false));
        let started_sound = false;
        let points_vector = vec![];
        let (for_tx, for_rx): (Sender<f32>, Receiver<f32>) = mpsc::channel();
        let captured_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
        let drain_graphs = true;
        Self {
            current_chirp,
            duration: DEFAULT_DURATION,
            is_playing,
            started_sound,
            start_time,
            points_vector,
            for_tx,
            for_rx,
            captured_buffer,
            last_for: 0.0,
            input_device_name: "Default".to_string(),
            output_device_name: "Default".to_string(),
            drain_graphs,
        }
    }

    fn plot(&mut self) {
        self.is_playing.store(true, Ordering::SeqCst);
        self.start_time = Instant::now();
    }

    fn stop(&mut self) {
        self.is_playing.store(false, Ordering::SeqCst);
        self.started_sound = false;
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
        let duration_clone = duration.clone();
        let is_playing = self.is_playing.clone();
        let sound = Chirp::new(DEFAULT_SAMPLE_RATE, 500.0, 20000.0, duration_clone);
        self.current_chirp = sound.clone();
        spawn(move || {
            audio::play_output(output_device_name, sound, is_playing);
        });

        // Start the wave capturing thread.
        let is_playing = self.is_playing.clone();
        spawn(move || {
            audio::capture_input(
                input_device_name,
                DEFAULT_SAMPLE_RATE,
                captured_buffer,
                for_tx,
                is_playing,
            )
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

    fn paint_drain_graphs_checkbox(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.drain_graphs, "Drain graphs");
        });
    }
    fn paint_duration_input(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Duration: ");
            if self.is_playing.load(Ordering::SeqCst) {
                ui.disable();
            }
            let mut val = format!("{}", self.duration).to_string();
            ui.add(egui::TextEdit::singleline(&mut val));
            ui.label("Seconds");
            if val == "" {
                self.duration = 0.0;
            }
            if let Ok(parsed_val) = val.parse::<f32>() {
                self.duration = parsed_val;
            } else {
                ui.colored_label(
                    egui::Color32::RED,
                    "Invalid input, it should be floating number in the form of 100.0",
                );
            }
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

    fn paint_output_wave(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::horizontal().show(ui, |ui| {
            let mut points_to_plot = self.points_vector.clone();
            let points_to_plot_len = points_to_plot.len();
            if self.drain_graphs {
                let downsampled_sample_rate =
                    (DEFAULT_SAMPLE_RATE / DEFAULT_DOWNSAMPLE_FACTOR) as usize;
                if points_to_plot.len() > downsampled_sample_rate * 5 {
                    points_to_plot.drain(0..points_to_plot_len - downsampled_sample_rate * 5);
                }
            }
            Plot::new("Sine Wave")
                .height(240.0)
                .allow_drag(true)
                .show(ui, |plot_ui| {
                    plot_ui.line(Line::new(PlotPoints::new(points_to_plot)));
                });
        });
    }

    fn paint_frequency_of_resonance(&self, ui: &mut egui::Ui) {
        ui.label(format!("Frequency of resonance: {:.2} Hz", self.last_for));
    }

    fn update_outgoing_wave_graph(&mut self) {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let max_time = elapsed.min(self.duration);

        // Plot the sine wave over time
        let samples_to_show = (max_time * DEFAULT_SAMPLE_RATE) as usize;

        let downsample_factor = DEFAULT_DOWNSAMPLE_FACTOR as usize;
        let chirp_segement = self
            .current_chirp
            .clone()
            .enumerate()
            .filter(|(i, _)| i % downsample_factor == 0)
            .take(samples_to_show / downsample_factor)
            .map(|(_, val)| val)
            .collect::<Vec<f32>>();

        let points: Vec<[f64; 2]> = chirp_segement
            .iter()
            .enumerate()
            .map(|(i, &val)| {
                // Multiply by tghe downsample_factor to restore the units to seconds.
                let time = (i * downsample_factor) as f32 / DEFAULT_SAMPLE_RATE;
                [time as f64, val as f64]
            })
            .collect();
        self.points_vector = points;
        // Stop playing after 15 seconds
        if elapsed >= self.duration {
            self.is_playing.store(false, Ordering::SeqCst);
            self.started_sound = false;
        }
    }
}

impl eframe::App for MainUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::scroll_area::ScrollArea::vertical().show(ui, |ui| {
                self.paint_window_title(ui);
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("Chrip controls"));
                            self.paint_duration_input(ui);
                        });
                    });
                });
                ui.horizontal(|ui| {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("Play controls"));
                            self.paint_sound_devices_dropdown(ui);
                            self.paint_drain_graphs_checkbox(ui);
                            self.paint_start_and_stop_buttons(ui);
                        });
                    });
                });
                self.paint_output_wave(ui);

                if self.is_playing.load(Ordering::SeqCst) {
                    self.start_sound();
                    self.update_outgoing_wave_graph();
                    ui.label("Calculating frequency of resonance...");
                }

                let captured_buffer = self.captured_buffer.lock().unwrap();
                ui.label(format!("Captured buffer len {}", captured_buffer.len()));
                if let Ok(freq) = self.for_rx.try_recv() {
                    self.last_for = freq;
                }
                if !self.is_playing.load(Ordering::SeqCst) {
                    self.paint_frequency_of_resonance(ui);
                }

                let buffer_to_plot = captured_buffer.clone();
                let buf_len = buffer_to_plot.len();

                let mut points: Vec<[f64; 2]> = buffer_to_plot
                    .into_iter()
                    .enumerate()
                    .map(|(i, x)| [(i as f32 / 44100.0) as f64, x as f64])
                    .collect();

                if self.drain_graphs {
                    if buf_len > 44100 * 5 {
                        points.drain(0..buf_len - 44100 * 5);
                    }
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

fn main() {
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "Caliber",
        native_options,
        Box::new(|cc| Ok(Box::new(MainUI::new(cc)))),
    );
}

#[cfg(test)]
mod tests {
    use rodio::{source::SineWave, source::Source, OutputStream, Sink};

    #[test]
    fn test_make_a4_sound() {
        let a4_freq = 440.0;
        let sine_wave = SineWave::new(a4_freq).take_duration(std::time::Duration::from_secs(2));
        // Play the sound for 2 seconds through the speakers
        // // Get an output stream handle to the default physical sound device
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        // Load a sound from a file, using a path relative to Cargo.toml
        // Play the sound directly on the device
        sink.append(sine_wave);
        sink.sleep_until_end();
    }
}
