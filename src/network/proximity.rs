//! Unified LAN + BLE proximity view.
//!
//! This is deliberately a projection layer: discovery sources remain
//! independent, while CYB RADAR gets one truthful peer model.

use std::time::Instant;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ProximitySource {
    Lan,
    Ble,
    LanAndBle,
}

#[derive(Clone, Debug)]
pub(crate) struct NearbyPeer {
    pub(crate) node_id: String,
    pub(crate) lan_address: Option<String>,
    pub(crate) rssi: Option<i16>,
    pub(crate) source: ProximitySource,
    pub(crate) last_seen: Instant,
}

pub(crate) fn merge(
    lan: &[crate::network::lan::LanPeer],
    ble: &[crate::network::ble::BlePeer],
) -> Vec<NearbyPeer> {
    let mut peers = Vec::new();

    for p in lan {
        peers.push(NearbyPeer {
            node_id: p.node_id.clone(),
            lan_address: Some(p.address.clone()),
            rssi: None,
            source: ProximitySource::Lan,
            last_seen: p.last_seen,
        });
    }

    for p in ble {
        let suffix = p.name.strip_prefix("cybOS-").unwrap_or("").to_ascii_lowercase();
        if let Some(existing) = peers.iter_mut().find(|x| {
            let id_suffix = x
                .node_id
                .replace('-', "")
                .chars()
                .take(8)
                .collect::<String>()
                .to_ascii_lowercase();
            id_suffix == suffix
        }) {
            existing.rssi = Some(p.rssi);
            existing.source = ProximitySource::LanAndBle;
            existing.last_seen = p.last_seen;
        } else {
            peers.push(NearbyPeer {
                node_id: p.name.clone(),
                lan_address: None,
                rssi: Some(p.rssi),
                source: ProximitySource::Ble,
                last_seen: p.last_seen,
            });
        }
    }

    peers
}

#[cfg(test)]
mod tests {
    use super::{merge, ProximitySource};

    #[test]
    fn lan_and_ble_are_correlated_by_node_suffix() {
        let now = std::time::Instant::now();
        let lan = vec![crate::network::lan::LanPeer {
            node_id: "cyb-ab12cd34".into(),
            address: "192.168.1.20".into(),
            version: "0.7.0".into(),
            distance_m: None,
            last_seen: now,
        }];
        let ble = vec![crate::network::ble::BlePeer {
            device_id: "radio".into(),
            name: "cybOS-AB12CD34".into(),
            rssi: -51,
            last_seen: now,
        }];

        let merged = merge(&lan, &ble);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].source, ProximitySource::LanAndBle);
        assert_eq!(merged[0].rssi, Some(-51));
    }
}
