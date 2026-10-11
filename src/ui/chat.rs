use crate::CybOs;
use eframe::egui;
use egui::{Color32, RichText, Vec2};

impl CybOs {
    pub(crate) fn chat_page(&mut self, ui: &mut egui::Ui) {
        if let Some(answer) = self.poll_robot_job() {
            self.robot_output = answer.clone();
            self.push_chat_message("ROBOTCYB", answer, false);
            self.add_event("ROBOT", "RobotCYB local chat worker completed the request");
            self.chat_output = self.robot_output.clone();
        }

        let neon = Color32::from_rgb(0, 255, 150);
        let dim = Color32::from_rgb(105, 185, 145);
        let panel = Color32::from_rgb(5, 18, 13);

        ui.vertical(|ui| {
            ui.label(RichText::new("∴  CYBCHAT").size(24.0).strong().color(neon));

            ui.label(
                RichText::new("Encrypted device messages and local assistant")
                    .size(11.0)
                    .color(dim),
            );

            ui.add_space(14.0);
            ui.collapsing("My device identity", |ui| {
                ui.add(egui::Label::new(format!("Node ID: {}", self.node_id)).selectable(true));
                if let Some(fp) = &self.local_secure_fingerprint {
                    ui.label("SHA-256 fingerprint — compare the entire value");
                    ui.add(egui::Label::new(fp).selectable(true).wrap());
                }
            });

            if !self.pending_secure_trust.is_empty() {
                let pending = self.pending_secure_trust.clone();
                egui::Frame::NONE
                    .fill(Color32::from_rgb(32, 22, 8))
                    .stroke(egui::Stroke::new(1.0, Color32::from_rgb(235, 165, 60)))
                    .corner_radius(egui::CornerRadius::same(10))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("⚠  VERIFY NEW SECURE PEER")
                                .size(12.0)
                                .strong()
                                .color(Color32::from_rgb(255, 190, 70)),
                        );
                        ui.label(
                            RichText::new("Compare the entire fingerprint with the other device through a trusted channel. Approve only a matching key, then verify the connection or retry your message. Approval never sends a message automatically.")
                                .size(12.0)
                                .color(Color32::from_rgb(240, 220, 185)),
                        );
                        ui.add_space(6.0);
                        egui::ScrollArea::vertical()
                            .id_salt("secure-peer-approvals")
                            .max_height(170.0)
                            .show(ui, |ui| {
                                for (index, item) in pending.iter().enumerate().rev() {
                                    ui.separator();
                                    ui.label(RichText::new(format!("PEER · {}", item.node_id)).size(12.0).strong().color(neon));
                                    ui.add(egui::Label::new(format!("SHA-256 · {}", item.fingerprint)).selectable(true).wrap());
                                    ui.horizontal(|ui| {
                                        let trust = ui.button(RichText::new("Approve verified key").strong().color(neon))
                                            .on_hover_text("Persist this exact public key only after you have verified the fingerprint out of band. The sender must retry the message after approval.")
                                            .clicked();
                                        let reject = ui.button(RichText::new("Reject key").strong().color(Color32::from_rgb(255, 110, 110)))
                                            .on_hover_text("Reject this peer without trusting or persisting the new key.")
                                            .clicked();
                                        if trust {
                                            self.approve_pending_secure_peer(index);
                                        } else if reject {
                                            self.pending_secure_trust.remove(index);
                                            self.secure_status = format!("SECURE CHAT · KEY REJECTED · {}", item.node_id);
                                        }
                                    });
                                }
                            });
                    });
                ui.add_space(10.0);
            }

            ui.columns(2, |columns| {
                let ui = &mut columns[0];
                egui::Frame::NONE
                    .fill(panel)
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());

                        ui.label(RichText::new("DEVICES").size(11.0).strong().color(neon));
                        ui.add_space(8.0);

                        ui.collapsing("Add device by address", |ui| {
                            ui.label("Node ID from the other device's My device identity");
                            ui.add(egui::TextEdit::singleline(&mut self.manual_node_input).desired_width(f32::INFINITY).hint_text("Node ID"));
                            ui.label("IPv4 address · secure port 39394");
                            ui.add(egui::TextEdit::singleline(&mut self.manual_address_input).desired_width(f32::INFINITY).hint_text("192.168.1.20"));
                            if ui.add_enabled(self.secure_send_task.is_none(), egui::Button::new("Save and select")).clicked() {
                                self.save_manual_chat_peer();
                            }
                            ui.label("Saving an address does not approve its key.");
                        });
                        if !self.manual_peer_status.is_empty() {
                            ui.add(egui::Label::new(&self.manual_peer_status).wrap());
                        }
                        if ui.add_enabled(self.lan_scan.is_none(), egui::Button::new(if self.lan_scan.is_some() { "Searching…" } else { "Find nearby devices" })).clicked() {
                            self.start_lan_scan();
                        }
                        ui.add_space(8.0);
                        if self.chat_peers().is_empty() {
                            ui.label(
                                RichText::new("No devices yet. Search nearby or add an address.")
                                    .size(12.0)
                                    .color(dim),
                            );
                        } else {
                            egui::ScrollArea::vertical().id_salt("chat-devices").max_height(190.0).show(ui, |ui| {
                            for peer in self.chat_peers() {
                                let selected = self.lan_target.as_deref() == Some(peer.node_id.as_str());

                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("◈").size(16.0).color(neon));

                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new(&peer.node_id)
                                                .size(12.0)
                                                .strong()
                                                .color(if selected { neon } else {
                                                    Color32::from_rgb(175, 235, 205)
                                                }),
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "{} · {}",
                                                peer.address, if peer.version == "manual" { "saved address" } else { "discovered" }
                                            ))
                                            .size(11.0)
                                            .color(dim),
                                        );
                                    });

                                    let trust = match crate::network::peer_trust::approved_key(&self.store, &peer.node_id) {
                                        Ok(Some(_)) => "Key approved",
                                        Ok(None) => "Approval required",
                                        Err(_) => "Identity error",
                                    };
                                    ui.vertical(|ui| {
                                        ui.label(trust);
                                        ui.small(if self.lan_peers.iter().any(|p| p.node_id == peer.node_id) { "Seen on LAN" } else { "Availability unknown" });
                                    });
                                    let label = if selected { "Selected" } else { "Select" };
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new(label)
                                                    .size(11.0)
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

                                if self.manual_peers.iter().any(|p| p.node_id == peer.node_id) {
                                    if ui.add_enabled(self.secure_send_task.is_none(), egui::Button::new("Remove saved address")).clicked() {
                                        self.remove_manual_chat_peer(&peer.node_id);
                                    }
                                }
                                ui.add_space(7.0);
                            }
                            });
                        }
                    });

                let ui = &mut columns[1];

                egui::Frame::NONE
                    .fill(panel)
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_min_width(220.0);

                        ui.label(RichText::new("CONNECTION & DELIVERY").size(11.0).strong().color(neon));
                        ui.add_space(8.0);

                        let target = self
                            .lan_target
                            .as_deref()
                            .unwrap_or("Select a device");

                        ui.label(
                            RichText::new(format!(
                                "Devices: {}\nRecipient: {}\n{}",
                                self.chat_peers().len(),
                                target,
                                self.secure_status
                            ))
                            .size(12.0)
                            .color(Color32::from_rgb(150, 215, 180)),
                        );

                        if ui.add_enabled(self.secure_startup_error.is_none() && self.lan_target.is_some() && self.secure_send_task.is_none(), egui::Button::new("Verify connection"))
                            .on_hover_text("Check the device's Noise key without sending a message. Verify the fingerprint before approval.").clicked() {
                            self.check_secure_connection();
                        }
                        if let Some(id) = &self.lan_target {
                            if let Ok(Some(key)) = crate::network::peer_trust::approved_key(&self.store, id) {
                                ui.collapsing("Approved recipient fingerprint", |ui| {
                                    ui.add(egui::Label::new(crate::network::secure_chat::fingerprint(&key)).selectable(true).wrap());
                                });
                            }
                        }
                        ui.add_space(10.0);

                        ui.label(
                            RichText::new("SECURITY BOUNDARY")
                                .size(12.0)
                                .strong()
                                .color(neon),
                        );

                        ui.label(
                            RichText::new(
                                "Compare full SHA-256 fingerprints through a trusted channel before approving either peer. Noise encrypts transport; peer identity requires approval.",
                            )
                            .size(11.0)
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
                        RichText::new("CONVERSATION")
                            .size(11.0)
                            .strong()
                            .color(neon),
                    );

                    ui.add_space(7.0);

                    egui::ScrollArea::vertical()
                        .max_height(180.0)
                        .auto_shrink([false, true])
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
                                            .size(12.0)
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
                    ui.label(RichText::new("MESSAGE").size(11.0).strong().color(neon));

                    ui.add_space(5.0);

                    let _response = ui.add(
                        egui::TextEdit::multiline(&mut self.chat_input)
                            .desired_rows(3)
                            .desired_width(f32::INFINITY)
                            .hint_text("Write a CybChat message..."),
                    );

                    ui.add_space(7.0);

                    ui.horizontal(|ui| {
                        let local_busy = self.robot_job.is_some();
                        let send = ui
                            .add_enabled(
                                !local_busy && self.secure_send_task.is_none() && !self.chat_input.trim().is_empty(),
                                egui::Button::new(
                                    RichText::new("Ask local assistant")
                                        .size(12.0)
                                        .strong()
                                        .color(neon),
                                )
                                .min_size(Vec2::new(170.0, 34.0)),
                            )
                            .clicked();

                        let can_send_secure =
                            self.secure_startup_error.is_none() && self.lan_target.as_ref().is_some_and(|id| self.chat_peers().iter().any(|p| &p.node_id == id)) && self.secure_send_task.is_none() && !self.chat_input.trim().is_empty();

                        let send_secure = ui
                            .add_enabled(
                                can_send_secure,
                                egui::Button::new(
                                    RichText::new("Send to device")
                                        .size(11.0)
                                        .strong()
                                        .color(neon),
                                )
                                .min_size(Vec2::new(155.0, 34.0)),
                            )
                            .on_disabled_hover_text(
                                "Select a saved or discovered device first.",
                            )
                            .clicked();

                        if send
                        {
                            let t = self.chat_input.trim().to_string();

                            if !t.is_empty() {
                                self.chat_output = "ROBOTCYB · PROCESSING".into();
                                self.push_chat_message("YOU", t.clone(), true);
                                let _ = self.start_robot_job(t.clone());
                                self.chat_input.clear();
                                self.add_event("CHAT", &format!("Local message accepted by RobotCYB: {}", t));
                                self.notify("MESSAGE ACCEPTED BY BOUNDED ROBOT WORKER");
                            }
                        }

                        if send_secure {
                            let t = self.chat_input.trim().to_string();

                            if !t.is_empty() {
                                self.send_secure_chat(&t);
                            }
                        }

                        ui.label(
                            RichText::new(if local_busy {
                                "LOCAL-FIRST · ROBOT WORKER RUNNING · PEER APPROVAL REQUIRED"
                            } else {
                                "LOCAL-FIRST · NOISE ENCRYPTION · ACK AFTER PERSISTENCE"
                            })
                                .size(11.0)
                                .color(dim),
                        );
                    });
                });
        });
    }
}
