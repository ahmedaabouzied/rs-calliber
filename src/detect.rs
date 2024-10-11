use crate::audio;
use cpal::traits::DeviceTrait;
use egui_plot::{Line, Plot, PlotPoints};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
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
    captured_sample_rate: f32,
    duration: f32,
    sine_wave: crate::wave::Wave,
    captured_buffer: Arc<Mutex<Vec<f32>>>,
    points_vector: Vec<[f64; 2]>,
    down_sample_factor: f32,
    start_time: Instant,
    for_tx: Sender<f32>,
    for_rx: Receiver<f32>,

    input_device_name: String,
    output_device_name: String,

    drain_graphs: bool,
    is_playing: Arc<AtomicBool>,
    started_playing: bool,
}

impl DetectTab {
    pub fn new() -> Self {
        let sine_wave_freq: f32 = 441.0; // Default to A4 note.
        let (for_tx, for_rx): (Sender<f32>, Receiver<f32>) = mpsc::channel();

        Self {
            sine_wave_freq,
            points_vector: Vec::new(),
            output_sample_rate: 192000.0,
            captured_sample_rate: 192000.0,
            down_sample_factor: 100.0,
            duration: 5.0,
            input_device_name: "Default".to_string(),
            output_device_name: "Default".to_string(),
            drain_graphs: true,
            start_time: Instant::now(),
            is_playing: Arc::new(AtomicBool::new(false)),
            started_playing: false,
            sine_wave: crate::wave::Wave::new(192000.0, sine_wave_freq, 5.0),
            captured_buffer: Arc::new(Mutex::new(Vec::<f32>::new())),
            for_tx,
            for_rx,
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

        let for_tx = self.for_tx.clone();
        let captured_buffer = self.captured_buffer.clone();

        // Play the sound in a separate thread
        let output_device_name = self.output_device_name.clone();
        let input_device_name = self.input_device_name.clone();

        // Start the wave playing thread.
        let is_playing = self.is_playing.clone();
        let wave =
            crate::wave::Wave::new(self.output_sample_rate, self.sine_wave_freq, self.duration);
        self.sine_wave = wave.clone();
        spawn(move || {
            audio::play_output(output_device_name, wave, is_playing);
        });

        // Start the wave capturing thread.
        let is_playing = self.is_playing.clone();
        let sample_rate = self.captured_sample_rate.clone();

        spawn(move || {
            audio::capture_input(
                input_device_name,
                sample_rate,
                captured_buffer,
                for_tx,
                is_playing,
            )
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

    fn paint_captured_input_sample_rate(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Captured input sample rate: ");
            if self.is_playing.load(Ordering::SeqCst) {
                ui.disable();
            }
            let mut val = format!("{}", self.captured_sample_rate).to_string();
            ui.add(egui::TextEdit::singleline(&mut val));
            ui.label("Hz");
            if val == "" {
                self.captured_sample_rate = 0.0;
            }
            if let Ok(parsed_val) = val.parse::<f32>() {
                self.captured_sample_rate = parsed_val;
            } else {
                ui.colored_label(
                    egui::Color32::RED,
                    "Invalid input, it should be floating number in the form of 100.0",
                );
            }
        });
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
            if self.is_playing.load(Ordering::SeqCst) {
                ui.disable();
            }
            if ui.button("Clear").clicked() {
                self.points_vector.clear();
                self.captured_buffer.lock().unwrap().clear();
            }
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
                    self.paint_captured_input_sample_rate(ui);
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

        let mut buffer_to_plot = Vec::new();
        {
            let captured_buffer = self.captured_buffer.lock().unwrap();
            buffer_to_plot = captured_buffer.clone();
        }

        let buf_len = buffer_to_plot.len();

        let mut points: Vec<[f64; 2]> = buffer_to_plot
            .into_iter()
            .enumerate()
            .map(|(i, x)| [(i as f32 / self.captured_sample_rate) as f64, x as f64])
            .collect();

        if self.drain_graphs {
            if buf_len > self.captured_sample_rate as usize * 5 {
                points.drain(0..buf_len - self.captured_sample_rate as usize * 5);
            }
        }
        ui.add_space(20.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("Captured Input"));
                let line = Line::new(PlotPoints::new(points));
                let plot = Plot::new("Received audio").height(240.0);
                plot.show(ui, |plot_ui| {
                    plot_ui.line(line);
                });
                if self.is_playing.load(Ordering::SeqCst) {
                    ui.disable();
                }
                ui.horizontal(|ui| {
                    if ui.button("Export to wav").clicked {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("captured.wav")
                            .set_can_create_directories(true)
                            .save_file()
                        {
                            let captured_buffer = self.captured_buffer.lock().unwrap();
                            let sample_rate = self.captured_sample_rate as u32;
                            audio::save_mono_vec_to_wav(&captured_buffer, sample_rate, &path)
                                .unwrap();
                        }
                    };
                    if ui.button("Export to CSV (Excel)").clicked {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("captured.csv")
                            .set_can_create_directories(true)
                            .save_file()
                        {
                            let captured_buffer = self.captured_buffer.lock().unwrap();
                            let sample_rate = self.captured_sample_rate as u32;
                            audio::save_mono_vec_with_db_to_csv(
                                &captured_buffer,
                                sample_rate,
                                &path,
                            )
                            .unwrap();
                        }
                    };
                });
                // Request a repaint to keep the animation running
                ctx.request_repaint();
            });
        });
    }
}
