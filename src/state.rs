//! cybOS application state and initialization.
//!
//! Owns the central application state and its default lifecycle.

use crate::config::DEFAULT_CICADA_WALLET;
use crate::models::{Event, GraphLink, GraphNode};
use crate::navigation::Page;
use crate::store::Store;

use eframe::egui::Vec2;
use std::process::Child;
use std::sync::mpsc::Receiver;
use std::time::Instant;

pub(crate) type RobotJobResult = Result<String, String>;

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
    pub(crate) balance_task: Option<std::sync::mpsc::Receiver<(Option<f64>, Option<f64>)>>,

    pub(crate) events: Vec<Event>,
    pub(crate) nodes: Vec<GraphNode>,
    pub(crate) links: Vec<GraphLink>,
    pub(crate) graph_zoom: f32,
    pub(crate) graph_pan: Vec2,
    pub(crate) selected_node: Option<String>,
    pub(crate) temperature: f32,
    pub(crate) battery: f32,

    // Local identity and runtime
    pub(crate) node_id: String,
    pub(crate) runtime: crate::runtime::Runtime,
    pub(crate) status: String,
    pub(crate) qwen_child: Option<Child>,
    pub(crate) qwen_status: String,
    pub(crate) qwen_retry_after: Instant,
    pub(crate) toast: Option<(String, Instant)>,
    pub(crate) camera_zone: usize,
    pub(crate) search_focus: bool,

    // Directed LAN communication
    pub(crate) last_scan: Option<Instant>,
    pub(crate) lan_peers: Vec<crate::network::lan::LanPeer>,
    pub(crate) lan_scan: Option<std::sync::mpsc::Receiver<Vec<crate::network::lan::LanPeer>>>,
    pub(crate) lan_events: std::sync::mpsc::Receiver<crate::network::lan::LanEvent>,
    pub(crate) lan_send_task:
        Option<std::sync::mpsc::Receiver<crate::network::lan::LanSendStatus>>,
    pub(crate) lan_target: Option<String>,
    pub(crate) lan_delivery_status: String,

    pub(crate) remember_note: String,
}

impl Default for CybOs {
    fn default() -> Self {
        let store = crate::runtime::open_store();
        let node_id = crate::runtime::load_or_create_node_id(&store);
        let events = crate::runtime::load_events(&store);

        let mut chat = store.chat_messages();
        if chat.is_empty() {
            chat.push((
                "ROBOTCYB".into(),
                "Local cybOS node initialized. Qwen availability is detected separately.".into(),
                false,
            ));
            store.add_chat_message(
                "ROBOTCYB",
                "Local cybOS node initialized. Qwen availability is detected separately.",
                false,
            );
        }

        let lan_events = crate::network::lan::spawn_listener(node_id.clone());

        let mut app = Self {
            store,
            page: Page::Dashboard,
            search: String::new(),
            robot_input: String::new(),
            robot_output: String::from("ROBOTCYB READY\n\nLOCAL AGENT · AWAITING QWEN\nAwaiting request..."),
            robot_job: None,
            robot_status: "IDLE".into(),
            robot_job: None,
            robot_status: "IDLE".into(),
            chat_input: String::new(),
            chat_output: String::from("CYBCHAT READY\n\nLOCAL-FIRST CHANNEL\nAwaiting message..."),
            chat,

            cicada_wallet: DEFAULT_CICADA_WALLET.into(),
            cicada_balance: None,
            sol_balance: None,
            payment_uri: String::new(),
            payment_status: String::from("PAYMENT REQUEST READY"),
            balance_refresh: std::time::Instant::now() - std::time::Duration::from_secs(60),
            balance_task: None,

            events,
            nodes: Vec::new(),
            links: Vec::new(),
            graph_zoom: 1.0,
            graph_pan: Vec2::ZERO,
            selected_node: None,
            temperature: 24.0,
            battery: 100.0,

            node_id,
            runtime: crate::runtime::Runtime::new(),
            status: "LOCAL-FIRST · READY".into(),
            qwen_child: None,
            qwen_status: "QWEN · OFFLINE".into(),
            qwen_retry_after: Instant::now(),
            toast: None,
            camera_zone: 0,
            search_focus: false,

            last_scan: None,
            lan_peers: Vec::new(),
            lan_scan: None,
            lan_events,
            lan_send_task: None,
            lan_target: None,
            lan_delivery_status: "NO DIRECT LAN MESSAGE YET".into(),

            remember_note: String::new(),
        };

        app.initialize_graph();
        app
    }
}
