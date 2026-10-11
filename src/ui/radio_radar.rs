//! Opt-in read-only dump1090 JSON adapter; aircraft are not CybOS users.
use eframe::egui;
use serde::Deserialize;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
#[derive(Clone, Deserialize)]
struct Aircraft {
    hex: String,
    #[serde(default)]
    flight: Option<String>,
    #[serde(default)]
    lat: Option<f64>,
    #[serde(default)]
    lon: Option<f64>,
    seen: f64,
    #[serde(default)]
    seen_pos: Option<f64>,
}
#[derive(Clone, Deserialize)]
struct Feed {
    now: f64,
    aircraft: Vec<Aircraft>,
}
#[derive(Clone)]
struct Request(Arc<Mutex<Option<Result<Feed, String>>>>);
fn parse(bytes: &[u8]) -> Result<Feed, String> {
    if bytes.len() > 2_000_000 {
        return Err("Слишком большой ответ приёмника".into());
    }
    let mut feed: Feed =
        serde_json::from_slice(bytes).map_err(|_| "Неверный формат dump1090 aircraft.json")?;
    if !feed.now.is_finite() || feed.now <= 0.0 || feed.aircraft.len() > 5000 {
        return Err("Некорректный размер или время данных".into());
    }
    feed.aircraft.retain(|a| {
        let hex = a.hex.strip_prefix('~').unwrap_or(&a.hex);
        hex.len() == 6
            && hex.bytes().all(|b| b.is_ascii_hexdigit())
            && a.seen.is_finite()
            && (0.0..=30.0).contains(&a.seen)
    });
    for a in &mut feed.aircraft {
        let valid = a.lat.zip(a.lon).is_some_and(|(lat, lon)| {
            lat.is_finite()
                && lon.is_finite()
                && (-90.0..=90.0).contains(&lat)
                && (-180.0..=180.0).contains(&lon)
        }) && a
            .seen_pos
            .is_some_and(|age| age.is_finite() && (0.0..=30.0).contains(&age));
        if !valid {
            a.lat = None;
            a.lon = None;
        }
        if let Some(name) = &mut a.flight {
            *name = name.chars().filter(|c| !c.is_control()).take(32).collect();
        }
    }
    Ok(feed)
}
fn fetch(port: u16) -> Result<Feed, String> {
    if port == 0 {
        return Err("Укажите порт приёмника".into());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .max_redirects(0)
        .build()
        .into();
    let mut response = agent
        .get(format!("http://127.0.0.1:{port}/data/aircraft.json"))
        .call()
        .map_err(|_| "Локальный dump1090 недоступен. Запустите приёмник или импортируйте JSON.")?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(2_000_000)
        .read_to_vec()
        .map_err(|_| "Ответ приёмника недоступен")?;
    parse(&bytes)
}
pub(crate) fn show(ui: &mut egui::Ui) {
    ui.heading("Авиационный радар");
    ui.label("Отдельный источник: ваш ADS-B приёмник dump1090. Самолёты не считаются пользователями CybOS. Без приёмника данные не появятся.");
    let id = egui::Id::new("adsb_request");
    let port_id = egui::Id::new("adsb_port");
    let mut port = ui
        .ctx()
        .data(|d| d.get_temp::<u16>(port_id))
        .unwrap_or(8088);
    let busy = ui
        .ctx()
        .data(|d| d.get_temp::<Request>(id))
        .is_some_and(|r| r.0.lock().map(|slot| slot.is_none()).unwrap_or(true));
    ui.horizontal(|ui| {
        ui.label("127.0.0.1 :");
        ui.add(egui::DragValue::new(&mut port).range(1..=65535));
        if ui
            .add_enabled(!busy, egui::Button::new("Получить данные приёмника"))
            .clicked()
        {
            let request = Request(Arc::new(Mutex::new(None)));
            ui.ctx().data_mut(|d| d.insert_temp(id, request.clone()));
            let ctx = ui.ctx().clone();
            std::thread::spawn(move || {
                let result = fetch(port);
                if let Ok(mut slot) = request.0.lock() {
                    *slot = Some(result);
                }
                ctx.request_repaint();
            });
        }
    });
    ui.ctx().data_mut(|d| d.insert_temp(port_id, port));
    ui.collapsing("Импорт aircraft.json", |ui| {
        let input_id = egui::Id::new("adsb_input");
        let mut input = ui
            .ctx()
            .data(|d| d.get_temp::<String>(input_id))
            .unwrap_or_default();
        ui.add(
            egui::TextEdit::multiline(&mut input)
                .desired_rows(3)
                .char_limit(2_000_000),
        );
        if ui.button("Прочитать JSON").clicked() {
            let result = parse(input.as_bytes());
            ui.ctx()
                .data_mut(|d| d.insert_temp(id, Request(Arc::new(Mutex::new(Some(result))))));
        }
        ui.ctx().data_mut(|d| d.insert_temp(input_id, input));
    });
    let Some(request) = ui.ctx().data(|d| d.get_temp::<Request>(id)) else {
        return;
    };
    let Ok(slot) = request.0.lock() else {
        return;
    };
    match slot.as_ref() {
        None => {
            ui.label("Ожидаем приёмник…");
        }
        Some(Err(e)) => {
            ui.label(e);
        }
        Some(Ok(feed)) => {
            let now = chrono::Utc::now().timestamp() as f64;
            let age = now - feed.now;
            ui.label(format!(
                "{} объектов · время источника: {:.0} UNIX",
                feed.aircraft.len(),
                feed.now
            ));
            if !(-30.0..=120.0).contains(&age) {
                ui.colored_label(
                    egui::Color32::YELLOW,
                    "Снимок устарел или часы приёмника отличаются. Это не текущая ситуация.",
                );
            }
            egui::Grid::new("adsb_rows").striped(true).show(ui, |ui| {
                for label in ["ICAO", "Позывной", "Координаты", "Возраст, с"]
                {
                    ui.strong(label);
                }
                ui.end_row();
                for a in feed.aircraft.iter().take(200) {
                    ui.label(&a.hex);
                    ui.label(a.flight.as_deref().unwrap_or("—"));
                    ui.label(match a.lat.zip(a.lon) {
                        Some((lat, lon)) => format!("{lat:.4}, {lon:.4}"),
                        None => "Позиция недоступна".into(),
                    });
                    ui.label(format!("{:.1}", a.seen));
                    ui.end_row();
                }
            });
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_stale_targets_and_does_not_invent_missing_positions() {
        let feed=parse(br#"{"now":1700000000,"aircraft":[{"hex":"abcdef","flight":"FLIGHT","seen":1,"lat":10,"lon":20,"seen_pos":50},{"hex":"123456","seen":40},{"hex":"bad","seen":1}]}"#).unwrap();
        assert_eq!(feed.aircraft.len(), 1);
        assert!(feed.aircraft[0].lat.is_none());
        assert!(parse(br#"{"now":0,"aircraft":[]}"#).is_err());
    }
}
