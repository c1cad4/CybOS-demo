use crate::cybdex::{CybDexCandle, CybDexPair, CybDexTimeframe};
use crate::CybOs;
use eframe::egui;
use egui::{Color32, RichText, Stroke, Vec2};

impl CybOs {
    pub(crate) fn cybdex_page(&mut self, ui: &mut egui::Ui) {
        self.poll_cybdex();
        let neon = Self::neon();
        let dim = Color32::from_rgb(70, 155, 112);

        ui.label(
            RichText::new("CYBDEX · SOLANA MARKET TERMINAL · DISCOVERY → POOL → OHLCV → ROUTING")
                .size(11.0)
                .strong()
                .color(dim),
        );
        ui.add_space(8.0);

        egui::Frame::new()
            .fill(Color32::from_rgb(4, 16, 11))
            .stroke(Stroke::new(1.0, Color32::from_rgb(24, 100, 64)))
            .corner_radius(12)
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("SEARCH").size(11.0).strong().color(neon));
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.cybdex_query)
                            .desired_width(420.0)
                            .hint_text("SOL / USDC · symbol · name · Solana mint"),
                    );
                    let enter = response.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let clicked = ui
                        .add_enabled(!self.cybdex.is_busy(), egui::Button::new("FIND PAIRS")).on_hover_text("Search market data for token pairs matching the symbol, name, or mint above.")
                        .clicked();

                    if enter || clicked {
                        match self.cybdex.search(self.cybdex_query.clone()) {
                            Ok(()) => {
                                self.cybdex_status = "CYBDEX · SEARCHING".into();
                                self.runtime.set_status("CYBDEX", "RUNNING");
                            }
                            Err(error) => self.notify(format!("CYBDEX: {error}")),
                        }
                    }

                    if ui
                        .add_enabled(!self.cybdex.is_busy(), egui::Button::new("REFRESH")).on_hover_text("Fetch the latest available data for the selected pair.")
                        .clicked()
                    {
                        self.refresh_cybdex_pair();
                    }
                });

                ui.add_space(7.0);
                ui.label(RichText::new(&self.cybdex_status).size(9.0).color(dim));
                ui.label(
                    RichText::new(
                        "DexScreener discovery · GeckoTerminal OHLCV · read-only in this phase",
                    )
                    .size(8.0)
                    .color(dim),
                );
            });

        ui.add_space(10.0);

        if !self.cybdex_pairs.is_empty() {
            egui::Frame::new()
                .fill(Color32::from_rgb(3, 13, 9))
                .stroke(Stroke::new(1.0, Color32::from_rgb(20, 70, 48)))
                .corner_radius(10)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("PAIR DISCOVERY").size(12.0).strong().color(neon));
                        ui.label(
                            RichText::new(format!("{} SOLANA PAIR(S)", self.cybdex_pairs.len()))
                                .size(9.0)
                                .color(dim),
                        );
                    });
                    ui.add_space(7.0);

                    for pair in self.cybdex_pairs.clone() {
                        let selected = self
                            .cybdex_selected_pair
                            .as_ref()
                            .map(|p| p.pair_address == pair.pair_address)
                            .unwrap_or(false);

                        egui::Frame::new()
                            .fill(if selected {
                                Color32::from_rgb(7, 28, 18)
                            } else {
                                Color32::from_rgb(4, 17, 12)
                            })
                            .stroke(Stroke::new(
                                1.0,
                                if selected {
                                    neon
                                } else {
                                    Color32::from_rgb(18, 58, 40)
                                },
                            ))
                            .corner_radius(8)
                            .inner_margin(egui::Margin::same(9))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "{}/{}",
                                                pair.base_symbol, pair.quote_symbol
                                            ))
                                            .size(13.0)
                                            .strong()
                                            .color(neon),
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "{} · {}",
                                                pair.dex_id.to_uppercase(),
                                                short_address(&pair.pair_address)
                                            ))
                                            .size(8.0)
                                            .color(dim),
                                        );
                                    });

                                    if let Some(price) = pair.price_usd {
                                        ui.label(RichText::new(price_text(price)).size(12.0).strong());
                                    }
                                    if let Some(liq) = pair.liquidity_usd {
                                        ui.label(
                                            RichText::new(format!("LIQ {}", money_text(liq)))
                                                .size(9.0)
                                                .color(dim),
                                        );
                                    }
                                    if let Some(vol) = pair.volume_h24 {
                                        ui.label(
                                            RichText::new(format!("VOL {}", money_text(vol)))
                                                .size(9.0)
                                                .color(dim),
                                        );
                                    }
                                    if let Some(change) = pair.price_change_h24 {
                                        ui.label(
                                            RichText::new(format!("{change:+.2}%"))
                                                .size(9.0)
                                                .color(if change >= 0.0 {
                                                    neon
                                                } else {
                                                    Color32::from_rgb(255, 105, 105)
                                                }),
                                        );
                                    }

                                    if ui
                                        .add_enabled(
                                            !self.cybdex.is_busy(),
                                            egui::Button::new(if selected { "OPEN" } else { "VIEW" }),
                                        )
                                        .clicked()
                                    {
                                        self.cybdex_selected_pair = Some(pair.clone());
                                        self.refresh_cybdex_pair();
                                    }
                                });
                            });
                        ui.add_space(5.0);
                    }
                });
        }

        if let Some(pair) = self.cybdex_selected_pair.clone() {
            ui.add_space(10.0);
            self.draw_cybdex_market(ui, &pair);
        } else {
            egui::Frame::new()
                .fill(Color32::from_rgb(4, 16, 11))
                .stroke(Stroke::new(1.0, Color32::from_rgb(22, 70, 48)))
                .corner_radius(10)
                .inner_margin(egui::Margin::same(14))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("SELECT A PAIR TO OPEN THE FULL MARKET VIEW")
                            .size(11.0)
                            .strong()
                            .color(neon),
                    );
                    ui.label(
                        RichText::new(
                            "Phase 1 is read-only: pair metrics, pool identity, candles and volume. Routing/signing comes later.",
                        )
                        .size(9.0)
                        .color(dim),
                    );
                });
        }
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
                                RichText::new(format!("LIQ {}", money_text(liq)))
                                    .size(9.0)
                                    .color(dim),
                            );
                        }
                        if let Some(vol) = pair.volume_h24 {
                            ui.label(
                                RichText::new(format!("VOL {}", money_text(vol)))
                                    .size(9.0)
                                    .color(dim),
                            );
                        }
                    });
                });

                ui.add_space(9.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("TIMEFRAME").size(9.0).color(dim));
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
                    ui.label(RichText::new("AUTO REFRESH 20s").size(8.0).color(dim));
                });
            });

        ui.add_space(8.0);

        if self.cybdex_candles.is_empty() {
            ui.label(
                RichText::new("NO OHLCV DATA")
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
                ui.label(RichText::new("MARKET FACTS").size(11.0).strong().color(neon));
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
                        RichText::new(format!("BUYS {} · SELLS {}", pair.buys_h24, pair.sells_h24))
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
                        "ROADMAP: MARKET READ-ONLY ✓  →  ROUTE QUOTES  →  WALLET SIGNING  →  OWN AMM/DEX PROGRAM",
                    )
                    .size(8.0)
                    .color(dim),
                );
            });
    }

    fn draw_candles(&self, ui: &mut egui::Ui, candles: &[CybDexCandle]) {
        let neon = Self::neon();
        let dim = Color32::from_rgb(70, 155, 112);
        let desired = Vec2::new(ui.available_width(), 430.0);
        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 8.0, Color32::from_rgb(2, 10, 7));

        let chart = rect.shrink2(Vec2::new(54.0, 24.0));
        let price_height = chart.height() * 0.70;
        let volume_top = chart.top() + price_height + 18.0;
        let volume_height = chart.bottom() - volume_top;
        let min_price = candles.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
        let max_price = candles.iter().map(|c| c.high).fold(f64::NEG_INFINITY, f64::max);
        let range = (max_price - min_price).max(f64::EPSILON);
        let max_volume = candles.iter().map(|c| c.volume).fold(0.0_f64, f64::max).max(f64::EPSILON);
        let count = candles.len().max(1);
        let step = chart.width() / count as f32;
        let body_width = (step * 0.62).clamp(2.0, 14.0);

        for i in 0..=5 {
            let t = i as f32 / 5.0;
            let y = chart.top() + price_height * t;
            painter.line_segment(
                [egui::pos2(chart.left(), y), egui::pos2(chart.right(), y)],
                Stroke::new(1.0, Color32::from_rgb(10, 42, 29)),
            );
            painter.text(
                egui::pos2(rect.left() + 5.0, y),
                egui::Align2::LEFT_CENTER,
                axis_price(max_price - range * f64::from(t)),
                egui::FontId::monospace(9.0),
                dim,
            );
        }

        for (idx, candle) in candles.iter().enumerate() {
            let x = chart.left() + step * (idx as f32 + 0.5);
            let y = |price: f64| {
                chart.top() + ((max_price - price) / range) as f32 * price_height
            };
            let open_y = y(candle.open);
            let close_y = y(candle.close);
            let high_y = y(candle.high);
            let low_y = y(candle.low);
            let up = candle.close >= candle.open;
            let stroke = Stroke::new(
                1.0,
                if up { neon } else { Color32::from_rgb(255, 105, 105) },
            );

            painter.line_segment([egui::pos2(x, high_y), egui::pos2(x, low_y)], stroke);
            painter.rect(
                egui::Rect::from_min_max(
                    egui::pos2(x - body_width / 2.0, open_y.min(close_y)),
                    egui::pos2(
                        x + body_width / 2.0,
                        open_y.max(close_y).max(open_y.min(close_y) + 1.0),
                    ),
                ),
                1.0,
                if up {
                    Color32::from_rgb(8, 60, 36)
                } else {
                    Color32::from_rgb(75, 20, 20)
                },
                stroke,
                egui::epaint::StrokeKind::Inside,
            );

            let vh = volume_height * (candle.volume / max_volume) as f32;
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(x - body_width / 2.0, chart.bottom() - vh),
                    egui::pos2(x + body_width / 2.0, chart.bottom()),
                ),
                0.0,
                if up {
                    Color32::from_rgb(7, 44, 29)
                } else {
                    Color32::from_rgb(52, 25, 25)
                },
            );
        }

        if let Some(pos) = response.hover_pos() {
            if chart.contains(pos) {
                let index = (((pos.x - chart.left()) / step).floor() as usize).min(count - 1);
                let candle = &candles[index];
                let x = chart.left() + step * (index as f32 + 0.5);
                let y = chart.top() + ((max_price - candle.close) / range) as f32 * price_height;
                painter.line_segment(
                    [egui::pos2(x, chart.top()), egui::pos2(x, chart.bottom())],
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(120, 255, 180, 90)),
                );
                painter.line_segment(
                    [egui::pos2(chart.left(), y), egui::pos2(chart.right(), y)],
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(120, 255, 180, 70)),
                );
                painter.text(
                    rect.center_top() + Vec2::new(0.0, 5.0),
                    egui::Align2::CENTER_TOP,
                    format!(
                        "{} · O {} H {} L {} C {}",
                        format_timestamp(candle.timestamp),
                        axis_price(candle.open),
                        axis_price(candle.high),
                        axis_price(candle.low),
                        axis_price(candle.close)
                    ),
                    egui::FontId::monospace(9.0),
                    neon,
                );
            }
        }

        ui.label(
            RichText::new(format!(
                "{} candles · {} · OHLC + volume · hover for crosshair",
                candles.len(),
                self.cybdex_timeframe.label()
            ))
            .size(8.0)
            .color(dim),
        );
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

fn axis_price(value: f64) -> String {
    if value >= 1.0 {
        format!("{value:.4}")
    } else if value >= 0.01 {
        format!("{value:.6}")
    } else if value == 0.0 {
        "0".into()
    } else {
        format!("{value:.10}")
    }
}

fn format_timestamp(timestamp: i64) -> String {
    use chrono::{DateTime, Utc};
    DateTime::<Utc>::from_timestamp(timestamp, 0)
        .map(|dt| dt.format("%m-%d %H:%M").to_string())
        .unwrap_or_else(|| timestamp.to_string())
}
