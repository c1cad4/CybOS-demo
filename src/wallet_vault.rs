//! Local CybOS profile recovery key. Not a Monero/CybHero spend-key derivation.
use bip39::{Language, Mnemonic};
use chacha20poly1305::{
    KeyInit, XChaCha20Poly1305, XNonce,
    aead::{Aead, Payload},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::Path,
};
use zeroize::Zeroizing;
const AAD: &[u8] = b"CybOS profile vault v1; not a blockchain wallet";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Vault {
    version: u32,
    pub id: String,
    salt: [u8; 16],
    nonce: [u8; 24],
    sealed_entropy: Vec<u8>,
}
fn random<const N: usize>() -> Result<[u8; N], String> {
    let mut value = [0; N];
    getrandom::fill(&mut value).map_err(|_| "Системный генератор случайности недоступен")?;
    Ok(value)
}
fn identity(entropy: &[u8; 32]) -> String {
    let mut hash = Sha256::new();
    hash.update(AAD);
    hash.update(entropy);
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}
fn key(password: &str, salt: &[u8; 16]) -> Result<Zeroizing<[u8; 32]>, String> {
    let params = argon2::Params::new(65536, 3, 1, Some(32)).map_err(|_| "Ошибка параметров KDF")?;
    let mut key = Zeroizing::new([0; 32]);
    argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, &mut *key)
        .map_err(|_| "Ошибка обработки пароля")?;
    Ok(key)
}
impl Vault {
    pub fn generate() -> Result<Zeroizing<String>, String> {
        let entropy = Zeroizing::new(random::<32>()?);
        let mnemonic = Mnemonic::from_entropy_in(Language::English, &*entropy)
            .map_err(|_| "Ошибка создания фразы")?;
        Ok(Zeroizing::new(mnemonic.to_string()))
    }
    fn entropy(phrase: &str) -> Result<Zeroizing<[u8; 32]>, String> {
        let mnemonic = Mnemonic::parse_in_normalized(Language::English, phrase.trim())
            .map_err(|_| "Неверная seed-фраза или контрольная сумма")?;
        if mnemonic.word_count() != 24 {
            return Err("Нужно ровно 24 слова BIP39 на английском".into());
        }
        let bytes = Zeroizing::new(mnemonic.to_entropy());
        let mut entropy = Zeroizing::new([0; 32]);
        entropy.copy_from_slice(&bytes);
        Ok(entropy)
    }
    pub fn seal(phrase: &str, password: &str) -> Result<Self, String> {
        if password.chars().count() < 12 || password.len() > 1024 {
            return Err("Пароль: от 12 символов, не больше 1024 байт".into());
        }
        let entropy = Self::entropy(phrase)?;
        let salt = random()?;
        let nonce = random()?;
        let key = key(password, &salt)?;
        let id = identity(&entropy);
        let mut aad = AAD.to_vec();
        aad.extend_from_slice(id.as_bytes());
        let cipher = XChaCha20Poly1305::new_from_slice(&*key).map_err(|_| "Ошибка ключа")?;
        let sealed_entropy = cipher
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: &*entropy,
                    aad: &aad,
                },
            )
            .map_err(|_| "Ошибка шифрования")?;
        Ok(Self {
            version: 1,
            id,
            salt,
            nonce,
            sealed_entropy,
        })
    }
    pub fn unlock(&self, password: &str) -> Result<(), String> {
        if password.len() > 1024 {
            return Err("Неверный пароль".into());
        }
        let key = key(password, &self.salt)?;
        let cipher = XChaCha20Poly1305::new_from_slice(&*key).map_err(|_| "Ошибка ключа")?;
        let mut aad = AAD.to_vec();
        aad.extend_from_slice(self.id.as_bytes());
        let entropy = Zeroizing::new(
            cipher
                .decrypt(
                    XNonce::from_slice(&self.nonce),
                    Payload {
                        msg: &self.sealed_entropy,
                        aad: &aad,
                    },
                )
                .map_err(|_| "Неверный пароль или повреждённое хранилище")?,
        );
        let value: &[u8; 32] = entropy
            .as_slice()
            .try_into()
            .map_err(|_| "Повреждённое хранилище")?;
        if identity(value) != self.id {
            return Err("Повреждённый идентификатор профиля".into());
        }
        Ok(())
    }
    pub fn recovery_matches(&self, phrase: &str) -> Result<(), String> {
        if identity(&*Self::entropy(phrase)?) != self.id {
            return Err("Фраза другого профиля".into());
        }
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NONBLOCK);
        }
        let mut file = options
            .open(path)
            .map_err(|_| "Не удалось открыть хранилище")?;
        if !file
            .metadata()
            .map_err(|_| "Не удалось проверить хранилище")?
            .is_file()
        {
            return Err("Хранилище должно быть обычным файлом".into());
        }
        let mut raw = String::new();
        std::io::Read::by_ref(&mut file)
            .take(16385)
            .read_to_string(&mut raw)
            .map_err(|_| "Не удалось прочитать хранилище")?;
        if raw.len() > 16384 {
            return Err("Хранилище слишком большое".into());
        }
        let vault: Self = serde_json::from_str(&raw)
            .map_err(|_| "Повреждённое хранилище; сброс не выполняется")?;
        if vault.version != 1
            || vault.id.len() != 64
            || !vault.id.bytes().all(|c| c.is_ascii_hexdigit())
            || vault.sealed_entropy.len() != 48
        {
            return Err("Неподдерживаемое хранилище".into());
        }
        Ok(vault)
    }
    fn write(&self, path: &Path, replace: bool) -> Result<(), String> {
        let parent = path.parent().ok_or("Нужна папка хранилища")?;
        std::fs::create_dir_all(parent).map_err(|_| "Не удалось создать папку хранилища")?;
        let suffix = random::<8>()?
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let temp = parent.join(format!(".vault-{suffix}.tmp"));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let result = (|| {
            let mut file = options
                .open(&temp)
                .map_err(|_| "Не удалось создать временный файл")?;
            let raw = serde_json::to_vec(self).map_err(|_| "Ошибка сохранения")?;
            file.write_all(&raw)
                .and_then(|_| file.sync_all())
                .map_err(|_| "Ошибка записи хранилища")?;
            if replace {
                std::fs::rename(&temp, path).map_err(|_| "Не удалось обновить хранилище")?;
            } else {
                std::fs::hard_link(&temp, path)
                    .map_err(|_| "Хранилище уже существует или недоступно")?;
            }
            Ok(())
        })();
        let _ = std::fs::remove_file(&temp);
        result
    }
    pub fn save_new(&self, path: &Path) -> Result<(), String> {
        self.write(path, false)
    }
    pub fn recover(&self, path: &Path, phrase: &str, password: &str) -> Result<Self, String> {
        self.recovery_matches(phrase)?;
        let current = Self::load(path)?;
        if serde_json::to_vec(&current).ok() != serde_json::to_vec(self).ok() {
            return Err("Хранилище изменилось; откройте заново".into());
        }
        let new = Self::seal(phrase, password)?;
        new.write(path, true)?;
        Ok(new)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    // Published BIP39 zero-entropy test vector, never a user wallet.
    const PHRASE: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    #[test]
    fn sealed_profile_rejects_wrong_password_and_tampering() {
        let mut vault = Vault::seal(PHRASE, "test-password-123").unwrap();
        vault.unlock("test-password-123").unwrap();
        assert!(vault.unlock("wrong").is_err());
        assert!(!serde_json::to_string(&vault).unwrap().contains("abandon"));
        vault.sealed_entropy[0] ^= 1;
        assert!(vault.unlock("test-password-123").is_err());
    }
    #[test]
    fn recovery_requires_exact_24_word_profile() {
        let vault = Vault::seal(PHRASE, "test-password-123").unwrap();
        vault.recovery_matches(PHRASE).unwrap();
        assert!(Vault::seal("abandon", "test-password-123").is_err());
        let other = Vault::generate().unwrap();
        assert!(vault.recovery_matches(&other).is_err());
        assert_eq!(other.split_whitespace().count(), 24);
    }
    #[test]
    fn atomic_file_recovery_and_failed_restore_preserve_profile() {
        let directory =
            std::env::temp_dir().join(format!("cybos-vault-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("vault.json");
        let vault = Vault::seal(PHRASE, "test-password-123").unwrap();
        vault.save_new(&path).unwrap();
        assert!(vault.save_new(&path).is_err());
        let before = std::fs::read(&path).unwrap();
        assert!(vault.recover(&path, "abandon", "new-password-123").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let recovered = vault.recover(&path, PHRASE, "new-password-123").unwrap();
        recovered.unlock("new-password-123").unwrap();
        assert!(recovered.unlock("test-password-123").is_err());
        assert_eq!(Vault::load(&path).unwrap().id, vault.id);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
}
