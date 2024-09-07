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
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let sine_wave = generate_sine_wave(A4_FREQ, DURATION, SAMPLE_RATE);
        let start_time = Instant::now();
        let is_playing = false;
        let last_time = 0.0;
        Self {
            sine_wave,
            is_playing,
            start_time,
            last_time,
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Caliber");

            if ui.button("Plot").clicked() {
                self.is_playing = true;
                self.start_time = Instant::now();
            }

            if ui.button("Play").clicked() {
                self.is_playing = true;
                self.start_time = Instant::now();
            }

            if self.is_playing {
                let elapsed = self.start_time.elapsed().as_secs_f32();
                let max_time = elapsed.min(DURATION);

                // Plot the sine wave over time
                let samples_to_show = (max_time * SAMPLE_RATE) as usize;
                let sine_wave_segment = &self.sine_wave[..samples_to_show];

                let points: PlotPoints = sine_wave_segment
                    .iter()
                    .enumerate()
                    .map(|(i, &val)| {
                        let time = i as f32 / SAMPLE_RATE;
                        [time as f64, val as f64]
                    })
                    .collect();

                // Create the line to plot
                let line = Line::new(points);

                // Plot the wave
                Plot::new("Sine Wave")
                    .view_aspect(2.0) // Aspect ratio of the plot
                    .show(ui, |plot_ui| {
                        plot_ui.line(line);
                    });

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
