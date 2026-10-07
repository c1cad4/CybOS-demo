//! Local LAN discovery for cybOS nodes.
//!
//! Discovery uses a small UDP broadcast handshake. No credentials or
//! application data are transmitted by the discovery protocol.

use crate::config::APP_VERSION;
use std::net::UdpSocket;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

const LAN_DISCOVERY_PORT: u16 = 39393;
const DISCOVERY_PREFIX: &str = "CYBOS_DISCOVER";
const RESPONSE_PREFIX: &str = "CYBOS_PEER";

#[derive(Clone, Debug)]
pub(crate) struct LanPeer {
    pub(crate) node_id: String,
    pub(crate) address: String,
    pub(crate) version: String,
}

pub(crate) fn spawn_listener(node_id: String) {
    thread::spawn(move || {
        let socket = match UdpSocket::bind(("0.0.0.0", LAN_DISCOVERY_PORT)) {
            Ok(socket) => socket,
            Err(_) => return,
        };

        let mut buffer = [0_u8; 1024];

        loop {
            let Ok((size, peer_addr)) = socket.recv_from(&mut buffer) else {
                break;
            };

            let Ok(message) = std::str::from_utf8(&buffer[..size]) else {
                continue;
            };

            let mut parts = message.split_whitespace();

            if parts.next() != Some(DISCOVERY_PREFIX) {
                continue;
            }

            let Some(sender_id) = parts.next() else {
                continue;
            };

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
        }
    });
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
                let Some(prefix) = parts.next() else {
                    continue;
                };

                if prefix != RESPONSE_PREFIX {
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
}
