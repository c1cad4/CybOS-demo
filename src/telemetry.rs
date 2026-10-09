//! Local, opt-in-by-default process and host telemetry for the System page.
//! Metrics stay on-device; this module performs no network requests.
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use sysinfo::{Components, Disks, Networks, System};

const SAMPLE_INTERVAL: Duration = Duration::from_secs(2);
const HISTORY_LIMIT: usize = 60;
const PROCESS_LIMIT: usize = 8;

#[derive(Clone, Debug)]
pub(crate) struct ProcessMetric {
    pub(crate) pid: String,
    pub(crate) name: String,
    pub(crate) cpu_percent: f32,
    pub(crate) memory_bytes: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct TelemetrySample {
    pub(crate) timestamp: String,
    pub(crate) cpu_percent: f32,
    pub(crate) memory_used_bytes: u64,
    pub(crate) memory_total_bytes: u64,
    pub(crate) process_count: usize,
    pub(crate) disk_total_bytes: u64,
    pub(crate) disk_available_bytes: u64,
    pub(crate) network_received_bytes: u64,
    pub(crate) network_transmitted_bytes: u64,
    pub(crate) temperature_celsius: Option<f32>,
}

pub(crate) struct TelemetrySampler {
    system: System,
    last_sample: Instant,
    pub(crate) current: TelemetrySample,
    pub(crate) history: VecDeque<TelemetrySample>,
    pub(crate) top_processes: Vec<ProcessMetric>,
    pub(crate) sensor_count: usize,
}

impl TelemetrySampler {
    pub(crate) fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        let components = Components::new_with_refreshed_list();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        let temperature = components
            .list()
            .iter()
            .filter_map(|component| component.temperature())
            .filter(|value| value.is_finite())
            .reduce(f32::max);
        let disk_total_bytes = disks.list().iter().map(|disk| disk.total_space()).sum();
        let disk_available_bytes = disks.list().iter().map(|disk| disk.available_space()).sum();
        let network_received_bytes = networks.iter().map(|(_, data)| data.received()).sum();
        let network_transmitted_bytes = networks.list().iter().map(|(_, data)| data.transmitted()).sum();
        let current = TelemetrySample {
            timestamp: chrono::Local::now().to_rfc3339(),
            cpu_percent: system.global_cpu_info().cpu_usage(),
            memory_used_bytes: system.used_memory(),
            memory_total_bytes: system.total_memory(),
            process_count: system.processes().len(),
            disk_total_bytes,
            disk_available_bytes,
            network_received_bytes,
            network_transmitted_bytes,
            temperature_celsius: temperature,
        };
        Self {
            system,
            last_sample: Instant::now(),
            current: current.clone(),
            history: VecDeque::from([current]),
            top_processes: Vec::new(),
            sensor_count: components.list().len(),
        }
    }

    pub(crate) fn system_cpu_count(&self) -> usize {
        self.system.cpus().len()
    }

    /// Samples at most once every two seconds to keep monitoring overhead low.
    pub(crate) fn refresh_if_due(&mut self) {
        if self.last_sample.elapsed() < SAMPLE_INTERVAL {
            return;
        }
        self.system.refresh_all();
        let components = Components::new_with_refreshed_list();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        let temperature = components
            .list()
            .iter()
            .filter_map(|component| component.temperature())
            .filter(|value| value.is_finite())
            .reduce(f32::max);

        let mut processes = self.system.processes().iter().map(|(pid, process)| ProcessMetric {
            pid: pid.to_string(),
            name: process.name().to_string_lossy().into_owned(),
            cpu_percent: process.cpu_usage(),
            memory_bytes: process.memory(),
        }).collect::<Vec<_>>();
        processes.sort_by(|a, b| b.cpu_percent.total_cmp(&a.cpu_percent));
        processes.truncate(PROCESS_LIMIT);
        self.top_processes = processes;
        self.sensor_count = components.list().len();

        let sample = TelemetrySample {
            timestamp: chrono::Local::now().to_rfc3339(),
            cpu_percent: self.system.global_cpu_info().cpu_usage(),
            memory_used_bytes: self.system.used_memory(),
            memory_total_bytes: self.system.total_memory(),
            process_count: self.system.processes().len(),
            disk_total_bytes: disks.list().iter().map(|disk| disk.total_space()).sum(),
            disk_available_bytes: disks.list().iter().map(|disk| disk.available_space()).sum(),
            network_received_bytes: networks.list().iter().map(|(_, data)| data.received()).sum(),
            network_transmitted_bytes: networks.list().iter().map(|(_, data)| data.transmitted()).sum(),
            temperature_celsius: temperature,
        };
        self.current = sample.clone();
        self.history.push_back(sample);
        while self.history.len() > HISTORY_LIMIT {
            self.history.pop_front();
        }
        self.last_sample = Instant::now();
    }

    pub(crate) fn export_json(&self) -> serde_json::Value {
        serde_json::json!({
            "sample_interval_seconds": SAMPLE_INTERVAL.as_secs(),
            "history_limit": HISTORY_LIMIT,
            "current": {
                "timestamp": self.current.timestamp,
                "cpu_percent": self.current.cpu_percent,
                "memory_used_bytes": self.current.memory_used_bytes,
                "memory_total_bytes": self.current.memory_total_bytes,
                "process_count": self.current.process_count,
                "disk_total_bytes": self.current.disk_total_bytes,
                "disk_available_bytes": self.current.disk_available_bytes,
                "network_received_bytes_since_os_counters_reset": self.current.network_received_bytes,
                "network_transmitted_bytes_since_os_counters_reset": self.current.network_transmitted_bytes,
                "temperature_celsius": self.current.temperature_celsius,
            },
            "history": self.history.iter().map(|sample| serde_json::json!({
                "timestamp": sample.timestamp,
                "cpu_percent": sample.cpu_percent,
                "memory_used_bytes": sample.memory_used_bytes,
                "memory_total_bytes": sample.memory_total_bytes,
                "temperature_celsius": sample.temperature_celsius,
            })).collect::<Vec<_>>(),
            "top_processes": self.top_processes.iter().map(|process| serde_json::json!({
                "pid": process.pid,
                "name": process.name,
                "cpu_percent": process.cpu_percent,
                "memory_bytes": process.memory_bytes,
            })).collect::<Vec<_>>(),
            "temperature_sensor_count": self.sensor_count,
            "privacy": "Collected locally. No telemetry is uploaded by this module."
        })
    }
}
