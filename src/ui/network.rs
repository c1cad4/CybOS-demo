use crate::{CybOs, Icon};
use eframe::egui;
use egui::{Color32, RichText, Stroke, Vec2};

impl CybOs {
    pub(crate) fn network(&mut self, ui: &mut egui::Ui) {
        self.poll_lan_scan();
        self.poll_lan_send();

        let neon = Self::neon();
        let dim = Color32::from_rgb(55, 145, 105);

        ui.label(
            RichText::new("ONE NODE IDENTITY · DISCOVERY · DIRECT DELIVERY · NO FAKE CONNECTIONS")
                .size(11.0)
                .color(dim),
        );

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

                    if ui.button("COPY ID").clicked() {
                        ui.ctx().copy_text(self.node_id.clone());
                        self.notify("NODE ID COPIED");
                    }

                    let scanning = self.lan_scan.is_some();

                    let scan_label = if scanning {
                        "SCANNING…"
                    } else {
                        "SCAN LAN"
                    };

                    if ui
                        .add_enabled(!scanning, egui::Button::new(scan_label))
                        .clicked()
                    {
                        self.start_lan_scan();
                        self.notify("LAN SCAN STARTED");
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
                    RichText::new("LAN PEERS · DIRECT ADDRESSING")
                        .size(11.0)
                        .strong()
                        .color(neon),
                );

                ui.add_space(8.0);

                if self.lan_scan.is_some() {
                    ui.label(
                        RichText::new("Listening for cybOS discovery responses…")
                            .size(10.0)
                            .color(dim),
                    );
                } else if self.lan_peers.is_empty() {
                    ui.label(
                        RichText::new(
                            "No other cybOS nodes discovered on the local broadcast network.",
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
                                        RichText::new(if selected { "TARGET" } else { "SELECT" })
                                            .size(8.0)
                                            .strong()
                                            .color(neon),
                                    )
                                    .min_size(Vec2::new(64.0, 24.0)),
                                )
                                .clicked()
                            {
                                self.lan_target = Some(peer.node_id.clone());
                                self.notify(format!("LAN TARGET: {}", peer.node_id));
                            }
                        });

                        ui.add_space(6.0);
                    }
                }
            });

        ui.add_space(10.0);

        for (name, status, live) in [
            ("LOCAL LOOPBACK", "READY", true),
            ("LAN DISCOVERY", "ACTIVE · UDP BROADCAST · DISCOVERY ONLY", true),
            ("LAN CHAT", "DIRECT UDP · DELIVERY ACK · ENCRYPTED WIRE", true),
            ("BLUETOOTH MESH", "ADAPTER ONLY · NOT CONNECTED", false),
            ("P2P", "ADAPTER ONLY · NOT CONNECTED", false),
            ("NOSTR FALLBACK", "AVAILABLE · NOT CONNECTED", false),
        ] {
            egui::Frame::new()
                .fill(Color32::from_rgb(4, 16, 11))
                .stroke(Stroke::new(
                    1.0,
                    if live {
                        Color32::from_rgb(30, 110, 70)
                    } else {
                        Color32::from_rgb(18, 48, 34)
                    },
                ))
                .corner_radius(9)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::hover());

                        Self::paint_icon(
                            ui.painter(),
                            rect.center(),
                            Icon::Network,
                            if live { neon } else { dim },
                            16.0,
                        );

                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(name)
                                    .size(12.0)
                                    .strong()
                                    .color(if live {
                                        neon
                                    } else {
                                        Color32::from_rgb(160, 200, 175)
                                    }),
                            );

                            ui.label(RichText::new(status).size(10.0).color(dim));
                        });
                    });
                });

            ui.add_space(6.0);
        }
    }
}
