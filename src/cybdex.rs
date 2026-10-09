//! CYBDEX market data plane.
//!
//! Read-only Solana market infrastructure:
//! DexScreener for pair discovery/current metrics, GeckoTerminal for OHLCV.
//! The worker is isolated from the egui thread and explicitly bounded so a
//! future route/signing layer can be added without coupling it to the UI.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread,
    time::Duration,
};

use crate::runtime::WorkerContract;

const HTTP_BUDGET: Duration = Duration::from_secs(2);
const WORKER_BUDGET: Duration = Duration::from_secs(8);
const MAX_QUERY: usize = 256;
const MAX_RESULTS: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CybDexTimeframe {
    Min5,
    Min15,
    Hour1,
    Hour4,
    Day1,
}

impl CybDexTimeframe {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Min5 => "5M",
            Self::Min15 => "15M",
            Self::Hour1 => "1H",
            Self::Hour4 => "4H",
            Self::Day1 => "1D",
        }
    }

    fn gecko_path(self) -> (&'static str, u32) {
        match self {
            Self::Min5 => ("minute", 5),
            Self::Min15 => ("minute", 15),
            Self::Hour1 => ("hour", 1),
            Self::Hour4 => ("hour", 4),
            Self::Day1 => ("day", 1),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CybDexPair {
    pub(crate) chain_id: String,
    pub(crate) dex_id: String,
    pub(crate) pair_address: String,
    pub(crate) url: String,
    pub(crate) base_address: String,
    pub(crate) base_name: String,
    pub(crate) base_symbol: String,
    pub(crate) quote_address: String,
    pub(crate) quote_name: String,
    pub(crate) quote_symbol: String,
    pub(crate) price_native: Option<f64>,
    pub(crate) price_usd: Option<f64>,
    pub(crate) price_change_m5: Option<f64>,
    pub(crate) price_change_h1: Option<f64>,
    pub(crate) price_change_h6: Option<f64>,
    pub(crate) price_change_h24: Option<f64>,
    pub(crate) volume_h24: Option<f64>,
    pub(crate) liquidity_usd: Option<f64>,
    pub(crate) fdv: Option<f64>,
    pub(crate) market_cap: Option<f64>,
    pub(crate) buys_h24: u64,
    pub(crate) sells_h24: u64,
    pub(crate) created_at_ms: Option<i64>,
}

#[derive(Clone, Debug)]
pub(crate) struct CybDexCandle {
    pub(crate) timestamp: i64,
    pub(crate) open: f64,
    pub(crate) high: f64,
    pub(crate) low: f64,
    pub(crate) close: f64,
    pub(crate) volume: f64,
}

pub(crate) enum CybDexEvent {
    SearchResults(Vec<CybDexPair>),
    PairLoaded {
        pair: CybDexPair,
        candles: Vec<CybDexCandle>,
    },
    Status(String),
    Error(String),
}

enum CybDexCommand {
    Search(String),
    LoadPair {
        pair_address: String,
        timeframe: CybDexTimeframe,
    },
    Shutdown,
}

pub(crate) struct CybDexRuntime {
    tx: Sender<CybDexCommand>,
    rx: Receiver<CybDexEvent>,
    busy: Arc<AtomicBool>,
}

impl Default for CybDexRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl CybDexRuntime {
    pub(crate) fn new() -> Self {
        let (tx, command_rx) = mpsc::channel();
        let (event_tx, rx) = mpsc::channel();
        let busy = Arc::new(AtomicBool::new(false));
        let busy_thread = Arc::clone(&busy);

        thread::Builder::new()
            .name("cybdex-market".into())
            .spawn(move || run_worker(command_rx, event_tx, busy_thread))
            .expect("failed to spawn CYBDEX worker");

        Self { tx, rx, busy }
    }

    pub(crate) fn search(&self, query: String) -> Result<(), String> {
        let query = query.trim();
        if query.is_empty() {
            return Err("Enter a token symbol, token name or Solana mint".into());
        }
        if query.len() > MAX_QUERY {
            return Err("CYBDEX query is too large".into());
        }
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "CYBDEX is already processing a request".to_string())?;
        if self.tx.send(CybDexCommand::Search(query.to_string())).is_err() {
            self.busy.store(false, Ordering::Release);
            return Err("CYBDEX worker is not running".to_string());
        }
        Ok(())
    }

    pub(crate) fn load_pair(
        &self,
        pair_address: String,
        timeframe: CybDexTimeframe,
    ) -> Result<(), String> {
        let address = pair_address.trim();
        if address.is_empty() || address.len() > MAX_QUERY {
            return Err("Invalid pool address".into());
        }
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "CYBDEX is already processing a request".to_string())?;
        if self.tx.send(CybDexCommand::LoadPair {
            pair_address: address.to_string(),
            timeframe,
        }).is_err() {
            self.busy.store(false, Ordering::Release);
            return Err("CYBDEX worker is not running".to_string());
        }
        Ok(())
    }

    pub(crate) fn poll(&self) -> Vec<CybDexEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.rx.try_recv() {
            events.push(event);
        }
        events
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.busy.load(Ordering::Acquire)
    }
}

impl Drop for CybDexRuntime {
    fn drop(&mut self) {
        let _ = self.tx.send(CybDexCommand::Shutdown);
    }
}

fn run_worker(
    command_rx: Receiver<CybDexCommand>,
    event_tx: Sender<CybDexEvent>,
    busy: Arc<AtomicBool>,
) {
    while let Ok(command) = command_rx.recv() {
        if matches!(&command, CybDexCommand::Shutdown) {
            break;
        }

        busy.store(true, Ordering::Release);
        let contract = WorkerContract::new("CYBDEX", WORKER_BUDGET);

        let result = match command {
            CybDexCommand::Search(query) => {
                let _ = event_tx.send(CybDexEvent::Status("CYBDEX · SEARCHING · DEXSCREENER".into()));
                if contract.expired() {
                    Err("CYBDEX search deadline expired".into())
                } else {
                    search_pairs(&query)
                }
                .map(|pairs| {
                    let _ = event_tx.send(CybDexEvent::SearchResults(pairs));
                    "CYBDEX · SEARCH READY".to_string()
                })
            }
            CybDexCommand::LoadPair {
                pair_address,
                timeframe,
            } => {
                let _ = event_tx.send(CybDexEvent::Status(format!(
                    "CYBDEX · LOADING · {} · {}",
                    short_address(&pair_address),
                    timeframe.label()
                )));

                if contract.expired() {
                    Err("CYBDEX pair deadline expired".into())
                } else {
                    load_pair(&pair_address, timeframe)
                }
                .map(|(pair, candles)| {
                    let _ = event_tx.send(CybDexEvent::PairLoaded { pair, candles });
                    "CYBDEX · MARKET READY".to_string()
                })
            }
            CybDexCommand::Shutdown => Ok("CYBDEX · STOPPED".to_string()),
        };

        match result {
            Ok(status) => {
                contract.finish("READY");
                let _ = event_tx.send(CybDexEvent::Status(status));
            }
            Err(error) => {
                contract.finish("ERROR");
                let _ = event_tx.send(CybDexEvent::Error(format!("CYBDEX · {error}")));
            }
        }

        busy.store(false, Ordering::Release);
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(HTTP_BUDGET))
        .build()
        .into()
}

fn search_pairs(query: &str) -> Result<Vec<CybDexPair>, String> {
    let url = if looks_like_solana_address(query) {
        format!("https://api.dexscreener.com/token-pairs/v1/solana/{query}")
    } else {
        format!(
            "https://api.dexscreener.com/latest/dex/search?q={}",
            percent_encode(query)
        )
    };

    let response = agent()
        .get(&url)
        .header("accept", "application/json")
        .call()
        .map_err(|e| format!("market discovery request failed: {e}"))?;

    let value = response
        .into_body()
        .read_json::<serde_json::Value>()
        .map_err(|e| format!("market discovery JSON failed: {e}"))?;

    let raw_pairs = value
        .get("pairs")
        .and_then(|v| v.as_array())
        .or_else(|| value.get("data").and_then(|v| v.as_array()))
        .ok_or_else(|| "DexScreener returned no pairs".to_string())?;

    let mut pairs = raw_pairs
        .iter()
        .filter_map(parse_pair)
        .filter(|pair| pair.chain_id == "solana")
        .collect::<Vec<_>>();

    pairs.sort_by(|a, b| {
        b.liquidity_usd
            .partial_cmp(&a.liquidity_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b.volume_h24
                    .partial_cmp(&a.volume_h24)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    pairs.dedup_by(|a, b| a.pair_address == b.pair_address);
    pairs.truncate(MAX_RESULTS);

    Ok(pairs)
}

fn load_pair(
    pair_address: &str,
    timeframe: CybDexTimeframe,
) -> Result<(CybDexPair, Vec<CybDexCandle>), String> {
    let pair_url =
        format!("https://api.dexscreener.com/latest/dex/pairs/solana/{pair_address}");

    let response = agent()
        .get(&pair_url)
        .header("accept", "application/json")
        .call()
        .map_err(|e| format!("pair request failed: {e}"))?;

    let value = response
        .into_body()
        .read_json::<serde_json::Value>()
        .map_err(|e| format!("pair JSON failed: {e}"))?;

    let raw_pair = value
        .get("pair")
        .or_else(|| value.get("pairs").and_then(|v| v.as_array()).and_then(|v| v.first()))
        .ok_or_else(|| "DexScreener pair not found".to_string())?;

    let pair = parse_pair(raw_pair).ok_or_else(|| "invalid pair payload".to_string())?;
    let candles = load_ohlcv(pair_address, timeframe)?;

    Ok((pair, candles))
}

fn load_ohlcv(
    pool: &str,
    timeframe: CybDexTimeframe,
) -> Result<Vec<CybDexCandle>, String> {
    let (bucket, aggregate) = timeframe.gecko_path();
    let url = format!(
        "https://api.geckoterminal.com/api/v2/networks/solana/pools/{pool}/ohlcv/{bucket}?aggregate={aggregate}&limit=200&currency=usd"
    );

    let response = agent()
        .get(&url)
        .header("accept", "application/json")
        .call()
        .map_err(|e| format!("OHLCV request failed: {e}"))?;

    let value = response
        .into_body()
        .read_json::<serde_json::Value>()
        .map_err(|e| format!("OHLCV JSON failed: {e}"))?;

    let rows = value
        .pointer("/data/attributes/ohlcv_list")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "GeckoTerminal returned no OHLCV data".to_string())?;

    let mut candles = rows
        .iter()
        .filter_map(|row| {
            let values = row.as_array()?;
            if values.len() < 6 {
                return None;
            }
            Some(CybDexCandle {
                timestamp: values[0].as_i64()?,
                open: number(&values[1])?,
                high: number(&values[2])?,
                low: number(&values[3])?,
                close: number(&values[4])?,
                volume: number(&values[5])?,
            })
        })
        .collect::<Vec<_>>();

    candles.sort_by_key(|c| c.timestamp);
    Ok(candles)
}

fn parse_pair(value: &serde_json::Value) -> Option<CybDexPair> {
    let base = value.get("baseToken")?;
    let quote = value.get("quoteToken")?;
    let txns = value.get("txns").and_then(|v| v.get("h24"));
    let changes = value.get("priceChange");

    Some(CybDexPair {
        chain_id: value.get("chainId")?.as_str()?.to_string(),
        dex_id: value.get("dexId")?.as_str()?.to_string(),
        pair_address: value.get("pairAddress")?.as_str()?.to_string(),
        url: value.get("url").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        base_address: base.get("address").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        base_name: base.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        base_symbol: base.get("symbol").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        quote_address: quote.get("address").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        quote_name: quote.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        quote_symbol: quote.get("symbol").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        price_native: value.get("priceNative").and_then(number),
        price_usd: value.get("priceUsd").and_then(number),
        price_change_m5: changes.and_then(|v| v.get("m5")).and_then(number),
        price_change_h1: changes.and_then(|v| v.get("h1")).and_then(number),
        price_change_h6: changes.and_then(|v| v.get("h6")).and_then(number),
        price_change_h24: changes.and_then(|v| v.get("h24")).and_then(number),
        volume_h24: value.get("volume").and_then(|v| v.get("h24")).and_then(number),
        liquidity_usd: value.get("liquidity").and_then(|v| v.get("usd")).and_then(number),
        fdv: value.get("fdv").and_then(number),
        market_cap: value.get("marketCap").and_then(number),
        buys_h24: txns.and_then(|v| v.get("buys")).and_then(|v| v.as_u64()).unwrap_or(0),
        sells_h24: txns.and_then(|v| v.get("sells")).and_then(|v| v.as_u64()).unwrap_or(0),
        created_at_ms: value.get("pairCreatedAt").and_then(|v| v.as_i64()),
    })
}

fn number(value: &serde_json::Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str()?.parse::<f64>().ok())
}

fn looks_like_solana_address(value: &str) -> bool {
    let len = value.len();
    (32..=44).contains(&len) && value.bytes().all(|b| b.is_ascii_alphanumeric())
}

fn percent_encode(value: &str) -> String {
    value.bytes().map(|byte| match byte {
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (byte as char).to_string(),
        _ => format!("%{byte:02X}"),
    }).collect()
}

fn short_address(value: &str) -> String {
    if value.len() <= 12 {
        value.to_string()
    } else {
        format!("{}…{}", &value[..6], &value[value.len() - 6..])
    }
}

#[cfg(test)]
mod tests {
    use super::{looks_like_solana_address, parse_pair, percent_encode};

    #[test]
    fn recognizes_solana_like_mints() {
        assert!(looks_like_solana_address("9QLCEL7Xo9VTwgBeAYU1PWX7JJ8joKxQCYw3msjUpump"));
        assert!(!looks_like_solana_address("not-a-mint"));
    }

    #[test]
    fn percent_encodes_search() {
        assert_eq!(percent_encode("SOL/USDC"), "SOL%2FUSDC");
    }

    #[test]
    fn parses_pair_payload() {
        let payload = serde_json::json!({
            "chainId":"solana",
            "dexId":"raydium",
            "pairAddress":"POOL",
            "baseToken":{"address":"BASE","name":"Token","symbol":"TOK"},
            "quoteToken":{"address":"QUOTE","name":"USD Coin","symbol":"USDC"},
            "priceNative":"2.5",
            "priceUsd":"2.5",
            "priceChange":{"h24":12.5},
            "volume":{"h24":1000.0},
            "liquidity":{"usd":5000.0},
            "txns":{"h24":{"buys":7,"sells":3}}
        });
        let pair = parse_pair(&payload).expect("pair should parse");
        assert_eq!(pair.base_symbol, "TOK");
        assert_eq!(pair.quote_symbol, "USDC");
        assert_eq!(pair.buys_h24, 7);
    }
}
