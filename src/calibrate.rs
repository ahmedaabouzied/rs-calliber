// GUI
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

// Audio
use cpal::traits::DeviceTrait;

use crate::audio;
use crate::chirp::Chirp;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::spawn;
use std::time::Instant;
use std::vec::Vec;

use crate::utils::Result;

// Constants
const DEFAULT_SAMPLE_RATE: f32 = 192000.0;
const DEFAULT_CAPTURED_INPUT_SAMPLE_RATE: f32 = 44100.0;
const DEFAULT_DOWNSAMPLE_FACTOR: f32 = 1000.0;

pub struct CalibrateTab {
    current_chirp: Option<Chirp>,
    duration: Option<f32>,
    chirp_start: Option<f32>,
    chirp_end: Option<f32>,
    output_sample_rate: Option<f32>,
    captured_input_sample_rate: f32,
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
    tasker: crate::task::Tasker,
    status_tx: tokio::sync::mpsc::Sender<String>,
}

impl CalibrateTab {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        status_tx: tokio::sync::mpsc::Sender<String>,
    ) -> Self {
        let captured_input_sample_rate = DEFAULT_CAPTURED_INPUT_SAMPLE_RATE;
        let start_time = Instant::now();
        let is_playing = Arc::new(AtomicBool::new(false));
        let started_sound = false;
        let points_vector = vec![];
        let (for_tx, for_rx): (Sender<f32>, Receiver<f32>) = mpsc::channel();
        let captured_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
        let drain_graphs = true;
        Self {
            chirp_start: None,
            chirp_end: None,
            output_sample_rate: None,
            current_chirp: None,
            duration: None,
            captured_input_sample_rate,
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
            tasker: crate::task::Tasker::new(),
            status_tx,
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

    fn start_sound(&mut self) -> Result<()> {
        if self.started_sound {
            return Ok(());
        }
        self.started_sound = true;
        if self.current_chirp.is_none() {
            let tx = self.status_tx.clone();
            self.tasker.spawn(async move {
                tx.send("Please choose a chirp input file".to_string())
                    .await
                    .unwrap_or_else(|e| eprintln!("{}", e));
            });
            return Ok(());
        }
        // Play the sound in a separate thread
        let for_tx = self.for_tx.clone();
        let captured_buffer = self.captured_buffer.clone();
        let input_device_name = self.input_device_name.clone();
        let output_device_name = self.output_device_name.clone();

        // Start the wave playing thread.
        let is_playing = self.is_playing.clone();
        let sound = self.current_chirp.clone().ok_or("no chirp found")?;
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
        Ok(())
    }

    fn paint_start_and_stop_buttons(&mut self, ui: &mut egui::Ui) -> Result<()> {
        ui.horizontal(|ui| {
            if ui.button("Start").clicked() {
                self.plot();
            };
            if ui.button("Stop").clicked() {
                self.stop();
            };
            if self.is_playing.load(Ordering::SeqCst) {
                ui.disable();
            }
            if ui.button("Clear").clicked() {
                self.points_vector.clear();
                if let Ok(mut buffer) = self.captured_buffer.lock() {
                    buffer.clear();
                    return;
                }
                self.captured_buffer = Arc::new(Mutex::new(Vec::new()));
            }
        });
        Ok(())
    }

    fn send_error(&mut self, msg: String) {
        let tx = self.status_tx.clone();
        self.tasker.spawn(async move {
            tx.send(format!("error: {}", msg))
                .await
                .unwrap_or_else(|e| {
                    eprintln!("{}", e);
                })
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
            if self.current_chirp.is_some() {
                ui.label(format!(
                    "{} s",
                    self.duration.unwrap_or_else(|| {
                        self.send_error("duration not found".to_string());
                        0.0
                    })
                ));
            }
        });
    }

    fn paint_chirp_start_input(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Chrip start frequency: ");
            if self.current_chirp.is_some() {
                ui.label(format!(
                    "{} Hz",
                    self.chirp_start.unwrap_or_else(|| {
                        self.send_error("chrip start frequency not found".to_string());
                        0.0
                    })
                ));
            }
        });
    }

    fn paint_chirp_end_input(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Chrip end frequency: ");
            if self.current_chirp.is_some() {
                ui.label(format!(
                    "{} Hz",
                    self.chirp_end.unwrap_or_else(|| {
                        self.send_error("chirp end frequency not found".to_string());
                        0.0
                    })
                ));
            }
        });
    }

    fn paint_input_file_input(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("File input: ");
            if ui.button("Select input file").clicked {
                let file = rfd::FileDialog::new()
                    .add_filter("wav", &["wav"])
                    .set_directory("/")
                    .pick_file();
                if file.is_none() {
                    let tx = self.status_tx.clone();
                    self.tasker.spawn(async move {
                        tx.send("no file selected".to_string())
                            .await
                            .unwrap_or_else(|e| eprintln!("{}", e));
                    });
                    return;
                }
                let file = match file {
                    Some(v) => v,
                    None => {
                        self.send_error("corrupted file".to_string());
                        return;
                    }
                };
                let path = file.to_path_buf();
                ui.label(format!(
                    "{}",
                    match path.to_str() {
                        Some(v) => v,
                        None => {
                            self.send_error("corrupted file path".to_string());
                            return;
                        }
                    }
                ));
                let wav_data = match hound::WavReader::open(path) {
                    Ok(v) => v,
                    Err(e) => {
                        self.send_error(e.to_string());
                        return;
                    }
                };
                let chirp = match crate::chirp::Chirp::try_from(wav_data) {
                    Ok(v) => v,
                    Err(e) => {
                        self.send_error(e.to_string());
                        return;
                    }
                };
                self.duration = Some(chirp.duration.clone());
                self.output_sample_rate = Some(chirp.sample_rate.clone());
                self.chirp_start = Some(chirp.start_freq.clone());
                self.chirp_end = Some(chirp.end_freq.clone());
                self.current_chirp = Some(chirp);
            };
        });
    }

    fn paint_output_sample_rate_input(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Chrip sample rate: ");
            if self.current_chirp.is_some() {
                ui.label(format!(
                    "{} sample/second",
                    self.output_sample_rate.unwrap_or(0.0)
                ));
            }
        });
    }

    fn paint_captured_input_sample_rate(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Captured input sample rate: ");
            if self.is_playing.load(Ordering::SeqCst) {
                ui.disable();
            }
            let mut val = format!("{}", self.captured_input_sample_rate).to_string();
            ui.add(egui::TextEdit::singleline(&mut val));
            ui.label("Hz");
            if val == "" {
                self.captured_input_sample_rate = 0.0;
            }
            if let Ok(parsed_val) = val.parse::<f32>() {
                self.captured_input_sample_rate = parsed_val;
            } else {
                ui.colored_label(
                    egui::Color32::RED,
                    "Invalid input, it should be floating number in the form of 100.0",
                );
            }
        });
    }

    fn paint_sound_devices_dropdown(&mut self, ui: &mut egui::Ui) -> Result<()> {
        let input_devices = audio::get_input_devices()?;
        let output_devices = audio::get_output_devices()?;

        ui.horizontal(|ui| {
            ui.label("Input device:");
            egui::ComboBox::new("input_device", "")
                .selected_text(self.input_device_name.to_string())
                .show_ui(ui, |ui| {
                    for kind in input_devices {
                        ui.selectable_value(
                            &mut self.input_device_name,
                            kind.name().unwrap_or("Default".to_string()),
                            kind.name().unwrap_or("Default".to_string()),
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
                            kind.name().unwrap_or("Default".to_string()),
                            kind.name().unwrap_or("Default".to_string()),
                        );
                    }
                });
        });
        Ok(())
    }

    fn paint_output_wave(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::horizontal().show(ui, |ui| {
            let mut points_to_plot = self.points_vector.clone();
            let points_to_plot_len = points_to_plot.len();
            if self.drain_graphs {
                let downsampled_sample_rate =
                    (self.output_sample_rate.unwrap_or(DEFAULT_SAMPLE_RATE)
                        / DEFAULT_DOWNSAMPLE_FACTOR) as usize;
                if points_to_plot.len() > downsampled_sample_rate * 5 {
                    points_to_plot.drain(0..points_to_plot_len - downsampled_sample_rate * 5);
                }
            }

            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(format!(
                        "Chirp output downsampled with {} factor",
                        DEFAULT_DOWNSAMPLE_FACTOR
                    )));
                    Plot::new("Sine Wave")
                        .height(240.0)
                        .allow_drag(true)
                        .show(ui, |plot_ui| {
                            plot_ui.line(Line::new(PlotPoints::new(points_to_plot)));
                        });
                });
                ui.horizontal(|ui| {
                    if ui.button("Export to wav").clicked {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("captured.wav")
                            .set_can_create_directories(true)
                            .save_file()
                        {
                            let tx = self.status_tx.clone();
                            let captured_buffer = match self.current_chirp.clone() {
                                Some(v) => v.samples,
                                None => {
                                    self.send_error("empty chirp buffer".to_string());
                                    return;
                                }
                            };
                            let sample_rate = self.captured_input_sample_rate as u32;
                            self.tasker.spawn(async move {
                                tx.send("Saving wav file".to_string())
                                    .await
                                    .unwrap_or_else(|e| {
                                        eprintln!("error: {}", e);
                                        return;
                                    });
                                audio::save_mono_vec_to_wav(&captured_buffer, sample_rate, &path)
                                    .unwrap_or_else(|e| {
                                        eprintln!("error: {}", e);
                                        return;
                                    });
                                tx.send("Done saving wav file".to_string())
                                    .await
                                    .unwrap_or_else(|e| {
                                        eprintln!("error: {}", e);
                                        return;
                                    });
                            });
                        }
                    };
                    if ui.button("Export to CSV (Excel)").clicked {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("captured.csv")
                            .set_can_create_directories(true)
                            .save_file()
                        {
                            let tx = self.status_tx.clone();
                            let captured_buffer = match self.current_chirp.clone() {
                                Some(v) => v.samples,
                                None => {
                                    self.send_error("failed to save captured buffer".to_string());
                                    return;
                                }
                            };
                            let sample_rate = self.captured_input_sample_rate as u32;
                            self.tasker.spawn(async move {
                                tx.send("Saving csv file".to_string())
                                    .await
                                    .unwrap_or_else(|e| {
                                        eprintln!("error: {}", e);
                                        return;
                                    });
                                audio::save_mono_vec_with_db_to_csv(
                                    &captured_buffer,
                                    sample_rate,
                                    &path,
                                )
                                .await
                                .unwrap_or_else(|e| {
                                    eprintln!("{}", e);
                                    return;
                                });
                                tx.send("Done saving csv file".to_string())
                                    .await
                                    .unwrap_or_else(|e| {
                                        eprintln!("error: {}", e);
                                        return;
                                    });
                            });
                        }
                    };
                });
            });
        });
    }

    fn paint_frequency_of_resonance(&self, ui: &mut egui::Ui) {
        ui.label(format!("Frequency of resonance: {:.2} Hz", self.last_for));
    }

    fn update_outgoing_wave_graph(&mut self) -> Result<()> {
        if self.current_chirp.is_none() {
            return Ok(());
        }
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let max_time = elapsed.min(self.duration.ok_or("duration is null")?);

        // Plot the sine wave over time
        let samples_to_show =
            (max_time * self.output_sample_rate.ok_or("sample rate is null")?) as usize;

        let downsample_factor = DEFAULT_DOWNSAMPLE_FACTOR as usize;
        let chirp_segement = self
            .current_chirp
            .clone()
            .ok_or("chirp is null")?
            .enumerate()
            .filter(|(i, _)| i % downsample_factor == 0)
            .take(samples_to_show / downsample_factor)
            .map(|(_, val)| val)
            .collect::<Vec<f32>>();

        let mut points: Vec<[f64; 2]> = Vec::new();
        for (i, &val) in chirp_segement.iter().enumerate() {
            let time = (i * downsample_factor) as f32
                / self.output_sample_rate.ok_or("sample rate is null")?;
            points.push([time as f64, val as f64]);
        }
        self.points_vector = points;
        // Stop playing after 15 seconds
        if elapsed >= self.duration.ok_or("duration is null")? {
            self.is_playing.store(false, Ordering::SeqCst);
            self.started_sound = false;
        }
        Ok(())
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) -> Result<()> {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("Chrip controls"));
                    self.paint_duration_input(ui);
                    self.paint_chirp_start_input(ui);
                    self.paint_chirp_end_input(ui);
                    self.paint_output_sample_rate_input(ui);
                    self.paint_captured_input_sample_rate(ui);
                    self.paint_input_file_input(ui);
                });
            });
        });
        ui.add_space(20.0);
        ui.horizontal(|ui| {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("Play controls"));
                    self.paint_sound_devices_dropdown(ui)
                        .unwrap_or_else(|e| self.send_error(e.to_string()));
                    self.paint_drain_graphs_checkbox(ui);
                    self.paint_start_and_stop_buttons(ui)
                        .unwrap_or_else(|e| self.send_error(e.to_string()));
                });
            });
        });
        ui.add_space(20.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("Results"));
                if !self.is_playing.load(Ordering::SeqCst) {
                    self.paint_frequency_of_resonance(ui);
                } else {
                    ui.label("Capturing input ...");
                }
            });
        });
        ui.add_space(20.0);
        self.paint_output_wave(ui);

        if self.is_playing.load(Ordering::SeqCst) {
            self.start_sound()?;
            self.update_outgoing_wave_graph()
                .unwrap_or_else(|e| self.send_error(e.to_string()));
            ui.label("Calculating frequency of resonance...");
        }

        let mut buffer_to_plot = Vec::new();
        {
            if let Ok(captured_buffer) = self.captured_buffer.lock() {
                if let Ok(freq) = self.for_rx.try_recv() {
                    self.last_for = freq;
                }
                buffer_to_plot = captured_buffer.clone();
            };
        }

        let buf_len = buffer_to_plot.len();

        let mut points: Vec<[f64; 2]> = buffer_to_plot
            .into_iter()
            .enumerate()
            .map(|(i, x)| {
                [
                    (i as f32 / self.captured_input_sample_rate) as f64,
                    x as f64,
                ]
            })
            .collect();

        if self.drain_graphs {
            if buf_len > self.captured_input_sample_rate as usize * 5 {
                points.drain(0..buf_len - self.captured_input_sample_rate as usize * 5);
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
                            let tx = self.status_tx.clone();
                            if let Ok(captured_buffer) = self.captured_buffer.lock() {
                                let captured_buffer = captured_buffer.clone();
                                let sample_rate = self.captured_input_sample_rate as u32;
                                self.tasker.spawn(async move {
                                    tx.send("Saving wav file".to_string()).await.unwrap_or_else(
                                        |e| {
                                            eprintln!("{}", e);
                                            return;
                                        },
                                    );
                                    audio::save_mono_vec_to_wav(
                                        &captured_buffer,
                                        sample_rate,
                                        &path,
                                    )
                                    .unwrap_or_else(|e| {
                                        eprintln!("{}", e);
                                        return;
                                    });
                                    tx.send("Done saving wav file".to_string())
                                        .await
                                        .unwrap_or_else(|e| {
                                            eprintln!("{}", e);
                                            return;
                                        });
                                });
                            };
                        }
                    };
                    if ui.button("Export to CSV (Excel)").clicked {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("captured.csv")
                            .set_can_create_directories(true)
                            .save_file()
                        {
                            let tx = self.status_tx.clone();
                            if let Ok(captured_buffer) = self.captured_buffer.lock() {
                                let captured_buffer = captured_buffer.clone();
                                let sample_rate = self.captured_input_sample_rate as u32;
                                self.tasker.spawn(async move {
                                    tx.send("Saving csv file".to_string()).await.unwrap_or_else(
                                        |e| {
                                            eprintln!("{}", e);
                                            return;
                                        },
                                    );
                                    audio::save_mono_vec_with_db_to_csv(
                                        &captured_buffer,
                                        sample_rate,
                                        &path,
                                    )
                                    .await
                                    .unwrap_or_else(|e| {
                                        eprintln!("{}", e);
                                        return;
                                    });
                                    tx.send("Done saving csv file".to_string())
                                        .await
                                        .unwrap_or_else(|e| {
                                            eprintln!("{}", e);
                                            return;
                                        });
                                });
                            };
                        }
                    };
                });
                // Request a repaint to keep the animation running
                ctx.request_repaint();
            });
        });
        Ok(())
    }
}
