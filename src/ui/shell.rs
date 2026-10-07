use crate::navigation::Page;
use crate::state::CybOs;
use crate::Icon;
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

use crate::config::APP_VERSION;
use std::time::Duration;

impl CybOs {
    pub(crate) fn shell_ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ensure_qwen();
        self.neon_theme(ui.ctx());

        let neon = Self::neon();
        let dim = Color32::from_rgb(0, 100, 65);
        let bg = Color32::from_rgb(2, 8, 6);
        let panel = Color32::from_rgb(4, 15, 10);

        if ui.input(|i| i.key_pressed(egui::Key::K) && (i.modifiers.command || i.modifiers.mac_cmd))
        {
            self.search_focus = true;
        }

        {
            let painter = ui.ctx().layer_painter(egui::LayerId::background());
            let rect = ui.max_rect();
            painter.rect_filled(rect, 0.0, bg);
            for i in 0..180u32 {
                let x = ((i.wrapping_mul(97) % 1000) as f32) / 1000.0;
                let y = ((i.wrapping_mul(193).wrapping_add(37) % 1000) as f32) / 1000.0;
                let px = rect.left() + rect.width() * x;
                let py = rect.top() + rect.height() * y;
                let pulse = (((i as f32) * 0.73).sin() * 0.5 + 0.5) as u8;
                let alpha = 35u8.saturating_add(pulse / 2);
                painter.circle_filled(
                    egui::pos2(px, py),
                    if i % 17 == 0 { 1.35 } else { 0.65 },
                    Color32::from_rgba_unmultiplied(80, 255, 170, alpha),
                );
            }
        }

        egui::Panel::top("cybos_header")
            .exact_size(64.0)
            .frame(egui::Frame::NONE.fill(bg).inner_margin(egui::Margin::symmetric(16, 10)))
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("CICADAFARM").size(13.0).strong().color(neon));
                    ui.add_space(10.0);
                    ui.label(RichText::new("C Y B O S").size(20.0).strong().color(neon));
                    ui.add_space(10.0);
                    ui.label(RichText::new("ROBOTCYB").size(13.0).strong().color(neon));
                    ui.add_space(18.0);

                    let (search_icon, _) =
                        ui.allocate_exact_size(Vec2::splat(22.0), egui::Sense::hover());
                    Self::paint_icon(ui.painter(), search_icon.center(), Icon::Search, neon, 16.0);

                    let search = ui.add_sized(
                        [280.0, 30.0],
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("⌘K  search pages, tokens, cameras…")
                            .id(egui::Id::new("cybos_search")),
                    );
                    if self.search_focus {
                        search.request_focus();
                        self.search_focus = false;
                    }
                    if (search.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        || ui
                            .add(egui::Button::new(RichText::new("GO").strong().color(neon)))
                            .clicked()
                    {
                        self.apply_search();
                    }

                    let (plus_icon, plus_resp) =
                        ui.allocate_exact_size(Vec2::splat(22.0), egui::Sense::click());
                    Self::paint_icon(ui.painter(), plus_icon.center(), Icon::Plus, neon, 16.0);
                    if plus_resp.on_hover_text("Write a local event").clicked()
                        || ui
                            .add(egui::Button::new(RichText::new("+ EVENT").strong().color(neon)))
                            .on_hover_text("Write a local event")
                            .clicked()
                    {
                        self.add_event("USER", "Manual event created from cybOS");
                        self.notify("EVENT WRITTEN");
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("• {}", self.qwen_status))
                                .size(10.0)
                                .color(neon),
                        );
                    });
                });
            });

        egui::Panel::left("cybos_left")
            .exact_size(78.0)
            .frame(egui::Frame::NONE.fill(panel))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    let items = [
                        (Page::Dashboard, "DASHBOARD"),
                        (Page::Graph, "GRAPH"),
                        (Page::Network, "NETWORK"),
                        (Page::Brain, "BRAIN"),
                        (Page::Farm, "FARM"),
                        (Page::Robot, "ROBOT"),
                        (Page::Cameras, "CAMERAS"),
                        (Page::System, "SYSTEM"),
                        (Page::Assets, "TOKENS"),
                        (Page::Chat, "CYBCHAT"),
                    ];
                    for (page, tooltip) in items {
                        if self.rail_icon(ui, page.icon(), tooltip, self.page == page) {
                            self.go(page);
                        }
                    }
                });
            });

        egui::Panel::right("cybos_right")
            .exact_size(78.0)
            .frame(egui::Frame::NONE.fill(panel))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    let actions = [
                        (Icon::Node, "LIVE NODE", Page::Dashboard),
                        (Icon::Assets, "TOKENS", Page::Assets),
                        (Icon::Energy, "ENERGY", Page::System),
                        (Icon::Environment, "ENVIRONMENT", Page::Farm),
                        (Icon::Activity, "ACTIVITY", Page::Dashboard),
                    ];
                    for (icon, tooltip, page) in actions {
                        if self.rail_icon(ui, icon, tooltip, self.page == page) {
                            self.go(page);
                            if icon == Icon::Activity {
                                self.notify("ACTIVITY LOG ON DASHBOARD");
                            }
                        }
                    }
                });
            });

        egui::Panel::bottom("cybos_bottom")
            .exact_size(40.0)
            .frame(egui::Frame::NONE.fill(bg))
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    let bus = [
                        ("LIVE NODE", Page::Dashboard),
                        ("QWEN", Page::Brain),
                        ("MEMORY", Page::Brain),
                        ("GRAPH", Page::Graph),
                        ("FARM", Page::Farm),
                        ("ENERGY", Page::System),
                        ("NETWORK", Page::Network),
                        ("CHAT", Page::Chat),
                    ];
                    for (name, page) in bus {
                        let active = self.page == page;
                        let text = RichText::new(name).size(11.0).strong().color(if active {
                            neon
                        } else {
                            dim
                        });
                        if ui.add(egui::Button::new(text).frame(false)).clicked() {
                            self.go(page);
                        }
                        ui.label(RichText::new("·").size(12.0).color(dim));
                    }
                    ui.label(
                        RichText::new(format!("v{}", APP_VERSION))
                            .size(10.0)
                            .color(dim),
                    );
                });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(2, 8, 6, 238))
                    .inner_margin(egui::Margin::symmetric(18, 14)),
            )
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::hover());
                        Self::paint_icon(ui.painter(), icon_rect.center(), self.page.icon(), neon, 18.0);
                        ui.label(
                            RichText::new(self.page.title())
                                .size(18.0)
                                .strong()
                                .color(neon),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("• {}", self.status))
                                    .size(11.0)
                                    .color(neon),
                            );
                        });
                    });
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);
                    match self.page {
                        Page::Graph => self.graph(ui),
                        Page::Dashboard => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.dashboard(ui));
                        }
                        Page::Robot => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.robot(ui));
                        }
                        Page::Farm => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.farm(ui));
                        }
                        Page::Chat => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.chat_page(ui));
                        }
                        Page::Brain => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.brain(ui));
                        }
                        Page::Network => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.network(ui));
                        }
                        Page::Assets => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.assets(ui));
                        }
                        Page::System => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.system(ui));
                        }
                        Page::Cameras => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.cameras(ui));
                        }
                    }
                });
            });

        if let Some((msg, started)) = self.toast.clone() {
            if started.elapsed() < Duration::from_secs(3) {
                egui::Area::new("cybos_toast".into())
                    .anchor(egui::Align2::RIGHT_TOP, [-28.0, 78.0])
                    .show(ui.ctx(), |ui| {
                        egui::Frame::new()
                            .fill(Color32::from_rgb(6, 28, 18))
                            .stroke(Stroke::new(1.0, neon))
                            .corner_radius(10)
                            .inner_margin(egui::Margin::symmetric(14, 10))
                            .show(ui, |ui| {
                                ui.label(RichText::new(msg).size(12.0).strong().color(neon));
                            });
                    });
            } else {
                self.toast = None;
            }
        }

        ui.ctx().request_repaint_after(Duration::from_millis(80));
    }
}
