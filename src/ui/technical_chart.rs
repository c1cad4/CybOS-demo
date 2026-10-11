use crate::{cybdex::CybDexCandle, store::Store, technical_analysis as ta};
use eframe::egui::{self, Color32, Stroke, Vec2};
use serde::{Deserialize, Serialize};

const GREEN: Color32 = Color32::from_rgb(70, 230, 165);
const RED: Color32 = Color32::from_rgb(240, 100, 110);
const GOLD: Color32 = Color32::from_rgb(235, 195, 90);
const BLUE: Color32 = Color32::from_rgb(90, 170, 255);
const GRID: Color32 = Color32::from_rgb(22, 43, 37);

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
enum Tool {
    #[default]
    Cursor,
    Level,
    Trend,
    Fibonacci,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
struct Anchor {
    timestamp: i64,
    price: f64,
}
#[derive(Clone, Serialize, Deserialize)]
enum Drawing {
    Level(f64),
    Trend(Anchor, Anchor),
    Fibonacci(Anchor, Anchor),
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    sma: bool,
    ema: bool,
    bands: bool,
    rsi: bool,
    macd: bool,
    volume: bool,
    sma_period: usize,
    ema_period: usize,
    bars: usize,
    offset: usize,
    tool: Tool,
    drawings: Vec<Drawing>,
    #[serde(skip)]
    pending: Option<Anchor>,
    #[serde(skip)]
    pan: f32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            sma: true,
            ema: false,
            bands: false,
            rsi: true,
            macd: false,
            volume: true,
            sma_period: 20,
            ema_period: 50,
            bars: 100,
            offset: 0,
            tool: Tool::Cursor,
            drawings: vec![],
            pending: None,
            pan: 0.0,
        }
    }
}

fn stamp(t: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(t, 0)
        .map(|d| d.format("%d.%m %H:%M").to_string())
        .unwrap_or_default()
}
fn price(v: f64) -> String {
    if v.abs() >= 1.0 {
        format!("{v:.4}")
    } else {
        format!("{v:.8}")
    }
}

fn plot_series(
    p: &egui::Painter,
    rect: egui::Rect,
    series: &[Option<f64>],
    start: usize,
    end: usize,
    min: f64,
    max: f64,
    color: Color32,
) {
    let step = rect.width() / (end - start).max(1) as f32;
    let span = (max - min).max(1e-15);
    let mut segment = vec![];
    for (i, value) in series.iter().enumerate().take(end).skip(start) {
        if let Some(value) = value.filter(|v| v.is_finite()) {
            segment.push(egui::pos2(
                rect.left() + (i - start) as f32 * step + step * 0.5,
                rect.bottom() - ((value - min) / span) as f32 * rect.height(),
            ));
        } else if segment.len() > 1 {
            p.add(egui::Shape::line(
                std::mem::take(&mut segment),
                Stroke::new(1.5, color),
            ));
        } else {
            segment.clear();
        }
    }
    if segment.len() > 1 {
        p.add(egui::Shape::line(segment, Stroke::new(1.5, color)));
    }
}

pub(crate) fn show(
    ui: &mut egui::Ui,
    store: &Store,
    candles: &[CybDexCandle],
    pair: &str,
) -> Option<egui::Rect> {
    let mut candles: Vec<_> = candles
        .iter()
        .filter(|c| {
            [c.open, c.high, c.low, c.close, c.volume]
                .iter()
                .all(|v| v.is_finite())
                && c.low >= 0.0
                && c.high >= c.low
                && c.open >= c.low
                && c.open <= c.high
                && c.close >= c.low
                && c.close <= c.high
                && c.volume >= 0.0
        })
        .cloned()
        .collect();
    candles.sort_by_key(|c| c.timestamp);
    candles.dedup_by_key(|c| c.timestamp);
    if candles.is_empty() {
        ui.label("Нет корректных свечей для анализа.");
        return None;
    }
    let key = format!("technical-chart:{pair}");
    let id = egui::Id::new(&key);
    let mut settings = ui
        .ctx()
        .data_mut(|d| d.get_temp::<Settings>(id))
        .unwrap_or_else(|| {
            store
                .get(&key)
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        });
    let before = serde_json::to_string(&settings).unwrap();
    settings.sma_period = settings.sma_period.clamp(2, 200);
    settings.ema_period = settings.ema_period.clamp(2, 200);
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut settings.sma, "SMA")
            .on_hover_text("Простая средняя цены закрытия");
        ui.checkbox(&mut settings.ema, "EMA")
            .on_hover_text("Экспоненциальная средняя цены закрытия");
        ui.checkbox(&mut settings.bands, "Боллинджер")
            .on_hover_text("SMA 20 ± два стандартных отклонения");
        ui.checkbox(&mut settings.rsi, "RSI 14")
            .on_hover_text("Индекс относительной силы, сглаживание Уайлдера");
        ui.checkbox(&mut settings.macd, "MACD")
            .on_hover_text("EMA 12 − EMA 26, сигнал EMA 9");
        ui.checkbox(&mut settings.volume, "Объём");
    });
    ui.collapsing("Параметры средних", |ui| {
        ui.horizontal(|ui| {
            ui.label("Период SMA");
            ui.add(egui::DragValue::new(&mut settings.sma_period).range(2..=200));
            ui.label("Период EMA");
            ui.add(egui::DragValue::new(&mut settings.ema_period).range(2..=200));
        });
    });
    ui.horizontal_wrapped(|ui| {
        for (tool, label) in [
            (Tool::Cursor, "Курсор"),
            (Tool::Level, "Уровень"),
            (Tool::Trend, "Линия тренда"),
            (Tool::Fibonacci, "Фибоначчи"),
        ] {
            if ui.selectable_label(settings.tool == tool, label).clicked() {
                settings.tool = tool;
                settings.pending = None;
            }
        }
        if ui
            .add_enabled(
                !settings.drawings.is_empty(),
                egui::Button::new("Отменить линию"),
            )
            .clicked()
        {
            settings.drawings.pop();
        }
        if ui.button("Сбросить вид").clicked() {
            settings.bars = 100;
            settings.offset = 0;
            settings.pending = None;
            settings.tool = Tool::Cursor;
        }
    });
    match settings.tool {
        Tool::Level => {
            ui.small("Нажми на графике, чтобы поставить горизонтальный уровень.");
        }
        Tool::Trend | Tool::Fibonacci => {
            ui.small(if settings.pending.is_some() {
                "Выбери вторую точку. Esc отменяет первую."
            } else {
                "Выбери две точки на графике."
            });
        }
        _ => {
            ui.small("Наведи курсор: цена, время и OHLC. Перетаскивай график, чтобы двигаться по истории.");
        }
    }
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        settings.pending = None;
    }
    let len = candles.len();
    settings.bars = settings.bars.clamp(1, len);
    ui.horizontal_wrapped(|ui| {
        ui.add(egui::Slider::new(&mut settings.bars, len.min(20)..=len).text("Свечей на экране"));
        settings.offset = settings.offset.min(len - settings.bars);
        ui.add(
            egui::Slider::new(&mut settings.offset, 0..=len - settings.bars)
                .text("Назад по истории"),
        );
        if ui.button("К последней свече").clicked() {
            settings.offset = 0;
        }
    });
    let end = len - settings.offset;
    let start = end - settings.bars;
    let visible = &candles[start..end];
    let closes: Vec<_> = candles.iter().map(|c| c.close).collect();
    let sma = ta::sma(&closes, settings.sma_period);
    let ema = ta::ema(&closes, settings.ema_period);
    let (upper, middle, lower) = ta::bollinger(&closes, 20, 2.0);
    let mut min = visible.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
    let mut max = visible
        .iter()
        .map(|c| c.high)
        .fold(f64::NEG_INFINITY, f64::max);
    for (on, series) in [
        (settings.sma, &sma),
        (settings.ema, &ema),
        (settings.bands, &upper),
        (settings.bands, &lower),
    ] {
        if on {
            for v in series[start..end].iter().flatten() {
                min = min.min(*v);
                max = max.max(*v);
            }
        }
    }
    let pad = ((max - min) * 0.06).max(max.abs() * 0.001).max(1e-15);
    min -= pad;
    max += pad;
    let height = if settings.volume { 410.0 } else { 350.0 };
    let (outer, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height),
        egui::Sense::click_and_drag(),
    );
    ui.painter()
        .rect_filled(outer, 10.0, Color32::from_rgb(3, 13, 11));
    let chart = egui::Rect::from_min_max(
        outer.min + Vec2::new(10.0, 30.0),
        outer.max - Vec2::new(85.0, 28.0),
    );
    let plot = egui::Rect::from_min_max(
        chart.min,
        egui::pos2(
            chart.right(),
            chart.bottom() - if settings.volume { 80.0 } else { 0.0 },
        ),
    );
    let p = ui.painter_at(plot);
    let step = plot.width() / visible.len() as f32;
    let x = |index: usize| plot.left() + (index as f32 - start as f32 + 0.5) * step;
    let y = |v: f64| plot.bottom() - ((v - min) / (max - min)) as f32 * plot.height();
    for i in 0..=5 {
        let value = min + (max - min) * i as f64 / 5.0;
        let cy = y(value);
        p.line_segment(
            [egui::pos2(plot.left(), cy), egui::pos2(plot.right(), cy)],
            Stroke::new(1.0, GRID),
        );
        ui.painter().text(
            egui::pos2(plot.right() + 6.0, cy),
            egui::Align2::LEFT_CENTER,
            price(value),
            egui::FontId::monospace(11.0),
            Color32::GRAY,
        );
    }
    for (index, c) in visible.iter().enumerate() {
        let cx = x(start + index);
        let color = if c.close >= c.open { GREEN } else { RED };
        let width = (step * 0.65).clamp(1.0, 15.0);
        p.line_segment(
            [egui::pos2(cx, y(c.high)), egui::pos2(cx, y(c.low))],
            Stroke::new(1.0, color),
        );
        p.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(cx - width * 0.5, y(c.open).min(y(c.close))),
                egui::pos2(cx + width * 0.5, y(c.open).max(y(c.close)) + 1.0),
            ),
            1.0,
            color,
        );
        if settings.volume {
            let max_volume = visible.iter().map(|c| c.volume).fold(1e-15_f64, f64::max);
            let h = (c.volume / max_volume) as f32 * 60.0;
            ui.painter_at(chart).rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(cx - width * 0.5, chart.bottom() - h),
                    egui::pos2(cx + width * 0.5, chart.bottom()),
                ),
                0.0,
                color.gamma_multiply(0.5),
            );
        }
    }
    if settings.sma {
        plot_series(&p, plot, &sma, start, end, min, max, GOLD);
    }
    if settings.ema {
        plot_series(&p, plot, &ema, start, end, min, max, BLUE);
    }
    if settings.bands {
        for series in [&upper, &middle, &lower] {
            plot_series(
                &p,
                plot,
                series,
                start,
                end,
                min,
                max,
                Color32::from_rgb(175, 125, 245),
            );
        }
    }
    for index in [start, start + (end - start) / 2, end - 1] {
        ui.painter().text(
            egui::pos2(x(index), outer.bottom() - 8.0),
            egui::Align2::CENTER_BOTTOM,
            stamp(candles[index].timestamp),
            egui::FontId::monospace(11.0),
            Color32::GRAY,
        );
    }
    let hovered = response.hover_pos().filter(|pos| plot.contains(*pos));
    let selected = hovered
        .map(|pos| start + (((pos.x - plot.left()) / step).floor() as usize).min(visible.len() - 1))
        .unwrap_or(end - 1);
    let c = &candles[selected];
    ui.painter().text(
        outer.left_top() + Vec2::new(12.0, 8.0),
        egui::Align2::LEFT_TOP,
        format!(
            "{} UTC · O {}  H {}  L {}  C {} · V {:.2}",
            stamp(c.timestamp),
            price(c.open),
            price(c.high),
            price(c.low),
            price(c.close),
            c.volume
        ),
        egui::FontId::monospace(11.0),
        GREEN,
    );
    if let Some(pos) = hovered {
        p.line_segment(
            [
                egui::pos2(x(selected), plot.top()),
                egui::pos2(x(selected), plot.bottom()),
            ],
            Stroke::new(1.0, Color32::GRAY),
        );
        p.line_segment(
            [
                egui::pos2(plot.left(), pos.y),
                egui::pos2(plot.right(), pos.y),
            ],
            Stroke::new(1.0, Color32::GRAY),
        );
        let value = min + (plot.bottom() - pos.y) as f64 / plot.height() as f64 * (max - min);
        ui.painter().text(
            egui::pos2(plot.right() + 6.0, pos.y),
            egui::Align2::LEFT_CENTER,
            price(value),
            egui::FontId::monospace(11.0),
            GREEN,
        );
        if response.clicked() {
            let anchor = Anchor {
                timestamp: c.timestamp,
                price: value,
            };
            match settings.tool {
                Tool::Level => settings.drawings.push(Drawing::Level(value)),
                Tool::Trend | Tool::Fibonacci => {
                    if let Some(first) = settings.pending.take() {
                        settings.drawings.push(if settings.tool == Tool::Trend {
                            Drawing::Trend(first, anchor)
                        } else {
                            Drawing::Fibonacci(first, anchor)
                        });
                    } else {
                        settings.pending = Some(anchor);
                    }
                }
                _ => {}
            }
        }
    }
    if response.dragged() && settings.tool == Tool::Cursor {
        settings.pan += response.drag_delta().x / step;
        let shift = settings.pan.trunc() as isize;
        if shift != 0 {
            settings.offset = (settings.offset as isize + shift)
                .clamp(0, (len - settings.bars) as isize) as usize;
            settings.pan -= shift as f32;
        }
    }
    let point = |a: Anchor| {
        egui::pos2(
            x(candles.partition_point(|c| c.timestamp < a.timestamp)),
            y(a.price),
        )
    };
    for drawing in &settings.drawings {
        match *drawing {
            Drawing::Level(v) => {
                p.line_segment(
                    [
                        egui::pos2(plot.left(), y(v)),
                        egui::pos2(plot.right(), y(v)),
                    ],
                    Stroke::new(1.3, GOLD),
                );
                p.text(
                    egui::pos2(plot.left() + 4.0, y(v) - 3.0),
                    egui::Align2::LEFT_BOTTOM,
                    price(v),
                    egui::FontId::monospace(11.0),
                    GOLD,
                );
            }
            Drawing::Trend(a, b) => {
                p.line_segment([point(a), point(b)], Stroke::new(1.8, BLUE));
            }
            Drawing::Fibonacci(a, b) => {
                let left = point(a).x.min(point(b).x);
                let right = point(a).x.max(point(b).x);
                for ratio in [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0] {
                    let v = a.price + (b.price - a.price) * ratio;
                    let cy = y(v);
                    p.line_segment(
                        [egui::pos2(left, cy), egui::pos2(right, cy)],
                        Stroke::new(1.0, GOLD),
                    );
                    p.text(
                        egui::pos2(left + 4.0, cy - 3.0),
                        egui::Align2::LEFT_BOTTOM,
                        format!("{:.1}% · {}", ratio * 100.0, price(v)),
                        egui::FontId::monospace(10.0),
                        GOLD,
                    );
                }
            }
        }
    }
    if let Some(a) = settings.pending {
        p.circle_filled(point(a), 4.0, BLUE);
        if let Some(pos) = hovered {
            p.line_segment([point(a), pos], Stroke::new(1.0, BLUE));
        }
    }
    if settings.rsi {
        let series = ta::rsi(&closes, 14);
        oscillator(
            ui,
            "RSI 14 · уровни 30 / 70",
            &series,
            None,
            start,
            end,
            0.0,
            100.0,
            true,
        );
    }
    if settings.macd {
        let (line, signal, hist) = ta::macd(&closes, 12, 26, 9);
        let values: Vec<_> = line[start..end]
            .iter()
            .chain(signal[start..end].iter())
            .chain(hist[start..end].iter())
            .flatten()
            .copied()
            .collect();
        let low = values.iter().copied().fold(0.0_f64, f64::min);
        let high = values.iter().copied().fold(0.0_f64, f64::max);
        let padding = (high - low).max(1e-15) * 0.1;
        oscillator(
            ui,
            "MACD 12 / 26 / 9 · линия / сигнал / гистограмма",
            &line,
            Some((&signal, &hist)),
            start,
            end,
            low - padding,
            high + padding,
            false,
        );
    }
    if candles.len() < 50 {
        ui.small("Короткая история: часть индикаторов появится после накопления достаточного количества свечей.");
    }
    if settings.drawings.len() > 100 {
        settings.drawings.drain(0..settings.drawings.len() - 100);
    }
    let after = serde_json::to_string(&settings).unwrap();
    if after != before {
        store.set(&key, &after);
    }
    ui.ctx().data_mut(|d| d.insert_temp(id, settings));
    Some(plot)
}

fn oscillator(
    ui: &mut egui::Ui,
    label: &str,
    series: &[Option<f64>],
    extra: Option<(&[Option<f64>], &[Option<f64>])>,
    start: usize,
    end: usize,
    min: f64,
    max: f64,
    rsi: bool,
) {
    ui.small(label);
    let (outer, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 95.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(outer, 8.0, Color32::from_rgb(3, 13, 11));
    let r = egui::Rect::from_min_max(
        outer.min + Vec2::new(10.0, 5.0),
        outer.max - Vec2::new(85.0, 5.0),
    );
    let p = ui.painter_at(r);
    let y = |v: f64| r.bottom() - ((v - min) / (max - min).max(1e-15)) as f32 * r.height();
    for level in if rsi {
        vec![30.0, 50.0, 70.0]
    } else {
        vec![0.0]
    } {
        p.line_segment(
            [
                egui::pos2(r.left(), y(level)),
                egui::pos2(r.right(), y(level)),
            ],
            Stroke::new(1.0, GRID),
        );
        ui.painter().text(
            egui::pos2(r.right() + 6.0, y(level)),
            egui::Align2::LEFT_CENTER,
            price(level),
            egui::FontId::monospace(10.0),
            Color32::GRAY,
        );
    }
    if let Some((signal, hist)) = extra {
        let step = r.width() / (end - start) as f32;
        for (i, v) in hist.iter().enumerate().take(end).skip(start) {
            if let Some(v) = v {
                let x = r.left() + (i - start) as f32 * step;
                p.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(x, y(*v).min(y(0.0))),
                        egui::pos2(x + step * 0.7, y(*v).max(y(0.0))),
                    ),
                    0.0,
                    if *v >= 0.0 {
                        GREEN.gamma_multiply(0.5)
                    } else {
                        RED.gamma_multiply(0.5)
                    },
                );
            }
        }
        plot_series(&p, r, signal, start, end, min, max, GOLD);
    }
    plot_series(&p, r, series, start, end, min, max, BLUE);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chart_renders_at_multiple_sizes_and_persists_drawn_level() {
        let store = Store::test_store(":memory:");
        let settings = Settings {
            bands: true,
            ema: true,
            macd: true,
            tool: Tool::Level,
            ..Settings::default()
        };
        store.set(
            "technical-chart:test-pool",
            &serde_json::to_string(&settings).unwrap(),
        );
        let candles: Vec<_> = (0..80)
            .map(|i| CybDexCandle {
                timestamp: 1700000000 + i * 60,
                open: 10.0 + i as f64 / 10.0,
                close: 10.2 + i as f64 / 10.0,
                high: 11.0 + i as f64 / 10.0,
                low: 9.0 + i as f64 / 10.0,
                volume: 100.0 + i as f64,
            })
            .collect();
        let ctx = egui::Context::default();
        let mut plot = None;
        for width in [800.0, 1600.0] {
            for _ in 0..2 {
                let output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 1200.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        plot = show(ui, &store, &candles, "test-pool");
                    },
                );
                assert!(output.shapes.len() > 100);
                output.drop_without_applying_deltas();
            }
            assert!(plot.unwrap().is_finite());
            assert!(plot.unwrap().width() > width * 0.7);
        }
        let pos = plot.unwrap().center();
        for pressed in [true, false] {
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1600.0, 1200.0),
                    )),
                    events: vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::default(),
                        },
                    ],
                    ..Default::default()
                },
                |ui| {
                    let _ = show(ui, &store, &candles, "test-pool");
                },
            );
            output.drop_without_applying_deltas();
        }
        let saved: Settings =
            serde_json::from_str(&store.get("technical-chart:test-pool").unwrap()).unwrap();
        assert_eq!(saved.drawings.len(), 1);
        assert!(matches!(saved.drawings[0], Drawing::Level(v) if v.is_finite()));
    }
}
