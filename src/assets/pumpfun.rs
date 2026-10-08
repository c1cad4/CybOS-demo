//! PumpFun-style Solana program reference.
//!
//! This is intentionally a read-only integration surface. It exposes the
//! upstream program identity and source provenance for CybDex/Assets without
//! adding wallet signing, swaps, liquidity operations, or deployments.

pub(crate) const PROGRAM_ID: &str =
    "7wUQXRQtBzTmyp9kcrmok9FKcc4RSYXxPYN9FGDLnqxb";
pub(crate) const CLUSTER: &str = "devnet";
pub(crate) const SOURCE_REPOSITORY: &str =
    "https://github.com/0xAllan123/pumpfun-smart-contract";
pub(crate) const LICENSE: &str = "MIT";
pub(crate) const MODE: &str = "READ-ONLY PROGRAM REFERENCE";

pub(crate) const CAPABILITIES: &[&str] = &[
    "initialize",
    "add_liquidity",
    "remove_liquidity",
    "swap",
    "create_raydium_pool",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pumpfun_reference_is_pinned() {
        assert_eq!(PROGRAM_ID.len(), 44);
        assert_eq!(CLUSTER, "devnet");
        assert_eq!(LICENSE, "MIT");
        assert!(SOURCE_REPOSITORY.contains("pumpfun-smart-contract"));
        assert!(CAPABILITIES.contains(&"swap"));
    }
}
