use crate::CybOs;
use eframe::egui;
use egui::{Color32, RichText, Stroke, Vec2};
use std::collections::HashMap;

impl CybOs {
        pub(crate) fn graph(&mut self, ui: &mut egui::Ui) {
            ui.heading("CYBERGRAPH");
            ui.label("Local-first knowledge topology · nodes · links · events · queries");
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("⟳").size(20.0).color(Self::green()))
                            .frame(false),
                    )
                    .clicked()
                {
                    self.graph_zoom = 1.0;
                    self.graph_pan = Vec2::ZERO;
                }
    
                ui.add(egui::Slider::new(&mut self.graph_zoom, 0.4..=2.5).text("zoom"));
    
                let pulse = ((ui.input(|i| i.time) * 2.5).sin() * 0.5 + 0.5) as f32;
    
                ui.separator();
    
                ui.label(
                    RichText::new("• LIVE GRAPH")
                        .strong()
                        .color(Color32::from_rgb(
                            70 + (pulse * 80.0) as u8,
                            220,
                            120 + (pulse * 60.0) as u8,
                        )),
                );
    
                ui.separator();
    
                ui.label(RichText::new(format!("NODES {}", self.nodes.len())).strong());
    
                ui.label(RichText::new(format!("LINKS {}", self.links.len())).strong());
    
                ui.label(RichText::new(format!(
                    "DENSITY {:.2}",
                    if self.nodes.len() > 1 {
                        self.links.len() as f32 / self.nodes.len() as f32
                    } else {
                        0.0
                    }
                )));
            });
            let desired = Vec2::new(ui.available_width(), ui.available_height().max(460.0));
            let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());
            if resp.dragged() {
                self.graph_pan += resp.drag_delta();
            }
            let painter = ui.painter_at(rect);
            let center = rect.center() + self.graph_pan;
            let pos: HashMap<String, egui::Pos2> = self
                .nodes
                .iter()
                .map(|n| (n.id.clone(), center + Vec2::new(n.x, n.y) * self.graph_zoom))
                .collect();
            for l in &self.links {
                if let (Some(a), Some(b)) = (pos.get(&l.from), pos.get(&l.to)) {
                    painter.line_segment([*a, *b], Stroke::new(1.2, Color32::from_rgb(45, 110, 75)));
                }
            }
            for n in &self.nodes {
                if let Some(p) = pos.get(&n.id) {
                    let selected = self.selected_node.as_deref() == Some(&n.id);
    
                    let degree = self
                        .links
                        .iter()
                        .filter(|l| l.from == n.id || l.to == n.id)
                        .count();
    
                    let growth_radius = (degree as f32 * 2.0).min(10.0);
    
                    let radius = if selected {
                        26.0 + growth_radius
                    } else {
                        21.0 + growth_radius
                    };
    
                    let pulse = ((ui.input(|i| i.time) * 2.5).sin() * 0.5 + 0.5) as f32;
    
                    if degree > 0 {
                        painter.circle_stroke(
                            *p,
                            radius + 4.0 + pulse * 3.0,
                            Stroke::new(1.0 + pulse, Color32::from_rgb(35, 130, 75)),
                        );
                    }
    
                    painter.circle_filled(
                        *p,
                        radius,
                        if selected {
                            Self::green()
                        } else {
                            Color32::from_rgb(12, 50, 30)
                        },
                    );
                    painter.text(
                        *p,
                        egui::Align2::CENTER_CENTER,
                        &n.label,
                        egui::FontId::proportional(10.0),
                        if selected {
                            Color32::BLACK
                        } else {
                            Color32::WHITE
                        },
                    );
                }
            }
            if resp.clicked() {
                if let Some(pointer) = resp.interact_pointer_pos() {
                    self.selected_node = self
                        .nodes
                        .iter()
                        .find(|n| {
                            center.distance(pointer + Vec2::ZERO) < f32::MAX
                                && (center + Vec2::new(n.x, n.y) * self.graph_zoom).distance(pointer)
                                    < 32.0
                        })
                        .map(|n| n.id.clone());
                }
            }
            if let Some(id) = &self.selected_node {
                if let Some(n) = self.nodes.iter().find(|n| &n.id == id) {
                    egui::Area::new("inspector".into())
                        .fixed_pos(rect.right_top() + Vec2::new(-250.0, 10.0))
                        .show(ui.ctx(), |ui| {
                            egui::Frame::popup(ui.style()).show(ui, |ui| {
                                ui.label(RichText::new(&n.label).strong());
                                ui.label(format!("type: {}", n.kind));
                                ui.label(format!("id: {}", n.id));
                                ui.separator();
                                ui.label("Relationships");
                                for l in &self.links {
                                    if l.from == n.id || l.to == n.id {
                                        ui.label(format!(
                                            "{} → {}",
                                            l.relation,
                                            if l.from == n.id { &l.to } else { &l.from }
                                        ));
                                    }
                                }
                            });
                        });
                }
            }
        }
}
