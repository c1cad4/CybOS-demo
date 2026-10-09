use crate::{CybOs, Page};
use eframe::egui;
use egui::{Color32, RichText, Stroke};

impl CybOs {
    pub(crate) fn dashboard(&mut self, ui: &mut egui::Ui) {
        let neon = Self::green();
        let dim = Color32::from_rgb(75, 155, 115);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading(RichText::new("CYBOS CORE").strong().color(neon));
                ui.label(
                    RichText::new("CICADAFARM = BODY  ·  ROBOTCYB = MIND  ·  CYBOS = CONNECTION")
                        .size(10.0)
                        .color(dim),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let qwen_color = if self.qwen_status.contains("ONLINE") {
                    neon
                } else {
                    Color32::from_rgb(225, 170, 85)
                };
                ui.label(RichText::new(format!("● {}", self.qwen_status)).size(10.0).color(qwen_color));
            });
        });

        ui.add_space(10.0);

        egui::Frame::new()
            .fill(Color32::from_rgb(2, 10, 7))
            .stroke(Stroke::new(1.0, Color32::from_rgb(25, 100, 62)))
            .corner_radius(14)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                let desired = egui::vec2(ui.available_width().max(280.0), 350.0);
                let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
                let painter = ui.painter_at(rect);
                let center = rect.center();
                let x_span = ((rect.width() * 0.5) - 70.0).clamp(72.0, 230.0);
                let y_span = 112.0_f32.min(rect.height() * 0.34);

                let nodes = [
                    ("◈", "ROBOTCYB", Page::Robot, egui::pos2(center.x, center.y - y_span)),
                    ("Ψ", "BRAIN", Page::Brain, egui::pos2(center.x - x_span, center.y - 52.0)),
                    ("◇", "TOKENS", Page::Assets, egui::pos2(center.x + x_span, center.y - 52.0)),
                    ("∿", "GRAPH", Page::Graph, egui::pos2(center.x - x_span, center.y + 52.0)),
                    ("⟷", "NETWORK", Page::Network, egui::pos2(center.x + x_span, center.y + 52.0)),
                    ("⌬", "CICADAFARM", Page::Farm, egui::pos2(center.x, center.y + y_span)),
                ];

                // Draw links first so the node cards stay crisp and readable.
                for (_, _, _, target) in nodes {
                    painter.line_segment(
                        [center, target],
                        Stroke::new(1.0, Color32::from_rgb(25, 90, 55)),
                    );
                    painter.circle_filled(
                        egui::pos2((center.x + target.x) * 0.5, (center.y + target.y) * 0.5),
                        2.0,
                        Color32::from_rgb(50, 180, 100),
                    );
                }

                let core_rect = egui::Rect::from_center_size(center, egui::vec2(142.0, 70.0));
                painter.rect_filled(core_rect, 12.0, Color32::from_rgb(5, 25, 16));
                painter.rect_stroke(
                    core_rect,
                    12.0,
                    Stroke::new(2.0, Color32::from_rgb(50, 200, 120)),
                    egui::StrokeKind::Outside,
                );
                painter.circle_filled(center + egui::vec2(0.0, -8.0), 7.0, neon);
                painter.circle_stroke(center + egui::vec2(0.0, -8.0), 17.0, Stroke::new(1.0, dim));
                painter.text(
                    center + egui::vec2(0.0, 17.0),
                    egui::Align2::CENTER_CENTER,
                    "CYBOS CORE",
                    egui::FontId::proportional(13.0),
                    neon,
                );

                for (symbol, label, page, pos) in nodes {
                    let node_rect = egui::Rect::from_center_size(pos, egui::vec2(116.0, 54.0));
                    let response = ui.interact(node_rect, ui.id().with(label), egui::Sense::click());
                    let active = self.page == page;
                    let border = if active {
                        Color32::from_rgb(80, 220, 130)
                    } else if response.hovered() {
                        Color32::from_rgb(55, 170, 100)
                    } else {
                        Color32::from_rgb(25, 90, 55)
                    };

                    painter.rect_filled(node_rect, 9.0, Color32::from_rgb(4, 18, 12));
                    painter.rect_stroke(
                        node_rect,
                        9.0,
                        Stroke::new(if active { 2.0 } else { 1.0 }, border),
                        egui::StrokeKind::Outside,
                    );
                    painter.text(
                        pos + egui::vec2(0.0, -8.0),
                        egui::Align2::CENTER_CENTER,
                        symbol,
                        egui::FontId::proportional(20.0),
                        neon,
                    );
                    painter.text(
                        pos + egui::vec2(0.0, 13.0),
                        egui::Align2::CENTER_CENTER,
                        label,
                        egui::FontId::proportional(9.5),
                        Color32::from_rgb(165, 215, 185),
                    );

                    if response.clicked() {
                        self.page = page;
                    }
                }

                painter.text(
                    egui::pos2(rect.left() + 8.0, rect.bottom() - 7.0),
                    egui::Align2::LEFT_BOTTOM,
                    "LOCAL-FIRST · LIVE STATUS ONLY",
                    egui::FontId::proportional(9.0),
                    dim,
                );
                let node_short = self.node_id.chars().take(8).collect::<String>();
                painter.text(
                    egui::pos2(rect.right() - 8.0, rect.bottom() - 7.0),
                    egui::Align2::RIGHT_BOTTOM,
                    format!("NODE {}", node_short),
                    egui::FontId::proportional(9.0),
                    Color32::GRAY,
                );
            });

        ui.add_space(10.0);

        // Use only values that exist in local state; do not invent farm telemetry.
        egui::Grid::new("cybos_status_matrix")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                self.card(ui, "LOCAL MEMORIES", &self.store.memories().len().to_string());
                self.card(ui, "GRAPH NODES", &self.nodes.len().to_string());
                ui.end_row();
                self.card(ui, "LOCAL EVENTS", &self.events.len().to_string());
                self.card(
                    ui,
                    "LOCAL QWEN",
                    if self.qwen_status.contains("ONLINE") { "ONLINE" } else { "NOT CONNECTED" },
                );
                ui.end_row();
            });

        ui.add_space(10.0);
        self.event_list(ui, 7);
    }
}
