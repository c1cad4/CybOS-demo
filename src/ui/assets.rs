use crate::config::{CICADAFARM_MINT, ROBOTCYB_MINT};
use crate::CybOs;
use eframe::egui;
use egui::{Color32, RichText, Stroke};

impl CybOs {
    pub(crate) fn assets(&mut self, ui: &mut egui::Ui) {
        self.token_matrix(ui);

        ui.add_space(18.0);
        ui.separator();
        ui.add_space(10.0);

        ui.heading(
            RichText::new(crate::language::tr(self.language, "public_identifiers"))
                .strong()
                .color(Self::green()),
        );

        self.asset(ui, "$CICADAFARM", CICADAFARM_MINT, crate::language::tr(self.language, "physical_world"));
        self.asset(ui, "$ROBOTCYB", ROBOTCYB_MINT, crate::language::tr(self.language, "digital_world"));
    }

    pub(crate) fn asset(&mut self, ui: &mut egui::Ui, name: &str, mint: &str, desc: &str) {
        egui::Frame::new()
            .fill(Color32::from_rgb(7, 25, 16))
            .stroke(Stroke::new(1.0, Color32::from_rgb(28, 74, 48)))
            .corner_radius(9)
            .inner_margin(egui::Margin::same(13))
            .show(ui, |ui| {
                ui.label(RichText::new(name).size(20.0).strong());
                ui.label(desc);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(mint).small().color(Color32::GRAY));
                    if ui.button(crate::language::tr(self.language, "copy")).clicked() {
                        ui.ctx().copy_text(mint.into());
                        self.notify(format!("{} MINT COPIED", name));
                    }
                });
            });
        ui.add_space(8.0);
    }
}
