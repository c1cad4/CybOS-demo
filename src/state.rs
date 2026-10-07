//! cybOS application state and initialization.
//!
//! Owns the central application state and its default lifecycle.

use crate::navigation::Page;
use crate::models::{Event, GraphLink, GraphNode};
use crate::store::Store;
use crate::config::DEFAULT_CICADA_WALLET;

use eframe::egui::Vec2;
use std::process::Child;
use std::time::Instant;

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
        let store = crate::runtime::open_store();
        let node_id = crate::runtime::load_or_create_node_id(&store);
        let events = crate::runtime::load_events(&store);
        let mut chat = store.chat_messages();
        if chat.is_empty() {
            chat.push((
                "ROBOTCYB".into(),
                "CicadaFarm is online. Your local cybOS node is ready.".into(),
                false,
            ));
            store.add_chat_message(
                "ROBOTCYB",
                "CicadaFarm is online. Your local cybOS node is ready.",
                false,
            );
        }

        let mut app = Self {
            store,
            page: Page::Dashboard,
            search: String::new(),
            robot_input: String::new(),
            robot_output: String::from("ROBOTCYB READY\n\nLOCAL AGENT ONLINE\nAwaiting request..."),
            chat_input: String::new(),
            chat_output: String::from("CYBCHAT READY\n\nLOCAL-FIRST CHANNEL\nAwaiting message..."),
            chat,
            cicada_wallet: DEFAULT_CICADA_WALLET.into(),
            cicada_balance: None,
            sol_balance: None,
            payment_uri: String::new(),
            payment_status: String::from("PAYMENT REQUEST READY"),
            balance_refresh: std::time::Instant::now() - std::time::Duration::from_secs(60),
            events,
            nodes: Vec::new(),
            links: Vec::new(),
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

        app.initialize_graph();

        app
    }
}
