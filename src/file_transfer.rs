//! Resumable, stop-and-wait transfer over bounded authenticated chat envelopes.
use base64::{Engine, engine::general_purpose::STANDARD};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Seek, SeekFrom, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use zeroize::Zeroizing;
pub(crate) const PREFIX: &str = "CYBCHAT-TRANSFER/1:";
const PART: usize = 8192;
const LIMIT: u64 = 32 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum Packet {
    Offer {
        id: String,
        name: String,
        size: u64,
        hash: String,
    },
    Chunk {
        id: String,
        index: u64,
        hash: String,
        data: String,
    },
    Finish {
        id: String,
    },
}
#[derive(Clone)]
pub(crate) struct Status {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub done: u64,
    pub phase: String,
    pub outgoing: bool,
}
pub(crate) fn schema(c: &Connection) -> Result<(), String> {
    c.execute_batch("CREATE TABLE IF NOT EXISTS file_in(id TEXT PRIMARY KEY,peer TEXT NOT NULL,name TEXT NOT NULL,size INTEGER NOT NULL,hash TEXT NOT NULL,next INTEGER NOT NULL DEFAULT 0,phase TEXT NOT NULL DEFAULT 'receiving',file TEXT);
 CREATE TABLE IF NOT EXISTS file_parts(id TEXT NOT NULL,part INTEGER NOT NULL,data BLOB NOT NULL,PRIMARY KEY(id,part));
 CREATE TABLE IF NOT EXISTS file_out(id TEXT PRIMARY KEY,peer TEXT NOT NULL,file TEXT NOT NULL,name TEXT NOT NULL,size INTEGER NOT NULL,hash TEXT NOT NULL,next INTEGER NOT NULL DEFAULT 0,phase TEXT NOT NULL DEFAULT 'paused');").map_err(|_|"Таблицы передач недоступны".into())
}
fn hash(data: &[u8]) -> String {
    hex(&Sha256::digest(data))
}
fn hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}
fn valid_hash(h: &str) -> bool {
    h.len() == 64
        && h.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn wire(p: &Packet) -> Result<String, String> {
    let s = format!(
        "{PREFIX}{}",
        serde_json::to_string(p).map_err(|_| "Ошибка пакета")?
    );
    if s.len() > 16 * 1024 {
        return Err("Пакет слишком большой".into());
    }
    Ok(s)
}
fn parse(s: &str) -> Result<Packet, String> {
    if s.len() > 16 * 1024 {
        return Err("Пакет слишком большой".into());
    }
    let p: Packet = serde_json::from_str(s.strip_prefix(PREFIX).ok_or("Не передача")?)
        .map_err(|_| "Повреждённый пакет передачи")?;
    let id = match &p {
        Packet::Offer { id, .. } | Packet::Chunk { id, .. } | Packet::Finish { id } => id,
    };
    uuid::Uuid::parse_str(id).map_err(|_| "Некорректный ID передачи")?;
    Ok(p)
}
fn chunk(c: &Connection, table: &str, id: &str, index: u64) -> Result<Zeroizing<Vec<u8>>, String> {
    if table != "protected_files" {
        return Err("Недопустимая таблица".into());
    }
    let (row, size): (i64, u64) = c
        .query_row(
            "SELECT rowid,length(content) FROM protected_files WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| "Исходный файл недоступен")?;
    if size > LIMIT || index >= size.div_ceil(PART as u64) {
        return Err("Недопустимая позиция файла".into());
    }
    let offset = index * PART as u64;
    let mut bytes = Zeroizing::new(vec![0; (size - offset).min(PART as u64) as usize]);
    let mut blob = c
        .blob_open("main", "protected_files", "content", row, true)
        .map_err(|_| "Не удалось открыть исходный файл")?;
    blob.seek(SeekFrom::Start(offset))
        .and_then(|_| blob.read_exact(&mut bytes))
        .map_err(|_| "Не удалось прочитать часть файла")?;
    Ok(bytes)
}

pub(crate) fn create(c: &Connection, peer: &str, file: &str) -> Result<String, String> {
    schema(c)?;
    let (name, size): (String, u64) = c
        .query_row(
            "SELECT name,length(content) FROM protected_files WHERE id=?1",
            [file],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| "Файл недоступен")?;
    if size > LIMIT || crate::chat_attachment::encode(&name, b"").is_err() {
        return Err("Недопустимое имя или размер файла".into());
    }
    let mut digest = Sha256::new();
    for index in 0..size.div_ceil(PART as u64) {
        digest.update(&*chunk(c, "protected_files", file, index)?);
    }
    let id = uuid::Uuid::new_v4().to_string();
    c.execute(
        "INSERT INTO file_out(id,peer,file,name,size,hash) VALUES(?1,?2,?3,?4,?5,?6)",
        params![id, peer, file, name, size, hex(&digest.finalize())],
    )
    .map_err(|_| "Не удалось создать передачу")?;
    Ok(id)
}
pub(crate) fn statuses(c: &Connection, peer: &str) -> Result<Vec<Status>, String> {
    schema(c)?;
    let mut stmt=c.prepare("SELECT id,name,size,done,phase,outgoing FROM (SELECT id,name,size,min(next*8192,size) AS done,phase,1 AS outgoing,rowid AS ordinal FROM file_out WHERE peer=?1 UNION ALL SELECT id,name,size,min(next*8192,size),phase,0,rowid FROM file_in WHERE peer=?1) ORDER BY CASE WHEN phase IN ('complete','cancelled') THEN 1 ELSE 0 END,ordinal DESC LIMIT 128").map_err(|_|"Список передач недоступен")?;
    stmt.query_map([peer], |r| {
        Ok(Status {
            id: r.get(0)?,
            name: r.get(1)?,
            size: r.get(2)?,
            done: r.get(3)?,
            phase: r.get(4)?,
            outgoing: r.get(5)?,
        })
    })
    .map_err(|_| "Список передач недоступен")?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|_| "Список передач повреждён".into())
}
/// Called inside the same transaction as replay protection, before ACK.
pub(crate) fn receive(tx: &Transaction, peer: &str, text: &str) -> Result<(), String> {
    match parse(text)? {
        Packet::Offer {
            id,
            name,
            size,
            hash,
        } => {
            if size > LIMIT
                || !valid_hash(&hash)
                || crate::chat_attachment::encode(&name, b"").is_err()
            {
                return Err("Недопустимое предложение файла".into());
            }
            let existing: Option<(String, String, u64, String)> = tx
                .query_row(
                    "SELECT peer,name,size,hash FROM file_in WHERE id=?1",
                    [&id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .optional()
                .map_err(|_| "Ошибка передачи")?;
            if let Some(e) = existing {
                if e != (peer.into(), name, size, hash) {
                    return Err("ID передачи уже принадлежит другому файлу или участнику".into());
                }
                return Ok(());
            }
            let active: u64 = tx
                .query_row(
                    "SELECT count(*) FROM file_in WHERE phase='receiving'",
                    [],
                    |r| r.get(0),
                )
                .map_err(|_| "Ошибка квоты")?;
            let ledger: u64 = tx
                .query_row("SELECT count(*) FROM file_in", [], |r| r.get(0))
                .map_err(|_| "Ошибка квоты")?;
            let reserved: u64 = tx
                .query_row(
                    "SELECT coalesce(sum(size),0) FROM file_in WHERE phase='receiving'",
                    [],
                    |r| r.get(0),
                )
                .map_err(|_| "Ошибка квоты")?;
            let stored: u64 = tx
                .query_row(
                    "SELECT coalesce(sum(length(content)),0) FROM protected_files",
                    [],
                    |r| r.get(0),
                )
                .map_err(|_| "Ошибка квоты")?;
            let files: u64 = tx
                .query_row("SELECT count(*) FROM protected_files", [], |r| r.get(0))
                .map_err(|_| "Ошибка квоты")?;
            if files.saturating_add(active) >= 1024 {
                return Err("Лимит защищённых файлов исчерпан".into());
            }
            if active >= 8
                || ledger >= 1024
                || stored.saturating_add(reserved).saturating_add(size) > 256 * 1024 * 1024
            {
                return Err("Квота входящих файлов исчерпана".into());
            }
            tx.execute(
                "INSERT INTO file_in(id,peer,name,size,hash) VALUES(?1,?2,?3,?4,?5)",
                params![id, peer, name, size, hash],
            )
            .map_err(|_| "Не удалось сохранить предложение")?;
        }
        Packet::Chunk {
            id,
            index,
            hash: expected,
            data,
        } => {
            if data.len() > PART.div_ceil(3) * 4 || !valid_hash(&expected) {
                return Err("Часть слишком велика".into());
            }
            let bytes = Zeroizing::new(STANDARD.decode(data).map_err(|_| "Повреждённая часть")?);
            if hash(&bytes) != expected {
                return Err("Контрольная сумма части не совпадает".into());
            }
            let (size, next, phase): (u64, u64, String) = tx
                .query_row(
                    "SELECT size,next,phase FROM file_in WHERE id=?1 AND peer=?2",
                    params![id, peer],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .map_err(|_| "Сначала требуется предложение от этого участника")?;
            if phase == "cancelled"
                || index >= size.div_ceil(PART as u64)
                || bytes.len() as u64 != (size - index * PART as u64).min(PART as u64)
            {
                return Err("Недопустимая часть".into());
            }
            if index < next {
                if phase == "complete" {
                    let file: String = tx
                        .query_row(
                            "SELECT file FROM file_in WHERE id=?1 AND peer=?2",
                            params![id, peer],
                            |r| r.get(0),
                        )
                        .map_err(|_| "Завершённый файл недоступен")?;
                    let old = chunk(tx, "protected_files", &file, index)?;
                    if old.as_slice() != bytes.as_slice() {
                        return Err("Повторная часть отличается".into());
                    }
                    return Ok(());
                }
                let old: Vec<u8> = tx
                    .query_row(
                        "SELECT data FROM file_parts WHERE id=?1 AND part=?2",
                        params![id, index],
                        |r| r.get(0),
                    )
                    .map_err(|_| "Сохранённая часть отсутствует")?;
                if old.as_slice() != bytes.as_slice() {
                    return Err("Повторная часть отличается".into());
                }
                return Ok(());
            }
            if index != next || phase != "receiving" {
                return Err("Части должны идти по порядку".into());
            }
            tx.execute(
                "INSERT INTO file_parts(id,part,data) VALUES(?1,?2,?3)",
                params![id, index, bytes.as_slice()],
            )
            .map_err(|_| "Не удалось сохранить часть")?;
            tx.execute("UPDATE file_in SET next=next+1 WHERE id=?1", [id])
                .map_err(|_| "Не удалось сохранить позицию")?;
        }
        Packet::Finish { id } => {
            let (name, size, next, expected, phase): (String, u64, u64, String, String) = tx
                .query_row(
                    "SELECT name,size,next,hash,phase FROM file_in WHERE id=?1 AND peer=?2",
                    params![id, peer],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                )
                .map_err(|_| "Передача не найдена")?;
            if phase == "complete" {
                return Ok(());
            }
            if phase != "receiving" || next != size.div_ceil(PART as u64) {
                return Err("Не все части получены".into());
            }
            let count: u64 = tx
                .query_row("SELECT count(*) FROM protected_files", [], |r| r.get(0))
                .map_err(|_| "Ошибка квоты")?;
            if count >= 1024 {
                return Err("Лимит защищённых файлов исчерпан".into());
            }
            let stored: u64 = tx
                .query_row(
                    "SELECT coalesce(sum(length(content)),0) FROM protected_files",
                    [],
                    |r| r.get(0),
                )
                .map_err(|_| "Ошибка квоты")?;
            if stored.saturating_add(size) > 256 * 1024 * 1024 {
                return Err("Квота защищённых файлов исчерпана".into());
            }
            let mut digest = Sha256::new();
            let file = uuid::Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO protected_files(id,name,content) VALUES(?1,?2,zeroblob(?3))",
                params![file, name, size],
            )
            .map_err(|_| "Не удалось создать файл")?;
            let row = tx.last_insert_rowid();
            {
                let mut blob = tx
                    .blob_open("main", "protected_files", "content", row, false)
                    .map_err(|_| "Не удалось открыть файл")?;
                for index in 0..next {
                    let bytes = Zeroizing::new(
                        tx.query_row(
                            "SELECT data FROM file_parts WHERE id=?1 AND part=?2",
                            params![id, index],
                            |r| r.get::<_, Vec<u8>>(0),
                        )
                        .map_err(|_| "Часть отсутствует")?,
                    );
                    digest.update(&*bytes);
                    blob.write_all(&bytes)
                        .map_err(|_| "Не удалось собрать файл")?;
                }
            }
            if hex(&digest.finalize()) != expected {
                return Err("Контрольная сумма файла не совпадает".into());
            }
            tx.execute("DELETE FROM file_parts WHERE id=?1", [&id])
                .map_err(|_| "Не удалось очистить части")?;
            tx.execute(
                "UPDATE file_in SET phase='complete',file=?2 WHERE id=?1",
                params![id, file],
            )
            .map_err(|_| "Не удалось завершить передачу")?;
        }
    }
    Ok(())
}
pub(crate) fn discard(c: &Connection, id: &str, peer: &str) -> Result<(), String> {
    schema(c)?;
    let tx = rusqlite::Transaction::new_unchecked(c, rusqlite::TransactionBehavior::Immediate)
        .map_err(|_| "Хранилище занято")?;
    tx.execute("DELETE FROM file_parts WHERE id=?1 AND EXISTS(SELECT 1 FROM file_in WHERE id=?1 AND peer=?2 AND phase='receiving')",params![id,peer]).map_err(|_|"Ошибка отмены")?;
    tx.execute(
        "UPDATE file_in SET phase='cancelled' WHERE id=?1 AND peer=?2 AND phase='receiving'",
        params![id, peer],
    )
    .map_err(|_| "Ошибка отмены")?;
    tx.commit().map_err(|_| "Ошибка отмены".into())
}
fn phase(c: &Connection, id: &str, value: &str) -> Result<(), String> {
    c.execute(
        "UPDATE file_out SET phase=?2 WHERE id=?1",
        params![id, value],
    )
    .map_err(|_| "Не удалось сохранить состояние")?;
    Ok(())
}
pub(crate) fn send(
    c: &Connection,
    id: &str,
    peer: &str,
    address: &str,
    sender: &str,
    key: &[u8],
    approved: &[u8],
    pause: Arc<AtomicBool>,
) -> Result<(), String> {
    let deliver = |p: Packet| -> Result<(), String> {
        match crate::network::secure_chat::send(
            sender,
            peer,
            address,
            key,
            Some(approved),
            &wire(&p)?,
        ) {
            crate::network::secure_chat::SecureSendStatus::Delivered { .. } => Ok(()),
            crate::network::secure_chat::SecureSendStatus::Failed { reason, .. } => Err(reason),
            _ => Err("Ключ участника изменился".into()),
        }
    };
    send_with(c, id, peer, pause, deliver)
}
fn send_with<F: Fn(Packet) -> Result<(), String>>(
    c: &Connection,
    id: &str,
    peer: &str,
    pause: Arc<AtomicBool>,
    deliver: F,
) -> Result<(), String> {
    schema(c)?;
    let (file, name, size, expected, mut next): (String, String, u64, String, u64) = c
        .query_row(
            "SELECT file,name,size,hash,next FROM file_out WHERE id=?1 AND peer=?2",
            params![id, peer],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .map_err(|_| "Передача не найдена")?;
    phase(c, id, "sending")?;
    let result = (|| {
        deliver(Packet::Offer {
            id: id.into(),
            name,
            size,
            hash: expected,
        })?;
        while next < size.div_ceil(PART as u64) {
            if pause.load(Ordering::Relaxed) {
                return Err("Приостановлено; можно продолжить".into());
            }
            let bytes = chunk(c, "protected_files", &file, next)?;
            deliver(Packet::Chunk {
                id: id.into(),
                index: next,
                hash: hash(&bytes),
                data: STANDARD.encode(&*bytes),
            })?;
            next += 1;
            c.execute("UPDATE file_out SET next=?2 WHERE id=?1", params![id, next])
                .map_err(|_| "Не удалось сохранить позицию")?;
        }
        if pause.load(Ordering::Relaxed) {
            return Err("Приостановлено; можно продолжить".into());
        }
        deliver(Packet::Finish { id: id.into() })?;
        phase(c, id, "complete")
    })();
    if result.is_err() {
        let _ = phase(c, id, "paused");
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    fn deliver(store: &crate::store::Store, peer: &str, p: Packet) -> Result<bool, String> {
        store.accept_secure_message(&uuid::Uuid::new_v4().to_string(), peer, &wire(&p)?)
    }
    fn offer(id: &str, data: &[u8]) -> Packet {
        Packet::Offer {
            id: id.into(),
            name: "file.bin".into(),
            size: data.len() as u64,
            hash: hash(data),
        }
    }
    fn part(id: &str, index: u64, data: &[u8]) -> Packet {
        Packet::Chunk {
            id: id.into(),
            index,
            hash: hash(data),
            data: STANDARD.encode(data),
        }
    }
    #[test]
    fn reopen_resume_duplicates_and_atomic_finalization() {
        let root = std::env::temp_dir().join(format!("cybos-parts-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("db");
        let id = uuid::Uuid::new_v4().to_string();
        let data = vec![42; PART + 17];
        {
            let store = crate::store::Store::test_store(path.to_str().unwrap());
            assert!(deliver(&store, "alice", offer(&id, &data)).unwrap());
            assert!(deliver(&store, "alice", part(&id, 0, &data[..PART])).unwrap());
        }
        {
            let store = crate::store::Store::test_store(path.to_str().unwrap());
            assert!(deliver(&store, "alice", offer(&id, &data)).unwrap());
            assert!(deliver(&store, "alice", part(&id, 0, &data[..PART])).unwrap());
            assert!(deliver(&store, "alice", part(&id, 0, &vec![43; PART])).is_err());
            assert!(deliver(&store, "bob", part(&id, 1, &data[PART..])).is_err());
            assert!(deliver(&store, "alice", Packet::Finish { id: id.clone() }).is_err());
            assert!(store.protected_files().unwrap().is_empty());
            assert!(deliver(&store, "alice", part(&id, 1, &data[PART..])).unwrap());
            assert!(deliver(&store, "alice", Packet::Finish { id: id.clone() }).unwrap());
            assert!(deliver(&store, "alice", Packet::Finish { id: id.clone() }).unwrap());
            assert!(deliver(&store, "alice", part(&id, 0, &data[..PART])).unwrap());
            assert!(deliver(&store, "alice", part(&id, 0, &vec![43; PART])).is_err());
            assert_eq!(store.protected_files().unwrap().len(), 1);
            assert!(store.chat_messages().is_empty());
            assert_eq!(store.file_transfers("alice").unwrap()[0].phase, "complete");
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn cancellation_quota_order_and_empty_file() {
        let store = crate::store::Store::test_store(":memory:");
        let id = uuid::Uuid::new_v4().to_string();
        let data = vec![8; PART + 1];
        deliver(&store, "alice", offer(&id, &data)).unwrap();
        assert!(deliver(&store, "alice", part(&id, 1, &data[PART..])).is_err());
        deliver(&store, "alice", part(&id, 0, &data[..PART])).unwrap();
        store.discard_transfer(&id, "alice").unwrap();
        assert!(deliver(&store, "alice", part(&id, 0, &data[..PART])).is_err());
        assert!(deliver(&store, "alice", Packet::Finish { id }).is_err());
        let empty = uuid::Uuid::new_v4().to_string();
        deliver(&store, "alice", offer(&empty, b"")).unwrap();
        deliver(&store, "alice", Packet::Finish { id: empty }).unwrap();
        assert_eq!(store.protected_files().unwrap()[0].bytes, 0);
        for _ in 0..8 {
            deliver(
                &store,
                "alice",
                offer(&uuid::Uuid::new_v4().to_string(), b"x"),
            )
            .unwrap();
        }
        assert!(
            deliver(
                &store,
                "alice",
                offer(&uuid::Uuid::new_v4().to_string(), b"x")
            )
            .is_err()
        );
    }
    #[test]
    fn lost_ack_pause_and_resume_keep_confirmed_offset() {
        let root = std::env::temp_dir().join(format!("cybos-outbox-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("out.db");
        let sender = crate::secure_store::open(&path, &[10; 32]).unwrap();
        let data = vec![77; PART * 2 + 9];
        crate::secure_files::save_bytes(&sender, "file.bin", &data).unwrap();
        let file = crate::secure_files::list(&sender).unwrap()[0].id.clone();
        let id = create(&sender, "bob", &file).unwrap();
        let receiver = crate::store::Store::test_store(":memory:");
        let failed = std::cell::Cell::new(false);
        assert!(
            send_with(
                &sender,
                &id,
                "bob",
                Arc::new(AtomicBool::new(false)),
                |packet| {
                    let lose =
                        matches!(&packet, Packet::Chunk { index: 0, .. }) && !failed.replace(true);
                    deliver(&receiver, "alice", packet)?;
                    if lose {
                        Err("ACK потерян".into())
                    } else {
                        Ok(())
                    }
                }
            )
            .is_err()
        );
        assert_eq!(statuses(&sender, "bob").unwrap()[0].done, 0);
        drop(sender);
        let sender = crate::secure_store::open(&path, &[10; 32]).unwrap();
        let pause = Arc::new(AtomicBool::new(false));
        let stop = pause.clone();
        assert!(
            send_with(&sender, &id, "bob", pause, |packet| {
                let part = matches!(&packet, Packet::Chunk { index: 0, .. });
                deliver(&receiver, "alice", packet)?;
                if part {
                    stop.store(true, Ordering::Relaxed)
                }
                Ok(())
            })
            .is_err()
        );
        assert_eq!(statuses(&sender, "bob").unwrap()[0].done, PART as u64);
        send_with(
            &sender,
            &id,
            "bob",
            Arc::new(AtomicBool::new(false)),
            |packet| {
                deliver(&receiver, "alice", packet)?;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(statuses(&sender, "bob").unwrap()[0].phase, "complete");
        assert_eq!(receiver.protected_files().unwrap().len(), 1);
        drop(sender);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn bad_final_hash_rolls_back_destination() {
        let receiver = crate::store::Store::test_store(":memory:");
        let id = uuid::Uuid::new_v4().to_string();
        let data = b"content";
        deliver(
            &receiver,
            "alice",
            Packet::Offer {
                id: id.clone(),
                name: "file.bin".into(),
                size: data.len() as u64,
                hash: hash(b"different"),
            },
        )
        .unwrap();
        deliver(&receiver, "alice", part(&id, 0, data)).unwrap();
        assert!(deliver(&receiver, "alice", Packet::Finish { id }).is_err());
        assert!(receiver.protected_files().unwrap().is_empty());
    }
    #[test]
    fn incremental_assembly_stays_in_encrypted_storage() {
        let root =
            std::env::temp_dir().join(format!("cybos-stream-cipher-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("files.db");
        let mut conn = crate::secure_store::open(&path, &[9; 32]).unwrap();
        schema(&conn).unwrap();
        crate::secure_files::schema(&conn).unwrap();
        let data = b"unique-transfer-secret-marker".repeat(1200);
        let id = uuid::Uuid::new_v4().to_string();
        {
            let tx = conn.transaction().unwrap();
            receive(&tx, "alice", &wire(&offer(&id, &data)).unwrap()).unwrap();
            tx.commit().unwrap();
        }
        for (index, bytes) in data.chunks(PART).enumerate() {
            let tx = conn.transaction().unwrap();
            receive(
                &tx,
                "alice",
                &wire(&part(&id, index as u64, bytes)).unwrap(),
            )
            .unwrap();
            tx.commit().unwrap();
        }
        {
            let tx = conn.transaction().unwrap();
            receive(
                &tx,
                "alice",
                &wire(&Packet::Finish { id: id.clone() }).unwrap(),
            )
            .unwrap();
            tx.commit().unwrap();
        }
        let actual: Vec<u8> = conn
            .query_row("SELECT content FROM protected_files", [], |r| r.get(0))
            .unwrap();
        assert_eq!(actual, data);
        drop(conn);
        let reopened = crate::secure_store::open(&path, &[9; 32]).unwrap();
        assert_eq!(statuses(&reopened, "alice").unwrap()[0].phase, "complete");
        drop(reopened);
        let marker = b"unique-transfer-secret-marker";
        assert!(
            !std::fs::read(&path)
                .unwrap()
                .windows(marker.len())
                .any(|w| w == marker)
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
