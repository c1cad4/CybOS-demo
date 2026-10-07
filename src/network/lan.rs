//! Authenticated LAN discovery, peer key exchange and encrypted CybChat transport.
use crate::config::APP_VERSION;
use crate::crypto;
use crate::identity::NodeIdentity;
use crate::network::onion;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
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
const ONION_BIND_PREFIX: &str = "CYBOS_ONION_BIND";
const ONION_PREFIX: &str = onion::ONION_PREFIX;
const ONION_DELIVERY_PREFIX: &str = onion::ONION_DELIVERY_PREFIX;
const ONION_REVERSE_PREFIX: &str = "CYBOS_ONION_REVERSE";
const MAX_CHAT_BYTES: usize = 1800;
const MAX_WIRE_BYTES: usize = 4096;
const CHAT_ACK_TIMEOUT: Duration = Duration::from_millis(900);
const KEY_TIMEOUT: Duration = Duration::from_millis(900);
const REPLAY_WINDOW_SECS: u64 = 300;
const MAX_SESSIONS: usize = 128;
const MAX_ONION_ROUTES: usize = 256;

#[derive(Clone, Debug)]
pub(crate) struct LanPeer {
    pub(crate) node_id: String,
    pub(crate) address: String,
    pub(crate) version: String,
    pub(crate) public_key: Option<String>,
    pub(crate) fingerprint: Option<String>,
    pub(crate) trusted: bool,
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

#[derive(Clone, Debug)]
pub(crate) struct OnionRoutePeer {
    pub(crate) node_id: String,
    pub(crate) address: String,
    pub(crate) public_key_b64: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OnionRouteBind {
    route_id: String,
    hop_index: u8,
    source_id: String,
    source_public_key: String,
    previous_node_id: String,
    previous_address: String,
    next_node_id: String,
    next_address: String,
    expires_at: u64,
    signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OnionReverseAck {
    route_id: String,
    packet_id: String,
    hop_index: u8,
    ack: LanAck,
}

#[derive(Clone, Debug)]
struct OnionRouteBinding {
    source_id: String,
    hop_index: u8,
    previous_node_id: String,
    previous_address: String,
    next_node_id: String,
    next_address: String,
    expires_at: u64,
}

#[derive(Clone)]
struct Session {
    root_key: [u8; 32],
    key: [u8; 32],
    public_key: Vec<u8>,
    counter: u64,
}

static SEND_SESSIONS: OnceLock<Mutex<HashMap<String, Session>>> = OnceLock::new();

#[cfg(test)]
static LAST_SENT_WIRE: OnceLock<Mutex<Option<String>>> = OnceLock::new();
#[cfg(test)]
static INTEGRATION_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn send_sessions() -> &'static Mutex<HashMap<String, Session>> {
    SEND_SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn clear_send_session(peer_id: &str) {
    if let Ok(mut sessions) = send_sessions().lock() {
        sessions.remove(peer_id);
    }
}

#[cfg(test)]
fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    INTEGRATION_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("integration test lock poisoned")
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

fn signed_onion_route_bind(bind: &OnionRouteBind) -> Vec<u8> {
    [
        crypto::PROTOCOL,
        "onion-bind",
        bind.route_id.as_str(),
        &bind.hop_index.to_string(),
        bind.source_id.as_str(),
        bind.source_public_key.as_str(),
        bind.previous_node_id.as_str(),
        bind.previous_address.as_str(),
        bind.next_node_id.as_str(),
        bind.next_address.as_str(),
        &bind.expires_at.to_string(),
    ]
    .join("|")
    .into_bytes()
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
    let ack_state = Arc::clone(&ack_enabled);
    thread::spawn(move || {
        let socket = match UdpSocket::bind((listen_addr, listen_port)) { Ok(s) => s, Err(_) => return };
        let mut buffer = [0u8; 8192];
        let mut sessions: HashMap<String, Session> = HashMap::new();
        let mut seen_messages: HashMap<String, u64> = HashMap::new();
        let mut onion_bindings: HashMap<String, OnionRouteBinding> = HashMap::new();
        let mut onion_cache = onion::OnionRelayCache::new();

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


            if let Some(payload) = message.strip_prefix(ONION_BIND_PREFIX).and_then(|r| r.strip_prefix(' ')) {
                let Ok(bind) = serde_json::from_str::<OnionRouteBind>(payload) else { continue };
                if bind.source_id == node_id
                    || bind.route_id.is_empty()
                    || bind.previous_node_id.is_empty()
                    || bind.previous_address.is_empty()
                    || bind.next_node_id.is_empty()
                    || bind.next_address.is_empty()
                    || bind.hop_index as usize >= onion::MAX_ONION_HOPS
                    || !fresh_timestamp(bind.expires_at)
                {
                    continue;
                }

                let Ok(source_public_key) = STANDARD.decode(&bind.source_public_key) else { continue };
                let Ok(signature) = STANDARD.decode(&bind.signature) else { continue };
                if crypto::node_id_from_public_key(&source_public_key) != bind.source_id
                    || !crypto::verify_signature(
                        &source_public_key,
                        &signed_onion_route_bind(&bind),
                        &signature,
                    )
                {
                    continue;
                }

                if !sessions.contains_key(&bind.source_id) {
                    continue;
                }

                if bind.hop_index == 0 && bind.previous_node_id != bind.source_id {
                    continue;
                }
                let previous_address = if bind.hop_index == 0 {
                    peer_addr.to_string()
                } else {
                    bind.previous_address.clone()
                };

                let binding = OnionRouteBinding {
                    source_id: bind.source_id.clone(),
                    hop_index: bind.hop_index,
                    previous_node_id: bind.previous_node_id.clone(),
                    previous_address,
                    next_node_id: bind.next_node_id.clone(),
                    next_address: bind.next_address.clone(),
                    expires_at: bind.expires_at,
                };

                if let Some(existing) = onion_bindings.get(&bind.route_id) {
                    if existing.source_id != binding.source_id
                        || existing.hop_index != binding.hop_index
                        || existing.previous_node_id != binding.previous_node_id
                        || existing.previous_address != binding.previous_address
                        || existing.next_node_id != binding.next_node_id
                        || existing.next_address != binding.next_address
                    {
                        continue;
                    }
                } else {
                    if onion_bindings.len() >= MAX_ONION_ROUTES {
                        continue;
                    }
                    onion_bindings.insert(bind.route_id.clone(), binding);
                }
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
                sessions.insert(init.from.clone(), Session { root_key: key, key, public_key, counter: 0 });

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


            if let Some(payload) = message.strip_prefix(ONION_PREFIX).and_then(|r| r.strip_prefix(' ')) {
                let Ok(packet) = serde_json::from_str::<onion::OnionPacket>(payload) else { continue };
                let Some(binding) = onion_bindings.get(&packet.route_id).cloned() else { continue };
                let now = now_secs();
                if packet.hop_index != binding.hop_index
                    || packet.expires_at > binding.expires_at
                    || packet.expires_at < now
                {
                    continue;
                }
                let Some(source_session) = sessions.get(&binding.source_id) else { continue };

                match onion::peel(
                    &mut onion_cache,
                    &packet,
                    &packet.route_id,
                    binding.hop_index,
                    &source_session.root_key,
                    now,
                ) {
                    Ok(onion::PeelResult::Forward(forward)) => {
                        if forward.next_node_id != binding.next_node_id
                            || forward.next_address != binding.next_address
                        {
                            continue;
                        }
                        let Ok(next_addr) = forward.next_address.parse::<SocketAddr>() else { continue };
                        let Ok(body) = serde_json::to_string(&forward.packet) else { continue };
                        if body.as_bytes().len() > onion::MAX_ONION_BYTES {
                            continue;
                        }
                        let _ = socket.send_to(
                            format!("{} {}", ONION_PREFIX, body).as_bytes(),
                            next_addr,
                        );
                    }
                    Ok(onion::PeelResult::Deliver(deliver)) => {
                        if deliver.destination_id != binding.next_node_id
                            || deliver.destination_address != binding.next_address
                        {
                            continue;
                        }
                        let delivery = onion::OnionDelivery {
                            route_id: packet.route_id.clone(),
                            packet_id: packet.packet_id.clone(),
                            payload: deliver.payload,
                        };
                        let Ok(body) = serde_json::to_string(&delivery) else { continue };
                        if body.as_bytes().len() > MAX_WIRE_BYTES {
                            continue;
                        }
                        let Ok(next_addr) = deliver.destination_address.parse::<SocketAddr>() else { continue };
                        let _ = socket.send_to(
                            format!("{} {}", ONION_DELIVERY_PREFIX, body).as_bytes(),
                            next_addr,
                        );
                    }
                    Err(_) => {}
                }
                continue;
            }

            if let Some(payload) = message.strip_prefix(ONION_DELIVERY_PREFIX).and_then(|r| r.strip_prefix(' ')) {
                let Ok(delivery) = serde_json::from_str::<onion::OnionDelivery>(payload) else { continue };
                if delivery.payload.len() > MAX_WIRE_BYTES {
                    continue;
                }
                let Ok(envelope) = serde_json::from_slice::<WireEnvelope>(&delivery.payload) else { continue };
                if envelope.to != node_id
                    || envelope.from == node_id
                    || envelope.message_id.trim().is_empty()
                    || !fresh_timestamp(envelope.timestamp)
                {
                    continue;
                }

                let cutoff = now_secs().saturating_sub(REPLAY_WINDOW_SECS);
                seen_messages.retain(|_, ts| *ts >= cutoff);
                if seen_messages.contains_key(&envelope.message_id) {
                    continue;
                }

                let Some(session) = sessions.get_mut(&envelope.from) else { continue };
                if envelope.counter != session.counter + 1 {
                    continue;
                }
                let signed = [
                    crypto::PROTOCOL,
                    "message",
                    envelope.message_id.as_str(),
                    envelope.from.as_str(),
                    envelope.to.as_str(),
                    &envelope.timestamp.to_string(),
                    &envelope.counter.to_string(),
                    envelope.nonce.as_str(),
                    envelope.ciphertext.as_str(),
                ]
                .join("|")
                .into_bytes();
                let Ok(signature) = STANDARD.decode(&envelope.signature) else { continue };
                if !crypto::verify_signature(&session.public_key, &signed, &signature) {
                    continue;
                }
                let message_key = match crypto::ratchet_key(&session.key, envelope.counter) {
                    Ok(key) => key,
                    Err(_) => continue,
                };
                let associated = aad(
                    &envelope.message_id,
                    &envelope.from,
                    &envelope.to,
                    envelope.timestamp,
                    envelope.counter,
                );
                let Ok(plaintext) = crypto::decrypt(
                    &message_key,
                    &associated,
                    &envelope.nonce,
                    &envelope.ciphertext,
                ) else { continue };
                if plaintext.len() > MAX_CHAT_BYTES {
                    continue;
                }
                let Ok(text) = String::from_utf8(plaintext) else { continue };

                let Ok(next) = crypto::ratchet_chain(&session.key, envelope.counter) else {
                    continue;
                };
                session.key = next;
                session.counter = envelope.counter;
                seen_messages.insert(envelope.message_id.clone(), envelope.timestamp);

                let ack_signed = [
                    crypto::PROTOCOL,
                    "ack",
                    envelope.message_id.as_str(),
                    node_id.as_str(),
                    envelope.from.as_str(),
                ]
                .join("|");
                let ack = LanAck {
                    message_id: envelope.message_id.clone(),
                    from: node_id.clone(),
                    to: envelope.from.clone(),
                    signature: STANDARD.encode(crypto::sign(&identity, ack_signed.as_bytes())),
                };
                if let Ok(body) = serde_json::to_string(&OnionReverseAck {
                    route_id: delivery.route_id,
                    packet_id: delivery.packet_id,
                    hop_index: u8::MAX,
                    ack,
                }) {
                    let _ = socket.send_to(
                        format!("{} {}", ONION_REVERSE_PREFIX, body).as_bytes(),
                        peer_addr,
                    );
                }
                let _ = tx.send(LanEvent::Chat {
                    message_id: envelope.message_id,
                    node_id: envelope.from,
                    message: text,
                });
                continue;
            }

            if let Some(payload) = message.strip_prefix(ONION_REVERSE_PREFIX).and_then(|r| r.strip_prefix(' ')) {
                let Ok(mut reverse) = serde_json::from_str::<OnionReverseAck>(payload) else { continue };
                let Some(binding) = onion_bindings.get(&reverse.route_id).cloned() else { continue };
                if reverse.ack.to != binding.source_id
                    || reverse.ack.message_id != reverse.packet_id
                    || peer_addr.to_string() != binding.next_address
                {
                    continue;
                }

                let expected_incoming = if binding.hop_index == onion::MAX_ONION_HOPS as u8 {
                    u8::MAX
                } else {
                    binding.hop_index
                };
                if reverse.hop_index != u8::MAX && reverse.hop_index != expected_incoming {
                    continue;
                }
                reverse.hop_index = binding.hop_index.saturating_sub(1);
                let Ok(body) = serde_json::to_string(&reverse) else { continue };
                let Ok(previous_addr) = binding.previous_address.parse::<SocketAddr>() else { continue };
                let _ = socket.send_to(
                    format!("{} {}", ONION_REVERSE_PREFIX, body).as_bytes(),
                    previous_addr,
                );
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
                if ack_state.load(Ordering::Acquire) {
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
                peers.push(LanPeer {
                    node_id: peer_id.to_string(),
                    address: addr.ip().to_string(),
                    version,
                    fingerprint: Some(crypto::fingerprint(&public_key)),
                    public_key: Some(public_key_b64.to_string()),
                    trusted: false,
                });
            }
            Err(_) => break,
        }
    }
    peers
}

fn ensure_onion_session(
    socket: &UdpSocket,
    identity: &NodeIdentity,
    peer: &OnionRoutePeer,
) -> Result<Session, String> {
    let peer_public_key = STANDARD
        .decode(&peer.public_key_b64)
        .map_err(|_| "invalid peer identity key".to_string())?;
    if crypto::node_id_from_public_key(&peer_public_key) != peer.node_id {
        return Err("peer identity key is invalid".into());
    }

    if let Ok(sessions) = send_sessions().lock() {
        if let Some(existing) = sessions.get(&peer.node_id) {
            if existing.public_key == peer_public_key {
                return Ok(existing.clone());
            }
        }
    }

    socket
        .set_read_timeout(Some(KEY_TIMEOUT))
        .map_err(|e| e.to_string())?;
    let (private, init_public) = crypto::ephemeral().map_err(|e| e.to_string())?;
    let init_public_b64 = STANDARD.encode(&init_public);
    let identity_public_b64 = STANDARD.encode(identity.public_key());
    let timestamp = now_secs();
    let init = KeyInit {
        from: identity.node_id(),
        to: peer.node_id.clone(),
        public_key: identity_public_b64.clone(),
        ephemeral_public_key: init_public_b64.clone(),
        timestamp,
        signature: STANDARD.encode(crypto::sign(
            identity,
            &signed_key_init(
                &identity.node_id(),
                &peer.node_id,
                &identity_public_b64,
                &init_public_b64,
                timestamp,
            ),
        )),
    };
    let body = serde_json::to_string(&init).map_err(|e| e.to_string())?;
    socket
        .send_to(
            format!("{} {}", KEY_INIT_PREFIX, body).as_bytes(),
            &peer.address,
        )
        .map_err(|e| e.to_string())?;

    let mut buffer = [0u8; 8192];
    let reply = loop {
        match socket.recv_from(&mut buffer) {
            Ok((size, _)) => {
                let Ok(text) = std::str::from_utf8(&buffer[..size]) else { continue };
                let Some(payload) = text
                    .strip_prefix(KEY_REPLY_PREFIX)
                    .and_then(|r| r.strip_prefix(' '))
                else {
                    continue;
                };
                let Ok(reply) = serde_json::from_str::<KeyReply>(payload) else {
                    continue;
                };
                if reply.from != peer.node_id
                    || reply.to != identity.node_id()
                    || !fresh_timestamp(reply.timestamp)
                    || reply.initiator_ephemeral_public_key != init_public_b64
                {
                    continue;
                }
                let Ok(sig) = STANDARD.decode(&reply.signature) else { continue };
                let signed = signed_key_reply(
                    &reply.from,
                    &reply.to,
                    &reply.initiator_ephemeral_public_key,
                    &reply.responder_ephemeral_public_key,
                    reply.timestamp,
                );
                if crypto::verify_signature(&peer_public_key, &signed, &sig) {
                    break reply;
                }
            }
            Err(_) => {
                clear_send_session(&peer.node_id);
                return Err("key exchange timeout".into());
            }
        }
    };

    let reply_public = STANDARD
        .decode(&reply.responder_ephemeral_public_key)
        .map_err(|_| "invalid key reply".to_string())?;
    let transcript = session_transcript(
        &identity.node_id(),
        &peer.node_id,
        &init_public,
        &reply_public,
    );
    let key = crypto::derive_session_key(private, &reply_public, &transcript)
        .map_err(|e| e.to_string())?;

    let session = Session {
        root_key: key,
        key,
        public_key: peer_public_key,
        counter: 0,
    };
    if let Ok(mut sessions) = send_sessions().lock() {
        sessions.insert(peer.node_id.clone(), session.clone());
    }
    Ok(session)
}

fn send_onion_route_bind(
    socket: &UdpSocket,
    identity: &NodeIdentity,
    peer: &OnionRoutePeer,
    route_id: &str,
    hop_index: u8,
    previous_node_id: &str,
    previous_address: &str,
    next_node_id: &str,
    next_address: &str,
    expires_at: u64,
) -> Result<(), String> {
    let mut bind = OnionRouteBind {
        route_id: route_id.to_string(),
        hop_index,
        source_id: identity.node_id(),
        source_public_key: STANDARD.encode(identity.public_key()),
        previous_node_id: previous_node_id.to_string(),
        previous_address: previous_address.to_string(),
        next_node_id: next_node_id.to_string(),
        next_address: next_address.to_string(),
        expires_at,
        signature: String::new(),
    };
    bind.signature = STANDARD.encode(crypto::sign(
        identity,
        &signed_onion_route_bind(&bind),
    ));
    let body = serde_json::to_string(&bind).map_err(|e| e.to_string())?;
    socket
        .send_to(
            format!("{} {}", ONION_BIND_PREFIX, body).as_bytes(),
            &peer.address,
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn send_onion_private_chat(
    identity: &NodeIdentity,
    destination: &OnionRoutePeer,
    relays: &[OnionRoutePeer],
    message: &str,
) -> LanSendStatus {
    let message_id = Uuid::new_v4().to_string();
    if message.trim().is_empty() {
        return LanSendStatus::Failed {
            message_id,
            peer_id: destination.node_id.clone(),
            reason: "empty message".into(),
        };
    }
    if message.as_bytes().len() > MAX_CHAT_BYTES {
        return LanSendStatus::Failed {
            message_id,
            peer_id: destination.node_id.clone(),
            reason: format!("message exceeds {} bytes", MAX_CHAT_BYTES),
        };
    }
    if relays.is_empty() || relays.len() > onion::MAX_ONION_HOPS {
        return LanSendStatus::Failed {
            message_id,
            peer_id: destination.node_id.clone(),
            reason: "invalid onion relay count".into(),
        };
    }

    let mut seen_ids = std::collections::HashSet::new();
    for peer in relays.iter().chain(std::iter::once(destination)) {
        if peer.node_id == identity.node_id() || !seen_ids.insert(peer.node_id.clone()) {
            return LanSendStatus::Failed {
                message_id,
                peer_id: destination.node_id.clone(),
                reason: "onion route contains duplicate node identity".into(),
            };
        }
        if peer.address.parse::<SocketAddr>().is_err() {
            return LanSendStatus::Failed {
                message_id,
                peer_id: destination.node_id.clone(),
                reason: "onion route contains invalid address".into(),
            };
        }
    }

    let socket = match UdpSocket::bind(("0.0.0.0", 0)) {
        Ok(socket) => socket,
        Err(e) => {
            return LanSendStatus::Failed {
                message_id,
                peer_id: destination.node_id.clone(),
                reason: e.to_string(),
            }
        }
    };
    let route_id = onion::new_route_id();
    let packet_id = onion::new_packet_id();
    let expires_at = now_secs() + 60;

    let mut relay_sessions = Vec::with_capacity(relays.len());
    for relay in relays {
        match ensure_onion_session(&socket, identity, relay) {
            Ok(session) => relay_sessions.push(session),
            Err(reason) => {
                return LanSendStatus::Failed {
                    message_id,
                    peer_id: destination.node_id.clone(),
                    reason: format!("relay session {}: {}", relay.node_id, reason),
                }
            }
        }
    }
    let mut destination_session = match ensure_onion_session(&socket, identity, destination) {
        Ok(session) => session,
        Err(reason) => {
            return LanSendStatus::Failed {
                message_id,
                peer_id: destination.node_id.clone(),
                reason: format!("destination session: {}", reason),
            }
        }
    };

    let source_socket_address = socket
        .local_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|_| "127.0.0.1:0".into());

    for (index, relay) in relays.iter().enumerate() {
        let previous_node_id = if index == 0 {
            identity.node_id()
        } else {
            &relays[index - 1].node_id
        };
        let previous_address = if index == 0 {
            source_socket_address.as_str()
        } else {
            &relays[index - 1].address
        };
        let next_node_id = if index + 1 < relays.len() {
            &relays[index + 1].node_id
        } else {
            &destination.node_id
        };
        let next_address = if index + 1 < relays.len() {
            &relays[index + 1].address
        } else {
            &destination.address
        };

        if let Err(reason) = send_onion_route_bind(
            &socket,
            identity,
            relay,
            &route_id,
            index as u8,
            previous_node_id,
            previous_address,
            next_node_id,
            next_address,
            expires_at,
        ) {
            return LanSendStatus::Failed {
                message_id,
                peer_id: destination.node_id.clone(),
                reason: format!("relay bind {}: {}", relay.node_id, reason),
            };
        }
    }

    let timestamp = now_secs();
    let counter = destination_session.counter + 1;
    let associated = aad(
        &message_id,
        &identity.node_id(),
        &destination.node_id,
        timestamp,
        counter,
    );
    let message_key = match crypto::ratchet_key(&destination_session.key, counter) {
        Ok(key) => key,
        Err(reason) => {
            return LanSendStatus::Failed {
                message_id,
                peer_id: destination.node_id.clone(),
                reason: reason.into(),
            }
        }
    };
    let (nonce, ciphertext) =
        match crypto::encrypt(&message_key, &associated, message.as_bytes()) {
            Ok(value) => value,
            Err(reason) => {
                return LanSendStatus::Failed {
                    message_id,
                    peer_id: destination.node_id.clone(),
                    reason: reason.into(),
                }
            }
        };
    let signed = [
        crypto::PROTOCOL,
        "message",
        message_id.as_str(),
        identity.node_id().as_str(),
        destination.node_id.as_str(),
        &timestamp.to_string(),
        &counter.to_string(),
        nonce.as_str(),
        ciphertext.as_str(),
    ]
    .join("|")
    .into_bytes();
    let envelope = WireEnvelope {
        message_id: message_id.clone(),
        from: identity.node_id(),
        to: destination.node_id.clone(),
        timestamp,
        counter,
        nonce,
        ciphertext,
        signature: STANDARD.encode(crypto::sign(identity, &signed)),
    };
    let e2e_payload = match serde_json::to_vec(&envelope) {
        Ok(body) if body.len() <= MAX_WIRE_BYTES => body,
        Ok(_) => {
            return LanSendStatus::Failed {
                message_id,
                peer_id: destination.node_id.clone(),
                reason: "encrypted envelope exceeds wire limit".into(),
            }
        }
        Err(e) => {
            return LanSendStatus::Failed {
                message_id,
                peer_id: destination.node_id.clone(),
                reason: e.to_string(),
            }
        }
    };

    let hops: Vec<onion::OnionHop> = relays
        .iter()
        .map(|peer| onion::OnionHop {
            node_id: peer.node_id.clone(),
            address: peer.address.clone(),
        })
        .collect();
    let relay_keys: Vec<[u8; 32]> =
        relay_sessions.iter().map(|session| session.root_key).collect();
    let packet = match onion::wrap(
        &route_id,
        &packet_id,
        expires_at,
        &e2e_payload,
        &hops,
        &relay_keys,
        &destination.node_id,
        &destination.address,
    ) {
        Ok(packet) => packet,
        Err(reason) => {
            return LanSendStatus::Failed {
                message_id,
                peer_id: destination.node_id.clone(),
                reason: reason.into(),
            }
        }
    };
    let body = match serde_json::to_string(&packet) {
        Ok(body) if body.len() <= onion::MAX_ONION_BYTES => body,
        Ok(_) => {
            return LanSendStatus::Failed {
                message_id,
                peer_id: destination.node_id.clone(),
                reason: "onion packet exceeds wire limit".into(),
            }
        }
        Err(e) => {
            return LanSendStatus::Failed {
                message_id,
                peer_id: destination.node_id.clone(),
                reason: e.to_string(),
            }
        }
    };

    #[cfg(test)]
    remember_sent_wire(&format!("{} {}", ONION_PREFIX, body));

    let _ = socket.set_read_timeout(Some(CHAT_ACK_TIMEOUT));
    if let Err(e) = socket.send_to(
        format!("{} {}", ONION_PREFIX, body).as_bytes(),
        &relays[0].address,
    ) {
        return LanSendStatus::Failed {
            message_id,
            peer_id: destination.node_id.clone(),
            reason: e.to_string(),
        };
    }

    let destination_public_key = match STANDARD.decode(&destination.public_key_b64) {
        Ok(key) => key,
        Err(_) => {
            return LanSendStatus::Failed {
                message_id,
                peer_id: destination.node_id.clone(),
                reason: "invalid destination identity key".into(),
            }
        }
    };

    let mut buffer = [0u8; 8192];
    loop {
        match socket.recv_from(&mut buffer) {
            Ok((size, _)) => {
                let Ok(text) = std::str::from_utf8(&buffer[..size]) else { continue };
                let Some(ack_payload) = text
                    .strip_prefix(ONION_REVERSE_PREFIX)
                    .and_then(|rest| rest.strip_prefix(' '))
                else {
                    continue;
                };
                let Ok(reverse) = serde_json::from_str::<OnionReverseAck>(ack_payload) else {
                    continue;
                };
                if reverse.route_id != route_id
                    || reverse.packet_id != packet_id
                    || reverse.hop_index != 0
                    || reverse.ack.message_id != message_id
                    || reverse.ack.to != identity.node_id()
                    || reverse.ack.from != destination.node_id
                {
                    continue;
                }
                let Ok(sig) = STANDARD.decode(&reverse.ack.signature) else { continue };
                let ack_signed = [
                    crypto::PROTOCOL,
                    "ack",
                    reverse.ack.message_id.as_str(),
                    reverse.ack.from.as_str(),
                    reverse.ack.to.as_str(),
                ]
                .join("|");
                if !crypto::verify_signature(
                    &destination_public_key,
                    ack_signed.as_bytes(),
                    &sig,
                ) {
                    continue;
                }
                let Ok(next) =
                    crypto::ratchet_chain(&destination_session.key, counter)
                else {
                    return LanSendStatus::Failed {
                        message_id,
                        peer_id: destination.node_id.clone(),
                        reason: "cannot advance destination ratchet".into(),
                    };
                };
                destination_session.key = next;
                destination_session.counter = counter;
                if let Ok(mut sessions) = send_sessions().lock() {
                    sessions.insert(destination.node_id.clone(), destination_session.clone());
                }
                return LanSendStatus::Delivered {
                    message_id,
                    peer_id: destination.node_id.clone(),
                };
            }
            Err(_) => {
                clear_send_session(&destination.node_id);
                return LanSendStatus::TimedOut {
                    message_id,
                    peer_id: destination.node_id.clone(),
                };
            }
        }
    }
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
        session = Some(Session { root_key: key, key, public_key: peer_public_key.clone(), counter: 0 });
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
                self.lan_peers = peers.into_iter().filter_map(|mut peer| {
                    let Some(key) = peer.public_key.as_deref() else { return None; };
                    match self.store.peer_pin(&peer.node_id) {
                        Some(pinned) if pinned == key => {
                            peer.trusted = true;
                            Some(peer)
                        }
                        Some(_) => {
                            self.add_event("SECURITY", format!("Pinned-key conflict for {}", peer.node_id));
                            None
                        }
                        None => Some(peer),
                    }
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

    pub(crate) fn trust_lan_peer(&mut self, peer_id: &str) {
        let Some(peer) = self.lan_peers.iter().find(|p| p.node_id == peer_id).cloned() else {
            self.notify("LAN PEER NOT FOUND");
            return;
        };

        let Some(public_key) = peer.public_key.as_deref() else {
            self.notify("PEER HAS NO AUTHENTICATED IDENTITY KEY");
            return;
        };

        if !self.store.trust_peer_key(&peer.node_id, public_key) {
            self.add_event("SECURITY", format!("Rejected peer key replacement for {}", peer.node_id));
            self.notify("PEER KEY CONFLICT — TRUST REJECTED");
            return;
        }

        if let Some(current) = self.lan_peers.iter_mut().find(|p| p.node_id == peer.node_id) {
            current.trusted = true;
        }

        self.lan_target = Some(peer.node_id.clone());
        self.add_event(
            "SECURITY",
            format!(
                "Explicitly trusted LAN peer {} · fingerprint {}",
                peer.node_id,
                peer.fingerprint.as_deref().unwrap_or("UNKNOWN")
            ),
        );
        self.notify(format!("TRUSTED LAN PEER {}", peer.node_id));
    }

    pub(crate) fn send_lan_onion_chat(&mut self, message: &str) {
        if self.lan_send_task.is_some() {
            self.notify("LAN DELIVERY ALREADY IN PROGRESS");
            return;
        }
        let message = message.trim().to_string();
        if message.is_empty() {
            return;
        }

        let Some(target_id) = self.lan_target.clone() else {
            self.notify("SELECT A LAN DESTINATION FIRST");
            return;
        };
        let Some(destination) = self
            .lan_peers
            .iter()
            .find(|peer| peer.node_id == target_id && peer.trusted)
            .cloned()
        else {
            self.notify("DESTINATION MUST BE TRUSTED");
            return;
        };

        let relays: Vec<_> = self
            .lan_peers
            .iter()
            .filter(|peer| peer.trusted && peer.node_id != target_id)
            .take(crate::network::onion::MAX_ONION_HOPS)
            .cloned()
            .collect();

        if relays.is_empty() {
            self.notify("ONION ROUTING NEEDS AT LEAST ONE TRUSTED RELAY");
            return;
        }

        let destination = crate::network::lan::OnionRoutePeer {
            node_id: destination.node_id,
            address: format!("{}:{}", destination.address, LAN_DISCOVERY_PORT),
            public_key_b64: destination.public_key.unwrap_or_default(),
        };
        let relay_peers: Vec<crate::network::lan::OnionRoutePeer> = relays
            .into_iter()
            .filter_map(|peer| {
                Some(crate::network::lan::OnionRoutePeer {
                    node_id: peer.node_id,
                    address: format!("{}:{}", peer.address, LAN_DISCOVERY_PORT),
                    public_key_b64: peer.public_key?,
                })
            })
            .collect();

        if relay_peers.is_empty() {
            self.notify("NO TRUSTED RELAY WITH AN AUTHENTICATED KEY");
            return;
        }

        let (tx, rx) = mpsc::channel();
        self.lan_send_task = Some(rx);
        self.lan_delivery_status = format!(
            "ONION ROUTE · {} RELAY(S) → {}",
            relay_peers.len(),
            destination.node_id
        );
        let identity = self.identity.clone();

        thread::spawn(move || {
            let result =
                send_onion_private_chat(&identity, &destination, &relay_peers, &message);
            let _ = tx.send(result);
        });
    }

    pub(crate) fn send_lan_chat(&mut self, message: &str) {
        if self.lan_send_task.is_some() { self.notify("LAN DELIVERY ALREADY IN PROGRESS"); return; }
        let message = message.trim().to_string();
        if message.is_empty() { return; }
        let Some(target_id) = self.lan_target.clone() else { self.notify("SELECT A LAN PEER FIRST"); return };
        let Some(peer) = self.lan_peers.iter().find(|p| p.node_id == target_id).cloned() else { self.notify("LAN TARGET IS NO LONGER AVAILABLE"); return };
        if !peer.trusted {
            self.notify("CONFIRM PEER FINGERPRINT FIRST");
            return;
        }
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
        let _guard = test_guard();
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
        let _guard = test_guard();
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
    fn routed_onion_chat_crosses_two_relays_and_returns_e2e_ack() {
        let _guard = test_guard();

        let source = NodeIdentity::generate_for_test();
        let relay_a = NodeIdentity::generate_for_test();
        let relay_b = NodeIdentity::generate_for_test();
        let destination = NodeIdentity::generate_for_test();

        let relay_a_port = free_port();
        let relay_b_port = free_port();
        let destination_port = free_port();

        let _relay_a_events =
            spawn_listener_on_port(relay_a.node_id(), relay_a.clone(), relay_a_port);
        let _relay_b_events =
            spawn_listener_on_port(relay_b.node_id(), relay_b.clone(), relay_b_port);
        let destination_events = spawn_listener_on_port(
            destination.node_id(),
            destination.clone(),
            destination_port,
        );
        thread::sleep(Duration::from_millis(60));

        let destination_peer = OnionRoutePeer {
            node_id: destination.node_id(),
            address: format!("127.0.0.1:{destination_port}"),
            public_key_b64: STANDARD.encode(destination.public_key()),
        };
        let relays = vec![
            OnionRoutePeer {
                node_id: relay_a.node_id(),
                address: format!("127.0.0.1:{relay_a_port}"),
                public_key_b64: STANDARD.encode(relay_a.public_key()),
            },
            OnionRoutePeer {
                node_id: relay_b.node_id(),
                address: format!("127.0.0.1:{relay_b_port}"),
                public_key_b64: STANDARD.encode(relay_b.public_key()),
            },
        ];

        let result = send_onion_private_chat(
            &source,
            &destination_peer,
            &relays,
            "hello through the onion",
        );
        assert!(matches!(
            result,
            LanSendStatus::Delivered { .. }
        ));

        wait_for_chat(&destination_events, "hello through the onion");
    }

    #[test]
    fn live_udp_rejects_stale_timestamp_and_wrong_recipient() {
        let _guard = test_guard();
        let alice = NodeIdentity::generate_for_test();
        let bob = NodeIdentity::generate_for_test();
        let bob_id = bob.node_id();
        let bob_port = free_port();
        let bob_events = spawn_listener_on_port(bob_id.clone(), bob.clone(), bob_port);
        thread::sleep(Duration::from_millis(40));

        let stale_id = Uuid::new_v4().to_string();
        let stale_ts = now_secs().saturating_sub(REPLAY_WINDOW_SECS + 1);
        let stale_signed = [
            crypto::PROTOCOL,
            "message",
            stale_id.as_str(),
            alice.node_id().as_str(),
            bob_id.as_str(),
            &stale_ts.to_string(),
            "1",
            "invalid-nonce",
            "invalid-ciphertext",
        ].join("|");
        let stale = WireEnvelope {
            message_id: stale_id,
            from: alice.node_id(),
            to: bob_id.clone(),
            timestamp: stale_ts,
            counter: 1,
            nonce: "invalid-nonce".into(),
            ciphertext: "invalid-ciphertext".into(),
            signature: STANDARD.encode(crypto::sign(&alice, stale_signed.as_bytes())),
        };

        let socket = UdpSocket::bind(("127.0.0.1", 0)).unwrap();
        socket
            .send_to(
                format!("{} {}", CHAT_PREFIX, serde_json::to_string(&stale).unwrap()).as_bytes(),
                ("127.0.0.1", bob_port),
            )
            .unwrap();
        assert!(bob_events.recv_timeout(Duration::from_millis(250)).is_err());

        let wrong_id = Uuid::new_v4().to_string();
        let wrong_ts = now_secs();
        let wrong_to = "cyb-wrong-recipient".to_string();
        let wrong_signed = [
            crypto::PROTOCOL,
            "message",
            wrong_id.as_str(),
            alice.node_id().as_str(),
            wrong_to.as_str(),
            &wrong_ts.to_string(),
            "1",
            "invalid-nonce",
            "invalid-ciphertext",
        ].join("|");
        let wrong = WireEnvelope {
            message_id: wrong_id,
            from: alice.node_id(),
            to: wrong_to,
            timestamp: wrong_ts,
            counter: 1,
            nonce: "invalid-nonce".into(),
            ciphertext: "invalid-ciphertext".into(),
            signature: STANDARD.encode(crypto::sign(&alice, wrong_signed.as_bytes())),
        };

        socket
            .send_to(
                format!("{} {}", CHAT_PREFIX, serde_json::to_string(&wrong).unwrap()).as_bytes(),
                ("127.0.0.1", bob_port),
            )
            .unwrap();
        assert!(bob_events.recv_timeout(Duration::from_millis(250)).is_err());
    }

    #[test]
    fn forged_ack_signature_is_rejected() {
        let alice = NodeIdentity::generate_for_test();
        let bob = NodeIdentity::generate_for_test();
        let message_id = Uuid::new_v4().to_string();

        let signed = [
            crypto::PROTOCOL,
            "ack",
            message_id.as_str(),
            bob.node_id().as_str(),
            alice.node_id().as_str(),
        ]
        .join("|");

        let forged = crypto::sign(&alice, signed.as_bytes());
        assert!(!crypto::verify_signature(
            bob.public_key(),
            signed.as_bytes(),
            &forged,
        ));

        let valid = crypto::sign(&bob, signed.as_bytes());
        assert!(crypto::verify_signature(
            bob.public_key(),
            signed.as_bytes(),
            &valid,
        ));
    }

    #[test]
    fn live_udp_replay_and_ciphertext_tampering_are_rejected() {
        let _guard = test_guard();
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
