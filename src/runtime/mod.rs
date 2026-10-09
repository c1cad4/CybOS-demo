//! cybOS local runtime.
//!
//! Every runtime cell exposes explicit inputs, outputs, status, heartbeat and
//! a cooperative execution budget. The scheduler never performs network or
//! model work itself; those operations stay in bounded worker threads.

use crate::store::Store;
use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use uuid::Uuid;

const WORKER_TRACE_LIMIT: usize = 100;
static WORKER_TRACES: OnceLock<Mutex<VecDeque<WorkerTrace>>> = OnceLock::new();

#[derive(Clone, Debug)]
pub(crate) struct WorkerTrace {
    pub(crate) id: String,
    pub(crate) cell: String,
    pub(crate) started_at: String,
    pub(crate) elapsed_ms: u128,
    pub(crate) budget_ms: u128,
    pub(crate) status: String,
}

fn trace_store() -> &'static Mutex<VecDeque<WorkerTrace>> {
    WORKER_TRACES.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn record_worker_trace(trace: WorkerTrace) {
    if let Ok(mut traces) = trace_store().lock() {
        if let Some(existing) = traces.iter_mut().find(|item| item.id == trace.id) {
            *existing = trace;
        } else {
            traces.push_front(trace);
        }
        while traces.len() > WORKER_TRACE_LIMIT {
            traces.pop_back();
        }
    }
}

pub(crate) fn recent_worker_traces() -> Vec<WorkerTrace> {
    trace_store().lock().map(|traces| traces.iter().cloned().collect()).unwrap_or_default()
}

#[derive(Clone, Debug)]
pub(crate) struct WorkerContract {
    pub(crate) cell: &'static str,
    pub(crate) started: Instant,
    pub(crate) deadline: Instant,
    task_id: String,
    started_at: String,
    budget: Duration,
    status: std::sync::Arc<std::sync::Mutex<&'static str>>,
    heartbeat: std::sync::Arc<std::sync::Mutex<Instant>>,
    runs: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl WorkerContract {
    pub(crate) fn new(cell: &'static str, budget: Duration) -> Self {
        let now = Instant::now();
        let task_id = Uuid::new_v4().to_string();
        let started_at = chrono::Local::now().to_rfc3339();
        record_worker_trace(WorkerTrace {
            id: task_id.clone(),
            cell: cell.to_string(),
            started_at: started_at.clone(),
            elapsed_ms: 0,
            budget_ms: budget.as_millis(),
            status: "RUNNING".into(),
        });
        Self {
            cell,
            started: now,
            deadline: now + budget,
            task_id,
            started_at,
            budget,
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
        record_worker_trace(WorkerTrace {
            id: self.task_id.clone(),
            cell: self.cell.to_string(),
            started_at: self.started_at.clone(),
            elapsed_ms: self.started.elapsed().as_millis(),
            budget_ms: self.budget.as_millis(),
            status: status.to_string(),
        });
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
                RuntimeCell::new("ASSETS", "RPC, market API", "balances, market data", 8),
                RuntimeCell::new("BROWSER", "address, protocol", "document, links, route", 12),
                RuntimeCell::new("CYBDEX", "search, pair, OHLCV", "markets, candles, routes", 10),
                RuntimeCell::new("CYBLEX", "magnet, local files, peer events", "torrents, progress, magnet, seed state", 8),
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
        assert!(runtime.cells.len() >= 10);
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
        let worker = super::WorkerContract::new("ROBOTCYB", std::time::Duration::from_secs(1));
        assert_eq!(worker.status(), "RUNNING");
        assert!(!worker.expired());
        assert!(worker.remaining() <= std::time::Duration::from_secs(1));
        assert!(worker.heartbeat_age_ms() < 1000);
    }

    #[test]
    fn cell_status_can_track_worker_lifecycle() {
        let mut runtime = Runtime::new();

        runtime.set_status("ROBOTCYB", "RUNNING");
        assert_eq!(runtime.cell("ROBOTCYB").unwrap().status, "RUNNING");

        runtime.set_status("ROBOTCYB", "TIMEOUT");
        runtime.tick();
        assert_eq!(runtime.cell("ROBOTCYB").unwrap().status, "TIMEOUT");

        runtime.set_status("ROBOTCYB", "READY");
        assert_eq!(runtime.cell("ROBOTCYB").unwrap().status, "READY");
    }
}
