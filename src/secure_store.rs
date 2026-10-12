//! SQLCipher connections and reversible snapshot migration of the legacy SQLite file.
use rusqlite::{Connection, params};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};
use zeroize::Zeroizing;
static SESSION_KEY: Mutex<Option<Zeroizing<[u8; 32]>>> = Mutex::new(None);

pub(crate) fn set_session_key(key: Zeroizing<[u8; 32]>) -> Result<(), String> {
    let mut slot = SESSION_KEY
        .lock()
        .map_err(|_| "Сессия хранилища недоступна")?;
    if slot.is_some() {
        return Err("Ключ сессии уже установлен".into());
    }
    *slot = Some(key);
    Ok(())
}
pub(crate) fn clear_session_key() {
    if let Ok(mut slot) = SESSION_KEY.lock() {
        *slot = None;
    }
}
pub(crate) fn open_session(base: &Path) -> Result<Connection, String> {
    let slot = SESSION_KEY
        .lock()
        .map_err(|_| "Сессия хранилища недоступна")?;
    let key = slot
        .as_ref()
        .ok_or("Вход в профиль требуется до открытия базы")?;
    let path = base.join("cybos-encrypted.db");
    if !path.is_file() {
        return Err("Зашифрованная база не подготовлена; открытый fallback запрещён".into());
    }
    open(&path, key)
}
fn raw_key(key: &[u8; 32]) -> Zeroizing<String> {
    Zeroizing::new(format!(
        "x'{}'",
        key.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}
fn cipher_available(conn: &Connection) -> Result<(), String> {
    let version: String = conn
        .query_row("PRAGMA cipher_version", [], |row| row.get(0))
        .map_err(|_| "SQLCipher недоступен; открытая база не используется")?;
    if version.is_empty() {
        return Err("SQLCipher недоступен".into());
    }
    Ok(())
}
pub(crate) fn open(path: &Path, key: &[u8; 32]) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|_| "Не удалось открыть зашифрованную базу")?;
    cipher_available(&conn)?;
    let encoded = raw_key(key);
    conn.pragma_update(None, "key", encoded.as_str())
        .map_err(|_| "Не удалось установить ключ базы")?;
    conn.execute_batch("PRAGMA temp_store=MEMORY; PRAGMA cipher_memory_security=ON;")
        .map_err(|_| "Не удалось настроить защиту базы")?;
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |row| {
        row.get::<_, i64>(0)
    })
    .map_err(|_| "Ключ базы не подходит или зашифрованная база повреждена")?;
    Ok(conn)
}
fn integrity(conn: &Connection) -> Result<(), String> {
    let status: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|_| "Проверка базы не удалась")?;
    if status != "ok" {
        return Err("Целостность базы не подтверждена".into());
    }
    Ok(())
}

pub(crate) fn prepare(base: &Path, key: &[u8; 32]) -> Result<PathBuf, String> {
    std::fs::create_dir_all(base).map_err(|_| "Папка данных недоступна")?;
    let target = base.join("cybos-encrypted.db");
    if target
        .try_exists()
        .map_err(|_| "Не удалось проверить базу")?
    {
        let conn = open(&target, key)?;
        integrity(&conn)?;
        return Ok(target);
    }
    let temporary = base.join(format!(".cybos-encrypted-{}.tmp", uuid::Uuid::new_v4()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(&temporary)
        .map_err(|_| "Не удалось создать закрытый временный файл")?;
    let legacy = base.join("cybos.db");
    let result = (|| {
        if legacy
            .try_exists()
            .map_err(|_| "Не удалось проверить старую базу")?
        {
            if !legacy.is_file() {
                return Err("Старая база должна быть обычным файлом".into());
            }
            let source = Connection::open(&legacy).map_err(|_| "Старая база недоступна")?;
            cipher_available(&source)?;
            source
                .busy_timeout(std::time::Duration::from_secs(5))
                .map_err(|_| "Не удалось настроить миграцию")?;
            source
                .execute_batch("PRAGMA temp_store=MEMORY; BEGIN;")
                .map_err(|_| "Не удалось открыть снимок старой базы")?;
            integrity(&source)?;
            let version: i64 = source
                .pragma_query_value(None, "user_version", |row| row.get(0))
                .map_err(|_| "Метаданные старой базы недоступны")?;
            let encoded = raw_key(key);
            source
                .execute(
                    "ATTACH DATABASE ?1 AS encrypted KEY ?2",
                    params![
                        temporary.to_str().ok_or("Путь базы не UTF-8")?,
                        encoded.as_str()
                    ],
                )
                .map_err(|_| "Не удалось подготовить зашифрованную копию")?;
            source
                .query_row("SELECT sqlcipher_export('encrypted')", [], |_| Ok(()))
                .map_err(|_| "Миграция не завершена; старые данные сохранены")?;
            source
                .pragma_update(Some("encrypted"), "user_version", version)
                .map_err(|_| "Не удалось перенести метаданные")?;
            source
                .execute_batch("COMMIT; DETACH DATABASE encrypted;")
                .map_err(|_| "Не удалось завершить снимок")?;
        } else {
            let conn = open(&temporary, key)?;
            // Materialize an encrypted page even for a new empty profile.
            conn.execute_batch("CREATE TABLE cybos_storage_meta(version INTEGER NOT NULL);INSERT INTO cybos_storage_meta VALUES(1);").map_err(|_|"Не удалось создать защищённую базу")?;
        }
        let verified = open(&temporary, key)?;
        integrity(&verified)?;
        drop(verified);
        std::fs::OpenOptions::new()
            .write(true)
            .open(&temporary)
            .and_then(|file| file.sync_all())
            .map_err(|_| "Не удалось сохранить зашифрованную копию")?;
        std::fs::hard_link(&temporary, &target)
            .map_err(|_| "Зашифрованная база уже появилась или публикация недоступна")?;
        #[cfg(unix)]
        {
            let _ = std::fs::File::open(base).and_then(|directory| directory.sync_all());
        }
        Ok(target.clone())
    })();
    let _ = std::fs::remove_file(&temporary);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("cybos-cipher-test-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn migration_preserves_plain_source_and_encrypts_search_and_keys() {
        let folder = Fixture::new();
        let legacy = folder.0.join("cybos.db");
        let source = Connection::open(&legacy).unwrap();
        source.execute_batch("PRAGMA journal_mode=WAL;PRAGMA user_version=7;CREATE TABLE secrets(value TEXT);INSERT INTO secrets VALUES('private-marker-not-on-disk');CREATE VIRTUAL TABLE search USING fts5(text);INSERT INTO search VALUES('knowledge-marker-not-on-disk');").unwrap();
        let path = prepare(&folder.0, &[9; 32]).unwrap();
        let encrypted = open(&path, &[9; 32]).unwrap();
        assert_eq!(
            encrypted
                .query_row("SELECT value FROM secrets", [], |row| row
                    .get::<_, String>(0))
                .unwrap(),
            "private-marker-not-on-disk"
        );
        assert_eq!(
            encrypted
                .query_row(
                    "SELECT count(*) FROM search WHERE search MATCH 'knowledge'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        assert_eq!(
            encrypted
                .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            7
        );
        assert!(open(&path, &[8; 32]).is_err());
        encrypted.execute_batch("PRAGMA journal_mode=WAL;INSERT INTO secrets VALUES('wal-private-marker-never-plaintext');").unwrap();
        let wal = std::fs::read(path.with_file_name("cybos-encrypted.db-wal")).unwrap();
        assert!(
            !wal.windows(b"wal-private-marker-never-plaintext".len())
                .any(|bytes| bytes == b"wal-private-marker-never-plaintext")
        );
        let reopened = open(&path, &[9; 32]).unwrap();
        assert_eq!(
            reopened
                .query_row("SELECT count(*) FROM secrets", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            2
        );
        drop(reopened);

        let bytes = std::fs::read(&path).unwrap();
        assert!(!bytes.starts_with(b"SQLite format 3"));
        assert!(
            !bytes
                .windows(26)
                .any(|w| w == b"private-marker-not-on-disk")
        );
        assert_eq!(
            source
                .query_row("SELECT count(*) FROM secrets", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(prepare(&folder.0, &[9; 32]).unwrap(), path);
    }
    #[test]
    fn corrupted_existing_target_cannot_trigger_plaintext_fallback() {
        let folder = Fixture::new();
        std::fs::write(folder.0.join("cybos-encrypted.db"), b"broken").unwrap();
        assert!(prepare(&folder.0, &[9; 32]).is_err());
        assert_eq!(
            std::fs::read(folder.0.join("cybos-encrypted.db")).unwrap(),
            b"broken"
        );
    }
}
