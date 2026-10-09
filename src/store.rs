use chrono::Local;
use rusqlite::{params, Connection};
use std::{fs, path::PathBuf, time::Duration};
use uuid::Uuid;

use crate::agent_economy::{AgentManifest, AgentTask, KnowledgeRecord, LedgerEntry};
use crate::agent_runtime::{CapabilitySpec, WorkflowCheckpoint, WorkflowRun};
use crate::models::{Event, GraphLink, GraphNode, Memory};

pub(crate) struct Store {
    pub(crate) path: PathBuf,
    conn: Connection,
}
impl Store {
    pub(crate) fn open() -> Self {
        let base = dirs_fallback();
        let _ = fs::create_dir_all(&base);
        let path = base.join("cybos.db");
        let conn = Connection::open(&path).expect("cannot open cybOS database");
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

            CREATE TABLE IF NOT EXISTS agent_tasks(
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                title TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                payload TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_agent_tasks_status
                ON agent_tasks(status);

            CREATE TABLE IF NOT EXISTS agent_ledger(
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                kind TEXT NOT NULL,
                amount REAL NOT NULL,
                currency TEXT NOT NULL,
                task_id TEXT,
                payload TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_agent_ledger_timestamp
                ON agent_ledger(timestamp);

            CREATE TABLE IF NOT EXISTS agent_manifests(
                id TEXT PRIMARY KEY,
                display_name TEXT NOT NULL,
                created_at TEXT NOT NULL,
                payload TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS knowledge_records(
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                source_uri TEXT NOT NULL,
                collected_at TEXT NOT NULL,
                payload TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_knowledge_records_source
                ON knowledge_records(source_uri);

            CREATE TABLE IF NOT EXISTS agent_capabilities(
                id TEXT PRIMARY KEY, risk TEXT NOT NULL,
                requires_approval INTEGER NOT NULL, payload TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS workflow_runs(
                id TEXT PRIMARY KEY, workflow_name TEXT NOT NULL,
                status TEXT NOT NULL, updated_at TEXT NOT NULL, payload TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_workflow_runs_status ON workflow_runs(status);
            CREATE TABLE IF NOT EXISTS workflow_checkpoints(
                id TEXT PRIMARY KEY, run_id TEXT NOT NULL, step INTEGER NOT NULL,
                created_at TEXT NOT NULL, payload TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_workflow_checkpoints_run_step ON workflow_checkpoints(run_id, step);
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

    /// Claims a secure message id exactly once, retaining the latest 1024 ids.
    pub(crate) fn claim_secure_message_id(&self, message_id: &str, sender: &str) -> bool {
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

    pub(crate) fn save_agent_manifest(&self, manifest: &AgentManifest) -> Result<(), String> {
        manifest.validate()?;
        let payload = serde_json::to_string(manifest)
            .map_err(|error| format!("cannot serialize agent manifest: {error}"))?;
        self.conn.execute(
            "INSERT INTO agent_manifests(id,display_name,created_at,payload)
             VALUES(?1,?2,?3,?4)
             ON CONFLICT(id) DO UPDATE SET
                 display_name=excluded.display_name,
                 payload=excluded.payload",
            params![manifest.id, manifest.display_name, manifest.created_at, payload],
        ).map_err(|error| format!("cannot save agent manifest: {error}"))?;
        Ok(())
    }

    pub(crate) fn agent_manifests(&self) -> Vec<AgentManifest> {
        let mut st = match self.conn.prepare(
            "SELECT payload FROM agent_manifests ORDER BY created_at DESC LIMIT 500"
        ) {
            Ok(st) => st,
            Err(_) => return Vec::new(),
        };
        st.query_map([], |row| row.get::<_, String>(0))
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|payload| serde_json::from_str::<AgentManifest>(&payload).ok())
            .collect()
    }

    pub(crate) fn save_knowledge_record(&self, record: &KnowledgeRecord) -> Result<(), String> {
        record.validate()?;
        let payload = serde_json::to_string(record)
            .map_err(|error| format!("cannot serialize knowledge record: {error}"))?;
        self.conn.execute(
            "INSERT INTO knowledge_records(id,title,source_uri,collected_at,payload)
             VALUES(?1,?2,?3,?4,?5)
             ON CONFLICT(id) DO UPDATE SET
                 title=excluded.title,
                 source_uri=excluded.source_uri,
                 payload=excluded.payload",
            params![record.id, record.title, record.source_uri, record.collected_at, payload],
        ).map_err(|error| format!("cannot save knowledge record: {error}"))?;
        Ok(())
    }

    pub(crate) fn knowledge_records(&self) -> Vec<KnowledgeRecord> {
        let mut st = match self.conn.prepare(
            "SELECT payload FROM knowledge_records ORDER BY collected_at DESC LIMIT 2000"
        ) {
            Ok(st) => st,
            Err(_) => return Vec::new(),
        };
        st.query_map([], |row| row.get::<_, String>(0))
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|payload| serde_json::from_str::<KnowledgeRecord>(&payload).ok())
            .collect()
    }

    /// Persist a validated task contract. No work is executed by this method.
    pub(crate) fn save_agent_task(&self, task: &AgentTask) -> Result<(), String> {
        task.validate()?;
        let payload = serde_json::to_string(task)
            .map_err(|error| format!("cannot serialize agent task: {error}"))?;
        self.conn.execute(
            "INSERT INTO agent_tasks(id,status,title,updated_at,payload)
             VALUES(?1,?2,?3,?4,?5)
             ON CONFLICT(id) DO UPDATE SET
                 status=excluded.status,
                 title=excluded.title,
                 updated_at=excluded.updated_at,
                 payload=excluded.payload",
            params![task.id, task.status.as_str(), task.title, task.updated_at, payload],
        ).map_err(|error| format!("cannot save agent task: {error}"))?;
        Ok(())
    }

    pub(crate) fn agent_tasks(&self) -> Vec<AgentTask> {
        let mut st = match self.conn.prepare(
            "SELECT payload FROM agent_tasks ORDER BY updated_at DESC LIMIT 500"
        ) {
            Ok(st) => st,
            Err(_) => return Vec::new(),
        };
        st.query_map([], |row| row.get::<_, String>(0))
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|payload| serde_json::from_str::<AgentTask>(&payload).ok())
            .collect()
    }

    /// Append an accounting record. It does not send money or assert settlement.
    pub(crate) fn append_agent_ledger(&self, entry: &LedgerEntry) -> Result<(), String> {
        entry.validate()?;
        let payload = serde_json::to_string(entry)
            .map_err(|error| format!("cannot serialize ledger entry: {error}"))?;
        self.conn.execute(
            "INSERT INTO agent_ledger(id,timestamp,kind,amount,currency,task_id,payload)
             VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![
                entry.id,
                entry.timestamp,
                entry.kind.as_str(),
                entry.amount,
                entry.currency,
                entry.task_id,
                payload
            ],
        ).map_err(|error| format!("cannot append ledger entry: {error}"))?;
        Ok(())
    }

    pub(crate) fn agent_ledger(&self) -> Vec<LedgerEntry> {
        let mut st = match self.conn.prepare(
            "SELECT payload FROM agent_ledger ORDER BY timestamp DESC, rowid DESC LIMIT 1000"
        ) {
            Ok(st) => st,
            Err(_) => return Vec::new(),
        };
        st.query_map([], |row| row.get::<_, String>(0))
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|payload| serde_json::from_str::<LedgerEntry>(&payload).ok())
            .collect()
    }

    /// Register a declared capability. This stores metadata only; it never invokes the tool.
    pub(crate) fn save_capability(&self, capability: &CapabilitySpec) -> Result<(), String> {
        capability.validate()?;
        let payload = serde_json::to_string(capability)
            .map_err(|error| format!("cannot serialize capability: {error}"))?;
        self.conn.execute(
            "INSERT INTO agent_capabilities(id,risk,requires_approval,payload)
             VALUES(?1,?2,?3,?4)
             ON CONFLICT(id) DO UPDATE SET risk=excluded.risk,
                 requires_approval=excluded.requires_approval, payload=excluded.payload",
            params![capability.id, capability.risk.as_str(), capability.requires_approval as i64, payload],
        ).map_err(|error| format!("cannot save capability: {error}"))?;
        Ok(())
    }

    pub(crate) fn capabilities(&self) -> Vec<CapabilitySpec> {
        let mut st = match self.conn.prepare("SELECT payload FROM agent_capabilities ORDER BY id ASC") {
            Ok(st) => st, Err(_) => return Vec::new(),
        };
        st.query_map([], |row| row.get::<_, String>(0))
            .ok().into_iter().flatten().filter_map(Result::ok)
            .filter_map(|payload| serde_json::from_str::<CapabilitySpec>(&payload).ok()).collect()
    }

    pub(crate) fn save_workflow_run(&self, run: &WorkflowRun) -> Result<(), String> {
        run.validate()?;
        let payload = serde_json::to_string(run)
            .map_err(|error| format!("cannot serialize workflow run: {error}"))?;
        self.conn.execute(
            "INSERT INTO workflow_runs(id,workflow_name,status,updated_at,payload)
             VALUES(?1,?2,?3,?4,?5)
             ON CONFLICT(id) DO UPDATE SET workflow_name=excluded.workflow_name,
                 status=excluded.status, updated_at=excluded.updated_at, payload=excluded.payload",
            params![run.id, run.workflow_name, run.status.as_str(), run.updated_at, payload],
        ).map_err(|error| format!("cannot save workflow run: {error}"))?;
        Ok(())
    }

    pub(crate) fn workflow_runs(&self) -> Vec<WorkflowRun> {
        let mut st = match self.conn.prepare("SELECT payload FROM workflow_runs ORDER BY updated_at DESC LIMIT 500") {
            Ok(st) => st, Err(_) => return Vec::new(),
        };
        st.query_map([], |row| row.get::<_, String>(0))
            .ok().into_iter().flatten().filter_map(Result::ok)
            .filter_map(|payload| serde_json::from_str::<WorkflowRun>(&payload).ok()).collect()
    }

    /// Checkpoints are append-only records; callers save the parent run first.
    pub(crate) fn append_workflow_checkpoint(&self, checkpoint: &WorkflowCheckpoint) -> Result<(), String> {
        checkpoint.validate()?;
        let payload = serde_json::to_string(checkpoint)
            .map_err(|error| format!("cannot serialize workflow checkpoint: {error}"))?;
        self.conn.execute(
            "INSERT INTO workflow_checkpoints(id,run_id,step,created_at,payload)
             VALUES(?1,?2,?3,?4,?5)",
            params![checkpoint.id, checkpoint.run_id, checkpoint.step, checkpoint.created_at, payload],
        ).map_err(|error| format!("cannot append workflow checkpoint: {error}"))?;
        Ok(())
    }

    pub(crate) fn workflow_checkpoints(&self, run_id: &str) -> Vec<WorkflowCheckpoint> {
        let mut st = match self.conn.prepare(
            "SELECT payload FROM workflow_checkpoints WHERE run_id=?1 ORDER BY step ASC, rowid ASC"
        ) {
            Ok(st) => st, Err(_) => return Vec::new(),
        };
        st.query_map([run_id], |row| row.get::<_, String>(0))
            .ok().into_iter().flatten().filter_map(Result::ok)
            .filter_map(|payload| serde_json::from_str::<WorkflowCheckpoint>(&payload).ok()).collect()
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
fn dirs_fallback() -> PathBuf {
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
    fn agent_work_and_ledger_round_trip() {
        use crate::agent_economy::{AgentTask, AgentTaskStatus, LedgerEntry, LedgerKind};

        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        conn.execute_batch(
            "CREATE TABLE agent_tasks(
                id TEXT PRIMARY KEY, status TEXT NOT NULL, title TEXT NOT NULL,
                updated_at TEXT NOT NULL, payload TEXT NOT NULL
             );
             CREATE TABLE agent_ledger(
                id TEXT PRIMARY KEY, timestamp TEXT NOT NULL, kind TEXT NOT NULL,
                amount REAL NOT NULL, currency TEXT NOT NULL, task_id TEXT, payload TEXT NOT NULL
             );",
        ).expect("agent schema");
        let store = Store {
            path: std::path::PathBuf::from(":memory:"),
            conn,
        };

        let mut task = AgentTask::proposal(
            "Farm report", "Summarize weekly sensor readings",
            "Report includes sources and missing-data notes", 20.0, 3.5,
        ).expect("valid task");
        task.transition(AgentTaskStatus::Ready).expect("ready");
        store.save_agent_task(&task).expect("save task");
        assert_eq!(store.agent_tasks().len(), 1);
        assert_eq!(store.agent_tasks()[0].title, "Farm report");

        let entry = LedgerEntry::new(
            LedgerKind::Income, 12.0, "USD", Some(task.id.clone()),
            "Customer accepted report",
        ).expect("valid ledger entry");
        store.append_agent_ledger(&entry).expect("append ledger");
        let ledger = store.agent_ledger();
        assert_eq!(ledger.len(), 1);
        assert_eq!(ledger[0].amount, 12.0);
        assert_eq!(ledger[0].currency, "USD");
    }

    #[test]
    fn agent_manifest_and_knowledge_provenance_round_trip() {
        use crate::agent_economy::{AgentManifest, KnowledgeRecord};

        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        conn.execute_batch(
            "CREATE TABLE agent_manifests(
                id TEXT PRIMARY KEY, display_name TEXT NOT NULL, created_at TEXT NOT NULL, payload TEXT NOT NULL
             );
             CREATE TABLE knowledge_records(
                id TEXT PRIMARY KEY, title TEXT NOT NULL, source_uri TEXT NOT NULL,
                collected_at TEXT NOT NULL, payload TEXT NOT NULL
             );",
        ).expect("schema");
        let store = Store {
            path: std::path::PathBuf::from(":memory:"),
            conn,
        };

        let manifest = AgentManifest::new(
            "RobotCYB", "Generate farm reports", vec!["read_observations".into()],
            vec!["observe".into(), "create".into()], 100, false,
        ).expect("manifest");
        store.save_agent_manifest(&manifest).expect("save manifest");
        assert_eq!(store.agent_manifests()[0].display_name, "RobotCYB");

        let record = KnowledgeRecord::new(
            "Soil sensor observation", "cicadafarm://soil/zone-a", "sensor",
            "owner-provided", 0.9, "", "Local observation",
        ).expect("knowledge record");
        store.save_knowledge_record(&record).expect("save knowledge");
        let loaded = store.knowledge_records();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].source_uri, "cicadafarm://soil/zone-a");
    }

    #[test]
    fn workflow_and_capability_records_round_trip() {
        use crate::agent_runtime::{CapabilityRisk, CapabilitySpec, WorkflowCheckpoint, WorkflowRun, WorkflowStatus};

        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        conn.execute_batch(
            "CREATE TABLE agent_capabilities(id TEXT PRIMARY KEY, risk TEXT NOT NULL, requires_approval INTEGER NOT NULL, payload TEXT NOT NULL);
             CREATE TABLE workflow_runs(id TEXT PRIMARY KEY, workflow_name TEXT NOT NULL, status TEXT NOT NULL, updated_at TEXT NOT NULL, payload TEXT NOT NULL);
             CREATE TABLE workflow_checkpoints(id TEXT PRIMARY KEY, run_id TEXT NOT NULL, step INTEGER NOT NULL, created_at TEXT NOT NULL, payload TEXT NOT NULL);"
        ).expect("workflow schema");
        let store = Store { path: std::path::PathBuf::from(":memory:"), conn };

        let capability = CapabilitySpec::new(
            "farm.read_observations", "Read local farm observations", CapabilityRisk::ReadOnly, false, 4096
        ).expect("valid capability");
        store.save_capability(&capability).expect("save capability");
        assert_eq!(store.capabilities().len(), 1);
        assert_eq!(store.capabilities()[0].id, "farm.read_observations");

        let mut run = WorkflowRun::new("farm_report", Some("task-1".into()), serde_json::json!({"zone":"north"}))
            .expect("valid run");
        run.transition(WorkflowStatus::Running).expect("start");
        store.save_workflow_run(&run).expect("save run");
        let checkpoint = WorkflowCheckpoint::new(
            run.id.clone(), 1, "collect", "observations collected", serde_json::json!({"count":4})
        ).expect("valid checkpoint");
        store.append_workflow_checkpoint(&checkpoint).expect("append checkpoint");
        assert_eq!(store.workflow_runs().len(), 1);
        assert_eq!(store.workflow_runs()[0].workflow_name, "farm_report");
        assert_eq!(store.workflow_checkpoints(&run.id).len(), 1);
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
