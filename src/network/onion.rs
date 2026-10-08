//! Multi-hop onion packet construction and relay peeling for CybChat.
//!
//! Long-term node identity remains Ed25519. Each adjacent hop uses its own
//! authenticated X25519 session root; this module derives a separate AEAD
//! layer key from that root for the route packet. A relay learns only its
//! immediate next hop and never the end-to-end payload.

use base64::Engine as _;
use crate::crypto;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::net::{SocketAddr, UdpSocket};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

pub(crate) const MAX_ONION_HOPS: usize = 4;
pub(crate) const MAX_ONION_BYTES: usize = 4096;
pub(crate) const ONION_TTL_SECS: u64 = 120;
const REPLAY_CACHE_LIMIT: usize = 512;
const MAX_ROUTE_ID_BYTES: usize = 64;
const MAX_PACKET_ID_BYTES: usize = 64;
const MAX_SESSION_ID_BYTES: usize = 64;
const MAX_NODE_ID_BYTES: usize = 128;
const MAX_ADDRESS_BYTES: usize = 128;
const MAX_NONCE_BYTES: usize = 24;
const MIN_CIPHERTEXT_BYTES: usize = 16;
const MAX_PAYLOAD_BYTES: usize = 3072;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OnionHop {
    pub(crate) node_id: String,
    pub(crate) address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct OnionPacket {
    pub(crate) version: u8,
    pub(crate) route_id: String,
    pub(crate) packet_id: String,
    pub(crate) session_id: String,
    pub(crate) hop_index: u8,
    pub(crate) expires_at: u64,
    pub(crate) nonce: String,
    pub(crate) ciphertext: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ForwardPacket {
    pub(crate) next_node_id: String,
    pub(crate) next_address: String,
    pub(crate) packet: OnionPacket,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DeliverPacket {
    pub(crate) destination_id: String,
    pub(crate) destination_address: String,
    pub(crate) payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PeelResult {
    Forward(ForwardPacket),
    Deliver(DeliverPacket),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct OnionReversePacket {
    pub(crate) version: u8,
    pub(crate) route_id: String,
    pub(crate) packet_id: String,
    pub(crate) hop_index: u8,
    pub(crate) expires_at: u64,
    pub(crate) nonce: String,
    pub(crate) ciphertext: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReverseForward {
    pub(crate) previous_node_id: String,
    pub(crate) previous_address: String,
    pub(crate) payload: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct OnionRelayTable {
    bindings: Arc<Mutex<HashMap<(String, u8), [u8; 32]>>>,
}

impl OnionRelayTable {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn bind_route(
        &self,
        route_id: &str,
        hop_index: u8,
        session_key: [u8; 32],
    ) -> Result<(), &'static str> {
        let mut bindings = self.bindings.lock().map_err(|_| "relay table poisoned")?;
        if bindings.len() >= 256 && !bindings.contains_key(&(route_id.to_string(), hop_index)) {
            return Err("relay route table full");
        }
        bindings.insert((route_id.to_string(), hop_index), session_key);
        Ok(())
    }

    fn key_for(&self, route_id: &str, hop_index: u8) -> Option<[u8; 32]> {
        self.bindings
            .lock()
            .ok()
            .and_then(|bindings| bindings.get(&(route_id.to_string(), hop_index)).copied())
    }

    pub(crate) fn remove_route(&self, route_id: &str) {
        if let Ok(mut bindings) = self.bindings.lock() {
            bindings.retain(|(id, _), _| id != route_id);
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct OnionDelivery {
    pub(crate) route_id: String,
    pub(crate) packet_id: String,
    pub(crate) payload: Vec<u8>,
}

pub(crate) const ONION_PREFIX: &str = "CYBOS_ONION";
pub(crate) const ONION_DELIVERY_PREFIX: &str = "CYBOS_ONION_DELIVERY";

pub(crate) fn spawn_udp_relay(
    bind_addr: SocketAddr,
    table: OnionRelayTable,
    stop: Arc<AtomicBool>,
) -> std::io::Result<thread::JoinHandle<()>> {
    let socket = UdpSocket::bind(bind_addr)?;
    socket.set_read_timeout(Some(Duration::from_millis(100)))?;

    let handle = thread::spawn(move || {
        let mut cache = OnionRelayCache::new();
        let mut buffer = [0u8; MAX_ONION_BYTES + 1024];

        while stop.load(Ordering::Acquire) {
            let Ok((size, peer_addr)) = socket.recv_from(&mut buffer) else {
                continue;
            };
            if size > MAX_ONION_BYTES + 512 {
                continue;
            }
            let Ok(text) = std::str::from_utf8(&buffer[..size]) else {
                continue;
            };
            let Some(payload) = text
                .strip_prefix(ONION_PREFIX)
                .and_then(|rest| rest.strip_prefix(' '))
            else {
                continue;
            };
            let Ok(packet) = serde_json::from_str::<OnionPacket>(payload) else {
                continue;
            };
            let Some(key) = table.key_for(&packet.route_id, packet.hop_index) else {
                continue;
            };

            let now = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                Ok(value) => value.as_secs(),
                Err(_) => continue,
            };

            match peel(
                &mut cache,
                &packet,
                &packet.route_id,
                packet.hop_index,
                &key,
                now,
            ) {
                Ok(PeelResult::Forward(forward)) => {
                    let Ok(next_addr) = forward.next_address.parse::<SocketAddr>() else {
                        continue;
                    };
                    let Ok(body) = serde_json::to_string(&forward.packet) else {
                        continue;
                    };
                    let _ = socket.send_to(
                        format!("{} {}", ONION_PREFIX, body).as_bytes(),
                        next_addr,
                    );
                }
                Ok(PeelResult::Deliver(delivery)) => {
                    let Ok(destination) = delivery.destination_address.parse::<SocketAddr>() else {
                        continue;
                    };
                    let envelope = OnionDelivery {
                        route_id: packet.route_id.clone(),
                        packet_id: packet.packet_id.clone(),
                        payload: delivery.payload,
                    };
                    let Ok(body) = serde_json::to_string(&envelope) else {
                        continue;
                    };
                    let _ = socket.send_to(
                        format!("{} {}", ONION_DELIVERY_PREFIX, body).as_bytes(),
                        destination,
                    );
                }
                Err(_) => {
                    let _ = peer_addr;
                }
            }
        }
    });

    Ok(handle)
}

#[derive(Clone, Debug)]
pub(crate) struct OnionRelayCache {
    seen: HashMap<String, u64>,
    order: VecDeque<String>,
}

impl OnionRelayCache {
    pub(crate) fn new() -> Self {
        Self {
            seen: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    fn purge(&mut self, now: u64) {
        self.seen.retain(|_, expiry| *expiry >= now);
        while let Some(oldest) = self.order.front() {
            if self.seen.contains_key(oldest) {
                break;
            }
            self.order.pop_front();
        }
    }

    fn mark(&mut self, cache_key: String, expires_at: u64) {
        if self.seen.len() >= REPLAY_CACHE_LIMIT {
            if let Some(oldest) = self.order.pop_front() {
                self.seen.remove(&oldest);
            }
        }
        self.seen.insert(cache_key.clone(), expires_at);
        self.order.push_back(cache_key);
    }
}

fn bounded_text(value: &str, max_bytes: usize) -> bool {
    !value.is_empty() && value.len() <= max_bytes && value.bytes().all(|byte| byte >= 0x20 && byte != 0x7f)
}

fn valid_base64_field(value: &str, max_decoded: usize, min_decoded: usize) -> bool {
    let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(value) else {
        return false;
    };
    decoded.len() >= min_decoded && decoded.len() <= max_decoded
}

fn validate_packet_shape(packet: &OnionPacket) -> Result<(), &'static str> {
    if packet.version != 1 {
        return Err("unsupported onion version");
    }
    if !bounded_text(&packet.route_id, MAX_ROUTE_ID_BYTES)
        || !bounded_text(&packet.packet_id, MAX_PACKET_ID_BYTES)
        || !bounded_text(&packet.session_id, MAX_SESSION_ID_BYTES)
        || !valid_base64_field(&packet.nonce, MAX_NONCE_BYTES, 12)
        || !valid_base64_field(
            &packet.ciphertext,
            MAX_ONION_BYTES,
            MIN_CIPHERTEXT_BYTES,
        )
    {
        return Err("invalid onion packet fields");
    }
    Ok(())
}

fn validate_reverse_shape(packet: &OnionReversePacket) -> Result<(), &'static str> {
    if packet.version != 1 {
        return Err("unsupported reverse onion version");
    }
    if !bounded_text(&packet.route_id, MAX_ROUTE_ID_BYTES)
        || !bounded_text(&packet.packet_id, MAX_PACKET_ID_BYTES)
        || !valid_base64_field(&packet.nonce, MAX_NONCE_BYTES, 12)
        || !valid_base64_field(
            &packet.ciphertext,
            MAX_ONION_BYTES,
            MIN_CIPHERTEXT_BYTES,
        )
    {
        return Err("invalid reverse onion packet fields");
    }
    Ok(())
}

fn validate_frame(frame: &LayerFrame) -> Result<(), &'static str> {
    match frame {
        LayerFrame::Forward {
            next_node_id,
            next_address,
            packet,
        } => {
            if !bounded_text(next_node_id, MAX_NODE_ID_BYTES)
                || !bounded_text(next_address, MAX_ADDRESS_BYTES)
            {
                return Err("invalid onion forwarding target");
            }
            validate_packet_shape(packet)?;
        }
        LayerFrame::Deliver {
            destination_id,
            destination_address,
            payload,
        } => {
            if !bounded_text(destination_id, MAX_NODE_ID_BYTES)
                || !bounded_text(destination_address, MAX_ADDRESS_BYTES)
                || payload.is_empty()
                || payload.len() > MAX_PAYLOAD_BYTES
            {
                return Err("invalid onion delivery frame");
            }
        }
    }
    Ok(())
}

fn validate_reverse_frame(frame: &ReverseFrame) -> Result<(), &'static str> {
    if !bounded_text(&frame.previous_node_id, MAX_NODE_ID_BYTES)
        || !bounded_text(&frame.previous_address, MAX_ADDRESS_BYTES)
        || frame.payload.is_empty()
        || frame.payload.len() > MAX_PAYLOAD_BYTES
    {
        return Err("invalid reverse onion frame");
    }
    Ok(())
}

pub(crate) fn new_route_id() -> String {
    Uuid::new_v4().to_string()
}

pub(crate) fn new_packet_id() -> String {
    Uuid::new_v4().to_string()
}

fn layer_aad(
    route_id: &str,
    packet_id: &str,
    hop_index: u8,
    expires_at: u64,
    direction: &str,
) -> Vec<u8> {
    [
        crypto::PROTOCOL,
        "onion-layer-v1",
        route_id,
        packet_id,
        &hop_index.to_string(),
        &expires_at.to_string(),
        direction,
    ]
    .join("|")
    .into_bytes()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum LayerFrame {
    Forward {
        next_node_id: String,
        next_address: String,
        packet: OnionPacket,
    },
    Deliver {
        destination_id: String,
        destination_address: String,
        payload: Vec<u8>,
    },
}

fn encode_frame(frame: &LayerFrame) -> Result<Vec<u8>, &'static str> {
    serde_json::to_vec(frame).map_err(|_| "cannot encode onion frame")
}

pub(crate) fn wrap(
    route_id: &str,
    packet_id: &str,
    expires_at: u64,
    payload: &[u8],
    hops: &[OnionHop],
    hop_session_ids: &[String],
    hop_session_keys: &[[u8; 32]],
    destination_id: &str,
    destination_address: &str,
) -> Result<OnionPacket, &'static str> {
    if hops.is_empty() || hops.len() > MAX_ONION_HOPS {
        return Err("invalid onion hop count");
    }
    if hops.len() != hop_session_keys.len() || hops.len() != hop_session_ids.len() {
        return Err("onion hop/session metadata count mismatch");
    }
    if hop_session_ids.iter().any(|id| id.trim().is_empty() || id.len() > 64) {
        return Err("invalid onion session id");
    }
    let unique_session_ids: HashSet<&str> =
        hop_session_ids.iter().map(String::as_str).collect();
    if unique_session_ids.len() != hop_session_ids.len() {
        return Err("duplicate onion session id");
    }
    if payload.is_empty() {
        return Err("empty onion payload");
    }
    if expires_at == 0 {
        return Err("invalid onion expiry");
    }

    let mut inner: Option<OnionPacket> = None;

    for index in (0..hops.len()).rev() {
        let hop_index = index as u8;
        let next_node_id = if index + 1 < hops.len() {
            hops[index + 1].node_id.clone()
        } else {
            destination_id.to_string()
        };
        let next_address = if index + 1 < hops.len() {
            hops[index + 1].address.clone()
        } else {
            destination_address.to_string()
        };

        let frame = match inner.take() {
            Some(packet) => LayerFrame::Forward {
                next_node_id,
                next_address,
                packet,
            },
            None => LayerFrame::Deliver {
                destination_id: destination_id.to_string(),
                destination_address: destination_address.to_string(),
                payload: payload.to_vec(),
            },
        };

        let frame_bytes = encode_frame(&frame)?;
        let key = crypto::onion_layer_key(
            &hop_session_keys[index],
            route_id,
            packet_id,
            hop_index,
            "forward",
        )?;
        let aad = layer_aad(route_id, packet_id, hop_index, expires_at, "forward");
        let (nonce, ciphertext) = crypto::encrypt(&key, &aad, &frame_bytes)?;

        let packet = OnionPacket {
            version: 1,
            route_id: route_id.to_string(),
            packet_id: packet_id.to_string(),
            session_id: hop_session_ids[index].clone(),
            hop_index,
            expires_at,
            nonce,
            ciphertext,
        };

        let encoded_len = serde_json::to_vec(&packet)
            .map_err(|_| "cannot size onion packet")?
            .len();
        if encoded_len > MAX_ONION_BYTES {
            return Err("onion packet exceeds wire limit");
        }
        inner = Some(packet);
    }

    inner.ok_or("onion packet construction failed")
}

pub(crate) fn peel(
    relay: &mut OnionRelayCache,
    packet: &OnionPacket,
    expected_route_id: &str,
    expected_hop_index: u8,
    hop_session_key: &[u8; 32],
    now: u64,
) -> Result<PeelResult, &'static str> {
    validate_packet_shape(packet)?;
    if packet.route_id != expected_route_id {
        return Err("unexpected onion route");
    }
    if packet.hop_index != expected_hop_index {
        return Err("unexpected onion hop index");
    }
    if packet.expires_at < now {
        return Err("onion packet expired");
    }
    if packet.expires_at - now > ONION_TTL_SECS {
        return Err("invalid onion expiry");
    }

    let cache_key = format!(
        "{}:{}:{}:{}",
        packet.route_id, packet.packet_id, packet.session_id, packet.hop_index
    );
    relay.purge(now);
    if relay.seen.contains_key(&cache_key) {
        return Err("onion packet replayed");
    }

    let key = crypto::onion_layer_key(
        hop_session_key,
        &packet.route_id,
        &packet.packet_id,
        packet.hop_index,
        "forward",
    )?;
    let aad = layer_aad(
        &packet.route_id,
        &packet.packet_id,
        packet.hop_index,
        packet.expires_at,
        "forward",
    );
    let frame_bytes = crypto::decrypt(&key, &aad, &packet.nonce, &packet.ciphertext)?;

    let frame = serde_json::from_slice::<LayerFrame>(&frame_bytes)
        .map_err(|_| "invalid onion frame")?;
    validate_frame(&frame)?;

    match frame {
        LayerFrame::Forward {
            next_node_id,
            next_address,
            packet: inner,
        } => {
            if inner.version != 1
                || inner.route_id != packet.route_id
                || inner.packet_id != packet.packet_id
                || inner.session_id.trim().is_empty()
                || inner.hop_index != packet.hop_index.saturating_add(1)
                || inner.expires_at != packet.expires_at
            {
                return Err("invalid nested onion packet");
            }
            relay.mark(cache_key, packet.expires_at);
            Ok(PeelResult::Forward(ForwardPacket {
                next_node_id,
                next_address,
                packet: inner,
            }))
        }
        LayerFrame::Deliver {
            destination_id,
            destination_address,
            payload,
        } => {
            relay.mark(cache_key, packet.expires_at);
            Ok(PeelResult::Deliver(DeliverPacket {
                destination_id,
                destination_address,
                payload,
            }))
        }
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReverseFrame {
    previous_node_id: String,
    previous_address: String,
    payload: Vec<u8>,
}

pub(crate) fn wrap_reverse_hop(
    route_id: &str,
    packet_id: &str,
    hop_index: u8,
    expires_at: u64,
    payload: &[u8],
    previous_node_id: &str,
    previous_address: &str,
    hop_session_key: &[u8; 32],
) -> Result<OnionReversePacket, &'static str> {
    if payload.is_empty() {
        return Err("empty reverse onion payload");
    }
    if previous_node_id.is_empty() || previous_address.is_empty() {
        return Err("invalid reverse onion target");
    }
    let frame = ReverseFrame {
        previous_node_id: previous_node_id.to_string(),
        previous_address: previous_address.to_string(),
        payload: payload.to_vec(),
    };
    let bytes = serde_json::to_vec(&frame).map_err(|_| "cannot encode reverse onion frame")?;
    let key = crypto::onion_layer_key(
        hop_session_key,
        route_id,
        packet_id,
        hop_index,
        "reverse",
    )?;
    let aad = layer_aad(route_id, packet_id, hop_index, expires_at, "reverse");
    let (nonce, ciphertext) = crypto::encrypt(&key, &aad, &bytes)?;
    Ok(OnionReversePacket {
        version: 1,
        route_id: route_id.to_string(),
        packet_id: packet_id.to_string(),
        hop_index,
        expires_at,
        nonce,
        ciphertext,
    })
}

pub(crate) fn peel_reverse(
    packet: &OnionReversePacket,
    expected_route_id: &str,
    expected_hop_index: u8,
    hop_session_key: &[u8; 32],
    now: u64,
) -> Result<ReverseForward, &'static str> {
    validate_reverse_shape(packet)?;
    if packet.route_id != expected_route_id {
        return Err("unexpected reverse onion route");
    }
    if packet.hop_index != expected_hop_index {
        return Err("unexpected reverse onion hop index");
    }
    if packet.expires_at < now
        || packet.expires_at.saturating_sub(now) > ONION_TTL_SECS
    {
        return Err("invalid reverse onion expiry");
    }

    let key = crypto::onion_layer_key(
        hop_session_key,
        &packet.route_id,
        &packet.packet_id,
        packet.hop_index,
        "reverse",
    )?;
    let aad = layer_aad(
        &packet.route_id,
        &packet.packet_id,
        packet.hop_index,
        packet.expires_at,
        "reverse",
    );
    let bytes = crypto::decrypt(&key, &aad, &packet.nonce, &packet.ciphertext)?;
    let frame: ReverseFrame =
        serde_json::from_slice(&bytes).map_err(|_| "invalid reverse onion frame")?;
    validate_reverse_frame(&frame)?;

    Ok(ReverseForward {
        previous_node_id: frame.previous_node_id,
        previous_address: frame.previous_address,
        payload: frame.payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;

    fn hop(name: &str, address: &str) -> OnionHop {
        OnionHop {
            node_id: name.to_string(),
            address: address.to_string(),
        }
    }

    fn free_port() -> u16 {
        UdpSocket::bind(("127.0.0.1", 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    #[test]
    fn three_hop_udp_relay_forwards_only_through_bound_routes() {
        let route = new_route_id();
        let packet_id = new_packet_id();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expires = now + 60;
        let keys = [[11u8; 32], [12u8; 32], [13u8; 32]];
        let relay_ports = [free_port(), free_port(), free_port()];
        let destination_port = free_port();
        let hops = vec![
            hop("relay-a", &format!("127.0.0.1:{}", relay_ports[0])),
            hop("relay-b", &format!("127.0.0.1:{}", relay_ports[1])),
            hop("relay-c", &format!("127.0.0.1:{}", relay_ports[2])),
        ];

        let tables = [
            OnionRelayTable::new(),
            OnionRelayTable::new(),
            OnionRelayTable::new(),
        ];
        for index in 0..3 {
            tables[index]
                .bind_route(&route, index as u8, keys[index])
                .unwrap();
        }

        let stops = [
            Arc::new(AtomicBool::new(true)),
            Arc::new(AtomicBool::new(true)),
            Arc::new(AtomicBool::new(true)),
        ];
        let mut handles = Vec::new();
        for index in 0..3 {
            handles.push(
                spawn_udp_relay(
                    format!("127.0.0.1:{}", relay_ports[index])
                        .parse()
                        .unwrap(),
                    tables[index].clone(),
                    stops[index].clone(),
                )
                .unwrap(),
            );
        }

        let destination = UdpSocket::bind(("127.0.0.1", destination_port)).unwrap();
        destination
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let packet = wrap(
            &route,
            &packet_id,
            expires,
            b"udp onion secret",
            &hops,
            &vec!["s-a".into(), "s-b".into(), "s-c".into()],
            &keys,
            "cyb-destination",
            &format!("127.0.0.1:{}", destination_port),
        )
        .unwrap();

        let source = UdpSocket::bind(("127.0.0.1", 0)).unwrap();
        let body = serde_json::to_string(&packet).unwrap();
        source
            .send_to(
                format!("{} {}", ONION_PREFIX, body).as_bytes(),
                format!("127.0.0.1:{}", relay_ports[0]),
            )
            .unwrap();

        let mut buffer = [0u8; 8192];
        let (size, sender) = destination.recv_from(&mut buffer).unwrap();
        let text = std::str::from_utf8(&buffer[..size]).unwrap();
        let payload = text.strip_prefix(ONION_DELIVERY_PREFIX)
            .and_then(|rest| rest.strip_prefix(' '))
            .unwrap();
        let delivery: OnionDelivery = serde_json::from_str(payload).unwrap();

        assert_eq!(sender.ip(), std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
        assert_eq!(delivery.route_id, route);
        assert_eq!(delivery.packet_id, packet_id);
        assert_eq!(delivery.payload, b"udp onion secret");

        for stop in stops {
            stop.store(false, Ordering::Release);
        }
        for handle in handles {
            let _ = handle.join();
        }
    }

    #[test]
    fn three_hop_onion_keeps_payload_and_future_hops_hidden() {
        let route = new_route_id();
        let packet_id = new_packet_id();
        let expires = 1_000_000_100;
        let hops = vec![
            hop("relay-a", "127.0.0.1:40001"),
            hop("relay-b", "127.0.0.1:40002"),
            hop("relay-c", "127.0.0.1:40003"),
        ];
        let keys = [[1u8; 32], [2u8; 32], [3u8; 32]];
        let payload = b"end-to-end secret";

        let outer = wrap(
            &route,
            &packet_id,
            expires,
            payload,
            &hops,
            &vec!["session-a".into(), "session-b".into(), "session-c".into()],
            &keys,
            "cyb-destination",
            "127.0.0.1:40004",
        )
        .unwrap();

        let encoded = serde_json::to_string(&outer).unwrap();
        assert!(!encoded.contains("end-to-end"));
        assert!(!encoded.contains("relay-b"));
        assert!(!encoded.contains("relay-c"));
        assert!(!encoded.contains("cyb-destination"));

        let mut relay_a = OnionRelayCache::new();
        let first = peel(
            &mut relay_a,
            &outer,
            &route,
            0,
            &keys[0],
            1_000_000_000,
        )
        .unwrap();
        let PeelResult::Forward(first) = first else {
            panic!("expected relay forwarding")
        };
        assert_eq!(first.next_node_id, "relay-b");
        assert_eq!(first.next_address, "127.0.0.1:40002");
        assert_eq!(first.packet.hop_index, 1);

        let mut relay_b = OnionRelayCache::new();
        let second = peel(
            &mut relay_b,
            &first.packet,
            &route,
            1,
            &keys[1],
            1_000_000_000,
        )
        .unwrap();
        let PeelResult::Forward(second) = second else {
            panic!("expected second relay forwarding")
        };
        assert_eq!(second.next_node_id, "relay-c");
        assert_eq!(second.next_address, "127.0.0.1:40003");
        assert_eq!(second.packet.hop_index, 2);

        let mut relay_c = OnionRelayCache::new();
        let third = peel(
            &mut relay_c,
            &second.packet,
            &route,
            2,
            &keys[2],
            1_000_000_000,
        )
        .unwrap();
        let PeelResult::Deliver(deliver) = third else {
            panic!("expected destination delivery")
        };
        assert_eq!(deliver.destination_id, "cyb-destination");
        assert_eq!(deliver.destination_address, "127.0.0.1:40004");
        assert_eq!(deliver.payload, payload);
    }

    #[test]
    fn reverse_onion_ack_can_cross_back_through_three_relays() {
        let route = new_route_id();
        let packet_id = new_packet_id();
        let expires = 1_000_000_100;
        let keys = [[21u8; 32], [22u8; 32], [23u8; 32]];
        let payload = b"signed destination ack";

        let packet_c = wrap_reverse_hop(
            &route,
            &packet_id,
            2,
            expires,
            payload,
            "relay-b",
            "127.0.0.1:41002",
            &keys[2],
        )
        .unwrap();
        let from_c = peel_reverse(
            &packet_c,
            &route,
            2,
            &keys[2],
            1_000_000_000,
        )
        .unwrap();
        assert_eq!(from_c.previous_node_id, "relay-b");
        assert_eq!(from_c.payload, payload);

        let packet_b = wrap_reverse_hop(
            &route,
            &packet_id,
            1,
            expires,
            &from_c.payload,
            "relay-a",
            "127.0.0.1:41001",
            &keys[1],
        )
        .unwrap();
        let from_b = peel_reverse(
            &packet_b,
            &route,
            1,
            &keys[1],
            1_000_000_000,
        )
        .unwrap();

        let packet_a = wrap_reverse_hop(
            &route,
            &packet_id,
            0,
            expires,
            &from_b.payload,
            "source",
            "127.0.0.1:41000",
            &keys[0],
        )
        .unwrap();
        let source = peel_reverse(
            &packet_a,
            &route,
            0,
            &keys[0],
            1_000_000_000,
        )
        .unwrap();

        assert_eq!(source.previous_node_id, "source");
        assert_eq!(source.payload, payload);
    }

    #[test]
    fn onion_replay_is_rejected_per_hop() {
        let route = new_route_id();
        let packet_id = new_packet_id();
        let hops = vec![hop("relay", "127.0.0.1:40101")];
        let keys = [[9u8; 32]];
        let packet = wrap(
            &route,
            &packet_id,
            1_000_000_100,
            b"payload",
            &hops,
            &vec!["session-a".into()],
            &keys,
            "destination",
            "127.0.0.1:40102",
        )
        .unwrap();

        let mut cache = OnionRelayCache::new();
        assert!(peel(
            &mut cache,
            &packet,
            &route,
            0,
            &keys[0],
            1_000_000_000,
        )
        .is_ok());
        assert_eq!(
            peel(
                &mut cache,
                &packet,
                &route,
                0,
                &keys[0],
                1_000_000_000,
            ),
            Err("onion packet replayed")
        );
    }

    #[test]
    fn onion_tampering_and_wrong_hop_key_are_rejected() {
        let route = new_route_id();
        let hops = vec![
            hop("relay-a", "127.0.0.1:40201"),
            hop("relay-b", "127.0.0.1:40202"),
        ];
        let keys = [[7u8; 32], [8u8; 32]];
        let mut packet = wrap(
            &route,
            &new_packet_id(),
            1_000_000_100,
            b"payload",
            &hops,
            &vec!["session-a".into(), "session-b".into()],
            &keys,
            "destination",
            "127.0.0.1:40203",
        )
        .unwrap();

        let mut cache = OnionRelayCache::new();
        assert_eq!(
            peel(
                &mut cache,
                &packet,
                &route,
                0,
                &keys[1],
                1_000_000_000,
            ),
            Err("authentication failed")
        );

        let mut ciphertext = base64::engine::general_purpose::STANDARD
            .decode(&packet.ciphertext)
            .unwrap();
        ciphertext[0] ^= 1;
        packet.ciphertext =
            base64::engine::general_purpose::STANDARD.encode(ciphertext);

        assert_eq!(
            peel(
                &mut OnionRelayCache::new(),
                &packet,
                &route,
                0,
                &keys[0],
                1_000_000_000,
            ),
            Err("authentication failed")
        );
    }

    #[test]
    #[test]
    fn tampered_packet_does_not_poison_replay_cache() {
        let route = new_route_id();
        let packet_id = new_packet_id();
        let hops = vec![hop("relay-a", "127.0.0.1:40401")];
        let keys = [[17u8; 32]];
        let mut packet = wrap(
            &route,
            &packet_id,
            1_000_000_100,
            b"payload",
            &hops,
            &vec!["session-a".into()],
            &keys,
            "destination",
            "127.0.0.1:40402",
        )
        .unwrap();

        let mut cache = OnionRelayCache::new();
        let mut ciphertext = base64::engine::general_purpose::STANDARD
            .decode(&packet.ciphertext)
            .unwrap();
        ciphertext[0] ^= 1;
        packet.ciphertext =
            base64::engine::general_purpose::STANDARD.encode(ciphertext);

        assert_eq!(
            peel(
                &mut cache,
                &packet,
                &route,
                0,
                &keys[0],
                1_000_000_000,
            ),
            Err("authentication failed")
        );

        let original = wrap(
            &route,
            &packet_id,
            1_000_000_100,
            b"payload",
            &hops,
            &vec!["session-a".into()],
            &keys,
            "destination",
            "127.0.0.1:40402",
        )
        .unwrap();

        assert!(peel(
            &mut cache,
            &original,
            &route,
            0,
            &keys[0],
            1_000_000_000,
        )
        .is_ok());
    }

    fn malformed_onion_shapes_are_rejected_before_replay_state_changes() {
        let route = new_route_id();
        let packet = OnionPacket {
            version: 1,
            route_id: route.clone(),
            packet_id: new_packet_id(),
            session_id: "session".into(),
            hop_index: 0,
            expires_at: 1_000_000_100,
            nonce: "%%%".into(),
            ciphertext: "%%%".into(),
        };

        let mut cache = OnionRelayCache::new();
        assert_eq!(
            peel(&mut cache, &packet, &route, 0, &[1u8; 32], 1_000_000_000),
            Err("invalid onion packet fields")
        );

        let replay_key = format!(
            "{}:{}:{}:{}",
            packet.route_id, packet.packet_id, packet.session_id, packet.hop_index
        );
        assert!(!cache.seen.contains_key(&replay_key));
    }

    #[test]
    fn malformed_reverse_frame_is_rejected_after_decryption() {
        let route = new_route_id();
        let packet_id = new_packet_id();
        let key = [41u8; 32];
        let expires_at = 1_000_000_100;

        let bytes = serde_json::to_vec(&serde_json::json!({
            "previous_node_id": "",
            "previous_address": "127.0.0.1:1",
            "payload": [1, 2, 3]
        }))
        .unwrap();
        let layer_key = crypto::onion_layer_key(&key, &route, &packet_id, 0, "reverse").unwrap();
        let aad = layer_aad(&route, &packet_id, 0, expires_at, "reverse");
        let (nonce, ciphertext) = crypto::encrypt(&layer_key, &aad, &bytes).unwrap();

        let packet = OnionReversePacket {
            version: 1,
            route_id: route.clone(),
            packet_id,
            hop_index: 0,
            expires_at,
            nonce,
            ciphertext,
        };

        assert_eq!(
            peel_reverse(
                &packet,
                &route,
                0,
                &key,
                1_000_000_000,
            ),
            Err("invalid reverse onion frame")
        );
    }

    #[test]
    fn onion_expiry_and_hop_binding_are_rejected() {
        let route = new_route_id();
        let hops = vec![hop("relay", "127.0.0.1:40301")];
        let keys = [[7u8; 32]];
        let packet = wrap(
            &route,
            &new_packet_id(),
            1_000,
            b"payload",
            &hops,
            &vec!["session-a".into()],
            &keys,
            "destination",
            "127.0.0.1:40302",
        )
        .unwrap();

        assert_eq!(
            peel(
                &mut OnionRelayCache::new(),
                &packet,
                &route,
                1,
                &keys[0],
                900,
            ),
            Err("unexpected onion hop index")
        );
        assert_eq!(
            peel(
                &mut OnionRelayCache::new(),
                &packet,
                &route,
                0,
                &keys[0],
                1_001,
            ),
            Err("onion packet expired")
        );
    }
}
