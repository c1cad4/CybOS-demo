use crate::config::APP_VERSION;
use crate::CybOs;
use eframe::egui;
use egui::{Color32, RichText};
use std::process::Command;

impl CybOs {
    pub(crate) fn system(&mut self, ui: &mut egui::Ui) {
        let neon = Self::neon();
        let dim = Color32::from_rgb(55, 145, 105);
        ui.label(RichText::new("NATIVE DESKTOP RUNTIME · NO BROWSER SHELL").size(11.0).color(dim));
        ui.add_space(10.0);
        egui::Grid::new("sys").num_columns(3).spacing([10.0, 8.0]).show(ui, |ui| {
            self.card(ui, "VERSION", APP_VERSION);
            self.card(ui, "BATTERY", &format!("{:.0}%", self.battery));
            self.card(ui, "TEMP", &format!("{:.1}°C", self.temperature));
            ui.end_row();
        });
        ui.add_space(12.0);
        for (k, v) in [
            ("NODE", self.node_id.as_str()),
            ("DATABASE", self.store.path.to_str().unwrap_or("—")),
            ("RENDERER", "egui / eframe"),
            ("QWEN", self.qwen_status.as_str()),
            ("KEYS", "not stored"),
        ] {
            ui.horizontal(|ui| {
                ui.label(RichText::new(k).size(11.0).strong().color(neon).extra_letter_spacing(1.2));
                ui.label(RichText::new(v).size(12.0).color(Color32::from_rgb(180, 225, 200)));
            });
            ui.add_space(4.0);
        }
        ui.add_space(12.0);
        ui.label(RichText::new("RUNTIME CELLS").size(11.0).strong().color(neon));
        ui.add_space(6.0);
        ui.label(
            RichText::new(format!(
                "{} / {} cells READY · tick {}",
                self.runtime.healthy_count(),
                self.runtime.cells.len(),
                self.runtime.ticks
            ))
            .size(10.0)
            .color(dim),
        );
        for cell in &self.runtime.cells {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} · {} · {}ms budget · heartbeat {}ms ago",
                        cell.id,
                        cell.status,
                        cell.budget.as_millis(),
                        cell.heartbeat_age_ms()
                    ))
                    .size(9.0)
                    .color(Color32::from_rgb(165, 220, 190)),
                );
            });
        }
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button("COPY NODE ID").clicked() {
                ui.ctx().copy_text(self.node_id.clone());
                self.notify("NODE ID COPIED");
            }
            if ui.button("OPEN DATA FOLDER").clicked() {
                if let Some(parent) = self.store.path.parent() {
                    let _ = Command::new("open").arg(parent).spawn();
                    self.notify("OPENED LOCAL DATA FOLDER");
                }
            }
            if ui.button("WRITE SYSTEM EVENT").clicked() {
                self.add_event("SYSTEM", "Manual system pulse from cybOS");
                self.notify("SYSTEM EVENT WRITTEN");
            }
        });
    }
}
