use eframe::egui::{self, Color32, Stroke};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

#[derive(Clone)]
struct Samples {
    last: Instant,
    values: VecDeque<f32>,
}

/// Recorded values only: one sample per second, with no synthetic history.
pub(crate) fn record(ctx: &egui::Context, name: &str, value: f32) {
    let id = egui::Id::new(("measured_telemetry", name));
    ctx.data_mut(|data| {
        let mut s = data.get_temp::<Samples>(id).unwrap_or_else(|| Samples {
            last: Instant::now() - Duration::from_secs(1),
            values: VecDeque::new(),
        });
        if s.last.elapsed() >= Duration::from_secs(1) {
            s.values.push_back(value.max(0.0));
            if s.values.len() > 60 {
                s.values.pop_front();
            }
            s.last = Instant::now();
        }
        data.insert_temp(id, s.clone());
        s
    });
}

pub(crate) fn chart(ui: &mut egui::Ui, name: &str, value: f32, unit: &str) {
    record(ui.ctx(), name, value);
    let id = egui::Id::new(("measured_telemetry", name));
    let Some(samples) = ui.ctx().data(|d| d.get_temp::<Samples>(id)) else {
        return;
    };
    ui.label(format!(
        "{name} · {value:.0} {unit} · последние 60 измерений"
    ));
    let (r, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 44.0), egui::Sense::hover());
    let painter = ui.painter_at(r);
    let color = Color32::from_rgb(65, 225, 160);
    painter.line_segment(
        [r.left_bottom(), r.right_bottom()],
        Stroke::new(1.0, Color32::DARK_GRAY),
    );
    let max = samples.values.iter().copied().fold(1.0_f32, f32::max);
    let points: Vec<_> = samples
        .values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            egui::pos2(
                r.left() + i as f32 / 59.0 * r.width(),
                r.bottom() - v / max * (r.height() - 4.0),
            )
        })
        .collect();
    if points.len() > 1 {
        painter.add(egui::Shape::line(points, Stroke::new(1.5, color)));
    } else if let Some(p) = points.first() {
        painter.circle_filled(*p, 2.0, color);
    }
}

#[derive(Clone)]
struct ProcessMeter {
    last: Instant,
    cpu_seconds: Option<f64>,
    cpu_percent: Option<f32>,
    ram_mib: Option<f32>,
}
#[cfg(unix)]
fn cpu_seconds() -> Option<f64> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    // getrusage initializes the complete struct on success; no uninitialized data is read on failure.
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return None;
    }
    let usage = unsafe { usage.assume_init() };
    Some(
        usage.ru_utime.tv_sec as f64
            + usage.ru_utime.tv_usec as f64 / 1e6
            + usage.ru_stime.tv_sec as f64
            + usage.ru_stime.tv_usec as f64 / 1e6,
    )
}
#[cfg(not(unix))]
fn cpu_seconds() -> Option<f64> {
    None
}
#[cfg(target_os = "macos")]
fn ram_mib() -> Option<f32> {
    let mut info = std::mem::MaybeUninit::<libc::mach_task_basic_info>::uninit();
    let mut count = libc::MACH_TASK_BASIC_INFO_COUNT;
    // The flavor and count describe exactly mach_task_basic_info. Mach writes this output on success.
    let result = unsafe {
        libc::task_info(
            libc::mach_task_self(),
            libc::MACH_TASK_BASIC_INFO,
            info.as_mut_ptr().cast(),
            &mut count,
        )
    };
    if result != libc::KERN_SUCCESS || count != libc::MACH_TASK_BASIC_INFO_COUNT {
        return None;
    }
    Some(unsafe { info.assume_init() }.resident_size as f32 / (1024.0 * 1024.0))
}
#[cfg(target_os = "linux")]
fn ram_mib() -> Option<f32> {
    let text = std::fs::read_to_string("/proc/self/statm").ok()?;
    let pages: u64 = text.split_whitespace().nth(1)?.parse().ok()?;
    let page_bytes = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page_bytes <= 0 {
        return None;
    }
    Some(pages as f32 * page_bytes as f32 / (1024.0 * 1024.0))
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn ram_mib() -> Option<f32> {
    None
}

pub(crate) fn record_process(ctx: &egui::Context) -> (Option<f32>, Option<f32>) {
    let id = egui::Id::new("dispatcher_process_meter");
    let meter = ctx.data_mut(|d| {
        let mut meter = d
            .get_temp::<ProcessMeter>(id)
            .unwrap_or_else(|| ProcessMeter {
                last: Instant::now(),
                cpu_seconds: cpu_seconds(),
                cpu_percent: None,
                ram_mib: ram_mib(),
            });
        if meter.last.elapsed() >= Duration::from_secs(1) {
            let now = cpu_seconds();
            meter.cpu_percent = now.zip(meter.cpu_seconds).map(|(now, before)| {
                ((now - before).max(0.0) / meter.last.elapsed().as_secs_f64() * 100.0) as f32
            });
            meter.cpu_seconds = now;
            meter.ram_mib = ram_mib();
            meter.last = Instant::now();
        }
        d.insert_temp(id, meter.clone());
        meter
    });
    if let Some(value) = meter.cpu_percent {
        record(ctx, "CPU CybOS", value);
    }
    if let Some(value) = meter.ram_mib {
        record(ctx, "Память CybOS", value);
    }
    (meter.cpu_percent, meter.ram_mib)
}
#[cfg(test)]
mod tests {
    #[test]
    #[cfg(target_os = "macos")]
    fn process_memory_and_cpu_are_measured_by_the_os() {
        assert!(super::ram_mib().unwrap() > 0.0);
        assert!(super::cpu_seconds().unwrap() >= 0.0);
    }
}
