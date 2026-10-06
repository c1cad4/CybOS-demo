#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Local;
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use uuid::Uuid;

use serde_json::json;

const CICADAFARM_MINT: &str = "9QLCEL7Xo9VTwgBeAYU1PWX7JJ8joKxQCYw3msjUpump";
const ROBOTCYB_MINT: &str = "8WZiguAp8NyFnwm8Z97k6sCbSRCWaW1YYXKeCTpupump";

#[derive(Clone, Debug, Default)]
struct TokenMarket {
    name: String,
    mint: String,
    price: Option<f64>,
    price_change_24h: Option<f64>,
    volume_24h: Option<f64>,
    liquidity: Option<f64>,
    market_cap: Option<f64>,
    candles: Vec<(i64, f64)>,
    pool: Option<String>,
}

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

#[derive(Clone, Serialize, Deserialize)]
struct Event {
    id: String,
    time: String,
    kind: String,
    text: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct GraphNode {
    id: String,
    label: String,
    kind: String,
    x: f32,
    y: f32,
}
#[derive(Clone, Serialize, Deserialize)]
struct GraphLink {
    from: String,
    to: String,
    relation: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct Memory {
    id: String,
    time: String,
    text: String,
    source: String,
    importance: f32,
}

#[derive(Clone, Serialize, Deserialize)]
struct LearningFact {
    subject: String,
    subject_label: String,
    subject_kind: String,
    relation: String,
    object: String,
    object_label: String,
    object_kind: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct LearningResponse {
    facts: Vec<LearningFact>,
}

struct Store {
    path: PathBuf,
    conn: Connection,
}
impl Store {
    fn open() -> Self {
        let base = dirs_fallback();
        let _ = fs::create_dir_all(&base);
        let path = base.join("cybos.db");
        let conn = Connection::open(&path).expect("cannot open cybOS database");
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS events(
                id TEXT PRIMARY KEY,
                time TEXT,
                kind TEXT,
                text TEXT
            );

            CREATE TABLE IF NOT EXISTS kv(
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS memories(
                id TEXT PRIMARY KEY,
                time TEXT NOT NULL,
                text TEXT NOT NULL,
                source TEXT NOT NULL,
                importance REAL NOT NULL
            );

            CREATE TABLE IF NOT EXISTS graph_nodes(
                id TEXT PRIMARY KEY,
                label TEXT NOT NULL,
                kind TEXT NOT NULL,
                x REAL NOT NULL,
                y REAL NOT NULL
            );

            CREATE TABLE IF NOT EXISTS graph_links(
                from_id TEXT NOT NULL,
                to_id TEXT NOT NULL,
                relation TEXT NOT NULL,
                PRIMARY KEY(from_id, to_id, relation)
            );

            CREATE INDEX IF NOT EXISTS idx_memories_time
                ON memories(time);

            CREATE INDEX IF NOT EXISTS idx_memories_text
                ON memories(text);
            "#,
        )
        .expect("cannot initialize database");
        Self { path, conn }
    }
    fn get(&self, key: &str) -> Option<String> {
        self.conn
            .query_row("SELECT value FROM kv WHERE key=?1", [key], |r| r.get(0))
            .ok()
    }
    fn set(&self, key: &str, value: &str) {
        let _ = self.conn.execute("INSERT INTO kv(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key,value]);
    }
    fn add_memory(&self, memory: &Memory) {
        let _ = self.conn.execute(
            "INSERT OR IGNORE INTO memories(id,time,text,source,importance)
             VALUES(?1,?2,?3,?4,?5)",
            params![
                memory.id,
                memory.time,
                memory.text,
                memory.source,
                memory.importance
            ],
        );
    }

    fn memories(&self) -> Vec<Memory> {
        let mut st = self
            .conn
            .prepare(
                "SELECT id,time,text,source,importance
                 FROM memories
                 ORDER BY rowid DESC
                 LIMIT 200",
            )
            .unwrap();

        st.query_map([], |r| {
            Ok(Memory {
                id: r.get(0)?,
                time: r.get(1)?,
                text: r.get(2)?,
                source: r.get(3)?,
                importance: r.get(4)?,
            })
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect()
    }

    fn search_memories(&self, query: &str) -> Vec<Memory> {
        let q = query.trim().to_lowercase();

        if q.is_empty() {
            return Vec::new();
        }

        let words: Vec<String> = q
            .split_whitespace()
            .map(|w| {
                w.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                    .to_string()
            })
            .filter(|w| w.len() >= 2)
            .collect();

        if words.is_empty() {
            return Vec::new();
        }

        self.memories()
            .into_iter()
            .filter(|m| {
                let text = m.text.to_lowercase();
                words.iter().any(|word| text.contains(word))
            })
            .take(8)
            .collect()
    }

    fn graph_nodes(&self) -> Vec<GraphNode> {
        let mut st = self
            .conn
            .prepare(
                "SELECT id,label,kind,x,y
                 FROM graph_nodes
                 ORDER BY rowid ASC",
            )
            .unwrap();

        st.query_map([], |r| {
            Ok(GraphNode {
                id: r.get(0)?,
                label: r.get(1)?,
                kind: r.get(2)?,
                x: r.get(3)?,
                y: r.get(4)?,
            })
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect()
    }

    fn graph_links(&self) -> Vec<GraphLink> {
        let mut st = self
            .conn
            .prepare(
                "SELECT from_id,to_id,relation
                 FROM graph_links
                 ORDER BY rowid ASC",
            )
            .unwrap();

        st.query_map([], |r| {
            Ok(GraphLink {
                from: r.get(0)?,
                to: r.get(1)?,
                relation: r.get(2)?,
            })
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect()
    }

    fn save_graph_node(&self, node: &GraphNode) {
        let _ = self.conn.execute(
            "INSERT INTO graph_nodes(id,label,kind,x,y)
             VALUES(?1,?2,?3,?4,?5)
             ON CONFLICT(id) DO UPDATE SET
                 label=excluded.label,
                 kind=excluded.kind,
                 x=excluded.x,
                 y=excluded.y",
            params![node.id, node.label, node.kind, node.x, node.y],
        );
    }

    fn save_graph_link(&self, link: &GraphLink) {
        let _ = self.conn.execute(
            "INSERT OR IGNORE INTO graph_links(from_id,to_id,relation)
             VALUES(?1,?2,?3)",
            params![link.from, link.to, link.relation],
        );
    }

    fn events(&self) -> Vec<Event> {
        let mut st = self
            .conn
            .prepare("SELECT id,time,kind,text FROM events ORDER BY rowid DESC LIMIT 100")
            .unwrap();
        st.query_map([], |r| {
            Ok(Event {
                id: r.get(0)?,
                time: r.get(1)?,
                kind: r.get(2)?,
                text: r.get(3)?,
            })
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect()
    }
    fn add_event(&self, kind: &str, text: &str) {
        let _ = self.conn.execute(
            "INSERT INTO events VALUES(?1,?2,?3,?4)",
            params![
                Uuid::new_v4().to_string(),
                Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                kind,
                text
            ],
        );
    }
}
fn dirs_fallback() -> PathBuf {
    if let Ok(p) = std::env::var("HOME") {
        PathBuf::from(p).join("Library/Application Support/cybOS")
    } else {
        PathBuf::from(".cybOS")
    }
}

struct CybOs {
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

    fn ensure_qwen(&mut self) {
        let address = "127.0.0.1:8080";

        // Qwen уже работает — ничего не запускаем повторно.
        if let Ok(addr) = address.parse() {
            if std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(50)).is_ok() {
                self.qwen_status = "QWEN · ONLINE".into();
                self.status = "LOCAL-FIRST · QWEN ONLINE".into();
                return;
            }
        }

        // Если процесс уже был запущен cybOS, ждём его загрузки.
        if let Some(child) = &mut self.qwen_child {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.qwen_child = None;
                    self.qwen_status = "QWEN · RESTARTING".into();
                }
                Ok(None) => {
                    self.qwen_status = "QWEN · STARTING".into();
                    return;
                }
                Err(_) => {
                    self.qwen_child = None;
                }
            }
        }

        let home = match std::env::var("HOME") {
            Ok(value) => PathBuf::from(value),
            Err(_) => {
                self.qwen_status = "QWEN · HOME ERROR".into();
                return;
            }
        };

        let executable = home.join("cybAI/.venv/bin/mlx_lm.server");

        match Command::new(&executable)
            .args([
                "--model",
                "mlx-community/Qwen3.5-9B-MLX-4bit",
                "--host",
                "127.0.0.1",
                "--port",
                "8080",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => {
                self.qwen_child = Some(child);
                self.qwen_status = "QWEN · STARTING".into();
                self.status = "LOCAL-FIRST · QWEN STARTING".into();
            }
            Err(error) => {
                self.qwen_status = format!("QWEN · START FAILED: {}", error);
                self.status = "LOCAL-FIRST · QWEN ERROR".into();
            }
        }
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
    fn build_brain_context(&self, query: &str) -> String {
        let mut context = String::new();

        context.push_str("=== cybOS LIVE CONTEXT ===\n");

        context.push_str("\nSYSTEM:\n");
        context.push_str(&format!(
            "Node ID: {}\nStatus: {}\nTemperature: {:.1} C\nBattery: {:.0}%\n",
            self.node_id, self.status, self.temperature, self.battery
        ));

        context.push_str("\nKNOWLEDGE GRAPH NODES:\n");
        for node in &self.nodes {
            context.push_str(&format!("- {} | {}\n", node.id, node.label));
        }

        context.push_str("\nKNOWLEDGE GRAPH RELATIONSHIPS:\n");
        for link in &self.links {
            context.push_str(&format!(
                "- {} --{}--> {}\n",
                link.from, link.relation, link.to
            ));
        }

        context.push_str("\nRELEVANT MEMORY:\n");

        let memories = self.store.search_memories(query);

        if memories.is_empty() {
            context.push_str("- No relevant stored memory found.\n");
        } else {
            for memory in memories {
                context.push_str(&format!(
                    "- [{}] source={} importance={:.1}: {}\n",
                    memory.time, memory.source, memory.importance, memory.text
                ));
            }
        }

        context.push_str("\nRECENT EVENTS:\n");
        for event in self.events.iter().take(12) {
            context.push_str(&format!(
                "- [{}] {}: {}\n",
                event.time, event.kind, event.text
            ));
        }

        context.push_str("\n=== END cybOS LIVE CONTEXT ===");

        context
    }

    // ============================================================
    // RobotCYB Tools — local access to cybOS memory and state
    // ============================================================

    fn add_graph_node(&mut self, id: &str, label: &str, kind: &str) -> bool {
        if self.nodes.iter().any(|n| n.id == id) {
            return false;
        }

        let index = self.nodes.len() as f32;

        let angle = index * 1.7;
        let radius = 280.0 + (index % 5.0) * 55.0;

        let x = angle.cos() * radius;
        let y = angle.sin() * radius;

        let node = GraphNode {
            id: id.to_string(),
            label: label.to_string(),
            kind: kind.to_string(),
            x,
            y,
        };

        self.store.save_graph_node(&node);
        self.nodes.push(node);

        self.add_event("GRAPH", format!("New node added: {} ({})", label, kind));

        true
    }

    fn add_graph_link(&mut self, from: &str, to: &str, relation: &str) -> bool {
        if !self.nodes.iter().any(|n| n.id == from) || !self.nodes.iter().any(|n| n.id == to) {
            return false;
        }

        if self
            .links
            .iter()
            .any(|l| l.from == from && l.to == to && l.relation == relation)
        {
            return false;
        }

        let link = GraphLink {
            from: from.to_string(),
            to: to.to_string(),
            relation: relation.to_string(),
        };

        self.store.save_graph_link(&link);
        self.links.push(link);

        self.add_event(
            "GRAPH",
            format!("New relationship: {} --{}--> {}", from, relation, to),
        );

        true
    }

    fn tool_system_status(&self) -> String {
        format!(
            "System status:
Node ID: {}
Status: {}
Temperature: {:.1} C
Battery: {:.0}%",
            self.node_id, self.status, self.temperature, self.battery
        )
    }

    fn tool_farm_status(&self) -> String {
        let mut result = String::new();

        result.push_str("CicadaFarm status:\n");
        result.push_str("Farm node: CicadaFarm\n");
        result.push_str("Knowledge Graph entities:\n");

        for node in &self.nodes {
            if node.id.contains("cicada") || node.id.contains("hive") || node.id.contains("apiary")
            {
                result.push_str(&format!("- {}: {}\n", node.id, node.label));
            }
        }

        result
    }

    fn tool_get_events(&self, limit: usize) -> String {
        let mut result = String::new();

        result.push_str(&format!("Recent cybOS events (limit {}):\n", limit));

        for event in self.events.iter().take(limit) {
            result.push_str(&format!(
                "- [{}] {}: {}\n",
                event.time, event.kind, event.text
            ));
        }

        result
    }

    fn tool_get_entity(&self, query: &str) -> String {
        let q = query.to_lowercase();
        let mut result = String::new();

        for node in &self.nodes {
            if node.id.to_lowercase().contains(&q) || node.label.to_lowercase().contains(&q) {
                result.push_str(&format!(
                    "Entity found:\nID: {}\nLabel: {}\n",
                    node.id, node.label
                ));

                for link in &self.links {
                    if link.from == node.id {
                        result.push_str(&format!(
                            "Relationship: {} --{}--> {}\n",
                            link.from, link.relation, link.to
                        ));
                    }

                    if link.to == node.id {
                        result.push_str(&format!(
                            "Relationship: {} --{}--> {}\n",
                            link.from, link.relation, link.to
                        ));
                    }
                }
            }
        }

        if result.is_empty() {
            format!("No entity found matching '{}'.", query)
        } else {
            result
        }
    }

    fn tool_search_knowledge(&self, query: &str) -> String {
        let q = query.to_lowercase();

        let mut terms: Vec<String> = q
            .split(|c: char| {
                c.is_whitespace()
                    || matches!(
                        c,
                        ',' | '.' | ':' | ';' | '!' | '?' | '(' | ')' | '"' | '\''
                    )
            })
            .filter(|term| term.len() >= 3)
            .map(|term| term.to_string())
            .collect();

        terms.sort();
        terms.dedup();

        let mut matched_nodes: Vec<&GraphNode> = Vec::new();

        for node in &self.nodes {
            let node_id = node.id.to_lowercase();
            let node_label = node.label.to_lowercase();

            let matched = terms
                .iter()
                .any(|term| node_id.contains(term) || node_label.contains(term));

            if matched {
                matched_nodes.push(node);
            }
        }

        let mut result = String::new();

        result.push_str(&format!("Knowledge search results for '{}':\n", query));

        if matched_nodes.is_empty() {
            result.push_str("No matching local knowledge found.\n");
            return result;
        }

        result.push_str("\nMATCHED ENTITIES:\n");

        for node in &matched_nodes {
            result.push_str(&format!("- ENTITY: {} | {}\n", node.id, node.label));
        }

        result.push_str("\nRELATIONSHIPS BETWEEN MATCHED OR RELATED ENTITIES:\n");

        let matched_ids: std::collections::HashSet<&str> =
            matched_nodes.iter().map(|node| node.id.as_str()).collect();

        let mut relationship_found = false;

        for link in &self.links {
            if matched_ids.contains(link.from.as_str()) || matched_ids.contains(link.to.as_str()) {
                result.push_str(&format!(
                    "- {} --{}--> {}\n",
                    link.from, link.relation, link.to
                ));

                relationship_found = true;
            }
        }

        if !relationship_found {
            result.push_str("No relationships found for matched entities.\n");
        }

        result.push_str("\nMATCHING EVENTS:\n");

        let mut event_found = false;

        for event in &self.events {
            let event_text = event.text.to_lowercase();
            let event_kind = event.kind.to_lowercase();

            if terms
                .iter()
                .any(|term| event_text.contains(term) || event_kind.contains(term))
            {
                result.push_str(&format!(
                    "- [{}] {}: {}\n",
                    event.time, event.kind, event.text
                ));

                event_found = true;
            }
        }

        if !event_found {
            result.push_str("No matching events found.\n");
        }

        result
    }

    fn tool_create_event(&mut self, kind: &str, text: &str) -> String {
        self.add_event(kind, text.to_string());

        format!(
            "Event created successfully.\nKind: {}\nText: {}",
            kind, text
        )
    }

    // ============================================================
    // RobotCYB Tool Router
    // Read-only routing layer between user queries and cybOS tools
    // ============================================================

    fn route_tool(&self, query: &str) -> Option<String> {
        let q = query.to_lowercase();

        // System / node status
        if q.contains("статус системы")
            || q.contains("состояние системы")
            || q.contains("статус cybos")
            || q.contains("состояние cybos")
            || q.contains("system status")
            || q.contains("cybos status")
            || q.contains("температур")
            || q.contains("батар")
        {
            return Some(self.tool_system_status());
        }

        // Recent events
        if q.contains("последние события")
            || q.contains("что произошло")
            || q.contains("события cybos")
            || q.contains("recent events")
            || q.contains("what happened")
        {
            return Some(self.tool_get_events(10));
        }

        // Entity lookup
        if q.contains("hive-")
            || q.contains("hive ")
            || q.contains("улей")
            || q.contains("пасек")
            || q.contains("apiary")
        {
            let candidates = ["Hive-003", "hive-003", "apiary-north", "hive"];

            for candidate in candidates {
                if q.contains(&candidate.to_lowercase()) {
                    return Some(self.tool_get_entity(candidate));
                }
            }

            return Some(self.tool_search_knowledge(query));
        }

        // CicadaFarm
        if q.contains("cicadafarm")
            || q.contains("цикадаферм")
            || q.contains("ферм")
            || q.contains("farm")
        {
            return Some(self.tool_farm_status());
        }

        // General knowledge search
        if q.contains("найди")
            || q.contains("поиск")
            || q.contains("информац")
            || q.contains("знаешь о")
            || q.contains("расскажи о")
            || q.contains("search")
            || q.contains("find information")
            || q.contains("what do you know about")
        {
            return Some(self.tool_search_knowledge(query));
        }

        None
    }

    fn qwen_visible_content<'a>(value: &'a serde_json::Value) -> Option<&'a str> {
        let message = &value["choices"][0]["message"];

        if let Some(content) = message["content"].as_str() {
            if !content.trim().is_empty() {
                return Some(content.trim());
            }
        }

        if let Some(reasoning) = message["reasoning"].as_str() {
            if !reasoning.trim().is_empty() {
                return Some(reasoning.trim());
            }
        }

        if let Some(text) = value["choices"][0]["text"].as_str() {
            if !text.trim().is_empty() {
                return Some(text.trim());
            }
        }

        None
    }

    fn parse_planner_json(content: &str) -> Option<serde_json::Value> {
        let clean = content.trim();

        if let Ok(value) = serde_json::from_str::<serde_json::Value>(clean) {
            return Some(value);
        }

        let stripped = clean
            .strip_prefix("```json")
            .or_else(|| clean.strip_prefix("```JSON"))
            .unwrap_or(clean)
            .trim();

        let stripped = stripped.strip_prefix("```").unwrap_or(stripped).trim();

        let stripped = stripped.strip_suffix("```").unwrap_or(stripped).trim();

        if let Ok(value) = serde_json::from_str::<serde_json::Value>(stripped) {
            return Some(value);
        }

        let start = stripped.find('{')?;
        let end = stripped.rfind('}')?;

        if end <= start {
            return None;
        }

        serde_json::from_str(&stripped[start..=end]).ok()
    }

    fn select_tool_with_qwen(&self, q: &str) -> Option<(String, String)> {
        let url = "http://127.0.0.1:8080/v1/chat/completions";

        let system_prompt = r#"
You are the action planner and autonomous brain of RobotCYB.

You have full freedom to answer using your own pretrained knowledge,
reasoning, local cybOS data, or available tools.

A tool is OPTIONAL, not mandatory.

Use a tool when it materially improves the answer:
- local tools for CicadaFarm, memories, Knowledge Graph, events and node state;
- web_search/web_fetch for current or external information;
- no tool for ordinary stable knowledge that you already know.

Do not refuse a normal question merely because the answer is absent
from the local database.

Available tools:

1. system_status
Use for current system status, temperature, battery or node state.

2. get_events
Use for recent cybOS events.
Argument should be a number such as "10".

3. get_entity
Use for a specific entity such as Hive-003, apiary-north,
Weather-001, RobotCYB or CicadaFarm.
Argument should be the entity name.

4. search_knowledge
Use for general searches through the local Knowledge Graph and events.
Argument should be the search query.

5. farm_status
Use for general CicadaFarm information.
Argument should normally be "CicadaFarm".

6. web_search
Use when local cybOS knowledge is insufficient or when the user asks for
public, current, external, legal, government, scientific, technical,
document, website, or other internet information.
Argument should be the complete web search query.

7. web_fetch
Use to open and read a specific URL returned by web_search.
Argument must be the complete URL.

Important web rule:
For factual, legal, governmental, scientific or document questions,
search results alone are not sufficient.
After web_search, use web_fetch on a relevant source before returning
a final answer whenever a readable source is available.

Prefer official or primary sources for legal and government questions.

Return ONLY valid JSON.

If a tool is required:
{"action":"tool","tool":"TOOL_NAME","arguments":"ARGUMENT"}

If the user can be answered without a tool:
{"action":"final","answer":"ANSWER"}

Never output explanations outside JSON.
"#;

        let payload = json!({
            "model": "mlx-community/Qwen3.5-9B-MLX-4bit",
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": q
                }
            ],
            "max_tokens": 250,
            "temperature": 0.0,
            "chat_template_kwargs": {
                "enable_thinking": false
            }
        });

        let response = ureq::post(url)
            .header("Content-Type", "application/json")
            .send_json(&payload)
            .ok()?;

        let body = response.into_body().read_to_string().ok()?;
        let value: serde_json::Value = serde_json::from_str(&body).ok()?;

        let content = Self::qwen_visible_content(&value)?;
        let decision = Self::parse_planner_json(content)?;

        if decision["action"].as_str()? != "tool" {
            return None;
        }

        let tool = decision["tool"].as_str()?.to_string();

        let arguments = decision["arguments"].as_str().unwrap_or("").to_string();

        Some((tool, arguments))
    }

    fn normalize_learning_id(value: &str) -> Option<String> {
        let mut id = String::new();

        for ch in value.trim().to_lowercase().chars() {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                id.push(ch);
            } else if ch.is_whitespace() {
                id.push('-');
            }
        }

        while id.contains("--") {
            id = id.replace("--", "-");
        }

        let id = id.trim_matches('-').to_string();

        if id.is_empty() || id.len() > 80 {
            None
        } else {
            Some(id)
        }
    }

    fn valid_learning_fact(fact: &LearningFact) -> bool {
        !fact.subject.trim().is_empty()
            && !fact.subject_label.trim().is_empty()
            && !fact.subject_kind.trim().is_empty()
            && !fact.relation.trim().is_empty()
            && !fact.object.trim().is_empty()
            && !fact.object_label.trim().is_empty()
            && !fact.object_kind.trim().is_empty()
            && fact.relation.len() <= 80
            && fact.subject_kind.len() <= 40
            && fact.object_kind.len() <= 40
    }

    fn learn_from_user_message(&mut self, q: &str) {
        let text = q.trim();

        if text.is_empty() {
            return;
        }

        let url = "http://127.0.0.1:8080/v1/chat/completions";

        let system_prompt = r#"
You are the cybOS Knowledge Extractor.

Extract only explicit factual knowledge from the user's message.

Return ONLY valid JSON:

{
  "facts": [
    {
      "subject": "stable-id",
      "subject_label": "human readable label",
      "subject_kind": "ENTITY_KIND",
      "relation": "relation_name",
      "object": "stable-id-or-value",
      "object_label": "human readable value",
      "object_kind": "ENTITY_KIND"
    }
  ]
}

Rules:
- Extract only facts explicitly stated by the user.
- Never invent facts.
- Never infer unstated facts.
- IDs must be lowercase ASCII using hyphens.
- Use concise relations such as located_at, has_frames, contains.
- Numeric values use object_kind VALUE.
- Places use object_kind PLACE.
- Hives use subject_kind BEE.
- If there is no factual knowledge, return {"facts":[]}.
- Return JSON only.
"#;

        let payload = json!({
            "model": "mlx-community/Qwen3.5-9B-MLX-4bit",
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": text
                }
            ],
            "max_tokens": 1200,
            "temperature": 0.0,
            "chat_template_kwargs": {
                "enable_thinking": false
            }
        });

        let response = match ureq::post(url)
            .header("Content-Type", "application/json")
            .send_json(&payload)
        {
            Ok(resp) => resp,
            Err(_) => return,
        };

        let body = match response.into_body().read_to_string() {
            Ok(body) => body,
            Err(_) => return,
        };

        let value: serde_json::Value = match serde_json::from_str(&body) {
            Ok(value) => value,
            Err(_) => return,
        };

        let message = &value["choices"][0]["message"];

        let content = message["content"]
            .as_str()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| {
                message["reasoning"]
                    .as_str()
                    .filter(|v| !v.trim().is_empty())
            })
            .or_else(|| {
                value["choices"][0]["text"]
                    .as_str()
                    .filter(|v| !v.trim().is_empty())
            });

        let Some(content) = content else {
            return;
        };

        let cleaned = content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let json_text = if serde_json::from_str::<LearningResponse>(cleaned).is_ok() {
            cleaned.to_string()
        } else {
            let start = match cleaned.find('{') {
                Some(v) => v,
                None => return,
            };

            let end = match cleaned.rfind('}') {
                Some(v) => v,
                None => return,
            };

            cleaned[start..=end].to_string()
        };

        let learning: LearningResponse = match serde_json::from_str(&json_text) {
            Ok(value) => value,
            Err(_) => return,
        };

        let mut accepted = 0usize;

        for fact in learning.facts {
            if !Self::valid_learning_fact(&fact) {
                continue;
            }

            let Some(subject_id) = Self::normalize_learning_id(&fact.subject) else {
                continue;
            };

            let Some(object_id) = Self::normalize_learning_id(&fact.object) else {
                continue;
            };

            if subject_id == object_id {
                continue;
            }

            let subject_kind = fact.subject_kind.trim().to_uppercase();
            let object_kind = fact.object_kind.trim().to_uppercase();
            let relation = fact.relation.trim().to_lowercase();

            if !self.nodes.iter().any(|n| n.id == subject_id) {
                self.add_graph_node(&subject_id, fact.subject_label.trim(), &subject_kind);
            }

            if !self.nodes.iter().any(|n| n.id == object_id) {
                self.add_graph_node(&object_id, fact.object_label.trim(), &object_kind);
            }

            if self.add_graph_link(&subject_id, &object_id, &relation) {
                accepted += 1;
            }
        }

        if accepted > 0 {
            let memory = Memory {
                id: Uuid::new_v4().to_string(),
                time: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                text: text.to_string(),
                source: "USER".into(),
                importance: 1.0,
            };

            self.store.add_memory(&memory);

            self.add_event("MEMORY", format!("User knowledge stored: {}", text));

            self.add_event(
                "LEARNING",
                format!(
                    "Accepted {} graph relationship(s) from user message",
                    accepted
                ),
            );
        }
    }

    fn tool_web_search(&self, query: &str) -> String {
        fn percent_encode(input: &str) -> String {
            let mut out = String::new();

            for byte in input.bytes() {
                match byte {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                        out.push(byte as char)
                    }
                    b' ' => out.push('+'),
                    _ => {
                        out.push('%');
                        out.push_str(&format!("{:02X}", byte));
                    }
                }
            }

            out
        }

        fn html_unescape(input: &str) -> String {
            input
                .replace("&amp;", "&")
                .replace("&quot;", "\"")
                .replace("&#x27;", "'")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&nbsp;", " ")
        }

        fn percent_decode(input: &str) -> String {
            let bytes = input.as_bytes();
            let mut out = Vec::with_capacity(bytes.len());
            let mut i = 0;

            while i < bytes.len() {
                if bytes[i] == b'%' && i + 2 < bytes.len() {
                    let h1 = bytes[i + 1] as char;
                    let h2 = bytes[i + 2] as char;

                    let hex = format!("{}{}", h1, h2);

                    if let Ok(value) = u8::from_str_radix(&hex, 16) {
                        out.push(value);
                        i += 3;
                        continue;
                    }
                }

                if bytes[i] == b'+' {
                    out.push(b' ');
                } else {
                    out.push(bytes[i]);
                }

                i += 1;
            }

            String::from_utf8_lossy(&out).to_string()
        }

        fn resolve_url(href: &str) -> String {
            let href = html_unescape(href.trim());

            if let Some(pos) = href.find("uddg=") {
                let value = &href[pos + 5..];
                let value = value.split('&').next().unwrap_or(value);

                return percent_decode(value);
            }

            if href.starts_with("//") {
                return format!("https:{}", href);
            }

            href
        }

        fn source_class(url: &str) -> &'static str {
            if url.contains("government.ru")
                || url.contains("pravo.gov.ru")
                || url.contains("publication.pravo.gov.ru")
            {
                "OFFICIAL"
            } else if url.contains("consultant.ru") || url.contains("garant.ru") {
                "LEGAL DATABASE"
            } else {
                "OTHER"
            }
        }

        let query = query.trim();

        if query.is_empty() {
            return "Web search query is empty.".into();
        }

        let encoded = percent_encode(query);

        let url = format!("https://html.duckduckgo.com/html/?q={}", encoded);

        let response = match ureq::get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Macintosh; Intel Mac OS X) cybOS/0.6",
            )
            .call()
        {
            Ok(response) => response,
            Err(error) => {
                return format!("Web search failed: {}", error);
            }
        };

        let body = match response.into_body().read_to_string() {
            Ok(body) => body,
            Err(error) => {
                return format!("Web search response could not be read: {}", error);
            }
        };

        let mut results = Vec::new();
        let mut remaining = body.as_str();

        while let Some(start) = remaining.find("result__a") {
            remaining = &remaining[start..];

            let href_start = match remaining.find("href=\"") {
                Some(pos) => pos + 6,
                None => break,
            };

            let href_end = match remaining[href_start..].find('"') {
                Some(pos) => href_start + pos,
                None => break,
            };

            let raw_href = &remaining[href_start..href_end];
            let href = resolve_url(raw_href);

            let title_start = match remaining[href_end..].find('>') {
                Some(pos) => href_end + pos + 1,
                None => break,
            };

            let title_end = match remaining[title_start..].find("</a>") {
                Some(pos) => title_start + pos,
                None => break,
            };

            let title = html_unescape(&remaining[title_start..title_end])
                .trim()
                .to_string();

            if !href.is_empty() && !title.is_empty() {
                results.push(format!(
                    "{}. {}\nSOURCE: {}\nURL: {}",
                    results.len() + 1,
                    title,
                    source_class(&href),
                    href
                ));
            }

            remaining = &remaining[title_end..];

            if results.len() >= 5 {
                break;
            }
        }

        if results.is_empty() {
            return format!("Web search returned no parsed results for: {}", query);
        }

        format!(
            "WEB SEARCH RESULTS FOR: {}\n\n{}\n\nIMPORTANT:\nSearch results are discovery data only. Read a relevant source with web_fetch before making factual claims.",
            query,
            results.join("\n\n")
        )
    }

    fn tool_web_fetch(&self, url: &str) -> String {
        fn html_unescape(input: &str) -> String {
            input
                .replace("&amp;", "&")
                .replace("&quot;", "\"")
                .replace("&#x27;", "'")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&nbsp;", " ")
        }

        fn html_to_text(html: &str) -> String {
            let body = if let Some(start) = html.find("<body") {
                if let Some(open_end) = html[start..].find('>') {
                    let body_start = start + open_end + 1;

                    if let Some(close) = html[body_start..].find("</body>") {
                        &html[body_start..body_start + close]
                    } else {
                        &html[body_start..]
                    }
                } else {
                    html
                }
            } else {
                html
            };

            let mut cleaned = body.to_string();

            loop {
                let Some(start) = cleaned.find("<script") else {
                    break;
                };

                let Some(end_rel) = cleaned[start..].find("</script>") else {
                    cleaned.replace_range(start.., "");
                    break;
                };

                let end = start + end_rel + "</script>".len();
                cleaned.replace_range(start..end, " ");
            }

            loop {
                let Some(start) = cleaned.find("<style") else {
                    break;
                };

                let Some(end_rel) = cleaned[start..].find("</style>") else {
                    cleaned.replace_range(start.., "");
                    break;
                };

                let end = start + end_rel + "</style>".len();
                cleaned.replace_range(start..end, " ");
            }

            let mut result = String::new();
            let mut in_tag = false;

            for ch in cleaned.chars() {
                match ch {
                    '<' => in_tag = true,
                    '>' => {
                        in_tag = false;
                        result.push(' ');
                    }
                    _ if in_tag => {}
                    '\n' | '\r' | '\t' => result.push(' '),
                    _ => result.push(ch),
                }
            }

            let result = html_unescape(&result);

            result.split_whitespace().collect::<Vec<_>>().join(" ")
        }

        let url = url.trim();

        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return format!("Invalid web URL: {}", url);
        }

        let response = match ureq::get(url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Macintosh; Intel Mac OS X) cybOS/0.6",
            )
            .call()
        {
            Ok(response) => response,
            Err(error) => {
                return format!("Web fetch failed for {}: {}", url, error);
            }
        };

        let body = match response.into_body().read_to_string() {
            Ok(body) => body,
            Err(error) => {
                return format!("Web source could not be read as text: {}", error);
            }
        };

        let text = html_to_text(&body);

        if text.trim().is_empty() {
            return format!("Web source returned no readable text: {}", url);
        }

        let max_chars = 7000usize;

        let excerpt = if text.chars().count() > max_chars {
            text.chars().take(max_chars).collect::<String>()
        } else {
            text
        };

        format!("WEB SOURCE\nURL: {}\n\nSOURCE TEXT:\n{}", url, excerpt)
    }

    fn web_urls_from_result(result: &str) -> Vec<String> {
        let mut scored: Vec<(i32, String)> = Vec::new();
        let mut current_title = String::new();

        for line in result.lines() {
            let line = line.trim();

            if let Some((prefix, rest)) = line.split_once(". ") {
                if prefix.chars().all(|c| c.is_ascii_digit()) {
                    current_title = rest.trim().to_string();
                    continue;
                }
            }

            let Some(raw_url) = line.strip_prefix("URL: ") else {
                continue;
            };

            let url = raw_url.trim();

            if !(url.starts_with("http://") || url.starts_with("https://")) {
                continue;
            }

            let title = current_title.to_lowercase();
            let url_lower = url.to_lowercase();

            let mut score = 0i32;

            // Primary official sources first.
            if url_lower.contains("publication.pravo.gov.ru") {
                score += 150;
            }

            if url_lower.contains("pravo.gov.ru") {
                score += 130;
            }

            if url_lower.contains("government.ru/docs/all/") {
                score += 120;
            } else if url_lower.contains("government.ru") {
                score += 90;
            }

            // Direct legal-document pages.
            if url_lower.contains("/document/")
                || url_lower.contains("cons_doc_")
                || url_lower.contains("/doc/")
                || url_lower.ends_with(".pdf")
            {
                score += 40;
            }

            // Legal databases.
            if url_lower.contains("consultant.ru") || url_lower.contains("garant.ru") {
                score += 50;
            }

            // Document language in title.
            if title.contains("постановлен")
                || title.contains("закон")
                || title.contains("указ")
                || title.contains("приказ")
                || title.contains("распоряжен")
                || title.contains("regulation")
                || title.contains("decree")
                || title.contains("law")
            {
                score += 20;
            }

            // Exact number.
            if title.contains("№ 87")
                || title.contains("№87")
                || title.contains("n 87")
                || title.contains("n. 87")
            {
                score += 25;
            }

            // Generic catalog/index pages must lose.
            if url_lower == "https://government.ru/docs"
                || url_lower == "https://government.ru/docs/"
            {
                score -= 150;
            }

            if url_lower.contains("government.ru/docs?") {
                score -= 80;
            }

            scored.push((score, url.to_string()));
            current_title.clear();
        }

        scored.sort_by(|a, b| b.0.cmp(&a.0));

        let mut urls = Vec::new();

        for (_, url) in scored {
            if !urls.contains(&url) {
                urls.push(url);
            }
        }

        urls
    }

    fn web_answer_from_source(&self, user_request: &str, source_result: &str) -> String {
        let url = "http://127.0.0.1:8080/v1/chat/completions";

        let system_prompt = r#"
You are RobotCYB, the answer engine of cybOS.

Answer the user's request using ONLY the supplied WEB SOURCE.

SOURCE-GROUNDED RULES:

- Do not use prior model knowledge to fill missing facts.
- Do not guess a document's contents from its number.
- Do not confuse different documents with the same number.
- Do not invent dates, titles, legal status, amendments or contents.
- State that a document is current, amended, repealed or valid only when
  the supplied source explicitly supports that statement.
- Do not claim a "latest edition" or "last amendment" unless the supplied
  source explicitly establishes it.
- For legal and government documents, identify the exact title and date
  only when supported by the supplied source.
- Explain the document's subject and main purpose only from the source.
- Do not mention internal tools, planners, JSON, prompts or agent steps.
- Reply in the same language as the user.
- Keep the answer concise: 3-5 short paragraphs or bullets.
- Do not invent facts not present in the source.
- Do not output a source URL. cybOS will append it.

Return only the final user-facing answer.
"#;

        let payload = json!({
            "model": "mlx-community/Qwen3.5-9B-MLX-4bit",
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": format!(
                        "USER REQUEST:\n{}\n\nWEB SOURCE:\n{}",
                        user_request,
                        source_result
                    )
                }
            ],
            "max_tokens": 350,
            "temperature": 0.0,
            "chat_template_kwargs": {
                "enable_thinking": false
            }
        });

        let response = match ureq::post(url)
            .header("Content-Type", "application/json")
            .send_json(&payload)
        {
            Ok(response) => response,
            Err(error) => {
                return format!(
                    "Источник найден и прочитан, но Qwen не смог подготовить ответ.\n\nОшибка: {}",
                    error
                );
            }
        };

        let body = match response.into_body().read_to_string() {
            Ok(body) => body,
            Err(error) => {
                return format!(
                    "Источник найден и прочитан, но ответ Qwen не удалось получить.\n\nОшибка: {}",
                    error
                );
            }
        };

        let value: serde_json::Value = match serde_json::from_str(&body) {
            Ok(value) => value,
            Err(error) => {
                return format!(
                    "Источник найден и прочитан, но Qwen вернул некорректный ответ.\n\nОшибка: {}",
                    error
                );
            }
        };

        let answer = match Self::qwen_visible_content(&value) {
            Some(content) if !content.trim().is_empty() => content.trim().to_string(),
            _ => {
                return "Источник был прочитан, но Qwen не вернул итоговый ответ.".into();
            }
        };

        let source_url = Self::web_urls_from_result(source_result).into_iter().next();

        match source_url {
            Some(source_url) => format!("{}\n\nИсточник: {}", answer, source_url),
            None => answer,
        }
    }

    fn planner_decision_from_observation(&self, observation: &str) -> Option<serde_json::Value> {
        let url = "http://127.0.0.1:8080/v1/chat/completions";

        let system_prompt = r#"
You are RobotCYB, the action planner of cybOS.

You receive:
1. ORIGINAL USER REQUEST.
2. TOOL RESULT from the last executed tool.

Your job is to complete ALL parts of the original request.

Tool protocol:

{"action":"tool","tool":"TOOL_NAME","arguments":"ARGUMENT"}

or:

{"action":"final","answer":"ANSWER"}

Available tools:
system_status
get_events
get_entity
search_knowledge
farm_status
web_search
web_fetch

WEB RULES:

web_search is only a discovery step.
It provides titles and URLs, not verified document contents.

For legal, governmental, scientific, technical, current or document questions:
1. Inspect the web_search results.
2. Choose a relevant source.
3. Use web_fetch on that URL.
4. Only then return final.

Prefer official or primary sources.
Do not treat a search-result title as the content of a document.
Do not invent facts absent from the supplied source text.

If web_fetch fails, say that the source could not be read.
Do not replace unread source content with guesses.

Never output intermediate narration such as:
"I am searching..."
"Process started..."
"I will look..."
Those are NOT valid responses.

If another tool is required, return the tool JSON.
If all requested tasks are completed, return final JSON.

Return ONLY valid JSON.
"#;

        for retry in 0..3 {
            let retry_note = if retry == 0 {
                ""
            } else {
                "\n\nRETRY: Your previous planner output was invalid. Return ONLY one valid JSON object with action=tool or action=final. No prose.\n"
            };

            let payload = json!({
                "model": "mlx-community/Qwen3.5-9B-MLX-4bit",
                "messages": [
                    {
                        "role": "system",
                        "content": system_prompt
                    },
                    {
                        "role": "user",
                        "content": format!(
                            "{}{}",
                            observation,
                            retry_note
                        )
                    }
                ],
                "max_tokens": 350,
                "temperature": 0.0,
                "chat_template_kwargs": {
                    "enable_thinking": false
                }
            });

            let response = match ureq::post(url)
                .header("Content-Type", "application/json")
                .send_json(&payload)
            {
                Ok(response) => response,
                Err(_) => continue,
            };

            let body = match response.into_body().read_to_string() {
                Ok(body) => body,
                Err(_) => continue,
            };

            let value: serde_json::Value = match serde_json::from_str(&body) {
                Ok(value) => value,
                Err(_) => continue,
            };

            let Some(content) = Self::qwen_visible_content(&value) else {
                continue;
            };

            let Some(decision) = Self::parse_planner_json(content) else {
                continue;
            };

            let Some(action) = decision["action"].as_str() else {
                continue;
            };

            match action {
                "tool" => {
                    let Some(tool) = decision["tool"].as_str() else {
                        continue;
                    };

                    let allowed = matches!(
                        tool,
                        "system_status"
                            | "get_events"
                            | "get_entity"
                            | "search_knowledge"
                            | "farm_status"
                            | "web_search"
                            | "web_fetch"
                    );

                    if !allowed {
                        continue;
                    }

                    if decision["arguments"].as_str().is_none() {
                        continue;
                    }

                    return Some(decision);
                }

                "final" => {
                    if let Some(answer) = decision["answer"].as_str() {
                        if !answer.trim().is_empty() {
                            return Some(decision);
                        }
                    }
                }

                _ => continue,
            }
        }

        None
    }

    fn conversation_context(&self, current_question: &str) -> String {
        let mut lines = Vec::new();

        for (who, message, _) in self.chat.iter().rev().take(10).rev() {
            lines.push(format!("{}: {}", who, message));
        }

        let history = if lines.is_empty() {
            "No previous conversation.".to_string()
        } else {
            lines.join("\n")
        };

        format!(
            "=== CONVERSATION CONTEXT ===\n{}\n=== CURRENT USER REQUEST ===\n{}\n=== END CONVERSATION CONTEXT ===",
            history, current_question
        )
    }

    fn previous_user_query(&self) -> Option<String> {
        // Prefer the last substantive web request, not a follow-up
        // such as "а действует ли он сейчас?".
        for (who, message, _) in self.chat.iter().rev() {
            if who != "YOU" || message.trim().is_empty() {
                continue;
            }

            let message = message.trim();

            if Self::is_web_request(message) && !Self::is_web_followup(message) {
                return Some(message.to_string());
            }
        }

        // Fallback: any previous user message.
        for (who, message, _) in self.chat.iter().rev() {
            if who == "YOU" && !message.trim().is_empty() {
                return Some(message.trim().to_string());
            }
        }

        None
    }

    fn is_web_request(q: &str) -> bool {
        let lower = q.to_lowercase();

        lower.contains("найди в интернете")
            || lower.contains("найти в интернете")
            || lower.contains("поищи в интернете")
            || lower.contains("проверь в интернете")
            || lower.contains("в интернете")
            || lower.contains("источник")
            || lower.contains("ссылк")
            || lower.contains("актуальн")
            || lower.contains("действует ли")
            || lower.contains("утратил силу")
            || lower.contains("отменен")
            || lower.contains("отменено")
            || lower.contains("постановлен")
            || lower.contains("закон")
            || lower.contains("указ")
            || lower.contains("приказ")
            || lower.contains("норматив")
            || lower.contains("правительств")
    }

    fn is_web_followup(q: &str) -> bool {
        let lower = q.to_lowercase();

        let pronoun = lower.contains("он")
            || lower.contains("него")
            || lower.contains("нему")
            || lower.contains("этот документ")
            || lower.contains("это постановление")
            || lower.contains("этого документа")
            || lower.contains("в нем")
            || lower.contains("в нём");

        let continuation = lower.contains("какие изменения")
            || lower.contains("какие изменения внесли")
            || lower.contains("что изменилось")
            || lower.contains("действует ли")
            || lower.contains("действует сейчас")
            || lower.contains("актуален ли")
            || lower.contains("актуально ли")
            || lower.contains("отменен ли")
            || lower.contains("отменили ли")
            || lower.contains("последние изменения")
            || lower.contains("текущий статус")
            || lower.contains("сейчас");

        pronoun || continuation
    }

    fn web_followup_query(&self, q: &str) -> Option<String> {
        if !Self::is_web_followup(q) {
            return None;
        }

        let previous = self.previous_user_query()?;

        let lower = q.to_lowercase();

        if lower.contains("какие изменения")
            || lower.contains("что изменилось")
            || lower.contains("последние изменения")
        {
            return Some(format!(
                "{} изменения редакция актуальные изменения",
                previous
            ));
        }

        if lower.contains("действует")
            || lower.contains("актуален")
            || lower.contains("сейчас")
            || lower.contains("текущий статус")
            || lower.contains("отменен")
        {
            return Some(format!(
                "{} действует ли сейчас актуальный статус на 2026 год",
                previous
            ));
        }

        Some(format!("{} {}", previous, q))
    }

    fn agent_answer(&mut self, q: &str) -> String {
        let lower = q.trim().to_lowercase();

        let learning_request = lower.starts_with("запомни")
            || lower.starts_with("сохрани")
            || lower.starts_with("учти")
            || lower.starts_with("запиши")
            || lower.starts_with("remember")
            || lower.starts_with("save this")
            || lower.starts_with("store this");

        if learning_request {
            self.learn_from_user_message(q);

            return "Запомнил. Данные обработаны и сохранены в Knowledge Graph и локальной памяти cybOS.".into();
        }

        let conversation = self.conversation_context(q);

        // ----------------------------------------------------
        // Direct web path:
        // explicit internet request or follow-up to a web case
        // skips the initial planner call.
        // ----------------------------------------------------

        let forced_web_query = if let Some(query) = self.web_followup_query(q) {
            Some(query)
        } else if Self::is_web_request(q) {
            Some(q.to_string())
        } else {
            None
        };

        if let Some(web_query) = forced_web_query {
            let search_result = self.tool_web_search(&web_query);

            let urls = Self::web_urls_from_result(&search_result);

            if urls.is_empty() {
                return "Веб-поиск не нашёл подходящих источников.".into();
            }

            for url in urls {
                let fetched = self.tool_web_fetch(&url);

                if !fetched.starts_with("WEB SOURCE\n") {
                    continue;
                }

                return self.web_answer_from_source(&conversation, &fetched);
            }

            return "Поиск выполнился, но cybOS не смог открыть источник. Непроверенная информация не была выдана как факт.".into();
        }

        // ----------------------------------------------------
        // Normal local agent path
        // ----------------------------------------------------

        let mut current_query = conversation.clone();
        let mut forced_tool: Option<(String, String)> = None;

        for step in 0..5 {
            let selection = if let Some(tool) = forced_tool.take() {
                Some(tool)
            } else {
                self.select_tool_with_qwen(&current_query)
            };

            let Some((tool, arguments)) = selection else {
                return self.robot_answer(q);
            };

            let result = match tool.as_str() {
                "system_status" => self.tool_system_status(),

                "get_events" => {
                    let limit = arguments.parse::<usize>().unwrap_or(10).clamp(1, 50);

                    self.tool_get_events(limit)
                }

                "get_entity" => self.tool_get_entity(&arguments),

                "search_knowledge" => self.tool_search_knowledge(&arguments),

                "farm_status" => self.tool_farm_status(),

                "web_search" => self.tool_web_search(&arguments),

                "web_fetch" => {
                    let fetched = self.tool_web_fetch(&arguments);

                    if fetched.starts_with("WEB SOURCE\n") {
                        return self.web_answer_from_source(&conversation, &fetched);
                    }

                    fetched
                }

                _ => {
                    return format!("RobotCYB: неизвестный инструмент {}.", tool);
                }
            };

            // If the planner itself selected web_search,
            // immediately fetch the best candidate.
            if tool == "web_search" {
                let urls = Self::web_urls_from_result(&result);

                for url in urls {
                    let fetched = self.tool_web_fetch(&url);

                    if fetched.starts_with("WEB SOURCE\n") {
                        return self.web_answer_from_source(&conversation, &fetched);
                    }
                }

                return "Поиск выполнился, но источник не удалось открыть. Непроверенная информация не была выдана как факт.".into();
            }

            let observation = format!(
                "ORIGINAL CONVERSATION:

{}

AGENT STEP:
{}

LAST TOOL:
{}

TOOL ARGUMENTS:
{}

TOOL RESULT:
{}
",
                conversation,
                step + 1,
                tool,
                arguments,
                result
            );

            let Some(decision) = self.planner_decision_from_observation(&observation) else {
                return "RobotCYB не смог завершить запрос в проверяемом режиме.".into();
            };

            match decision["action"].as_str() {
                Some("final") => {
                    if let Some(answer) = decision["answer"].as_str() {
                        if !answer.trim().is_empty() {
                            return answer.to_string();
                        }
                    }

                    return self.robot_answer(q);
                }

                Some("tool") => {
                    let next_tool = decision["tool"].as_str().unwrap_or("");

                    let next_arguments = decision["arguments"].as_str().unwrap_or("");

                    if next_tool.is_empty() {
                        return self.robot_answer(q);
                    }

                    forced_tool = Some((next_tool.to_string(), next_arguments.to_string()));

                    current_query = format!(
                        "{}\n\nPrevious tool result:\n{}\n\nRequested next tool: {}\nArguments: {}",
                        conversation, result, next_tool, next_arguments
                    );
                }

                _ => {
                    return self.robot_answer(q);
                }
            }
        }

        "RobotCYB достиг максимального числа шагов агента.".into()
    }

    fn robot_answer(&self, q: &str) -> String {
        let url = "http://127.0.0.1:8080/v1/chat/completions";

        let brain_context = self.build_brain_context(q);

        // Local Tool Router
        let tool_result = self.route_tool(q);

        let tool_context = match tool_result {
            Some(result) => format!(
                "\n\n=== LOCAL TOOL RESULT ===\n{}\n=== END LOCAL TOOL RESULT ===",
                result
            ),
            None => String::from(
                "\n\n=== LOCAL TOOL RESULT ===\nNo local tool was selected.\n=== END LOCAL TOOL RESULT ===",
            ),
        };

        let system_prompt = format!(
            "You are RobotCYB, the autonomous AI brain of cybOS.

You are a general-purpose intelligent assistant connected to a local
cybOS node and CicadaFarm.

You have freedom to:
- use your pretrained knowledge;
- reason about the user's question;
- use local memory and Knowledge Graph;
- use system state and farm tools;
- use internet tools when they are available and useful.

IMPORTANT:
The local database is NOT the limit of your knowledge.
Do not say that you cannot answer simply because something is absent
from SQLite or the Knowledge Graph.

For local farm facts, prefer the actual local data when it exists.
For current or externally changing information, internet information
may be more appropriate.
For ordinary stable knowledge, answer directly from your knowledge.

Think for yourself and choose the best available source.
Do not invent facts when you genuinely do not know them.

Answer naturally, directly and completely.
Reply in the same language as the user.

=== LIVE CONTEXT ===
{}

=== TOOL CONTEXT ===
{}
",
            brain_context, tool_context
        );

        let payload = json!({
            "model": "mlx-community/Qwen3.5-9B-MLX-4bit",
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": q
                }
            ],
            "max_tokens": 400,
            "temperature": 0.7,
            "chat_template_kwargs": {
                "enable_thinking": false
            }
        });

        match ureq::post(url)
            .header("Content-Type", "application/json")
            .send_json(&payload)
        {
            Ok(resp) => match resp.into_body().read_to_string() {
                Ok(body) => match serde_json::from_str::<serde_json::Value>(&body) {
                    Ok(value) => {
                        let message = &value["choices"][0]["message"];

                        if let Some(content) = message["content"].as_str() {
                            if !content.trim().is_empty() {
                                return content.to_string();
                            }
                        }

                        if let Some(reasoning) = message["reasoning"].as_str() {
                            if !reasoning.trim().is_empty() {
                                return reasoning.to_string();
                            }
                        }

                        if let Some(text) = value["choices"][0]["text"].as_str() {
                            if !text.trim().is_empty() {
                                return text.to_string();
                            }
                        }

                        "Qwen returned no visible answer.".to_string()
                    }
                    Err(e) => format!("Qwen returned invalid JSON: {}", e),
                },
                Err(e) => format!("Failed to read Qwen response: {}", e),
            },
            Err(e) => format!(
                "Qwen is offline. Start mlx_lm.server on 127.0.0.1:8080.\n\nError: {}",
                e
            ),
        }
    }

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
