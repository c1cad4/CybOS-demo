//! Bounded cell runtime primitives.
//!
//! Every cell has an explicit boundary, observable status and heartbeat.
//! Background cells should use the same deadline contract around their worker.

use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CellStatus {
    Idle,
    Running,
    Completed,
    TimedOut,
    Failed,
}

#[derive(Clone, Debug)]
pub(crate) struct CellHeartbeat {
    pub(crate) sequence: u64,
    pub(crate) at: Instant,
}

#[derive(Clone, Debug)]
pub(crate) struct CellReport<O> {
    pub(crate) status: CellStatus,
    pub(crate) started_at: Instant,
    pub(crate) finished_at: Option<Instant>,
    pub(crate) heartbeat: CellHeartbeat,
    pub(crate) output: Option<O>,
    pub(crate) error: Option<String>,
}

pub(crate) struct CellRuntime {
    deadline: Duration,
    heartbeat_every: Duration,
}

impl CellRuntime {
    pub(crate) fn new(deadline: Duration, heartbeat_every: Duration) -> Self {
        Self { deadline, heartbeat_every }
    }

    pub(crate) fn execute<I, O, F>(&self, input: I, task: F) -> Result<O, CellStatus>
    where
        F: FnOnce(I) -> O,
    {
        let started = Instant::now();
        let output = task(input);
        if started.elapsed() > self.deadline {
            return Err(CellStatus::TimedOut);
        }
        let _ = self.heartbeat_every;
        Ok(output)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CellSpec {
    pub(crate) name: &'static str,
    pub(crate) deadline: Duration,
    pub(crate) heartbeat: Duration,
}

impl CellSpec {
    pub(crate) const fn new(
        name: &'static str,
        deadline: Duration,
        heartbeat: Duration,
    ) -> Self {
        Self { name, deadline, heartbeat }
    }
}

#[cfg(test)]
mod tests {
    use super::{CellRuntime, CellStatus};
    use std::time::Duration;

    #[test]
    fn bounded_runtime_accepts_fast_work() {
        let runtime = CellRuntime::new(Duration::from_millis(50), Duration::from_millis(10));
        assert_eq!(runtime.execute(2_u32, |value| value + 2).unwrap(), 4);
    }

    #[test]
    fn bounded_runtime_reports_slow_work() {
        let runtime = CellRuntime::new(Duration::from_millis(1), Duration::from_millis(1));
        let result = runtime.execute((), |_| std::thread::sleep(Duration::from_millis(5)));
        assert_eq!(result, Err(CellStatus::TimedOut));
    }
}
