use crate::app::{CybOs, Icon};
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

impl CybOs {

    pub(crate) fn green() -> Color32 {
        Color32::from_rgb(125, 255, 189)
    }

    pub(crate) fn neon() -> Color32 {
        Color32::from_rgb(0, 255, 150)
    }

    pub(crate) fn paint_icon(painter: &egui::Painter, c: egui::Pos2, icon: Icon, color: Color32, s: f32) {
        let st = Stroke::new((s * 0.09).clamp(1.1, 2.4), color);
        match icon {
            Icon::Dashboard => {
                painter.line_segment(
                    [c + egui::vec2(-s * 0.42, -s * 0.28), c + egui::vec2(s * 0.42, -s * 0.28)],
                    st,
                );
                painter.line_segment(
                    [c + egui::vec2(-s * 0.42, s * 0.28), c + egui::vec2(s * 0.42, s * 0.28)],
                    st,
                );
                painter.line_segment(
                    [c + egui::vec2(-s * 0.18, -s * 0.28), c + egui::vec2(s * 0.18, s * 0.28)],
                    st,
                );
            }
            Icon::Graph => {
                let mut pts = Vec::new();
                for i in 0..=16 {
                    let t = i as f32 / 16.0;
                    let x = c.x - s * 0.46 + t * s * 0.92;
                    let y = c.y + (t * std::f32::consts::TAU).sin() * s * 0.28;
                    pts.push(egui::pos2(x, y));
                }
                painter.add(egui::Shape::line(pts, st));
            }
            Icon::Network => {
                painter.line_segment(
                    [c + egui::vec2(-s * 0.38, -s * 0.18), c + egui::vec2(-s * 0.08, 0.0)],
                    st,
                );
                painter.line_segment(
                    [c + egui::vec2(-s * 0.38, s * 0.18), c + egui::vec2(-s * 0.08, 0.0)],
                    st,
                );
                painter.line_segment(
                    [c + egui::vec2(s * 0.38, -s * 0.18), c + egui::vec2(s * 0.08, 0.0)],
                    st,
                );
                painter.line_segment(
                    [c + egui::vec2(s * 0.38, s * 0.18), c + egui::vec2(s * 0.08, 0.0)],
                    st,
                );
                painter.line_segment(
                    [c + egui::vec2(-s * 0.08, 0.0), c + egui::vec2(s * 0.08, 0.0)],
                    st,
                );
            }
            Icon::Brain => {
                let hex = (0..6)
                    .map(|i| {
                        let a = i as f32 * std::f32::consts::TAU / 6.0 - std::f32::consts::FRAC_PI_2;
                        c + egui::vec2(a.cos(), a.sin()) * s * 0.42
                    })
                    .collect::<Vec<_>>();
                painter.add(egui::Shape::closed_line(hex, st));
                painter.circle_filled(c, s * 0.09, color);
            }
            Icon::Farm => {
                let hex = (0..6)
                    .map(|i| {
                        let a = i as f32 * std::f32::consts::TAU / 6.0;
                        c + egui::vec2(a.cos(), a.sin()) * s * 0.38
                    })
                    .collect::<Vec<_>>();
                painter.add(egui::Shape::closed_line(hex, st));
                painter.circle_stroke(c, s * 0.14, st);
            }
            Icon::Robot => {
                let d = s * 0.38;
                painter.add(egui::Shape::closed_line(
                    vec![
                        c + egui::vec2(0.0, -d),
                        c + egui::vec2(d, 0.0),
                        c + egui::vec2(0.0, d),
                        c + egui::vec2(-d, 0.0),
                    ],
                    st,
                ));
                painter.circle_filled(c, s * 0.1, color);
            }
            Icon::Camera => {
                let r = egui::Rect::from_center_size(c + egui::vec2(0.0, 2.0), egui::vec2(s * 0.78, s * 0.5));
                painter.rect_stroke(r, 3.0, st, egui::StrokeKind::Middle);
                painter.circle_stroke(c + egui::vec2(0.0, 2.0), s * 0.14, st);
                painter.rect_stroke(
                    egui::Rect::from_center_size(c + egui::vec2(-s * 0.16, -s * 0.28), egui::vec2(s * 0.22, s * 0.12)),
                    2.0,
                    st,
                    egui::StrokeKind::Middle,
                );
            }
            Icon::System => {
                for i in 0..6 {
                    let a = i as f32 * std::f32::consts::TAU / 6.0;
                    painter.line_segment(
                        [c + egui::vec2(a.cos(), a.sin()) * s * 0.22, c + egui::vec2(a.cos(), a.sin()) * s * 0.42],
                        st,
                    );
                }
                painter.circle_stroke(c, s * 0.18, st);
            }
            Icon::Assets => {
                let d = s * 0.36;
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        c + egui::vec2(0.0, -d),
                        c + egui::vec2(d, 0.0),
                        c + egui::vec2(0.0, d),
                        c + egui::vec2(-d, 0.0),
                    ],
                    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 40),
                    st,
                ));
            }
            Icon::Chat => {
                painter.circle_stroke(c + egui::vec2(-s * 0.12, -s * 0.04), s * 0.22, st);
                painter.circle_stroke(c + egui::vec2(s * 0.16, s * 0.08), s * 0.16, st);
                for k in 0..3 {
                    painter.circle_filled(
                        c + egui::vec2(-s * 0.22 + k as f32 * s * 0.1, -s * 0.04),
                        s * 0.035,
                        color,
                    );
                }
            }
            Icon::Energy => {
                painter.add(egui::Shape::line(
                    vec![
                        c + egui::vec2(s * 0.08, -s * 0.42),
                        c + egui::vec2(-s * 0.12, 0.02),
                        c + egui::vec2(s * 0.1, 0.02),
                        c + egui::vec2(-s * 0.08, s * 0.42),
                    ],
                    st,
                ));
            }
            Icon::Environment => {
                painter.circle_stroke(c, s * 0.18, st);
                for i in 0..8 {
                    let a = i as f32 * std::f32::consts::TAU / 8.0;
                    painter.line_segment(
                        [c + egui::vec2(a.cos(), a.sin()) * s * 0.26, c + egui::vec2(a.cos(), a.sin()) * s * 0.4],
                        st,
                    );
                }
            }
            Icon::Activity => {
                let pts = [
                    c + egui::vec2(-s * 0.4, s * 0.1),
                    c + egui::vec2(-s * 0.22, s * 0.1),
                    c + egui::vec2(-s * 0.1, -s * 0.28),
                    c + egui::vec2(0.08, s * 0.32),
                    c + egui::vec2(0.22, -s * 0.08),
                    c + egui::vec2(s * 0.4, -s * 0.08),
                ];
                painter.add(egui::Shape::line(pts.to_vec(), st));
            }
            Icon::Node => {
                painter.circle_stroke(c, s * 0.32, st);
                painter.circle_stroke(c, s * 0.18, st);
                painter.circle_filled(c, s * 0.07, color);
            }
            Icon::Search => {
                painter.circle_stroke(c + egui::vec2(-s * 0.08, -s * 0.08), s * 0.22, st);
                painter.line_segment(
                    [c + egui::vec2(s * 0.08, s * 0.08), c + egui::vec2(s * 0.32, s * 0.32)],
                    st,
                );
            }
            Icon::Plus => {
                painter.line_segment([c + egui::vec2(-s * 0.28, 0.0), c + egui::vec2(s * 0.28, 0.0)], st);
                painter.line_segment([c + egui::vec2(0.0, -s * 0.28), c + egui::vec2(0.0, s * 0.28)], st);
            }
        }
    }

    pub(crate) fn rail_icon(&mut self, ui: &mut egui::Ui, icon: Icon, tooltip: &str, active: bool) -> bool {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(64.0, 54.0), egui::Sense::click());
        let painter = ui.painter();
        let pulse = ((ui.input(|i| i.time) * 2.4).sin() * 0.5 + 0.5) as f32;
        let c = rect.center();
        let color = if active {
            Self::neon()
        } else if response.hovered() {
            Color32::from_rgb(90, 230, 160)
        } else {
            Color32::from_rgb(42, 120, 82)
        };
        if active {
            painter.rect_filled(
                egui::Rect::from_min_max(rect.left_top(), rect.left_bottom() + egui::vec2(3.0, 0.0)),
                0.0,
                Self::neon(),
            );
            painter.circle_stroke(
                c,
                20.0 + pulse * 1.6,
                Stroke::new(1.1, Color32::from_rgba_unmultiplied(0, 255, 150, 90)),
            );
        } else if response.hovered() {
            painter.circle_stroke(c, 19.0, Stroke::new(1.0, color));
        }
        Self::paint_icon(painter, c, icon, color, if active { 22.0 } else { 20.0 });
        response.clone().on_hover_text(tooltip);
        response.clicked()
    }

    pub(crate) fn neon_theme(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();

        visuals.override_text_color = Some(Color32::from_rgb(210, 255, 228));

        visuals.extreme_bg_color = Color32::from_rgb(1, 6, 5);

        visuals.faint_bg_color = Color32::from_rgb(4, 17, 12);

        visuals.code_bg_color = Color32::from_rgb(2, 11, 8);

        visuals.window_fill = Color32::from_rgb(3, 12, 9);

        visuals.panel_fill = Color32::from_rgb(2, 9, 7);

        visuals.hyperlink_color = Color32::from_rgb(80, 255, 170);

        ctx.set_visuals(visuals);
    }

    pub(crate) fn card(&self, ui: &mut egui::Ui, title: &str, value: &str) {
        egui::Frame::new()
            .fill(Color32::from_rgb(3, 16, 11))
            .stroke(Stroke::new(1.0, Color32::from_rgb(24, 96, 62)))
            .corner_radius(10)
            .inner_margin(egui::Margin::same(13))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                let pulse = ((ui.input(|i| i.time) * 1.7).sin() * 0.5 + 0.5) as f32;

                ui.painter().circle_stroke(
                    rect.right_top() + Vec2::new(-9.0, 9.0),
                    3.0 + pulse,
                    Stroke::new(1.0, Color32::from_rgb(55, 190, 120)),
                );

                ui.set_min_width(145.0);

                ui.label(
                    RichText::new(value)
                        .size(26.0)
                        .strong()
                        .color(Color32::from_rgb(190, 255, 220)),
                );

                ui.label(
                    RichText::new(title)
                        .size(9.0)
                        .strong()
                        .color(Color32::from_rgb(90, 175, 125)),
                );
            });
    }

    pub(crate) fn event_list(&self, ui: &mut egui::Ui, n: usize) {
        ui.heading("EVENTS");
        egui::ScrollArea::vertical()
            .max_height(190.0)
            .show(ui, |ui| {
                for e in self.events.iter().take(n) {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&e.time).small().color(Color32::GRAY));
                        ui.label(RichText::new(&e.kind).small().color(Self::green()));
                        ui.label(&e.text);
                    });
                }
            });
    }
}
