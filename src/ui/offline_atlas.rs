//! First-stage offline atlas tooling: inspect local MBTiles archives without network access.
//! This deliberately does not claim map rendering, GPS fixes, or route planning.
use crate::CybOs;
use eframe::egui::{self, Color32, RichText, Stroke};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
struct MbtilesInfo {
    name: String,
    format: String,
    min_zoom: String,
    max_zoom: String,
    bounds: String,
    center: String,
    tile_count: i64,
    bytes: u64,
}

fn inspect_mbtiles(path: &Path) -> Result<MbtilesInfo, String> {
    if !path.is_file() {
        return Err("DATA MISSING · file path does not point to a file".into());
    }
    let bytes = std::fs::metadata(path)
        .map_err(|e| format!("ERROR · cannot read file metadata: {e}"))?
        .len();
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("ERROR · not a readable SQLite/MBTiles archive: {e}"))?;

    let mut metadata = std::collections::BTreeMap::<String, String>::new();
    {
        let mut stmt = conn.prepare("SELECT name, value FROM metadata")
            .map_err(|e| format!("ERROR · MBTiles metadata table unavailable: {e}"))?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| format!("ERROR · cannot read MBTiles metadata: {e}"))?;
        for row in rows {
            let (key, value) = row.map_err(|e| format!("ERROR · invalid metadata row: {e}"))?;
            metadata.insert(key, value);
        }
    }
    let tile_count: i64 = conn.query_row("SELECT COUNT(*) FROM tiles", [], |row| row.get(0))
        .map_err(|e| format!("ERROR · MBTiles tiles table unavailable: {e}"))?;
    if tile_count <= 0 {
        return Err("DATA MISSING · archive contains no map tiles".into());
    }

    Ok(MbtilesInfo {
        name: metadata.get("name").cloned().unwrap_or_else(|| path.file_name().unwrap_or_default().to_string_lossy().into_owned()),
        format: metadata.get("format").cloned().unwrap_or_else(|| "unknown".into()),
        min_zoom: metadata.get("minzoom").cloned().unwrap_or_else(|| "unknown".into()),
        max_zoom: metadata.get("maxzoom").cloned().unwrap_or_else(|| "unknown".into()),
        bounds: metadata.get("bounds").cloned().unwrap_or_else(|| "not declared".into()),
        center: metadata.get("center").cloned().unwrap_or_else(|| "not declared".into()),
        tile_count,
        bytes,
    })
}

fn format_bytes(bytes: u64) -> String {
    const MIB: u64 = 1024 * 1024;
    if bytes >= MIB {
        format!("{:.2} MiB", bytes as f64 / MIB as f64)
    } else {
        format!("{bytes} bytes")
    }
}


fn tile_center(info: &MbtilesInfo) -> Option<(u8, i64, i64)> {
    let center: Vec<f64> = info.center.split(',').filter_map(|part| part.trim().parse().ok()).collect();
    let (lon, lat, center_zoom) = if center.len() >= 2 {
        (center[0], center[1], center.get(2).copied())
    } else {
        let bounds: Vec<f64> = info.bounds.split(',').filter_map(|part| part.trim().parse().ok()).collect();
        if bounds.len() != 4 { return None; }
        ((bounds[0] + bounds[2]) / 2.0, (bounds[1] + bounds[3]) / 2.0, None)
    };
    if !lon.is_finite() || !lat.is_finite() || !(-180.0..=180.0).contains(&lon) { return None; }
    let zoom = center_zoom
        .map(|z| z.round() as i64)
        .or_else(|| info.max_zoom.parse::<i64>().ok())
        .unwrap_or(4)
        .clamp(0, 14) as u8;
    let n = 1_i64 << zoom;
    let x = (((lon + 180.0) / 360.0 * n as f64).floor() as i64).clamp(0, n - 1);
    let safe_lat = lat.clamp(-85.05112878, 85.05112878).to_radians();
    let y = (((1.0 - (safe_lat.tan().asinh() / std::f64::consts::PI)) / 2.0 * n as f64).floor() as i64).clamp(0, n - 1);
    Some((zoom, x, y))
}

fn read_tile(path: &Path, zoom: u8, x: i64, slippy_y: i64) -> Result<Option<Vec<u8>>, String> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("cannot open tile archive: {e}"))?;
    let n = 1_i64 << zoom;
    let x = x.rem_euclid(n);
    let tms_y = n - 1 - slippy_y;
    match conn.query_row(
        "SELECT tile_data FROM tiles WHERE zoom_level=?1 AND tile_column=?2 AND tile_row=?3",
        rusqlite::params![i64::from(zoom), x, tms_y],
        |row| row.get::<_, Vec<u8>>(0),
    ) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(format!("cannot read map tile: {error}")),
    }
}

impl CybOs {
    pub(crate) fn offline_atlas(&mut self, ui: &mut egui::Ui) {
        let neon = Self::neon();
        let dim = Color32::from_rgb(135, 180, 150);
        let panel = Color32::from_rgb(4, 18, 12);
        let edge = Color32::from_rgb(24, 90, 56);

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(crate::language::tr(self.language, "offline_atlas")).size(19.0).strong().color(neon));
            ui.label(RichText::new("LOCAL MAP ARCHIVES · NETWORK NOT REQUIRED FOR INSPECTION").size(9.0).color(dim));
        });
        ui.add_space(6.0);
        ui.label(RichText::new("Stage 1: locate and validate a local .mbtiles archive. Keep maps on disk before travelling; this page never downloads tiles automatically.").size(11.0).color(Color32::LIGHT_GRAY));
        ui.add_space(12.0);

        egui::Frame::new().fill(panel).stroke(Stroke::new(1.0, edge))
            .corner_radius(10).inner_margin(egui::Margin::same(14)).show(ui, |ui| {
                ui.label(RichText::new("MBTILES ARCHIVE PATH").size(10.0).strong().color(neon));
                let mut path = self.store.get("offline_atlas_path").unwrap_or_default();
                let response = ui.add_sized([ui.available_width(), 34.0], egui::TextEdit::singleline(&mut path)
                    .hint_text("/path/to/region.mbtiles"));
                if response.changed() {
                    self.store.set("offline_atlas_path", &path);
                    self.store.set("offline_atlas_status", "NOT CHECKED");
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.button("INSPECT LOCAL ARCHIVE").clicked() {
                        let result = if path.trim().is_empty() {
                            Err("DATA MISSING · enter a local archive path first".to_string())
                        } else {
                            inspect_mbtiles(Path::new(path.trim()))
                        };
                        match result {
                            Ok(info) => {
                                let report = format!(
                                    "ARCHIVE-VALIDATED · READABLE MBTILES\nName: {}\nFormat: {}\nTiles: {}\nZoom: {}–{}\nBounds (W,S,E,N): {}\nCenter (lon,lat,zoom): {}\nFile size: {}\n\nValidation means the local SQLite/MBTiles tables and tile count are readable. It does not certify map licensing, visual rendering, GPS, or routing.",
                                    info.name, info.format, info.tile_count, info.min_zoom, info.max_zoom,
                                    info.bounds, info.center, format_bytes(info.bytes)
                                );
                                ui.ctx().data_mut(|data| {
                                    data.insert_temp(
                                        egui::Id::new(("offline-atlas-info", path.trim().to_owned())),
                                        info.clone(),
                                    );
                                });
                                self.store.set("offline_atlas_status", &report);
                                self.notify("LOCAL MAP ARCHIVE INSPECTED");
                            }
                            Err(error) => {
                                self.store.set("offline_atlas_status", &error);
                                self.notify("MAP ARCHIVE NOT READY");
                            }
                        }
                    }
                    if ui.button("CLEAR PATH").clicked() {
                        self.store.set("offline_atlas_path", "");
                        self.store.set("offline_atlas_status", "NOT CHECKED");
                    }
                });
                ui.add_space(8.0);
                let status = self.store.get("offline_atlas_status").unwrap_or_else(|| "NOT CHECKED".into());
                ui.label(RichText::new(&status).size(10.0).color(if status.starts_with("ARCHIVE-VALIDATED") { neon } else { Color32::LIGHT_GRAY }));

                if status.starts_with("ARCHIVE-VALIDATED") {
                    let info = ui.ctx().data(|data| {
                        data.get_temp::<MbtilesInfo>(egui::Id::new(("offline-atlas-info", path.trim().to_owned())))
                    });
                    if let Some(info) = info {
                        if let Some((zoom, center_x, center_y)) = tile_center(&info) {
                            ui.add_space(12.0);
                            ui.label(RichText::new(format!("LOCAL TILE PREVIEW · ZOOM {zoom} · NO NETWORK")).size(10.0).strong().color(neon));
                            ui.label(RichText::new("Preview uses only tiles already stored in this MBTiles file. Blank cells mean the archive has no tile at that coordinate or uses an unsupported image format.").size(9.0).color(dim));
                            ui.add_space(6.0);
                            egui::Frame::new().fill(Color32::from_rgb(1, 8, 5)).stroke(Stroke::new(1.0, edge))
                                .corner_radius(8).inner_margin(egui::Margin::same(5)).show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        for dx in -1_i64..=1 {
                                            ui.vertical(|ui| {
                                                for dy in -1_i64..=1 {
                                                    let x = center_x + dx;
                                                    let y = center_y + dy;
                                                    let tile_id = egui::Id::new(("offline-atlas-tile", path.trim().to_owned(), zoom, x, y));
                                                    let cached = ui.ctx().data(|data| data.get_temp::<Option<egui::TextureHandle>>(tile_id));
                                                    let texture = if let Some(cached) = cached {
                                                        cached
                                                    } else {
                                                        let loaded = read_tile(Path::new(path.trim()), zoom, x, y)
                                                            .ok().flatten()
                                                            .and_then(|bytes| image::load_from_memory(&bytes).ok())
                                                            .map(|decoded| {
                                                                let decoded = decoded.to_rgba8();
                                                                let size = [decoded.width() as usize, decoded.height() as usize];
                                                                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, decoded.as_raw());
                                                                ui.ctx().load_texture(
                                                                    format!("offline-atlas-{zoom}-{x}-{y}"),
                                                                    color_image,
                                                                    egui::TextureOptions::LINEAR,
                                                                )
                                                            });
                                                        ui.ctx().data_mut(|data| data.insert_temp(tile_id, loaded.clone()));
                                                        loaded
                                                    };
                                                    if let Some(texture) = texture {
                                                        ui.image((texture.id(), egui::vec2(192.0, 192.0)));
                                                    } else {
                                                        let (rect, _) = ui.allocate_exact_size(egui::vec2(192.0, 192.0), egui::Sense::hover());
                                                        ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(7, 23, 16));
                                                        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "NO TILE", egui::FontId::proportional(9.0), dim);
                                                    }
                                                }
                                            });
                                        }
                                    });
                                });
                        } else {
                            ui.label(RichText::new("MAP PREVIEW UNAVAILABLE · archive center/bounds metadata is missing or invalid").size(9.0).color(dim));
                        }
                    }
                }
            });

        ui.add_space(12.0);
        ui.columns(3, |columns| {
            for (index, (title, body)) in [
                ("MAP DATA", "MBTiles is a local tile container. It may contain raster maps or satellite imagery, depending on the archive."),
                ("GPS RECEIVER", "Live position needs a supported GNSS/GPS receiver and operating-system access. No location is fabricated here."),
                ("ROUTING", "Offline turn-by-turn routing needs a separate road/trail graph and routing engine. A tile archive alone cannot calculate routes."),
            ].iter().enumerate() {
                egui::Frame::new().fill(panel).stroke(Stroke::new(1.0, edge))
                    .corner_radius(9).inner_margin(egui::Margin::same(11)).show(&mut columns[index], |ui| {
                        ui.label(RichText::new(*title).size(10.0).strong().color(neon));
                        ui.add_space(5.0);
                        ui.label(RichText::new(*body).size(10.0).color(Color32::LIGHT_GRAY));
                    });
            }
        });
        ui.add_space(12.0);
        ui.label(RichText::new("NEXT: map tile rendering → region packs and storage estimates → GNSS status → offline route graph. Satellite imagery must retain provider attribution, capture date, and license terms.").size(10.0).color(dim));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_archive_is_reported_without_network_access() {
        let result = inspect_mbtiles(Path::new("/definitely-not-a-real-cybos-map.mbtiles"));
        assert!(result.unwrap_err().starts_with("DATA MISSING"));
    }

    #[test]
    fn byte_sizes_are_human_readable() {
        assert_eq!(format_bytes(512), "512 bytes");
        assert_eq!(format_bytes(2 * 1024 * 1024), "2.00 MiB");
    }
}
