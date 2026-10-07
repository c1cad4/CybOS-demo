//! cybOS application state and initialization.
//!
//! Owns the central application state and its default lifecycle.

use crate::app::Page;
use crate::models::{Event, GraphLink, GraphNode};
use crate::store::Store;

use eframe::egui::Vec2;
use std::process::Child;
use std::time::Instant;
use uuid::Uuid;

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
