use crate::navigation::Page;
use crate::state::CybOs;
use crate::Icon;
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

use crate::config::APP_VERSION;
use std::time::Duration;

impl CybOs {
    pub(crate) fn shell_ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.runtime.tick();
        self.ensure_qwen();
        self.poll_lan_send();
        self.poll_lan_events();
        self.poll_cyblex();
        self.poll_cybdex();
        self.poll_browser();
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
                            .hint_text(crate::language::tr(self.language, "search_hint"))
                            .id(egui::Id::new("cybos_search")),
                    );
                    if self.search_focus {
                        search.request_focus();
                        self.search_focus = false;
                    }
                    if (search.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        || ui
                            .add(egui::Button::new(RichText::new(crate::language::tr(self.language, "go")).strong().color(neon)))
                            .on_hover_text("Open the best matching page for the search text. Try a page name or keyword.")
                            .clicked()
                    {
                        self.apply_search();
                    }

                    let (plus_icon, plus_resp) =
                        ui.allocate_exact_size(Vec2::splat(22.0), egui::Sense::click());
                    Self::paint_icon(ui.painter(), plus_icon.center(), Icon::Plus, neon, 16.0);
                    if plus_resp.on_hover_text("Write a local event").clicked()
                        || ui
                            .add(egui::Button::new(RichText::new(crate::language::tr(self.language, "event")).strong().color(neon)))
                            .on_hover_text("Write a local event")
                            .clicked()
                    {
                        self.add_event("USER", "Manual event created from cybOS");
                        self.notify("EVENT WRITTEN");
                    }

                    let mut selected_language = self.language;
                    egui::ComboBox::from_id_salt("cybos_language")
                        .selected_text(selected_language.label())
                        .width(105.0)
                        .show_ui(ui, |ui| {
                            ui.label(RichText::new(crate::language::tr(selected_language, "language")).size(9.0));
                            for language in crate::language::Language::ALL {
                                ui.selectable_value(&mut selected_language, language, language.label());
                            }
                        });
                    if selected_language != self.language {
                        self.language = selected_language;
                        self.store.set("language", self.language.code());
                        self.notify(format!("LANGUAGE SET: {}", self.language.label()));
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
                        (Page::Dashboard, crate::language::page_title(self.language, Page::Dashboard)),
                        (Page::Graph, crate::language::page_title(self.language, Page::Graph)),
                        (Page::Network, crate::language::page_title(self.language, Page::Network)),
                        (Page::Radar, crate::language::page_title(self.language, Page::Radar)),
                        (Page::Brain, crate::language::page_title(self.language, Page::Brain)),
                        (Page::Farm, crate::language::page_title(self.language, Page::Farm)),
                        (Page::Robot, crate::language::page_title(self.language, Page::Robot)),
                        (Page::Cameras, crate::language::page_title(self.language, Page::Cameras)),
                        (Page::System, crate::language::page_title(self.language, Page::System)),
                        (Page::Assets, crate::language::page_title(self.language, Page::Assets)),
                        (Page::Chat, crate::language::page_title(self.language, Page::Chat)),
                        (Page::CybLex, crate::language::page_title(self.language, Page::CybLex)),
                        (Page::CybDex, crate::language::page_title(self.language, Page::CybDex)),
                        (Page::Browser, crate::language::page_title(self.language, Page::Browser)),
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
                        if ui.add(egui::Button::new(text).frame(false)).on_hover_text(page.guidance().0).clicked() {
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
                            RichText::new(crate::language::page_title(self.language, self.page))
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
                    ui.add_space(6.0);

                    // Consistent, beginner-friendly orientation on every screen.
                    let (purpose, first_step, limitation) = crate::language::page_guidance(self.language, self.page);
                    egui::Frame::new()
                        .fill(Color32::from_rgb(5, 20, 13))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(20, 75, 48)))
                        .corner_radius(8.0)
                        .inner_margin(egui::Margin::symmetric(10, 7))
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(crate::language::tr(self.language, "quick_guide")).size(9.0).strong().color(neon));
                                ui.label(RichText::new(purpose).size(10.0).color(Color32::from_rgb(175, 215, 190)));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.collapsing(RichText::new(crate::language::tr(self.language, "how_to_use")).size(9.0).strong().color(neon), |ui| {
                                        ui.label(RichText::new(crate::language::tr(self.language, "start_here")).size(9.0).strong().color(neon));
                                        ui.label(RichText::new(first_step).size(10.0).color(Color32::from_rgb(195, 225, 205)));
                                        ui.add_space(4.0);
                                        ui.label(RichText::new(crate::language::tr(self.language, "good_to_know")).size(9.0).strong().color(neon));
                                        ui.label(RichText::new(limitation).size(10.0).color(Color32::from_rgb(195, 225, 205)));
                                        ui.add_space(4.0);
                                        ui.label(RichText::new(crate::language::tr(self.language, "tip")).size(9.0).color(Color32::GRAY));
                                    });
                                });
                            });
                        });
                    ui.add_space(6.0);
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
                        Page::Network | Page::Radar => {
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
                        Page::CybLex => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.cyblex(ui));
                        }
                        Page::CybDex => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.cybdex_page(ui));
                        }
                        Page::Browser => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.browser(ui));
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
