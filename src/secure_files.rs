//! Small managed files inside authenticated SQLCipher storage; no custom cipher format.
use rusqlite::{Connection, params};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};
use zeroize::Zeroizing;
pub(crate) const MAX_FILE: u64 = 32 * 1024 * 1024;
const MAX_TOTAL: u64 = 256 * 1024 * 1024;
#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub id: String,
    pub name: String,
    pub bytes: u64,
}
pub(crate) fn schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS protected_files(id TEXT PRIMARY KEY,name TEXT NOT NULL,content BLOB NOT NULL);")
        .map_err(|_| "Хранилище файлов недоступно".into())
}
pub(crate) fn list(conn: &Connection) -> Result<Vec<Entry>, String> {
    schema(conn)?;
    let mut statement = conn
        .prepare("SELECT id,name,length(content) FROM protected_files ORDER BY rowid DESC")
        .map_err(|_| "Не удалось прочитать список")?;
    statement
        .query_map([], |r| {
            Ok(Entry {
                id: r.get(0)?,
                name: r.get(1)?,
                bytes: r.get(2)?,
            })
        })
        .map_err(|_| "Не удалось прочитать список")?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Список повреждён".into())
}
pub(crate) fn import(conn: &mut Connection, source: &Path) -> Result<(), String> {
    let file = fs::File::open(source).map_err(|_| "Не удалось открыть исходный файл")?;
    let metadata = file.metadata().map_err(|_| "Не удалось проверить файл")?;
    if !metadata.is_file() || metadata.len() > MAX_FILE {
        return Err("Выберите обычный файл размером до 32 МБ".into());
    }
    let name = source
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty() && s.len() <= 1024)
        .ok_or("Имя файла недоступно")?;
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(MAX_FILE + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Не удалось прочитать файл")?;
    if bytes.len() as u64 > MAX_FILE {
        return Err("Файл вырос; предел — 32 МБ".into());
    }
    save_bytes(conn, name, &bytes)
}
pub(crate) fn save_bytes(conn: &Connection, name: &str, bytes: &[u8]) -> Result<(), String> {
    if name.is_empty() || name.len() > 1024 || bytes.len() as u64 > MAX_FILE {
        return Err("Недопустимый файл".into());
    }
    schema(conn)?;
    let tx = rusqlite::Transaction::new_unchecked(conn, rusqlite::TransactionBehavior::Immediate)
        .map_err(|_| "Хранилище занято")?;
    let used: u64 = tx
        .query_row(
            "SELECT coalesce(sum(length(content)),0) FROM protected_files",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "Не удалось проверить объём")?;
    let count: u64 = tx
        .query_row("SELECT count(*) FROM protected_files", [], |r| r.get(0))
        .map_err(|_| "Не удалось проверить список")?;
    let transfers_exist: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='file_in')",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "Ошибка квоты передач")?;
    let reserved: u64 = if transfers_exist {
        tx.query_row(
            "SELECT coalesce(sum(size),0) FROM file_in WHERE phase='receiving'",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "Ошибка квоты передач")?
    } else {
        0
    };
    let reserved_files: u64 = if transfers_exist {
        tx.query_row(
            "SELECT count(*) FROM file_in WHERE phase='receiving'",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "Ошибка квоты передач")?
    } else {
        0
    };
    if used
        .saturating_add(reserved)
        .saturating_add(bytes.len() as u64)
        > MAX_TOTAL
        || count.saturating_add(reserved_files) >= 1024
    {
        return Err("Лимит хранилища: 256 МБ и 1024 файла".into());
    }
    tx.execute(
        "INSERT INTO protected_files(id,name,content) VALUES(?1,?2,?3)",
        params![uuid::Uuid::new_v4().to_string(), name, bytes],
    )
    .map_err(|_| "Не удалось сохранить файл")?;
    tx.commit()
        .map_err(|_| "Не удалось завершить сохранение".into())
}
pub(crate) fn outgoing(conn: &Connection, id: &str) -> Result<String, String> {
    schema(conn)?;
    let (name, bytes) = conn
        .query_row(
            "SELECT name,content FROM protected_files WHERE id=?1 AND length(content)<=?2",
            params![id, crate::chat_attachment::MAX_BYTES],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?)),
        )
        .map_err(|_| "Выберите защищённый файл до 8 КиБ")?;
    let bytes = Zeroizing::new(bytes);
    crate::chat_attachment::encode(&name, &bytes)
}
pub(crate) fn export(conn: &Connection, id: &str, destination: &Path) -> Result<(), String> {
    schema(conn)?;
    // Caller supplies the complete destination; stored names are never used as paths.
    if !destination.is_absolute() || destination.file_name().is_none() {
        return Err("Укажите полный путь нового файла".into());
    }
    let bytes = Zeroizing::new(
        conn.query_row(
            "SELECT content FROM protected_files WHERE id=?1 AND length(content)<=?2",
            params![id, MAX_FILE],
            |r| r.get::<_, Vec<u8>>(0),
        )
        .map_err(|_| "Файл недоступен")?,
    );
    let parent = destination.parent().ok_or("Папка назначения недоступна")?;
    let temp = parent.join(format!(".cybos-export-{}.tmp", uuid::Uuid::new_v4()));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temp)
        .map_err(|_| "Не удалось создать файл в папке назначения")?;
    let result = (|| {
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "Не удалось записать копию")?;
        drop(file);
        fs::hard_link(&temp, destination).map_err(
            |_| "Файл назначения уже существует или файловая система не поддерживает экспорт",
        )?;
        #[cfg(unix)]
        {
            let _ = fs::File::open(parent).and_then(|f| f.sync_all());
        }
        Ok(())
    })();
    let _ = fs::remove_file(&temp);
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encrypted_roundtrip_preserves_source_and_refuses_overwrite() {
        let root = std::env::temp_dir().join(format!("cybos-files-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let source = root.join("личный.txt");
        let content = b"unique-secret-attachment-marker";
        fs::write(&source, content).unwrap();
        let database = root.join("files.db");
        let mut conn = crate::secure_store::open(&database, &[7; 32]).unwrap();
        import(&mut conn, &source).unwrap();
        let entries = list(&conn).unwrap();
        assert_eq!(entries[0].name, "личный.txt");
        let output = root.join("copy.txt");
        export(&conn, &entries[0].id, &output).unwrap();
        assert_eq!(fs::read(&output).unwrap(), content);
        fs::write(&output, b"keep-existing").unwrap();
        assert!(export(&conn, &entries[0].id, &output).is_err());
        assert_eq!(fs::read(&output).unwrap(), b"keep-existing");
        assert_eq!(fs::read(&source).unwrap(), content);
        assert!(export(&conn, "missing", &root.join("missing.txt")).is_err());
        assert!(!root.join("missing.txt").exists());
        let large = root.join("large");
        fs::File::create(&large)
            .unwrap()
            .set_len(MAX_FILE + 1)
            .unwrap();
        assert!(import(&mut conn, &large).is_err());
        assert_eq!(list(&conn).unwrap().len(), 1);
        drop(conn);
        assert!(crate::secure_store::open(&database, &[8; 32]).is_err());
        assert!(
            !fs::read(&database)
                .unwrap()
                .windows(content.len())
                .any(|s| s == content)
        );
        fs::remove_dir_all(root).unwrap();
    }
}
