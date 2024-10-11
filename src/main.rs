// GUI
use eframe::egui;

mod audio;
mod calibrate;
mod chirp;
mod detect;
mod freq;
mod task;
mod wave;

struct MainUI {
    selected_tab: u8,
    detect_tab: detect::DetectTab,
    calibrate_tab: calibrate::CalibrateTab,
}

impl MainUI {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            selected_tab: 0, // Default on the calibration page.
            detect_tab: detect::DetectTab::new(),
            calibrate_tab: calibrate::CalibrateTab::new(_cc),
        }
    }
}

impl eframe::App for MainUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::scroll_area::ScrollArea::vertical().show(ui, |ui| {
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
                });
                ui.separator();
                match self.selected_tab {
                    0 => self.calibrate_tab.render(ui, ctx, _frame),
                    1 => self.detect_tab.render(ui, ctx, _frame),
                    _ => {}
                }
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
