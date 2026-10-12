use chrono::Local;
use rusqlite::{params, Connection, OptionalExtension};
use std::{fs, path::PathBuf, time::Duration};
use uuid::Uuid;

use crate::models::{Event, GraphLink, GraphNode, Memory};

pub(crate) struct Store {
    pub(crate) path: PathBuf,
    conn: Connection,
}
impl Store {
    pub(crate) fn file_transfers(&self,peer:&str)->Result<Vec<crate::file_transfer::Status>,String>{crate::file_transfer::statuses(&self.conn,peer)}
    pub(crate) fn discard_transfer(&self,id:&str,peer:&str)->Result<(),String>{crate::file_transfer::discard(&self.conn,id,peer)}

    pub(crate) fn protected_files(&self) -> Result<Vec<crate::secure_files::Entry>,String> { crate::secure_files::list(&self.conn) }
    pub(crate) fn attachment_message(&self, id: &str) -> Result<String,String> { crate::secure_files::outgoing(&self.conn,id) }
    pub(crate) fn save_chat_attachment(&self, message: &str) -> Result<(),String> {
        let file=crate::chat_attachment::decode(message)?;
        crate::secure_files::save_bytes(&self.conn,&file.name,&file.content)
    }

    pub(crate) fn open() -> Self {
        let base = dirs_fallback();
        let _ = fs::create_dir_all(&base);
        let path = base.join("cybos-encrypted.db");
        let conn = crate::secure_store::open_session(&base).expect("authenticated encrypted database required");
        let _ = conn.busy_timeout(Duration::from_secs(3));
        let _ = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;");
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
                mine INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_chat_messages_time
                ON chat_messages(time);

            CREATE TABLE IF NOT EXISTS secure_message_ids(
                message_id TEXT PRIMARY KEY,
                time TEXT NOT NULL,
                sender TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_secure_message_ids_time
                ON secure_message_ids(time);
            "#,
        )
        .expect("cannot initialize database");
        Self { path, conn }
    }
    #[cfg(test)]
    pub(crate) fn test_store(path: &str) -> Self {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch("CREATE TABLE IF NOT EXISTS kv(key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS secure_message_ids(message_id TEXT PRIMARY KEY, time TEXT NOT NULL, sender TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS chat_messages(id TEXT PRIMARY KEY,time TEXT NOT NULL,who TEXT NOT NULL,text TEXT NOT NULL,mine INTEGER NOT NULL);").unwrap();
        Self { path: PathBuf::from(path), conn }
    }

    #[cfg(test)]
    pub(crate) fn test_execute(&self, sql: &str) { self.conn.execute_batch(sql).unwrap(); }

    pub(crate) fn get(&self, key: &str) -> Option<String> {
        self.conn
            .query_row("SELECT value FROM kv WHERE key=?1", [key], |r| r.get(0))
            .ok()
    }
    pub(crate) fn set(&self, key: &str, value: &str) {
        let _ = self.conn.execute("INSERT INTO kv(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key,value]);
    }
    pub(crate) fn get_checked(&self, key: &str) -> Result<Option<String>, String> {
        self.conn.query_row("SELECT value FROM kv WHERE key=?1", [key], |r| r.get(0))
            .optional().map_err(|e| format!("read identity: {e}"))
    }

    pub(crate) fn set_checked(&self, key: &str, value: &str) -> Result<(), String> {
        self.conn.execute("INSERT INTO kv(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key, value])
            .map(|_| ()).map_err(|e| format!("persist identity: {e}"))
    }

    pub(crate) fn persist_node_identity(&self, generated: &str) -> Result<String, String> {
        self.conn.execute("INSERT OR IGNORE INTO kv(key,value) VALUES('node_id',?1)", [generated])
            .map_err(|e| format!("persist node identity: {e}"))?;
        self.get_checked("node_id")?.ok_or_else(|| "persisted node identity missing".into())
    }

    pub(crate) fn persist_noise_identity(&self, generated: &str) -> Result<String, String> {
        // Two starting processes must converge on the existing key, never replace it.
        self.conn.execute("INSERT OR IGNORE INTO kv(key,value) VALUES('noise_static_private_hex',?1)", [generated])
            .map_err(|e| format!("persist Noise identity: {e}"))?;
        self.get_checked("noise_static_private_hex")?
            .ok_or_else(|| "persisted Noise identity missing".into())
    }

    pub(crate) fn approve_secure_peer(&self, node_id: &str, key: &[u8]) -> Result<(), String> {
        crate::network::peer_trust::validate_peer(node_id, key)?;
        let encoded = key.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let pin_name = format!("noise_peer_key:{node_id}");
        let tx = self.conn.unchecked_transaction().map_err(|e| e.to_string())?;
        tx.execute("INSERT OR IGNORE INTO kv(key,value) VALUES(?1,?2)", params![pin_name, encoded]).map_err(|e| e.to_string())?;
        let existing: String = tx.query_row("SELECT value FROM kv WHERE key=?1", [&pin_name], |r| r.get(0)).map_err(|e| e.to_string())?;
        if crate::network::secure_chat::decode_key(&existing)? != key {
            return Err("Noise identity key changed; existing pin preserved".into());
        }
        tx.execute("INSERT INTO kv(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![format!("noise_peer_approved:{node_id}"), encoded]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| format!("persist peer approval: {e}"))
    }

    /// Commit replay protection and chat history together before delivery ACK.
    pub(crate) fn accept_secure_message(&self, message_id: &str, sender: &str, text: &str) -> Result<bool, String> {
        if text.starts_with(crate::chat_attachment::PREFIX) { crate::chat_attachment::decode(text)?; }
        immunocybchain::validate_message_identity(message_id, sender)
            .map_err(|e| format!("invalid message identity: {e:?}"))?;
        if text.starts_with(crate::file_transfer::PREFIX) {crate::file_transfer::schema(&self.conn)?;crate::secure_files::schema(&self.conn)?;}
        let tx = rusqlite::Transaction::new_unchecked(&self.conn,rusqlite::TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
        if text.starts_with(crate::file_transfer::PREFIX) {crate::file_transfer::receive(&tx,sender,text)?;}
        let time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let inserted = tx.execute("INSERT OR IGNORE INTO secure_message_ids(message_id,time,sender) VALUES(?1,?2,?3)", params![message_id, time, sender])
            .map_err(|e| format!("persist replay identity: {e}"))?;
        if inserted == 0 { return Ok(false); }
        if !text.starts_with(crate::file_transfer::PREFIX) {
            tx.execute("INSERT INTO chat_messages(id,time,who,text,mine) VALUES(?1,?2,?3,?4,0)", params![Uuid::new_v4().to_string(), time, format!("CYB:{sender}"), text]).map_err(|e| format!("persist secure message: {e}"))?;
        }
        tx.execute("DELETE FROM secure_message_ids WHERE message_id NOT IN (SELECT message_id FROM secure_message_ids ORDER BY rowid DESC LIMIT 1024)", [])
            .map_err(|e| format!("prune replay identities: {e}"))?;
        tx.commit().map_err(|e| format!("commit secure message: {e}"))?;
        Ok(true)
    }

    /// Persist a user statement and its graph node together. A statement is not a verified fact.
    pub(crate) fn capture_user_message(&self, message: &str) -> Result<bool, String> {
        use sha2::{Digest, Sha256};
        let text: String = message.trim().chars().take(8192).collect();
        if text.is_empty() { return Ok(false); }
        let id = format!("user-message:{:x}", Sha256::digest(text.as_bytes()));
        let tx = self.conn.unchecked_transaction().map_err(|e|e.to_string())?;
        let time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let inserted = tx.execute("INSERT OR IGNORE INTO memories(id,time,text,source,importance) VALUES(?1,?2,?3,'USER_MESSAGE',0.5)",params![id,time,text]).map_err(|e|e.to_string())?;
        tx.execute("INSERT OR IGNORE INTO graph_nodes(id,label,kind,x,y) VALUES('user-knowledge','Мои сообщения и заметки','USER',0,0)",[]).map_err(|e|e.to_string())?;
        let index: i64 = tx.query_row("SELECT COUNT(*) FROM graph_nodes", [], |r|r.get(0)).map_err(|e|e.to_string())?;
        let angle = index as f64 * 2.399963;
        let radius = 120.0 + (index as f64).sqrt() * 30.0;
        let label: String = text.chars().take(48).collect();
        tx.execute("INSERT OR IGNORE INTO graph_nodes(id,label,kind,x,y) VALUES(?1,?2,'USER_MESSAGE',?3,?4)",params![id,label,angle.cos()*radius,angle.sin()*radius]).map_err(|e|e.to_string())?;
        tx.execute("INSERT OR IGNORE INTO graph_links(from_id,to_id,relation) VALUES('user-knowledge',?1,'написал')",[id]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e|e.to_string())?;
        Ok(inserted != 0)
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

        // Indexed retrieval covers old memories too; UI history remains capped at 200.
        // Treat user text as quoted terms, never as an FTS query language or SQL.
        if self.ensure_memory_search().is_ok() {
            let expression = words.iter().take(12).map(|word| format!("\"{}\"*", word.replace('"', "\"\""))).collect::<Vec<_>>().join(" OR ");
            if let Ok(mut statement) = self.conn.prepare("SELECT m.id,m.time,m.text,m.source,m.importance FROM memory_search s JOIN memories m ON m.rowid=s.rowid WHERE memory_search MATCH ?1 ORDER BY bm25(memory_search),m.rowid DESC LIMIT 8") {
                if let Ok(rows) = statement.query_map([expression], |r| Ok(Memory {id:r.get(0)?,time:r.get(1)?,text:r.get(2)?,source:r.get(3)?,importance:r.get(4)?})) {
                    return rows.filter_map(Result::ok).collect();
                }
            }
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

    fn ensure_memory_search(&self) -> Result<(), String> {
        if self.get_checked("memory_search_v1")?.is_some() { return Ok(()); }
        let tx=self.conn.unchecked_transaction().map_err(|e|e.to_string())?;
        tx.execute_batch("CREATE VIRTUAL TABLE IF NOT EXISTS memory_search USING fts5(text,content='memories',content_rowid='rowid',tokenize='unicode61');
            CREATE TRIGGER IF NOT EXISTS memory_search_insert AFTER INSERT ON memories BEGIN INSERT INTO memory_search(rowid,text) VALUES(new.rowid,new.text); END;
            CREATE TRIGGER IF NOT EXISTS memory_search_delete AFTER DELETE ON memories BEGIN INSERT INTO memory_search(memory_search,rowid,text) VALUES('delete',old.rowid,old.text); END;
            CREATE TRIGGER IF NOT EXISTS memory_search_update AFTER UPDATE ON memories BEGIN INSERT INTO memory_search(memory_search,rowid,text) VALUES('delete',old.rowid,old.text); INSERT INTO memory_search(rowid,text) VALUES(new.rowid,new.text); END;
            INSERT INTO memory_search(memory_search) VALUES('rebuild');
            INSERT OR REPLACE INTO kv(key,value) VALUES('memory_search_v1','1');").map_err(|e|e.to_string())?;
        tx.commit().map_err(|e|e.to_string())
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
                "SELECT who,text,mine
                 FROM chat_messages
                 ORDER BY rowid DESC
                 LIMIT 500",
            )
            .unwrap();

        let mut messages: Vec<_> = st.query_map([], |r| {
            let who: String = r.get(0)?;
            let text: String = r.get(1)?;
            let mine: i64 = r.get(2)?;
            Ok((who, text, mine != 0))
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect();
        messages.reverse();
        messages
    }

    pub(crate) fn add_chat_message(&self, who: &str, text: &str, mine: bool) {
        let _ = self.conn.execute(
            "INSERT INTO chat_messages(id,time,who,text,mine)
             VALUES(?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                who,
                text,
                if mine { 1 } else { 0 }
            ],
        );
    }

    /// Claims a secure message id exactly once, retaining the latest 1024 ids.
    pub(crate) fn claim_secure_message_id(&self, message_id: &str, sender: &str) -> bool {
        if immunocybchain::validate_message_identity(message_id, sender).is_err() { return false; }
        let inserted = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO secure_message_ids(message_id,time,sender)
                 VALUES(?1,?2,?3)",
                params![
                    message_id,
                    Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    sender
                ],
            )
            .map(|count| count == 1)
            .unwrap_or(false);

        if inserted {
            let _ = self.conn.execute(
                "DELETE FROM secure_message_ids
                 WHERE message_id NOT IN (
                     SELECT message_id
                     FROM secure_message_ids
                     ORDER BY rowid DESC
                     LIMIT 1024
                 )",
                [],
            );
        }

        inserted
    }

    pub(crate) fn database_integrity(&self) -> String {
        self.conn
            .query_row("PRAGMA integrity_check(1)", [], |row| row.get::<_, String>(0))
            .unwrap_or_else(|error| format!("ERROR: {error}"))
    }

    pub(crate) fn exportable_chat(&self) -> Vec<(String, String, bool)> {
        self.chat_messages()
    }

    pub(crate) fn exportable_memories(&self) -> Vec<Memory> {
        self.memories()
    }

    pub(crate) fn exportable_events(&self) -> Vec<Event> {
        self.events()
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
pub(crate) fn dirs_fallback() -> PathBuf {
    if let Some(path) = std::env::var_os("CYBOS_DATA_DIR").filter(|path| !path.is_empty()) {
        return PathBuf::from(path);
    }
    if let Ok(p) = std::env::var("HOME") {
        PathBuf::from(p).join("Library/Application Support/cybOS")
    } else {
        PathBuf::from(".cybOS")
    }
}



#[cfg(test)]
mod tests {
    use super::Store;
    use rusqlite::Connection;

    #[test]
    fn secure_message_id_claim_is_idempotent() {
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        conn.execute_batch(
            "CREATE TABLE secure_message_ids(
                message_id TEXT PRIMARY KEY,
                time TEXT NOT NULL,
                sender TEXT NOT NULL
            );",
        )
        .expect("schema");

        let store = Store {
            path: std::path::PathBuf::from(":memory:"),
            conn,
        };

        assert!(store.claim_secure_message_id("message-1", "node-a"));
        assert!(!store.claim_secure_message_id("message-1", "node-a"));
        assert!(store.claim_secure_message_id("message-2", "node-a"));
    }

    #[test]
    fn competing_identity_writes_preserve_the_first_key() {
        let store = Store::test_store(":memory:");
        let first = "07".repeat(32);
        let second = "08".repeat(32);
        assert_eq!(store.persist_noise_identity(&first).unwrap(), first);
        assert_eq!(store.persist_noise_identity(&second).unwrap(), first);
    }

    #[test]
    fn secure_history_and_replay_claim_roll_back_together() {
        let store = Store::test_store(":memory:");
        store.test_execute("CREATE TRIGGER deny_chat BEFORE INSERT ON chat_messages BEGIN SELECT RAISE(ABORT, 'disk full fixture'); END;");
        assert!(store.accept_secure_message("message-1", "node-a", "body").is_err());
        assert_eq!(store.conn.query_row("SELECT COUNT(*) FROM secure_message_ids", [], |r| r.get::<_, i64>(0)).unwrap(), 0);
        store.test_execute("DROP TRIGGER deny_chat;");
        assert!(store.accept_secure_message("message-1", "node-a", "body").unwrap());
        assert!(!store.accept_secure_message("message-1", "node-a", "body").unwrap());
        assert_eq!(store.chat_messages().len(), 1);
    }

    #[test]
    fn replay_pruning_failure_also_rolls_back_chat_history() {
        let store = Store::test_store(":memory:");
        for i in 0..1024 { store.accept_secure_message(&format!("id-{i}"), "node-a", "body").unwrap(); }
        store.test_execute("CREATE TRIGGER deny_prune BEFORE DELETE ON secure_message_ids BEGIN SELECT RAISE(ABORT, 'prune failed'); END;");
        assert!(store.accept_secure_message("id-new", "node-a", "body").is_err());
        assert_eq!(store.conn.query_row("SELECT COUNT(*) FROM chat_messages", [], |r| r.get::<_, i64>(0)).unwrap(), 1024);
        store.test_execute("DROP TRIGGER deny_prune;");
        assert!(store.accept_secure_message("id-new", "node-a", "body").unwrap());
        assert_eq!(store.conn.query_row("SELECT COUNT(*) FROM secure_message_ids", [], |r| r.get::<_, i64>(0)).unwrap(), 1024);
    }

    #[test]
    fn chat_reopen_keeps_latest_messages_in_chronological_order() {
        let store = Store::test_store(":memory:");
        for i in 0..505 { store.add_chat_message("YOU", &format!("message-{i}"), true); }
        let messages = store.chat_messages();
        assert_eq!(messages.len(), 500);
        assert_eq!(messages.first().unwrap().1, "message-5");
        assert_eq!(messages.last().unwrap().1, "message-504");
    }

    #[test]
    fn database_integrity_check_reports_ok() {
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        let store = Store {
            path: std::path::PathBuf::from(":memory:"),
            conn,
        };
        assert_eq!(store.database_integrity(), "ok");
    }
}
