use crate::audio;
use cpal::traits::DeviceTrait;
use egui_plot::{Line, Plot, PlotPoints};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::spawn;
use std::time::Instant;

#[derive(Debug)]
pub struct DetectTab {
    sine_wave_freq: f32,
    output_sample_rate: f32,
    duration: f32,
    sine_wave: crate::wave::Wave,
    points_vector: Vec<[f64; 2]>,
    down_sample_factor: f32,
    start_time: Instant,

    input_device_name: String,
    output_device_name: String,

    drain_graphs: bool,
    is_playing: Arc<AtomicBool>,
    started_playing: bool,
}

impl DetectTab {
    pub fn new() -> Self {
        let sine_wave_freq: f32 = 441.0; // Default to A4 note.

        Self {
            sine_wave_freq,
            points_vector: Vec::new(),
            output_sample_rate: 192000.0,
            down_sample_factor: 100.0,
            duration: 5.0,
            input_device_name: "Default".to_string(),
            output_device_name: "Default".to_string(),
            drain_graphs: true,
            start_time: Instant::now(),
            is_playing: Arc::new(AtomicBool::new(false)),
            started_playing: false,
            sine_wave: crate::wave::Wave::new(192000.0, sine_wave_freq, 5.0),
        }
    }

    fn paint_output_sample_rate_input(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Output wave sample rate: ");
            if self.is_playing.load(Ordering::SeqCst) {
                ui.disable();
            }
            let mut val = format!("{}", self.output_sample_rate).to_string();
            ui.add(egui::TextEdit::singleline(&mut val));
            ui.label("Hz");
            if val == "" {
                self.output_sample_rate = 0.0;
            }
            if let Ok(parsed_val) = val.parse::<f32>() {
                self.output_sample_rate = parsed_val;
            } else {
                ui.colored_label(
                    egui::Color32::RED,
                    "Invalid input, it should be floating number in the form of 100.0",
                );
            }
        });
    }

    fn paint_output_freq_input(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Output wave frequency: ");
            if self.is_playing.load(Ordering::SeqCst) {
                ui.disable();
            }
            let mut val = format!("{}", self.sine_wave_freq).to_string();
            ui.add(egui::TextEdit::singleline(&mut val));
            ui.label("Hz");
            if val == "" {
                self.sine_wave_freq = 0.0;
            }
            if let Ok(parsed_val) = val.parse::<f32>() {
                self.sine_wave_freq = parsed_val;
            } else {
                ui.colored_label(
                    egui::Color32::RED,
                    "Invalid input, it should be floating number in the form of 100.0",
                );
            }
        });
    }

    fn start_sound(&mut self) {
        if self.started_playing {
            return;
        }
        self.started_playing = true;
        self.start_time = Instant::now();
        // Play the sound in a separate thread
        let output_device_name = self.output_device_name.clone();

        // Start the wave playing thread.
        let is_playing = self.is_playing.clone();
        let wave =
            crate::wave::Wave::new(self.output_sample_rate, self.sine_wave_freq, self.duration);
        self.sine_wave = wave.clone();
        spawn(move || {
            audio::play_output(output_device_name, wave, is_playing);
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
            ui.label("Hz");
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

    fn update_outgoing_wave_graph(&mut self) {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let max_time = elapsed.min(self.duration);

        // Plot the sine wave over time
        let samples_to_show = (max_time * self.output_sample_rate) as usize;

        let downsample_factor = self.down_sample_factor as usize;
        let segment = self
            .sine_wave
            .clone()
            .enumerate()
            .filter(|(i, _)| i % downsample_factor == 0)
            .take(samples_to_show / downsample_factor)
            .map(|(_, val)| val)
            .collect::<Vec<f32>>();

        let points: Vec<[f64; 2]> = segment
            .iter()
            .enumerate()
            .map(|(i, &val)| {
                // Multiply by tghe downsample_factor to restore the units to seconds.
                let time = (i * downsample_factor) as f32 / self.output_sample_rate;
                [time as f64, val as f64]
            })
            .collect();
        self.points_vector = points;
        // Stop playing after 15 seconds
        if elapsed >= self.duration {
            self.is_playing.store(false, Ordering::SeqCst);
            self.started_playing = false;
        }
    }

    fn paint_output_wave(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::horizontal().show(ui, |ui| {
            let mut points_to_plot = self.points_vector.clone();
            let points_to_plot_len = points_to_plot.len();
            if self.drain_graphs {
                let downsampled_sample_rate =
                    (self.output_sample_rate / self.down_sample_factor) as usize;
                if points_to_plot.len() > downsampled_sample_rate * 5 {
                    points_to_plot.drain(0..points_to_plot_len - downsampled_sample_rate * 5);
                }
            }

            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("Output"));
                    Plot::new("Sine Wave")
                        .height(240.0)
                        .allow_drag(true)
                        .show(ui, |plot_ui| {
                            plot_ui.line(Line::new(PlotPoints::new(points_to_plot)));
                        });
                });
            });
        });
    }

    fn paint_drain_graphs_checkbox(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.drain_graphs, "Drain graphs");
        });
    }

    fn paint_start_and_stop_buttons(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Start").clicked() {
                self.is_playing.store(true, Ordering::SeqCst);
            };
            if ui.button("Stop").clicked() {
                self.is_playing.store(false, Ordering::SeqCst);
                self.started_playing = false;
            };
        });
    }

    pub fn render(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.with_layout(
                egui::Layout::top_down_justified(egui::Align::Center),
                |ui| {
                    ui.label(egui::RichText::new("Output wave controls"));
                    self.paint_output_sample_rate_input(ui);
                    self.paint_output_freq_input(ui);
                    self.paint_duration_input(ui);
                },
            );
        });

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.with_layout(
                egui::Layout::top_down_justified(egui::Align::Center),
                |ui| {
                    ui.label(egui::RichText::new("Sound controls"));
                    self.paint_sound_devices_dropdown(ui);
                    self.paint_drain_graphs_checkbox(ui);
                    self.paint_start_and_stop_buttons(ui);
                },
            );
        });

        ui.add_space(20.0);
        self.paint_output_wave(ui);

        if self.is_playing.load(Ordering::SeqCst) {
            self.start_sound();
            self.update_outgoing_wave_graph();
        }

        ctx.request_repaint();
    }
}
