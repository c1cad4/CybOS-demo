//! Solana wallet and token balance service.

use crate::config::CICADAFARM_MINT;
use crate::state::CybOs;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;

impl CybOs {
    fn fetch_cicada_balances(wallet: &str) -> (Option<f64>, Option<f64>) {
        const RPC: &str = "https://api.mainnet-beta.solana.com";

        let sol_balance = {
            let sol_body = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getBalance",
                "params": [wallet]
            });

            ureq::post(RPC)
                .header("content-type", "application/json")
                .send(sol_body.to_string())
                .ok()
                .and_then(|response| response.into_body().read_json::<serde_json::Value>().ok())
                .and_then(|value| value["result"]["value"].as_u64())
                .map(|lamports| lamports as f64 / 1_000_000_000.0)
        };

        let cicada_balance = {
            let token_body = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "getTokenAccountsByOwner",
                "params": [
                    wallet,
                    {
                        "mint": CICADAFARM_MINT
                    },
                    {
                        "encoding": "jsonParsed"
                    }
                ]
            });

            ureq::post(RPC)
                .header("content-type", "application/json")
                .send(token_body.to_string())
                .ok()
                .and_then(|response| response.into_body().read_json::<serde_json::Value>().ok())
                .and_then(|value| {
                    let mut total = 0.0;

                    for account in value["result"]["value"].as_array()? {
                        if let Some(amount) = account["account"]["data"]["parsed"]["info"]
                            ["tokenAmount"]["uiAmount"]
                            .as_f64()
                        {
                            total += amount;
                        }
                    }

                    Some(total)
                })
        };

        (cicada_balance, sol_balance)
    }

    pub(crate) fn refresh_cicada_balances(&mut self) {
        if self.balance_task.is_some() {
            return;
        }

        let wallet = self.cicada_wallet.clone();
        let (tx, rx) = mpsc::channel();

        self.balance_task = Some(rx);

        thread::spawn(move || {
            let result = Self::fetch_cicada_balances(&wallet);
            let _ = tx.send(result);
        });
    }

    pub(crate) fn poll_cicada_balances(&mut self) {
        let Some(rx) = &self.balance_task else {
            return;
        };

        match rx.try_recv() {
            Ok((cicada_balance, sol_balance)) => {
                self.cicada_balance = cicada_balance;
                self.sol_balance = sol_balance;
                self.balance_task = None;
                self.balance_refresh = std::time::Instant::now();
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.balance_task = None;
            }
        }
    }
}
