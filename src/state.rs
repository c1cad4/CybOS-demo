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

#[derive(Clone)]
pub(crate) struct PendingSecureTrust {
    pub(crate) node_id: String,
    pub(crate) fingerprint: String,
    pub(crate) public_key: Vec<u8>,
}

pub(crate) struct CybOs {
    pub(crate) store: Store,
    pub(crate) language: crate::language::Language,
    pub(crate) page: Page,
    pub(crate) search: String,
    pub(crate) robot_input: String,
    pub(crate) robot_output: String,
    pub(crate) robot_job: Option<Receiver<RobotJobResult>>,
    pub(crate) robot_contract: Option<crate::runtime::WorkerContract>,
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
    pub(crate) balance_contract: Option<crate::runtime::WorkerContract>,

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
    pub(crate) database_integrity: String,
    pub(crate) cyblex: crate::cyblex::CybLexRuntime,
    pub(crate) cyblex_source: String,
    pub(crate) cyblex_download_path: String,
    pub(crate) cyblex_seed_path: String,
    pub(crate) cyblex_status: String,
    pub(crate) cyblex_torrents: Vec<crate::cyblex::CybLexTorrent>,
    pub(crate) cybdex: crate::cybdex::CybDexRuntime,
    pub(crate) cybdex_query: String,
    pub(crate) cybdex_pairs: Vec<crate::cybdex::CybDexPair>,
    pub(crate) cybdex_selected_pair: Option<crate::cybdex::CybDexPair>,
    pub(crate) cybdex_candles: Vec<crate::cybdex::CybDexCandle>,
    pub(crate) cybdex_timeframe: crate::cybdex::CybDexTimeframe,
    pub(crate) cybdex_status: String,
    pub(crate) cybdex_last_refresh: Instant,
    pub(crate) browser: crate::network::browser::BrowserRuntime,
    pub(crate) browser_url: String,
    pub(crate) browser_title: String,
    pub(crate) browser_resolved_url: String,
    pub(crate) browser_route: String,
    pub(crate) browser_status: String,
    pub(crate) browser_text: String,
    pub(crate) browser_links: Vec<crate::network::browser::BrowserLink>,
    pub(crate) browser_document_bytes: Option<usize>,
    pub(crate) browser_contract: Option<crate::runtime::WorkerContract>,
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
    pub(crate) lan_scan_contract: Option<crate::runtime::WorkerContract>,
    pub(crate) lan_events: crate::network::lan::Listener,
    pub(crate) lan_send_task:
        Option<std::sync::mpsc::Receiver<crate::network::lan::LanSendStatus>>,
    pub(crate) lan_send_contract: Option<crate::runtime::WorkerContract>,
    pub(crate) lan_target: Option<String>,
    pub(crate) lan_delivery_status: String,
    pub(crate) radar_visible: bool,
    pub(crate) radar_visibility: Arc<AtomicBool>,
    pub(crate) ble_peers: Vec<crate::network::ble::BlePeer>,
    pub(crate) ble_scan: Option<std::sync::mpsc::Receiver<Result<Vec<crate::network::ble::BlePeer>, String>>>,
    pub(crate) ble_scan_contract: Option<crate::runtime::WorkerContract>,
    pub(crate) ble_status: String,
    pub(crate) nearby_peers: Vec<crate::network::proximity::NearbyPeer>,
    pub(crate) noise_private_key: Vec<u8>,
    pub(crate) secure_listener: crate::network::secure_chat::Listener,
    pub(crate) secure_send_task: Option<std::sync::mpsc::Receiver<crate::network::secure_chat::SecureSendStatus>>,
    pub(crate) secure_send_contract: Option<crate::runtime::WorkerContract>,
    pub(crate) secure_status: String,
    pub(crate) pending_secure_trust: Vec<PendingSecureTrust>,

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
        let database_integrity = store.database_integrity();
        let language = crate::language::Language::from_code(&store.get("language").unwrap_or_else(|| "en".into()));
        let radar_visibility = Arc::new(AtomicBool::new(radar_visible));
        let lan_events = crate::network::lan::spawn_listener(node_id.clone(), radar_visibility.clone());
        let (noise_private_key, secure_listener, secure_status) =
            match crate::network::secure_chat::load_or_create_static_key(&store) {
                Ok(key) => {
                    let listener = crate::network::secure_chat::spawn_listener(
                        node_id.clone(),
                        key.clone(),
                    );
                    let status = listener
                        .startup_error
                        .as_ref()
                        .map(|error| format!("SECURE CHAT · LISTENER ERROR · {}", error))
                        .unwrap_or_else(|| "SECURE CHAT · READY".into());
                    (key, listener, status)
                }
                Err(error) => (
                    Vec::new(),
                    crate::network::secure_chat::Listener::failed(format!("secure identity unavailable: {}", error)),
                    format!("SECURE CHAT · IDENTITY ERROR · {}", error),
                ),
            };

        let mut app = Self {
            store,
            language,
            page: Page::Dashboard,
            search: String::new(),
            robot_input: String::new(),
            robot_output: String::from("ROBOTCYB READY\n\nLOCAL AGENT · AWAITING QWEN\nAwaiting request..."),
            robot_job: None,
            robot_contract: None,
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
            balance_contract: None,

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
            database_integrity,
            cyblex: crate::cyblex::CybLexRuntime::new(),
            cyblex_source: String::new(),
            cyblex_download_path: Self::cyblex_default_download_path(),
            cyblex_seed_path: String::new(),
            cyblex_status: "CYBLEX · STARTING".into(),
            cyblex_torrents: Vec::new(),
            cybdex: crate::cybdex::CybDexRuntime::new(),
            cybdex_query: String::new(),
            cybdex_pairs: Vec::new(),
            cybdex_selected_pair: None,
            cybdex_candles: Vec::new(),
            cybdex_timeframe: crate::cybdex::CybDexTimeframe::Hour1,
            cybdex_status: "CYBDEX · READY · READ-ONLY MARKET DATA".into(),
            cybdex_last_refresh: Instant::now(),
            browser: crate::network::browser::BrowserRuntime::new(),
            browser_url: "https://cyberia.blog".into(),
            browser_title: String::new(),
            browser_resolved_url: String::new(),
            browser_route: "IDLE".into(),
            browser_status: "READY".into(),
            browser_text: String::new(),
            browser_links: Vec::new(),
            browser_document_bytes: None,
            browser_contract: None,
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
            lan_scan_contract: None,
            lan_events,
            lan_send_task: None,
            lan_send_contract: None,
            lan_target: None,
            lan_delivery_status: "NO DIRECT LAN MESSAGE YET".into(),
            radar_visible,
            radar_visibility,
            ble_peers: Vec::new(),
            ble_scan: None,
            ble_scan_contract: None,
            ble_status: "BLE · IDLE".into(),
            nearby_peers: Vec::new(),
            noise_private_key,
            secure_listener,
            secure_send_task: None,
            secure_send_contract: None,
            secure_status,
            pending_secure_trust: Vec::new(),

            remember_note: String::new(),
        };

        app.initialize_graph();
        app
    }
}




impl CybOs {
    pub(crate) fn navigate_browser(&mut self) {
        let url = self.browser_url.trim().to_string();
        if self.browser_contract.is_some() {
            self.notify("CYBBROWSER REQUEST ALREADY RUNNING");
            return;
        }

        let contract = crate::runtime::WorkerContract::new(
            "BROWSER",
            std::time::Duration::from_secs(12),
        );

        match self.browser.navigate(url.clone()) {
            Ok(()) => {
                self.browser_contract = Some(contract);
                self.browser_status = format!("RESOLVING · {url}");
                self.browser_route = "RESOLVING".into();
                self.runtime.set_status("BROWSER", "RUNNING");
            }
            Err(error) => {
                self.browser_status = format!("ERROR · {error}");
                self.runtime.set_status("BROWSER", "ERROR");
                self.notify("CYBBROWSER REQUEST REJECTED");
            }
        }
    }

    pub(crate) fn poll_browser(&mut self) {
        if let Some(contract) = self.browser_contract.clone() {
            if contract.expired() {
                self.browser_contract = None;
                contract.finish("TIMEOUT");
                self.browser_status = "TIMEOUT".into();
                self.runtime.set_status("BROWSER", "ERROR");
                self.notify("CYBBROWSER TIMEOUT");
                return;
            }
        }

        for event in self.browser.poll() {
            match event {
                crate::network::browser::BrowserEvent::Status(status) => {
                    self.browser_status = status;
                }
                crate::network::browser::BrowserEvent::Document(document) => {
                    self.browser_url = document.requested_url.clone();
                    self.browser_title = document.title;
                    self.browser_resolved_url = document.resolved_url;
                    self.browser_route = document.route.label().into();
                    self.browser_text = document.text;
                    self.browser_links = document.links;
                    self.browser_document_bytes = Some(document.bytes);
                    self.browser_status = "READY".into();

                    if let Some(contract) = self.browser_contract.take() {
                        contract.finish("READY");
                    }
                    self.runtime.set_status("BROWSER", "READY");
                }
                crate::network::browser::BrowserEvent::Error(error) => {
                    self.browser_status = format!("ERROR · {error}");
                    self.browser_route = "ERROR".into();
                    if let Some(contract) = self.browser_contract.take() {
                        contract.finish("ERROR");
                    }
                    self.runtime.set_status("BROWSER", "ERROR");
                    self.add_event("BROWSER", self.browser_status.clone());
                }
            }
        }

        if self.browser_contract.is_some() {
            self.runtime.set_status("BROWSER", "RUNNING");
        }
    }
}

impl CybOs {
    pub(crate) fn poll_cybdex(&mut self) {
        for event in self.cybdex.poll() {
            match event {
                crate::cybdex::CybDexEvent::SearchResults(pairs) => {
                    self.cybdex_pairs = pairs;
                    self.cybdex_status =
                        format!("CYBDEX · {} SOLANA PAIR(S) FOUND", self.cybdex_pairs.len());
                    self.runtime.set_status("CYBDEX", "READY");
                }
                crate::cybdex::CybDexEvent::PairLoaded { pair, candles } => {
                    self.cybdex_selected_pair = Some(pair);
                    self.cybdex_candles = candles;
                    self.cybdex_last_refresh = Instant::now();
                    self.cybdex_status = format!(
                        "CYBDEX · LIVE · {} · {}",
                        self.cybdex_selected_pair
                            .as_ref()
                            .map(|p| format!("{}/{}", p.base_symbol, p.quote_symbol))
                            .unwrap_or_else(|| "PAIR".into()),
                        self.cybdex_timeframe.label()
                    );
                    self.runtime.set_status("CYBDEX", "READY");
                }
                crate::cybdex::CybDexEvent::Status(status) => {
                    self.cybdex_status = status;
                    if !self.cybdex.is_busy() {
                        self.runtime.set_status("CYBDEX", "READY");
                    }
                }
                crate::cybdex::CybDexEvent::Error(error) => {
                    self.cybdex_status = error.clone();
                    self.runtime.set_status("CYBDEX", "ERROR");
                    self.add_event("CYBDEX", error);
                }
            }
        }

        if self.cybdex_selected_pair.is_some()
            && self.cybdex_last_refresh.elapsed() >= std::time::Duration::from_secs(20)
            && !self.cybdex.is_busy()
        {
            self.refresh_cybdex_pair();
        }
    }

    pub(crate) fn refresh_cybdex_pair(&mut self) {
        let Some(pair) = self.cybdex_selected_pair.clone() else {
            return;
        };

        if self.cybdex.is_busy() {
            return;
        }

        if self
            .cybdex
            .load_pair(pair.pair_address, self.cybdex_timeframe)
            .is_ok()
        {
            self.cybdex_status =
                format!("CYBDEX · AUTO REFRESH · {}", self.cybdex_timeframe.label());
            self.runtime.set_status("CYBDEX", "RUNNING");
        }
    }
}

impl CybOs {
    pub(crate) fn cyblex_default_download_path() -> String {
        std::env::var("HOME")
            .map(|home| format!("{home}/Downloads/CybLex"))
            .unwrap_or_else(|_| "Downloads/CybLex".into())
    }

    pub(crate) fn poll_cyblex(&mut self) {
        for event in self.cyblex.poll() {
            match event {
                crate::cyblex::CybLexEvent::Snapshot(torrents) => {
                    self.cyblex_torrents = torrents;
                    self.runtime.set_status(
                        "CYBLEX",
                        if self.cyblex_torrents.is_empty() { "READY" } else { "RUNNING" },
                    );
                }
                crate::cyblex::CybLexEvent::Status(status) => {
                    self.cyblex_status = status;
                    self.runtime.set_status("CYBLEX", "READY");
                    self.add_event("CYBLEX", self.cyblex_status.clone());
                }
                crate::cyblex::CybLexEvent::Error(error) => {
                    self.cyblex_status = format!("CYBLEX · ERROR · {error}");
                    self.runtime.set_status("CYBLEX", "ERROR");
                    self.add_event("CYBLEX", self.cyblex_status.clone());
                }
            }
        }
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
            let event = match self.secure_listener.try_recv() {
                Ok(event) => event,
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    if let Some(error) = self.secure_listener.startup_error.as_deref() {
                        self.secure_status = format!("SECURE CHAT · UNAVAILABLE · {}", error);
                    } else {
                        self.secure_status = "SECURE CHAT · LISTENER STOPPED".into();
                    }
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
                    public_key,
                    reply,
                } => {
                    let trusted = match self.store.secure_peer_key_is_pinned(&node_id, &public_key) {
                        Ok(true) => true,
                        Ok(false) => {
                            if self.pending_secure_trust.len() >= 8 {
                                let _ = reply.send(crate::network::secure_chat::SecureReply::Reject(
                                    "too many pending peer trust approvals".into(),
                                ));
                                self.secure_status = "SECURE CHAT · TRUST QUEUE FULL".into();
                                continue;
                            }
                            if !self.pending_secure_trust.iter().any(|pending| pending.node_id == node_id) {
                                self.pending_secure_trust.push(PendingSecureTrust {
                                    node_id: node_id.clone(),
                                    fingerprint: fingerprint.clone(),
                                    public_key,
                                });
                            }
                            // Never keep an unauthenticated TCP worker alive while a human
                            // decides whether to trust a new identity. Reject this first
                            // message; the sender can retry after the key is approved.
                            let _ = reply.send(crate::network::secure_chat::SecureReply::Reject(
                                "first-contact peer identity requires approval; retry after approval".into(),
                            ));
                            self.secure_status = format!("SECURE CHAT · TRUST APPROVAL REQUIRED · {}", node_id);
                            self.runtime.set_status("CYBCHAT", "WAITING");
                            self.add_event(
                                "SECURITY",
                                format!("First-contact peer {} is awaiting explicit trust approval · SHA-256 {}", node_id, fingerprint),
                            );
                            self.notify("NEW SECURE PEER · VERIFY FINGERPRINT BEFORE TRUSTING");
                            continue;
                        }
                        Err(reason) => {
                            self.secure_status =
                                format!("SECURE CHAT · IDENTITY PIN REJECTED · {}", node_id);
                            self.runtime.set_status("CYBCHAT", "ERROR");
                            let _ = reply.send(crate::network::secure_chat::SecureReply::Reject(
                                reason.clone(),
                            ));
                            self.add_event(
                                "SECURITY",
                                format!(
                                    "Rejected secure message from {}: peer identity pin failed: {}",
                                    node_id, reason
                                ),
                            );
                            self.notify("SECURE PEER IDENTITY PIN FAILED");
                            continue;
                        }
                    };

                    if !self.store.claim_secure_message_id(&message_id, &node_id) {
                        self.secure_status =
                            format!("SECURE CHAT · REPLAY REJECTED · {}", node_id);
                        let _ = reply.send(crate::network::secure_chat::SecureReply::Reject(
                            "replayed message id".into(),
                        ));
                        self.add_event(
                            "SECURITY",
                            format!(
                                "Rejected replayed secure message {} from {}",
                                message_id, node_id
                            ),
                        );
                        self.notify("SECURE REPLAY REJECTED");
                        continue;
                    }

                    self.upsert_secure_peer(&node_id, &fingerprint, trusted);
                    self.push_chat_message(format!("CYB:{}", node_id), message.clone(), false);
                    self.add_event(
                        "CHAT",
                        format!(
                            "Secure message from {} · {} · fp {} · TRUSTED",
                            node_id, message_id, fingerprint
                        ),
                    );
                    self.secure_status = format!("SECURE CHAT · RECEIVED · {} · TRUSTED", node_id);
                    let _ = reply.send(crate::network::secure_chat::SecureReply::Ack);
                    self.runtime.set_status("CYBCHAT", "READY");
                }
            }
        }
    }

    pub(crate) fn approve_pending_secure_peer(&mut self, index: usize) {
        if index >= self.pending_secure_trust.len() {
            return;
        }
        let pending = self.pending_secure_trust.remove(index);
        match self.store.pin_secure_peer_key(&pending.node_id, &pending.public_key) {
            Ok(_) => {
                self.upsert_secure_peer(&pending.node_id, &pending.fingerprint, true);
                self.add_event(
                    "SECURITY",
                    format!("User approved first-contact identity {} · SHA-256 {}", pending.node_id, pending.fingerprint),
                );
                self.secure_status = format!("SECURE CHAT · TRUSTED · {} · RETRY MESSAGE", pending.node_id);
                self.runtime.set_status("CYBCHAT", "READY");
                self.notify(format!("SECURE PEER TRUSTED: {} · RETRY YOUR MESSAGE", pending.node_id));
            }
            Err(reason) => {
                self.secure_status = format!("SECURE CHAT · IDENTITY PIN FAILED · {}", pending.node_id);
                self.runtime.set_status("CYBCHAT", "ERROR");
                self.add_event("SECURITY", format!("Could not pin approved peer {}: {}", pending.node_id, reason));
                self.notify("SECURE PEER KEY COULD NOT BE SAVED");
            }
        }
    }

    pub(crate) fn reject_pending_secure_peer(&mut self, index: usize) {
        if index >= self.pending_secure_trust.len() {
            return;
        }
        let pending = self.pending_secure_trust.remove(index);
        self.secure_status = format!("SECURE CHAT · PEER REJECTED · {}", pending.node_id);
        self.add_event("SECURITY", format!("User rejected first-contact peer {}", pending.node_id));
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
        let contract = crate::runtime::WorkerContract::new("CYBCHAT", std::time::Duration::from_secs(12));
        let worker_contract = contract.clone();
        self.secure_send_contract = Some(contract);
        self.secure_status = format!("SECURE CHAT · CONNECTING · {}", peer.node_id);
        self.runtime.set_status("CYBCHAT", "RUNNING");

        let sender_id = self.node_id.clone();
        let private_key = self.noise_private_key.clone();

        std::thread::spawn(move || {
            worker_contract.heartbeat();
            if worker_contract.expired() {
                worker_contract.finish("TIMEOUT");
                return;
            }
            let status = crate::network::secure_chat::send(
                &sender_id,
                &peer.node_id,
                &peer.address,
                &private_key,
                &message,
            );
            worker_contract.finish(match status {
                crate::network::secure_chat::SecureSendStatus::Delivered { .. } => "READY",
                crate::network::secure_chat::SecureSendStatus::Failed { .. } => "ERROR",
            });
            let _ = tx.send(status);
        });
    }

    pub(crate) fn poll_secure_send(&mut self) {
        let Some(rx) = &self.secure_send_task else {
            return;
        };

        if let Some(contract) = self.secure_send_contract.clone() {
            if contract.expired() {
                self.secure_send_task = None;
                self.secure_send_contract = None;
                contract.finish("TIMEOUT");
                self.secure_status = "SECURE CHAT · TIMEOUT".into();
                self.runtime.set_status("CYBCHAT", "ERROR");
                return;
            }
        }

        match rx.try_recv() {
            Ok(status) => {
                self.secure_send_task = None;
                self.secure_send_contract = None;
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
                if let Some(contract) = self.secure_send_contract.take() {
                    contract.finish("ERROR");
                }
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
                    self.ble_status = "BLE · ADVERTISER EXITED · RETRYING".into();
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
