//! Read-only operating-system process snapshot. No copied task-manager source.
use eframe::egui;
use std::{
    io::Read,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
#[derive(Clone, Debug)]
struct Process {
    pid: u32,
    cpu: f32,
    memory_mib: f32,
    name: String,
}
#[derive(Clone)]
struct Snapshot(Arc<Mutex<Option<Result<Vec<Process>, String>>>>);
fn parse(raw: &str) -> Vec<Process> {
    raw.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let pid = parts.next()?.parse().ok()?;
            let cpu: f32 = parts.next()?.parse().ok()?;
            let rss: u64 = parts.next()?.parse().ok()?;
            let name = parts.collect::<Vec<_>>().join(" ");
            if !cpu.is_finite() || cpu < 0.0 || name.is_empty() {
                return None;
            }
            Some(Process {
                pid,
                cpu,
                memory_mib: rss as f32 / 1024.0,
                name,
            })
        })
        .take(4096)
        .collect()
}
fn measure() -> Result<Vec<Process>, String> {
    #[cfg(unix)]
    {
        use std::process::{Command, Stdio};
        let mut child = Command::new("ps")
            .args(["-axo", "pid=,pcpu=,rss=,comm="])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        let mut stdout = child.stdout.take().ok_or("Не удалось прочитать процессы")?;
        // Read concurrently so a large process list cannot fill the pipe and deadlock.
        let reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout
                .by_ref()
                .take(2_000_001)
                .read_to_end(&mut bytes)
                .map(|_| bytes)
        });
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                let bytes = reader
                    .join()
                    .map_err(|_| "Сбор процессов прерван")?
                    .map_err(|e| e.to_string())?;
                if !status.success() || bytes.len() > 2_000_000 {
                    return Err("Список процессов недоступен или слишком велик".into());
                }
                return Ok(parse(&String::from_utf8_lossy(&bytes)));
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Истекло время получения процессов".into());
            }
            std::thread::sleep(Duration::from_millis(30));
        }
    }
    #[cfg(not(unix))]
    {
        Err("Сбор списка процессов Windows пока не реализован".into())
    }
}
pub(crate) fn show(ui: &mut egui::Ui) {
    let id = egui::Id::new("process_snapshot");
    ui.collapsing("Процессы компьютера", |ui| {
        ui.label("Данные операционной системы. CPU здесь — усреднение ps; верхний график CybOS показывает нагрузку за интервал.");
        let busy = ui.ctx().data(|d| d.get_temp::<Snapshot>(id)).is_some_and(|s| s.0.lock().map(|v| v.is_none()).unwrap_or(true));
        if ui.add_enabled(!busy, egui::Button::new("Обновить список процессов")).clicked() {
            let data=Snapshot(Arc::new(Mutex::new(None)));ui.ctx().data_mut(|d|d.insert_temp(id,data.clone()));
            let ctx=ui.ctx().clone();std::thread::spawn(move||{let result=measure();if let Ok(mut slot)=data.0.lock(){*slot=Some(result);}ctx.request_repaint();});
        }
        let Some(snapshot)=ui.ctx().data(|d|d.get_temp::<Snapshot>(id)) else{return;};
        let Ok(data)=snapshot.0.lock() else{return;};
        match data.as_ref() {
            None=>{ui.label("Собираем процессы…");},Some(Err(e))=>{ui.label(e);},
            Some(Ok(processes))=>{
                let filter_id=egui::Id::new("process_filter");let mut filter=ui.ctx().data(|d|d.get_temp::<String>(filter_id)).unwrap_or_default();
                ui.add(egui::TextEdit::singleline(&mut filter).hint_text("Название процесса"));ui.ctx().data_mut(|d|d.insert_temp(filter_id,filter.clone()));
                let mut sorted=processes.iter().filter(|p|p.name.to_lowercase().contains(&filter.to_lowercase())).collect::<Vec<_>>();sorted.sort_by(|a,b|b.cpu.total_cmp(&a.cpu));
                egui::ScrollArea::vertical().max_height(260.0).show(ui,|ui|{egui::Grid::new("process_rows").striped(true).show(ui,|ui|{
                    for title in ["PID","Процесс","CPU %","Память МиБ"]{ui.strong(title);}ui.end_row();
                    for p in sorted.iter().take(100){ui.label(p.pid.to_string());ui.label(&p.name);ui.label(format!("{:.1}",p.cpu));ui.label(format!("{:.1}",p.memory_mib));ui.end_row();}
                });});
            }
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parser_keeps_names_with_spaces_and_rejects_invalid_measurements() {
        let rows =
            parse("12 25.0 2048 /Applications/CybOS app\n13 NaN 20 bad\n14 -2 30 bad\nheader");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].memory_mib, 2.0);
        assert_eq!(rows[0].name, "/Applications/CybOS app");
    }
    #[cfg(unix)]
    #[test]
    #[ignore = "ps process enumeration is prohibited by the execution sandbox; verify in the native desktop app"]
    fn actual_os_snapshot_contains_current_process() {
        assert!(
            measure()
                .unwrap()
                .iter()
                .any(|p| p.pid == std::process::id())
        );
    }
}
