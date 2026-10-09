use crate::CybOs;
use eframe::egui;
use egui::{Color32, RichText, Stroke};

impl CybOs {
    pub(crate) fn cyblex(&mut self, ui: &mut egui::Ui) {
        self.poll_cyblex();

        let neon = Self::neon();
        let dim = Color32::from_rgb(55, 145, 105);

        egui::Frame::new()
            .fill(Color32::from_rgb(4, 16, 11))
            .stroke(Stroke::new(1.0, Color32::from_rgb(24, 100, 64)))
            .corner_radius(12)
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("CYBLEX · DECENTRALIZED DIGITAL ARCHIVE")
                        .size(14.0)
                        .strong()
                        .color(neon),
                );
                ui.label(
                    RichText::new(
                        "Embedded librqbit 9.0.1 · DHT / trackers · native local-first control plane",
                    )
                    .size(10.0)
                    .color(dim),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "AUTHORIZED CONTENT ONLY · OWNED · PUBLIC-DOMAIN · OPEN-LICENSE · CREATOR-AUTHORIZED",
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
                ui.label(RichText::new("DOWNLOAD").size(12.0).strong().color(neon));
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.cyblex_source)
                            .desired_width((ui.available_width() - 150.0).max(150.0))
                            .hint_text("magnet:?… or https://…/file.torrent"),
                    );

                    if ui.button("START DOWNLOAD").clicked() {
                        match self.cyblex.add_source(
                            self.cyblex_source.clone(),
                            self.cyblex_download_path.clone(),
                        ) {
                            Ok(()) => self.notify("CYBLEX DOWNLOAD QUEUED"),
                            Err(error) => self.notify(format!("CYBLEX: {error}")),
                        }
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(RichText::new("OUTPUT").size(9.0).color(dim));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.cyblex_download_path)
                            .desired_width((ui.available_width() - 90.0).max(150.0)),
                    );

                    if ui.button("DEFAULT").clicked() {
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
                    RichText::new("SEED / SHARE")
                        .size(12.0)
                        .strong()
                        .color(neon),
                );
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.cyblex_seed_path)
                            .desired_width((ui.available_width() - 150.0).max(150.0))
                            .hint_text("~/path/to/file-or-folder"),
                    );

                    if ui.button("CREATE + SEED").clicked() {
                        match self.cyblex.seed_path(self.cyblex_seed_path.clone()) {
                            Ok(()) => self.notify("CYBLEX SEEDING QUEUED"),
                            Err(error) => self.notify(format!("CYBLEX: {error}")),
                        }
                    }
                });

                ui.label(
                    RichText::new(
                        "Creates a .torrent sidecar beside the shared content and reports a magnet URI.",
                    )
                    .size(9.0)
                    .color(dim),
                );
            });

        ui.add_space(12.0);

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(&self.cyblex_status)
                    .size(10.0)
                    .strong()
                    .color(neon),
            );
            ui.label(
                RichText::new(format!("{} TORRENT(S)", self.cyblex_torrents.len()))
                    .size(9.0)
                    .color(dim),
            );
        });

        ui.add_space(6.0);

        if self.cyblex_torrents.is_empty() {
            ui.label(
                RichText::new(
                    "No active CybLex torrents. Add a magnet/.torrent source or seed authorized local content.",
                )
                .size(10.0)
                .color(dim),
            );
        }

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
                            if ui.button("RESUME").clicked() {
                                if let Err(error) = self.cyblex.resume(torrent.id) {
                                    self.notify(format!("CYBLEX: {error}"));
                                } else {
                                    self.notify("CYBLEX RESUME QUEUED");
                                }
                            }
                        } else if ui.button("PAUSE").clicked() {
                            if let Err(error) = self.cyblex.pause(torrent.id) {
                                self.notify(format!("CYBLEX: {error}"));
                            } else {
                                self.notify("CYBLEX PAUSE QUEUED");
                            }
                        }

                        if ui.button("REMOVE").clicked() {
                            if let Err(error) = self.cyblex.forget(torrent.id, false) {
                                self.notify(format!("CYBLEX: {error}"));
                            } else {
                                self.notify("CYBLEX TORRENT REMOVAL QUEUED");
                            }
                        }
                    });

                    ui.add(
                        egui::ProgressBar::new(progress)
                            .text(format!(
                                "{:.1}% · {} / {} bytes",
                                progress * 100.0,
                                torrent.progress_bytes,
                                torrent.total_bytes
                            ))
                            .desired_width(f32::INFINITY),
                    );

                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(format!("UPLOADED · {} bytes", torrent.uploaded_bytes))
                                .size(9.0)
                                .color(dim),
                        );
                        ui.label(
                            RichText::new(format!("HASH {}", torrent.info_hash))
                                .size(8.0)
                                .color(dim),
                        );

                        if ui.button("COPY MAGNET").clicked() {
                            ui.ctx().copy_text(torrent.magnet_uri.clone());
                            self.notify("CYBLEX MAGNET COPIED");
                        }
                    });

                    ui.label(
                        RichText::new(&torrent.output_folder)
                            .size(8.0)
                            .color(dim),
                    );

                    if torrent.finished {
                        ui.label(
                            RichText::new("SEEDING / COMPLETE")
                                .size(9.0)
                                .strong()
                                .color(neon),
                        );
                    }

                    if let Some(error) = &torrent.error {
                        ui.label(
                            RichText::new(format!("ERROR · {error}"))
                                .size(9.0)
                                .color(Color32::from_rgb(220, 120, 110)),
                        );
                    }
                });

            ui.add_space(6.0);
        }
    }
}
