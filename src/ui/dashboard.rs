use crate::{CybOs, Page};
use eframe::egui;
use egui::{Color32, RichText, Stroke};

impl CybOs {
    pub(crate) fn dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("◎ CYBOS CORE").strong().color(Self::green()));

        ui.label(
            RichText::new("LOCAL-FIRST · CONNECTED SYSTEM")
                .small()
                .color(Color32::GRAY),
        );

        ui.add_space(10.0);

        egui::Frame::new()
            .fill(Color32::from_rgb(2, 10, 7))
            .stroke(Stroke::new(1.0, Color32::from_rgb(25, 100, 62)))
            .corner_radius(14)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                let desired = egui::vec2(ui.available_width(), 480.0);

                let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());

                let painter = ui.painter_at(rect);

                let center = rect.center();

                let nodes = [
                    ("◈", "ROBOTCYB", Page::Robot, egui::vec2(0.0, -165.0)),
                    ("Ψ", "BRAIN", Page::Brain, egui::vec2(-245.0, -65.0)),
                    ("◇", "TOKENS", Page::Assets, egui::vec2(245.0, -65.0)),
                    ("∿", "GRAPH", Page::Graph, egui::vec2(-245.0, 85.0)),
                    ("⟷", "NETWORK", Page::Network, egui::vec2(245.0, 85.0)),
                    ("⌬", "CICADAFARM", Page::Farm, egui::vec2(0.0, 170.0)),
                    // EXTRA MATHEMATICAL SYMBOLS
                    ("∆", "DELTA", Page::Dashboard, egui::vec2(-95.0, -170.0)),
                    ("√", "CORE", Page::Brain, egui::vec2(95.0, -170.0)),
                    ("∞", "MEMORY", Page::Brain, egui::vec2(-95.0, 170.0)),
                    ("⊕", "ENERGY", Page::System, egui::vec2(95.0, 170.0)),
                ];

                // -------------------------------------------------
                // CONNECTION BUS
                // -------------------------------------------------

                for (_, _, _, offset) in nodes {
                    let target = center + offset;

                    painter.line_segment(
                        [center, target],
                        Stroke::new(1.0, Color32::from_rgb(25, 90, 55)),
                    );

                    painter.circle_filled(
                        egui::pos2((center.x + target.x) * 0.5, (center.y + target.y) * 0.5),
                        2.5,
                        Color32::from_rgb(50, 180, 100),
                    );
                }

                // -------------------------------------------------
                // CENTRAL CORE
                // -------------------------------------------------

                let core_rect = egui::Rect::from_center_size(center, egui::vec2(170.0, 105.0));

                painter.rect_filled(core_rect, 14.0, Color32::from_rgb(5, 25, 16));

                painter.rect_stroke(
                    core_rect,
                    14.0,
                    Stroke::new(2.0, Color32::from_rgb(50, 180, 100)),
                    egui::StrokeKind::Outside,
                );

                painter.circle_filled(center, 13.0, Color32::from_rgb(20, 120, 70));

                painter.circle_stroke(
                    center,
                    25.0,
                    Stroke::new(1.0, Color32::from_rgb(50, 180, 100)),
                );

                painter.text(
                    egui::pos2(center.x, center.y + 33.0),
                    egui::Align2::CENTER_CENTER,
                    "CYBOS CORE",
                    egui::FontId::proportional(16.0),
                    Self::green(),
                );

                // -------------------------------------------------
                // CLICKABLE NODES
                // -------------------------------------------------

                for (symbol, label, page, offset) in nodes {
                    let pos = center + offset;

                    let node_rect = egui::Rect::from_center_size(pos, egui::vec2(150.0, 68.0));

                    let response =
                        ui.interact(node_rect, ui.id().with(label), egui::Sense::click());

                    let active = self.page == page;

                    let border = if active {
                        Color32::from_rgb(80, 220, 130)
                    } else if response.hovered() {
                        Color32::from_rgb(55, 170, 100)
                    } else {
                        Color32::from_rgb(25, 90, 55)
                    };

                    painter.rect_filled(node_rect, 10.0, Color32::from_rgb(4, 18, 12));

                    painter.rect_stroke(
                        node_rect,
                        10.0,
                        Stroke::new(if active { 2.0 } else { 1.0 }, border),
                        egui::StrokeKind::Outside,
                    );

                    painter.text(
                        egui::pos2(pos.x, pos.y - 9.0),
                        egui::Align2::CENTER_CENTER,
                        symbol,
                        egui::FontId::proportional(25.0),
                        Self::green(),
                    );

                    painter.text(
                        egui::pos2(pos.x, pos.y + 18.0),
                        egui::Align2::CENTER_CENTER,
                        label,
                        egui::FontId::proportional(11.0),
                        Color32::from_rgb(150, 200, 170),
                    );

                    if response.clicked() {
                        self.page = page;
                    }
                }

                // -------------------------------------------------
                // STATUS
                // -------------------------------------------------

                painter.text(
                    egui::pos2(rect.left() + 16.0, rect.bottom() - 18.0),
                    egui::Align2::LEFT_CENTER,
                    "• LOCAL NODE ONLINE",
                    egui::FontId::proportional(10.0),
                    Color32::from_rgb(70, 180, 110),
                );

                painter.text(
                    egui::pos2(rect.right() - 16.0, rect.bottom() - 18.0),
                    egui::Align2::RIGHT_CENTER,
                    format!("NODE {}", &self.node_id[..8.min(self.node_id.len())]),
                    egui::FontId::proportional(10.0),
                    Color32::GRAY,
                );
            });

        ui.add_space(12.0);

        egui::Grid::new("cybos_status_matrix")
            .num_columns(4)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                self.card(ui, "TEMPERATURE", "24°C");
                self.card(ui, "HIVES", "4");
                self.card(ui, "ANIMALS", "60+");
                self.card(ui, "NODE", "ONLINE");
                ui.end_row();
            });

        ui.add_space(12.0);

        self.event_list(ui, 7);
    }
}
