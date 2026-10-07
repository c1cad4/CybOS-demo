//! Local LAN discovery and directed CybChat transport.
//!
//! Discovery is broadcast-only. Chat is explicitly addressed to one peer,
//! carries a message identity, and uses a bounded delivery acknowledgement.
//! Payloads are still plaintext on the LAN; end-to-end cryptography is a
//! separate transport layer and is deliberately not faked here.

use crate::config::APP_VERSION;
use serde::{Deserialize, Serialize};
use std::net::UdpSocket;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

const LAN_DISCOVERY_PORT: u16 = 39393;
const DISCOVERY_PREFIX: &str = "CYBOS_DISCOVER";
const RESPONSE_PREFIX: &str = "CYBOS_PEER";
const CHAT_PREFIX: &str = "CYBOS_CHAT";
const ACK_PREFIX: &str = "CYBOS_ACK";
const MAX_CHAT_BYTES: usize = 1800;
const CHAT_ACK_TIMEOUT: Duration = Duration::from_millis(700);

#[derive(Clone, Debug)]
pub(crate) struct LanPeer {
    pub(crate) node_id: String,
    pub(crate) address: String,
    pub(crate) version: String,
}

#[derive(Clone, Debug)]
pub(crate) enum LanEvent {
    Chat {
        message_id: String,
        node_id: String,
        message: String,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum LanSendStatus {
    Delivered {
        message_id: String,
        peer_id: String,
    },
    TimedOut {
        message_id: String,
        peer_id: String,
    },
    Failed {
        message_id: String,
        peer_id: String,
        reason: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct LanMessage {
    message_id: String,
    from: String,
    to: String,
    message: String,
    version: String,
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

            if let Some(payload) = message
                .strip_prefix(CHAT_PREFIX)
                .and_then(|rest| rest.strip_prefix(' '))
            {
                let Ok(envelope) = serde_json::from_str::<LanMessage>(payload) else {
                    continue;
                };

                if envelope.to != node_id || envelope.from == node_id {
                    continue;
                }

                if envelope.message.trim().is_empty()
                    || envelope.message.as_bytes().len() > MAX_CHAT_BYTES
                    || envelope.message_id.trim().is_empty()
                {
                    continue;
                }

                let ack = format!(
                    "{} {} {}",
                    ACK_PREFIX,
                    envelope.message_id,
                    node_id
                );

                let _ = socket.send_to(ack.as_bytes(), peer_addr);

                let _ = tx.send(LanEvent::Chat {
                    message_id: envelope.message_id,
                    node_id: envelope.from,
                    message: envelope.message,
                });
            }
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

pub(crate) fn send_private_chat(
    sender_id: &str,
    peer_id: &str,
    peer_address: &str,
    message: &str,
) -> LanSendStatus {
    let message_id = Uuid::new_v4().to_string();

    if message.trim().is_empty() {
        return LanSendStatus::Failed {
            message_id,
            peer_id: peer_id.to_string(),
            reason: "empty message".into(),
        };
    }

    if message.as_bytes().len() > MAX_CHAT_BYTES {
        return LanSendStatus::Failed {
            message_id,
            peer_id: peer_id.to_string(),
            reason: format!("message exceeds {} bytes", MAX_CHAT_BYTES),
        };
    }

    let socket = match UdpSocket::bind(("0.0.0.0", 0)) {
        Ok(socket) => socket,
        Err(error) => {
            return LanSendStatus::Failed {
                message_id,
                peer_id: peer_id.to_string(),
                reason: error.to_string(),
            };
        }
    };

    let _ = socket.set_read_timeout(Some(CHAT_ACK_TIMEOUT));

    let target = format!("{}:{}", peer_address, LAN_DISCOVERY_PORT);

    let envelope = LanMessage {
        message_id: message_id.clone(),
        from: sender_id.to_string(),
        to: peer_id.to_string(),
        message: message.to_string(),
        version: APP_VERSION.to_string(),
    };

    let payload = match serde_json::to_string(&envelope) {
        Ok(value) => value,
        Err(error) => {
            return LanSendStatus::Failed {
                message_id,
                peer_id: peer_id.to_string(),
                reason: error.to_string(),
            };
        }
    };

    let wire = format!("{} {}", CHAT_PREFIX, payload);

    if let Err(error) = socket.send_to(wire.as_bytes(), target.as_str()) {
        return LanSendStatus::Failed {
            message_id,
            peer_id: peer_id.to_string(),
            reason: error.to_string(),
        };
    }

    let mut buffer = [0_u8; 1024];

    loop {
        match socket.recv_from(&mut buffer) {
            Ok((size, _addr)) => {
                let Ok(ack) = std::str::from_utf8(&buffer[..size]) else {
                    continue;
                };

                let mut parts = ack.split_whitespace();

                if parts.next() != Some(ACK_PREFIX) {
                    continue;
                }

                let Some(ack_message_id) = parts.next() else {
                    continue;
                };

                let Some(ack_peer_id) = parts.next() else {
                    continue;
                };

                if ack_message_id == message_id && ack_peer_id == peer_id {
                    return LanSendStatus::Delivered {
                        message_id,
                        peer_id: peer_id.to_string(),
                    };
                }
            }
            Err(_) => {
                return LanSendStatus::TimedOut {
                    message_id,
                    peer_id: peer_id.to_string(),
                };
            }
        }
    }
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

                let target_still_exists = self
                    .lan_target
                    .as_deref()
                    .map(|target| self.lan_peers.iter().any(|peer| peer.node_id == target))
                    .unwrap_or(false);

                if !target_still_exists {
                    self.lan_target = self
                        .lan_peers
                        .first()
                        .map(|peer| peer.node_id.clone());
                }

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
        if self.lan_send_task.is_some() {
            self.notify("LAN DELIVERY ALREADY IN PROGRESS");
            return;
        }

        let message = message.trim().to_string();
        if message.is_empty() {
            return;
        }

        let Some(target_id) = self.lan_target.clone() else {
            self.notify("SELECT A LAN PEER FIRST");
            return;
        };

        let Some(peer) = self
            .lan_peers
            .iter()
            .find(|peer| peer.node_id == target_id)
            .cloned()
        else {
            self.notify("LAN TARGET IS NO LONGER AVAILABLE");
            return;
        };

        let (tx, rx) = mpsc::channel();
        self.lan_send_task = Some(rx);
        self.lan_delivery_status = format!("SENDING TO {}", peer.node_id);

        let sender_id = self.node_id.clone();

        thread::spawn(move || {
            let result = send_private_chat(
                &sender_id,
                &peer.node_id,
                &peer.address,
                &message,
            );
            let _ = tx.send(result);
        });
    }

    pub(crate) fn poll_lan_send(&mut self) {
        let Some(rx) = &self.lan_send_task else {
            return;
        };

        match rx.try_recv() {
            Ok(status) => {
                self.lan_send_task = None;

                match status {
                    LanSendStatus::Delivered {
                        message_id,
                        peer_id,
                    } => {
                        self.lan_delivery_status = format!(
                            "DELIVERED · {} · {}",
                            peer_id, message_id
                        );
                        self.add_event(
                            "CHAT",
                            format!(
                                "LAN message delivered to {} · {}",
                                peer_id, message_id
                            ),
                        );
                        self.notify("LAN MESSAGE DELIVERED");
                    }
                    LanSendStatus::TimedOut {
                        message_id,
                        peer_id,
                    } => {
                        self.lan_delivery_status = format!(
                            "DELIVERY TIMEOUT · {} · {}",
                            peer_id, message_id
                        );
                        self.add_event(
                            "CHAT",
                            format!(
                                "LAN message delivery timeout to {} · {}",
                                peer_id, message_id
                            ),
                        );
                        self.notify("LAN DELIVERY TIMEOUT");
                    }
                    LanSendStatus::Failed {
                        message_id,
                        peer_id,
                        reason,
                    } => {
                        self.lan_delivery_status = format!(
                            "DELIVERY FAILED · {} · {}",
                            peer_id, reason
                        );
                        self.add_event(
                            "CHAT",
                            format!(
                                "LAN message failed to {} · {} · {}",
                                peer_id, message_id, reason
                            ),
                        );
                        self.notify("LAN MESSAGE FAILED");
                    }
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.lan_send_task = None;
            }
        }
    }

    pub(crate) fn poll_lan_events(&mut self) {
        loop {
            let event = match self.lan_events.try_recv() {
                Ok(event) => event,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            };

            match event {
                LanEvent::Chat {
                    message_id,
                    node_id,
                    message,
                } => {
                    let display = format!("LAN:{}", node_id);
                    self.push_chat_message(display, message.clone(), false);
                    self.add_event(
                        "CHAT",
                        format!(
                            "LAN message received from {} · {}",
                            node_id, message_id
                        ),
                    );
                }
            }
        }
    }
}



#[cfg(test)]
mod tests {
    use super::LanMessage;

    #[test]
    fn directed_message_envelope_roundtrips() {
        let message = LanMessage {
            message_id: "msg-001".into(),
            from: "cyb-a".into(),
            to: "cyb-b".into(),
            message: "hello from cybOS".into(),
            version: "0.7.0".into(),
        };

        let encoded = serde_json::to_string(&message).expect("encode message");
        let decoded: LanMessage =
            serde_json::from_str(&encoded).expect("decode message");

        assert_eq!(decoded.message_id, "msg-001");
        assert_eq!(decoded.from, "cyb-a");
        assert_eq!(decoded.to, "cyb-b");
        assert_eq!(decoded.message, "hello from cybOS");
    }
}
