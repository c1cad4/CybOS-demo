use chrono::Local;
use rusqlite::{params, Connection};
use std::{fs, path::PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use uuid::Uuid;

use crate::crypto;
use crate::models::{Event, GraphLink, GraphNode, Memory};

pub(crate) struct Store {
    pub(crate) path: PathBuf,
    conn: Connection,
    chat_key: Option<[u8; 32]>,
}
impl Store {
    pub(crate) fn open() -> Self {
        let base = dirs_fallback();
        let _ = fs::create_dir_all(&base);

        #[cfg(unix)]
        if let Ok(metadata) = fs::metadata(&base) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o700);
            let _ = fs::set_permissions(&base, permissions);
        }

        let path = base.join("cybos.db");
        let conn = Connection::open(&path).expect("cannot open cybOS database");

        #[cfg(unix)]
        if let Ok(metadata) = fs::metadata(&path) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o600);
            let _ = fs::set_permissions(&path, permissions);
        }
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS events(
                id TEXT PRIMARY KEY,
                time TEXT,
                kind TEXT,
                text TEXT
            );

            CREATE TABLE IF NOT EXISTS kv(
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS memories(
                id TEXT PRIMARY KEY,
                time TEXT NOT NULL,
                text TEXT NOT NULL,
                source TEXT NOT NULL,
                importance REAL NOT NULL
            );

            CREATE TABLE IF NOT EXISTS graph_nodes(
                id TEXT PRIMARY KEY,
                label TEXT NOT NULL,
                kind TEXT NOT NULL,
                x REAL NOT NULL,
                y REAL NOT NULL
            );

            CREATE TABLE IF NOT EXISTS graph_links(
                from_id TEXT NOT NULL,
                to_id TEXT NOT NULL,
                relation TEXT NOT NULL,
                PRIMARY KEY(from_id, to_id, relation)
            );

            CREATE INDEX IF NOT EXISTS idx_memories_time
                ON memories(time);

            CREATE INDEX IF NOT EXISTS idx_memories_text
                ON memories(text);

            CREATE TABLE IF NOT EXISTS chat_messages(
                id TEXT PRIMARY KEY,
                time TEXT NOT NULL,
                who TEXT NOT NULL,
                text TEXT NOT NULL,
                mine INTEGER NOT NULL,
                encrypted INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_chat_messages_time
                ON chat_messages(time);
            "#,
        )
        .expect("cannot initialize database");

        // Migrate legacy plaintext databases once. Avoid issuing ALTER TABLE
        // on every Store::open(), because the test suite and multiple app
        // components can legitimately open the same SQLite database in parallel.
        let has_encrypted_column = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('chat_messages') WHERE name='encrypted'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;

        if !has_encrypted_column {
            let _ = conn.execute(
                "ALTER TABLE chat_messages ADD COLUMN encrypted INTEGER NOT NULL DEFAULT 0",
                [],
            );
        }
        Self {
            path,
            conn,
            chat_key: None,
        }
    }

    pub(crate) fn configure_chat_key(&mut self, key: [u8; 32]) {
        self.chat_key = Some(key);
    }

    fn chat_aad(id: &str, time: &str, who: &str, mine: bool) -> Vec<u8> {
        [
            b"cybOS/chat-storage/v1".as_slice(),
            id.as_bytes(),
            time.as_bytes(),
            who.as_bytes(),
            if mine { b"1".as_slice() } else { b"0".as_slice() },
        ]
        .concat()
    }

    fn encrypt_chat_text(
        &self,
        id: &str,
        time: &str,
        who: &str,
        mine: bool,
        text: &str,
    ) -> Option<String> {
        let key = self.chat_key.as_ref()?;
        let aad = Self::chat_aad(id, time, who, mine);
        let (nonce, ciphertext) = crypto::encrypt(key, &aad, text.as_bytes()).ok()?;
        Some(format!("v1:{nonce}:{ciphertext}"))
    }

    fn decrypt_chat_text(
        &self,
        id: &str,
        time: &str,
        who: &str,
        mine: bool,
        encrypted: bool,
        stored: &str,
    ) -> Option<String> {
        if !encrypted {
            return Some(stored.to_string());
        }

        let key = self.chat_key.as_ref()?;
        let mut parts = stored.splitn(2, ':');
        let nonce = parts.next()?;
        let ciphertext = parts.next()?;
        let aad = Self::chat_aad(id, time, who, mine);
        let plaintext = crypto::decrypt(key, &aad, nonce, ciphertext).ok()?;
        String::from_utf8(plaintext).ok()
    }

    pub(crate) fn get(&self, key: &str) -> Option<String> {
        self.conn
            .query_row("SELECT value FROM kv WHERE key=?1", [key], |r| r.get(0))
            .ok()
    }
    pub(crate) fn set(&self, key: &str, value: &str) {
        let _ = self.conn.execute("INSERT INTO kv(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key,value]);
    }
    pub(crate) fn delete(&self, key: &str) {
        let _ = self.conn.execute("DELETE FROM kv WHERE key=?1", [key]);
    }
    pub(crate) fn add_memory(&self, memory: &Memory) {
        let _ = self.conn.execute(
            "INSERT OR IGNORE INTO memories(id,time,text,source,importance)
             VALUES(?1,?2,?3,?4,?5)",
            params![
                memory.id,
                memory.time,
                memory.text,
                memory.source,
                memory.importance
            ],
        );
    }

    pub(crate) fn memories(&self) -> Vec<Memory> {
        let mut st = self
            .conn
            .prepare(
                "SELECT id,time,text,source,importance
                 FROM memories
                 ORDER BY rowid DESC
                 LIMIT 200",
            )
            .unwrap();

        st.query_map([], |r| {
            Ok(Memory {
                id: r.get(0)?,
                time: r.get(1)?,
                text: r.get(2)?,
                source: r.get(3)?,
                importance: r.get(4)?,
            })
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect()
    }

    pub(crate) fn search_memories(&self, query: &str) -> Vec<Memory> {
        let q = query.trim().to_lowercase();

        if q.is_empty() {
            return Vec::new();
        }

        let words: Vec<String> = q
            .split_whitespace()
            .map(|w| {
                w.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                    .to_string()
            })
            .filter(|w| w.len() >= 2)
            .collect();

        if words.is_empty() {
            return Vec::new();
        }

        self.memories()
            .into_iter()
            .filter(|m| {
                let text = m.text.to_lowercase();
                words.iter().any(|word| text.contains(word))
            })
            .take(8)
            .collect()
    }

    pub(crate) fn graph_nodes(&self) -> Vec<GraphNode> {
        let mut st = self
            .conn
            .prepare(
                "SELECT id,label,kind,x,y
                 FROM graph_nodes
                 ORDER BY rowid ASC",
            )
            .unwrap();

        st.query_map([], |r| {
            Ok(GraphNode {
                id: r.get(0)?,
                label: r.get(1)?,
                kind: r.get(2)?,
                x: r.get(3)?,
                y: r.get(4)?,
            })
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect()
    }

    pub(crate) fn graph_links(&self) -> Vec<GraphLink> {
        let mut st = self
            .conn
            .prepare(
                "SELECT from_id,to_id,relation
                 FROM graph_links
                 ORDER BY rowid ASC",
            )
            .unwrap();

        st.query_map([], |r| {
            Ok(GraphLink {
                from: r.get(0)?,
                to: r.get(1)?,
                relation: r.get(2)?,
            })
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect()
    }

    pub(crate) fn save_graph_node(&self, node: &GraphNode) {
        let _ = self.conn.execute(
            "INSERT INTO graph_nodes(id,label,kind,x,y)
             VALUES(?1,?2,?3,?4,?5)
             ON CONFLICT(id) DO UPDATE SET
                 label=excluded.label,
                 kind=excluded.kind,
                 x=excluded.x,
                 y=excluded.y",
            params![node.id, node.label, node.kind, node.x, node.y],
        );
    }

    pub(crate) fn save_graph_link(&self, link: &GraphLink) {
        let _ = self.conn.execute(
            "INSERT OR IGNORE INTO graph_links(from_id,to_id,relation)
             VALUES(?1,?2,?3)",
            params![link.from, link.to, link.relation],
        );
    }

    pub(crate) fn chat_messages(&self) -> Vec<(String, String, bool)> {
        let mut st = self
            .conn
            .prepare(
                "SELECT id,time,who,text,mine,encrypted
                 FROM chat_messages
                 ORDER BY rowid DESC
                 LIMIT 500",
            )
            .unwrap();

        let rows: Vec<(String, String, String, String, bool, bool)> = st
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get::<_, i64>(4)? != 0,
                    r.get::<_, i64>(5)? != 0,
                ))
            })
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        let mut rows = rows;
        rows.reverse();

        rows
            .into_iter()
            .map(|(id, time, who, stored_text, mine, encrypted)| {
                let plain =
                    match self.decrypt_chat_text(&id, &time, &who, mine, encrypted, &stored_text) {
                        Some(text) if encrypted => text,
                        Some(text) => {
                            if let Some(encrypted_text) =
                                self.encrypt_chat_text(&id, &time, &who, mine, &text)
                            {
                                let _ = self.conn.execute(
                                    "UPDATE chat_messages SET text=?1, encrypted=1 WHERE id=?2",
                                    params![encrypted_text, id],
                                );
                            }
                            text
                        }
                        None => "[encrypted message unavailable]".to_string(),
                    };

                (who, plain, mine)
            })
            .collect()
    }

    pub(crate) fn add_chat_message(&self, who: &str, text: &str, mine: bool) {
        let id = Uuid::new_v4().to_string();
        let time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let (stored_text, encrypted) = match self.chat_key {
            Some(_) => match self.encrypt_chat_text(&id, &time, who, mine, text) {
                Some(encrypted_text) => (encrypted_text, 1),
                None => return,
            },
            None => (text.to_string(), 0),
        };

        let _ = self.conn.execute(
            "INSERT INTO chat_messages(id,time,who,text,mine,encrypted)
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![id, time, who, stored_text, if mine { 1 } else { 0 }, encrypted],
        );
    }

    pub(crate) fn peer_pin(&self, node_id: &str) -> Option<String> {
        self.get(&format!("peer_pin:{node_id}"))
    }

    pub(crate) fn trust_peer_key(&self, node_id: &str, public_key_b64: &str) -> bool {
        let key = format!("peer_pin:{node_id}");
        match self.get(&key) {
            Some(existing) => existing == public_key_b64,
            None => {
                self.set(&key, public_key_b64);
                true
            }
        }
    }

    pub(crate) fn events(&self) -> Vec<Event> {
        let mut st = self
            .conn
            .prepare("SELECT id,time,kind,text FROM events ORDER BY rowid DESC LIMIT 100")
            .unwrap();
        st.query_map([], |r| {
            Ok(Event {
                id: r.get(0)?,
                time: r.get(1)?,
                kind: r.get(2)?,
                text: r.get(3)?,
            })
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect()
    }
    pub(crate) fn add_event(&self, kind: &str, text: &str) {
        let _ = self.conn.execute(
            "INSERT INTO events VALUES(?1,?2,?3,?4)",
            params![
                Uuid::new_v4().to_string(),
                Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                kind,
                text
            ],
        );
    }
}
fn dirs_fallback() -> PathBuf {
    if let Ok(p) = std::env::var("HOME") {
        PathBuf::from(p).join("Library/Application Support/cybOS")
    } else {
        PathBuf::from(".cybOS")
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypted_chat_roundtrip_hides_plaintext_at_rest() {
        let mut store = Store::open();
        store.configure_chat_key([7u8; 32]);
        let who = format!("TEST-ENCRYPT-{}", Uuid::new_v4());
        let plaintext = "secret chat storage payload";

        store.add_chat_message(&who, plaintext, true);

        let stored: String = store
            .conn
            .query_row(
                "SELECT text FROM chat_messages WHERE who=?1 ORDER BY rowid DESC LIMIT 1",
                [&who],
                |r| r.get(0),
            )
            .unwrap();

        assert!(!stored.contains(plaintext));

        let encrypted: i64 = store
            .conn
            .query_row(
                "SELECT encrypted FROM chat_messages WHERE who=?1 ORDER BY rowid DESC LIMIT 1",
                [&who],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(encrypted, 1);

        let messages = store.chat_messages();
        assert!(messages
            .iter()
            .any(|(entry_who, text, mine)| entry_who == &who && text == plaintext && *mine));
    }

    #[test]
    fn legacy_plaintext_chat_row_is_migrated_on_read() {
        let mut store = Store::open();
        store.configure_chat_key([8u8; 32]);
        let who = format!("TEST-MIGRATE-{}", Uuid::new_v4());
        let id = Uuid::new_v4().to_string();
        let time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let plaintext = "legacy plaintext message";

        store
            .conn
            .execute(
                "INSERT INTO chat_messages(id,time,who,text,mine,encrypted)
                 VALUES(?1,?2,?3,?4,1,0)",
                params![id, time, who, plaintext],
            )
            .unwrap();

        let messages = store.chat_messages();
        assert!(messages
            .iter()
            .any(|(entry_who, text, mine)| entry_who == &who && text == plaintext && *mine));

        let (stored, encrypted): (String, i64) = store
            .conn
            .query_row(
                "SELECT text,encrypted FROM chat_messages WHERE id=?1",
                [&id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(encrypted, 1);
        assert!(!stored.contains(plaintext));
    }

    #[test]
    fn tampered_encrypted_chat_row_is_not_returned_as_plaintext() {
        let mut store = Store::open();
        store.configure_chat_key([9u8; 32]);
        let who = format!("TEST-TAMPER-{}", Uuid::new_v4());
        let plaintext = "tamper detection payload";

        store.add_chat_message(&who, plaintext, true);

        let id: String = store
            .conn
            .query_row(
                "SELECT id FROM chat_messages WHERE who=?1 ORDER BY rowid DESC LIMIT 1",
                [&who],
                |row| row.get(0),
            )
            .unwrap();

        store
            .conn
            .execute(
                "UPDATE chat_messages
                 SET text='AAAA:AAAA',encrypted=1
                 WHERE id=?1",
                [&id],
            )
            .unwrap();

        let messages = store.chat_messages();
        assert!(messages
            .iter()
            .any(|(entry_who, text, mine)| {
                entry_who == &who
                    && text == "[encrypted message unavailable]"
                    && *mine
            }));
        assert!(!messages
            .iter()
            .any(|(_, text, _)| text == plaintext));
    }

    #[test]
    fn tofu_rejects_peer_key_replacement() {
        let store = Store::open();
        let node_id = format!("test-peer-{}", Uuid::new_v4());
        let first = "ZmFrZS1rZXktMQ==";
        let replacement = "ZmFrZS1rZXktMg==";

        assert!(store.trust_peer_key(&node_id, first));
        assert_eq!(store.peer_pin(&node_id).as_deref(), Some(first));
        assert!(store.trust_peer_key(&node_id, first));
        assert!(!store.trust_peer_key(&node_id, replacement));
        assert_eq!(store.peer_pin(&node_id).as_deref(), Some(first));
    }
}
