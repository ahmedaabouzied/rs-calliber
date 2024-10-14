// GUI
use eframe::egui;

mod audio;
mod calibrate;
mod chirp;
mod detect;
mod freq;
mod task;
mod utils;
mod wave;

use utils::Result;

struct MainUI {
    selected_tab: u8,
    detect_tab: detect::DetectTab,
    calibrate_tab: calibrate::CalibrateTab,
    status: String,
    status_timeout: std::time::Duration,
    status_updated_at: std::time::Instant,
    status_rx: tokio::sync::mpsc::Receiver<String>,
    status_tx: tokio::sync::mpsc::Sender<String>,
}

impl MainUI {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (status_tx, status_rx) = tokio::sync::mpsc::channel::<String>(1);
        Self {
            selected_tab: 0, // Default on the calibration page.
            detect_tab: detect::DetectTab::new(status_tx.clone()),
            calibrate_tab: calibrate::CalibrateTab::new(_cc, status_tx.clone()),
            status: "Running".to_string(),
            status_timeout: std::time::Duration::from_secs(3),
            status_updated_at: std::time::Instant::now(),
            status_rx,
            status_tx,
        }
    }

    fn update_status(&mut self) {
        match self.status_rx.try_recv() {
            Ok(v) => {
                self.status = v;
                self.status_updated_at = std::time::Instant::now();
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {}
            Err(e) => {
                self.status = e.to_string();
                self.status_updated_at = std::time::Instant::now();
            }
        }
        if self.status_updated_at.elapsed() > self.status_timeout && self.status.contains("Done") {
            self.status_updated_at = std::time::Instant::now();
            self.status = "Running".to_string();
        }
    }

    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
        ui: &mut egui::Ui,
    ) -> Result<()> {
        egui::scroll_area::ScrollArea::vertical().show(ui, |ui| match self.selected_tab {
            0 => self
                .calibrate_tab
                .render(ui, ctx, _frame)
                .unwrap_or_else(|e| {
                    self.status = e.to_string();
                    ()
                }),
            1 => self.detect_tab.render(ui, ctx, _frame).unwrap_or_else(|e| {
                self.status = e.to_string();
                ()
            }),
            _ => (),
        });
        self.update_status();
        Ok(())
    }

    fn show_error_popup(&mut self, ctx: &egui::Context, msg: String) {
        let screen_rect = ctx.screen_rect();
        let error_window_rect = egui::Rect::from_min_size(
            screen_rect.center() - egui::vec2(100.0, 0.0),
            egui::vec2(200.0, 50.0),
        );
        egui::Window::new("Error")
            .collapsible(false)
            .title_bar(true)
            .fixed_rect(error_window_rect)
            .show(ctx, |ui| {
                ui.label(msg);
                if ui.button("Dismiss").clicked {
                    self.status = "Running".to_string();
                }
            });
    }
}

impl eframe::App for MainUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let visuals = egui::Visuals::light();
        ctx.set_visuals(visuals);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                let callibrate_btn =
                    egui::Button::new("Calibrate").fill(if self.selected_tab == 0 {
                        egui::Color32::from_rgb(180, 180, 180)
                    } else {
                        egui::Color32::from_rgb(240, 240, 240)
                    });
                let detect_btn = egui::Button::new("Detect").fill(if self.selected_tab == 1 {
                    egui::Color32::from_rgb(180, 180, 180)
                } else {
                    egui::Color32::from_rgb(240, 240, 240)
                });

                if ui.add(callibrate_btn).clicked() {
                    self.selected_tab = 0;
                }
                if ui.add(detect_btn).clicked() {
                    self.selected_tab = 1;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(egui::Label::new(self.status.clone()));
                });
            });
            ui.separator();
            match self.update(ctx, _frame, ui) {
                Ok(_) => {}
                Err(e) => {
                    self.status = e.to_string();
                }
            }
            if self.status.contains("error") {
                self.show_error_popup(ctx, self.status.clone());
            }
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
