#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod store;

mod brain;
mod ai;
mod network;

use models::{
    Event,
    GraphLink,
    GraphNode,
    Memory,
    TokenMarket,
};
use store::Store;
use chrono::Local;
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use std::{
    collections::HashMap,
    process::{Child, Command},
    time::{Duration, Instant},
};
use uuid::Uuid;


const CICADAFARM_MINT: &str = "9QLCEL7Xo9VTwgBeAYU1PWX7JJ8joKxQCYw3msjUpump";
const ROBOTCYB_MINT: &str = "8WZiguAp8NyFnwm8Z97k6sCbSRCWaW1YYXKeCTpupump";


static TOKEN_MARKET_CACHE: std::sync::OnceLock<
    std::sync::Mutex<(std::time::Instant, Vec<TokenMarket>)>,
> = std::sync::OnceLock::new();

const APP_VERSION: &str = "0.6.0";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Dashboard,
    Robot,
    Farm,
    Chat,
    Graph,
    Brain,
    Network,
    Assets,
    Cameras,
    System,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Icon {
    Dashboard,
    Graph,
    Network,
    Brain,
    Farm,
    Robot,
    Camera,
    System,
    Assets,
    Chat,
    Energy,
    Environment,
    Activity,
    Node,
    Search,
    Plus,
}

impl Page {
    fn title(self) -> &'static str {
        match self {
            Page::Dashboard => "CENTRAL CYBOS DASHBOARD",
            Page::Graph => "CYBOS GRAPH",
            Page::Network => "NETWORK MATRIX",
            Page::Brain => "CYBOS BRAIN",
            Page::Farm => "CICADAFARM",
            Page::Robot => "ROBOTCYB",
            Page::System => "SYSTEM CORE",
            Page::Chat => "CYBCHAT",
            Page::Assets => "ASSETS",
            Page::Cameras => "FARM CAMERAS",
        }
    }

    fn icon(self) -> Icon {
        match self {
            Page::Dashboard => Icon::Dashboard,
            Page::Graph => Icon::Graph,
            Page::Network => Icon::Network,
            Page::Brain => Icon::Brain,
            Page::Farm => Icon::Farm,
            Page::Robot => Icon::Robot,
            Page::System => Icon::System,
            Page::Chat => Icon::Chat,
            Page::Assets => Icon::Assets,
            Page::Cameras => Icon::Camera,
        }
    }

    fn matches_query(self, q: &str) -> bool {
        let keys = match self {
            Page::Dashboard => "dashboard home core sigma live node activity",
            Page::Robot => "robot robotcyb agent qwen ai",
            Page::Farm => "farm cicadafarm hive chicken goat honey eggs environment",
            Page::Chat => "chat cybchat message peer",
            Page::Graph => "graph cybergraph nodes links",
            Page::Brain => "brain memory knowledge qwen",
            Page::Network => "network lan p2p ble nostr",
            Page::Assets => "assets tokens cicadafarm robotcyb mint solana",
            Page::Cameras => "cameras camera rtsp farm live",
            Page::System => "system energy battery node database",
        };
        keys.contains(q) || self.title().to_lowercase().contains(q)
    }
}





pub(crate) struct CybOs {
    store: Store,
    page: Page,
    search: String,
    robot_input: String,
    robot_output: String,
    chat_input: String,
    chat_output: String,
    chat: Vec<(String, String, bool)>,
    // CicadaFarm commerce
    cicada_wallet: String,
    cicada_balance: Option<f64>,
    sol_balance: Option<f64>,
    payment_uri: String,
    payment_status: String,
    balance_refresh: std::time::Instant,
    events: Vec<Event>,
    nodes: Vec<GraphNode>,
    links: Vec<GraphLink>,
    graph_zoom: f32,
    graph_pan: Vec2,
    selected_node: Option<String>,
    temperature: f32,
    battery: f32,
    node_id: String,
    status: String,
    qwen_child: Option<Child>,
    qwen_status: String,
    toast: Option<(String, Instant)>,
    camera_zone: usize,
    search_focus: bool,
    last_scan: Option<Instant>,
    remember_note: String,
}
impl Default for CybOs {
    fn default() -> Self {
        let store = Store::open();
        let node_id = store.get("node_id").unwrap_or_else(|| {
            let v = format!("cyb-{}", Uuid::new_v4().to_string()[..8].to_string());
            store.set("node_id", &v);
            v
        });
        let mut events = store.events();
        if events.is_empty() {
            store.add_event("SYSTEM", "cybOS local runtime initialized");
            events = store.events();
        }
        let farm = String::from("cicadafarm");
        let robot = String::from("robotcyb");
        let hive = String::from("hive-003");
        let apiary = String::from("apiary-north");
        let mut app = Self {
            store,
            page: Page::Dashboard,
            search: String::new(),
            robot_input: String::new(),
            robot_output: String::from("ROBOTCYB READY\n\nLOCAL AGENT ONLINE\nAwaiting request..."),
            chat_input: String::new(),
            chat_output: String::from("CYBCHAT READY\n\nLOCAL-FIRST CHANNEL\nAwaiting message..."),
            chat: vec![(
                "ROBOTCYB".into(),
                "CicadaFarm is online. Your local cybOS node is ready.".into(),
                false,
            )],
            cicada_wallet: String::from("AHBz386tP36ZrABN7STzxV66u1f7CwnaqrFBnevb4ff8"),
            cicada_balance: None,
            sol_balance: None,
            payment_uri: String::new(),
            payment_status: String::from("PAYMENT REQUEST READY"),
            balance_refresh: std::time::Instant::now() - std::time::Duration::from_secs(60),
            events,
            nodes: vec![
                GraphNode {
                    id: farm.clone(),
                    label: "CICADAFARM".into(),
                    kind: "FARM".into(),
                    x: 0.0,
                    y: 0.0,
                },
                GraphNode {
                    id: robot.clone(),
                    label: "ROBOTCYB".into(),
                    kind: "AGENT".into(),
                    x: -220.0,
                    y: -110.0,
                },
                GraphNode {
                    id: hive.clone(),
                    label: "HIVE #003".into(),
                    kind: "BEE".into(),
                    x: 220.0,
                    y: -110.0,
                },
                GraphNode {
                    id: apiary.clone(),
                    label: "NORTH APIARY".into(),
                    kind: "PLACE".into(),
                    x: 220.0,
                    y: 110.0,
                },
                GraphNode {
                    id: "weather-001".into(),
                    label: "WEATHER SENSOR".into(),
                    kind: "SENSOR".into(),
                    x: -220.0,
                    y: 120.0,
                },
                GraphNode {
                    id: "knowledge".into(),
                    label: "KNOWLEDGE".into(),
                    kind: "BRAIN".into(),
                    x: 0.0,
                    y: 210.0,
                },
            ],
            links: vec![
                GraphLink {
                    from: farm.clone(),
                    to: robot,
                    relation: "has_agent".into(),
                },
                GraphLink {
                    from: farm.clone(),
                    to: hive.clone(),
                    relation: "contains".into(),
                },
                GraphLink {
                    from: hive,
                    to: apiary,
                    relation: "located_at".into(),
                },
                GraphLink {
                    from: farm.clone(),
                    to: "weather-001".into(),
                    relation: "observed_by".into(),
                },
                GraphLink {
                    from: farm,
                    to: "knowledge".into(),
                    relation: "feeds".into(),
                },
            ],
            graph_zoom: 1.0,
            graph_pan: Vec2::ZERO,
            selected_node: None,
            temperature: 24.0,
            battery: 100.0,
            node_id,
            status: "LOCAL-FIRST · READY".into(),
            qwen_child: None,
            qwen_status: "QWEN · STARTING".into(),
            toast: None,
            camera_zone: 0,
            search_focus: false,
            last_scan: None,
            remember_note: String::new(),
        };

        let stored_nodes = app.store.graph_nodes();
        let stored_links = app.store.graph_links();

        if stored_nodes.is_empty() {
            app.persist_current_graph();
        } else {
            app.nodes = stored_nodes;
            app.links = stored_links;
        }

        app
    }
}
impl CybOs {
    fn fetch_token_market(name: &str, mint: &str) -> TokenMarket {
        let mut result = TokenMarket {
            name: name.into(),
            mint: mint.into(),
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

    fn refresh_cicada_balances(&mut self) {
        const RPC: &str = "https://api.mainnet-beta.solana.com";

        // ----------------------------------------------------
        // SOL balance
        // ----------------------------------------------------
        let sol_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBalance",
            "params": [self.cicada_wallet]
        });

        if let Ok(response) = ureq::post(RPC)
            .header("content-type", "application/json")
            .send(sol_body.to_string())
        {
            if let Ok(value) = response.into_body().read_json::<serde_json::Value>() {
                if let Some(lamports) = value["result"]["value"].as_u64() {
                    self.sol_balance = Some(lamports as f64 / 1_000_000_000.0);
                }
            }
        }

        // ----------------------------------------------------
        // $CICADAFARM SPL token balance
        // ----------------------------------------------------
        let token_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "getTokenAccountsByOwner",
            "params": [
                self.cicada_wallet,
                {
                    "mint": CICADAFARM_MINT
                },
                {
                    "encoding": "jsonParsed"
                }
            ]
        });

        if let Ok(response) = ureq::post(RPC)
            .header("content-type", "application/json")
            .send(token_body.to_string())
        {
            if let Ok(value) = response.into_body().read_json::<serde_json::Value>() {
                let mut total = 0.0;

                if let Some(accounts) = value["result"]["value"].as_array() {
                    for account in accounts {
                        if let Some(amount) =
                            account["account"]["data"]["parsed"]["info"]["tokenAmount"]["uiAmount"]
                                .as_f64()
                        {
                            total += amount;
                        }
                    }
                }

                self.cicada_balance = Some(total);
            }
        }
    }

    fn token_markets(&self) -> Vec<TokenMarket> {
        let cache = TOKEN_MARKET_CACHE.get_or_init(|| {
            std::sync::Mutex::new((
                std::time::Instant::now()
                    .checked_sub(std::time::Duration::from_secs(120))
                    .unwrap_or_else(std::time::Instant::now),
                Vec::new(),
            ))
        });

        let mut guard = cache.lock().unwrap();

        if guard.1.is_empty() || guard.0.elapsed() >= std::time::Duration::from_secs(60) {
            let markets = vec![
                Self::fetch_token_market("$CICADAFARM", CICADAFARM_MINT),
                Self::fetch_token_market("$ROBOTCYB", ROBOTCYB_MINT),
            ];

            guard.0 = std::time::Instant::now();
            guard.1 = markets;
        }

        guard.1.clone()
    }

    fn token_matrix(&mut self, ui: &mut egui::Ui) {
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



    fn persist_current_graph(&self) {
        for node in &self.nodes {
            self.store.save_graph_node(node);
        }

        for link in &self.links {
            self.store.save_graph_link(link);
        }
    }

    fn green() -> Color32 {
        Color32::from_rgb(125, 255, 189)
    }

    fn neon() -> Color32 {
        Color32::from_rgb(0, 255, 150)
    }

    fn dim() -> Color32 {
        Color32::from_rgb(28, 92, 62)
    }

    fn notify(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), Instant::now()));
        self.status = "LOCAL-FIRST · READY".into();
    }

    fn go(&mut self, page: Page) {
        if self.page != page {
            self.page = page;
            self.notify(format!("OPENED {}", page.title()));
        }
    }

    fn apply_search(&mut self) {
        let q = self.search.trim().to_lowercase();
        if q.is_empty() {
            self.notify("TYPE A PAGE, TOKEN, OR SYSTEM NAME");
            return;
        }
        let pages = [
            Page::Dashboard,
            Page::Robot,
            Page::Farm,
            Page::Chat,
            Page::Graph,
            Page::Brain,
            Page::Network,
            Page::Assets,
            Page::Cameras,
            Page::System,
        ];
        if let Some(page) = pages.into_iter().find(|p| p.matches_query(&q)) {
            self.go(page);
            return;
        }
        if q.contains("event") || q.contains("log") {
            self.go(Page::Dashboard);
            return;
        }
        self.notify(format!("NO MATCH FOR “{}”", self.search.trim()));
    }

    fn paint_icon(painter: &egui::Painter, c: egui::Pos2, icon: Icon, color: Color32, s: f32) {
        let st = Stroke::new((s * 0.09).clamp(1.1, 2.4), color);
        match icon {
            Icon::Dashboard => {
                painter.line_segment(
                    [c + egui::vec2(-s * 0.42, -s * 0.28), c + egui::vec2(s * 0.42, -s * 0.28)],
                    st,
                );
                painter.line_segment(
                    [c + egui::vec2(-s * 0.42, s * 0.28), c + egui::vec2(s * 0.42, s * 0.28)],
                    st,
                );
                painter.line_segment(
                    [c + egui::vec2(-s * 0.18, -s * 0.28), c + egui::vec2(s * 0.18, s * 0.28)],
                    st,
                );
            }
            Icon::Graph => {
                let mut pts = Vec::new();
                for i in 0..=16 {
                    let t = i as f32 / 16.0;
                    let x = c.x - s * 0.46 + t * s * 0.92;
                    let y = c.y + (t * std::f32::consts::TAU).sin() * s * 0.28;
                    pts.push(egui::pos2(x, y));
                }
                painter.add(egui::Shape::line(pts, st));
            }
            Icon::Network => {
                painter.line_segment(
                    [c + egui::vec2(-s * 0.38, -s * 0.18), c + egui::vec2(-s * 0.08, 0.0)],
                    st,
                );
                painter.line_segment(
                    [c + egui::vec2(-s * 0.38, s * 0.18), c + egui::vec2(-s * 0.08, 0.0)],
                    st,
                );
                painter.line_segment(
                    [c + egui::vec2(s * 0.38, -s * 0.18), c + egui::vec2(s * 0.08, 0.0)],
                    st,
                );
                painter.line_segment(
                    [c + egui::vec2(s * 0.38, s * 0.18), c + egui::vec2(s * 0.08, 0.0)],
                    st,
                );
                painter.line_segment(
                    [c + egui::vec2(-s * 0.08, 0.0), c + egui::vec2(s * 0.08, 0.0)],
                    st,
                );
            }
            Icon::Brain => {
                let hex = (0..6)
                    .map(|i| {
                        let a = i as f32 * std::f32::consts::TAU / 6.0 - std::f32::consts::FRAC_PI_2;
                        c + egui::vec2(a.cos(), a.sin()) * s * 0.42
                    })
                    .collect::<Vec<_>>();
                painter.add(egui::Shape::closed_line(hex, st));
                painter.circle_filled(c, s * 0.09, color);
            }
            Icon::Farm => {
                let hex = (0..6)
                    .map(|i| {
                        let a = i as f32 * std::f32::consts::TAU / 6.0;
                        c + egui::vec2(a.cos(), a.sin()) * s * 0.38
                    })
                    .collect::<Vec<_>>();
                painter.add(egui::Shape::closed_line(hex, st));
                painter.circle_stroke(c, s * 0.14, st);
            }
            Icon::Robot => {
                let d = s * 0.38;
                painter.add(egui::Shape::closed_line(
                    vec![
                        c + egui::vec2(0.0, -d),
                        c + egui::vec2(d, 0.0),
                        c + egui::vec2(0.0, d),
                        c + egui::vec2(-d, 0.0),
                    ],
                    st,
                ));
                painter.circle_filled(c, s * 0.1, color);
            }
            Icon::Camera => {
                let r = egui::Rect::from_center_size(c + egui::vec2(0.0, 2.0), egui::vec2(s * 0.78, s * 0.5));
                painter.rect_stroke(r, 3.0, st, egui::StrokeKind::Middle);
                painter.circle_stroke(c + egui::vec2(0.0, 2.0), s * 0.14, st);
                painter.rect_stroke(
                    egui::Rect::from_center_size(c + egui::vec2(-s * 0.16, -s * 0.28), egui::vec2(s * 0.22, s * 0.12)),
                    2.0,
                    st,
                    egui::StrokeKind::Middle,
                );
            }
            Icon::System => {
                for i in 0..6 {
                    let a = i as f32 * std::f32::consts::TAU / 6.0;
                    painter.line_segment(
                        [c + egui::vec2(a.cos(), a.sin()) * s * 0.22, c + egui::vec2(a.cos(), a.sin()) * s * 0.42],
                        st,
                    );
                }
                painter.circle_stroke(c, s * 0.18, st);
            }
            Icon::Assets => {
                let d = s * 0.36;
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        c + egui::vec2(0.0, -d),
                        c + egui::vec2(d, 0.0),
                        c + egui::vec2(0.0, d),
                        c + egui::vec2(-d, 0.0),
                    ],
                    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 40),
                    st,
                ));
            }
            Icon::Chat => {
                painter.circle_stroke(c + egui::vec2(-s * 0.12, -s * 0.04), s * 0.22, st);
                painter.circle_stroke(c + egui::vec2(s * 0.16, s * 0.08), s * 0.16, st);
                for k in 0..3 {
                    painter.circle_filled(
                        c + egui::vec2(-s * 0.22 + k as f32 * s * 0.1, -s * 0.04),
                        s * 0.035,
                        color,
                    );
                }
            }
            Icon::Energy => {
                painter.add(egui::Shape::line(
                    vec![
                        c + egui::vec2(s * 0.08, -s * 0.42),
                        c + egui::vec2(-s * 0.12, 0.02),
                        c + egui::vec2(s * 0.1, 0.02),
                        c + egui::vec2(-s * 0.08, s * 0.42),
                    ],
                    st,
                ));
            }
            Icon::Environment => {
                painter.circle_stroke(c, s * 0.18, st);
                for i in 0..8 {
                    let a = i as f32 * std::f32::consts::TAU / 8.0;
                    painter.line_segment(
                        [c + egui::vec2(a.cos(), a.sin()) * s * 0.26, c + egui::vec2(a.cos(), a.sin()) * s * 0.4],
                        st,
                    );
                }
            }
            Icon::Activity => {
                let pts = [
                    c + egui::vec2(-s * 0.4, s * 0.1),
                    c + egui::vec2(-s * 0.22, s * 0.1),
                    c + egui::vec2(-s * 0.1, -s * 0.28),
                    c + egui::vec2(0.08, s * 0.32),
                    c + egui::vec2(0.22, -s * 0.08),
                    c + egui::vec2(s * 0.4, -s * 0.08),
                ];
                painter.add(egui::Shape::line(pts.to_vec(), st));
            }
            Icon::Node => {
                painter.circle_stroke(c, s * 0.32, st);
                painter.circle_stroke(c, s * 0.18, st);
                painter.circle_filled(c, s * 0.07, color);
            }
            Icon::Search => {
                painter.circle_stroke(c + egui::vec2(-s * 0.08, -s * 0.08), s * 0.22, st);
                painter.line_segment(
                    [c + egui::vec2(s * 0.08, s * 0.08), c + egui::vec2(s * 0.32, s * 0.32)],
                    st,
                );
            }
            Icon::Plus => {
                painter.line_segment([c + egui::vec2(-s * 0.28, 0.0), c + egui::vec2(s * 0.28, 0.0)], st);
                painter.line_segment([c + egui::vec2(0.0, -s * 0.28), c + egui::vec2(0.0, s * 0.28)], st);
            }
        }
    }

    fn rail_icon(&mut self, ui: &mut egui::Ui, icon: Icon, tooltip: &str, active: bool) -> bool {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(64.0, 54.0), egui::Sense::click());
        let painter = ui.painter();
        let pulse = ((ui.input(|i| i.time) * 2.4).sin() * 0.5 + 0.5) as f32;
        let c = rect.center();
        let color = if active {
            Self::neon()
        } else if response.hovered() {
            Color32::from_rgb(90, 230, 160)
        } else {
            Color32::from_rgb(42, 120, 82)
        };
        if active {
            painter.rect_filled(
                egui::Rect::from_min_max(rect.left_top(), rect.left_bottom() + egui::vec2(3.0, 0.0)),
                0.0,
                Self::neon(),
            );
            painter.circle_stroke(
                c,
                20.0 + pulse * 1.6,
                Stroke::new(1.1, Color32::from_rgba_unmultiplied(0, 255, 150, 90)),
            );
        } else if response.hovered() {
            painter.circle_stroke(c, 19.0, Stroke::new(1.0, color));
        }
        Self::paint_icon(painter, c, icon, color, if active { 22.0 } else { 20.0 });
        response.clone().on_hover_text(tooltip);
        response.clicked()
    }

    fn add_event(&mut self, kind: &str, text: impl Into<String>) {
        let text = text.into();
        self.store.add_event(kind, &text);
        self.events = self.store.events();
    }


    // ============================================================
    // RobotCYB Tools — local access to cybOS memory and state
    // ============================================================

















    // ============================================================
    // RobotCYB Tool Router
    // Read-only routing layer between user queries and cybOS tools
    // ============================================================







































    fn neon_theme(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();

        visuals.override_text_color = Some(Color32::from_rgb(210, 255, 228));

        visuals.extreme_bg_color = Color32::from_rgb(1, 6, 5);

        visuals.faint_bg_color = Color32::from_rgb(4, 17, 12);

        visuals.code_bg_color = Color32::from_rgb(2, 11, 8);

        visuals.window_fill = Color32::from_rgb(3, 12, 9);

        visuals.panel_fill = Color32::from_rgb(2, 9, 7);

        visuals.hyperlink_color = Color32::from_rgb(80, 255, 170);

        ctx.set_visuals(visuals);
    }

    fn nav_button(&mut self, ui: &mut egui::Ui, page: Page, icon: &str, label: &str) {
        let selected = self.page == page;

        let (rect, response) = ui.allocate_exact_size(Vec2::new(64.0, 62.0), egui::Sense::click());

        let painter = ui.painter();

        let pulse = ((ui.input(|i| i.time) * 2.2).sin() * 0.5 + 0.5) as f32;

        let center = rect.center();

        if selected {
            painter.circle_stroke(
                center,
                27.0 + pulse * 2.0,
                Stroke::new(2.0 + pulse, Color32::from_rgb(80, 255, 170)),
            );

            painter.circle_stroke(
                center,
                31.0 + pulse * 3.0,
                Stroke::new(0.7, Color32::from_rgb(30, 125, 85)),
            );
        } else {
            painter.circle_stroke(
                center,
                25.0,
                Stroke::new(1.0, Color32::from_rgb(28, 78, 55)),
            );
        }

        painter.circle_filled(
            center,
            21.0,
            if selected {
                Color32::from_rgb(24, 94, 60)
            } else {
                Color32::from_rgb(5, 24, 17)
            },
        );

        painter.circle_stroke(
            center,
            21.0,
            Stroke::new(
                1.0,
                if selected {
                    Color32::from_rgb(95, 255, 185)
                } else {
                    Color32::from_rgb(32, 105, 73)
                },
            ),
        );

        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(if selected { 20.0 } else { 18.0 }),
            if selected {
                Color32::from_rgb(220, 255, 235)
            } else {
                Color32::from_rgb(130, 220, 170)
            },
        );

        if response.clicked() {
            self.page = page;
        }

        response.on_hover_text(label);
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        self.neon_theme(ui.ctx());

        egui::Panel::left("sidebar")
            .resizable(false)
            .exact_size(86.0)
            .show(ui, |ui| {
                ui.add_space(14.0);

                ui.vertical_centered(|ui| {
                    let pulse = ((ui.input(|i| i.time) * 1.8).sin() * 0.5 + 0.5) as f32;

                    ui.painter().circle_stroke(
                        ui.cursor().center() + Vec2::new(0.0, 17.0),
                        24.0 + pulse * 2.0,
                        Stroke::new(1.2, Color32::from_rgb(70, 240, 160)),
                    );

                    ui.label(RichText::new("✦").size(27.0).strong().color(Self::green()));

                    ui.label(
                        RichText::new("CYBOS")
                            .size(9.0)
                            .strong()
                            .color(Color32::from_rgb(140, 255, 190)),
                    );
                });

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(10.0);

                self.nav_button(ui, Page::Dashboard, "◉", "DASHBOARD");

                self.nav_button(ui, Page::Robot, "◎", "ROBOTCYB");

                self.nav_button(ui, Page::Farm, "◇", "CICADAFARM");

                self.nav_button(ui, Page::Chat, "⌁", "CYBCHAT");

                self.nav_button(ui, Page::Graph, "◈", "CYBERGRAPH");

                self.nav_button(ui, Page::Brain, "∴", "BRAIN");

                self.nav_button(ui, Page::Network, "◇", "NETWORK");

                self.nav_button(ui, Page::Assets, "◆", "ASSETS");

                self.nav_button(ui, Page::System, "⚙", "SYSTEM");

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.separator();

                    ui.label(RichText::new("●").size(13.0).color(Self::green()));

                    ui.label(
                        RichText::new(format!("{}", self.qwen_status))
                            .size(7.5)
                            .color(Color32::from_rgb(100, 180, 140)),
                    );

                    ui.label(
                        RichText::new(format!("v{}", APP_VERSION))
                            .size(7.0)
                            .color(Color32::GRAY),
                    );
                });
            });
    }

    fn topbar(&mut self, ui: &mut egui::Ui) {
        egui::Frame::new()
            .fill(Color32::from_rgb(2, 12, 9))
            .stroke(Stroke::new(1.0, Color32::from_rgb(25, 90, 60)))
            .corner_radius(10)
            .inner_margin(egui::Margin::same(11))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                let painter = ui.painter();

                // Subtle HUD grid.
                let grid = Color32::from_rgb(7, 35, 24);

                let mut x = rect.left();

                while x < rect.right() {
                    painter.line_segment(
                        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                        Stroke::new(0.45, grid),
                    );

                    x += 34.0;
                }

                let mut y = rect.top();

                while y < rect.bottom() {
                    painter.line_segment(
                        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                        Stroke::new(0.45, grid),
                    );

                    y += 18.0;
                }

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("CICADA")
                            .size(10.0)
                            .color(Color32::from_rgb(110, 190, 145)),
                    );

                    ui.label(
                        RichText::new("◇ cybOS")
                            .size(20.0)
                            .strong()
                            .color(Self::green()),
                    );

                    ui.add_space(12.0);

                    ui.label(RichText::new("LOCAL NODE").size(9.0).color(Color32::GRAY));

                    ui.label(
                        RichText::new("• ONLINE")
                            .size(9.0)
                            .strong()
                            .color(Color32::from_rgb(90, 255, 170)),
                    );

                    ui.add_space(18.0);

                    ui.add_sized(
                        [310.0, 28.0],
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("⌘K  SEARCH CYBOS..."),
                    );

                    if ui
                        .button("＋ EVENT")
                        .on_hover_text("Create local cybOS event")
                        .clicked()
                    {
                        self.add_event("USER", "Manual event created from cybOS");
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("QWEN  {}", self.qwen_status))
                                .size(8.0)
                                .color(Color32::from_rgb(105, 210, 150)),
                        );
                    });
                });
            });
    }

    fn card(&self, ui: &mut egui::Ui, title: &str, value: &str) {
        egui::Frame::new()
            .fill(Color32::from_rgb(3, 16, 11))
            .stroke(Stroke::new(1.0, Color32::from_rgb(24, 96, 62)))
            .corner_radius(10)
            .inner_margin(egui::Margin::same(13))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                let pulse = ((ui.input(|i| i.time) * 1.7).sin() * 0.5 + 0.5) as f32;

                ui.painter().circle_stroke(
                    rect.right_top() + Vec2::new(-9.0, 9.0),
                    3.0 + pulse,
                    Stroke::new(1.0, Color32::from_rgb(55, 190, 120)),
                );

                ui.set_min_width(145.0);

                ui.label(
                    RichText::new(value)
                        .size(26.0)
                        .strong()
                        .color(Color32::from_rgb(190, 255, 220)),
                );

                ui.label(
                    RichText::new(title)
                        .size(9.0)
                        .strong()
                        .color(Color32::from_rgb(90, 175, 125)),
                );
            });
    }

    fn dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("◎ CYBOS CORE").strong().color(Self::green()));

        ui.label(
            RichText::new("LOCAL-FIRST · CONNECTED SYSTEM")
                .small()
                .color(Color32::GRAY),
        );

        ui.add_space(10.0);

        egui::Frame::new()
            .fill(Color32::from_rgb(2, 10, 7))
            .stroke(Stroke::new(1.0, Color32::from_rgb(25, 100, 62)))
            .corner_radius(14)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                let desired = egui::vec2(ui.available_width(), 480.0);

                let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());

                let painter = ui.painter_at(rect);

                let center = rect.center();

                let nodes = [
                    ("◈", "ROBOTCYB", Page::Robot, egui::vec2(0.0, -165.0)),
                    ("Ψ", "BRAIN", Page::Brain, egui::vec2(-245.0, -65.0)),
                    ("◇", "TOKENS", Page::Assets, egui::vec2(245.0, -65.0)),
                    ("∿", "GRAPH", Page::Graph, egui::vec2(-245.0, 85.0)),
                    ("⟷", "NETWORK", Page::Network, egui::vec2(245.0, 85.0)),
                    ("⌬", "CICADAFARM", Page::Farm, egui::vec2(0.0, 170.0)),
                    // EXTRA MATHEMATICAL SYMBOLS
                    ("∆", "DELTA", Page::Dashboard, egui::vec2(-95.0, -170.0)),
                    ("√", "CORE", Page::Brain, egui::vec2(95.0, -170.0)),
                    ("∞", "MEMORY", Page::Brain, egui::vec2(-95.0, 170.0)),
                    ("⊕", "ENERGY", Page::System, egui::vec2(95.0, 170.0)),
                ];

                // -------------------------------------------------
                // CONNECTION BUS
                // -------------------------------------------------

                for (_, _, _, offset) in nodes {
                    let target = center + offset;

                    painter.line_segment(
                        [center, target],
                        Stroke::new(1.0, Color32::from_rgb(25, 90, 55)),
                    );

                    painter.circle_filled(
                        egui::pos2((center.x + target.x) * 0.5, (center.y + target.y) * 0.5),
                        2.5,
                        Color32::from_rgb(50, 180, 100),
                    );
                }

                // -------------------------------------------------
                // CENTRAL CORE
                // -------------------------------------------------

                let core_rect = egui::Rect::from_center_size(center, egui::vec2(170.0, 105.0));

                painter.rect_filled(core_rect, 14.0, Color32::from_rgb(5, 25, 16));

                painter.rect_stroke(
                    core_rect,
                    14.0,
                    Stroke::new(2.0, Color32::from_rgb(50, 180, 100)),
                    egui::StrokeKind::Outside,
                );

                painter.circle_filled(center, 13.0, Color32::from_rgb(20, 120, 70));

                painter.circle_stroke(
                    center,
                    25.0,
                    Stroke::new(1.0, Color32::from_rgb(50, 180, 100)),
                );

                painter.text(
                    egui::pos2(center.x, center.y + 33.0),
                    egui::Align2::CENTER_CENTER,
                    "CYBOS CORE",
                    egui::FontId::proportional(16.0),
                    Self::green(),
                );

                // -------------------------------------------------
                // CLICKABLE NODES
                // -------------------------------------------------

                for (symbol, label, page, offset) in nodes {
                    let pos = center + offset;

                    let node_rect = egui::Rect::from_center_size(pos, egui::vec2(150.0, 68.0));

                    let response =
                        ui.interact(node_rect, ui.id().with(label), egui::Sense::click());

                    let active = self.page == page;

                    let border = if active {
                        Color32::from_rgb(80, 220, 130)
                    } else if response.hovered() {
                        Color32::from_rgb(55, 170, 100)
                    } else {
                        Color32::from_rgb(25, 90, 55)
                    };

                    painter.rect_filled(node_rect, 10.0, Color32::from_rgb(4, 18, 12));

                    painter.rect_stroke(
                        node_rect,
                        10.0,
                        Stroke::new(if active { 2.0 } else { 1.0 }, border),
                        egui::StrokeKind::Outside,
                    );

                    painter.text(
                        egui::pos2(pos.x, pos.y - 9.0),
                        egui::Align2::CENTER_CENTER,
                        symbol,
                        egui::FontId::proportional(25.0),
                        Self::green(),
                    );

                    painter.text(
                        egui::pos2(pos.x, pos.y + 18.0),
                        egui::Align2::CENTER_CENTER,
                        label,
                        egui::FontId::proportional(11.0),
                        Color32::from_rgb(150, 200, 170),
                    );

                    if response.clicked() {
                        self.page = page;
                    }
                }

                // -------------------------------------------------
                // STATUS
                // -------------------------------------------------

                painter.text(
                    egui::pos2(rect.left() + 16.0, rect.bottom() - 18.0),
                    egui::Align2::LEFT_CENTER,
                    "• LOCAL NODE ONLINE",
                    egui::FontId::proportional(10.0),
                    Color32::from_rgb(70, 180, 110),
                );

                painter.text(
                    egui::pos2(rect.right() - 16.0, rect.bottom() - 18.0),
                    egui::Align2::RIGHT_CENTER,
                    format!("NODE {}", &self.node_id[..8.min(self.node_id.len())]),
                    egui::FontId::proportional(10.0),
                    Color32::GRAY,
                );
            });

        ui.add_space(12.0);

        egui::Grid::new("cybos_status_matrix")
            .num_columns(4)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                self.card(ui, "TEMPERATURE", "24°C");
                self.card(ui, "HIVES", "4");
                self.card(ui, "ANIMALS", "60+");
                self.card(ui, "NODE", "ONLINE");
                ui.end_row();
            });

        ui.add_space(12.0);

        self.event_list(ui, 7);
    }

    fn event_list(&self, ui: &mut egui::Ui, n: usize) {
        ui.heading("EVENTS");
        egui::ScrollArea::vertical()
            .max_height(190.0)
            .show(ui, |ui| {
                for e in self.events.iter().take(n) {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&e.time).small().color(Color32::GRAY));
                        ui.label(RichText::new(&e.kind).small().color(Self::green()));
                        ui.label(&e.text);
                    });
                }
            });
    }
    fn robot(&mut self, ui: &mut egui::Ui) {
        let neon = Color32::from_rgb(0, 255, 150);
        let dim = Color32::from_rgb(55, 145, 105);
        let panel = Color32::from_rgb(5, 18, 13);

        ui.vertical(|ui| {
            ui.label(RichText::new("◉  ROBOTCYB").size(24.0).strong().color(neon));

            ui.label(
                RichText::new("LOCAL AI AGENT · REQUEST → REASONING → RESPONSE")
                    .size(11.0)
                    .color(dim),
            );

            ui.add_space(14.0);

            // ------------------------------------------------
            // ROBOT CORE / EYE
            // ------------------------------------------------
            egui::Frame::NONE
                .fill(panel)
                .corner_radius(egui::CornerRadius::same(14))
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let available = ui.available_width();

                        let eye_size = 150.0;

                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(eye_size, eye_size),
                            egui::Sense::hover(),
                        );

                        let painter = ui.painter_at(rect);
                        let c = rect.center();

                        painter.circle_stroke(c, 62.0, Stroke::new(2.0, neon));

                        painter.circle_stroke(c, 48.0, Stroke::new(1.0, dim));

                        painter.circle_filled(
                            c,
                            27.0,
                            Color32::from_rgba_unmultiplied(0, 255, 150, 28),
                        );

                        painter.circle_stroke(c, 27.0, Stroke::new(2.0, neon));

                        painter.circle_filled(c, 10.0, neon);

                        painter.circle_filled(c + Vec2::new(-4.0, -5.0), 3.0, Color32::WHITE);

                        // eye rays
                        for k in 0..8 {
                            let a = k as f32 * std::f32::consts::TAU / 8.0;
                            let a0 = c + Vec2::angled(a) * 69.0;
                            let a1 = c + Vec2::angled(a) * 78.0;

                            painter.line_segment([a0, a1], Stroke::new(1.0, dim));
                        }

                        ui.add_space(18.0);

                        ui.vertical(|ui| {
                            ui.label(RichText::new("ROBOTCYB").size(22.0).strong().color(neon));

                            ui.label(RichText::new("LOCAL AGENT").size(11.0).strong().color(dim));

                            ui.add_space(8.0);

                            ui.label(RichText::new("• READY").size(11.0).color(neon));

                            ui.label(
                                RichText::new(format!("QWEN · {}", self.qwen_status))
                                    .size(10.0)
                                    .color(dim),
                            );

                            ui.add_space(10.0);

                            ui.label(RichText::new("REQUEST").size(10.0).strong().color(dim));

                            ui.label(
                                RichText::new("Ask the local RobotCYB agent anything.")
                                    .size(11.0)
                                    .color(Color32::from_rgb(125, 180, 150)),
                            );

                            let _ = available;
                        });
                    });
                });

            ui.add_space(12.0);

            // ------------------------------------------------
            // REQUEST
            // ------------------------------------------------
            egui::Frame::NONE
                .fill(panel)
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("REQUEST").size(11.0).strong().color(neon));

                    ui.add_space(5.0);

                    let response = ui.add(
                        egui::TextEdit::multiline(&mut self.robot_input)
                            .desired_rows(4)
                            .desired_width(f32::INFINITY)
                            .hint_text("Enter a request for RobotCYB..."),
                    );

                    ui.add_space(7.0);

                    ui.horizontal(|ui| {
                        let send = ui
                            .add(
                                egui::Button::new(
                                    RichText::new("◉  SEND REQUEST")
                                        .size(12.0)
                                        .strong()
                                        .color(neon),
                                )
                                .min_size(Vec2::new(170.0, 34.0)),
                            )
                            .clicked();

                        if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                            || send
                        {
                            let q = self.robot_input.trim().to_string();

                            if !q.is_empty() {
                                let answer = self.agent_answer(&q);

                                self.robot_output = answer.clone();

                                self.chat.push(("YOU".into(), q.clone(), true));

                                self.chat.push(("ROBOTCYB".into(), answer, false));

                                self.robot_input.clear();

                                self.add_event("ROBOT", &format!("RobotCYB processed: {}", q));
                            }
                        }

                        ui.label(RichText::new("ENTER · SEND").size(9.0).color(dim));
                    });
                });

            ui.add_space(12.0);

            // ------------------------------------------------
            // RESPONSE
            // ------------------------------------------------
            egui::Frame::NONE
                .fill(panel)
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("RESPONSE").size(11.0).strong().color(neon));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("LOCAL OUTPUT").size(9.0).color(dim));
                        });
                    });

                    ui.add_space(6.0);

                    egui::Frame::NONE
                        .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 90))
                        .corner_radius(egui::CornerRadius::same(8))
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.set_min_height(145.0);

                            ui.label(
                                RichText::new(&self.robot_output)
                                    .size(12.0)
                                    .color(Color32::from_rgb(175, 235, 205)),
                            );
                        });
                });
        });
    }
    fn farm(&mut self, ui: &mut egui::Ui) {
        if self.balance_refresh.elapsed() >= std::time::Duration::from_secs(30) {
            self.refresh_cicada_balances();
            self.balance_refresh = std::time::Instant::now();
        }

        let neon = Color32::from_rgb(0, 255, 150);
        let dim = Color32::from_rgb(55, 145, 105);
        let panel = Color32::from_rgb(5, 18, 13);
        let soft = Color32::from_rgb(170, 225, 195);

        ui.vertical(|ui| {
            ui.label(
                RichText::new("⌬  CICADAFARM")
                    .size(24.0)
                    .strong()
                    .color(neon),
            );

            ui.label(
                RichText::new(
                    "LIVING FARM · DIRECT ACCESS · $CICADAFARM COMMERCE"
                )
                .size(11.0)
                .color(dim),
            );

            ui.add_space(14.0);

            // FARM STATUS
            ui.horizontal(|ui| {
                for (symbol, title, value) in [
                    ("∿", "CHICKENS", "60+"),
                    ("∆", "GOATS", "2"),
                    ("∞", "HIVES", "4"),
                    ("◉", "LAKES", "3"),
                ] {
                    egui::Frame::NONE
                        .fill(panel)
                        .corner_radius(egui::CornerRadius::same(10))
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            ui.set_min_width(115.0);

                            ui.label(
                                RichText::new(symbol)
                                    .size(21.0)
                                    .color(neon),
                            );

                            ui.label(
                                RichText::new(title)
                                    .size(8.0)
                                    .strong()
                                    .color(dim),
                            );

                            ui.label(
                                RichText::new(value)
                                    .size(17.0)
                                    .strong()
                                    .color(soft),
                            );
                        });
                }
            });

            ui.add_space(14.0);

            // ------------------------------------------------
            // WALLET
            // ------------------------------------------------
            egui::Frame::NONE
                .fill(panel)
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("◈  CICADAFARM PAYMENT WALLET")
                                .size(12.0)
                                .strong()
                                .color(neon),
                        );

                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(
                                    RichText::new("SOLANA")
                                        .size(9.0)
                                        .strong()
                                        .color(dim),
                                );
                            },
                        );
                    });

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&self.cicada_wallet)
                                .size(11.0)
                                .monospace()
                                .color(soft),
                        );

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("⧉")
                                        .size(17.0)
                                        .color(neon),
                                )
                                .frame(false),
                            )
                            .on_hover_text("Copy wallet")
                            .clicked()
                        {
                            ui.ctx().copy_text(self.cicada_wallet.clone());
                            self.payment_status =
                                "WALLET COPIED".into();
                        }
                    });

                    ui.add_space(7.0);

                    ui.horizontal(|ui| {
                        let balance = match self.cicada_balance {
                            Some(v) => format!("{:.4}", v),
                            None => "CHECKING…".into(),
                        };

                        ui.label(
                            RichText::new(format!(
                                "$CICADAFARM BALANCE  {}",
                                balance
                            ))
                            .size(10.0)
                            .strong()
                            .color(neon),
                        );

                        ui.add_space(20.0);

                        let sol = match self.sol_balance {
                            Some(v) => format!("{:.6} SOL", v),
                            None => "SOL BALANCE · CHECKING…".into(),
                        };

                        ui.label(
                            RichText::new(sol)
                                .size(10.0)
                                .color(dim),
                        );
                    });
                });

            ui.add_space(12.0);

            // ------------------------------------------------
            // PRODUCTS
            // ------------------------------------------------
            ui.horizontal(|ui| {
                // HONEY
                egui::Frame::NONE
                    .fill(panel)
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.set_min_width(310.0);

                        ui.label(
                            RichText::new("⊙  FARM HONEY")
                                .size(18.0)
                                .strong()
                                .color(neon),
                        );

                        ui.label(
                            RichText::new("1 KG")
                                .size(12.0)
                                .strong()
                                .color(soft),
                        );

                        ui.add_space(8.0);

                        ui.label(
                            RichText::new("40,000,000 $CICADAFARM")
                                .size(17.0)
                                .strong()
                                .color(neon),
                        );

                        ui.add_space(5.0);

                        ui.label(
                            RichText::new("CURRENTLY UNAVAILABLE")
                                .size(10.0)
                                .strong()
                                .color(Color32::from_rgb(230, 150, 80)),
                        );

                        ui.add_space(9.0);

                        ui.add_enabled(
                            false,
                            egui::Button::new(
                                RichText::new("⊙  BUY FOR $CICADAFARM")
                                    .size(11.0)
                                    .strong(),
                            )
                            .min_size(Vec2::new(250.0, 36.0)),
                        );

                        ui.label(
                            RichText::new("Payment disabled while stock = 0")
                                .size(9.0)
                                .color(dim),
                        );
                    });

                ui.add_space(12.0);

                // EGGS
                egui::Frame::NONE
                    .fill(panel)
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.set_min_width(310.0);

                        ui.label(
                            RichText::new("○  FARM EGGS")
                                .size(18.0)
                                .strong()
                                .color(neon),
                        );

                        ui.label(
                            RichText::new("50 EGGS")
                                .size(12.0)
                                .strong()
                                .color(soft),
                        );

                        ui.add_space(8.0);

                        ui.label(
                            RichText::new("10,000,000 $CICADAFARM")
                                .size(17.0)
                                .strong()
                                .color(neon),
                        );

                        ui.add_space(5.0);

                        ui.label(
                            RichText::new("PICKUP ONLY · CICADAFARM")
                                .size(10.0)
                                .strong()
                                .color(soft),
                        );

                        ui.label(
                            RichText::new("NO DELIVERY")
                                .size(9.0)
                                .color(dim),
                        );

                        ui.add_space(9.0);

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("○  BUY FOR $CICADAFARM")
                                        .size(11.0)
                                        .strong()
                                        .color(neon),
                                )
                                .min_size(Vec2::new(250.0, 36.0)),
                            )
                            .clicked()
                        {
                            self.payment_uri = format!(
                                "solana:{}?amount=10000000&spl-token={}",
                                self.cicada_wallet,
                                CICADAFARM_MINT
                            );

                            self.payment_status =
                                "EGGS PAYMENT REQUEST READY · PICKUP ONLY".into();

                            self.add_event(
                                "FARM",
                                "Eggs payment request created: 50 eggs",
                            );
                            self.notify("EGGS PAYMENT REQUEST READY");
                        }
                    });
            });

            ui.add_space(12.0);

            // PAYMENT REQUEST
            egui::Frame::NONE
                .fill(panel)
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("↗  PAYMENT REQUEST")
                                .size(11.0)
                                .strong()
                                .color(neon),
                        );

                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(
                                    RichText::new(&self.payment_status)
                                        .size(9.0)
                                        .color(dim),
                                );
                            },
                        );
                    });

                    ui.add_space(7.0);

                    if self.payment_uri.is_empty() {
                        ui.label(
                            RichText::new(
                                "Select a product to generate a Solana payment request."
                            )
                            .size(10.0)
                            .color(dim),
                        );
                    } else {
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(
                                    &mut self.payment_uri,
                                )
                                .desired_width(f32::INFINITY),
                            );

                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("⧉")
                                            .size(17.0)
                                            .color(neon),
                                    )
                                    .frame(false),
                                )
                                .on_hover_text("Copy payment request")
                                .clicked()
                            {
                                ui.ctx().copy_text(
                                    self.payment_uri.clone(),
                                );

                                self.payment_status =
                                    "PAYMENT REQUEST COPIED".into();
                            }
                        });
                    }

                    ui.add_space(7.0);

                    ui.label(
                        RichText::new(
                            "Payment is a request only. cybOS does not claim payment confirmation until the transaction is verified."
                        )
                        .size(9.0)
                        .color(dim),
                    );
                });
        });
    }
    fn chat_page(&mut self, ui: &mut egui::Ui) {
        let neon = Color32::from_rgb(0, 255, 150);
        let dim = Color32::from_rgb(55, 145, 105);
        let panel = Color32::from_rgb(5, 18, 13);

        ui.vertical(|ui| {
            ui.label(RichText::new("∴  CYBCHAT").size(24.0).strong().color(neon));

            ui.label(
                RichText::new("MATHEMATICAL CHANNEL · LOCAL-FIRST · PEERS · LAN · P2P · NOSTR")
                    .size(11.0)
                    .color(dim),
            );

            ui.add_space(14.0);

            ui.horizontal(|ui| {
                // PEERS
                egui::Frame::NONE
                    .fill(panel)
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_min_width(190.0);

                        ui.label(RichText::new("∴ PEERS").size(11.0).strong().color(neon));

                        ui.add_space(8.0);

                        for (symbol, name, status) in [
                            ("◉", "CICADA NODE", "ONLINE"),
                            ("◉", "ROBOTCYB", "LOCAL"),
                            ("⌬", "FARM NODE", "LOCAL"),
                            ("Ψ", "BRAIN", "LOCAL"),
                        ] {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(symbol).size(16.0).color(neon));

                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(name)
                                            .size(10.0)
                                            .strong()
                                            .color(Color32::from_rgb(175, 235, 205)),
                                    );

                                    ui.label(RichText::new(status).size(8.0).color(dim));
                                });
                            });

                            ui.add_space(7.0);
                        }
                    });

                ui.add_space(10.0);

                // TRANSPORT
                egui::Frame::NONE
                    .fill(panel)
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_min_width(190.0);

                        ui.label(RichText::new("⟷ TRANSPORT").size(11.0).strong().color(neon));

                        ui.add_space(8.0);

                        for (symbol, name, status) in [
                            ("⌂", "LOCAL", "READY"),
                            ("◌", "BLE", "READY"),
                            ("⟷", "LAN", "READY"),
                            ("↔", "P2P", "READY"),
                            ("∴", "NOSTR", "READY"),
                        ] {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(symbol).size(15.0).color(neon));
                                ui.label(
                                    RichText::new(format!("{} · {}", name, status))
                                        .size(9.0)
                                        .color(Color32::from_rgb(150, 215, 180)),
                                );
                            });
                        }
                    });
            });

            ui.add_space(12.0);

            // ------------------------------------------------
            // RESPONSE / HISTORY
            // ------------------------------------------------
            egui::Frame::NONE
                .fill(panel)
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("RESPONSE / MESSAGE STREAM")
                            .size(11.0)
                            .strong()
                            .color(neon),
                    );

                    ui.add_space(7.0);

                    egui::ScrollArea::vertical()
                        .max_height(230.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (who, msg, mine) in self.chat.iter().rev().take(30) {
                                let color = if *mine {
                                    neon
                                } else {
                                    Color32::from_rgb(150, 215, 180)
                                };

                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        RichText::new(format!("{}  ", who))
                                            .size(9.0)
                                            .strong()
                                            .color(color),
                                    );

                                    ui.label(
                                        RichText::new(msg)
                                            .size(11.0)
                                            .color(Color32::from_rgb(190, 230, 210)),
                                    );
                                });

                                ui.add_space(5.0);
                            }
                        });
                });

            ui.add_space(12.0);

            // ------------------------------------------------
            // INPUT
            // ------------------------------------------------
            egui::Frame::NONE
                .fill(panel)
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("INPUT").size(11.0).strong().color(neon));

                    ui.add_space(5.0);

                    let response = ui.add(
                        egui::TextEdit::multiline(&mut self.chat_input)
                            .desired_rows(3)
                            .desired_width(f32::INFINITY)
                            .hint_text("Write a CybChat message..."),
                    );

                    ui.add_space(7.0);

                    ui.horizontal(|ui| {
                        let send = ui
                            .add(
                                egui::Button::new(
                                    RichText::new("∴  SEND MESSAGE")
                                        .size(12.0)
                                        .strong()
                                        .color(neon),
                                )
                                .min_size(Vec2::new(170.0, 34.0)),
                            )
                            .clicked();

                        if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                            || send
                        {
                            let t = self.chat_input.trim().to_string();

                            if !t.is_empty() {
                                self.chat_output = t.clone();
                                self.chat.push(("YOU".into(), t.clone(), true));
                                let reply = self.agent_answer(&t);
                                self.chat.push(("ROBOTCYB".into(), reply, false));
                                self.chat_input.clear();
                                self.add_event("CHAT", &format!("Local message sent: {}", t));
                                self.notify("MESSAGE SENT ON LOCAL CHANNEL");
                            }
                        }

                        ui.label(
                            RichText::new("LOCAL-FIRST · NO FAKE NETWORK")
                                .size(9.0)
                                .color(dim),
                        );
                    });
                });
        });
    }
    fn graph(&mut self, ui: &mut egui::Ui) {
        ui.heading("CYBERGRAPH");
        ui.label("Local-first knowledge topology · nodes · links · events · queries");
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("⟳").size(20.0).color(Self::green()))
                        .frame(false),
                )
                .clicked()
            {
                self.graph_zoom = 1.0;
                self.graph_pan = Vec2::ZERO;
            }

            ui.add(egui::Slider::new(&mut self.graph_zoom, 0.4..=2.5).text("zoom"));

            let pulse = ((ui.input(|i| i.time) * 2.5).sin() * 0.5 + 0.5) as f32;

            ui.separator();

            ui.label(
                RichText::new("• LIVE GRAPH")
                    .strong()
                    .color(Color32::from_rgb(
                        70 + (pulse * 80.0) as u8,
                        220,
                        120 + (pulse * 60.0) as u8,
                    )),
            );

            ui.separator();

            ui.label(RichText::new(format!("NODES {}", self.nodes.len())).strong());

            ui.label(RichText::new(format!("LINKS {}", self.links.len())).strong());

            ui.label(RichText::new(format!(
                "DENSITY {:.2}",
                if self.nodes.len() > 1 {
                    self.links.len() as f32 / self.nodes.len() as f32
                } else {
                    0.0
                }
            )));
        });
        let desired = Vec2::new(ui.available_width(), ui.available_height().max(460.0));
        let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());
        if resp.dragged() {
            self.graph_pan += resp.drag_delta();
        }
        let painter = ui.painter_at(rect);
        let center = rect.center() + self.graph_pan;
        let pos: HashMap<String, egui::Pos2> = self
            .nodes
            .iter()
            .map(|n| (n.id.clone(), center + Vec2::new(n.x, n.y) * self.graph_zoom))
            .collect();
        for l in &self.links {
            if let (Some(a), Some(b)) = (pos.get(&l.from), pos.get(&l.to)) {
                painter.line_segment([*a, *b], Stroke::new(1.2, Color32::from_rgb(45, 110, 75)));
            }
        }
        for n in &self.nodes {
            if let Some(p) = pos.get(&n.id) {
                let selected = self.selected_node.as_deref() == Some(&n.id);

                let degree = self
                    .links
                    .iter()
                    .filter(|l| l.from == n.id || l.to == n.id)
                    .count();

                let growth_radius = (degree as f32 * 2.0).min(10.0);

                let radius = if selected {
                    26.0 + growth_radius
                } else {
                    21.0 + growth_radius
                };

                let pulse = ((ui.input(|i| i.time) * 2.5).sin() * 0.5 + 0.5) as f32;

                if degree > 0 {
                    painter.circle_stroke(
                        *p,
                        radius + 4.0 + pulse * 3.0,
                        Stroke::new(1.0 + pulse, Color32::from_rgb(35, 130, 75)),
                    );
                }

                painter.circle_filled(
                    *p,
                    radius,
                    if selected {
                        Self::green()
                    } else {
                        Color32::from_rgb(12, 50, 30)
                    },
                );
                painter.text(
                    *p,
                    egui::Align2::CENTER_CENTER,
                    &n.label,
                    egui::FontId::proportional(10.0),
                    if selected {
                        Color32::BLACK
                    } else {
                        Color32::WHITE
                    },
                );
            }
        }
        if resp.clicked() {
            if let Some(pointer) = resp.interact_pointer_pos() {
                self.selected_node = self
                    .nodes
                    .iter()
                    .find(|n| {
                        center.distance(pointer + Vec2::ZERO) < f32::MAX
                            && (center + Vec2::new(n.x, n.y) * self.graph_zoom).distance(pointer)
                                < 32.0
                    })
                    .map(|n| n.id.clone());
            }
        }
        if let Some(id) = &self.selected_node {
            if let Some(n) = self.nodes.iter().find(|n| &n.id == id) {
                egui::Area::new("inspector".into())
                    .fixed_pos(rect.right_top() + Vec2::new(-250.0, 10.0))
                    .show(ui.ctx(), |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.label(RichText::new(&n.label).strong());
                            ui.label(format!("type: {}", n.kind));
                            ui.label(format!("id: {}", n.id));
                            ui.separator();
                            ui.label("Relationships");
                            for l in &self.links {
                                if l.from == n.id || l.to == n.id {
                                    ui.label(format!(
                                        "{} → {}",
                                        l.relation,
                                        if l.from == n.id { &l.to } else { &l.from }
                                    ));
                                }
                            }
                        });
                    });
            }
        }
    }
    fn brain(&mut self, ui: &mut egui::Ui) {
        let neon = Self::neon();
        let dim = Color32::from_rgb(55, 145, 105);
        let memories = self.store.memories();
        ui.label(
            RichText::new("MEMORY · GRAPH · LOCAL AI · NO FAKE CLOUD BRAIN")
                .size(11.0)
                .color(dim),
        );
        ui.add_space(10.0);
        egui::Grid::new("brain").num_columns(4).spacing([10.0, 8.0]).show(ui, |ui| {
            self.card(ui, "MEMORY", &format!("{}", memories.len()));
            self.card(ui, "GRAPH", &format!("{}N", self.nodes.len()));
            self.card(ui, "QWEN", if self.qwen_status.contains("ONLINE") { "ONLINE" } else { "STANDBY" });
            self.card(ui, "EVENTS", &format!("{}", self.events.len()));
            ui.end_row();
        });
        ui.add_space(12.0);
        egui::Frame::new()
            .fill(Color32::from_rgb(5, 18, 13))
            .stroke(Stroke::new(1.0, Color32::from_rgb(28, 80, 52)))
            .corner_radius(12)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.label(RichText::new("WRITE TO LOCAL MEMORY").size(11.0).strong().color(neon));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.remember_note)
                            .hint_text("A fact cybOS should keep…")
                            .desired_width(ui.available_width() - 130.0),
                    );
                    if ui
                        .add(egui::Button::new(RichText::new("STORE").strong().color(neon)).min_size(Vec2::new(110.0, 30.0)))
                        .clicked()
                    {
                        let note = self.remember_note.trim().to_string();
                        if !note.is_empty() {
                            self.store.add_memory(&Memory {
                                id: Uuid::new_v4().to_string(),
                                time: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                                text: note.clone(),
                                source: "BRAIN".into(),
                                importance: 0.7,
                            });
                            self.remember_note.clear();
                            self.add_event("BRAIN", format!("Stored memory: {}", note));
                            self.notify("MEMORY STORED");
                        }
                    }
                });
            });
        ui.add_space(10.0);
        ui.label(RichText::new("RECENT MEMORY").size(11.0).strong().color(neon));
        egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
            if memories.is_empty() {
                ui.label(RichText::new("No stored memories yet. Write one above.").color(dim));
            }
            for m in memories.iter().take(40) {
                egui::Frame::new()
                    .fill(Color32::from_rgb(3, 14, 10))
                    .inner_margin(egui::Margin::same(8))
                    .corner_radius(8)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&m.time).small().color(Color32::GRAY));
                            ui.label(RichText::new(&m.source).small().color(neon));
                        });
                        ui.label(RichText::new(&m.text).size(12.0).color(Color32::from_rgb(190, 230, 210)));
                    });
                ui.add_space(4.0);
            }
        });
    }
    fn network(&mut self, ui: &mut egui::Ui) {
        let neon = Self::neon();
        let dim = Color32::from_rgb(55, 145, 105);
        ui.label(
            RichText::new("ONE IDENTITY · TRANSPORT ADAPTERS ARE PRESENT, NOT LIVE")
                .size(11.0)
                .color(dim),
        );
        ui.add_space(10.0);
        egui::Frame::new()
            .fill(Color32::from_rgb(5, 18, 13))
            .stroke(Stroke::new(1.0, Color32::from_rgb(28, 80, 52)))
            .corner_radius(12)
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("NODE  {}", self.node_id)).strong().color(neon));
                    if ui.button("COPY ID").clicked() {
                        ui.ctx().copy_text(self.node_id.clone());
                        self.notify("NODE ID COPIED");
                    }
                    if ui.button("SCAN LOCAL").clicked() {
                        self.last_scan = Some(Instant::now());
                        self.add_event(
                            "NETWORK",
                            "Local scan: no live BLE/LAN/P2P peers. Adapters remain ready, not connected.",
                        );
                        self.notify("SCAN COMPLETE · NO LIVE PEERS");
                    }
                });
                if let Some(t) = self.last_scan {
                    ui.label(
                        RichText::new(format!("LAST SCAN {}s AGO", t.elapsed().as_secs()))
                            .small()
                            .color(dim),
                    );
                }
            });
        ui.add_space(10.0);
        for (name, status, live) in [
            ("LOCAL LOOPBACK", "READY", true),
            ("BLUETOOTH MESH", "ADAPTER ONLY · NOT CONNECTED", false),
            ("LAN", "ADAPTER ONLY · NOT CONNECTED", false),
            ("P2P", "ADAPTER ONLY · NOT CONNECTED", false),
            ("NOSTR FALLBACK", "AVAILABLE · NOT CONNECTED", false),
        ] {
            egui::Frame::new()
                .fill(Color32::from_rgb(4, 16, 11))
                .stroke(Stroke::new(1.0, if live { Color32::from_rgb(30, 110, 70) } else { Color32::from_rgb(18, 48, 34) }))
                .corner_radius(9)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::hover());
                        Self::paint_icon(ui.painter(), rect.center(), Icon::Network, if live { neon } else { dim }, 16.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(name).size(12.0).strong().color(if live { neon } else { Color32::from_rgb(160, 200, 175) }));
                            ui.label(RichText::new(status).size(10.0).color(dim));
                        });
                    });
                });
            ui.add_space(6.0);
        }
    }
    fn cameras(&mut self, ui: &mut egui::Ui) {
        let neon = Self::neon();
        let dim = Color32::from_rgb(90, 170, 120);
        let zones = ["FARM YARD", "APIARY", "CHICKENS", "GOATS", "FOREST EDGE"];
        ui.label(
            RichText::new("LOCAL PREVIEW · NO FAKE RTSP STREAM · READY FOR A REAL CAMERA LATER")
                .size(11.0)
                .color(dim),
        );
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            for (i, name) in zones.iter().enumerate() {
                let active = self.camera_zone == i;
                let btn = egui::Button::new(
                    RichText::new(*name)
                        .size(11.0)
                        .strong()
                        .color(if active { Color32::BLACK } else { neon }),
                )
                .fill(if active { neon } else { Color32::from_rgb(6, 22, 15) })
                .min_size(Vec2::new(108.0, 34.0));
                if ui.add(btn).clicked() {
                    self.camera_zone = i;
                    self.add_event("CAMERA", format!("Selected camera zone: {}", name));
                    self.notify(format!("{} SELECTED", name));
                }
            }
        });
        ui.add_space(10.0);
        egui::Frame::new()
            .fill(Color32::from_rgb(1, 8, 6))
            .stroke(Stroke::new(1.0, Color32::from_rgb(20, 90, 55)))
            .corner_radius(16)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                let h = ui.available_height().max(360.0);
                let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h.min(420.0)), egui::Sense::hover());
                let painter = ui.painter_at(rect);
                let t = ui.input(|i| i.time) as f32;
                painter.rect_filled(rect, 12.0, Color32::from_rgb(2, 10, 8));
                for i in 0..18 {
                    let y = rect.top() + ((t * 28.0 + i as f32 * 22.0) % rect.height());
                    painter.line_segment(
                        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                        Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 255, 150, 18)),
                    );
                }
                Self::paint_icon(&painter, rect.center() + egui::vec2(0.0, -28.0), Icon::Camera, neon, 54.0);
                painter.text(
                    rect.center() + egui::vec2(0.0, 22.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{} · SIGNAL ABSENT", zones[self.camera_zone]),
                    egui::FontId::proportional(18.0),
                    neon,
                );
                painter.text(
                    rect.center() + egui::vec2(0.0, 48.0),
                    egui::Align2::CENTER_CENTER,
                    "Telemetry overlay only. Connect a real camera when the farm feed exists.",
                    egui::FontId::proportional(11.0),
                    Color32::from_rgb(90, 150, 115),
                );
                painter.text(
                    rect.left_top() + egui::vec2(16.0, 16.0),
                    egui::Align2::LEFT_TOP,
                    format!("CAM-{:02}  {}°C  NODE {}", self.camera_zone + 1, self.temperature as i32, &self.node_id[..8.min(self.node_id.len())]),
                    egui::FontId::monospace(11.0),
                    dim,
                );
            });
    }

    fn assets(&mut self, ui: &mut egui::Ui) {
        self.token_matrix(ui);

        ui.add_space(18.0);
        ui.separator();
        ui.add_space(10.0);

        ui.heading(
            RichText::new("◇ PUBLIC IDENTIFIERS")
                .strong()
                .color(Self::green()),
        );

        self.asset(ui, "$CICADAFARM", CICADAFARM_MINT, "PHYSICAL WORLD");
        self.asset(ui, "$ROBOTCYB", ROBOTCYB_MINT, "DIGITAL WORLD");
    }
    fn asset(&mut self, ui: &mut egui::Ui, name: &str, mint: &str, desc: &str) {
        egui::Frame::new()
            .fill(Color32::from_rgb(7, 25, 16))
            .stroke(Stroke::new(1.0, Color32::from_rgb(28, 74, 48)))
            .corner_radius(9)
            .inner_margin(egui::Margin::same(13))
            .show(ui, |ui| {
                ui.label(RichText::new(name).size(20.0).strong());
                ui.label(desc);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(mint).small().color(Color32::GRAY));
                    if ui.button("COPY").clicked() {
                        ui.ctx().copy_text(mint.into());
                        self.notify(format!("{} MINT COPIED", name));
                    }
                });
            });
        ui.add_space(8.0);
    }
    fn system(&mut self, ui: &mut egui::Ui) {
        let neon = Self::neon();
        let dim = Color32::from_rgb(55, 145, 105);
        ui.label(RichText::new("NATIVE DESKTOP RUNTIME · NO BROWSER SHELL").size(11.0).color(dim));
        ui.add_space(10.0);
        egui::Grid::new("sys").num_columns(3).spacing([10.0, 8.0]).show(ui, |ui| {
            self.card(ui, "VERSION", APP_VERSION);
            self.card(ui, "BATTERY", &format!("{:.0}%", self.battery));
            self.card(ui, "TEMP", &format!("{:.1}°C", self.temperature));
            ui.end_row();
        });
        ui.add_space(12.0);
        for (k, v) in [
            ("NODE", self.node_id.as_str()),
            ("DATABASE", self.store.path.to_str().unwrap_or("—")),
            ("RENDERER", "egui / eframe"),
            ("QWEN", self.qwen_status.as_str()),
            ("KEYS", "not stored"),
        ] {
            ui.horizontal(|ui| {
                ui.label(RichText::new(k).size(11.0).strong().color(neon).extra_letter_spacing(1.2));
                ui.label(RichText::new(v).size(12.0).color(Color32::from_rgb(180, 225, 200)));
            });
            ui.add_space(4.0);
        }
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button("COPY NODE ID").clicked() {
                ui.ctx().copy_text(self.node_id.clone());
                self.notify("NODE ID COPIED");
            }
            if ui.button("OPEN DATA FOLDER").clicked() {
                if let Some(parent) = self.store.path.parent() {
                    let _ = Command::new("open").arg(parent).spawn();
                    self.notify("OPENED LOCAL DATA FOLDER");
                }
            }
            if ui.button("WRITE SYSTEM EVENT").clicked() {
                self.add_event("SYSTEM", "Manual system pulse from cybOS");
                self.notify("SYSTEM EVENT WRITTEN");
            }
        });
    }
}
impl eframe::App for CybOs {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ensure_qwen();
        self.neon_theme(ui.ctx());

        let neon = Self::neon();
        let dim = Color32::from_rgb(0, 100, 65);
        let bg = Color32::from_rgb(2, 8, 6);
        let panel = Color32::from_rgb(4, 15, 10);

        if ui.input(|i| i.key_pressed(egui::Key::K) && (i.modifiers.command || i.modifiers.mac_cmd))
        {
            self.search_focus = true;
        }

        {
            let painter = ui.ctx().layer_painter(egui::LayerId::background());
            let rect = ui.max_rect();
            painter.rect_filled(rect, 0.0, bg);
            for i in 0..180u32 {
                let x = ((i.wrapping_mul(97) % 1000) as f32) / 1000.0;
                let y = ((i.wrapping_mul(193).wrapping_add(37) % 1000) as f32) / 1000.0;
                let px = rect.left() + rect.width() * x;
                let py = rect.top() + rect.height() * y;
                let pulse = (((i as f32) * 0.73).sin() * 0.5 + 0.5) as u8;
                let alpha = 35u8.saturating_add(pulse / 2);
                painter.circle_filled(
                    egui::pos2(px, py),
                    if i % 17 == 0 { 1.35 } else { 0.65 },
                    Color32::from_rgba_unmultiplied(80, 255, 170, alpha),
                );
            }
        }

        egui::Panel::top("cybos_header")
            .exact_size(64.0)
            .frame(egui::Frame::NONE.fill(bg).inner_margin(egui::Margin::symmetric(16, 10)))
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("CICADAFARM").size(13.0).strong().color(neon));
                    ui.add_space(10.0);
                    ui.label(RichText::new("C Y B O S").size(20.0).strong().color(neon));
                    ui.add_space(10.0);
                    ui.label(RichText::new("ROBOTCYB").size(13.0).strong().color(neon));
                    ui.add_space(18.0);

                    let (search_icon, _) =
                        ui.allocate_exact_size(Vec2::splat(22.0), egui::Sense::hover());
                    Self::paint_icon(ui.painter(), search_icon.center(), Icon::Search, neon, 16.0);

                    let search = ui.add_sized(
                        [280.0, 30.0],
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("⌘K  search pages, tokens, cameras…")
                            .id(egui::Id::new("cybos_search")),
                    );
                    if self.search_focus {
                        search.request_focus();
                        self.search_focus = false;
                    }
                    if (search.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        || ui
                            .add(egui::Button::new(RichText::new("GO").strong().color(neon)))
                            .clicked()
                    {
                        self.apply_search();
                    }

                    let (plus_icon, plus_resp) =
                        ui.allocate_exact_size(Vec2::splat(22.0), egui::Sense::click());
                    Self::paint_icon(ui.painter(), plus_icon.center(), Icon::Plus, neon, 16.0);
                    if plus_resp.on_hover_text("Write a local event").clicked()
                        || ui
                            .add(egui::Button::new(RichText::new("+ EVENT").strong().color(neon)))
                            .on_hover_text("Write a local event")
                            .clicked()
                    {
                        self.add_event("USER", "Manual event created from cybOS");
                        self.notify("EVENT WRITTEN");
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("• {}", self.qwen_status))
                                .size(10.0)
                                .color(neon),
                        );
                    });
                });
            });

        egui::Panel::left("cybos_left")
            .exact_size(78.0)
            .frame(egui::Frame::NONE.fill(panel))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    let items = [
                        (Page::Dashboard, "DASHBOARD"),
                        (Page::Graph, "GRAPH"),
                        (Page::Network, "NETWORK"),
                        (Page::Brain, "BRAIN"),
                        (Page::Farm, "FARM"),
                        (Page::Robot, "ROBOT"),
                        (Page::Cameras, "CAMERAS"),
                        (Page::System, "SYSTEM"),
                        (Page::Assets, "TOKENS"),
                        (Page::Chat, "CYBCHAT"),
                    ];
                    for (page, tooltip) in items {
                        if self.rail_icon(ui, page.icon(), tooltip, self.page == page) {
                            self.go(page);
                        }
                    }
                });
            });

        egui::Panel::right("cybos_right")
            .exact_size(78.0)
            .frame(egui::Frame::NONE.fill(panel))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    let actions = [
                        (Icon::Node, "LIVE NODE", Page::Dashboard),
                        (Icon::Assets, "TOKENS", Page::Assets),
                        (Icon::Energy, "ENERGY", Page::System),
                        (Icon::Environment, "ENVIRONMENT", Page::Farm),
                        (Icon::Activity, "ACTIVITY", Page::Dashboard),
                    ];
                    for (icon, tooltip, page) in actions {
                        if self.rail_icon(ui, icon, tooltip, self.page == page) {
                            self.go(page);
                            if icon == Icon::Activity {
                                self.notify("ACTIVITY LOG ON DASHBOARD");
                            }
                        }
                    }
                });
            });

        egui::Panel::bottom("cybos_bottom")
            .exact_size(40.0)
            .frame(egui::Frame::NONE.fill(bg))
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    let bus = [
                        ("LIVE NODE", Page::Dashboard),
                        ("QWEN", Page::Brain),
                        ("MEMORY", Page::Brain),
                        ("GRAPH", Page::Graph),
                        ("FARM", Page::Farm),
                        ("ENERGY", Page::System),
                        ("NETWORK", Page::Network),
                        ("CHAT", Page::Chat),
                    ];
                    for (name, page) in bus {
                        let active = self.page == page;
                        let text = RichText::new(name).size(11.0).strong().color(if active {
                            neon
                        } else {
                            dim
                        });
                        if ui.add(egui::Button::new(text).frame(false)).clicked() {
                            self.go(page);
                        }
                        ui.label(RichText::new("·").size(12.0).color(dim));
                    }
                    ui.label(
                        RichText::new(format!("v{}", APP_VERSION))
                            .size(10.0)
                            .color(dim),
                    );
                });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(2, 8, 6, 238))
                    .inner_margin(egui::Margin::symmetric(18, 14)),
            )
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::hover());
                        Self::paint_icon(ui.painter(), icon_rect.center(), self.page.icon(), neon, 18.0);
                        ui.label(
                            RichText::new(self.page.title())
                                .size(18.0)
                                .strong()
                                .color(neon),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("• {}", self.status))
                                    .size(11.0)
                                    .color(neon),
                            );
                        });
                    });
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);
                    match self.page {
                        Page::Graph => self.graph(ui),
                        Page::Dashboard => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.dashboard(ui));
                        }
                        Page::Robot => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.robot(ui));
                        }
                        Page::Farm => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.farm(ui));
                        }
                        Page::Chat => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.chat_page(ui));
                        }
                        Page::Brain => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.brain(ui));
                        }
                        Page::Network => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.network(ui));
                        }
                        Page::Assets => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.assets(ui));
                        }
                        Page::System => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.system(ui));
                        }
                        Page::Cameras => {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.cameras(ui));
                        }
                    }
                });
            });

        if let Some((msg, started)) = self.toast.clone() {
            if started.elapsed() < Duration::from_secs(3) {
                egui::Area::new("cybos_toast".into())
                    .anchor(egui::Align2::RIGHT_TOP, [-28.0, 78.0])
                    .show(ui.ctx(), |ui| {
                        egui::Frame::new()
                            .fill(Color32::from_rgb(6, 28, 18))
                            .stroke(Stroke::new(1.0, neon))
                            .corner_radius(10)
                            .inner_margin(egui::Margin::symmetric(14, 10))
                            .show(ui, |ui| {
                                ui.label(RichText::new(msg).size(12.0).strong().color(neon));
                            });
                    });
            } else {
                self.toast = None;
            }
        }

        ui.ctx().request_repaint_after(Duration::from_millis(80));
    }
}

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("cybOS — CicadaFarm + RobotCYB")
            .with_inner_size([1320.0, 840.0])
            .with_min_inner_size([1000.0, 680.0]),
        ..Default::default()
    };
    eframe::run_native(
        "cybOS",
        opts,
        Box::new(|_cc| Ok(Box::new(CybOs::default()))),
    )
}
