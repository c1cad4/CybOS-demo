use ed25519_dalek::{Signer, SigningKey};
use serde::Serialize;
use crate::store::Store;

/// The seed is persisted in the existing local SQLite store. This is not an OS keychain;
/// protect the user profile and database backups accordingly.
pub(crate) fn load_or_create_key(store: &Store) -> Result<SigningKey, String> {
    if let Some(encoded) = store.get("cybcore_ed25519_seed_hex") {
        let bytes: [u8; 32] = hex::decode(encoded)
            .map_err(|e| e.to_string())?
            .try_into().map_err(|_| "invalid persisted key length")?;
        return Ok(SigningKey::from_bytes(&bytes));
    }
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).map_err(|e| e.to_string())?;
    let key = SigningKey::from_bytes(&seed);
    store.set("cybcore_ed25519_seed_hex", &hex::encode(seed));
    Ok(key)
}


#[derive(Serialize)]
pub struct SignedCoreEvent {
    pub version: u16,
    pub event_id: String,
    pub author_key: String,
    pub kind: String,
    pub created_at_ms: i64,
    pub content: String,
    pub signature: String,
}

impl SignedCoreEvent {
    fn signing_bytes(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&(
            "cybcore-event-v1", self.version, &self.event_id, &self.author_key,
            &self.kind, self.created_at_ms, &self.content,
        )).map_err(|e| e.to_string())
    }

    pub fn sign(key: &SigningKey, kind: &str, content: &str) -> Result<Self, String> {
        if kind.is_empty() || kind.len() > 128
            || !kind.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            || content.len() > 16 * 1024 {
            return Err("invalid event kind or size".into());
        }
        let mut event = Self {
            version: 1,
            event_id: uuid::Uuid::new_v4().to_string(),
            author_key: hex::encode(key.verifying_key().to_bytes()),
            kind: kind.into(),
            created_at_ms: chrono::Utc::now().timestamp_millis(),
            content: content.into(),
            signature: String::new(),
        };
        event.signature = hex::encode(key.sign(&event.signing_bytes()?).to_bytes());
        Ok(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signature, Verifier};
    #[test]
    fn signature_roundtrip() {
        let key = SigningKey::from_bytes(&[3u8; 32]);
        let event = SignedCoreEvent::sign(&key, "robot.heartbeat", "{}").unwrap();
        let bytes: [u8; 64] = hex::decode(&event.signature).unwrap().try_into().unwrap();
        assert!(key.verifying_key().verify(
            &event.signing_bytes().unwrap(), &Signature::from_bytes(&bytes)
        ).is_ok());
    }
}
