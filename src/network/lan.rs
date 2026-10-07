//! Authenticated LAN discovery, peer key exchange and encrypted CybChat transport.
use crate::config::APP_VERSION;
use crate::crypto;
use crate::identity::NodeIdentity;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const LAN_DISCOVERY_PORT: u16 = 39393;
const DISCOVERY_PREFIX: &str = "CYBOS_DISCOVER";
const RESPONSE_PREFIX: &str = "CYBOS_PEER";
const KEY_INIT_PREFIX: &str = "CYBOS_KEY_INIT";
const KEY_REPLY_PREFIX: &str = "CYBOS_KEY_REPLY";
const CHAT_PREFIX: &str = "CYBOS_CHAT";
const ACK_PREFIX: &str = "CYBOS_ACK";
const MAX_CHAT_BYTES: usize = 1800;
const MAX_WIRE_BYTES: usize = 4096;
const CHAT_ACK_TIMEOUT: Duration = Duration::from_millis(900);
const KEY_TIMEOUT: Duration = Duration::from_millis(900);
const REPLAY_WINDOW_SECS: u64 = 300;
const MAX_SESSIONS: usize = 128;

#[derive(Clone, Debug)]
pub(crate) struct LanPeer {
    pub(crate) node_id: String,
    pub(crate) address: String,
    pub(crate) version: String,
    pub(crate) public_key: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) enum LanEvent {
    Chat { message_id: String, node_id: String, message: String },
}

#[derive(Clone, Debug)]
pub(crate) enum LanSendStatus {
    Delivered { message_id: String, peer_id: String },
    TimedOut { message_id: String, peer_id: String },
    Failed { message_id: String, peer_id: String, reason: String },
}

#[derive(Debug, Serialize, Deserialize)]
struct KeyInit {
    from: String,
    to: String,
    public_key: String,
    ephemeral_public_key: String,
    timestamp: u64,
    signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeyReply {
    from: String,
    to: String,
    initiator_ephemeral_public_key: String,
    responder_ephemeral_public_key: String,
    timestamp: u64,
    signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct WireEnvelope {
    message_id: String,
    from: String,
    to: String,
    timestamp: u64,
    counter: u64,
    nonce: String,
    ciphertext: String,
    signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LanAck {
    message_id: String,
    from: String,
    to: String,
    signature: String,
}

#[derive(Clone)]
struct Session {
    key: [u8; 32],
    public_key: Vec<u8>,
    counter: u64,
}

static SEND_SESSIONS: OnceLock<Mutex<HashMap<String, Session>>> = OnceLock::new();

#[cfg(test)]
static LAST_SENT_WIRE: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn send_sessions() -> &'static Mutex<HashMap<String, Session>> {
    SEND_SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn clear_send_session(peer_id: &str) {
    if let Ok(mut sessions) = send_sessions().lock() {
        sessions.remove(peer_id);
    }
}

#[cfg(test)]
fn last_sent_wire() -> Option<String> {
    LAST_SENT_WIRE.get_or_init(|| Mutex::new(None)).lock().ok().and_then(|v| v.clone())
}

#[cfg(test)]
fn remember_sent_wire(wire: &str) {
    if let Ok(mut value) = LAST_SENT_WIRE.get_or_init(|| Mutex::new(None)).lock() {
        *value = Some(wire.to_string());
    }
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn fresh_timestamp(ts: u64) -> bool {
    now_secs().abs_diff(ts) <= REPLAY_WINDOW_SECS
}

fn signed_key_init(from: &str, to: &str, public_key: &str, eph: &str, timestamp: u64) -> Vec<u8> {
    [crypto::PROTOCOL, "key-init", from, to, public_key, eph, &timestamp.to_string()].join("|").into_bytes()
}

fn signed_key_reply(from: &str, to: &str, init_eph: &str, reply_eph: &str, timestamp: u64) -> Vec<u8> {
    [crypto::PROTOCOL, "key-reply", from, to, init_eph, reply_eph, &timestamp.to_string()].join("|").into_bytes()
}

fn session_transcript(from: &str, to: &str, init_eph: &[u8], reply_eph: &[u8]) -> Vec<u8> {
    [crypto::PROTOCOL.as_bytes(), from.as_bytes(), to.as_bytes(), init_eph, reply_eph].concat()
}

fn aad(message_id: &str, from: &str, to: &str, timestamp: u64, counter: u64) -> Vec<u8> {
    [crypto::PROTOCOL, "aead", message_id, from, to, &timestamp.to_string(), &counter.to_string()].join("|").into_bytes()
}

pub(crate) fn spawn_listener(node_id: String, identity: NodeIdentity) -> Receiver<LanEvent> {
    spawn_listener_on_addr(node_id, identity, "0.0.0.0", LAN_DISCOVERY_PORT, Arc::new(AtomicBool::new(true))).0
}

fn spawn_listener_on_port(node_id: String, identity: NodeIdentity, listen_port: u16) -> Receiver<LanEvent> {
    spawn_listener_on_addr(node_id, identity, "127.0.0.1", listen_port, Arc::new(AtomicBool::new(true))).0
}

fn spawn_listener_on_port_with_ack(
    node_id: String,
    identity: NodeIdentity,
    listen_port: u16,
) -> (Receiver<LanEvent>, Arc<AtomicBool>) {
    spawn_listener_on_addr(
        node_id,
        identity,
        "127.0.0.1",
        listen_port,
        Arc::new(AtomicBool::new(true)),
    )
}

fn spawn_listener_on_addr(
    node_id: String,
    identity: NodeIdentity,
    listen_addr: &'static str,
    listen_port: u16,
    ack_enabled: Arc<AtomicBool>,
) -> (Receiver<LanEvent>, Arc<AtomicBool>) {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let socket = match UdpSocket::bind((listen_addr, listen_port)) { Ok(s) => s, Err(_) => return };
        let mut buffer = [0u8; 8192];
        let mut sessions: HashMap<String, Session> = HashMap::new();
        let mut seen_messages: HashMap<String, u64> = HashMap::new();

        loop {
            let Ok((size, peer_addr)) = socket.recv_from(&mut buffer) else { break };
            if size > MAX_WIRE_BYTES { continue; }
            let Ok(message) = std::str::from_utf8(&buffer[..size]) else { continue };

            if let Some(sender_id) = message.strip_prefix(DISCOVERY_PREFIX).and_then(|r| r.strip_prefix(' ')) {
                if sender_id == node_id { continue; }
                let public_key_b64 = STANDARD.encode(identity.public_key());
                let sig = crypto::sign(&identity, &crypto::peer_binding(&node_id, APP_VERSION, &public_key_b64));
                let response = format!("{} {} {} {} {}", RESPONSE_PREFIX, node_id, APP_VERSION, public_key_b64, STANDARD.encode(sig));
                let _ = socket.send_to(response.as_bytes(), peer_addr);
                continue;
            }

            if let Some(payload) = message.strip_prefix(KEY_INIT_PREFIX).and_then(|r| r.strip_prefix(' ')) {
                let Ok(init) = serde_json::from_str::<KeyInit>(payload) else { continue };
                if init.to != node_id || init.from == node_id || !fresh_timestamp(init.timestamp) { continue; }
                let Ok(public_key) = STANDARD.decode(&init.public_key) else { continue };
                let Ok(eph) = STANDARD.decode(&init.ephemeral_public_key) else { continue };
                let Ok(sig) = STANDARD.decode(&init.signature) else { continue };
                if crypto::node_id_from_public_key(&public_key) != init.from || eph.len() != 32 { continue; }
                if !crypto::verify_signature(
                    &public_key,
                    &signed_key_init(&init.from, &init.to, &init.public_key, &init.ephemeral_public_key, init.timestamp),
                    &sig,
                ) { continue; }

                let Ok((private, reply_eph)) = crypto::ephemeral() else { continue };
                let transcript = session_transcript(&init.from, &init.to, &eph, &reply_eph);
                let Ok(key) = crypto::derive_session_key(private, &eph, &transcript) else { continue };
                if !sessions.contains_key(&init.from) && sessions.len() >= MAX_SESSIONS {
                    continue;
                }
                sessions.insert(init.from.clone(), Session { key, public_key, counter: 0 });

                let timestamp = now_secs();
                let reply_eph_b64 = STANDARD.encode(&reply_eph);
                let reply = KeyReply {
                    from: node_id.clone(),
                    to: init.from.clone(),
                    initiator_ephemeral_public_key: init.ephemeral_public_key.clone(),
                    responder_ephemeral_public_key: reply_eph_b64.clone(),
                    timestamp,
                    signature: STANDARD.encode(crypto::sign(
                        &identity,
                        &signed_key_reply(&node_id, &init.from, &init.ephemeral_public_key, &reply_eph_b64, timestamp),
                    )),
                };
                if let Ok(body) = serde_json::to_string(&reply) {
                    let _ = socket.send_to(format!("{} {}", KEY_REPLY_PREFIX, body).as_bytes(), peer_addr);
                }
                continue;
            }

            if let Some(payload) = message.strip_prefix(CHAT_PREFIX).and_then(|r| r.strip_prefix(' ')) {
                let Ok(envelope) = serde_json::from_str::<WireEnvelope>(payload) else { continue };
                if envelope.to != node_id || envelope.from == node_id || envelope.message_id.trim().is_empty()
                    || !fresh_timestamp(envelope.timestamp) { continue; }

                let cutoff = now_secs().saturating_sub(REPLAY_WINDOW_SECS);
                seen_messages.retain(|_, ts| *ts >= cutoff);
                if seen_messages.contains_key(&envelope.message_id) { continue; }

                let Some(session) = sessions.get_mut(&envelope.from) else { continue };
                if envelope.counter != session.counter + 1 { continue; }
                let signed = [
                    crypto::PROTOCOL, "message", envelope.message_id.as_str(), envelope.from.as_str(),
                    envelope.to.as_str(), &envelope.timestamp.to_string(), &envelope.counter.to_string(),
                    envelope.nonce.as_str(), envelope.ciphertext.as_str()
                ].join("|").into_bytes();
                if !crypto::verify_signature(&session.public_key, &signed, &STANDARD.decode(&envelope.signature).unwrap_or_default()) { continue; }
                let message_key = match crypto::ratchet_key(&session.key, envelope.counter) { Ok(k) => k, Err(_) => continue };
                let associated = aad(&envelope.message_id, &envelope.from, &envelope.to, envelope.timestamp, envelope.counter);
                let Ok(plaintext) = crypto::decrypt(&message_key, &associated, &envelope.nonce, &envelope.ciphertext) else { continue };
                if plaintext.len() > MAX_CHAT_BYTES { continue; }
                let Ok(text) = String::from_utf8(plaintext) else { continue };

                let next = match crypto::ratchet_chain(&session.key, envelope.counter) {
                    Ok(next) => next,
                    Err(_) => continue,
                };
                session.key = next;
                session.counter = envelope.counter;
                seen_messages.insert(envelope.message_id.clone(), envelope.timestamp);
                let ack_signed = [crypto::PROTOCOL, "ack", envelope.message_id.as_str(), node_id.as_str(), envelope.from.as_str()].join("|");
                let ack = LanAck {
                    message_id: envelope.message_id.clone(),
                    from: node_id.clone(),
                    to: envelope.from.clone(),
                    signature: STANDARD.encode(crypto::sign(&identity, ack_signed.as_bytes())),
                };
                if ack_enabled.load(Ordering::Acquire) {
                    if let Ok(body) = serde_json::to_string(&ack) {
                        let _ = socket.send_to(format!("{} {}", ACK_PREFIX, body).as_bytes(), peer_addr);
                    }
                }
                let _ = tx.send(LanEvent::Chat { message_id: envelope.message_id, node_id: envelope.from, message: text });
            }
        }
    });
    (rx, ack_enabled)
}

pub(crate) fn scan(node_id: String) -> Vec<LanPeer> {
    let socket = match UdpSocket::bind(("0.0.0.0", 0)) { Ok(s) => s, Err(_) => return Vec::new() };
    if socket.set_broadcast(true).is_err() { return Vec::new(); }
    let _ = socket.set_read_timeout(Some(Duration::from_millis(700)));
    if socket.send_to(format!("{} {}", DISCOVERY_PREFIX, node_id).as_bytes(), ("255.255.255.255", LAN_DISCOVERY_PORT)).is_err() { return Vec::new(); }

    let mut peers = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        match socket.recv_from(&mut buffer) {
            Ok((size, addr)) => {
                let Ok(message) = std::str::from_utf8(&buffer[..size]) else { continue };
                let mut parts = message.split_whitespace();
                if parts.next() != Some(RESPONSE_PREFIX) { continue; }
                let Some(peer_id) = parts.next() else { continue };
                let version = parts.next().unwrap_or("unknown").to_string();
                let Some(public_key_b64) = parts.next() else { continue };
                let Some(signature_b64) = parts.next() else { continue };
                let Ok(public_key) = STANDARD.decode(public_key_b64) else { continue };
                let Ok(sig) = STANDARD.decode(signature_b64) else { continue };
                if crypto::node_id_from_public_key(&public_key) != peer_id
                    || !crypto::verify_signature(&public_key, &crypto::peer_binding(peer_id, &version, public_key_b64), &sig)
                { continue; }
                if peer_id == node_id || peers.iter().any(|p: &LanPeer| p.node_id == peer_id) { continue; }
                peers.push(LanPeer { node_id: peer_id.to_string(), address: addr.ip().to_string(), version, public_key: Some(public_key_b64.to_string()) });
            }
            Err(_) => break,
        }
    }
    peers
}

pub(crate) fn send_private_chat(
    identity: &NodeIdentity,
    peer_id: &str,
    peer_address: &str,
    peer_public_key_b64: &str,
    message: &str,
) -> LanSendStatus {
    send_private_chat_on_port(identity, peer_id, peer_address, LAN_DISCOVERY_PORT, peer_public_key_b64, message)
}

fn send_private_chat_on_port(
    identity: &NodeIdentity,
    peer_id: &str,
    peer_address: &str,
    peer_port: u16,
    peer_public_key_b64: &str,
    message: &str,
) -> LanSendStatus {
    let message_id = Uuid::new_v4().to_string();
    if message.trim().is_empty() {
        return LanSendStatus::Failed { message_id, peer_id: peer_id.to_string(), reason: "empty message".into() };
    }
    if message.as_bytes().len() > MAX_CHAT_BYTES {
        return LanSendStatus::Failed { message_id, peer_id: peer_id.to_string(), reason: format!("message exceeds {} bytes", MAX_CHAT_BYTES) };
    }
    let peer_public_key = match STANDARD.decode(peer_public_key_b64) {
        Ok(k) if crypto::node_id_from_public_key(&k) == peer_id => k,
        _ => return LanSendStatus::Failed { message_id, peer_id: peer_id.to_string(), reason: "peer identity key is invalid".into() },
    };
    let socket = match UdpSocket::bind(("0.0.0.0", 0)) {
        Ok(s) => s,
        Err(e) => return LanSendStatus::Failed { message_id, peer_id: peer_id.to_string(), reason: e.to_string() },
    };
    let target = format!("{}:{}", peer_address, peer_port);
    let mut session = send_sessions().lock().ok().and_then(|g| g.get(peer_id).cloned());

    if session.is_none() {
        let _ = socket.set_read_timeout(Some(KEY_TIMEOUT));
        let (private, init_public) = match crypto::ephemeral() {
            Ok(v) => v,
            Err(e) => return LanSendStatus::Failed { message_id, peer_id: peer_id.to_string(), reason: e.into() },
        };
        let init_public_b64 = STANDARD.encode(&init_public);
        let identity_public_b64 = STANDARD.encode(identity.public_key());
        let timestamp = now_secs();
        let init = KeyInit {
            from: identity.node_id(),
            to: peer_id.to_string(),
            public_key: identity_public_b64.clone(),
            ephemeral_public_key: init_public_b64.clone(),
            timestamp,
            signature: STANDARD.encode(crypto::sign(identity, &signed_key_init(
                &identity.node_id(), peer_id, &identity_public_b64, &init_public_b64, timestamp,
            ))),
        };
        let body = match serde_json::to_string(&init) {
            Ok(v) => v,
            Err(e) => return LanSendStatus::Failed { message_id, peer_id: peer_id.to_string(), reason: e.to_string() },
        };
        if let Err(e) = socket.send_to(format!("{} {}", KEY_INIT_PREFIX, body).as_bytes(), &target) {
            return LanSendStatus::Failed { message_id, peer_id: peer_id.to_string(), reason: e.to_string() };
        }
        let mut buffer = [0u8; 8192];
        let reply = loop {
            match socket.recv_from(&mut buffer) {
                Ok((size, _)) => {
                    let Ok(text) = std::str::from_utf8(&buffer[..size]) else { continue };
                    let Some(payload) = text.strip_prefix(KEY_REPLY_PREFIX).and_then(|r| r.strip_prefix(' ')) else { continue };
                    let Ok(reply) = serde_json::from_str::<KeyReply>(payload) else { continue };
                    if reply.from != peer_id || reply.to != identity.node_id()
                        || !fresh_timestamp(reply.timestamp)
                        || reply.initiator_ephemeral_public_key != init_public_b64 { continue; }
                    let Ok(sig) = STANDARD.decode(&reply.signature) else { continue };
                    let signed = signed_key_reply(&reply.from, &reply.to, &reply.initiator_ephemeral_public_key, &reply.responder_ephemeral_public_key, reply.timestamp);
                    if crypto::verify_signature(&peer_public_key, &signed, &sig) { break reply; }
                }
                Err(_) => {
                clear_send_session(peer_id);
                return LanSendStatus::TimedOut { message_id, peer_id: peer_id.to_string() };
            },
            }
        };
        let reply_public = match STANDARD.decode(&reply.responder_ephemeral_public_key) {
            Ok(v) => v,
            Err(_) => return LanSendStatus::Failed { message_id, peer_id: peer_id.to_string(), reason: "invalid key reply".into() },
        };
        let transcript = session_transcript(&identity.node_id(), peer_id, &init_public, &reply_public);
        let key = match crypto::derive_session_key(private, &reply_public, &transcript) {
            Ok(k) => k,
            Err(e) => return LanSendStatus::Failed { message_id, peer_id: peer_id.to_string(), reason: e.into() },
        };
        session = Some(Session { key, public_key: peer_public_key.clone(), counter: 0 });
    }

    let mut session = session.expect("session initialized");
    let counter = session.counter + 1;
    let timestamp = now_secs();
    let associated = aad(&message_id, &identity.node_id(), peer_id, timestamp, counter);
    let message_key = match crypto::ratchet_key(&session.key, counter) {
        Ok(k) => k,
        Err(e) => return LanSendStatus::Failed { message_id, peer_id: peer_id.to_string(), reason: e.into() },
    };
    let (nonce, ciphertext) = match crypto::encrypt(&message_key, &associated, message.as_bytes()) {
        Ok(v) => v,
        Err(e) => return LanSendStatus::Failed { message_id, peer_id: peer_id.to_string(), reason: e.into() },
    };
    let signed = [
        crypto::PROTOCOL, "message", message_id.as_str(), identity.node_id().as_str(),
        peer_id, &timestamp.to_string(), &counter.to_string(), nonce.as_str(), ciphertext.as_str()
    ].join("|").into_bytes();
    let envelope = WireEnvelope {
        message_id: message_id.clone(),
        from: identity.node_id(),
        to: peer_id.to_string(),
        timestamp,
        nonce,
        ciphertext,
        counter,
        signature: STANDARD.encode(crypto::sign(identity, &signed)),
    };
    let body = match serde_json::to_string(&envelope) {
        Ok(v) => v,
        Err(e) => return LanSendStatus::Failed { message_id, peer_id: peer_id.to_string(), reason: e.to_string() },
    };
    if body.as_bytes().len() > MAX_WIRE_BYTES {
        return LanSendStatus::Failed { message_id, peer_id: peer_id.to_string(), reason: "encrypted envelope exceeds wire limit".into() };
    }
    #[cfg(test)]
    remember_sent_wire(&format!("{} {}", CHAT_PREFIX, body));
    let _ = socket.set_read_timeout(Some(CHAT_ACK_TIMEOUT));
    if let Err(e) = socket.send_to(format!("{} {}", CHAT_PREFIX, body).as_bytes(), &target) {
        return LanSendStatus::Failed { message_id, peer_id: peer_id.to_string(), reason: e.to_string() };
    }
    let mut buffer = [0u8; 8192];
    loop {
        match socket.recv_from(&mut buffer) {
            Ok((size, _)) => {
                let Ok(ack_text) = std::str::from_utf8(&buffer[..size]) else { continue };
                let mut parts = ack_text.split_whitespace();
                if parts.next() != Some(ACK_PREFIX) { continue; }
                let Some(ack_body) = parts.next() else { continue };
                let Ok(ack) = serde_json::from_str::<LanAck>(ack_body) else { continue };
                if ack.message_id != message_id || ack.from != peer_id || ack.to != identity.node_id() { continue; }
                let Ok(sig) = STANDARD.decode(&ack.signature) else { continue };
                let ack_signed = [crypto::PROTOCOL, "ack", ack.message_id.as_str(), ack.from.as_str(), ack.to.as_str()].join("|");
                if crypto::verify_signature(&peer_public_key, ack_signed.as_bytes(), &sig) {
                    if let Ok(next) = crypto::ratchet_chain(&session.key, counter) {
                        session.key = next;
                        session.counter = counter;
                        if let Ok(mut sessions) = send_sessions().lock() { sessions.insert(peer_id.to_string(), session.clone()); }
                    }
                    return LanSendStatus::Delivered { message_id, peer_id: peer_id.to_string() };
                }
            }
            Err(_) => return LanSendStatus::TimedOut { message_id, peer_id: peer_id.to_string() },
        }
    }
}

impl crate::state::CybOs {
    pub(crate) fn start_lan_scan(&mut self) {
        if self.lan_scan.is_some() { return; }
        let node_id = self.node_id.clone();
        let (tx, rx) = mpsc::channel();
        self.lan_scan = Some(rx);
        thread::spawn(move || { let peers = scan(node_id); let _ = tx.send(peers); });
    }

    pub(crate) fn poll_lan_scan(&mut self) {
        let Some(rx) = &self.lan_scan else { return };
        match rx.try_recv() {
            Ok(peers) => {
                self.lan_peers = peers.into_iter().filter(|peer| {
                    peer.public_key.as_deref().map(|key| self.store.trust_peer_key(&peer.node_id, key)).unwrap_or(false)
                }).collect();
                let target_still_exists = self.lan_target.as_deref().map(|target| self.lan_peers.iter().any(|peer| peer.node_id == target)).unwrap_or(false);
                if !target_still_exists { self.lan_target = self.lan_peers.first().map(|peer| peer.node_id.clone()); }
                self.lan_scan = None;
                self.last_scan = Some(std::time::Instant::now());
                self.add_event("NETWORK", format!("LAN scan completed: {} authenticated peer(s) discovered", self.lan_peers.len()));
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => self.lan_scan = None,
        }
    }

    pub(crate) fn send_lan_chat(&mut self, message: &str) {
        if self.lan_send_task.is_some() { self.notify("LAN DELIVERY ALREADY IN PROGRESS"); return; }
        let message = message.trim().to_string();
        if message.is_empty() { return; }
        let Some(target_id) = self.lan_target.clone() else { self.notify("SELECT A LAN PEER FIRST"); return };
        let Some(peer) = self.lan_peers.iter().find(|p| p.node_id == target_id).cloned() else { self.notify("LAN TARGET IS NO LONGER AVAILABLE"); return };
        let Some(peer_public_key) = peer.public_key.clone() else { self.notify("PEER HAS NO AUTHENTICATED IDENTITY KEY"); return };

        let (tx, rx) = mpsc::channel();
        self.lan_send_task = Some(rx);
        self.lan_delivery_status = format!("KEY EXCHANGE → ENCRYPT → SEND TO {}", peer.node_id);
        let identity = self.identity.clone();
        thread::spawn(move || {
            let result = send_private_chat(&identity, &peer.node_id, &peer.address, &peer_public_key, &message);
            let _ = tx.send(result);
        });
    }

    pub(crate) fn poll_lan_send(&mut self) {
        let Some(rx) = &self.lan_send_task else { return };
        match rx.try_recv() {
            Ok(status) => {
                self.lan_send_task = None;
                match status {
                    LanSendStatus::Delivered { message_id, peer_id } => {
                        self.lan_delivery_status = format!("E2E DELIVERED · {} · {}", peer_id, message_id);
                        self.add_event("CHAT", format!("Encrypted LAN message delivered to {} · {}", peer_id, message_id));
                        self.notify("E2E LAN MESSAGE DELIVERED");
                    }
                    LanSendStatus::TimedOut { message_id, peer_id } => {
                        self.lan_delivery_status = format!("E2E DELIVERY TIMEOUT · {} · {}", peer_id, message_id);
                        self.add_event("CHAT", format!("Encrypted LAN delivery timeout to {} · {}", peer_id, message_id));
                        self.notify("E2E LAN DELIVERY TIMEOUT");
                    }
                    LanSendStatus::Failed { message_id, peer_id, reason } => {
                        self.lan_delivery_status = format!("E2E DELIVERY FAILED · {} · {}", peer_id, reason);
                        self.add_event("CHAT", format!("Encrypted LAN message failed to {} · {} · {}", peer_id, message_id, reason));
                        self.notify("E2E LAN MESSAGE FAILED");
                    }
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => self.lan_send_task = None,
        }
    }

    pub(crate) fn poll_lan_events(&mut self) {
        loop {
            let event = match self.lan_events.try_recv() {
                Ok(event) => event,
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            };
            match event {
                LanEvent::Chat { message_id, node_id, message } => {
                    self.push_chat_message(format!("LAN:{}", node_id), message.clone(), false);
                    self.add_event("CHAT", format!("E2E encrypted LAN message received from {} · {}", node_id, message_id));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::UdpSocket;

    fn free_port() -> u16 {
        UdpSocket::bind(("127.0.0.1", 0)).unwrap().local_addr().unwrap().port()
    }

    fn wait_for_chat(rx: &Receiver<LanEvent>, expected: &str) {
        match rx.recv_timeout(Duration::from_secs(2)).unwrap() {
            LanEvent::Chat { message, .. } => assert_eq!(message, expected),
        }
    }

    #[test]
    fn timestamp_window_rejects_old_messages() {
        assert!(fresh_timestamp(now_secs()));
        assert!(!fresh_timestamp(now_secs().saturating_sub(REPLAY_WINDOW_SECS + 1)));
    }

    #[test]
    fn encrypted_envelope_is_not_plaintext() {
        let key = [7u8; 32];
        let associated = aad("msg", "cyb-a", "cyb-b", 123, 1);
        let (nonce, ciphertext) = crypto::encrypt(&key, &associated, b"secret").unwrap();
        assert!(!ciphertext.contains("secret"));
        assert_eq!(crypto::decrypt(&key, &associated, &nonce, &ciphertext).unwrap(), b"secret");
    }

    #[test]
    fn two_node_udp_handshake_ack_and_ratchet_roundtrip() {
        let alice = NodeIdentity::generate_for_test();
        let bob = NodeIdentity::generate_for_test();
        let alice_id = alice.node_id();
        let bob_id = bob.node_id();
        let alice_port = free_port();
        let bob_port = free_port();

        let bob_events = spawn_listener_on_port(bob_id.clone(), bob.clone(), bob_port);
        let _alice_events = spawn_listener_on_port(alice_id.clone(), alice.clone(), alice_port);
        thread::sleep(Duration::from_millis(40));

        let bob_key = STANDARD.encode(bob.public_key());
        let first = send_private_chat_on_port(
            &alice, &bob_id, "127.0.0.1", bob_port, &bob_key, "hello over UDP",
        );
        assert!(matches!(first, LanSendStatus::Delivered { .. }));
        wait_for_chat(&bob_events, "hello over UDP");

        let second = send_private_chat_on_port(
            &alice, &bob_id, "127.0.0.1", bob_port, &bob_key, "second ratcheted message",
        );
        assert!(matches!(second, LanSendStatus::Delivered { .. }));
        wait_for_chat(&bob_events, "second ratcheted message");
    }

    #[test]
    fn lost_ack_forces_fresh_handshake_and_recovers() {
        let alice = NodeIdentity::generate_for_test();
        let bob = NodeIdentity::generate_for_test();
        let bob_id = bob.node_id();
        let bob_port = free_port();
        let (bob_events, ack_enabled) =
            spawn_listener_on_port_with_ack(bob_id.clone(), bob.clone(), bob_port);
        thread::sleep(Duration::from_millis(40));

        let bob_key = STANDARD.encode(bob.public_key());
        ack_enabled.store(false, Ordering::Release);

        let first = send_private_chat_on_port(
            &alice, &bob_id, "127.0.0.1", bob_port, &bob_key, "delivered-without-ack",
        );
        assert!(matches!(first, LanSendStatus::TimedOut { .. }));
        wait_for_chat(&bob_events, "delivered-without-ack");

        ack_enabled.store(true, Ordering::Release);

        let second = send_private_chat_on_port(
            &alice, &bob_id, "127.0.0.1", bob_port, &bob_key, "recovered-after-fresh-handshake",
        );
        assert!(matches!(second, LanSendStatus::Delivered { .. }));
        wait_for_chat(&bob_events, "recovered-after-fresh-handshake");
    }

    #[test]
    fn live_udp_replay_and_ciphertext_tampering_are_rejected() {
        let alice = NodeIdentity::generate_for_test();
        let bob = NodeIdentity::generate_for_test();
        let bob_id = bob.node_id();
        let bob_port = free_port();
        let bob_events = spawn_listener_on_port(bob_id.clone(), bob.clone(), bob_port);
        thread::sleep(Duration::from_millis(40));

        let bob_key = STANDARD.encode(bob.public_key());
        let delivered = send_private_chat_on_port(
            &alice, &bob_id, "127.0.0.1", bob_port, &bob_key, "original",
        );
        assert!(matches!(delivered, LanSendStatus::Delivered { .. }));
        wait_for_chat(&bob_events, "original");

        let wire = last_sent_wire().expect("integration sender should expose test wire capture");
        let payload = wire.strip_prefix("CYBOS_CHAT ").unwrap();
        let mut envelope: WireEnvelope = serde_json::from_str(payload).unwrap();

        let replay_socket = UdpSocket::bind(("127.0.0.1", 0)).unwrap();
        replay_socket.send_to(wire.as_bytes(), ("127.0.0.1", bob_port)).unwrap();
        assert!(bob_events.recv_timeout(Duration::from_millis(250)).is_err());

        envelope.message_id = Uuid::new_v4().to_string();
        let mut ciphertext = STANDARD.decode(&envelope.ciphertext).unwrap();
        ciphertext[0] ^= 1;
        envelope.ciphertext = STANDARD.encode(ciphertext);
        let tampered = format!("{} {}", CHAT_PREFIX, serde_json::to_string(&envelope).unwrap());
        replay_socket.send_to(tampered.as_bytes(), ("127.0.0.1", bob_port)).unwrap();
        assert!(bob_events.recv_timeout(Duration::from_millis(250)).is_err());
    }
}
