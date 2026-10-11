//! Explicit user approval binds a node id to one Noise public key.
use super::secure_chat::{decode_key, fingerprint};
use crate::store::Store;

#[derive(Clone, Debug)]
pub(crate) struct PendingPeer {
    pub(crate) node_id: String,
    pub(crate) public_key: Vec<u8>,
    pub(crate) fingerprint: String,
}

pub(crate) fn validate_peer(node_id: &str, key: &[u8]) -> Result<(), String> {
    if node_id.is_empty()
        || node_id.len() > 128
        || node_id.chars().any(char::is_control)
        || key.len() != 32
    {
        return Err("invalid peer identity".into());
    }
    Ok(())
}

pub(crate) fn approved_key(store: &Store, node_id: &str) -> Result<Option<Vec<u8>>, String> {
    let pin = store
        .get_checked(&format!("noise_peer_key:{node_id}"))?
        .map(|s| decode_key(&s))
        .transpose()?;
    let approval = store
        .get_checked(&format!("noise_peer_approved:{node_id}"))?
        .map(|s| decode_key(&s))
        .transpose()?;
    match (pin, approval) {
        (Some(pin), Some(approval)) if pin == approval => Ok(Some(pin)),
        (_, Some(_)) => Err("peer approval does not match persisted identity".into()),
        (_, None) => Ok(None), // Legacy automatic TOFU pins need explicit approval.
    }
}

pub(crate) fn check_peer(store: &Store, node_id: &str, key: &[u8]) -> Result<bool, String> {
    validate_peer(node_id, key)?;
    if let Some(encoded) = store.get_checked(&format!("noise_peer_key:{node_id}"))? {
        if decode_key(&encoded)? != key {
            return Err("Noise identity key changed".into());
        }
    }
    Ok(approved_key(store, node_id)?.as_deref() == Some(key))
}

pub(crate) fn queue_peer(
    queue: &mut Vec<PendingPeer>,
    node_id: String,
    key: Vec<u8>,
) -> Result<(), String> {
    validate_peer(&node_id, &key)?;
    if let Some(existing) = queue.iter().find(|p| p.node_id == node_id) {
        return if existing.public_key == key {
            Ok(())
        } else {
            Err("conflicting pending identity; reject before retrying".into())
        };
    }
    if queue.len() >= 16 {
        return Err("peer approval queue is full".into());
    }
    queue.push(PendingPeer {
        fingerprint: fingerprint(&key),
        node_id,
        public_key: key,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_and_legacy_tofu_peers_require_explicit_approval() {
        let store = Store::test_store(":memory:");
        let key = [7; 32];
        assert!(!check_peer(&store, "node-a", &key).unwrap());
        store
            .set_checked("noise_peer_key:node-a", &"07".repeat(32))
            .unwrap();
        assert!(!check_peer(&store, "node-a", &key).unwrap());
        store.approve_secure_peer("node-a", &key).unwrap();
        assert!(check_peer(&store, "node-a", &key).unwrap());
        assert!(check_peer(&store, "node-a", &[8; 32]).is_err());
        assert!(store.approve_secure_peer("node-a", &[8; 32]).is_err());
        assert_eq!(approved_key(&store, "node-a").unwrap(), Some(key.to_vec()));
    }

    #[test]
    fn approval_write_failure_does_not_publish_a_pin() {
        let store = Store::test_store(":memory:");
        store.test_execute("CREATE TRIGGER deny_approval BEFORE INSERT ON kv WHEN NEW.key LIKE 'noise_peer_approved:%' BEGIN SELECT RAISE(ABORT, 'read-only fixture'); END;");
        assert!(store.approve_secure_peer("node-a", &[7; 32]).is_err());
        assert_eq!(store.get_checked("noise_peer_key:node-a").unwrap(), None);
        assert_eq!(approved_key(&store, "node-a").unwrap(), None);
    }

    #[test]
    fn malformed_pin_or_mismatched_approval_fail_closed() {
        let store = Store::test_store(":memory:");
        store
            .set_checked("noise_peer_key:node-a", &"€".repeat(22))
            .unwrap();
        assert!(check_peer(&store, "node-a", &[7; 32]).is_err());
        store
            .set_checked("noise_peer_key:node-a", &"07".repeat(32))
            .unwrap();
        store
            .set_checked("noise_peer_approved:node-a", &"08".repeat(32))
            .unwrap();
        assert!(approved_key(&store, "node-a").is_err());
        store.test_execute("DROP TABLE kv;");
        assert!(approved_key(&store, "node-a").is_err());
    }

    #[test]
    fn approval_and_history_survive_database_reopen() {
        let path = std::env::temp_dir().join(format!("cybos-trust-{}.db", uuid::Uuid::new_v4()));
        let file = path.to_str().unwrap();
        {
            let store = Store::test_store(file);
            store.approve_secure_peer("node-a", &[7; 32]).unwrap();
            store
                .accept_secure_message("message-1", "node-a", "durable body")
                .unwrap();
        }
        {
            let store = Store::test_store(file);
            assert!(check_peer(&store, "node-a", &[7; 32]).unwrap());
            assert!(
                !store
                    .accept_secure_message("message-1", "node-a", "durable body")
                    .unwrap()
            );
            assert_eq!(store.chat_messages()[0].1, "durable body");
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn pending_approvals_are_bounded_deduplicated_and_cannot_be_replaced() {
        let mut queue = Vec::new();
        for i in 0..16 {
            queue_peer(&mut queue, format!("node-{i}"), vec![7; 32]).unwrap();
        }
        queue_peer(&mut queue, "node-0".into(), vec![7; 32]).unwrap();
        assert_eq!(queue.len(), 16);
        assert!(queue_peer(&mut queue, "node-extra".into(), vec![7; 32]).is_err());
        assert!(queue_peer(&mut queue, "node-0".into(), vec![8; 32]).is_err());
        assert_eq!(queue[0].public_key, vec![7; 32]);
        assert!(queue_peer(&mut Vec::new(), "node\nspoof".into(), vec![7; 32]).is_err());
    }
}
