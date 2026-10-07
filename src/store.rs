use chrono::Local;
use rusqlite::{params, Connection};
use std::{fs, path::PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use uuid::Uuid;

use crate::models::{Event, GraphLink, GraphNode, Memory};

pub(crate) struct Store {
    pub(crate) path: PathBuf,
    conn: Connection,
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
                mine INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_chat_messages_time
                ON chat_messages(time);
            "#,
        )
        .expect("cannot initialize database");
        Self { path, conn }
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
                "SELECT who,text,mine
                 FROM chat_messages
                 ORDER BY rowid ASC
                 LIMIT 500",
            )
            .unwrap();

        st.query_map([], |r| {
            let who: String = r.get(0)?;
            let text: String = r.get(1)?;
            let mine: i64 = r.get(2)?;
            Ok((who, text, mine != 0))
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect()
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
