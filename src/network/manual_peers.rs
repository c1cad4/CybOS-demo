//! Saved addresses are routing hints, never proof of peer identity.
use crate::store::Store;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ManualPeer {
    pub node_id: String,
    pub address: String,
}
const KEY: &str = "chat_manual_peers";
const LIMIT: usize = 32;

fn validate(node: &str, address: &str) -> Result<ManualPeer, String> {
    let node = node.trim();
    if node.is_empty() || node.len() > 128 || node.chars().any(char::is_control) {
        return Err("Enter a node ID (up to 128 characters).".into());
    }
    let ip: Ipv4Addr = address.trim().parse().map_err(|_| "Enter an IPv4 address, for example 192.168.1.20.")?;
    if ip.is_unspecified() || ip.is_multicast() || ip.is_broadcast() {
        return Err("Use an individual device address.".into());
    }
    Ok(ManualPeer { node_id: node.into(), address: ip.to_string() })
}
pub(crate) fn load(store: &Store) -> Result<Vec<ManualPeer>, String> {
    let Some(raw) = store.get_checked(KEY)? else { return Ok(Vec::new()); };
    let peers: Vec<ManualPeer> = serde_json::from_str(&raw).map_err(|_| "Saved devices could not be read.")?;
    if peers.len() > LIMIT { return Err("Saved device limit exceeded.".into()); }
    let mut checked = Vec::new();
    for peer in peers {
        let peer = validate(&peer.node_id, &peer.address)?;
        if checked.iter().any(|p: &ManualPeer| p.node_id == peer.node_id) { return Err("Duplicate saved node ID.".into()); }
        checked.push(peer);
    }
    Ok(checked)
}
pub(crate) fn save(store: &Store, peers: &[ManualPeer], local: &str, node: &str, address: &str) -> Result<Vec<ManualPeer>, String> {
    let peer = validate(node, address)?;
    if peer.node_id == local { return Err("Choose another device's node ID.".into()); }
    let mut updated = peers.to_vec();
    if let Some(existing) = updated.iter_mut().find(|p| p.node_id == peer.node_id) { *existing = peer; }
    else {
        if updated.len() >= LIMIT { return Err("At most 32 devices can be saved.".into()); }
        updated.push(peer);
    }
    persist(store, &updated)?;
    Ok(updated)
}
pub(crate) fn remove(store: &Store, peers: &[ManualPeer], node: &str) -> Result<Vec<ManualPeer>, String> {
    let updated: Vec<_> = peers.iter().filter(|p| p.node_id != node).cloned().collect();
    persist(store, &updated)?;
    Ok(updated)
}
fn persist(store: &Store, peers: &[ManualPeer]) -> Result<(), String> {
    store.set_checked(KEY, &serde_json::to_string(peers).map_err(|e| e.to_string())?)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn addresses_are_validated_without_granting_trust() {
        let store = Store::test_store(":memory:");
        for address in ["hostname", "0.0.0.0", "255.255.255.255", "224.0.0.1", "127.0.0.1:39394"] {
            assert!(save(&store, &[], "local", "peer", address).is_err());
        }
        assert!(save(&store, &[], "local", "local", "127.0.0.1").is_err());
        let peers = save(&store, &[], "local", "peer", "127.0.0.1").unwrap();
        assert_eq!(load(&store).unwrap()[0].address, "127.0.0.1");
        assert!(crate::network::peer_trust::approved_key(&store, "peer").unwrap().is_none());
        store.approve_secure_peer("peer", &[7; 32]).unwrap();
        let peers = save(&store, &peers, "local", "peer", "192.168.1.20").unwrap();
        assert_eq!(peers.len(), 1);
        assert!(remove(&store, &peers, "peer").unwrap().is_empty());
        assert_eq!(crate::network::peer_trust::approved_key(&store, "peer").unwrap(), Some(vec![7; 32]));
    }
    #[test]
    fn failed_write_preserves_saved_devices() {
        let store = Store::test_store(":memory:");
        let peers = save(&store, &[], "local", "peer", "127.0.0.1").unwrap();
        store.test_execute("CREATE TRIGGER deny_update BEFORE UPDATE ON kv BEGIN SELECT RAISE(ABORT, 'denied'); END;");
        assert!(save(&store, &peers, "local", "peer", "192.168.1.20").is_err());
        assert!(remove(&store, &peers, "peer").is_err());
        assert_eq!(load(&store).unwrap()[0].address, "127.0.0.1");
    }
    #[test]
    fn device_limit_allows_updates_but_rejects_new_entries() {
        let store = Store::test_store(":memory:");
        let mut peers = Vec::new();
        for i in 0..LIMIT { peers = save(&store, &peers, "local", &format!("peer-{i}"), "127.0.0.1").unwrap(); }
        assert!(save(&store, &peers, "local", "extra", "127.0.0.1").is_err());
        assert_eq!(save(&store, &peers, "local", "peer-0", "192.168.1.20").unwrap().len(), LIMIT);
        assert_eq!(load(&store).unwrap().len(), LIMIT);
    }
    #[test]
    fn corrupt_saved_devices_are_reported() {
        let store = Store::test_store(":memory:");
        store.set_checked(KEY, "broken").unwrap();
        assert!(load(&store).is_err());
    }
}
