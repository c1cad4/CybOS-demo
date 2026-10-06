use crate::CybOs;
use eframe::egui;
use egui::{Color32, RichText, Stroke, Vec2};

impl CybOs {
        pub(crate) fn robot(&mut self, ui: &mut egui::Ui) {
            let neon = Color32::from_rgb(0, 255, 150);
            let dim = Color32::from_rgb(55, 145, 105);
            let panel = Color32::from_rgb(5, 18, 13);
    
            ui.vertical(|ui| {
                ui.label(RichText::new("◉  ROBOTCYB").size(24.0).strong().color(neon));
    
                ui.label(
                    RichText::new("LOCAL AI AGENT · REQUEST → REASONING → RESPONSE")
                        .size(11.0)
                        .color(dim),
                );
    
                ui.add_space(14.0);
    
                // ------------------------------------------------
                // ROBOT CORE / EYE
                // ------------------------------------------------
                egui::Frame::NONE
                    .fill(panel)
                    .corner_radius(egui::CornerRadius::same(14))
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let available = ui.available_width();
    
                            let eye_size = 150.0;
    
                            let (rect, _) = ui.allocate_exact_size(
                                Vec2::new(eye_size, eye_size),
                                egui::Sense::hover(),
                            );
    
                            let painter = ui.painter_at(rect);
                            let c = rect.center();
    
                            painter.circle_stroke(c, 62.0, Stroke::new(2.0, neon));
    
                            painter.circle_stroke(c, 48.0, Stroke::new(1.0, dim));
    
                            painter.circle_filled(
                                c,
                                27.0,
                                Color32::from_rgba_unmultiplied(0, 255, 150, 28),
                            );
    
                            painter.circle_stroke(c, 27.0, Stroke::new(2.0, neon));
    
                            painter.circle_filled(c, 10.0, neon);
    
                            painter.circle_filled(c + Vec2::new(-4.0, -5.0), 3.0, Color32::WHITE);
    
                            // eye rays
                            for k in 0..8 {
                                let a = k as f32 * std::f32::consts::TAU / 8.0;
                                let a0 = c + Vec2::angled(a) * 69.0;
                                let a1 = c + Vec2::angled(a) * 78.0;
    
                                painter.line_segment([a0, a1], Stroke::new(1.0, dim));
                            }
    
                            ui.add_space(18.0);
    
                            ui.vertical(|ui| {
                                ui.label(RichText::new("ROBOTCYB").size(22.0).strong().color(neon));
    
                                ui.label(RichText::new("LOCAL AGENT").size(11.0).strong().color(dim));
    
                                ui.add_space(8.0);
    
                                ui.label(RichText::new("• READY").size(11.0).color(neon));
    
                                ui.label(
                                    RichText::new(format!("QWEN · {}", self.qwen_status))
                                        .size(10.0)
                                        .color(dim),
                                );
    
                                ui.add_space(10.0);
    
                                ui.label(RichText::new("REQUEST").size(10.0).strong().color(dim));
    
                                ui.label(
                                    RichText::new("Ask the local RobotCYB agent anything.")
                                        .size(11.0)
                                        .color(Color32::from_rgb(125, 180, 150)),
                                );
    
                                let _ = available;
                            });
                        });
                    });
    
                ui.add_space(12.0);
    
                // ------------------------------------------------
                // REQUEST
                // ------------------------------------------------
                egui::Frame::NONE
                    .fill(panel)
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new("REQUEST").size(11.0).strong().color(neon));
    
                        ui.add_space(5.0);
    
                        let response = ui.add(
                            egui::TextEdit::multiline(&mut self.robot_input)
                                .desired_rows(4)
                                .desired_width(f32::INFINITY)
                                .hint_text("Enter a request for RobotCYB..."),
                        );
    
                        ui.add_space(7.0);
    
                        ui.horizontal(|ui| {
                            let send = ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("◉  SEND REQUEST")
                                            .size(12.0)
                                            .strong()
                                            .color(neon),
                                    )
                                    .min_size(Vec2::new(170.0, 34.0)),
                                )
                                .clicked();
    
                            if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                                || send
                            {
                                let q = self.robot_input.trim().to_string();
    
                                if !q.is_empty() {
                                    let answer = self.agent_answer(&q);
    
                                    self.robot_output = answer.clone();
    
                                    self.chat.push(("YOU".into(), q.clone(), true));
    
                                    self.chat.push(("ROBOTCYB".into(), answer, false));
    
                                    self.robot_input.clear();
    
                                    self.add_event("ROBOT", &format!("RobotCYB processed: {}", q));
                                }
                            }
    
                            ui.label(RichText::new("ENTER · SEND").size(9.0).color(dim));
                        });
                    });
    
                ui.add_space(12.0);
    
                // ------------------------------------------------
                // RESPONSE
                // ------------------------------------------------
                egui::Frame::NONE
                    .fill(panel)
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("RESPONSE").size(11.0).strong().color(neon));
    
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new("LOCAL OUTPUT").size(9.0).color(dim));
                            });
                        });
    
                        ui.add_space(6.0);
    
                        egui::Frame::NONE
                            .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 90))
                            .corner_radius(egui::CornerRadius::same(8))
                            .inner_margin(12.0)
                            .show(ui, |ui| {
                                ui.set_min_height(145.0);
    
                                ui.label(
                                    RichText::new(&self.robot_output)
                                        .size(12.0)
                                        .color(Color32::from_rgb(175, 235, 205)),
                                );
                            });
                    });
            });
        }
}
