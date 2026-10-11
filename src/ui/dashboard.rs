use crate::{CybOs, Page};
use eframe::egui;
use egui::RichText;

impl CybOs {
    pub(crate) fn dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("cybOS");
        ui.label("Your local workspace: messages, assistant and device connections.");
        ui.add_space(16.0);
        ui.heading("Start here");
        ui.horizontal_wrapped(|ui| {
            for (label, page) in [
                ("Open chat", Page::Chat), ("Devices & network", Page::Network),
                ("Local assistant", Page::Robot), ("Memory & knowledge", Page::Brain),
                ("System diagnostics", Page::System),
            ] {
                if ui.add_sized([180.0, 42.0], egui::Button::new(label)).clicked() {
                    self.page = page;
                }
            }
        });
        ui.add_space(20.0);
        ui.heading("Current status");
        for (name, value) in [
                ("Node", self.node_id.clone()),
                ("Assistant", self.qwen_status.clone()),
                ("Database integrity", self.database_integrity.clone()),
                ("Secure chat", self.secure_status.clone()),
                ("Devices", format!("{} saved · {} discovered", self.manual_peers.len(), self.lan_peers.len())),
            ] {
                ui.horizontal_top(|ui| {
                    ui.add_sized([170.0, 22.0], egui::Label::new(RichText::new(name).strong()));
                    ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.add(egui::Label::new(value).wrap());
                    });
                });
                ui.add_space(6.0);
            }
        ui.add_space(20.0);
        ui.heading("Project modules");
        ui.label("Farm and token screens include demonstration data. They are not live telemetry.");
        ui.horizontal_wrapped(|ui| {
            for (label, page) in [("Farm demo", Page::Farm), ("Tokens", Page::Assets), ("Knowledge graph", Page::Graph)] {
                if ui.button(label).clicked() { self.page = page; }
            }
        });
        ui.add_space(20.0);
        ui.heading("Recent activity");
        self.event_list(ui, 7);
    }
}
