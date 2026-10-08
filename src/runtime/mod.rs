//! cybOS local runtime.
//!
//! Every runtime cell exposes explicit inputs, outputs, status, heartbeat and
//! a cooperative execution budget. The scheduler never performs network or
//! model work itself; those operations stay in bounded worker threads.

use crate::store::Store;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub(crate) struct WorkerContract {
    pub(crate) cell: &'static str,
    pub(crate) started: Instant,
    pub(crate) deadline: Instant,
    status: std::sync::Arc<std::sync::Mutex<&'static str>>,
    heartbeat: std::sync::Arc<std::sync::Mutex<Instant>>,
    runs: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl WorkerContract {
    pub(crate) fn new(cell: &'static str, budget: Duration) -> Self {
        let now = Instant::now();
        Self {
            cell,
            started: now,
            deadline: now + budget,
            status: std::sync::Arc::new(std::sync::Mutex::new("RUNNING")),
            heartbeat: std::sync::Arc::new(std::sync::Mutex::new(now)),
            runs: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    pub(crate) fn heartbeat(&self) {
        if let Ok(mut value) = self.heartbeat.lock() {
            *value = Instant::now();
        }
        self.runs.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub(crate) fn expired(&self) -> bool {
        Instant::now() >= self.deadline
    }

    pub(crate) fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }

    pub(crate) fn finish(&self, status: &'static str) {
        if let Ok(mut value) = self.status.lock() {
            *value = status;
        }
        self.heartbeat();
    }

    pub(crate) fn status(&self) -> &'static str {
        self.status.lock().map(|value| *value).unwrap_or("ERROR")
    }

    pub(crate) fn heartbeat_age_ms(&self) -> u128 {
        self.heartbeat
            .lock()
            .map(|value| value.elapsed().as_millis())
            .unwrap_or(u128::MAX)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RuntimeCell {
    pub(crate) id: &'static str,
    pub(crate) inputs: &'static str,
    pub(crate) outputs: &'static str,
    pub(crate) status: &'static str,
    pub(crate) heartbeat: Instant,
    pub(crate) budget: Duration,
    pub(crate) last_run: Duration,
    pub(crate) runs: u64,
    pub(crate) overruns: u64,
}

impl RuntimeCell {
    fn new(
        id: &'static str,
        inputs: &'static str,
        outputs: &'static str,
        budget_ms: u64,
    ) -> Self {
        Self {
            id,
            inputs,
            outputs,
            status: "READY",
            heartbeat: Instant::now(),
            budget: Duration::from_millis(budget_ms),
            last_run: Duration::ZERO,
            runs: 0,
            overruns: 0,
        }
    }

    fn tick(&mut self) {
        let started = Instant::now();

        // The scheduler tick itself is deliberately tiny. Real I/O and model
        // calls are submitted to worker threads and only polled by the cell.
        self.heartbeat = Instant::now();
        self.runs = self.runs.saturating_add(1);
        self.last_run = started.elapsed();

        if self.last_run > self.budget {
            self.status = "OVER_BUDGET";
            self.overruns = self.overruns.saturating_add(1);
        } else if !matches!(self.status, "RUNNING" | "ERROR") {
            self.status = "READY";
        }
    }

    pub(crate) fn heartbeat_age_ms(&self) -> u128 {
        self.heartbeat.elapsed().as_millis()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Runtime {
    pub(crate) cells: Vec<RuntimeCell>,
    pub(crate) ticks: u64,
    pub(crate) last_tick: Instant,
}

impl Runtime {
    pub(crate) fn new() -> Self {
        Self {
            cells: vec![
                RuntimeCell::new("SYSTEM", "clock, input", "events, state", 4),
                RuntimeCell::new("ROBOTCYB", "chat, memory, tools", "answer, actions", 8),
                RuntimeCell::new("CYBCHAT", "local, LAN events", "messages, delivery", 8),
                RuntimeCell::new("CYBERGRAPH", "entities, events", "nodes, relations", 6),
                RuntimeCell::new("CICADAFARM", "sensor, manual data", "farm state", 6),
                RuntimeCell::new("NETWORK", "discovery, transport", "peers, status", 8),
                RuntimeCell::new("RADAR", "LAN/BLE discovery", "nearby peers, proximity", 8),
                RuntimeCell::new("MEMORY", "notes, events", "memories", 4),
            ],
            ticks: 0,
            last_tick: Instant::now(),
        }
    }

    pub(crate) fn tick(&mut self) {
        self.last_tick = Instant::now();
        self.ticks = self.ticks.saturating_add(1);

        for cell in &mut self.cells {
            cell.tick();
        }
    }

    pub(crate) fn cell(&self, id: &str) -> Option<&RuntimeCell> {
        self.cells.iter().find(|cell| cell.id == id)
    }

    pub(crate) fn set_status(&mut self, id: &str, status: &'static str) {
        if let Some(cell) = self.cells.iter_mut().find(|cell| cell.id == id) {
            cell.status = status;
            cell.heartbeat = Instant::now();
        }
    }

    pub(crate) fn healthy_count(&self) -> usize {
        self.cells.iter().filter(|cell| cell.status == "READY").count()
    }
}

pub(crate) fn open_store() -> Store {
    Store::open()
}

pub(crate) fn load_or_create_node_id(store: &Store) -> String {
    store.get("node_id").unwrap_or_else(|| {
        let value = format!("cyb-{}", &Uuid::new_v4().to_string()[..8]);
        store.set("node_id", &value);
        value
    })
}

pub(crate) fn load_events(store: &Store) -> Vec<crate::models::Event> {
    let mut events = store.events();

    if events.is_empty() {
        store.add_event("SYSTEM", "cybOS local runtime initialized");
        events = store.events();
    }

    events
}

#[cfg(test)]
mod tests {
    use super::Runtime;

    #[test]
    fn all_core_cells_have_contracts() {
        let runtime = Runtime::new();
        assert!(runtime.cells.len() >= 8);
        assert!(runtime.cells.iter().all(|cell| !cell.inputs.is_empty()));
        assert!(runtime.cells.iter().all(|cell| !cell.outputs.is_empty()));
        assert!(runtime.cells.iter().all(|cell| cell.budget.as_millis() > 0));
    }

    #[test]
    fn tick_updates_heartbeats_and_run_counters() {
        let mut runtime = Runtime::new();
        runtime.tick();
        assert_eq!(runtime.ticks, 1);
        assert!(runtime.cells.iter().all(|cell| cell.runs == 1));
        assert!(runtime.cells.iter().all(|cell| cell.heartbeat_age_ms() < 1000));
        assert_eq!(runtime.healthy_count(), runtime.cells.len());
    }

    #[test]
    fn worker_contract_has_bounded_deadline() {
        let worker = super::WorkerContract::new("ROBOTCYB", Duration::from_secs(1));
        assert_eq!(worker.status(), "RUNNING");
        assert!(!worker.expired());
        assert!(worker.remaining() <= Duration::from_secs(1));
        assert!(worker.heartbeat_age_ms() < 1000);
    }

    #[test]
    fn cell_status_can_track_running_worker() {
        let mut runtime = Runtime::new();
        runtime.set_status("ROBOTCYB", "RUNNING");
        assert_eq!(runtime.cell("ROBOTCYB").unwrap().status, "RUNNING");
        runtime.set_status("ROBOTCYB", "READY");
        assert_eq!(runtime.cell("ROBOTCYB").unwrap().status, "READY");
    }
}
