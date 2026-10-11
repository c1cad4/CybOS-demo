//! Opt-in adapter to the user's own Prowlarr service; no bundled trackers.
use crate::movie_catalog::Movie;
use eframe::egui;
use serde::Deserialize;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Release {
    pub title: Option<String>,
    pub magnet_url: Option<String>,
    pub download_url: Option<String>,
    pub seeders: Option<u32>,
    pub size: Option<u64>,
    #[serde(default)]
    pub imdb_id: u64,
    #[serde(default)]
    pub tmdb_id: u64,
    pub indexer: Option<String>,
    pub protocol: Option<String>,
}
fn normalized(s: &str) -> String {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
fn quality(title: &str) -> u16 {
    let words = normalized(title);
    if words
        .split_whitespace()
        .any(|w| matches!(w, "2160p" | "4k"))
    {
        2160
    } else if words.split_whitespace().any(|w| w == "1080p") {
        1080
    } else if words.split_whitespace().any(|w| w == "720p") {
        720
    } else {
        0
    }
}
/// IDs take precedence; conflicting IDs are rejected even if titles look identical.
fn confidence(movie: &Movie, imdb: u64, release: &Release) -> Option<u8> {
    if release.protocol.as_deref().is_some_and(|p| p != "torrent") {
        return None;
    }
    if imdb > 0 && release.imdb_id > 0 && imdb != release.imdb_id {
        return None;
    }
    if !movie.tv && release.tmdb_id > 0 && release.tmdb_id != movie.id {
        return None;
    }
    if imdb > 0 && release.imdb_id == imdb {
        return Some(3);
    }
    if !movie.tv && release.tmdb_id == movie.id {
        return Some(3);
    }
    let title = normalized(release.title.as_deref()?);
    let original = normalized(if movie.original_title.is_empty() {
        &movie.title
    } else {
        &movie.original_title
    });
    let local = normalized(&movie.title);
    let year = movie.release_date.get(..4).unwrap_or("");
    let same_title = [&original, &local]
        .iter()
        .any(|name| !name.is_empty() && format!(" {title} ").contains(&format!(" {name} ")));
    let same_year = year.len() == 4 && title.split_whitespace().any(|w| w == year);
    if same_title && (same_year || movie.tv) {
        Some(1)
    } else {
        None
    }
}
fn rank(movie: &Movie, imdb: u64, releases: Vec<Release>, preferred: u16) -> Vec<(Release, u8)> {
    let mut ranked: Vec<_> = releases
        .into_iter()
        .filter_map(|release| {
            let score = confidence(movie, imdb, &release)?;
            let title = normalized(release.title.as_deref().unwrap_or(""));
            if title
                .split_whitespace()
                .any(|w| matches!(w, "cam" | "camrip" | "hdcam" | "telesync" | "ts"))
            {
                return None;
            }
            Some((release, score))
        })
        .collect();
    ranked.sort_by_key(|(r, c)| {
        std::cmp::Reverse((
            *c,
            u8::from(quality(r.title.as_deref().unwrap_or("")) == preferred),
            r.seeders.unwrap_or(0),
        ))
    });
    ranked
}
#[derive(Default)]
struct Search {
    results: Vec<(Release, u8)>,
    status: String,
    busy: bool,
}
#[derive(Clone)]
struct SearchView {
    movie: Movie,
    port: u16,
    key: String,
    preferred: u16,
    state: Arc<Mutex<Search>>,
}

pub(crate) fn select(ctx: &egui::Context, movie: Movie) {
    let id = egui::Id::new("cyblex_source_search");
    let previous = ctx.data(|d| d.get_temp::<SearchView>(id));
    let credentials = ctx
        .data(|d| d.get_temp::<(u16, String)>(egui::Id::new("prowlarr_session_credentials")))
        .unwrap_or((9696, String::new()));
    let view = SearchView {
        movie,
        port: previous.as_ref().map_or(credentials.0, |v| v.port),
        key: previous
            .as_ref()
            .map_or_else(|| credentials.1.clone(), |v| v.key.clone()),
        preferred: 1080,
        state: Arc::new(Mutex::new(Search::default())),
    };
    ctx.data_mut(|d| d.insert_temp(id, view));
}
fn search(
    movie: &Movie,
    token: &str,
    port: u16,
    key: &str,
    preferred: u16,
) -> Result<Vec<(Release, u8)>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
        .into();
    let kind = if movie.tv { "tv" } else { "movie" };
    let imdb = if token.is_empty() {
        0
    } else {
        agent
            .get(format!(
                "https://api.themoviedb.org/3/{kind}/{}/external_ids",
                movie.id
            ))
            .header("Authorization", &format!("Bearer {token}"))
            .call()
            .ok()
            .and_then(|mut r| {
                r.body_mut()
                    .with_config()
                    .limit(128 * 1024)
                    .read_json::<serde_json::Value>()
                    .ok()
            })
            .and_then(|v| {
                v.get("imdb_id")?
                    .as_str()?
                    .strip_prefix("tt")?
                    .parse::<u64>()
                    .ok()
            })
            .unwrap_or(0)
    };
    let query = format!(
        "{} {}",
        if movie.original_title.is_empty() {
            &movie.title
        } else {
            &movie.original_title
        },
        if movie.tv {
            ""
        } else {
            movie.release_date.get(..4).unwrap_or("")
        }
    );
    let mut reply = agent
        .get(format!("http://127.0.0.1:{port}/api/v1/search"))
        .header("X-Api-Key", key)
        .query("query", &query)
        .query("type", "search")
        .query("limit", "100")
        .call()
        .map_err(|_| {
            "Prowlarr недоступен. Проверь локальный сервис, порт и API-ключ".to_string()
        })?;
    let releases: Vec<Release> = reply
        .body_mut()
        .with_config()
        .limit(2 * 1024 * 1024)
        .read_json()
        .map_err(|_| "Некорректный ответ Prowlarr")?;
    Ok(rank(
        movie,
        imdb,
        releases.into_iter().take(100).collect(),
        preferred,
    ))
}
pub(crate) fn show(ui: &mut egui::Ui, token: &str) -> Option<String> {
    let id = egui::Id::new("cyblex_source_search");
    let Some(mut view) = ui.ctx().data(|d| d.get_temp::<SearchView>(id)) else {
        return None;
    };
    let mut selected = None;
    let mut closed = false;
    egui::Window::new(format!("Источники · {}", view.movie.title)).default_width(640.0).collapsible(false).show(ui.ctx(), |ui| {
        ui.label("Поиск в твоём локальном Prowlarr. Сервисы и трекеры подключаются отдельно; загрузки запускаешь ты.");
        ui.collapsing("Подключить Prowlarr", |ui| {
            ui.horizontal(|ui| { ui.label("Порт на этом компьютере"); ui.add(egui::DragValue::new(&mut view.port).range(1024..=65535)); });
            ui.add(egui::TextEdit::singleline(&mut view.key).password(true).hint_text("API-ключ Prowlarr: хранится только в памяти"));
            ui.hyperlink_to("Инструкция Prowlarr", "https://prowlarr.com/docs/");
        });
        ui.horizontal(|ui| {
            ui.label("Предпочтительное качество");
            ui.selectable_value(&mut view.preferred, 1080, "1080p");
            ui.selectable_value(&mut view.preferred, 2160, "4K");
        });
        let busy = view.state.lock().unwrap().busy;
        if ui.add_enabled(!busy && !view.key.trim().is_empty(), egui::Button::new(if busy { "Ищем…" } else { "Найти источники" })).clicked() {
            view.state.lock().unwrap().busy = true;
            let copy = view.clone(); let token = token.to_owned(); let ctx = ui.ctx().clone();
            std::thread::spawn(move || {
                let result = search(&copy.movie, &token, copy.port, copy.key.trim(), copy.preferred);
                let mut state = copy.state.lock().unwrap();
                match result { Ok(r) => { state.results = r; state.status = format!("Найдено подходящих источников: {}", state.results.len()); }, Err(e) => state.status = e }
                state.busy = false; ctx.request_repaint();
            });
        }
        let state = view.state.lock().unwrap();
        ui.label(&state.status);
        egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
            for (release, confidence) in &state.results {
                ui.separator();
                ui.strong(release.title.as_deref().unwrap_or("Без названия"));
                ui.small(format!("{} · сиды: {} · {:.1} ГБ · {}", release.indexer.as_deref().unwrap_or("Индексатор"), release.seeders.map(|s| s.to_string()).unwrap_or_else(|| "неизвестно".into()), release.size.unwrap_or(0) as f64 / 1e9, if *confidence == 3 { "Совпадает ID" } else { "Совпадает название — проверь год и выпуск" }));
                let magnet = release.magnet_url.as_deref().or(release.download_url.as_deref().filter(|s| s.starts_with("magnet:?"))).filter(|s| s.starts_with("magnet:?"));
                if ui.add_enabled(magnet.is_some(), egui::Button::new("Выбрать источник")).clicked() { selected = magnet.map(str::to_owned); }
                if magnet.is_none() { ui.small("Нет magnet-ссылки: добавь .torrent вручную через свою библиотеку"); }
            }
        });
        ui.small("Сиды и описание сообщает индексатор; они не подтверждают содержимое. Выбор источника ещё не запускает загрузку.");
        if ui.button("Закрыть").clicked() { closed = true; }
    });
    ui.ctx().data_mut(|d| {
        d.insert_temp(
            egui::Id::new("prowlarr_session_credentials"),
            (view.port, view.key.clone()),
        )
    });
    if closed {
        ui.ctx().data_mut(|d| d.remove::<SearchView>(id));
    } else {
        ui.ctx().data_mut(|d| d.insert_temp(id, view));
    }
    selected
}
#[cfg(test)]
mod tests {
    use super::*;
    fn movie() -> Movie {
        serde_json::from_str(r#"{"id":42,"title":"Фильм","original_title":"The Film","release_date":"2024-01-01","poster_path":null}"#).unwrap()
    }
    #[test]
    fn ranking_rejects_wrong_ids_and_cam_and_prioritizes_requested_quality() {
        let items = serde_json::from_str(
            r#"[
            {"title":"The Film 2024 4K","seeders":80,"tmdbId":42},
            {"title":"The Film 2024 1080p","seeders":5,"tmdbId":42},
            {"title":"The Film 2024 1080p","seeders":100,"tmdbId":99},
            {"title":"The Film 2024 CAM","seeders":900},
            {"title":"The Film 2023 1080p","seeders":200}
        ]"#,
        )
        .unwrap();
        let result = rank(&movie(), 0, items, 1080);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0.seeders, Some(5));
        let wrong = Release {
            imdb_id: 999,
            tmdb_id: 42,
            ..Default::default()
        };
        assert!(confidence(&movie(), 123, &wrong).is_none());
    }
}
