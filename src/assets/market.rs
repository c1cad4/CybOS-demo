//! Solana token market data and Assets market visualization.

use crate::models::TokenMarket;
use crate::state::CybOs;
use crate::config::{CICADAFARM_MINT, ROBOTCYB_MINT};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

static TOKEN_MARKET_CACHE: OnceLock<Mutex<(Instant, Vec<TokenMarket>)>> = OnceLock::new();

impl CybOs {
    fn fetch_token_market(name: &str, mint: &str) -> TokenMarket {
        let mut result = TokenMarket {
            name: name.into(),
            ..Default::default()
        };

        // Current token price.
        let price_url = format!(
            "https://api.geckoterminal.com/api/v2/simple/networks/solana/token_price/{}",
            mint
        );

        if let Ok(response) = ureq::get(&price_url)
            .header("accept", "application/json")
            .call()
        {
            if let Ok(value) = response.into_body().read_json::<serde_json::Value>() {
                if let Some(price) = value
                    .pointer("/data/attributes/token_prices")
                    .and_then(|v| v.get(mint))
                    .and_then(|v| v.as_str())
                    .and_then(|v| v.parse::<f64>().ok())
                {
                    result.price = Some(price);
                }
            }
        }

        // Find the most liquid/top pool.
        let pools_url = format!(
            "https://api.geckoterminal.com/api/v2/networks/solana/tokens/{}/pools",
            mint
        );

        if let Ok(response) = ureq::get(&pools_url)
            .header("accept", "application/json")
            .call()
        {
            if let Ok(value) = response.into_body().read_json::<serde_json::Value>() {
                if let Some(pool) = value
                    .get("data")
                    .and_then(|v| v.as_array())
                    .and_then(|v| v.first())
                {
                    result.pool = pool
                        .get("attributes")
                        .and_then(|a| a.get("address"))
                        .and_then(|v| v.as_str())
                        .map(str::to_owned);

                    if let Some(attrs) = pool.get("attributes") {
                        result.liquidity = attrs
                            .get("reserve_in_usd")
                            .and_then(|v| v.as_str())
                            .and_then(|v| v.parse().ok());

                        result.volume_24h = attrs
                            .get("volume_usd")
                            .and_then(|v| v.get("h24"))
                            .and_then(|v| v.as_str())
                            .and_then(|v| v.parse().ok());

                        result.market_cap = attrs
                            .get("market_cap_usd")
                            .and_then(|v| v.as_str())
                            .and_then(|v| v.parse().ok());

                        result.price_change_24h = attrs
                            .get("price_change_percentage")
                            .and_then(|v| v.get("h24"))
                            .and_then(|v| v.as_str())
                            .and_then(|v| v.parse().ok());
                    }
                }
            }
        }

        // Historical price candles from the selected pool.
        if let Some(pool) = result.pool.clone() {
            let chart_url = format!(
                "https://api.geckoterminal.com/api/v2/networks/solana/pools/{}/ohlcv/hour?aggregate=1&limit=48&currency=usd",
                pool
            );

            if let Ok(response) = ureq::get(&chart_url)
                .header("accept", "application/json")
                .call()
            {
                if let Ok(value) = response.into_body().read_json::<serde_json::Value>() {
                    if let Some(rows) = value
                        .pointer("/data/attributes/ohlcv_list")
                        .and_then(|v| v.as_array())
                    {
                        for row in rows {
                            if let Some(values) = row.as_array() {
                                if values.len() >= 5 {
                                    let timestamp = values[0].as_i64();
                                    let close = values[4].as_f64().or_else(|| {
                                        values[4].as_str().and_then(|x| x.parse().ok())
                                    });

                                    if let (Some(timestamp), Some(close)) = (timestamp, close) {
                                        result.candles.push((timestamp, close));
                                    }
                                }
                            }
                        }

                        result.candles.reverse();
                    }
                }
            }
        }

        result
    }

    fn token_markets(&self) -> Vec<TokenMarket> {
        let cache = TOKEN_MARKET_CACHE.get_or_init(|| {
            Mutex::new((
                Instant::now()
                    .checked_sub(Duration::from_secs(120))
                    .unwrap_or_else(Instant::now),
                Vec::new(),
            ))
        });

        let mut guard = cache.lock().unwrap();

        if guard.1.is_empty() || guard.0.elapsed() >= Duration::from_secs(60) {
            let markets = vec![
                Self::fetch_token_market("$CICADAFARM", CICADAFARM_MINT),
                Self::fetch_token_market("$ROBOTCYB", ROBOTCYB_MINT),
            ];

            guard.0 = std::time::Instant::now();
            guard.1 = markets;
        }

        guard.1.clone()
    }

    pub(crate) fn token_matrix(&mut self, ui: &mut egui::Ui) {
        let neon = Self::green();

        ui.heading(RichText::new("◇ TOKEN MATRIX").strong().color(neon));

        ui.label(
            RichText::new("SOLANA · ON-CHAIN MARKET DATA · GECKOTERMINAL")
                .small()
                .color(Color32::GRAY),
        );

        ui.add_space(12.0);

        let markets = self.token_markets();

        for token in markets {
            egui::Frame::new()
                .fill(Color32::from_rgb(5, 18, 12))
                .stroke(Stroke::new(1.0, Color32::from_rgb(25, 90, 58)))
                .corner_radius(10)
                .inner_margin(egui::Margin::same(14))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&token.name).size(20.0).strong().color(neon));

                        ui.separator();

                        if let Some(price) = token.price {
                            ui.label(RichText::new(format!("${:.8}", price)).size(19.0).strong());
                        } else {
                            ui.label(RichText::new("PRICE UNAVAILABLE").color(Color32::GRAY));
                        }

                        if let Some(change) = token.price_change_24h {
                            let sign = if change >= 0.0 { "+" } else { "" };

                            ui.label(RichText::new(format!("{}{:.2}% 24H", sign, change)).color(
                                if change >= 0.0 {
                                    neon
                                } else {
                                    Color32::from_rgb(255, 100, 100)
                                },
                            ));
                        }
                    });

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        if let Some(volume) = token.volume_24h {
                            ui.label(format!("VOL ${:.0}", volume));
                        }

                        if let Some(liquidity) = token.liquidity {
                            ui.label(format!("LIQ ${:.0}", liquidity));
                        }

                        if let Some(mcap) = token.market_cap {
                            ui.label(format!("MCAP ${:.0}", mcap));
                        }
                    });

                    ui.add_space(10.0);

                    if !token.candles.is_empty() {
                        let desired = Vec2::new(ui.available_width(), 120.0);
                        let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());

                        let min = token
                            .candles
                            .iter()
                            .map(|(_, p)| *p)
                            .fold(f64::INFINITY, f64::min);

                        let max = token
                            .candles
                            .iter()
                            .map(|(_, p)| *p)
                            .fold(f64::NEG_INFINITY, f64::max);

                        let range = (max - min).max(f64::EPSILON);
                        let n = token.candles.len().max(2);

                        let points: Vec<egui::Pos2> = token
                            .candles
                            .iter()
                            .enumerate()
                            .map(|(i, (_, price))| {
                                let x = rect.left() + rect.width() * (i as f32 / (n - 1) as f32);

                                let y = rect.bottom()
                                    - rect.height() * (((price - min) / range) as f32);

                                egui::pos2(x, y)
                            })
                            .collect();

                        ui.painter().line(points, Stroke::new(2.0, neon));

                        ui.painter().text(
                            rect.left_top() + Vec2::new(4.0, 4.0),
                            egui::Align2::LEFT_TOP,
                            "48H",
                            egui::FontId::monospace(10.0),
                            Color32::GRAY,
                        );
                    } else {
                        ui.label(RichText::new("NO OHLCV DATA").small().color(Color32::GRAY));
                    }
                });

            ui.add_space(10.0);
        }

        ui.add_space(6.0);

        ui.label(
            RichText::new("Market data: GeckoTerminal · public on-chain API · cached 60s")
                .small()
                .color(Color32::GRAY),
        );
    }
}
