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
            RichText::new("◇ PUBLIC IDENTIFIERS")
                .strong()
                .color(Self::green()),
        );

        self.asset(ui, "$CICADAFARM", CICADAFARM_MINT, "PHYSICAL WORLD");
        self.asset(ui, "$ROBOTCYB", ROBOTCYB_MINT, "DIGITAL WORLD");

        ui.add_space(18.0);
        ui.separator();
        ui.add_space(10.0);
        ui.heading(
            RichText::new("◇ SOLANA PROGRAM REFERENCE")
                .strong()
                .color(Self::green()),
        );

        egui::Frame::new()
            .fill(Color32::from_rgb(7, 25, 16))
            .stroke(Stroke::new(1.0, Color32::from_rgb(28, 74, 48)))
            .corner_radius(9)
            .inner_margin(egui::Margin::same(13))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("PUMPFUN-STYLE PROGRAM").size(15.0).strong().color(Self::green()));
                    ui.label(RichText::new(crate::assets::pumpfun::CLUSTER.to_uppercase()).size(9.0).color(Color32::GRAY));
                });
                ui.label(RichText::new(crate::assets::pumpfun::MODE).size(9.0).color(Color32::GRAY));
                ui.label(
                    RichText::new(crate::assets::pumpfun::PROGRAM_ID)
                        .size(9.0)
                        .color(Color32::LIGHT_GRAY),
                );
                ui.horizontal(|ui| {
                    if ui.button("COPY PROGRAM ID").clicked() {
                        ui.ctx().copy_text(crate::assets::pumpfun::PROGRAM_ID.into());
                        self.notify("PUMPFUN PROGRAM ID COPIED");
                    }
                    if ui.button("OPEN SOURCE").clicked() {
                        let _ = std::process::Command::new("open")
                            .arg(crate::assets::pumpfun::SOURCE_REPOSITORY)
                            .spawn();
                    }
                });
                ui.label(
                    RichText::new(format!(
                        "{} · {}",
                        crate::assets::pumpfun::LICENSE,
                        crate::assets::pumpfun::CAPABILITIES.join(" · ")
                    ))
                    .size(8.0)
                    .color(Color32::GRAY),
                );
            });
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
                    if ui.button("COPY").clicked() {
                        ui.ctx().copy_text(mint.into());
                        self.notify(format!("{} MINT COPIED", name));
                    }
                });
            });
        ui.add_space(8.0);
    }
}
