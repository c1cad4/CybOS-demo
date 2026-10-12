use crate::navigation::Page;
use crate::state::CybOs;
use crate::Icon;
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

use crate::config::APP_VERSION;
use std::time::Duration;

impl CybOs {
    pub(crate) fn shell_ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if ui.input(|i| i.key_pressed(egui::Key::Q) && i.modifiers.command) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        self.runtime.tick();
        if let Some(answer) = self.poll_robot_job() {
            self.robot_output = answer.clone();
            self.chat_output = answer.clone();
            self.push_chat_message("ROBOTCYB", answer, false);
        }
        self.ensure_qwen();
        self.poll_lan_scan();
        self.poll_secure_events();
        self.poll_secure_send();
        self.poll_group_chat(ui.ctx());
        self.vpn_setup.poll_diagnostics();
        self.poll_lan_send();
        self.poll_lan_events();
        self.poll_cyblex();
        self.poll_cybdex();
        self.poll_browser();
        self.neon_theme(ui.ctx());
        super::telemetry::record_process(ui.ctx());
        super::telemetry::record(ui.ctx(), "Устройства рядом", self.lan_peers.iter().filter(|p| p.last_seen.elapsed().as_secs() <= 20).count() as f32);
        super::telemetry::record(ui.ctx(), "Активные торренты", self.cyblex_torrents.iter().filter(|t| !t.paused).count() as f32);
        super::telemetry::record(ui.ctx(), "Размер страницы в режиме чтения", self.browser_document_bytes.unwrap_or(0) as f32 / 1024.0);
        super::telemetry::record(ui.ctx(), "Узлы графа знаний", self.nodes.len() as f32);

        let neon = Self::neon();
        let dim = Color32::from_rgb(105, 185, 145);
        let bg = Color32::from_rgb(2, 8, 6);
        let panel = Color32::from_rgb(4, 15, 10);

        if ui.input(|i| i.key_pressed(egui::Key::K) && (i.modifiers.command || i.modifiers.mac_cmd))
        {
            self.search_focus = true;
        }

        egui::Panel::top("cybos_header")
            .exact_size(64.0)
            .frame(
                egui::Frame::NONE
                    .fill(bg)
                    .inner_margin(egui::Margin::symmetric(16, 10)),
            )
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    crate::brand::show(ui, 40.0);
                    ui.label(RichText::new("CybOS").size(20.0).strong().color(neon));
                    crate::i18n::selector(ui, &self.store);
                    if ui.small_button(crate::i18n::tr(ui.ctx(), "Окно / полный экран")).clicked() {
                        let full = ui.ctx().input(|i| i.viewport().fullscreen.unwrap_or(false));
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Fullscreen(!full));
                    }
                    ui.add_space(10.0);
                    ui.add_space(18.0);

                    let (search_icon, _) =
                        ui.allocate_exact_size(Vec2::splat(22.0), egui::Sense::hover());
                    Self::paint_icon(ui.painter(), search_icon.center(), Icon::Search, neon, 16.0);

                    let search = ui.add_sized(
                        [ui.available_width().min(300.0).max(100.0) - 80.0, 30.0],
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text(crate::i18n::tr(ui.ctx(), "⌘K  Найти раздел…"))
                            .id(egui::Id::new("cybos_search")),
                    );
                    if self.search_focus {
                        search.request_focus();
                        self.search_focus = false;
                    }
                    if (search.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        || ui
                            .add(egui::Button::new(
                                RichText::new(crate::i18n::tr(ui.ctx(), "Найти")).strong().color(neon),
                            ))
                            .clicked()
                    {
                        self.apply_search();
                    }
                });
            });

        egui::Panel::left("cybos_left")
            .exact_size(190.0)
            .frame(egui::Frame::NONE.fill(panel).inner_margin(10))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(RichText::new(crate::i18n::tr(ui.ctx(), "ЛИЧНОЕ ПРОСТРАНСТВО")).size(10.0).color(dim));
                    ui.add_space(16.0);
                    for (page, label) in [
                        (Page::Graph, "Граф знаний"),
                        (Page::Chat, "CybChat"),
                        (Page::Brain, "Мозг"),
                        (Page::Browser, "Браузер"),
                        (Page::CybLex, "CybLex"),
                        (Page::Robot, "Агенты"),
                        (Page::Network, "Устройства"),
                        (Page::Vpn, "CybVPN"),
                        (Page::Wallet, "CybWallet"),
                        (Page::Assets, "Токены"),
                        (Page::CybDex, "CybDEX"),
                        (Page::System, "Cyb Диспетчер"),
                    ] {
                        if ui
                            .add_sized(
                                [ui.available_width(), 38.0],
                                egui::Button::new(crate::i18n::tr(ui.ctx(), label)).selected(self.page == page)
                                    .fill(if page == Page::Graph { Color32::from_rgb(20, 58, 44) } else if self.page == page { Color32::from_rgb(16, 45, 33) } else { panel }),
                            )
                            .clicked()
                        {
                            self.go(page);
                        }
                        ui.add_space(4.0);
                    }
                    ui.add_space(20.0);
                    ui.collapsing(crate::i18n::tr(ui.ctx(), "Дополнительно"), |ui| {
                        for (page, label) in [
                            (Page::Radar, "CybRadar"),
                            (Page::Farm, "Демо фермы"),
                            (Page::Cameras, "Камеры"),
                        ] {
                            if ui.selectable_label(self.page == page, crate::i18n::tr(ui.ctx(), label)).clicked() {
                                self.go(page);
                            }
                        }
                    });
                });
            });

        egui::Panel::bottom("cybos_bottom")
            .exact_size(30.0)
            .frame(egui::Frame::NONE.fill(bg))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("cybOS {}", APP_VERSION))
                            .size(11.0)
                            .color(dim),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(crate::i18n::tr(ui.ctx(), "Общение · знания · исследование"))
                            .size(11.0)
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
                        Self::paint_icon(
                            ui.painter(),
                            icon_rect.center(),
                            self.page.icon(),
                            neon,
                            18.0,
                        );
                        ui.label(
                            RichText::new(crate::i18n::tr(ui.ctx(), self.page.title()))
                                .size(18.0)
                                .strong()
                                .color(neon),
                        );
                    });
                    ui.label(RichText::new(self.page.description()).size(13.0).color(dim));
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);
                    match self.page {
                        Page::Wallet => {
                            ui.heading("CybWallet");
                            ui.label("Локальный профиль открыт. Ключ защищён паролем и 24 словами восстановления.");
                            ui.label(format!("ID профиля: {}",self.wallet_profile_id));
                            ui.label("Фраза BIP39 восстанавливает профиль CybOS. Для Monero/CybHero используется отдельный формат кошелька.");
                            ui.label("Кошелёк реальных средств и отправка транзакций пока не подключены. Это хранилище ключа доступа к приложению.");
                            ui.label("Для блокировки завершите CybOS: следующий запуск снова потребует вход. Активная SQLite-база зашифрована. Старый cybos.db остаётся открытой резервной копией; остальные файлы пока не зашифрованы.");
                        }
                        Page::Vpn => { egui::ScrollArea::vertical().show(ui, |ui| self.vpn_page(ui)); }
                        Page::Graph => self.graph(ui),
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
                        Page::Radar => {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                super::radio_radar::show(ui);
                                ui.separator();
                                ui.collapsing("Устройства CybOS рядом", |ui| self.network(ui));
                            });
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
