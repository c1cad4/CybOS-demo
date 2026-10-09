//! Verified, bounded Ed25519 event envelopes. No private keys are stored server-side.
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

pub const MAX_CONTENT_BYTES: usize = 16 * 1024;
pub const MAX_CLOCK_SKEW_MS: i64 = 5 * 60 * 1000;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignedEvent {
    pub version: u16,
    pub event_id: String,
    pub author_key: String,
    pub kind: String,
    pub created_at_ms: i64,
    pub content: String,
    pub signature: String,
}

impl SignedEvent {
    /// Canonical bytes signed by the node: a fixed domain separator followed by
    /// serde_json's deterministic serialization of a positional tuple.
    pub fn signing_bytes(&self) -> Result<Vec<u8>, &'static str> {
        serde_json::to_vec(&(
            "cybcore-event-v1",
            self.version,
            &self.event_id,
            &self.author_key,
            &self.kind,
            self.created_at_ms,
            &self.content,
        )).map_err(|_| "event encoding failed")
    }

    pub fn verify(&self, now_ms: i64) -> Result<(), &'static str> {
        if self.version != 1 { return Err("unsupported event version"); }
        if self.event_id.is_empty() || self.event_id.len() > 128
            || self.kind.is_empty() || self.kind.len() > 128
            || !self.kind.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        { return Err("invalid event identifier or kind"); }
        if self.content.len() > MAX_CONTENT_BYTES { return Err("event too large"); }
        if (i128::from(self.created_at_ms) - i128::from(now_ms)).abs()
            > i128::from(MAX_CLOCK_SKEW_MS) { return Err("event timestamp out of range"); }
        let pubkey: [u8; 32] = hex::decode(&self.author_key)
            .map_err(|_| "invalid public key")?
            .try_into().map_err(|_| "invalid public key length")?;
        let signature: [u8; 64] = hex::decode(&self.signature)
            .map_err(|_| "invalid signature")?
            .try_into().map_err(|_| "invalid signature length")?;
        let key = VerifyingKey::from_bytes(&pubkey).map_err(|_| "invalid public key")?;
        key.verify(&self.signing_bytes()?, &Signature::from_bytes(&signature))
            .map_err(|_| "signature verification failed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    #[test]
    fn accepts_valid_signature_and_rejects_tampering() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let mut event = SignedEvent {
            version: 1, event_id: "event-1".into(),
            author_key: hex::encode(key.verifying_key().to_bytes()),
            kind: "robot.heartbeat".into(), created_at_ms: 1000,
            content: "{}".into(), signature: String::new(),
        };
        event.signature = hex::encode(key.sign(&event.signing_bytes().unwrap()).to_bytes());
        assert!(event.verify(1000).is_ok());
        event.content = "{\"tampered\":true}".into();
        assert!(event.verify(1000).is_err());
    }
    #[test]
    fn rejects_expired_oversized_and_malformed_events() {
        let event = SignedEvent {
            version: 1, event_id: "x".into(), author_key: "invalid".into(),
            kind: "heartbeat".into(), created_at_ms: 0,
            content: "x".repeat(MAX_CONTENT_BYTES + 1), signature: String::new(),
        };
        assert!(event.verify(0).is_err());
        let mut event = event;
        event.content.clear();
        assert!(event.verify(MAX_CLOCK_SKEW_MS + 1).is_err());
    }
}
