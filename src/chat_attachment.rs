//! Bounded versioned attachments carried inside the existing authenticated chat envelope.
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;
pub(crate) const PREFIX: &str = "CYBCHAT-FILE/1:";
pub(crate) const MAX_BYTES: usize = 8 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    id: String,
    name: String,
    bytes: usize,
    sha256: String,
    content: String,
}
pub(crate) struct Attachment {
    pub name: String,
    pub content: Zeroizing<Vec<u8>>,
}
fn name_valid(name: &str) -> bool {
    !name.trim().is_empty()
        && name.len() <= 255
        && name != "."
        && name != ".."
        && !name
            .chars()
            .any(|c| c.is_control() || c == '/' || c == '\\')
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub(crate) fn encode(name: &str, bytes: &[u8]) -> Result<String, String> {
    if !name_valid(name) || bytes.len() > MAX_BYTES {
        return Err("Вложение: безопасное имя до 255 байт и размер до 8 КиБ".into());
    }
    let wire = Wire {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.into(),
        bytes: bytes.len(),
        sha256: digest(bytes),
        content: STANDARD.encode(bytes),
    };
    let message = format!(
        "{PREFIX}{}",
        serde_json::to_string(&wire).map_err(|_| "Не удалось подготовить вложение")?
    );
    if message.len() > 16 * 1024 {
        return Err("Вложение превышает лимит сообщения".into());
    }
    Ok(message)
}
pub(crate) fn decode(message: &str) -> Result<Attachment, String> {
    if message.len() > 16 * 1024 {
        return Err("Вложение превышает лимит сообщения".into());
    }
    let wire: Wire = serde_json::from_str(message.strip_prefix(PREFIX).ok_or("Не вложение")?)
        .map_err(|_| "Повреждённое вложение")?;
    if uuid::Uuid::parse_str(&wire.id).is_err()
        || !name_valid(&wire.name)
        || wire.bytes > MAX_BYTES
        || wire.content.len() > MAX_BYTES.div_ceil(3) * 4
    {
        return Err("Недопустимые параметры вложения".into());
    }
    let content = Zeroizing::new(
        STANDARD
            .decode(wire.content)
            .map_err(|_| "Повреждённые данные вложения")?,
    );
    if content.len() != wire.bytes || digest(&content) != wire.sha256 {
        return Err("Контрольная сумма вложения не совпадает".into());
    }
    Ok(Attachment {
        name: wire.name,
        content,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn binary_roundtrip_fits_existing_chat_frame() {
        let bytes = vec![255; MAX_BYTES];
        let message = encode("документ.bin", &bytes).unwrap();
        assert!(message.len() < 16 * 1024);
        let file = decode(&message).unwrap();
        assert_eq!(file.name, "документ.bin");
        assert_eq!(*file.content, bytes);
        assert!(encode("large", &vec![0; MAX_BYTES + 1]).is_err());
    }
    #[test]
    fn untrusted_paths_sizes_hashes_and_versions_are_refused() {
        for name in ["../file", "/file", "C:\\file", "..", "bad\nname"] {
            assert!(encode(name, b"x").is_err());
        }
        let message = encode("safe", b"abc").unwrap();
        for changed in [
            message.replace("YWJj", "YWJk"),
            message.replace("\"bytes\":3", "\"bytes\":8193"),
            message.replace("\"name\":\"safe\"", "\"name\":\"../evil\""),
            message.replace("FILE/1", "FILE/2"),
        ] {
            assert!(decode(&changed).is_err());
        }
    }
}
