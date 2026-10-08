//! Authenticated CybChat key exchange and encrypted wire primitives.
use base64::{engine::general_purpose::STANDARD, Engine as _};
use ring::{aead, agreement, digest, hkdf, rand, signature};
use crate::identity::{signing_key, NodeIdentity};

pub(crate) const PROTOCOL: &str = "cybchat-wire-v2";
pub(crate) const NONCE_LEN: usize = aead::NONCE_LEN;

pub(crate) fn sign(identity: &NodeIdentity, payload: &[u8]) -> Vec<u8> {
    signing_key(identity).sign(payload).as_ref().to_vec()
}

pub(crate) fn verify_signature(public_key: &[u8], payload: &[u8], sig: &[u8]) -> bool {
    if public_key.len() != 32 { return false; }
    signature::UnparsedPublicKey::new(&signature::ED25519, public_key)
        .verify(payload, sig).is_ok()
}

pub(crate) fn peer_binding(node_id: &str, version: &str, public_key_b64: &str) -> Vec<u8> {
    [PROTOCOL, "peer", node_id, version, public_key_b64].join("|").into_bytes()
}

pub(crate) fn peer_discovery_binding(
    node_id: &str,
    version: &str,
    public_key_b64: &str,
    challenge: &str,
) -> Vec<u8> {
    [
        PROTOCOL,
        "peer-discovery-v2",
        node_id,
        version,
        public_key_b64,
        challenge,
    ]
    .join("|")
    .into_bytes()
}

pub(crate) fn ephemeral() -> Result<(agreement::EphemeralPrivateKey, Vec<u8>), &'static str> {
    let rng = rand::SystemRandom::new();
    let private = agreement::EphemeralPrivateKey::generate(&agreement::X25519, &rng)
        .map_err(|_| "cannot create X25519 ephemeral key")?;
    let public = private.compute_public_key()
        .map_err(|_| "cannot compute X25519 public key")?;
    Ok((private, public.as_ref().to_vec()))
}

pub(crate) fn derive_session_key(
    private: agreement::EphemeralPrivateKey,
    peer_public: &[u8],
    transcript: &[u8],
) -> Result<[u8; 32], &'static str> {
    if peer_public.len() != 32 { return Err("invalid X25519 peer public key"); }
    let peer = agreement::UnparsedPublicKey::new(&agreement::X25519, peer_public);
    let shared = agreement::agree_ephemeral(private, &peer, |secret| secret.to_vec())
        .map_err(|_| "X25519 agreement failed")?;
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"cybOS/cybchat/wire/v2");
    let prk = salt.extract(&shared);
    let info = [b"session-key".as_slice(), transcript].concat();
    let refs: [&[u8]; 1] = [&info];
    let okm = prk.expand(&refs, &aead::CHACHA20_POLY1305)
        .map_err(|_| "HKDF failed")?;
    let mut key = [0u8; 32];
    okm.fill(&mut key).map_err(|_| "HKDF output failed")?;
    Ok(key)
}

pub(crate) fn encrypt(
    key: &[u8; 32],
    aad: &[u8],
    plaintext: &[u8],
) -> Result<(String, String), &'static str> {
    let rng = rand::SystemRandom::new();
    let mut nonce = [0u8; NONCE_LEN];
    rand::SecureRandom::fill(&rng, &mut nonce).map_err(|_| "cannot create nonce")?;
    let unbound = aead::UnboundKey::new(&aead::CHACHA20_POLY1305, key)
        .map_err(|_| "cannot create AEAD key")?;
    let cipher = aead::LessSafeKey::new(unbound);
    let mut ciphertext = plaintext.to_vec();
    cipher.seal_in_place_append_tag(
        aead::Nonce::assume_unique_for_key(nonce),
        aead::Aad::from(aad),
        &mut ciphertext,
    ).map_err(|_| "encryption failed")?;
    Ok((STANDARD.encode(nonce), STANDARD.encode(ciphertext)))
}

pub(crate) fn decrypt(
    key: &[u8; 32],
    aad: &[u8],
    nonce_b64: &str,
    ciphertext_b64: &str,
) -> Result<Vec<u8>, &'static str> {
    let nonce = STANDARD.decode(nonce_b64).map_err(|_| "invalid nonce")?;
    if nonce.len() != NONCE_LEN { return Err("invalid nonce length"); }
    let mut ciphertext = STANDARD.decode(ciphertext_b64).map_err(|_| "invalid ciphertext")?;
    let unbound = aead::UnboundKey::new(&aead::CHACHA20_POLY1305, key)
        .map_err(|_| "cannot create AEAD key")?;
    let cipher = aead::LessSafeKey::new(unbound);
    let plaintext = cipher.open_in_place(
        aead::Nonce::try_assume_unique_for_key(&nonce).map_err(|_| "invalid nonce")?,
        aead::Aad::from(aad),
        &mut ciphertext,
    ).map_err(|_| "authentication failed")?;
    Ok(plaintext.to_vec())
}

pub(crate) fn onion_layer_key(
    hop_session_key: &[u8; 32],
    route_id: &str,
    packet_id: &str,
    hop_index: u8,
    direction: &str,
) -> Result<[u8; 32], &'static str> {
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"cybOS/cybchat/onion/v1");
    let prk = salt.extract(hop_session_key);
    let info = [
        b"onion-layer-key".as_slice(),
        route_id.as_bytes(),
        packet_id.as_bytes(),
        &hop_index.to_be_bytes(),
        direction.as_bytes(),
    ]
    .concat();
    let refs: [&[u8]; 1] = [&info];
    let okm = prk
        .expand(&refs, &aead::CHACHA20_POLY1305)
        .map_err(|_| "onion HKDF failed")?;
    let mut key = [0u8; 32];
    okm.fill(&mut key).map_err(|_| "onion HKDF output failed")?;
    Ok(key)
}

pub(crate) fn local_storage_key(secret: &[u8]) -> [u8; 32] {
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"cybOS/local-storage/v1");
    let prk = salt.extract(secret);
    let refs: [&[u8]; 1] = [b"cybchat-sqlite-v1"];
    let okm = prk
        .expand(&refs, &aead::CHACHA20_POLY1305)
        .expect("local storage HKDF parameters must be valid");
    let mut key = [0u8; 32];
    okm.fill(&mut key)
        .expect("local storage HKDF output must be 32 bytes");
    key
}

pub(crate) fn envelope_bytes(
    message_id: &str,
    from: &str,
    to: &str,
    timestamp: u64,
    nonce: &str,
    ciphertext: &str,
) -> Vec<u8> {
    [
        PROTOCOL, "message", message_id, from, to, &timestamp.to_string(), nonce, ciphertext,
    ].join("|").into_bytes()
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) fn fingerprint(public_key: &[u8]) -> String {
    let hash = digest::digest(&digest::SHA256, public_key);
    hex(hash.as_ref())
}

pub(crate) fn node_id_from_public_key(public_key: &[u8]) -> String {
    let hash = digest::digest(&digest::SHA256, public_key);
    format!("cyb-{}", hex(&hash.as_ref()[..12]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::NodeIdentity;
    use crate::store::Store;

    #[test]
    fn ratchet_is_symmetric_and_counter_bound() {
        let root = [42u8; 32];
        let k1 = ratchet_key(&root, 1).unwrap();
        let next = ratchet_chain(&root, 1).unwrap();
        let k2 = ratchet_key(&next, 2).unwrap();
        assert_ne!(k1, k2);
        assert_eq!(ratchet_key(&root, 1).unwrap(), k1);
        assert_ne!(ratchet_key(&root, 2).unwrap(), k1);
    }

    #[test]
    fn x25519_handshake_roundtrips_and_encrypts() {
        let (alice_private, alice_public) = ephemeral().unwrap();
        let (bob_private, bob_public) = ephemeral().unwrap();
        let transcript = [alice_public.as_slice(), bob_public.as_slice()].concat();
        let a = derive_session_key(alice_private, &bob_public, &transcript).unwrap();
        let b = derive_session_key(bob_private, &alice_public, &transcript).unwrap();
        assert_eq!(a, b);
        let (nonce, ciphertext) = encrypt(&a, b"aad", b"hello").unwrap();
        assert_eq!(decrypt(&b, b"aad", &nonce, &ciphertext).unwrap(), b"hello");
    }

    #[test]
    fn aead_tampering_fails() {
        let key = [9u8; 32];
        let (nonce, ciphertext) = encrypt(&key, b"aad", b"hello").unwrap();

        assert!(decrypt(&key, b"tampered-aad", &nonce, &ciphertext).is_err());

        let mut bytes = STANDARD.decode(&ciphertext).unwrap();
        bytes[0] ^= 0x01;
        let tampered_ciphertext = STANDARD.encode(bytes);
        assert!(decrypt(&key, b"aad", &nonce, &tampered_ciphertext).is_err());
    }

    #[test]
    fn onion_layer_key_is_scoped_and_directional() {
        let root = [21u8; 32];
        let a = onion_layer_key(&root, "route-a", "packet-a", 0, "forward").unwrap();
        let b = onion_layer_key(&root, "route-a", "packet-a", 0, "forward").unwrap();
        let c = onion_layer_key(&root, "route-a", "packet-a", 1, "forward").unwrap();
        let d = onion_layer_key(&root, "route-a", "packet-a", 0, "reverse").unwrap();
        let e = onion_layer_key(&root, "route-b", "packet-a", 0, "forward").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
        assert_ne!(a, e);
    }

    #[test]
    fn local_storage_key_is_deterministic_and_scoped() {
        let a = local_storage_key(b"identity-key");
        let b = local_storage_key(b"identity-key");
        let c = local_storage_key(b"different-identity-key");

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn envelope_signature_rejects_wrong_recipient() {
        let store = Store::open();
        let identity = NodeIdentity::load_or_create(&store);
        let message = EncryptedMessage {
            version: 1,
            sender_id: identity.node_id(),
            sender_public_key: STANDARD.encode(identity.public_key()),
            ephemeral_public_key: STANDARD.encode([1u8; 32]),
            nonce: STANDARD.encode([2u8; NONCE_LEN]),
            ciphertext: STANDARD.encode(b"ciphertext"),
            signature: STANDARD.encode([0u8; 64]),
        };

        assert!(!verify_envelope(&message, "cyb-wrong-recipient"));
    }

    #[test]
    fn identity_signature_verifies() {
        let store = Store::open();
        let identity = NodeIdentity::load_or_create(&store);
        let payload = b"cybOS wire test";
        let sig = sign(&identity, payload);
        assert!(verify_signature(identity.public_key(), payload, &sig));
        assert!(!verify_signature(identity.public_key(), b"tampered", &sig));
    }
}
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
    let (private, ephemeral_public) = ephemeral()?;
    let sender_id = identity.node_id();
    let transcript = [
        PROTOCOL.as_bytes(), sender_id.as_bytes(), peer_id.as_bytes(),
        ephemeral_public.as_slice(), peer_x25519_public,
    ].concat();
    let key = derive_session_key(private, peer_x25519_public, &transcript)?;
    let aad = [PROTOCOL, "self-test", &sender_id, peer_id].join("|");
    let (nonce, ciphertext) = encrypt(&key, aad.as_bytes(), plaintext)?;
    let sender_public_key = STANDARD.encode(identity.public_key());
    let ephemeral_public_key = STANDARD.encode(&ephemeral_public);
    let signed = [
        PROTOCOL, "self-test", &sender_id, peer_id,
        &sender_public_key, &ephemeral_public_key, &nonce, &ciphertext,
    ].join("|").into_bytes();
    Ok(EncryptedMessage {
        version: 1,
        sender_id,
        sender_public_key,
        ephemeral_public_key,
        nonce,
        ciphertext,
        signature: STANDARD.encode(sign(identity, &signed)),
    })
}

pub(crate) fn verify_envelope(message: &EncryptedMessage, expected_peer_id: &str) -> bool {
    if message.version != 1 || message.sender_id != expected_peer_id {
        return false;
    }
    let Ok(public_key) = STANDARD.decode(&message.sender_public_key) else { return false };
    let Ok(sig) = STANDARD.decode(&message.signature) else { return false };
    if node_id_from_public_key(&public_key) != message.sender_id {
        return false;
    }
    let signed = [
        PROTOCOL, "self-test", &message.sender_id, expected_peer_id,
        &message.sender_public_key, &message.ephemeral_public_key,
        &message.nonce, &message.ciphertext,
    ].join("|").into_bytes();
    verify_signature(&public_key, &signed, &sig)
}


pub(crate) fn ratchet_key(chain_key: &[u8; 32], counter: u64) -> Result<[u8; 32], &'static str> {
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"cybOS/cybchat/ratchet/v1");
    let prk = salt.extract(chain_key);
    let info = format!("message-key:{counter}");
    let refs: [&[u8]; 1] = [info.as_bytes()];
    let okm = prk.expand(&refs, &aead::CHACHA20_POLY1305).map_err(|_| "ratchet HKDF failed")?;
    let mut key = [0u8; 32];
    okm.fill(&mut key).map_err(|_| "ratchet output failed")?;
    Ok(key)
}

pub(crate) fn ratchet_chain(chain_key: &[u8; 32], counter: u64) -> Result<[u8; 32], &'static str> {
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"cybOS/cybchat/ratchet-chain/v1");
    let prk = salt.extract(chain_key);
    let info = format!("chain-key:{counter}");
    let refs: [&[u8]; 1] = [info.as_bytes()];
    let okm = prk.expand(&refs, &aead::CHACHA20_POLY1305).map_err(|_| "chain HKDF failed")?;
    let mut key = [0u8; 32];
    okm.fill(&mut key).map_err(|_| "chain output failed")?;
    Ok(key)
}
