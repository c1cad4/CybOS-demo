use crate::CybOs;
use eframe::egui;
use egui::{Color32, RichText, Stroke, Vec2};

impl CybOs {
    pub(crate) fn browser(&mut self, ui: &mut egui::Ui) {
        self.poll_browser();

        let neon = Self::neon();
        let dim = Color32::from_rgb(55, 145, 105);

        egui::Frame::new()
            .fill(Color32::from_rgb(4, 16, 11))
            .stroke(Stroke::new(1.0, Color32::from_rgb(24, 100, 64)))
            .corner_radius(12)
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("CYBBROWSER")
                            .size(14.0)
                            .strong()
                            .color(neon),
                    );
                    ui.label(
                        RichText::new("PROTOCOL ROUTER · BOUNDED DOCUMENT ENGINE")
                            .size(9.0)
                            .color(dim),
                    );
                });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    let response = ui.add_sized(
                        [620.0, 32.0],
                        egui::TextEdit::singleline(&mut self.browser_url)
                            .hint_text("https://… · ipfs://… · ipns://… · ar://… · cyb://ipfs/…"),
                    );

                    let open = ui
                        .add(egui::Button::new(
                            RichText::new("◈ OPEN")
                                .strong()
                                .color(neon),
                        ))
                        .clicked();

                    if (response.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        || open
                    {
                        self.navigate_browser();
                    }
                });

                ui.add_space(6.0);

                ui.horizontal_wrapped(|ui| {
                    for (name, url) in [
                        ("CYBERIA", "https://cyberia.blog"),
                        ("IPFS TEST", "ipfs://bafybeigdyrzt5example"),
                        ("ARWEAVE", "ar://AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"),
                    ] {
                        if ui
                            .add(egui::Button::new(RichText::new(name).size(9.0).color(neon)))
                            .clicked()
                        {
                            self.browser_url = url.into();
                            self.navigate_browser();
                        }
                    }
                });
            });

        ui.add_space(10.0);

        egui::Frame::new()
            .fill(Color32::from_rgb(5, 18, 13))
            .stroke(Stroke::new(1.0, Color32::from_rgb(22, 80, 52)))
            .corner_radius(10)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("ROUTE · {}", self.browser_route))
                            .size(10.0)
                            .strong()
                            .color(neon),
                    );
                    ui.label(
                        RichText::new(format!("STATUS · {}", self.browser_status))
                            .size(10.0)
                            .color(dim),
                    );
                });

                ui.label(
                    RichText::new(
                        "No remote JavaScript is executed by this backend. Web2, IPFS/IPNS and Arweave are resolved as explicit protocol routes.",
                    )
                    .size(9.0)
                    .color(dim),
                );
            });

        ui.add_space(10.0);

        egui::Frame::new()
            .fill(Color32::from_rgb(3, 14, 10))
            .stroke(Stroke::new(1.0, Color32::from_rgb(24, 90, 58)))
            .corner_radius(12)
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                let title = if self.browser_title.is_empty() {
                    "NO DOCUMENT LOADED"
                } else {
                    self.browser_title.as_str()
                };

                ui.label(
                    RichText::new(title)
                        .size(16.0)
                        .strong()
                        .color(neon),
                );

                if !self.browser_resolved_url.is_empty() {
                    ui.label(
                        RichText::new(&self.browser_resolved_url)
                            .size(9.0)
                            .color(dim),
                    );
                }

                ui.add_space(10.0);

                if self.browser_text.is_empty() {
                    ui.label(
                        RichText::new(
                            "Enter an address to load a document into the native CybBrowser backend.",
                        )
                        .size(11.0)
                        .color(dim),
                    );
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(420.0)
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(&self.browser_text)
                                    .size(11.0)
                                    .color(Color32::from_rgb(195, 235, 212)),
                            );
                        });
                }
            });

        if !self.browser_links.is_empty() {
            ui.add_space(10.0);

            egui::Frame::new()
                .fill(Color32::from_rgb(4, 16, 11))
                .stroke(Stroke::new(1.0, Color32::from_rgb(22, 72, 48)))
                .corner_radius(10)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("LINKS · NATIVE ROUTER")
                            .size(11.0)
                            .strong()
                            .color(neon),
                    );
                    ui.add_space(6.0);

                    for link in self.browser_links.clone() {
                        ui.horizontal(|ui| {
                            if ui
                                .add(egui::Button::new(
                                    RichText::new(&link.label)
                                        .size(9.0)
                                        .color(neon),
                                ))
                                .clicked()
                            {
                                self.browser_url = link.url.clone();
                                self.navigate_browser();
                            }

                            ui.label(
                                RichText::new(link.url)
                                    .size(8.0)
                                    .color(dim),
                            );
                        });
                        ui.add_space(3.0);
                    }
                });
        }

        ui.add_space(10.0);

        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!(
                    "BROWSER CELL · {} · {}",
                    self.browser_status,
                    self.browser_document_bytes
                        .map(|n| format!("{} bytes", n))
                        .unwrap_or_else(|| "NO BODY".into())
                ))
                .size(9.0)
                .color(neon),
            );

            ui.label(
                RichText::new(
                    "VPN status is reported honestly from the network layer; this browser does not fabricate a WireGuard connection.",
                )
                .size(8.0)
                .color(dim),
            );
        });

        let _ = Vec2::ZERO;
    }
}
