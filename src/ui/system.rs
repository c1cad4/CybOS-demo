use crate::config::APP_VERSION;
use crate::CybOs;
use eframe::egui;
use egui::{Color32, RichText};
use std::process::Command;

impl CybOs {
    pub(crate) fn system(&mut self, ui: &mut egui::Ui) {
        let neon = Self::neon();
        let dim = Color32::from_rgb(55, 145, 105);
        ui.label(RichText::new("NATIVE DESKTOP RUNTIME · NO BROWSER SHELL").size(11.0).color(dim));
        ui.add_space(10.0);
        egui::Grid::new("sys").num_columns(3).spacing([10.0, 8.0]).show(ui, |ui| {
            self.card(ui, "VERSION", APP_VERSION);
            self.card(ui, "BATTERY", &format!("{:.0}%", self.battery));
            self.card(ui, "TEMP", &format!("{:.1}°C", self.temperature));
            ui.end_row();
        });
        ui.add_space(12.0);
        for (k, v) in [
            ("NODE", self.node_id.as_str()),
            ("DATABASE", self.store.path.to_str().unwrap_or("—")),
            ("RENDERER", "egui / eframe"),
            ("QWEN", self.qwen_status.as_str()),
            ("IDENTITY", "Ed25519 · persistent local key"),
            ("E2E", "X25519 · HKDF-SHA256 · ChaCha20-Poly1305"),
        ] {
            ui.horizontal(|ui| {
                ui.label(RichText::new(k).size(11.0).strong().color(neon).extra_letter_spacing(1.2));
                ui.label(RichText::new(v).size(12.0).color(Color32::from_rgb(180, 225, 200)));
            });
            ui.add_space(4.0);
        }
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button("COPY NODE ID").clicked() {
                ui.ctx().copy_text(self.node_id.clone());
                self.notify("NODE ID COPIED");
            }
            if ui.button("OPEN DATA FOLDER").clicked() {
                if let Some(parent) = self.store.path.parent() {
                    let _ = Command::new("open").arg(parent).spawn();
                    self.notify("OPENED LOCAL DATA FOLDER");
                }
            }
            if ui.button("WRITE SYSTEM EVENT").clicked() {
                self.add_event("SYSTEM", "Manual system pulse from cybOS");
                self.notify("SYSTEM EVENT WRITTEN");
            }
            if ui.button("TEST CRYPTO").clicked() {
                use ring::{agreement, rand};
                let rng = rand::SystemRandom::new();
                let peer = agreement::EphemeralPrivateKey::generate(&agreement::X25519, &rng)
                    .and_then(|key| key.compute_public_key())
                    .ok();
                let ok = peer
                    .and_then(|public| crate::crypto::encrypt_for_peer(
                        &self.identity,
                        "cyb-test-peer",
                        public.as_ref(),
                        b"cybOS crypto self-test",
                    ).ok())
                    .map(|envelope| crate::crypto::verify_envelope(&envelope, "cyb-test-peer"))
                    .unwrap_or(false);
                if ok {
                    self.add_event("CRYPTO", "Ed25519 envelope authentication self-test passed");
                    self.notify("CRYPTO SELF-TEST PASSED");
                } else {
                    self.notify("CRYPTO SELF-TEST FAILED");
                }
            }
        });
    }
}
