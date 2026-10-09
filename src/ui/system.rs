use crate::config::APP_VERSION;
use crate::CybOs;
use eframe::egui;
use egui::{Color32, RichText, Stroke, Vec2};
use std::fs;
use std::process::Command;

impl CybOs {
    pub(crate) fn system(&mut self, ui: &mut egui::Ui) {
        let neon = Self::neon();
        let dim = Color32::from_rgb(55, 145, 105);
        let soft = Color32::from_rgb(180, 225, 200);
        let telemetry = &self.telemetry;
        let sample = &telemetry.current;
        let memory_percent = if sample.memory_total_bytes > 0 {
            sample.memory_used_bytes as f32 / sample.memory_total_bytes as f32 * 100.0
        } else { 0.0 };

        ui.label(RichText::new("SYSTEM OBSERVABILITY · LOCAL TELEMETRY").size(12.0).strong().color(neon));
        ui.label(RichText::new("Host and process samples every 2 seconds · 60-point rolling history · no telemetry upload").size(10.0).color(dim));
        ui.add_space(10.0);

        egui::Grid::new("telemetry_summary").num_columns(3).spacing([10.0, 8.0]).show(ui, |ui| {
            self.card(ui, "CPU LOAD", &format!("{:.1}%", sample.cpu_percent.clamp(0.0, 100.0)));
            self.card(ui, "RAM USED", &format!("{} / {}", bytes(sample.memory_used_bytes), bytes(sample.memory_total_bytes)));
            self.card(ui, "TEMPERATURE", &sample.temperature_celsius.map(|t| format!("{t:.1} °C")).unwrap_or_else(|| "NOT EXPOSED".into()));
            ui.end_row();
            self.card(ui, "PROCESSES", &sample.process_count.to_string());
            self.card(ui, "DISK FREE", &bytes(sample.disk_available_bytes));
            self.card(ui, "CPU CORES", &self.telemetry.system_cpu_count().to_string());
            ui.end_row();
        });
        ui.add_space(12.0);

        let cpu_history = self.telemetry.history.iter().map(|s| s.cpu_percent.clamp(0.0, 100.0)).collect::<Vec<_>>();
        let ram_history = self.telemetry.history.iter().map(|s| {
            if s.memory_total_bytes == 0 { 0.0 } else { s.memory_used_bytes as f32 / s.memory_total_bytes as f32 * 100.0 }
        }).collect::<Vec<_>>();
        ui.columns(2, |columns| {
            columns[0].label(RichText::new("CPU · LAST 2 MIN").size(10.0).strong().color(neon));
            sparkline(&mut columns[0], &cpu_history, 100.0, neon);
            columns[0].label(RichText::new(format!("Current {:.1}% · samples {}", sample.cpu_percent, cpu_history.len())).size(9.0).color(dim));
            columns[1].label(RichText::new("MEMORY · LAST 2 MIN").size(10.0).strong().color(neon));
            sparkline(&mut columns[1], &ram_history, 100.0, Color32::from_rgb(100, 220, 180));
            columns[1].label(RichText::new(format!("Current {:.1}% · {} used", memory_percent, bytes(sample.memory_used_bytes))).size(9.0).color(dim));
        });

        ui.add_space(14.0);
        ui.label(RichText::new("PROCESS HOTSPOTS").size(11.0).strong().color(neon));
        ui.label(RichText::new("Top processes by reported CPU use; values are OS snapshots and may fluctuate between samples.").size(9.0).color(dim));
        egui::Grid::new("telemetry_processes").num_columns(4).striped(true).spacing([12.0, 5.0]).show(ui, |ui| {
            for heading in ["PROCESS", "PID", "CPU", "MEMORY"] {
                ui.label(RichText::new(heading).size(9.0).strong().color(dim));
            }
            ui.end_row();
            for process in &self.telemetry.top_processes {
                ui.label(RichText::new(clip(&process.name, 30)).size(10.0).color(soft));
                ui.label(RichText::new(&process.pid).size(9.0).color(dim));
                ui.label(RichText::new(format!("{:.1}%", process.cpu_percent)).size(9.0).color(soft));
                ui.label(RichText::new(bytes(process.memory_bytes)).size(9.0).color(soft));
                ui.end_row();
            }
        });
        if self.telemetry.top_processes.is_empty() {
            ui.label(RichText::new("Process list is not available yet.").size(10.0).color(dim));
        }

        ui.add_space(14.0);
        ui.label(RichText::new("STORAGE · NETWORK · SENSORS").size(11.0).strong().color(neon));
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(format!("Disk capacity {}", bytes(sample.disk_total_bytes))).size(10.0).color(soft));
            ui.label(RichText::new(format!("Free {}", bytes(sample.disk_available_bytes))).size(10.0).color(soft));
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(format!("Network received counter {}", bytes(sample.network_received_bytes))).size(10.0).color(soft));
            ui.label(RichText::new(format!("sent {}", bytes(sample.network_transmitted_bytes))).size(10.0).color(soft));
        });
        ui.label(RichText::new(format!(
            "Thermal sensors reported by OS: {}. Highest exposed reading: {}",
            self.telemetry.sensor_count,
            sample.temperature_celsius.map(|t| format!("{t:.1} °C")).unwrap_or_else(|| "unavailable on this platform / hardware".into())
        )).size(10.0).color(dim));
        ui.label(RichText::new("GPU utilization, VRAM, per-device temperature, and fan speed are not yet portable across supported Macs/PCs; cybOS will not substitute fabricated values.").size(9.0).color(dim));

        ui.add_space(14.0);
        ui.label(RichText::new("WORKERS & DISPATCH").size(11.0).strong().color(neon));
        ui.label(RichText::new(format!(
            "{} / {} runtime cells report healthy · scheduler ticks {}",
            self.runtime.healthy_count(), self.runtime.cells.len(), self.runtime.ticks
        )).size(10.0).color(dim));
        egui::Grid::new("telemetry_cells").num_columns(6).striped(true).spacing([9.0, 5.0]).show(ui, |ui| {
            for heading in ["CELL", "STATUS", "RUNS", "LAST RUN", "BUDGET", "OVERRUNS"] {
                ui.label(RichText::new(heading).size(8.0).strong().color(dim));
            }
            ui.end_row();
            for cell in &self.runtime.cells {
                ui.label(RichText::new(cell.id).size(9.0).color(soft));
                ui.label(RichText::new(cell.status).size(9.0).color(if cell.status == "ERROR" || cell.status == "OVER_BUDGET" { Color32::LIGHT_RED } else { soft }));
                ui.label(RichText::new(cell.runs.to_string()).size(9.0).color(soft));
                ui.label(RichText::new(format!("{} µs", cell.last_run.as_micros()).as_str()).size(9.0).color(soft));
                ui.label(RichText::new(format!("{} ms", cell.budget.as_millis()).as_str()).size(9.0).color(soft));
                ui.label(RichText::new(cell.overruns.to_string()).size(9.0).color(if cell.overruns > 0 { Color32::LIGHT_RED } else { soft }));
                ui.end_row();
            }
        });

        ui.add_space(14.0);
        ui.label(RichText::new("RECENT WORKER TASKS").size(11.0).strong().color(neon));
        ui.label(RichText::new("Execution spans from worker contracts · bounded to 100 tasks · queue wait and per-task CPU attribution are not yet measured.").size(9.0).color(dim));
        let traces = crate::runtime::recent_worker_traces();
        egui::Grid::new("telemetry_worker_traces").num_columns(5).striped(true).spacing([9.0, 5.0]).show(ui, |ui| {
            for heading in ["CELL", "STATUS", "ELAPSED", "BUDGET", "TASK ID"] {
                ui.label(RichText::new(heading).size(8.0).strong().color(dim));
            }
            ui.end_row();
            for trace in traces.iter().take(12) {
                ui.label(RichText::new(&trace.cell).size(9.0).color(soft));
                ui.label(RichText::new(&trace.status).size(9.0).color(if trace.status == "ERROR" || trace.status == "TIMEOUT" || trace.status == "OVER_BUDGET" { Color32::LIGHT_RED } else { soft }));
                ui.label(RichText::new(format!("{} ms", trace.elapsed_ms)).size(9.0).color(soft));
                ui.label(RichText::new(format!("{} ms", trace.budget_ms)).size(9.0).color(soft));
                ui.label(RichText::new(trace.id.chars().take(8).collect::<String>()).size(8.0).color(dim));
                ui.end_row();
            }
        });
        if traces.is_empty() {
            ui.label(RichText::new("No worker contracts have started in this session yet.").size(9.0).color(dim));
        }

        ui.add_space(14.0);
        ui.label(RichText::new("MEDIA & MAP INSTRUMENTATION STATUS").size(11.0).strong().color(neon));
        status_line(ui, "VIDEO", "Not instrumented: current Cameras page does not yet expose a connected decoder pipeline, frame rate, dropped frames, decode latency, or GPU video memory.");
        status_line(ui, "MAPS", "Offline Atlas currently inspects MBTiles archives; map rendering, tile-cache hit rate, render time, and GPU draw load are not yet implemented.");
        status_line(ui, "TASK QUEUE", "Runtime-cell ticks, budgets, run counts and overruns are shown above. Per-task queue wait time and worker-level CPU attribution still need explicit instrumentation.");

        ui.add_space(12.0);
        for (k, v) in [
            ("VERSION", APP_VERSION.to_string()),
            ("LANGUAGE", self.language.label().to_string()),
            ("NODE", self.node_id.clone()),
            ("DATABASE", self.store.path.to_str().unwrap_or("—").to_string()),
            ("RENDERER", "egui / eframe".to_string()),
            ("QWEN", self.qwen_status.clone()),
            ("KEYS", "Noise static + TOFU peer keys stored locally".to_string()),
        ] {
            ui.horizontal(|ui| {
                ui.label(RichText::new(k).size(10.0).strong().color(neon));
                ui.label(RichText::new(v).size(10.0).color(soft));
            });
        }
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            let integrity_ok = self.database_integrity == "ok";
            ui.label(RichText::new(format!("DATABASE INTEGRITY · {}", self.database_integrity)).size(10.0).color(if integrity_ok { neon } else { Color32::LIGHT_RED }));
            if ui.button(crate::language::tr(self.language, "check_database")).clicked() {
                self.database_integrity = self.store.database_integrity();
                self.notify(format!("DATABASE CHECK: {}", self.database_integrity));
            }
            if ui.button(crate::language::tr(self.language, "export_state")).clicked() {
                let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
                let path = dirs_fallback_for_export().join(format!("cybOS-state-{timestamp}.json"));
                let payload = serde_json::json!({
                    "version": APP_VERSION,
                    "node_id": self.node_id,
                    "status": self.status,
                    "qwen_status": self.qwen_status,
                    "telemetry": self.telemetry.export_json(),
                    "runtime": self.runtime.cells.iter().map(|cell| serde_json::json!({
                        "id": cell.id, "status": cell.status, "budget_ms": cell.budget.as_millis(),
                        "last_run_ms": cell.last_run.as_millis(), "runs": cell.runs, "overruns": cell.overruns,
                    })).collect::<Vec<_>>(),
                    "events": self.store.exportable_events(),
                    "memories": self.store.exportable_memories(),
                    "graph_nodes": &self.nodes,
                    "graph_links": &self.links,
                    "chat": self.store.exportable_chat(),
                    "note": "Private Noise keys and peer TOFU keys are intentionally excluded."
                });
                match serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())
                    .and_then(|content| fs::write(&path, content).map_err(|e| e.to_string())) {
                    Ok(()) => self.notify(format!("STATE EXPORTED: {}", path.display())),
                    Err(error) => self.notify(format!("STATE EXPORT FAILED: {error}")),
                }
            }
            if ui.button(crate::language::tr(self.language, "copy_node_id")).clicked() {
                ui.ctx().copy_text(self.node_id.clone());
                self.notify("NODE ID COPIED");
            }
            if ui.button(crate::language::tr(self.language, "open_data_folder")).clicked() {
                if let Some(parent) = self.store.path.parent() {
                    #[cfg(target_os = "macos")]
                    let mut command = Command::new("open");
                    #[cfg(target_os = "windows")]
                    let mut command = Command::new("explorer");
                    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
                    let mut command = Command::new("xdg-open");
                    let _ = command.arg(parent).spawn();
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

fn bytes(value: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut amount = value as f64;
    let mut unit = 0usize;
    while amount >= 1024.0 && unit < UNITS.len() - 1 {
        amount /= 1024.0;
        unit += 1;
    }
    format!("{amount:.1} {}", UNITS[unit])
}

fn clip(value: &str, max: usize) -> String {
    if value.chars().count() <= max { value.to_string() } else {
        format!("{}…", value.chars().take(max.saturating_sub(1)).collect::<String>())
    }
}

fn status_line(ui: &mut egui::Ui, name: &str, details: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(name).size(9.0).strong().color(Color32::from_rgb(100, 220, 180)));
        ui.label(RichText::new(details).size(9.0).color(Color32::from_rgb(165, 195, 180)));
    });
}

fn sparkline(ui: &mut egui::Ui, values: &[f32], max: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width().max(160.0), 58.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_stroke(rect, 3.0, Stroke::new(1.0, Color32::from_rgb(30, 80, 55)), egui::StrokeKind::Inside);
    for level in [0.25_f32, 0.5, 0.75] {
        let y = rect.bottom() - rect.height() * level;
        painter.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], Stroke::new(0.5, Color32::from_rgb(20, 55, 40)));
    }
    if values.len() < 2 { return; }
    let points = values.iter().enumerate().map(|(index, value)| {
        let x = rect.left() + rect.width() * index as f32 / (values.len() - 1) as f32;
        let y = rect.bottom() - rect.height() * (value / max).clamp(0.0, 1.0);
        egui::pos2(x, y)
    }).collect::<Vec<_>>();
    for pair in points.windows(2) {
        painter.line_segment([pair[0], pair[1]], Stroke::new(1.7, color));
    }
}

fn dirs_fallback_for_export() -> std::path::PathBuf {
    std::env::var("HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::PathBuf::from(".")).join("Desktop")
}
