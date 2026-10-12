//! Read-only, historical laboratory reports. Never authorizes mining or payments.
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::Read;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Report {
    schema: String,
    genesis: String,
    pub observed_at: String,
    pub chain_height: u64,
    tip_hash: String,
    pub states: BTreeMap<String, String>,
    pub allocations_atomic: BTreeMap<String, String>,
    wallet_balance: bool,
    payments_enabled: bool,
}
fn hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
impl Report {
    pub fn parse(raw: &str) -> Result<Self, String> {
        if raw.len() > 4 * 1024 * 1024 {
            return Err("Отчёт слишком большой".into());
        }
        let report: Self = serde_json::from_str(raw).map_err(|_| "Неверный формат отчёта")?;
        if report.schema != "cybhero.lab.reward-report.v1"
            || report.genesis != crate::mining::GENESIS
            || report.wallet_balance
            || report.payments_enabled
            || report.chain_height == 0
            || !hash(&report.tip_hash)
            || report.observed_at.len() > 64
            || report.observed_at.len() < 20
            || !report.observed_at.contains('T')
            || report.states.len() > 4096
            || report.allocations_atomic.len() > 4096
            || report.states.iter().any(|(key, state)| {
                !hash(key) || !["pending", "matured", "orphaned"].contains(&state.as_str())
            })
            || report.allocations_atomic.iter().any(|(key, amount)| {
                key.is_empty()
                    || key.len() > 128
                    || !key
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"._:-".contains(&c))
                    || amount.is_empty()
                    || !amount.bytes().all(|c| c.is_ascii_digit())
                    || amount.parse::<u128>().is_err()
            })
        {
            return Err("Отчёт другой сети или содержит недопустимые данные".into());
        }
        Ok(report)
    }
    pub fn load(path: &str) -> Result<Self, String> {
        let file = std::fs::File::open(path.trim()).map_err(|_| "Не удалось открыть отчёт")?;
        if !file
            .metadata()
            .map_err(|_| "Не удалось проверить файл")?
            .is_file()
        {
            return Err("Нужен обычный файл".into());
        }
        let mut raw = String::new();
        file.take(4 * 1024 * 1024 + 1)
            .read_to_string(&mut raw)
            .map_err(|_| "Не удалось прочитать отчёт")?;
        Self::parse(&raw)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> serde_json::Value {
        serde_json::json!({"schema":"cybhero.lab.reward-report.v1","genesis":crate::mining::GENESIS,"observed_at":"2026-10-12T00:00:00+00:00","chain_height":1,"tip_hash":crate::mining::GENESIS,"states":{},"allocations_atomic":{"worker":"18446744073709551616"},"wallet_balance":false,"payments_enabled":false})
    }
    #[test]
    fn accepts_atomic_strings_without_float_loss() {
        assert_eq!(
            Report::parse(&fixture().to_string())
                .unwrap()
                .allocations_atomic["worker"],
            "18446744073709551616"
        );
    }
    #[test]
    fn rejects_foreign_network_and_payment_claims() {
        for key in ["genesis", "payments_enabled", "allocations_atomic"] {
            let mut v = fixture();
            v[key] = match key {
                "genesis" => serde_json::json!("f".repeat(64)),
                "payments_enabled" => serde_json::json!(true),
                _ => serde_json::json!({"worker":"1.5"}),
            };
            assert!(Report::parse(&v.to_string()).is_err());
        }
    }
}
