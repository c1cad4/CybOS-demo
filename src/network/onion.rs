//! Multi-hop onion packet construction and relay peeling for CybChat.
//!
//! Long-term node identity remains Ed25519. Each adjacent hop uses its own
//! authenticated X25519 session root; this module derives a separate AEAD
//! layer key from that root for the route packet. A relay learns only its
//! immediate next hop and never the end-to-end payload.

use crate::crypto;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

pub(crate) const MAX_ONION_HOPS: usize = 4;
pub(crate) const MAX_ONION_BYTES: usize = 4096;
pub(crate) const ONION_TTL_SECS: u64 = 120;
const REPLAY_CACHE_LIMIT: usize = 512;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OnionHop {
    pub(crate) node_id: String,
    pub(crate) address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct OnionPacket {
    pub(crate) version: u8,
    pub(crate) route_id: String,
    pub(crate) packet_id: String,
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

    fn check_and_mark(
        &mut self,
        cache_key: String,
        expires_at: u64,
        now: u64,
    ) -> Result<(), &'static str> {
        self.purge(now);
        if expires_at < now {
            return Err("onion packet expired");
        }
        if self.seen.contains_key(&cache_key) {
            return Err("onion packet replayed");
        }
        if self.seen.len() >= REPLAY_CACHE_LIMIT {
            if let Some(oldest) = self.order.pop_front() {
                self.seen.remove(&oldest);
            }
        }
        self.seen.insert(cache_key.clone(), expires_at);
        self.order.push_back(cache_key);
        Ok(())
    }
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
) -> Vec<u8> {
    [
        crypto::PROTOCOL,
        "onion-layer-v1",
        route_id,
        packet_id,
        &hop_index.to_string(),
        &expires_at.to_string(),
    ]
    .join("|")
    .into_bytes()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
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
    hop_session_keys: &[[u8; 32]],
    destination_id: &str,
    destination_address: &str,
) -> Result<OnionPacket, &'static str> {
    if hops.is_empty() || hops.len() > MAX_ONION_HOPS {
        return Err("invalid onion hop count");
    }
    if hops.len() != hop_session_keys.len() {
        return Err("onion hop/session key count mismatch");
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
        let aad = layer_aad(route_id, packet_id, hop_index, expires_at);
        let (nonce, ciphertext) = crypto::encrypt(&key, &aad, &frame_bytes)?;

        let packet = OnionPacket {
            version: 1,
            route_id: route_id.to_string(),
            packet_id: packet_id.to_string(),
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
    if packet.version != 1 {
        return Err("unsupported onion version");
    }
    if packet.route_id != expected_route_id {
        return Err("unexpected onion route");
    }
    if packet.hop_index != expected_hop_index {
        return Err("unexpected onion hop index");
    }
    if packet.expires_at < now
        || packet.expires_at.saturating_sub(now) > ONION_TTL_SECS
    {
        return Err("invalid onion expiry");
    }

    let cache_key = format!(
        "{}:{}:{}",
        packet.route_id, packet.packet_id, packet.hop_index
    );
    relay.check_and_mark(cache_key, packet.expires_at, now)?;

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
    );
    let frame_bytes = crypto::decrypt(&key, &aad, &packet.nonce, &packet.ciphertext)?;

    match serde_json::from_slice::<LayerFrame>(&frame_bytes)
        .map_err(|_| "invalid onion frame")?
    {
        LayerFrame::Forward {
            next_node_id,
            next_address,
            packet: inner,
        } => {
            if inner.version != 1
                || inner.route_id != packet.route_id
                || inner.packet_id != packet.packet_id
                || inner.hop_index != packet.hop_index.saturating_add(1)
                || inner.expires_at != packet.expires_at
            {
                return Err("invalid nested onion packet");
            }
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
        } => Ok(PeelResult::Deliver(DeliverPacket {
            destination_id,
            destination_address,
            payload,
        })),
    }
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
