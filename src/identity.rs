//! Cryptographic node identity for cybOS.
use base64::{engine::general_purpose::STANDARD, Engine as _};
use ring::{digest, rand, signature};
use ring::signature::KeyPair;
use crate::store::Store;

const STORE_KEY: &str = "identity_ed25519_pkcs8_v1";

#[derive(Clone)]
pub(crate) struct NodeIdentity {
    pkcs8: Vec<u8>,
    public_key: Vec<u8>,
}

impl NodeIdentity {
    pub(crate) fn load_or_create(store: &Store) -> Self {
        if let Some(encoded) = store.get(STORE_KEY) {
            if let Ok(pkcs8) = STANDARD.decode(encoded) {
                if let Ok(pair) = signature::Ed25519KeyPair::from_pkcs8(&pkcs8) {
                    return Self { pkcs8, public_key: pair.public_key().as_ref().to_vec() };
                }
            }
        }

        let rng = rand::SystemRandom::new();
        let document = signature::Ed25519KeyPair::generate_pkcs8(&rng)
            .expect("OS random source must be available for cybOS identity");
        let pkcs8 = document.as_ref().to_vec();
        let pair = signature::Ed25519KeyPair::from_pkcs8(&pkcs8)
            .expect("generated Ed25519 key must parse");
        store.set(STORE_KEY, &STANDARD.encode(&pkcs8));

        Self { pkcs8, public_key: pair.public_key().as_ref().to_vec() }
    }

    pub(crate) fn node_id(&self) -> String {
        let hash = digest::digest(&digest::SHA256, &self.public_key);
        format!("cyb-{}", hex(&hash.as_ref()[..12]))
    }

    pub(crate) fn public_key(&self) -> &[u8] { &self.public_key }
    pub(crate) fn pkcs8(&self) -> &[u8] { &self.pkcs8 }
}

#[cfg(test)]
pub(crate) fn generate_for_test() -> Self {
    let rng = rand::SystemRandom::new();
    let document = signature::Ed25519KeyPair::generate_pkcs8(&rng)
        .expect("test Ed25519 generation must succeed");
    let pkcs8 = document.as_ref().to_vec();
    let pair = signature::Ed25519KeyPair::from_pkcs8(&pkcs8)
        .expect("test Ed25519 key must parse");
    Self {
        pkcs8,
        public_key: pair.public_key().as_ref().to_vec(),
    }
}


pub(crate) fn verify_node_id(node_id: &str, public_key: &[u8]) -> bool {
    let hash = digest::digest(&digest::SHA256, public_key);
    node_id == format!("cyb-{}", hex(&hash.as_ref()[..12]))
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(b"0123456789abcdef"[(byte >> 4) as usize]));
        out.push(char::from(b"0123456789abcdef"[(byte & 0x0f) as usize]));
    }
    out
}

pub(crate) fn signing_key(identity: &NodeIdentity) -> signature::Ed25519KeyPair {
    signature::Ed25519KeyPair::from_pkcs8(identity.pkcs8())
        .expect("stored cybOS identity must remain valid")
}

#[cfg(test)]
mod tests {
    use super::{hex, verify_node_id};
    use ring::{digest, rand, signature};
    use ring::signature::KeyPair;

    #[test]
    fn identity_id_is_derived_from_public_key() {
        let rng = rand::SystemRandom::new();
        let doc = signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let pair = signature::Ed25519KeyPair::from_pkcs8(doc.as_ref()).unwrap();
        let hash = digest::digest(&digest::SHA256, pair.public_key().as_ref());
        let id = format!("cyb-{}", hex(&hash.as_ref()[..12]));
        assert!(verify_node_id(&id, pair.public_key().as_ref()));
    }
}
