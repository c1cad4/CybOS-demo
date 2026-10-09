use crate::config::APP_VERSION;
use crate::CybOs;
use eframe::egui;
use egui::{Color32, RichText};
use std::process::Command;
use std::fs;

impl CybOs {
    pub(crate) fn system(&mut self, ui: &mut egui::Ui) {
        let neon = Self::neon();
        let dim = Color32::from_rgb(55, 145, 105);
        ui.label(RichText::new(crate::language::tr(self.language, "system_subtitle")).size(11.0).color(dim));
        ui.add_space(10.0);
        egui::Grid::new("sys").num_columns(3).spacing([10.0, 8.0]).show(ui, |ui| {
            self.card(ui, "VERSION", APP_VERSION);
            self.card(ui, "BATTERY", &format!("{:.0}%", self.battery));
            self.card(ui, "TEMP", &format!("{:.1}°C", self.temperature));
            ui.end_row();
        });
        ui.add_space(12.0);
        for (k, v) in [
            ("LANGUAGE", self.language.label()),
            ("NODE", self.node_id.as_str()),
            ("DATABASE", self.store.path.to_str().unwrap_or("—")),
            ("RENDERER", "egui / eframe"),
            ("QWEN", self.qwen_status.as_str()),
            ("KEYS", "Noise static + TOFU peer keys stored locally"),
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
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} · {} · {}ms budget · heartbeat {}ms ago",
                        cell.id,
                        cell.status,
                        cell.budget.as_millis(),
                        cell.heartbeat_age_ms()
                    ))
                    .size(9.0)
                    .color(Color32::from_rgb(165, 220, 190)),
                );
            });
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

            if ui.button(crate::language::tr(self.language, "check_database")).clicked() {
                self.database_integrity = self.store.database_integrity();
                self.notify(format!("DATABASE CHECK: {}", self.database_integrity));
            }

            if ui.button(crate::language::tr(self.language, "export_state")).clicked() {
                let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
                let path = dirs_fallback_for_export()
                    .join(format!("cybOS-state-{timestamp}.json"));

                let payload = serde_json::json!({
                    "version": APP_VERSION,
                    "node_id": self.node_id,
                    "status": self.status,
                    "qwen_status": self.qwen_status,
                    "runtime": self.runtime.cells.iter().map(|cell| serde_json::json!({
                        "id": cell.id,
                        "status": cell.status,
                        "budget_ms": cell.budget.as_millis(),
                        "last_run_ms": cell.last_run.as_millis(),
                        "runs": cell.runs,
                        "overruns": cell.overruns,
                    })).collect::<Vec<_>>(),
                    "events": self.store.exportable_events(),
                    "memories": self.store.exportable_memories(),
                    "graph_nodes": &self.nodes,
                    "graph_links": &self.links,
                    "chat": self.store.exportable_chat(),
                    "note": "Private Noise keys and peer TOFU keys are intentionally excluded."
                });

                match serde_json::to_string_pretty(&payload)
                    .map_err(|error| error.to_string())
                    .and_then(|content| {
                        fs::write(&path, content).map_err(|error| error.to_string())
                    }) {
                    Ok(()) => self.notify(format!("STATE EXPORTED: {}", path.display())),
                    Err(error) => self.notify(format!("STATE EXPORT FAILED: {}", error)),
                }
            }

            if ui.button(crate::language::tr(self.language, "copy_node_id")).clicked() {
                ui.ctx().copy_text(self.node_id.clone());
                self.notify("NODE ID COPIED");
            }
            if ui.button(crate::language::tr(self.language, "open_data_folder")).clicked() {
                if let Some(parent) = self.store.path.parent() {
                    let _ = Command::new("open").arg(parent).spawn();
                    self.notify("OPENED LOCAL DATA FOLDER");
                }
            }
            if ui.button(crate::language::tr(self.language, "write_system_event")).clicked() {
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
