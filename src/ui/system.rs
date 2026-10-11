use crate::CybOs;
use crate::config::APP_VERSION;
use eframe::egui;
use egui::{Color32, RichText};
use std::fs;
use std::process::Command;

impl CybOs {
    pub(crate) fn system(&mut self, ui: &mut egui::Ui) {
        ui.label("Что сейчас делает CybOS: соединения, загрузки, локальный ИИ и состояние данных.");
        ui.add_space(12.0);
        let (cpu, ram) = super::telemetry::record_process(ui.ctx());
        ui.columns(2, |columns| {
            if let Some(value) = cpu {
                super::telemetry::chart(&mut columns[0], "CPU CybOS", value, "% одного ядра");
            } else {
                columns[0].label("CPU: собираем первое измерение или измерение недоступно");
            }
            if let Some(value) = ram {
                super::telemetry::chart(&mut columns[1], "Память CybOS", value, "МиБ");
            } else {
                columns[1].label("Измерение памяти пока недоступно на этой системе");
            }
        });
        ui.columns(2, |columns| {
            super::telemetry::chart(
                &mut columns[0],
                "Устройства рядом",
                self.lan_peers
                    .iter()
                    .filter(|p| p.last_seen.elapsed().as_secs() <= 20)
                    .count() as f32,
                "шт.",
            );
            super::telemetry::chart(
                &mut columns[1],
                "Активные торренты",
                self.cyblex_torrents.iter().filter(|t| !t.paused).count() as f32,
                "шт.",
            );
        });
        ui.columns(2, |columns| {
            super::telemetry::chart(
                &mut columns[0],
                "Размер страницы в режиме чтения",
                self.browser_document_bytes.unwrap_or(0) as f32 / 1024.0,
                "КБ",
            );
            super::telemetry::chart(
                &mut columns[1],
                "Узлы графа знаний",
                self.nodes.len() as f32,
                "шт.",
            );
        });
        ui.add_space(12.0);
        egui::Grid::new("dispatcher-modules")
            .striped(true)
            .min_col_width(220.0)
            .spacing([24.0, 10.0])
            .show(ui, |ui| {
                ui.strong("Модуль");
                ui.strong("Состояние");
                ui.end_row();
                for (name, value) in [
                    ("ИИ-помощник", if self.qwen_status.contains("ONLINE") { "Локальная модель готова".into() } else { "Модель недоступна — проверь настройки".into() }),
                    ("CybChat", if self.secure_startup_error.is_some() { "Не удалось открыть соединение. Проверь, не запущена ли другая копия CybOS".into() } else if !self.pending_secure_trust.is_empty() { "Нужно сверить код нового устройства".into() } else if self.secure_send_task.is_some() { "Отправляется сообщение".into() } else if self.secure_status.contains("FAILED") || self.secure_status.contains("TIMEOUT") { "Последняя отправка не завершилась".into() } else { "Защищённое соединение готово".into() }),
                    ("Браузер", if self.browser_status.contains("ERROR") { "Не удалось открыть страницу".into() } else { "Готов к поиску и просмотру сайтов".into() }),
                    ("CybLex", if self.cyblex_status.contains("ERROR") { "Ошибка загрузки — смотри подробности".into() } else { format!("В библиотеке: {} торрентов", self.cyblex_torrents.len()) }),
                    ("Хранилище", if self.database_integrity == "ok" { "Проверено: ошибок нет".into() } else { self.database_integrity.clone() }),
                ] {
                    ui.label(name);
                    ui.add(egui::Label::new(value).wrap());
                    ui.end_row();
                }
            });
        ui.add_space(12.0);
        ui.small("CPU и память измеряются для процесса CybOS. 100% CPU — одно ядро; несколько ядер могут дать больше 100%. Отдельные процессы модели, браузера и плеера сюда не входят. Общесистемный трафик не измеряется.");
        ui.collapsing(
            "Состояния модулей: подробности",
            |ui| {
                for (name, status) in [
                    ("CybChat", &self.secure_status),
                    ("Браузер", &self.browser_status),
                    ("CybLex", &self.cyblex_status),
                ] {
                    ui.strong(name);
                    ui.add(egui::Label::new(status).wrap());
                    ui.add_space(6.0);
                }
            },
        );
        ui.collapsing(
            "Технические подробности и обслуживание",
            |ui| self.system_diagnostics(ui),
        );
    }

    fn system_diagnostics(&mut self, ui: &mut egui::Ui) {
        let neon = Self::neon();
        let dim = Color32::from_rgb(55, 145, 105);
        ui.label(
            RichText::new("NATIVE DESKTOP RUNTIME · NO BROWSER SHELL")
                .size(11.0)
                .color(dim),
        );
        ui.add_space(10.0);
        egui::Grid::new("sys")
            .num_columns(3)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                self.card(ui, "VERSION", APP_VERSION);
                self.card(ui, "DATABASE", &self.database_integrity);
                self.card(
                    ui,
                    "RUNTIME",
                    &format!(
                        "{} / {} ready",
                        self.runtime.healthy_count(),
                        self.runtime.cells.len()
                    ),
                );
                ui.end_row();
            });
        ui.add_space(12.0);
        for (k, v) in [
            ("NODE", self.node_id.as_str()),
            ("DATABASE", self.store.path.to_str().unwrap_or("—")),
            ("RENDERER", "egui / eframe"),
            ("QWEN", self.qwen_status.as_str()),
            ("KEYS", "Noise static + TOFU peer keys stored locally"),
        ] {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(k)
                        .size(11.0)
                        .strong()
                        .color(neon)
                        .extra_letter_spacing(1.2),
                );
                ui.label(
                    RichText::new(v)
                        .size(12.0)
                        .color(Color32::from_rgb(180, 225, 200)),
                );
            });
            ui.add_space(4.0);
        }
        ui.add_space(12.0);
        ui.label(
            RichText::new("RUNTIME CELLS")
                .size(11.0)
                .strong()
                .color(neon),
        );
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

            if ui.button("CHECK DATABASE").clicked() {
                self.database_integrity = self.store.database_integrity();
                self.notify(format!("DATABASE CHECK: {}", self.database_integrity));
            }

            if ui.button("EXPORT STATE").clicked() {
                let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
                let path = dirs_fallback_for_export().join(format!("cybOS-state-{timestamp}.json"));

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
