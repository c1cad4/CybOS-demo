#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod store;

mod brain;
mod ai;
mod network;
mod graph;
mod farm;
mod robot;
mod chat;
mod cameras;
mod assets;
mod ui;

use models::{
    Event,
    GraphLink,
    GraphNode,
    TokenMarket,
};
use store::Store;
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use std::{process::Child, 
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
