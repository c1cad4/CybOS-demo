use crate::state::CybOs;

impl eframe::App for CybOs {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame) {
        self.shell_ui(ui, frame);
    }
}
