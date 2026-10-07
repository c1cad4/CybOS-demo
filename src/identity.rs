//! Cryptographic node identity for cybOS.
use base64::{engine::general_purpose::STANDARD, Engine as _};
use ring::{digest, rand, signature};
use ring::signature::KeyPair;
use crate::store::Store;

const STORE_KEY: &str = "identity_ed25519_pkcs8_v1";

#[cfg(target_os = "macos")]
const KEYCHAIN_SERVICE: &str = "to.cicada.cybos";
#[cfg(target_os = "macos")]
const KEYCHAIN_ACCOUNT: &str = "identity-ed25519-pkcs8-v1";

#[derive(Clone)]
pub(crate) struct NodeIdentity {
    pkcs8: Vec<u8>,
    public_key: Vec<u8>,
}

impl NodeIdentity {
    pub(crate) fn load_or_create(store: &Store) -> Self {
        #[cfg(target_os = "macos")]
        {
            if let Some(pkcs8) = keychain_load().expect("cannot read cybOS identity from macOS Keychain") {
                return Self::from_pkcs8(pkcs8);
            }

            // One-time migration from the legacy SQLite-backed identity.
            if let Some(encoded) = store.get(STORE_KEY) {
                if let Ok(pkcs8) = STANDARD.decode(encoded) {
                    let identity = Self::from_pkcs8(pkcs8.clone());
                    keychain_store(&pkcs8).expect("cannot migrate cybOS identity into macOS Keychain");
                    store.delete(STORE_KEY);
                    return identity;
                }
            }

            let identity = Self::generate();
            keychain_store(&identity.pkcs8).expect("cannot persist cybOS identity in macOS Keychain");
            return identity;
        }

        #[cfg(not(target_os = "macos"))]
        {
            if let Some(encoded) = store.get(STORE_KEY) {
                if let Ok(pkcs8) = STANDARD.decode(encoded) {
                    if let Ok(pair) = signature::Ed25519KeyPair::from_pkcs8(&pkcs8) {
                        return Self {
                            pkcs8,
                            public_key: pair.public_key().as_ref().to_vec(),
                        };
                    }
                }
            }

            let identity = Self::generate();
            store.set(STORE_KEY, &STANDARD.encode(&identity.pkcs8));
            identity
        }
    }

    fn generate() -> Self {
        let rng = rand::SystemRandom::new();
        let document = signature::Ed25519KeyPair::generate_pkcs8(&rng)
            .expect("OS random source must be available for cybOS identity");
        Self::from_pkcs8(document.as_ref().to_vec())
    }

    #[cfg(debug_assertions)]
    pub(crate) fn generate_ephemeral() -> Self {
        Self::generate()
    }

    fn from_pkcs8(pkcs8: Vec<u8>) -> Self {
        let pair = signature::Ed25519KeyPair::from_pkcs8(&pkcs8)
            .expect("stored cybOS identity must remain a valid Ed25519 PKCS#8 key");

        Self {
            pkcs8,
            public_key: pair.public_key().as_ref().to_vec(),
        }
    }

    pub(crate) fn node_id(&self) -> String {
        let hash = digest::digest(&digest::SHA256, &self.public_key);
        format!("cyb-{}", hex(&hash.as_ref()[..12]))
    }

    pub(crate) fn public_key(&self) -> &[u8] { &self.public_key }
    pub(crate) fn pkcs8(&self) -> &[u8] { &self.pkcs8 }

    #[cfg(test)]
    pub(crate) fn generate_for_test() -> Self {
        Self::generate()
    }
}

#[cfg(target_os = "macos")]
fn keychain_load() -> Result<Option<Vec<u8>>, String> {
    use security_framework::passwords::{generic_password, PasswordOptions};

    match generic_password(PasswordOptions::new_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.code() == -25300 => Ok(None), // errSecItemNotFound
        Err(error) => Err(format!("macOS Keychain read failed with status {}", error.code())),
    }
}

#[cfg(target_os = "macos")]
fn keychain_store(pkcs8: &[u8]) -> Result<(), String> {
    use security_framework::passwords::set_generic_password;

    set_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT, pkcs8)
        .map_err(|error| format!("macOS Keychain write failed with status {}", error.code()))
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
    use super::{hex, verify_node_id, NodeIdentity};
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

    #[test]
    fn generated_identity_has_stable_node_id() {
        let identity = NodeIdentity::generate_for_test();
        assert_eq!(identity.node_id(), identity.node_id());
        assert!(verify_node_id(&identity.node_id(), identity.public_key()));
    }
}
