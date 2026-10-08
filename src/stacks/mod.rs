//! cybOS external stack registry.
//!
//! Each external technology owns an explicit boundary. A stack may communicate
//! with cybOS through a worker, adapter or local service, but it must not
//! become a hidden dependency of unrelated subsystems.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StackStatus {
    Active,
    AdapterReady,
    Planned,
    Excluded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StackDescriptor {
    pub(crate) id: &'static str,
    pub(crate) owner_cell: &'static str,
    pub(crate) transport: &'static str,
    pub(crate) purpose: &'static str,
    pub(crate) status: StackStatus,
}

pub(crate) const STACKS: &[StackDescriptor] = &[
    StackDescriptor { id: "CYBLEX_RQBIT", owner_cell: "CYBLEX", transport: "embedded Rust library + isolated Tokio worker", purpose: "BitTorrent download, seeding, DHT/trackers, magnet/.torrent archive distribution", status: StackStatus::Active },
    StackDescriptor { id: "CYBDEX_SOLANA_DATA", owner_cell: "CYBDEX", transport: "bounded HTTP + Solana RPC", purpose: "read-only Solana market and asset data plane", status: StackStatus::Active },
    StackDescriptor { id: "FARM_MODBUS", owner_cell: "CICADAFARM", transport: "future tokio-modbus adapter", purpose: "industrial sensors, meters and controllers over Modbus TCP/RTU", status: StackStatus::AdapterReady },
    StackDescriptor { id: "ROBOT_KLIPPER", owner_cell: "ROBOTCYB", transport: "future Klipper API adapter", purpose: "robot/3D-printer telemetry and bounded machine-control integration", status: StackStatus::Planned },
    StackDescriptor { id: "MEMORY_QDRANT", owner_cell: "MEMORY", transport: "future local HTTP/gRPC adapter", purpose: "vector retrieval for semantic memory and knowledge", status: StackStatus::Planned },
    StackDescriptor { id: "KNOWLEDGE_PARADEDB", owner_cell: "CYBERGRAPH", transport: "future PostgreSQL adapter", purpose: "BM25/full-text/hybrid search over large knowledge corpora", status: StackStatus::Planned },
    StackDescriptor { id: "BIO_RUST_BIO", owner_cell: "CICADAFARM", transport: "future bounded Rust worker", purpose: "bioinformatics algorithms for specialized biological datasets", status: StackStatus::Planned },
    StackDescriptor { id: "AI_RAY", owner_cell: "ROBOTCYB", transport: "future external compute adapter", purpose: "distributed Python/ML workloads outside the native desktop core", status: StackStatus::Planned },
    StackDescriptor { id: "ASYNC_STD", owner_cell: "NONE", transport: "not integrated", purpose: "avoid introducing a discontinued async runtime into cybOS", status: StackStatus::Excluded },
    StackDescriptor { id: "SOLANA_NODE", owner_cell: "ASSETS", transport: "RPC/client integration", purpose: "do not embed the full historical Solana validator repository in cybOS", status: StackStatus::Excluded },
];

pub(crate) fn active() -> impl Iterator<Item = &'static StackDescriptor> {
    STACKS.iter().filter(|stack| matches!(stack.status, StackStatus::Active))
}

#[cfg(test)]
mod tests {
    use super::{StackStatus, STACKS};

    #[test]
    fn stack_ids_are_unique() {
        for (index, left) in STACKS.iter().enumerate() {
            assert!(STACKS[index + 1..].iter().all(|right| right.id != left.id));
        }
    }

    #[test]
    fn cyblex_rqbit_is_the_active_archive_stack() {
        let stack = STACKS.iter().find(|stack| stack.id == "CYBLEX_RQBIT").expect("CYBLEX_RQBIT must be registered");
        assert_eq!(stack.status, StackStatus::Active);
        assert_eq!(stack.owner_cell, "CYBLEX");
    }

    #[test]
    fn excluded_runtimes_are_not_active() {
        assert!(STACKS.iter().filter(|stack| matches!(stack.status, StackStatus::Excluded)).all(|stack| !matches!(stack.status, StackStatus::Active)));
    }
}
