use crate::{CybOs, Icon};
use eframe::egui;
use egui::{Color32, RichText, Stroke, Vec2};

impl CybOs {
    pub(crate) fn network(&mut self, ui: &mut egui::Ui) {
        self.poll_lan_scan();
        self.refresh_proximity();
        self.poll_lan_send();
        self.sync_ble_advertiser();

        if let Some(contract) = self.ble_scan_contract.clone() {
            if contract.expired() {
                self.ble_scan = None;
                self.ble_scan_contract = None;
                contract.finish("TIMEOUT");
                self.ble_status = "BLE · SCAN TIMEOUT".into();
                self.runtime.set_status("RADAR", "ERROR");
            }
        }

        if let Some(result) = self.ble_scan.as_ref().and_then(crate::network::ble::poll_scan) {
            self.ble_scan = None;
            self.ble_scan_contract = None;
            match result {
                Ok(peers) => {
                    self.ble_peers = peers;
                    self.refresh_proximity();
                    self.ble_status = format!("BLE · {} CYBOS PEER(S)", self.ble_peers.len());
                    self.runtime.set_status("RADAR", "READY");
                    self.notify("BLE RADAR SCAN COMPLETE");
                }
                Err(error) => {
                    self.ble_status = format!("BLE · ERROR · {}", error);
                    self.runtime.set_status("RADAR", "ERROR");
                    self.notify("BLE RADAR UNAVAILABLE");
                }
            }
        }

        let neon = Self::neon();
        let dim = Color32::from_rgb(55, 145, 105);

        if ui.button(crate::i18n::tr(ui.ctx(), "CybVPN")).clicked() { self.go(crate::Page::Vpn); }
        ui.add_space(10.0);
        egui::Frame::new()
            .fill(Color32::from_rgb(4, 16, 11))
            .stroke(Stroke::new(1.0, Color32::from_rgb(24, 90, 58)))
            .corner_radius(10)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Показывать меня другим").size(11.0).strong().color(neon));
                    let label = if self.radar_visible { "Включено" } else { "Выключено" };
                    let button = egui::Button::new(RichText::new(label).strong().color(neon));
                    if ui.add(button).clicked() {
                        self.radar_visible = !self.radar_visible;
                        self.radar_visibility.store(self.radar_visible, std::sync::atomic::Ordering::Relaxed);
                        self.store.set("radar_visible", if self.radar_visible { "true" } else { "false" });
                        self.notify(if self.radar_visible { "CYB RADAR: Включено" } else { "CYB RADAR: Выключено" });
                        self.sync_ble_advertiser();
                    }
                });
                ui.label(RichText::new(
                    "OFF by default. Включено enables LAN discovery and native BLE advertising. BLE exposes only a short node identifier; no GPS coordinates are broadcast."
                ).size(9.0).color(dim));
            });

        ui.add_space(10.0);

        egui::Frame::new()
            .fill(Color32::from_rgb(5, 18, 13))
            .stroke(Stroke::new(1.0, Color32::from_rgb(28, 80, 52)))
            .corner_radius(12)
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("NODE  {}", self.node_id))
                            .strong()
                            .color(neon),
                    );

                    if ui.button("Копировать мой ID").clicked() {
                        ui.ctx().copy_text(self.node_id.clone());
                        self.notify("NODE ID COPIED");
                    }

                    let scanning = self.lan_scan.is_some();

                    let scan_label = if scanning {
                        "Поиск…"
                    } else {
                        "Найти устройства"
                    };

                    if ui
                        .add_enabled(!scanning, egui::Button::new(scan_label))
                        .clicked()
                    {
                        self.start_lan_scan();
                        self.notify("LAN SCAN STARTED");
                    }

                    let ble_scanning = self.ble_scan.is_some();
                    if ui
                        .add_enabled(!ble_scanning, egui::Button::new(if ble_scanning { "BLE Поиск…" } else { "Поиск Bluetooth" }))
                        .clicked()
                    {
                        self.ble_scan = Some(crate::network::ble::start_scan());
                        self.ble_scan_contract = Some(crate::runtime::WorkerContract::new("RADAR", std::time::Duration::from_secs(7)));
                        self.ble_status = "BLE · SCANNING · 5s WINDOW".into();
                        self.runtime.set_status("RADAR", "RUNNING");
                        self.notify("BLE RADAR SCAN STARTED");
                    }
                });

                if let Some(t) = self.last_scan {
                    ui.label(
                        RichText::new(format!(
                            "LAST SCAN {}s AGO · {} PEER(S)",
                            t.elapsed().as_secs(),
                            self.lan_peers.len()
                        ))
                        .small()
                        .color(dim),
                    );
                }

                ui.add_space(7.0);

                ui.label(
                    RichText::new(format!(
                        "LAN DELIVERY · {}",
                        self.lan_delivery_status
                    ))
                    .size(9.0)
                    .color(neon),
                );
            });

        ui.add_space(10.0);

        egui::Frame::new()
            .fill(Color32::from_rgb(4, 16, 11))
            .stroke(Stroke::new(1.0, Color32::from_rgb(22, 70, 45)))
            .corner_radius(10)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Устройства рядом")
                        .size(11.0)
                        .strong()
                        .color(neon),
                );

                ui.add_space(8.0);

                if self.lan_scan.is_some() {
                    ui.label(
                        RichText::new("Ищем устройства…")
                            .size(10.0)
                            .color(dim),
                    );
                } else if self.lan_peers.is_empty() {
                    ui.label(
                        RichText::new(
                            "Других устройств пока нет. Запусти CybOS на втором компьютере и включи видимость.",
                        )
                        .size(10.0)
                        .color(dim),
                    );
                } else {
                    for peer in self.lan_peers.clone() {
                        let selected = self.lan_target.as_deref() == Some(peer.node_id.as_str());

                        ui.horizontal(|ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::hover());

                            Self::paint_icon(
                                ui.painter(),
                                rect.center(),
                                Icon::Node,
                                if selected { neon } else { dim },
                                16.0,
                            );

                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(&peer.node_id)
                                        .size(11.0)
                                        .strong()
                                        .color(if selected { neon } else { Color32::from_rgb(175, 235, 205) }),
                                );

                                ui.label(
                                    RichText::new(format!(
                                        "{} · cybOS {}",
                                        peer.address, peer.version
                                    ))
                                    .size(9.0)
                                    .color(dim),
                                );
                            });

                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(if selected { "Выбрано" } else { "Выбрать" })
                                            .size(8.0)
                                            .strong()
                                            .color(neon),
                                    )
                                    .min_size(Vec2::new(64.0, 24.0)),
                                )
                                .clicked()
                            {
                                self.lan_target = Some(peer.node_id.clone());
                                self.notify(format!("LAN Выбрано: {}", peer.node_id));
                            }
                        });

                        ui.add_space(6.0);
                    }
                }
            });

        // CYB RADAR: real LAN peers now; physical distance is shown only when a transport provides it.
        egui::Frame::new()
            .fill(Color32::from_rgb(3, 14, 10))
            .stroke(Stroke::new(1.0, Color32::from_rgb(24, 100, 64)))
            .corner_radius(12)
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("CYB RADAR").size(13.0).strong().color(neon));
                    ui.label(RichText::new("Устройства в локальной сети").size(9.0).color(dim));
                });
                ui.label(RichText::new(
                    "Радар показывает обнаруженные устройства. Расстояние по локальной сети неизвестно."
                ).size(9.0).color(dim));
                ui.label(RichText::new(&self.ble_status).size(9.0).color(dim));
                ui.add_space(8.0);

                let available = ui.available_width().min(560.0);
                let size = Vec2::new(available, 140.0);
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                let painter = ui.painter_at(rect);
                let center = rect.center();
                let radius = rect.width().min(rect.height()) * 0.40;
                let ring = Stroke::new(1.0, Color32::from_rgb(22, 86, 57));

                for factor in [0.25_f32, 0.5, 0.75, 1.0] {
                    painter.circle_stroke(center, radius * factor, ring);
                }
                painter.line_segment([
                    egui::pos2(center.x - radius, center.y),
                    egui::pos2(center.x + radius, center.y),
                ], ring);
                painter.line_segment([
                    egui::pos2(center.x, center.y - radius),
                    egui::pos2(center.x, center.y + radius),
                ], ring);
                painter.circle_filled(center, 7.0, neon);
                painter.text(center + Vec2::new(10.0, -7.0), egui::Align2::LEFT_CENTER,
                    "YOU", egui::FontId::monospace(9.0), neon);

                for (index, peer) in self.nearby_peers.iter().enumerate() {
                    let strength = peer.rssi.map(|r| ((r as f32 + 100.0) / 70.0).clamp(0.12, 1.0)).unwrap_or(0.45);
                    let angle = index as f32 * 2.399;
                    let radial = peer.lan_address.as_ref().and(peer.rssi).map(|_| radius * (1.0 - strength * 0.72)).unwrap_or(radius * 0.82);
                    let point = center + Vec2::new(angle.cos() * radial, angle.sin() * radial);
                    painter.circle_filled(point, 6.0, neon);
                    let source = match peer.source {
                        crate::network::proximity::ProximitySource::Lan => "LAN",
                        crate::network::proximity::ProximitySource::Ble => "BLE",
                        crate::network::proximity::ProximitySource::LanAndBle => "LAN+BLE",
                    };
                    let signal = peer.rssi.map(|r| format!(" · {} dBm", r)).unwrap_or_default();
                    painter.text(point + Vec2::new(10.0, -8.0), egui::Align2::LEFT_CENTER,
                        format!("{} · {}{}", peer.node_id, source, signal), egui::FontId::monospace(8.0), neon);
                }

                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{} UNIFIED PEER(S) · {} LAN · {} BLE", self.nearby_peers.len(), self.lan_peers.len(), self.ble_peers.len())).size(9.0).color(neon));
                    ui.label(RichText::new("· RADAR POSITION IS NOT GEOLOCATION").size(9.0).color(dim));
                });
            });

        ui.add_space(10.0);


    }
}

impl CybOs {
    pub(crate) fn wireguard_setup_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading(crate::i18n::tr(ui.ctx(), "Защищённая сеть устройств"));
            ui.hyperlink_to("Установить WireGuard", "https://www.wireguard.com/install/");
            ui.label("Prepare a device profile and activate it in WireGuard. CLI diagnostics provide manual snapshots; App Store tunnels are not monitored.");
            ui.label("This profile routes only the private device subnet; it does not set a default internet route or DNS.");
            ui.separator();
            if ui.button(crate::i18n::tr(ui.ctx(), "Подготовить это устройство")).clicked() { self.vpn_setup.create_identity(&self.store); }
            if let Some(key) = &self.vpn_setup.public_key {
                ui.label("Device public key — add this on the server");
                ui.add(egui::Label::new(key).selectable(true).wrap());
                if ui.button(crate::i18n::tr(ui.ctx(), "Копировать публичный ключ")).clicked() { ui.ctx().copy_text(key.clone()); }
            }
            ui.collapsing(crate::i18n::tr(ui.ctx(), "Добавить приглашение"), |ui| {
                ui.add(egui::TextEdit::multiline(&mut self.vpn_setup.invitation_input).desired_rows(3).desired_width(f32::INFINITY).hint_text("Paste device-invitation.json; never paste private keys"));
                if ui.button("Load public invitation").clicked() { self.vpn_setup.import_invitation(); }
            });
            ui.collapsing(crate::i18n::tr(ui.ctx(), "Настроить сервер"), |ui| {
                ui.label("Server endpoint (IP:UDP-port)");
                let mut changed = ui.add(egui::TextEdit::singleline(&mut self.vpn_setup.profile.endpoint).hint_text("203.0.113.10:51820").desired_width(f32::INFINITY)).changed();
                ui.label("Server public key — verify with the administrator");
                changed |= ui.add(egui::TextEdit::singleline(&mut self.vpn_setup.profile.server_key).desired_width(f32::INFINITY)).changed();
                ui.label("Unique device address (/32)"); changed |= ui.text_edit_singleline(&mut self.vpn_setup.profile.address).changed();
                ui.label("Private device subnet (AllowedIPs)"); changed |= ui.text_edit_singleline(&mut self.vpn_setup.profile.network).changed();
                if changed { self.vpn_setup.settings_changed(); }
            });
            ui.checkbox(&mut self.vpn_setup.server_verified, "I verified the server public key through a trusted channel");
            let validation = self.vpn_setup.profile.validate();
            if let Err(error) = &validation { ui.label(error); }
            if ui.add_enabled(validation.is_ok() && self.vpn_setup.server_verified, egui::Button::new("Export WireGuard profile")).clicked() { self.vpn_setup.export(&self.store); }
            ui.add(egui::Label::new(&self.vpn_setup.status).wrap());
            if let Some(path) = &self.vpn_setup.export_path {
                ui.label("Private config file: import locally, do not share.");
                ui.add(egui::Label::new(path.display().to_string()).selectable(true).wrap());
            }
            ui.collapsing("CLI tunnel diagnostics", |ui| {
                ui.label("Reads public keys and handshake timestamps only. App Store tunnels are not monitored by the CLI.");
                if ui.add_enabled(self.vpn_setup.diagnostic_task.is_none(), egui::Button::new("Refresh WireGuard diagnostics")).clicked() { self.vpn_setup.start_diagnostics(); }
                ui.add(egui::Label::new(&self.vpn_setup.diagnostic_status).wrap());
            });
            ui.label("Если сервера ещё нет, можно подготовить устройство сейчас, а сервер добавить позже.");
        });
    }
}
