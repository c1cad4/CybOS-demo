use crate::CybOs;
use crate::cybdex::{CybDexCandle, CybDexPair, CybDexTimeframe};
use eframe::egui;
use egui::{Color32, RichText, Stroke};

impl CybOs {
    pub(crate) fn cybdex_page(&mut self, ui: &mut egui::Ui) {
        self.poll_cybdex();
        ui.label("Рынок Solana · выбери пару, изучи график и перейди к обмену.");
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let input = ui.add_sized(
                [(ui.available_width() - 250.0).max(180.0), 36.0],
                egui::TextEdit::singleline(&mut self.cybdex_query)
                    .hint_text("Название токена или адрес · например SOL"),
            );
            if ui
                .add_enabled(!self.cybdex.is_busy(), egui::Button::new("Найти пары"))
                .clicked()
                || (input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                if let Err(e) = self.cybdex.search(self.cybdex_query.clone()) {
                    self.notify(e);
                }
            }
            if ui
                .add_enabled(!self.cybdex.is_busy(), egui::Button::new("Обновить"))
                .clicked()
            {
                self.refresh_cybdex_pair();
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (symbol, mint) in [
                ("SOL", "So11111111111111111111111111111111111111112"),
                ("USDC", "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
                ("ROBOTCYB", crate::config::ROBOTCYB_MINT),
                ("CICADAFARM", crate::config::CICADAFARM_MINT),
            ] {
                if ui
                    .add_enabled(!self.cybdex.is_busy(), egui::Button::new(symbol))
                    .clicked()
                {
                    self.cybdex_query = mint.into();
                    if let Err(e) = self.cybdex.search(mint.into()) {
                        self.notify(e);
                    }
                }
            }
            ui.label(if self.cybdex.is_busy() {
                "Загружаем рыночные данные…"
            } else {
                "Данные: DexScreener / GeckoTerminal"
            });
        });
        ui.collapsing("Состояние источников", |ui| {
            ui.label(&self.cybdex_status);
        });
        ui.add_space(12.0);
        if self.cybdex_pairs.is_empty() {
            ui.label("Найди токен, чтобы увидеть доступные торговые пары.");
        } else {
            egui::ScrollArea::vertical()
                .id_salt("dex_pairs")
                .max_height(180.0)
                .show(ui, |ui| {
                    for pair in self.cybdex_pairs.clone() {
                        ui.horizontal_wrapped(|ui| {
                            let selected = self
                                .cybdex_selected_pair
                                .as_ref()
                                .is_some_and(|p| p.pair_address == pair.pair_address);
                            if ui
                                .add_enabled(
                                    !self.cybdex.is_busy(),
                                    egui::Button::new(format!(
                                        "{}/{} · {}",
                                        pair.base_symbol,
                                        pair.quote_symbol,
                                        format!(
                                            "{} · {}",
                                            pair.dex_id,
                                            short_address(&pair.pair_address)
                                        )
                                    ))
                                    .selected(selected),
                                )
                                .clicked()
                            {
                                self.cybdex_selected_pair = Some(pair.clone());
                                self.refresh_cybdex_pair();
                            }
                            if let Some(price) = pair.price_usd {
                                ui.label(price_text(price));
                            }
                            if let Some(change) = pair.price_change_h24 {
                                ui.colored_label(
                                    if change >= 0.0 {
                                        Self::neon()
                                    } else {
                                        Color32::LIGHT_RED
                                    },
                                    format!("{change:+.2}% за сутки"),
                                );
                            }
                            if let Some(liq) = pair.liquidity_usd {
                                ui.small(format!("Ликвидность {}", money_text(liq)));
                            }
                        });
                        ui.add_space(4.0);
                    }
                });
        }
        ui.add_space(12.0);
        if let Some(pair) = self.cybdex_selected_pair.clone() {
            self.draw_cybdex_market(ui, &pair);
        }
        ui.small(
            "Обмен и подписание выполняются в Jupiter. Встроенный кошелёк CybOS пока не подключён.",
        );
    }

    fn draw_cybdex_market(&mut self, ui: &mut egui::Ui, pair: &CybDexPair) {
        let neon = Self::neon();
        let dim = Color32::from_rgb(70, 155, 112);

        egui::Frame::new()
            .fill(Color32::from_rgb(3, 14, 10))
            .stroke(Stroke::new(1.0, Color32::from_rgb(26, 98, 63)))
            .corner_radius(12)
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(format!("{}/{}", pair.base_symbol, pair.quote_symbol))
                                .size(20.0)
                                .strong()
                                .color(neon),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} · {}",
                                pair.dex_id.to_uppercase(),
                                short_address(&pair.pair_address)
                            ))
                            .size(9.0)
                            .color(dim),
                        );
                    });

                    ui.separator();

                    if let Some(price) = pair.price_usd {
                        ui.label(RichText::new(price_text(price)).size(19.0).strong());
                    }
                    if let Some(change) = pair.price_change_h24 {
                        ui.label(
                            RichText::new(format!("{change:+.2}%"))
                                .size(12.0)
                                .strong()
                                .color(if change >= 0.0 {
                                    neon
                                } else {
                                    Color32::from_rgb(255, 105, 105)
                                }),
                        );
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(liq) = pair.liquidity_usd {
                            ui.label(
                                RichText::new(format!("Ликвидность {}", money_text(liq)))
                                    .size(9.0)
                                    .color(dim),
                            );
                        }
                        if let Some(vol) = pair.volume_h24 {
                            ui.label(
                                RichText::new(format!("Объём {}", money_text(vol)))
                                    .size(9.0)
                                    .color(dim),
                            );
                        }
                    });
                });

                ui.add_space(9.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Период").size(9.0).color(dim));
                    for tf in [
                        CybDexTimeframe::Min5,
                        CybDexTimeframe::Min15,
                        CybDexTimeframe::Hour1,
                        CybDexTimeframe::Hour4,
                        CybDexTimeframe::Day1,
                    ] {
                        let active = self.cybdex_timeframe == tf;
                        if ui
                            .add_enabled(
                                !self.cybdex.is_busy(),
                                egui::Button::new(
                                    RichText::new(tf.label())
                                        .size(9.0)
                                        .strong()
                                        .color(if active { neon } else { dim }),
                                ),
                            )
                            .clicked()
                        {
                            self.cybdex_timeframe = tf;
                            self.refresh_cybdex_pair();
                        }
                    }
                    ui.separator();
                    ui.label(RichText::new("AUTO Обновить 20s").size(8.0).color(dim));
                });
            });

        ui.add_space(8.0);

        if self.cybdex_candles.is_empty() {
            ui.label(
                RichText::new("Нет данных для свечного графика")
                    .size(11.0)
                    .strong()
                    .color(neon),
            );
        } else {
            self.draw_candles(ui, &self.cybdex_candles);
        }

        ui.add_space(8.0);
        egui::Frame::new()
            .fill(Color32::from_rgb(4, 16, 11))
            .stroke(Stroke::new(1.0, Color32::from_rgb(19, 62, 42)))
            .corner_radius(10)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.hyperlink_to("Обменять в Jupiter ↗", "https://jup.ag/swap");
                    if ui.button("Копировать адрес продаваемого токена").clicked() { ui.ctx().copy_text(pair.base_address.clone()); }
                    if ui.button("Копировать адрес второго токена").clicked() { ui.ctx().copy_text(pair.quote_address.clone()); }
                });
                ui.small("В Jupiter выбери токены по этим адресам. Проверь пару, сумму и комиссию в кошельке.");
                ui.label(RichText::new("Показатели рынка").size(11.0).strong().color(neon));
                ui.horizontal_wrapped(|ui| {
                    for (label, value) in [
                        ("5M", pair.price_change_m5),
                        ("1H", pair.price_change_h1),
                        ("6H", pair.price_change_h6),
                        ("24H", pair.price_change_h24),
                    ] {
                        if let Some(value) = value {
                            ui.label(
                                RichText::new(format!("{label} {value:+.2}%"))
                                    .size(9.0)
                                    .color(dim),
                            );
                        }
                    }
                    ui.label(
                        RichText::new(format!("Покупок {} · продаж {}", pair.buys_h24, pair.sells_h24))
                            .size(9.0)
                            .color(dim),
                    );
                    if let Some(fdv) = pair.fdv {
                        ui.label(
                            RichText::new(format!("FDV {}", money_text(fdv)))
                                .size(9.0)
                                .color(dim),
                        );
                    }
                    if let Some(mcap) = pair.market_cap {
                        ui.label(
                            RichText::new(format!("MCAP {}", money_text(mcap)))
                                .size(9.0)
                                .color(dim),
                        );
                    }
                });
                ui.label(
                    RichText::new(format!(
                        "BASE {} · QUOTE {}",
                        short_address(&pair.base_address),
                        short_address(&pair.quote_address)
                    ))
                    .size(8.0)
                    .color(dim),
                );
                ui.label(
                    RichText::new(
                        "Встроенное подписание сделок пока недоступно.",
                    )
                    .size(8.0)
                    .color(dim),
                );
            });
    }

    fn draw_candles(&self, ui: &mut egui::Ui, candles: &[CybDexCandle]) {
        let pair = self
            .cybdex_selected_pair
            .as_ref()
            .map(|p| p.pair_address.as_str())
            .unwrap_or("none");
        let _ = super::technical_chart::show(ui, &self.store, candles, pair);
    }
}

fn short_address(value: &str) -> String {
    if value.len() <= 14 {
        value.to_string()
    } else {
        format!("{}…{}", &value[..6], &value[value.len() - 6..])
    }
}

fn price_text(value: f64) -> String {
    if value >= 1.0 {
        "$".to_string() + &format!("{value:.4}")
    } else if value >= 0.01 {
        "$".to_string() + &format!("{value:.6}")
    } else {
        "$".to_string() + &format!("{value:.10}")
    }
}

fn money_text(value: f64) -> String {
    let sign = "$";
    match value.abs() {
        v if v >= 1_000_000_000.0 => format!("{sign}{:.2}B", value / 1_000_000_000.0),
        v if v >= 1_000_000.0 => format!("{sign}{:.2}M", value / 1_000_000.0),
        v if v >= 1_000.0 => format!("{sign}{:.2}K", value / 1_000.0),
        _ => format!("{sign}{:.0}", value),
    }
}
