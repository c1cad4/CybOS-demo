//! cybOS application core.
//!
//! Top-level application state, navigation and initialization.

use crate::models::{Event, GraphLink, GraphNode, TokenMarket};
use crate::{CICADAFARM_MINT, ROBOTCYB_MINT, TOKEN_MARKET_CACHE};
use crate::store::Store;

use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use std::process::Child;
use std::time::Instant;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
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
pub(crate) enum Icon {
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
    pub(crate) fn title(self) -> &'static str {
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

    pub(crate) fn icon(self) -> Icon {
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

    pub(crate) fn matches_query(self, q: &str) -> bool {
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
    pub(crate) store: Store,
    pub(crate) page: Page,
    pub(crate) search: String,
    pub(crate) robot_input: String,
    pub(crate) robot_output: String,
    pub(crate) chat_input: String,
    pub(crate) chat_output: String,
    pub(crate) chat: Vec<(String, String, bool)>,
    // CicadaFarm commerce
    pub(crate) cicada_wallet: String,
    pub(crate) cicada_balance: Option<f64>,
    pub(crate) sol_balance: Option<f64>,
    pub(crate) payment_uri: String,
    pub(crate) payment_status: String,
    pub(crate) balance_refresh: std::time::Instant,
    pub(crate) events: Vec<Event>,
    pub(crate) nodes: Vec<GraphNode>,
    pub(crate) links: Vec<GraphLink>,
    pub(crate) graph_zoom: f32,
    pub(crate) graph_pan: Vec2,
    pub(crate) selected_node: Option<String>,
    pub(crate) temperature: f32,
    pub(crate) battery: f32,
    pub(crate) node_id: String,
    pub(crate) status: String,
    pub(crate) qwen_child: Option<Child>,
    pub(crate) qwen_status: String,
    pub(crate) toast: Option<(String, Instant)>,
    pub(crate) camera_zone: usize,
    pub(crate) search_focus: bool,
    pub(crate) last_scan: Option<Instant>,
    pub(crate) remember_note: String,
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

    pub(crate) fn refresh_cicada_balances(&mut self) {
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


    fn persist_current_graph(&self) {
        for node in &self.nodes {
            self.store.save_graph_node(node);
        }

        for link in &self.links {
            self.store.save_graph_link(link);
        }
    }


    pub(crate) fn notify(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), Instant::now()));
        self.status = "LOCAL-FIRST · READY".into();
    }

    pub(crate) fn go(&mut self, page: Page) {
        if self.page != page {
            self.page = page;
            self.notify(format!("OPENED {}", page.title()));
        }
    }

    pub(crate) fn apply_search(&mut self) {
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


    pub(crate) fn add_event(&mut self, kind: &str, text: impl Into<String>) {
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


}
