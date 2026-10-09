use crate::config::APP_VERSION;
use crate::CybOs;
use eframe::egui;
use egui::{Color32, RichText};
use std::process::Command;
use std::fs;
use std::io::Write;

impl CybOs {
    pub(crate) fn system(&mut self, ui: &mut egui::Ui) {
        let neon = Self::neon();
        let dim = Color32::from_rgb(55, 145, 105);
        ui.label(RichText::new("NATIVE DESKTOP RUNTIME · NO BROWSER SHELL").size(11.0).color(dim));
        ui.add_space(10.0);
        egui::Grid::new("sys").num_columns(3).spacing([10.0, 8.0]).show(ui, |ui| {
            self.card(ui, "VERSION", APP_VERSION);
            self.card(ui, "RUNTIME CELLS", &self.runtime.cells.len().to_string());
            self.card(ui, "LOCAL EVENTS", &self.events.len().to_string());
            ui.end_row();
            self.card(ui, "GRAPH NODES", &self.nodes.len().to_string());
            self.card(ui, "MEMORIES", &self.store.memories().len().to_string());
            self.card(
                ui,
                "QWEN",
                if self.qwen_status.contains("ONLINE") { "ONLINE" } else { "OFFLINE" },
            );
            ui.end_row();
        });
        ui.add_space(12.0);
        for (k, v) in [
            ("NODE", self.node_id.as_str()),
            ("DATABASE", self.store.path.to_str().unwrap_or("—")),
            ("RENDERER", "egui / eframe"),
            ("QWEN", self.qwen_status.as_str()),
            ("KEYS", if cfg!(target_os = "macos") {
                "Noise private key: macOS Keychain · peer trust pins: local SQLite"
            } else {
                "Noise identity: local SQLite · peer trust pins: local SQLite"
            }),
        ] {
            ui.horizontal(|ui| {
                ui.label(RichText::new(k).size(11.0).strong().color(neon).extra_letter_spacing(1.2));
                ui.label(RichText::new(v).size(12.0).color(Color32::from_rgb(180, 225, 200)));
            });
            ui.add_space(4.0);
        }
        ui.add_space(12.0);
        ui.label(RichText::new("RUNTIME CELLS").size(11.0).strong().color(neon));
        ui.add_space(6.0);
        ui.label(
            RichText::new(format!(
                "{} / {} cells READY · tick {}",
                self.runtime.healthy_count(),
                self.runtime.cells.len(),
                self.runtime.ticks
            ))
            .size(10.0)
            .color(dim),
        );
        for cell in &self.runtime.cells {
            let status_color = match cell.status {
                "READY" => neon,
                "RUNNING" => Color32::from_rgb(110, 190, 255),
                "ERROR" | "TIMEOUT" | "OVER_BUDGET" => Color32::from_rgb(255, 130, 115),
                _ => Color32::from_rgb(225, 175, 90),
            };
            egui::Frame::new()
                .fill(Color32::from_rgb(4, 15, 10))
                .corner_radius(6)
                .inner_margin(egui::Margin::symmetric(9, 6))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("●").color(status_color).size(10.0));
                        ui.label(
                            RichText::new(format!("{} · {}", cell.id, cell.status))
                                .strong()
                                .size(10.0)
                                .color(status_color),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{}ms budget · last status signal {}ms ago · {} runs · {} overruns",
                                cell.budget.as_millis(),
                                cell.signal_age_ms(),
                                cell.runs,
                                cell.overruns
                            ))
                            .size(9.0)
                            .color(Color32::from_rgb(145, 190, 165)),
                        );
                    });
                });
            ui.add_space(3.0);
        }
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            let integrity_ok = self.database_integrity == "ok";
            ui.label(
                RichText::new(format!("DATABASE INTEGRITY · {}", self.database_integrity))
                    .size(10.0)
                    .color(if integrity_ok {
                        neon
                    } else {
                        Color32::from_rgb(255, 150, 120)
                    }),
            );

            if ui.button("CHECK DATABASE").clicked() {
                self.database_integrity = self.store.database_integrity();
                self.notify(format!("DATABASE CHECK: {}", self.database_integrity));
            }

            if ui.button("EXPORT STATE").clicked() {
                let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
                let path = dirs_fallback_for_export()
                    .join(format!("cybOS-state-{timestamp}.json"));

                // Diagnostics are metadata-only by default. Chat, memory text,
                // event text, trust records and credentials may contain private data.
                let payload = serde_json::json!({
                    "schema_version": 1,
                    "version": APP_VERSION,
                    "node_id": self.node_id,
                    "status": self.status,
                    "qwen_status": self.qwen_status,
                    "database_integrity": self.database_integrity,
                    "counts": {
                        "runtime_cells": self.runtime.cells.len(),
                        "ready_cells": self.runtime.healthy_count(),
                        "local_events": self.store.exportable_events().len(),
                        "memories": self.store.exportable_memories().len(),
                        "graph_nodes": self.nodes.len(),
                        "graph_links": self.links.len(),
                        "chat_messages": self.store.exportable_chat().len(),
                    },
                    "runtime": self.runtime.cells.iter().map(|cell| serde_json::json!({
                        "id": cell.id,
                        "status": cell.status,
                        "budget_ms": cell.budget.as_millis(),
                        "last_status_signal_age_ms": cell.signal_age_ms(),
                        "last_run_ms": cell.last_run.as_millis(),
                        "runs": cell.runs,
                        "overruns": cell.overruns,
                    })).collect::<Vec<_>>(),
                    "note": "Metadata-only diagnostic export. Private chat, memory/event text, Noise keys, peer trust pins, and credentials are excluded."
                });

                match serde_json::to_string_pretty(&payload)
                    .map_err(|error| error.to_string())
                    .and_then(|content| write_private_export(&path, content.as_bytes())) {
                    Ok(()) => self.notify(format!("STATE EXPORTED: {}", path.display())),
                    Err(error) => self.notify(format!("STATE EXPORT FAILED: {}", error)),
                }
            }

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
        });
    }
}

fn dirs_fallback_for_export() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("Desktop")
}

fn write_private_export(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("cannot create private diagnostic export {}: {error}", path.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("cannot write diagnostic export: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("cannot sync diagnostic export: {error}"))?;
    Ok(())
}
