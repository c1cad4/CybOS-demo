use crate::CybOs;
use eframe::egui;
use egui::{Color32, RichText, Vec2};

impl CybOs {
    pub(crate) fn chat_page(&mut self, ui: &mut egui::Ui) {
        self.poll_lan_scan();
        self.poll_lan_send();

        let neon = Color32::from_rgb(0, 255, 150);
        let dim = Color32::from_rgb(55, 145, 105);
        let panel = Color32::from_rgb(5, 18, 13);

        ui.vertical(|ui| {
            ui.label(RichText::new("∴  CYBCHAT").size(24.0).strong().color(neon));

            ui.label(
                RichText::new("MATHEMATICAL CHANNEL · LOCAL-FIRST · DIRECT PEERS · LAN · P2P · NOSTR")
                    .size(11.0)
                    .color(dim),
            );

            ui.add_space(14.0);

            ui.horizontal(|ui| {
                egui::Frame::NONE
                    .fill(panel)
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_min_width(300.0);

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

                        if self.lan_peers.is_empty() {
                            ui.label(
                                RichText::new("No direct LAN peers discovered.")
                                    .size(9.0)
                                    .color(dim),
                            );
                        } else {
                            for peer in self.lan_peers.clone() {
                                let selected = self.lan_target.as_deref() == Some(peer.node_id.as_str());

                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("◈").size(16.0).color(neon));

                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new(&peer.node_id)
                                                .size(9.0)
                                                .strong()
                                                .color(if selected { neon } else {
                                                    Color32::from_rgb(175, 235, 205)
                                                }),
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "{} · cybOS {}",
                                                peer.address, peer.version
                                            ))
                                            .size(8.0)
                                            .color(dim),
                                        );
                                    });

                                    let label = if selected { "TARGET" } else { "SELECT" };
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new(label)
                                                    .size(8.0)
                                                    .strong()
                                                    .color(neon),
                                            )
                                            .min_size(Vec2::new(58.0, 24.0)),
                                        )
                                        .clicked()
                                    {
                                        self.lan_target = Some(peer.node_id.clone());
                                        self.notify(format!("LAN TARGET: {}", peer.node_id));
                                    }
                                });

                                ui.add_space(7.0);
                            }
                        }
                    });

                ui.add_space(10.0);

                egui::Frame::NONE
                    .fill(panel)
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_min_width(220.0);

                        ui.label(RichText::new("⟷ TRANSPORT").size(11.0).strong().color(neon));
                        ui.add_space(8.0);

                        let target = self
                            .lan_target
                            .as_deref()
                            .unwrap_or("NO TARGET SELECTED");

                        ui.label(
                            RichText::new(format!(
                                "⌂ LOCAL · READY\n⟶ DIRECT LAN · {} PEER(S)\n◈ TARGET · {}\n✓ DELIVERY ACK · {}\n◌ BLE · ADAPTER ONLY\n↔ P2P · ADAPTER ONLY\n∴ NOSTR · ADAPTER ONLY",
                                self.lan_peers.len(),
                                target,
                                self.lan_delivery_status
                            ))
                            .size(9.0)
                            .color(Color32::from_rgb(150, 215, 180)),
                        );

                        ui.add_space(10.0);

                        ui.label(
                            RichText::new("SECURITY BOUNDARY")
                                .size(9.0)
                                .strong()
                                .color(neon),
                        );

                        ui.label(
                            RichText::new(
                                "Directed UDP only. Messages are not end-to-end encrypted yet.",
                            )
                            .size(8.0)
                            .color(dim),
                        );
                    });
            });

            ui.add_space(12.0);

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

                        let can_send_lan =
                            self.lan_target.is_some() && self.lan_send_task.is_none();

                        let send_lan = ui
                            .add_enabled(
                                can_send_lan,
                                egui::Button::new(
                                    RichText::new("↗  SEND DIRECT")
                                        .size(11.0)
                                        .strong()
                                        .color(neon),
                                )
                                .min_size(Vec2::new(135.0, 34.0)),
                            )
                            .on_disabled_hover_text(
                                "Select a LAN peer. Only one direct delivery can run at a time.",
                            )
                            .clicked();

                        if (response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter)))
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
                                let target = self
                                    .lan_target
                                    .as_deref()
                                    .unwrap_or("selected peer")
                                    .to_string();

                                self.push_chat_message(
                                    format!("YOU → {}", target),
                                    t.clone(),
                                    true,
                                );
                                self.send_lan_chat(&t);
                                self.chat_input.clear();
                            }
                        }

                        ui.label(
                            RichText::new("LOCAL-FIRST · DELIVERY IS ACKNOWLEDGED · PLAINTEXT")
                                .size(8.0)
                                .color(dim),
                        );
                    });
                });
        });
    }
}
