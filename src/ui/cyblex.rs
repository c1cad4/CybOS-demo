use crate::CybOs;
use eframe::egui;
use egui::{Color32, RichText, Stroke};

impl CybOs {
    pub(crate) fn cyblex(&mut self, ui: &mut egui::Ui) {
        self.poll_cyblex();
        if let Some(source) = crate::movie_catalog::show(ui, &self.store) {
            self.cyblex_source = source;
            self.notify("Источник выбран. Открой «Добавить загрузку» и подтверди скачивание");
        }
        ui.separator();
        ui.heading("Моя библиотека");
        ui.label("Перетащи сюда свой фильм, музыку или папку, чтобы подготовить раздачу.");
        let dropped = ui.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()));
        if let Some(path) = dropped { self.cyblex_seed_path = path.to_string_lossy().into_owned(); }


        let neon = Self::neon();
        let dim = Color32::from_rgb(55, 145, 105);

        ui.collapsing("＋ Добавить загрузку или поделиться своими файлами", |ui| {
        egui::Frame::new()
            .fill(Color32::from_rgb(4, 16, 11))
            .stroke(Stroke::new(1.0, Color32::from_rgb(24, 100, 64)))
            .corner_radius(12)
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Ваша библиотека CybLex")
                        .size(14.0)
                        .strong()
                        .color(neon),
                );
                ui.label(
                    RichText::new(
                        "Загружайте музыку, фильмы и другие файлы. Делитесь файлами с другими участниками.",
                    )
                    .size(10.0)
                    .color(dim),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "Для загрузки нужна magnet-ссылка или ссылка на .torrent. Для раздачи — файл или папка.",
                    )
                    .size(9.0)
                    .strong()
                    .color(neon),
                );
            });

        ui.add_space(10.0);

        egui::Frame::new()
            .fill(Color32::from_rgb(5, 18, 13))
            .stroke(Stroke::new(1.0, Color32::from_rgb(22, 80, 52)))
            .corner_radius(10)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.label(RichText::new("Загрузить файл").size(12.0).strong().color(neon));
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.cyblex_source)
                            .desired_width((ui.available_width() - 150.0).max(80.0))
                            .hint_text("magnet:?… or https://…/file.torrent"),
                    );

                    if ui.button("Загрузить").clicked() {
                        match self.cyblex.add_source(
                            self.cyblex_source.clone(),
                            self.cyblex_download_path.clone(),
                        ) {
                            Ok(()) => self.notify("Загрузка добавлена в очередь"),
                            Err(error) => self.notify(format!("CYBLEX: {error}")),
                        }
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Папка загрузки").size(9.0).color(dim));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.cyblex_download_path)
                            .desired_width((ui.available_width() - 150.0).max(80.0)),
                    );

                    if ui.button("По умолчанию").clicked() {
                        self.cyblex_download_path = Self::cyblex_default_download_path();
                    }
                });
            });

        ui.add_space(10.0);

        egui::Frame::new()
            .fill(Color32::from_rgb(4, 16, 11))
            .stroke(Stroke::new(1.0, Color32::from_rgb(22, 80, 52)))
            .corner_radius(10)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Раздать свои файлы")
                        .size(12.0)
                        .strong()
                        .color(neon),
                );
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.cyblex_seed_path)
                            .desired_width((ui.available_width() - 150.0).max(80.0))
                            .hint_text("Путь к файлу или папке"),
                    );

                    if ui.button("Начать раздачу").clicked() {
                        match self.cyblex.seed_path(self.cyblex_seed_path.clone()) {
                            Ok(()) => self.notify("Раздача добавлена в очередь"),
                            Err(error) => self.notify(format!("CYBLEX: {error}")),
                        }
                    }
                });

                ui.label(
                    RichText::new(
                        "Создаёт .torrent рядом с файлом и magnet-ссылку, которой можно поделиться.",
                    )
                    .size(9.0)
                    .color(dim),
                );
            });

        ui.add_space(12.0);

        });

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(if self.cyblex_torrents.is_empty() { "Библиотека готова" } else { "Загрузки и раздачи активны" })
                    .size(10.0)
                    .strong()
                    .color(neon),
            );
            ui.label(
                RichText::new(format!("Торрентов: {}", self.cyblex_torrents.len()))
                    .size(9.0)
                    .color(dim),
            );
        });

        ui.horizontal(|ui| {
            if ui.button("Остановить просмотр").clicked() { let _ = self.cyblex.stop_playback(); }
            ui.add(egui::Label::new(&self.cyblex_status).wrap());
        });
        ui.add_space(6.0);

        if self.cyblex_torrents.is_empty() {
            ui.label(
                RichText::new(
                    "Загрузок пока нет. Добавьте torrent-ссылку или начните раздачу своих файлов.",
                )
                .size(10.0)
                .color(dim),
            );
        }

        ui.horizontal_wrapped(|ui| {
            for torrent in self.cyblex_torrents.clone() {
                egui::Frame::group(ui.style()).corner_radius(12).inner_margin(12).show(ui, |ui| {
                    ui.set_width(160.0);
                    ui.label(RichText::new("▶").size(36.0).color(neon));
                    ui.label(RichText::new(&torrent.name).strong());
                    ui.small(if torrent.finished { "Готово к просмотру" } else { "Загружается" });
                    for (file, name) in &torrent.videos {
                        if ui.button("▶ Смотреть сейчас").on_hover_text(name).clicked() {
                            if let Err(e) = self.cyblex.play(torrent.id, *file) { self.cyblex_status = e; }
                        }
                    }
                    if ui.add_enabled(torrent.finished,egui::Button::new("Открыть папку")).clicked() {
                        #[cfg(target_os="macos")]
                        let _ = std::process::Command::new("open").arg(&torrent.output_folder).spawn();
                        #[cfg(target_os="windows")]
                        let _ = std::process::Command::new("explorer").arg(&torrent.output_folder).spawn();
                        #[cfg(target_os="linux")]
                        let _ = std::process::Command::new("xdg-open").arg(&torrent.output_folder).spawn();
                    }
                });
            }
        });
        ui.collapsing("Загрузки и раздачи · подробности", |ui| {
        for torrent in self.cyblex_torrents.clone() {
            let progress = if torrent.total_bytes == 0 {
                0.0
            } else {
                (torrent.progress_bytes as f32 / torrent.total_bytes as f32).clamp(0.0, 1.0)
            };

            egui::Frame::new()
                .fill(Color32::from_rgb(3, 13, 9))
                .stroke(Stroke::new(1.0, Color32::from_rgb(18, 68, 44)))
                .corner_radius(9)
                .inner_margin(egui::Margin::same(11))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&torrent.name)
                                .size(12.0)
                                .strong()
                                .color(neon),
                        );
                        ui.label(
                            RichText::new(format!("ID {} · {}", torrent.id, torrent.state))
                                .size(9.0)
                                .color(dim),
                        );

                        if torrent.paused {
                            if ui.button("Продолжить").clicked() {
                                let _ = self.cyblex.resume(torrent.id);
                            }
                        } else if ui.button("Пауза").clicked() {
                            let _ = self.cyblex.pause(torrent.id);
                        }

                        if ui.button("Убрать из списка").clicked() {
                            let _ = self.cyblex.forget(torrent.id, false);
                        }
                    });

                    ui.add(
                        egui::ProgressBar::new(progress)
                            .text(format!(
                                "{:.1}% · {} / {} байт",
                                progress * 100.0,
                                torrent.progress_bytes,
                                torrent.total_bytes
                            ))
                            .desired_width(f32::INFINITY),
                    );

                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(format!("Отдано · {} байт", torrent.uploaded_bytes))
                                .size(9.0)
                                .color(dim),
                        );
                        ui.label(
                            RichText::new(format!("HASH {}", torrent.info_hash))
                                .size(8.0)
                                .color(dim),
                        );

                        if ui.button("Поделиться ссылкой").clicked() {
                            ui.ctx().copy_text(torrent.magnet_uri.clone());
                            self.notify("Ссылка скопирована");
                        }
                    });

                    ui.label(
                        RichText::new(&torrent.output_folder)
                            .size(8.0)
                            .color(dim),
                    );

                    if torrent.finished {
                        ui.label(
                            RichText::new("Загружено · раздаётся")
                                .size(9.0)
                                .strong()
                                .color(neon),
                        );
                    }

                    if let Some(error) = &torrent.error {
                        ui.label(
                            RichText::new(format!("Ошибка · {error}"))
                                .size(9.0)
                                .color(Color32::from_rgb(220, 120, 110)),
                        );
                    }
                });

            ui.add_space(6.0);
        }
        });
    }
}
