//! Read-only, historical laboratory reports. Never authorizes mining or payments.
use serde::{Deserialize, Deserializer};
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
    #[serde(deserialize_with = "unique_map")]
    pub states: BTreeMap<String, String>,
    #[serde(deserialize_with = "unique_map")]
    pub allocations_atomic: BTreeMap<String, String>,
    #[serde(default, deserialize_with = "unique_map")]
    pub pending_atomic: BTreeMap<String, String>,
    wallet_balance: bool,
    payments_enabled: bool,
}
fn unique_map<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error> {
    struct Unique;
    impl<'de> serde::de::Visitor<'de> for Unique {
        type Value = BTreeMap<String, String>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a bounded map without duplicate keys")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut access: A,
        ) -> Result<Self::Value, A::Error> {
            let mut map = BTreeMap::new();
            while let Some((key, value)) = access.next_entry::<String, String>()? {
                if map.len() >= 4096 || map.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom(
                        "duplicate or excessive report entries",
                    ));
                }
            }
            Ok(map)
        }
    }
    deserializer.deserialize_map(Unique)
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
            || chrono::DateTime::parse_from_rfc3339(&report.observed_at).is_err()
            || report.states.len() > 4096
            || report.allocations_atomic.len() > 4096
            || report.states.iter().any(|(key, state)| {
                !hash(key) || !["pending", "matured", "orphaned"].contains(&state.as_str())
            })
            || report.pending_atomic.len() > 4096
            || report
                .allocations_atomic
                .iter()
                .chain(report.pending_atomic.iter())
                .any(|(key, amount)| {
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
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NONBLOCK);
        }
        let file = options
            .open(path.trim())
            .map_err(|_| "Не удалось открыть отчёт")?;
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
    #[test]
    fn rejects_duplicate_entries_and_invalid_dates() {
        let raw = fixture().to_string().replace(
            "\"worker\":\"18446744073709551616\"",
            "\"worker\":\"1\",\"worker\":\"2\"",
        );
        assert!(Report::parse(&raw).is_err());
        let mut value = fixture();
        value["observed_at"] = serde_json::json!("2026-99-99T99:99:99+00:00");
        assert!(Report::parse(&value.to_string()).is_err());
    }
    #[test]
    fn pending_amounts_are_exact_and_old_reports_remain_readable() {
        assert!(
            Report::parse(&fixture().to_string())
                .unwrap()
                .pending_atomic
                .is_empty()
        );
        let mut value = fixture();
        value["pending_atomic"] = serde_json::json!({"owner":"100"});
        assert_eq!(
            Report::parse(&value.to_string()).unwrap().pending_atomic["owner"],
            "100"
        );
        value["pending_atomic"] = serde_json::json!({"owner":"-1"});
        assert!(Report::parse(&value.to_string()).is_err());
    }
}
