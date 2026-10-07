//! Local LAN discovery and peer-to-peer message transport.
//!
//! Discovery and chat use a small UDP broadcast protocol. Only explicitly
//! addressed cybOS messages are accepted; no credentials or application
//! secrets are transmitted by this service.

use crate::config::APP_VERSION;
use serde_json::json;
use std::net::UdpSocket;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

const LAN_DISCOVERY_PORT: u16 = 39393;
const DISCOVERY_PREFIX: &str = "CYBOS_DISCOVER";
const RESPONSE_PREFIX: &str = "CYBOS_PEER";
const CHAT_PREFIX: &str = "CYBOS_CHAT";

#[derive(Clone, Debug)]
pub(crate) struct LanPeer {
    pub(crate) node_id: String,
    pub(crate) address: String,
    pub(crate) version: String,
}

#[derive(Clone, Debug)]
pub(crate) enum LanEvent {
    Chat {
        node_id: String,
        message: String,
    },
}

pub(crate) fn spawn_listener(node_id: String) -> Receiver<LanEvent> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let socket = match UdpSocket::bind(("0.0.0.0", LAN_DISCOVERY_PORT)) {
            Ok(socket) => socket,
            Err(_) => return,
        };

        let mut buffer = [0_u8; 4096];

        loop {
            let Ok((size, peer_addr)) = socket.recv_from(&mut buffer) else {
                break;
            };

            let Ok(message) = std::str::from_utf8(&buffer[..size]) else {
                continue;
            };

            if let Some(sender_id) = message
                .strip_prefix(DISCOVERY_PREFIX)
                .and_then(|rest| rest.strip_prefix(' '))
            {
                if sender_id == node_id {
                    continue;
                }

                let response = format!(
                    "{} {} {}",
                    RESPONSE_PREFIX,
                    node_id,
                    APP_VERSION
                );

                let _ = socket.send_to(response.as_bytes(), peer_addr);
                continue;
            }

            let Some(payload) = message
                .strip_prefix(CHAT_PREFIX)
                .and_then(|rest| rest.strip_prefix(' '))
            else {
                continue;
            };

            let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
                continue;
            };

            let Some(sender_id) = value["node_id"].as_str() else {
                continue;
            };

            if sender_id == node_id {
                continue;
            }

            let Some(chat_message) = value["message"].as_str() else {
                continue;
            };

            if chat_message.trim().is_empty() {
                continue;
            }

            let _ = tx.send(LanEvent::Chat {
                node_id: sender_id.to_string(),
                message: chat_message.to_string(),
            });
        }
    });

    rx
}

pub(crate) fn scan(node_id: String) -> Vec<LanPeer> {
    let socket = match UdpSocket::bind(("0.0.0.0", 0)) {
        Ok(socket) => socket,
        Err(_) => return Vec::new(),
    };

    if socket.set_broadcast(true).is_err() {
        return Vec::new();
    }

    let _ = socket.set_read_timeout(Some(Duration::from_millis(700)));

    let message = format!("{} {}", DISCOVERY_PREFIX, node_id);

    if socket
        .send_to(
            message.as_bytes(),
            ("255.255.255.255", LAN_DISCOVERY_PORT),
        )
        .is_err()
    {
        return Vec::new();
    }

    let mut peers = Vec::new();
    let mut buffer = [0_u8; 1024];

    loop {
        match socket.recv_from(&mut buffer) {
            Ok((size, addr)) => {
                let Ok(message) = std::str::from_utf8(&buffer[..size]) else {
                    continue;
                };

                let mut parts = message.split_whitespace();

                if parts.next() != Some(RESPONSE_PREFIX) {
                    continue;
                }

                let Some(peer_id) = parts.next() else {
                    continue;
                };

                let version = parts.next().unwrap_or("unknown").to_string();

                if peer_id == node_id {
                    continue;
                }

                if peers.iter().any(|p: &LanPeer| p.node_id == peer_id) {
                    continue;
                }

                peers.push(LanPeer {
                    node_id: peer_id.to_string(),
                    address: addr.ip().to_string(),
                    version,
                });
            }
            Err(_) => break,
        }
    }

    peers
}

pub(crate) fn send_chat(node_id: &str, message: &str) -> bool {
    let socket = match UdpSocket::bind(("0.0.0.0", 0)) {
        Ok(socket) => socket,
        Err(_) => return false,
    };

    if socket.set_broadcast(true).is_err() {
        return false;
    }

    let payload = json!({
        "node_id": node_id,
        "message": message,
        "version": APP_VERSION
    });

    let wire = format!("{} {}", CHAT_PREFIX, payload);

    socket
        .send_to(
            wire.as_bytes(),
            ("255.255.255.255", LAN_DISCOVERY_PORT),
        )
        .is_ok()
}

impl crate::state::CybOs {
    pub(crate) fn start_lan_scan(&mut self) {
        if self.lan_scan.is_some() {
            return;
        }

        let node_id = self.node_id.clone();
        let (tx, rx) = mpsc::channel();

        self.lan_scan = Some(rx);

        thread::spawn(move || {
            let peers = scan(node_id);
            let _ = tx.send(peers);
        });
    }

    pub(crate) fn poll_lan_scan(&mut self) {
        let Some(rx) = &self.lan_scan else {
            return;
        };

        match rx.try_recv() {
            Ok(peers) => {
                self.lan_peers = peers;
                self.lan_scan = None;
                self.last_scan = Some(std::time::Instant::now());
                self.add_event(
                    "NETWORK",
                    format!(
                        "LAN scan completed: {} peer(s) discovered",
                        self.lan_peers.len()
                    ),
                );
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.lan_scan = None;
            }
        }
    }

    pub(crate) fn send_lan_chat(&mut self, message: &str) {
        let node_id = self.node_id.clone();
        let message = message.trim().to_string();

        if message.is_empty() {
            return;
        }

        thread::spawn(move || {
            let _ = send_chat(&node_id, &message);
        });
    }

    pub(crate) fn poll_lan_events(&mut self) {
        loop {
            let event = match self.lan_events.try_recv() {
                Ok(event) => event,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            };

            match event {
                LanEvent::Chat { node_id, message } => {
                    let display = format!("LAN:{}", node_id);
                    self.push_chat_message(display, message.clone(), false);
                    self.add_event(
                        "CHAT",
                        format!("LAN message received from {}", node_id),
                    );
                }
            }
        }
    }
}
