//! E2E message primitives for CybChat.
//!
//! The node identity authenticates the sender. Each message uses a fresh
//! X25519 ephemeral key and ChaCha20-Poly1305 AEAD. This module deliberately
//! keeps session secrets in memory only.
//!
//! Protocol status: experimental v1. Forward secrecy for a complete,
//! stateful conversation and key rotation are future work.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use ring::{aead, agreement, digest, hkdf, rand, signature};

use crate::identity::{signing_key, NodeIdentity};

const PROTOCOL: &str = "cybchat-e2e-v1";

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct EncryptedMessage {
    pub(crate) version: u8,
    pub(crate) sender_id: String,
    pub(crate) sender_public_key: String,
    pub(crate) ephemeral_public_key: String,
    pub(crate) nonce: String,
    pub(crate) ciphertext: String,
    pub(crate) signature: String,
}

pub(crate) fn encrypt_for_peer(
    identity: &NodeIdentity,
    peer_id: &str,
    peer_x25519_public: &[u8],
    plaintext: &[u8],
) -> Result<EncryptedMessage, &'static str> {
    if peer_x25519_public.len() != 32 {
        return Err("invalid peer X25519 public key");
    }

    let rng = rand::SystemRandom::new();
    let ephemeral = agreement::EphemeralPrivateKey::generate(&agreement::X25519, &rng)
        .map_err(|_| "cannot create ephemeral key")?;
    let ephemeral_public = ephemeral
        .compute_public_key()
        .map_err(|_| "cannot compute ephemeral public key")?;

    let peer = agreement::UnparsedPublicKey::new(&agreement::X25519, peer_x25519_public);
    let key_bytes = agreement::agree_ephemeral(ephemeral, &peer, |secret| secret.to_vec())
        .map_err(|_| "X25519 agreement failed")?;

    let key = derive_aead_key(&key_bytes, peer_id.as_bytes())?;

    let mut nonce_bytes = [0_u8; aead::NONCE_LEN];
    rand::SecureRandom::fill(&rng, &mut nonce_bytes).map_err(|_| "cannot create nonce")?;

    let unbound = aead::UnboundKey::new(&aead::CHACHA20_POLY1305, &key)
        .map_err(|_| "cannot create AEAD key")?;
    let sealing = aead::LessSafeKey::new(unbound);
    let nonce = aead::Nonce::assume_unique_for_key(nonce_bytes);

    let sender_id = identity.node_id();
    let aad = format!("{PROTOCOL}|{sender_id}|{peer_id}");
    let mut ciphertext = plaintext.to_vec();
    sealing
        .seal_in_place_append_tag(nonce, aead::Aad::from(aad.as_bytes()), &mut ciphertext)
        .map_err(|_| "encryption failed")?;

    let sender_public_key = STANDARD.encode(identity.public_key());
    let ephemeral_public_key = STANDARD.encode(ephemeral_public.as_ref());
    let nonce = STANDARD.encode(nonce_bytes);
    let ciphertext_encoded = STANDARD.encode(&ciphertext);

    let signing_input = signed_bytes(
        &sender_id,
        peer_id,
        &sender_public_key,
        &ephemeral_public_key,
        &nonce,
        &ciphertext_encoded,
    );
    let signature = signing_key(identity).sign(&signing_input);

    Ok(EncryptedMessage {
        version: 1,
        sender_id,
        sender_public_key,
        ephemeral_public_key,
        nonce,
        ciphertext: ciphertext_encoded,
        signature: STANDARD.encode(signature.as_ref()),
    })
}

pub(crate) fn verify_envelope(message: &EncryptedMessage, expected_peer_id: &str) -> bool {
    if message.version != 1 || message.sender_id.is_empty() {
        return false;
    }
    let Ok(public_key) = STANDARD.decode(&message.sender_public_key) else { return false; };
    if public_key.len() != 32 {
        return false;
    }
    let hash = digest::digest(&digest::SHA256, &public_key);
    if message.sender_id != format!("cyb-{}", hex(&hash.as_ref()[..12])) {
        return false;
    }
    let Ok(signature_bytes) = STANDARD.decode(&message.signature) else { return false; };
    let input = signed_bytes(
        &message.sender_id,
        expected_peer_id,
        &message.sender_public_key,
        &message.ephemeral_public_key,
        &message.nonce,
        &message.ciphertext,
    );
    signature::UnparsedPublicKey::new(&signature::ED25519, &public_key)
        .verify(&input, &signature_bytes)
        .is_ok()
}

fn derive_aead_key(shared: &[u8], peer_id: &[u8]) -> Result<[u8; 32], &'static str> {
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"cybOS/cybchat/e2e/v1");
    let prk = salt.extract(shared);
    let info = [b"message-key".as_slice(), peer_id].concat();
    let okm = prk.expand(&[&info], &aead::CHACHA20_POLY1305)
        .map_err(|_| "KDF failed")?;
    let mut key = [0_u8; 32];
    okm.fill(&mut key).map_err(|_| "KDF output failed")?;
    Ok(key)
}

fn signed_bytes(
    sender_id: &str,
    recipient_id: &str,
    sender_public_key: &str,
    ephemeral_public_key: &str,
    nonce: &str,
    ciphertext: &str,
) -> Vec<u8> {
    [
        PROTOCOL,
        sender_id,
        recipient_id,
        sender_public_key,
        ephemeral_public_key,
        nonce,
        ciphertext,
    ].join("|").into_bytes()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::{encrypt_for_peer, verify_envelope};
    use crate::identity::NodeIdentity;
    use crate::store::Store;
    use ring::{agreement, rand};

    #[test]
    fn envelope_authenticates_sender() {
        let store = Store::open();
        let identity = NodeIdentity::load_or_create(&store);
        let rng = rand::SystemRandom::new();
        let peer = agreement::EphemeralPrivateKey::generate(&agreement::X25519, &rng).unwrap();
        let peer_public = peer.compute_public_key().unwrap();
        let envelope = encrypt_for_peer(&identity, "cyb-peer", peer_public.as_ref(), b"hello").unwrap();
        assert!(verify_envelope(&envelope, "cyb-peer"));
    }

    #[test]
    fn tampering_invalidates_signature() {
        let store = Store::open();
        let identity = NodeIdentity::load_or_create(&store);
        let rng = rand::SystemRandom::new();
        let peer = agreement::EphemeralPrivateKey::generate(&agreement::X25519, &rng).unwrap();
        let peer_public = peer.compute_public_key().unwrap();
        let mut envelope = encrypt_for_peer(&identity, "cyb-peer", peer_public.as_ref(), b"hello").unwrap();
        envelope.ciphertext.push('x');
        assert!(!verify_envelope(&envelope, "cyb-peer"));
    }
}
