//! cybOS application state and initialization.
//!
//! Owns the central application state and its default lifecycle.

use crate::config::DEFAULT_CICADA_WALLET;
use crate::models::{Event, GraphLink, GraphNode};
use crate::navigation::Page;
use crate::store::Store;

use eframe::egui::Vec2;
use std::process::Child;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Instant;

pub(crate) type RobotJobResult = Result<String, String>;

pub(crate) struct CybOs {
    pub(crate) store: Store,
    pub(crate) page: Page,
    pub(crate) search: String,
    pub(crate) robot_input: String,
    pub(crate) robot_output: String,
    pub(crate) robot_job: Option<Receiver<RobotJobResult>>,
    pub(crate) robot_status: String,
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
    pub(crate) ble_advertiser: Option<Child>,
    pub(crate) ble_advertiser_retry_after: Instant,
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
    pub(crate) radar_visible: bool,
    pub(crate) radar_visibility: Arc<AtomicBool>,
    pub(crate) ble_peers: Vec<crate::network::ble::BlePeer>,
    pub(crate) ble_scan: Option<std::sync::mpsc::Receiver<Result<Vec<crate::network::ble::BlePeer>, String>>>,
    pub(crate) ble_status: String,
    pub(crate) nearby_peers: Vec<crate::network::proximity::NearbyPeer>,
    pub(crate) noise_private_key: Vec<u8>,
    pub(crate) secure_events: std::sync::mpsc::Receiver<crate::network::secure_chat::SecureEvent>,
    pub(crate) secure_send_task: Option<std::sync::mpsc::Receiver<crate::network::secure_chat::SecureSendStatus>>,
    pub(crate) secure_status: String,

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

        let radar_visible = store.get("radar_visible").map(|v| v == "true").unwrap_or(false);
        let radar_visibility = Arc::new(AtomicBool::new(radar_visible));
        let lan_events = crate::network::lan::spawn_listener(node_id.clone(), radar_visibility.clone());
        let noise_private_key = crate::network::secure_chat::load_or_create_static_key(&store)
            .unwrap_or_default();
        let secure_events = crate::network::secure_chat::spawn_listener(
            node_id.clone(),
            noise_private_key.clone(),
        );

        let mut app = Self {
            store,
            page: Page::Dashboard,
            search: String::new(),
            robot_input: String::new(),
            robot_output: String::from("ROBOTCYB READY\n\nLOCAL AGENT · AWAITING QWEN\nAwaiting request..."),
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
            ble_advertiser: None,
            ble_advertiser_retry_after: Instant::now(),
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
            radar_visible,
            radar_visibility,
            ble_peers: Vec::new(),
            ble_scan: None,
            ble_status: "BLE · IDLE".into(),
            nearby_peers: Vec::new(),
            noise_private_key,
            secure_events,
            secure_send_task: None,
            secure_status: "SECURE CHAT · READY".into(),

            remember_note: String::new(),
        };

        app.initialize_graph();
        app
    }
}



impl CybOs {
    pub(crate) fn refresh_proximity(&mut self) {
        self.nearby_peers =
            crate::network::proximity::merge(&self.lan_peers, &self.ble_peers);
    }
}

impl CybOs {
    pub(crate) fn poll_secure_events(&mut self) {
        loop {
            let event = match self.secure_events.try_recv() {
                Ok(event) => event,
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.secure_status = "SECURE CHAT · LISTENER STOPPED".into();
                    self.runtime.set_status("CYBCHAT", "ERROR");
                    break;
                }
            };

            match event {
                crate::network::secure_chat::SecureEvent::Received {
                    message_id,
                    node_id,
                    message,
                    fingerprint,
                } => {
                    self.push_chat_message(format!("CYB:{}", node_id), message.clone(), false);
                    self.add_event(
                        "CHAT",
                        format!("Secure message from {} · {} · fp {}", node_id, message_id, fingerprint),
                    );
                    self.secure_status = format!("SECURE CHAT · RECEIVED · {}", node_id);
                    self.runtime.set_status("CYBCHAT", "READY");
                }
            }
        }
    }

    pub(crate) fn send_secure_chat(&mut self, message: &str) {
        if self.secure_send_task.is_some() {
            self.notify("SECURE DELIVERY ALREADY IN PROGRESS");
            return;
        }

        let message = message.trim().to_string();
        if message.is_empty() {
            return;
        }

        let Some(target_id) = self.lan_target.clone() else {
            self.notify("SELECT A DISCOVERED LAN PEER FIRST");
            return;
        };

        let Some(peer) = self.lan_peers.iter().find(|p| p.node_id == target_id).cloned() else {
            self.notify("SELECTED PEER IS NOT AVAILABLE");
            return;
        };

        let (tx, rx) = std::sync::mpsc::channel();
        self.secure_send_task = Some(rx);
        self.secure_status = format!("SECURE CHAT · CONNECTING · {}", peer.node_id);
        self.runtime.set_status("CYBCHAT", "RUNNING");

        let sender_id = self.node_id.clone();
        let private_key = self.noise_private_key.clone();

        std::thread::spawn(move || {
            let status = crate::network::secure_chat::send(
                &sender_id,
                &peer.node_id,
                &peer.address,
                &private_key,
                &message,
            );
            let _ = tx.send(status);
        });
    }

    pub(crate) fn poll_secure_send(&mut self) {
        let Some(rx) = &self.secure_send_task else {
            return;
        };

        match rx.try_recv() {
            Ok(status) => {
                self.secure_send_task = None;
                match status {
                    crate::network::secure_chat::SecureSendStatus::Delivered { message_id, peer_id } => {
                        self.secure_status = format!("SECURE CHAT · DELIVERED · {}", peer_id);
                        self.add_event("CHAT", format!("Encrypted message delivered to {} · {}", peer_id, message_id));
                        self.runtime.set_status("CYBCHAT", "READY");
                        self.notify("ENCRYPTED MESSAGE DELIVERED");
                    }
                    crate::network::secure_chat::SecureSendStatus::Failed { message_id, peer_id, reason } => {
                        self.secure_status = format!("SECURE CHAT · FAILED · {}", reason);
                        self.add_event("CHAT", format!("Encrypted message failed to {} · {} · {}", peer_id, message_id, reason));
                        self.runtime.set_status("CYBCHAT", "ERROR");
                        self.notify("ENCRYPTED MESSAGE FAILED");
                    }
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.secure_send_task = None;
                self.secure_status = "SECURE CHAT · WORKER DISCONNECTED".into();
                self.runtime.set_status("CYBCHAT", "ERROR");
            }
        }
    }
}

impl CybOs {
    pub(crate) fn sync_ble_advertiser(&mut self) {
        if !self.radar_visible {
            if self.ble_advertiser.is_some() {
                crate::network::ble_advertiser::stop(&mut self.ble_advertiser);
                self.ble_status = "BLE · ADVERTISING STOPPED · HIDDEN".into();
            }
            self.ble_advertiser_retry_after = Instant::now();
            return;
        }

        if self.ble_advertiser.is_some() {
            match crate::network::ble_advertiser::poll(&mut self.ble_advertiser) {
                Some(true) => {
                    self.ble_status = "BLE · ADVERTISING · OPT-IN".into();
                    self.runtime.set_status("RADAR", "READY");
                }
                Some(false) => {
                    self.ble_status = "BLE · ADVERTISER EXITED · RETRYING";
                    self.runtime.set_status("RADAR", "ERROR");
                    self.ble_advertiser_retry_after =
                        Instant::now() + std::time::Duration::from_secs(5);
                }
                None => {}
            }
            return;
        }

        if Instant::now() < self.ble_advertiser_retry_after {
            return;
        }

        match crate::network::ble_advertiser::start(&self.node_id) {
            Ok(child) => {
                self.ble_advertiser = Some(child);
                self.ble_status = "BLE · STARTING ADVERTISEMENT".into();
                self.runtime.set_status("RADAR", "RUNNING");
            }
            Err(error) => {
                self.ble_status = format!("BLE · ADVERTISER ERROR · {}", error);
                self.runtime.set_status("RADAR", "ERROR");
                self.ble_advertiser_retry_after =
                    Instant::now() + std::time::Duration::from_secs(5);
            }
        }
    }
}

impl Drop for CybOs {
    fn drop(&mut self) {
        crate::network::ble_advertiser::stop(&mut self.ble_advertiser);
        if let Some(child) = self.qwen_child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
