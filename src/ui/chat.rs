use crate::CybOs;
use eframe::egui;
use egui::{Color32, RichText, Vec2};

impl CybOs {
    pub(crate) fn chat_page(&mut self, ui: &mut egui::Ui) {
        self.poll_lan_scan();

        let neon = Color32::from_rgb(0, 255, 150);
        let dim = Color32::from_rgb(55, 145, 105);
        let panel = Color32::from_rgb(5, 18, 13);

        ui.vertical(|ui| {
            ui.label(RichText::new("∴  CYBCHAT").size(24.0).strong().color(neon));

            ui.label(
                RichText::new("MATHEMATICAL CHANNEL · LOCAL-FIRST · PEERS · LAN · P2P · NOSTR")
                    .size(11.0)
                    .color(dim),
            );

            ui.add_space(14.0);

            ui.horizontal(|ui| {
                // PEERS
                egui::Frame::NONE
                    .fill(panel)
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_min_width(220.0);

                        ui.label(RichText::new("∴ PEERS").size(11.0).strong().color(neon));
                        ui.add_space(8.0);

                        for (symbol, name, status) in [
                            ("◉", "LOCAL NODE", self.node_id.as_str()),
                            ("◉", "ROBOTCYB", "LOCAL AGENT"),
                            ("Ψ", "BRAIN", self.qwen_status.as_str()),
                        ] {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(symbol).size(16.0).color(neon));
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(name)
                                            .size(10.0)
                                            .strong()
                                            .color(Color32::from_rgb(175, 235, 205)),
                                    );
                                    ui.label(RichText::new(status).size(8.0).color(dim));
                                });
                            });
                            ui.add_space(7.0);
                        }

                        for peer in &self.lan_peers {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("◈").size(16.0).color(neon));
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new("LAN PEER")
                                            .size(10.0)
                                            .strong()
                                            .color(Color32::from_rgb(175, 235, 205)),
                                    );
                                    ui.label(
                                        RichText::new(&peer.node_id)
                                            .size(9.0)
                                            .color(neon),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "{} · v{}",
                                            peer.address, peer.version
                                        ))
                                        .size(8.0)
                                        .color(dim),
                                    );
                                });
                            });
                            ui.add_space(7.0);
                        }
                    });

                ui.add_space(10.0);

                // TRANSPORT
                egui::Frame::NONE
                    .fill(panel)
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_min_width(190.0);

                        ui.label(RichText::new("⟷ TRANSPORT").size(11.0).strong().color(neon));
                        ui.add_space(8.0);

                        ui.label(
                            RichText::new(format!(
                                "⌂ LOCAL · READY\n⟷ LAN · {} PEER(S)\n◌ BLE · ADAPTER ONLY\n↔ P2P · ADAPTER ONLY\n∴ NOSTR · ADAPTER ONLY",
                                self.lan_peers.len()
                            ))
                            .size(9.0)
                            .color(Color32::from_rgb(150, 215, 180)),
                        );
                    });
            });

            ui.add_space(12.0);

            // ------------------------------------------------
            // RESPONSE / HISTORY
            // ------------------------------------------------
            egui::Frame::NONE
                .fill(panel)
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("RESPONSE / MESSAGE STREAM")
                            .size(11.0)
                            .strong()
                            .color(neon),
                    );

                    ui.add_space(7.0);

                    egui::ScrollArea::vertical()
                        .max_height(230.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (who, msg, mine) in self.chat.iter().rev().take(30) {
                                let color = if *mine {
                                    neon
                                } else {
                                    Color32::from_rgb(150, 215, 180)
                                };

                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        RichText::new(format!("{}  ", who))
                                            .size(9.0)
                                            .strong()
                                            .color(color),
                                    );

                                    ui.label(
                                        RichText::new(msg)
                                            .size(11.0)
                                            .color(Color32::from_rgb(190, 230, 210)),
                                    );
                                });

                                ui.add_space(5.0);
                            }
                        });
                });

            ui.add_space(12.0);

            // ------------------------------------------------
            // INPUT
            // ------------------------------------------------
            egui::Frame::NONE
                .fill(panel)
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("INPUT").size(11.0).strong().color(neon));

                    ui.add_space(5.0);

                    let response = ui.add(
                        egui::TextEdit::multiline(&mut self.chat_input)
                            .desired_rows(3)
                            .desired_width(f32::INFINITY)
                            .hint_text("Write a CybChat message..."),
                    );

                    ui.add_space(7.0);

                    ui.horizontal(|ui| {
                        let send = ui
                            .add(
                                egui::Button::new(
                                    RichText::new("∴  SEND MESSAGE")
                                        .size(12.0)
                                        .strong()
                                        .color(neon),
                                )
                                .min_size(Vec2::new(170.0, 34.0)),
                            )
                            .clicked();

                        let send_lan = ui
                            .add_enabled(
                                !self.lan_peers.is_empty(),
                                egui::Button::new(
                                    RichText::new("↗  SEND LAN")
                                        .size(11.0)
                                        .strong()
                                        .color(neon),
                                )
                                .min_size(Vec2::new(125.0, 34.0)),
                            )
                            .on_disabled_hover_text("Discover at least one LAN peer first.")
                            .clicked();

                        if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                            || send
                        {
                            let t = self.chat_input.trim().to_string();

                            if !t.is_empty() {
                                self.chat_output = t.clone();
                                self.push_chat_message("YOU", t.clone(), true);
                                let reply = self.agent_answer(&t);
                                self.push_chat_message("ROBOTCYB", reply, false);
                                self.chat_input.clear();
                                self.add_event("CHAT", &format!("Local message sent: {}", t));
                                self.notify("MESSAGE SENT ON LOCAL CHANNEL");
                            }
                        }

                        if send_lan {
                            let t = self.chat_input.trim().to_string();

                            if !t.is_empty() {
                                self.push_chat_message("YOU", t.clone(), true);
                                self.send_lan_chat(&t);
                                self.chat_input.clear();
                                self.add_event("CHAT", &format!("LAN broadcast queued: {}", t));
                                self.notify("LAN MESSAGE QUEUED");
                            }
                        }

                        ui.label(
                            RichText::new("LOCAL-FIRST · NO FAKE NETWORK")
                                .size(9.0)
                                .color(dim),
                        );
                    });
                });
        });
    }
}
