use crate::{CybOs, Icon};
use eframe::egui;
use egui::{Color32, RichText, Stroke, Vec2};

impl CybOs {
        pub(crate) fn cameras(&mut self, ui: &mut egui::Ui) {
            let neon = Self::neon();
            let dim = Color32::from_rgb(90, 170, 120);
            let zones = ["FARM YARD", "APIARY", "CHICKENS", "GOATS", "FOREST EDGE"];
            ui.label(
                RichText::new("LOCAL PREVIEW · NO FAKE RTSP STREAM · READY FOR A REAL CAMERA LATER")
                    .size(11.0)
                    .color(dim),
            );
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                for (i, name) in zones.iter().enumerate() {
                    let active = self.camera_zone == i;
                    let btn = egui::Button::new(
                        RichText::new(*name)
                            .size(11.0)
                            .strong()
                            .color(if active { Color32::BLACK } else { neon }),
                    )
                    .fill(if active { neon } else { Color32::from_rgb(6, 22, 15) })
                    .min_size(Vec2::new(108.0, 34.0));
                    if ui.add(btn).clicked() {
                        self.camera_zone = i;
                        self.add_event("CAMERA", format!("Selected camera zone: {}", name));
                        self.notify(format!("{} SELECTED", name));
                    }
                }
            });
            ui.add_space(10.0);
            egui::Frame::new()
                .fill(Color32::from_rgb(1, 8, 6))
                .stroke(Stroke::new(1.0, Color32::from_rgb(20, 90, 55)))
                .corner_radius(16)
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    let h = ui.available_height().max(360.0);
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h.min(420.0)), egui::Sense::hover());
                    let painter = ui.painter_at(rect);
                    let t = ui.input(|i| i.time) as f32;
                    painter.rect_filled(rect, 12.0, Color32::from_rgb(2, 10, 8));
                    for i in 0..18 {
                        let y = rect.top() + ((t * 28.0 + i as f32 * 22.0) % rect.height());
                        painter.line_segment(
                            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                            Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 255, 150, 18)),
                        );
                    }
                    Self::paint_icon(&painter, rect.center() + egui::vec2(0.0, -28.0), Icon::Camera, neon, 54.0);
                    painter.text(
                        rect.center() + egui::vec2(0.0, 22.0),
                        egui::Align2::CENTER_CENTER,
                        format!("{} · SIGNAL ABSENT", zones[self.camera_zone]),
                        egui::FontId::proportional(18.0),
                        neon,
                    );
                    painter.text(
                        rect.center() + egui::vec2(0.0, 48.0),
                        egui::Align2::CENTER_CENTER,
                        "Telemetry overlay only. Connect a real camera when the farm feed exists.",
                        egui::FontId::proportional(11.0),
                        Color32::from_rgb(90, 150, 115),
                    );
                    painter.text(
                        rect.left_top() + egui::vec2(16.0, 16.0),
                        egui::Align2::LEFT_TOP,
                        format!("CAM-{:02}  {}°C  NODE {}", self.camera_zone + 1, self.temperature as i32, &self.node_id[..8.min(self.node_id.len())]),
                        egui::FontId::monospace(11.0),
                        dim,
                    );
                });
        }
}
