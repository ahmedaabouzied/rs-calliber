pub struct DetectTab {}

impl DetectTab {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label("Detect");
        });
    }
}
