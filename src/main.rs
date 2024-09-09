use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use std::f32::consts::PI;
use std::time::Instant;

// Constants
const A4_FREQ: f32 = 440.0;
const SAMPLE_RATE: f32 = 44100.0; // Standard audio sample rate
const DURATION: f32 = 15.0; // 15 seconds for the A4 note

fn generate_sine_wave(frequency: f32, duration: f32, sample_rate: f32) -> Vec<f32> {
    let total_samples = (duration * sample_rate) as usize;
    (0..total_samples)
        .map(|x| {
            let t = x as f32 / sample_rate;
            (t * frequency * 2.0 * PI).sin()
        })
        .collect()
}

struct MyEguiApp {
    sine_wave: Vec<f32>,
    is_playing: bool,
    start_time: Instant,
    last_time: f32,
    zoom_factor: f32,
    x_offset: f32,
    points_vector: Vec<[f64; 2]>,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let sine_wave = generate_sine_wave(A4_FREQ, DURATION, SAMPLE_RATE);
        let start_time = Instant::now();
        let is_playing = false;
        let last_time = 0.0;
        let zoom_factor = 1.0;
        let x_offset = 0.0;
        let points_vector = vec![];
        Self {
            sine_wave,
            is_playing,
            start_time,
            last_time,
            zoom_factor,
            x_offset,
            points_vector,
        }
    }
}

impl MyEguiApp {
    fn plot(&mut self) {
        self.is_playing = true;
        self.start_time = Instant::now();
    }

    fn stop(&mut self) {
        self.is_playing = false;
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
                let elapsed = self.start_time.elapsed().as_secs_f32();
                let max_time = elapsed.min(DURATION);

                // Plot the sine wave over time
                let samples_to_show = (max_time * SAMPLE_RATE) as usize;
                let sine_wave_segment = &self.sine_wave[..samples_to_show];

                let points: Vec<[f64; 2]> = sine_wave_segment
                    .iter()
                    .enumerate()
                    .map(|(i, &val)| {
                        let time = (i as f32 / SAMPLE_RATE);
                        [time as f64, val as f64]
                    })
                    .collect();
                self.points_vector = points;

                // Stop playing after 15 seconds
                if elapsed >= DURATION {
                    self.is_playing = false;
                }
            }

            // Request a repaint to keep the animation running
            if self.is_playing {
                ctx.request_repaint();
            }
        });
    }
}

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Caliber",
        native_options,
        Box::new(|cc| Ok(Box::new(MyEguiApp::new(cc)))),
    );
}
