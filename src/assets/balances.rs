//! Solana wallet and token balance service.

use crate::config::CICADAFARM_MINT;
use crate::state::CybOs;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::Duration;

impl CybOs {
    fn fetch_cicada_balances(wallet: &str) -> (Option<f64>, Option<f64>) {
        const RPC: &str = "https://api.mainnet-beta.solana.com";
        const WORKER_BUDGET: Duration = Duration::from_secs(10);
        const RPC_BUDGET: Duration = Duration::from_secs(4);
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(RPC_BUDGET))
            .build()
            .into();

        let sol_balance = {
            let sol_body = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getBalance",
                "params": [wallet]
            });

            agent.post(RPC)
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

            agent.post(RPC)
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
        let contract = crate::runtime::WorkerContract::new("ASSETS", WORKER_BUDGET);
        let worker_contract = contract.clone();
        self.balance_contract = Some(contract);
        self.runtime.set_status("ASSETS", "RUNNING");

        thread::spawn(move || {
            worker_contract.heartbeat();
            if worker_contract.expired() {
                worker_contract.finish("TIMEOUT");
                return;
            }
            let result = Self::fetch_cicada_balances(&wallet);
            worker_contract.finish("READY");
            let _ = tx.send(result);
        });
    }

    pub(crate) fn poll_cicada_balances(&mut self) {
        let Some(rx) = &self.balance_task else {
            return;
        };

        if let Some(contract) = self.balance_contract.clone() {
            if contract.expired() {
                self.balance_task = None;
                self.balance_contract = None;
                contract.finish("TIMEOUT");
                self.runtime.set_status("ASSETS", "ERROR");
                return;
            }
        }

        match rx.try_recv() {
            Ok((cicada_balance, sol_balance)) => {
                self.cicada_balance = cicada_balance;
                self.sol_balance = sol_balance;
                self.balance_task = None;
                self.balance_contract = None;
                self.balance_refresh = std::time::Instant::now();
                self.runtime.set_status("ASSETS", "READY");
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.balance_task = None;
                if let Some(contract) = self.balance_contract.take() {
                    contract.finish("ERROR");
                }
                self.runtime.set_status("ASSETS", "ERROR");
            }
        }
    }
}
