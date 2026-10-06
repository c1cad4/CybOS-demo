use crate::CybOs;
use crate::models::Memory;
use chrono::Local;
use eframe::egui;
use egui::{Color32, RichText, Stroke, Vec2};
use uuid::Uuid;

impl CybOs {
    pub(crate) fn brain(&mut self, ui: &mut egui::Ui) {
        let neon = Self::neon();
        let dim = Color32::from_rgb(55, 145, 105);
        let memories = self.store.memories();
        ui.label(
            RichText::new("MEMORY · GRAPH · LOCAL AI · NO FAKE CLOUD BRAIN")
                .size(11.0)
                .color(dim),
        );
        ui.add_space(10.0);
        egui::Grid::new("brain").num_columns(4).spacing([10.0, 8.0]).show(ui, |ui| {
            self.card(ui, "MEMORY", &format!("{}", memories.len()));
            self.card(ui, "GRAPH", &format!("{}N", self.nodes.len()));
            self.card(ui, "QWEN", if self.qwen_status.contains("ONLINE") { "ONLINE" } else { "STANDBY" });
            self.card(ui, "EVENTS", &format!("{}", self.events.len()));
            ui.end_row();
        });
        ui.add_space(12.0);
        egui::Frame::new()
            .fill(Color32::from_rgb(5, 18, 13))
            .stroke(Stroke::new(1.0, Color32::from_rgb(28, 80, 52)))
            .corner_radius(12)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.label(RichText::new("WRITE TO LOCAL MEMORY").size(11.0).strong().color(neon));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.remember_note)
                            .hint_text("A fact cybOS should keep…")
                            .desired_width(ui.available_width() - 130.0),
                    );
                    if ui
                        .add(egui::Button::new(RichText::new("STORE").strong().color(neon)).min_size(Vec2::new(110.0, 30.0)))
                        .clicked()
                    {
                        let note = self.remember_note.trim().to_string();
                        if !note.is_empty() {
                            self.store.add_memory(&Memory {
                                id: Uuid::new_v4().to_string(),
                                time: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                                text: note.clone(),
                                source: "BRAIN".into(),
                                importance: 0.7,
                            });
                            self.remember_note.clear();
                            self.add_event("BRAIN", format!("Stored memory: {}", note));
                            self.notify("MEMORY STORED");
                        }
                    }
                });
            });
        ui.add_space(10.0);
        ui.label(RichText::new("RECENT MEMORY").size(11.0).strong().color(neon));
        egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
            if memories.is_empty() {
                ui.label(RichText::new("No stored memories yet. Write one above.").color(dim));
            }
            for m in memories.iter().take(40) {
                egui::Frame::new()
                    .fill(Color32::from_rgb(3, 14, 10))
                    .inner_margin(egui::Margin::same(8))
                    .corner_radius(8)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&m.time).small().color(Color32::GRAY));
                            ui.label(RichText::new(&m.source).small().color(neon));
                        });
                        ui.label(RichText::new(&m.text).size(12.0).color(Color32::from_rgb(190, 230, 210)));
                    });
                ui.add_space(4.0);
            }
        });
    }
}
