//! TMDB metadata only. No media downloads are triggered by the catalog.
use crate::store::Store;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct Movie {
    pub id: u64,
    #[serde(alias = "name")]
    pub title: String,
    #[serde(default, alias = "original_name")]
    pub original_title: String,
    #[serde(default)]
    pub tv: bool,
    #[serde(default)]
    pub genre_ids: Vec<u64>,
    #[serde(default)]
    pub overview: String,
    #[serde(default)]
    #[serde(alias = "first_air_date")]
    pub release_date: String,
    #[serde(default)]
    pub vote_average: f64,
    pub poster_path: Option<String>,
    #[serde(skip)]
    pub poster: Option<egui::ColorImage>,
}
#[derive(Default)]
struct Catalog {
    movies: Vec<Movie>,
    status: String,
    busy: bool,
    updated: Option<Instant>,
    dirty: bool,
    loaded: bool,
    genres: std::collections::BTreeMap<u64, String>,
}
#[derive(Deserialize)]
struct Response {
    results: Vec<Movie>,
}

fn fetch(
    token: &str,
    shelf: usize,
) -> Result<(Vec<Movie>, std::collections::BTreeMap<u64, String>), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(12)))
        .build()
        .into();
    let mut response = agent
        .get(format!(
            "https://api.themoviedb.org/3/{}?language=ru-RU&page=1",
            match shelf {
                1 => "movie/popular",
                2 => "tv/popular",
                _ => "movie/now_playing",
            }
        ))
        .header("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|_| "Каталог недоступен. Проверь токен TMDB и подключение.".to_string())?;
    let data: Response = response
        .body_mut()
        .with_config()
        .limit(2 * 1024 * 1024)
        .read_json()
        .map_err(|_| "Не удалось прочитать каталог TMDB".to_string())?;
    let kind = if shelf == 2 { "tv" } else { "movie" };
    let genres = agent
        .get(format!(
            "https://api.themoviedb.org/3/genre/{kind}/list?language=ru-RU"
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
        .and_then(|v| v.get("genres").and_then(|v| v.as_array()).cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| Some((v.get("id")?.as_u64()?, v.get("name")?.as_str()?.to_owned())))
        .collect();
    Ok((
        data.results
            .into_iter()
            .take(20)
            .map(|mut m| {
                m.tv = shelf == 2;
                m
            })
            .collect(),
        genres,
    ))
}

fn poster(movie: &Movie, folder: &std::path::Path, network: bool) -> Option<egui::ColorImage> {
    let path = movie.poster_path.as_ref()?;
    if !path.starts_with('/')
        || !path
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "/._-".contains(c))
    {
        return None;
    }
    use sha2::{Digest, Sha256};
    let filename = format!("{:x}.img", Sha256::digest(path.as_bytes()));
    let cache_file = folder.join(filename);
    let bytes = if cache_file.metadata().is_ok_and(|m| m.len() <= 512 * 1024) {
        std::fs::read(&cache_file).ok()?
    } else {
        if !network {
            return None;
        }
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(3)))
            .build()
            .into();
        let mut response = agent
            .get(format!("https://image.tmdb.org/t/p/w185{path}"))
            .call()
            .ok()?;
        let bytes = response
            .body_mut()
            .with_config()
            .limit(512 * 1024)
            .read_to_vec()
            .ok()?;
        if std::fs::create_dir_all(folder).is_ok() {
            let temp = cache_file.with_extension("tmp");
            if std::fs::write(&temp, &bytes).is_ok() {
                let _ = std::fs::rename(&temp, &cache_file);
            }
            if let Ok(entries) = std::fs::read_dir(folder) {
                let mut cached: Vec<_> = entries
                    .flatten()
                    .filter_map(|e| {
                        let p = e.path();
                        if p.extension()?.to_str()? != "img" {
                            return None;
                        }
                        Some((e.metadata().ok()?.modified().ok()?, p))
                    })
                    .collect();
                cached.sort_by_key(|(time, _)| *time);
                let excess = cached.len().saturating_sub(120);
                for (_, p) in cached.into_iter().take(excess) {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
        bytes
    };
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(512);
    limits.max_image_height = Some(768);
    limits.max_alloc = Some(8 * 1024 * 1024);
    reader.limits(limits);
    let p = reader.decode().ok()?.into_rgba8();
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [p.width() as usize, p.height() as usize],
        &p,
    ))
}

#[derive(Serialize, Deserialize)]
struct Cache {
    saved: i64,
    movies: Vec<Movie>,
    #[serde(default)]
    genres: std::collections::BTreeMap<u64, String>,
}

pub(crate) fn show(ui: &mut egui::Ui, store: &Store) -> Option<String> {
    let shelf_id = egui::Id::new("tmdb_catalog_shelf");
    let old_shelf = ui
        .ctx()
        .data(|d| d.get_temp::<usize>(shelf_id))
        .unwrap_or(0);
    let mut shelf = old_shelf;
    ui.horizontal(|ui| {
        for (id, name) in ["Новинки", "Популярные фильмы", "Сериалы", "Избранное"]
            .iter()
            .enumerate()
        {
            ui.selectable_value(&mut shelf, id, *name);
        }
    });
    ui.ctx().data_mut(|d| d.insert_temp(shelf_id, shelf));
    let cache_key = format!("tmdb_catalog_cache:{shelf}");
    let poster_folder = store
        .path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("tmdb-posters");
    let state_id = egui::Id::new(("tmdb_catalog_state", shelf));
    let state = ui.ctx().data_mut(|d| {
        if let Some(s) = d.get_temp::<Arc<Mutex<Catalog>>>(state_id) {
            s
        } else {
            let s = Arc::new(Mutex::new(Catalog::default()));
            d.insert_temp(state_id, s.clone());
            s
        }
    });
    let token_id = egui::Id::new("tmdb_session_token");
    let mut token = ui
        .ctx()
        .data_mut(|d| d.get_temp::<String>(token_id))
        .unwrap_or_else(|| std::env::var("CYBOS_TMDB_TOKEN").unwrap_or_default());
    ui.heading(match shelf {
        1 => "Популярные фильмы",
        2 => "Сериалы",
        3 => "Избранное",
        _ => "Сейчас в кино",
    });
    ui.hyperlink_to(
        "Открыть новинки на TMDB",
        "https://www.themoviedb.org/movie/now-playing",
    );
    ui.label("Новинки обновляются автоматически. Фильмы загружаешь ты — по своей torrent-ссылке.");
    ui.collapsing("Подключить каталог TMDB", |ui| {
        ui.label("Вставь API Read Access Token из настроек TMDB. Он хранится только до закрытия приложения.");
        ui.add(egui::TextEdit::singleline(&mut token).password(true).hint_text("Токен TMDB"));
        ui.hyperlink_to("Получить токен", "https://www.themoviedb.org/settings/api");
    });
    ui.ctx()
        .data_mut(|d| d.insert_temp(token_id, token.clone()));
    let refresh = ui.button("Обновить новинки").clicked();
    let mut catalog = state.lock().unwrap();
    if shelf == 3 && old_shelf != shelf {
        catalog.loaded = false;
    }
    if !catalog.loaded {
        catalog.loaded = true;
        let raw = if shelf == 3 {
            let mut movies = Vec::new();
            let mut genres = std::collections::BTreeMap::new();
            for index in 0..3 {
                if let Some(cache) = store
                    .get(&format!("tmdb_catalog_cache:{index}"))
                    .and_then(|raw| serde_json::from_str::<Cache>(&raw).ok())
                {
                    genres.extend(cache.genres);
                    for movie in cache.movies {
                        let key = format!(
                            "tmdb_favorite:{}:{}",
                            if movie.tv { "tv" } else { "movie" },
                            movie.id
                        );
                        if store.get(&key).as_deref() == Some("1")
                            && !movies
                                .iter()
                                .any(|m: &Movie| m.id == movie.id && m.tv == movie.tv)
                        {
                            movies.push(movie);
                        }
                    }
                }
            }
            serde_json::to_string(&Cache {
                saved: chrono::Utc::now().timestamp(),
                movies,
                genres,
            })
            .ok()
        } else {
            store.get(&cache_key)
        };
        if let Some(raw) = raw {
            if let Ok(cache) = serde_json::from_str::<Cache>(&raw) {
                let age = chrono::Utc::now()
                    .timestamp()
                    .saturating_sub(cache.saved)
                    .max(0) as u64;
                catalog.movies = cache.movies.into_iter().take(60).collect();
                catalog.genres = cache.genres;
                let state = state.clone();
                let folder = poster_folder.clone();
                let movies = catalog.movies.clone();
                let ctx = ui.ctx().clone();
                std::thread::spawn(move || {
                    for movie in movies {
                        if let Some(p) = poster(&movie, &folder, false) {
                            if let Ok(mut c) = state.lock() {
                                if let Some(m) = c.movies.iter_mut().find(|m| m.id == movie.id) {
                                    m.poster = Some(p);
                                }
                            }
                            ctx.request_repaint();
                        }
                    }
                });
                catalog.status = "Показан сохранённый каталог".into();
                if age < 6 * 3600 {
                    catalog.updated = Instant::now().checked_sub(Duration::from_secs(age));
                }
            }
        }
    }
    if catalog.dirty {
        let cache = Cache {
            saved: chrono::Utc::now().timestamp(),
            movies: catalog.movies.clone(),
            genres: catalog.genres.clone(),
        };
        match serde_json::to_string(&cache)
            .map_err(|e| e.to_string())
            .and_then(|raw| store.set_checked(&cache_key, &raw))
        {
            Ok(()) => catalog.dirty = false,
            Err(e) => {
                catalog.dirty = false;
                catalog.status = format!("Каталог загружен, но не сохранён: {e}");
            }
        }
    }
    let due = catalog
        .updated
        .is_none_or(|t| t.elapsed() >= Duration::from_secs(6 * 3600));
    if shelf < 3 && !catalog.busy && !token.trim().is_empty() && (refresh || due) {
        catalog.busy = true;
        catalog.updated = Some(Instant::now()); // Also back off after failures.
        catalog.status = "Загружаем новинки и обложки…".into();
        let state = state.clone();
        let ctx = ui.ctx().clone();
        let folder = poster_folder.clone();
        std::thread::spawn(move || {
            match fetch(token.trim(), shelf) {
                Ok((movies, genres)) => {
                    {
                        let mut c = state.lock().unwrap();
                        c.movies = movies.clone();
                        c.dirty = true;
                        c.genres = genres;
                        c.status = "Загружаем обложки…".into();
                    }
                    ctx.request_repaint();
                    for movie in movies {
                        if let Some(p) = poster(&movie, &folder, true) {
                            let mut c = state.lock().unwrap();
                            if let Some(m) = c.movies.iter_mut().find(|m| m.id == movie.id) {
                                m.poster = Some(p);
                            }
                            ctx.request_repaint();
                        }
                    }
                    let mut c = state.lock().unwrap();
                    c.busy = false;
                    c.status = "Каталог обновлён".into();
                }
                Err(e) => {
                    let mut c = state.lock().unwrap();
                    c.busy = false;
                    c.status = e;
                }
            }
            ctx.request_repaint();
        });
    }
    if catalog.movies.is_empty() && !catalog.busy && catalog.status.is_empty() {
        ui.label("Подключи TMDB, чтобы видеть настоящие новинки и обложки. Твоя библиотека работает без него.");
    }
    ui.small(&catalog.status);
    if shelf == 3 && catalog.movies.is_empty() {
        ui.label("Нажми сердечко на карточке, чтобы сохранить фильм или сериал.");
    }
    let cols = ((ui.available_width() / 170.0) as usize).clamp(1, 6);
    let width = ((ui.available_width() - 12.0 * (cols - 1) as f32) / cols as f32).min(200.0);
    for row in catalog.movies.chunks(cols) {
        ui.horizontal_top(|ui| {
            for movie in row {
                ui.vertical(|ui| {
                    ui.set_width(width);
                    if let Some(poster) = &movie.poster {
                        let key = egui::Id::new(("tmdb_poster", movie.tv, movie.id));
                        let texture = ui
                            .ctx()
                            .data_mut(|d| d.get_temp::<egui::TextureHandle>(key))
                            .unwrap_or_else(|| {
                                let t = ui.ctx().load_texture(
                                    format!("movie-{}", movie.id),
                                    poster.clone(),
                                    egui::TextureOptions::LINEAR,
                                );
                                ui.ctx().data_mut(|d| d.insert_temp(key, t.clone()));
                                t
                            });
                        ui.image((texture.id(), egui::vec2(width, width * 1.5)));
                    } else {
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(width, width * 1.5),
                            egui::Sense::hover(),
                        );
                        ui.painter()
                            .rect_filled(rect, 12.0, egui::Color32::from_rgb(10, 35, 27));
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "CYBLEX",
                            egui::FontId::proportional(18.0),
                            egui::Color32::LIGHT_GREEN,
                        );
                    }
                    ui.small(
                        movie
                            .genre_ids
                            .iter()
                            .filter_map(|id| catalog.genres.get(id).cloned())
                            .collect::<Vec<_>>()
                            .join(" · "),
                    );
                    ui.label(egui::RichText::new(&movie.title).strong())
                        .on_hover_text(&movie.overview);
                    ui.small(format!(
                        "{} · {}",
                        movie.release_date,
                        if movie.vote_average > 0.0 {
                            format!("★ {:.1}", movie.vote_average)
                        } else {
                            "Пока нет оценки".into()
                        }
                    ));
                    if ui.button("Найти источники").clicked() {
                        crate::torrent_indexer::select(ui.ctx(), movie.clone());
                    }
                    let fav_key = format!(
                        "tmdb_favorite:{}:{}",
                        if movie.tv { "tv" } else { "movie" },
                        movie.id
                    );
                    let favorite = store.get(&fav_key).as_deref() == Some("1");
                    if ui
                        .button(if favorite {
                            "♥ В избранном"
                        } else {
                            "♡ В избранное"
                        })
                        .clicked()
                    {
                        let _ = store.set_checked(&fav_key, if favorite { "0" } else { "1" });
                    }
                    ui.hyperlink_to(
                        "О фильме",
                        format!(
                            "https://www.themoviedb.org/{}/{}",
                            if movie.tv { "tv" } else { "movie" },
                            movie.id
                        ),
                    );
                });
            }
        });
        ui.add_space(12.0);
    }
    drop(catalog);
    let selected = crate::torrent_indexer::show(
        ui,
        &ui.ctx()
            .data(|d| d.get_temp::<String>(token_id))
            .unwrap_or_default(),
    );
    ui.small(
        "Каталог: TMDB. This product uses the TMDB API but is not endorsed or certified by TMDB.",
    );
    selected
}

#[cfg(test)]
mod tests {
    #[test]
    fn incomplete_metadata_is_accepted_without_fake_ratings() {
        let data: super::Response =
            serde_json::from_str(r#"{"results":[{"id":1,"title":"Film","poster_path":null}]}"#)
                .unwrap();
        assert_eq!(data.results[0].title, "Film");
        assert!(data.results[0].poster.is_none());
        assert_eq!(data.results[0].release_date, "");
    }
}
