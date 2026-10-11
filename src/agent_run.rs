//! Durable run lifecycle inspired by checkpointed workflows; no automatic tool replay.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Run {
    pub id: String,
    pub query_hash: String,
    pub state: String,
    pub updated_at: String,
}
impl Run {
    pub fn create(query: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            query_hash: format!("{:x}", Sha256::digest(query.as_bytes())),
            state: "queued".into(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }
    pub fn transition(&mut self, state: &str) -> Result<(), String> {
        let permitted = matches!(
            (self.state.as_str(), state),
            ("queued", "running")
                | ("queued", "error")
                | ("running", "completed")
                | ("running", "error")
                | ("queued", "timeout")
                | ("running", "timeout")
        );
        if !permitted {
            return Err("Invalid agent lifecycle transition".into());
        }
        self.state = state.into();
        self.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(())
    }
    pub fn save(&self, store: &crate::store::Store) -> Result<(), String> {
        store.set_checked(
            &format!("agent-run:{}", self.id),
            &serde_json::to_string(self).map_err(|e| e.to_string())?,
        )
    }
    pub fn label(&self) -> &str {
        match self.state.as_str() {
            "queued" => "Задача поставлена в очередь",
            "running" => "Помощник работает",
            "completed" => "Ответ получен",
            "timeout" => "Истекло время ожидания",
            "error" => "Задача остановлена с ошибкой",
            _ => "Предыдущий запуск прерван",
        }
    }
}
pub(crate) fn last(store: &crate::store::Store) -> Option<Run> {
    store
        .get("agent_last_run")
        .and_then(|id| store.get(&format!("agent-run:{id}")))
        .and_then(|raw| serde_json::from_str(&raw).ok())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn final_state_cannot_be_resumed_and_queries_are_not_stored_in_checkpoint() {
        let mut run = Run::create("private query");
        run.transition("running").unwrap();
        run.transition("completed").unwrap();
        assert!(run.transition("running").is_err());
        assert!(
            !serde_json::to_string(&run)
                .unwrap()
                .contains("private query")
        );
        let store = crate::store::Store::test_store(":memory:");
        run.save(&store).unwrap();
        store.set_checked("agent_last_run", &run.id).unwrap();
        assert_eq!(last(&store).unwrap().state, "completed");
    }
}
