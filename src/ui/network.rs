use crate::{CybOs, Icon};
use eframe::egui;
use egui::{Color32, RichText, Stroke, Vec2};
use std::time::Instant;

impl CybOs {
    pub(crate) fn network(&mut self, ui: &mut egui::Ui) {
        let neon = Self::neon();
        let dim = Color32::from_rgb(55, 145, 105);
        ui.label(
            RichText::new("ONE IDENTITY · TRANSPORT ADAPTERS ARE PRESENT, NOT LIVE")
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
                    ui.label(RichText::new(format!("NODE  {}", self.node_id)).strong().color(neon));
                    if ui.button("COPY ID").clicked() {
                        ui.ctx().copy_text(self.node_id.clone());
                        self.notify("NODE ID COPIED");
                    }
                    if ui.button("SCAN LOCAL").clicked() {
                        self.last_scan = Some(Instant::now());
                        self.add_event(
                            "NETWORK",
                            "Local scan: no live BLE/LAN/P2P peers. Adapters remain ready, not connected.",
                        );
                        self.notify("SCAN COMPLETE · NO LIVE PEERS");
                    }
                });
                if let Some(t) = self.last_scan {
                    ui.label(
                        RichText::new(format!("LAST SCAN {}s AGO", t.elapsed().as_secs()))
                            .small()
                            .color(dim),
                    );
                }
            });
        ui.add_space(10.0);
        for (name, status, live) in [
            ("LOCAL LOOPBACK", "READY", true),
            ("BLUETOOTH MESH", "ADAPTER ONLY · NOT CONNECTED", false),
            ("LAN", "ADAPTER ONLY · NOT CONNECTED", false),
            ("P2P", "ADAPTER ONLY · NOT CONNECTED", false),
            ("NOSTR FALLBACK", "AVAILABLE · NOT CONNECTED", false),
        ] {
            egui::Frame::new()
                .fill(Color32::from_rgb(4, 16, 11))
                .stroke(Stroke::new(1.0, if live { Color32::from_rgb(30, 110, 70) } else { Color32::from_rgb(18, 48, 34) }))
                .corner_radius(9)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::hover());
                        Self::paint_icon(ui.painter(), rect.center(), Icon::Network, if live { neon } else { dim }, 16.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(name).size(12.0).strong().color(if live { neon } else { Color32::from_rgb(160, 200, 175) }));
                            ui.label(RichText::new(status).size(10.0).color(dim));
                        });
                    });
                });
            ui.add_space(6.0);
        }
    }
}
