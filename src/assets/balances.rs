//! Solana wallet and token balance service.

use crate::state::CybOs;
use crate::CICADAFARM_MINT;

impl CybOs {
    pub(crate) fn refresh_cicada_balances(&mut self) {
        const RPC: &str = "https://api.mainnet-beta.solana.com";

        // ----------------------------------------------------
        // SOL balance
        // ----------------------------------------------------
        let sol_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBalance",
            "params": [self.cicada_wallet]
        });

        if let Ok(response) = ureq::post(RPC)
            .header("content-type", "application/json")
            .send(sol_body.to_string())
        {
            if let Ok(value) = response.into_body().read_json::<serde_json::Value>() {
                if let Some(lamports) = value["result"]["value"].as_u64() {
                    self.sol_balance = Some(lamports as f64 / 1_000_000_000.0);
                }
            }
        }

        // ----------------------------------------------------
        // $CICADAFARM SPL token balance
        // ----------------------------------------------------
        let token_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "getTokenAccountsByOwner",
            "params": [
                self.cicada_wallet,
                {
                    "mint": CICADAFARM_MINT
                },
                {
                    "encoding": "jsonParsed"
                }
            ]
        });

        if let Ok(response) = ureq::post(RPC)
            .header("content-type", "application/json")
            .send(token_body.to_string())
        {
            if let Ok(value) = response.into_body().read_json::<serde_json::Value>() {
                let mut total = 0.0;

                if let Some(accounts) = value["result"]["value"].as_array() {
                    for account in accounts {
                        if let Some(amount) = account["account"]["data"]["parsed"]["info"]
                            ["tokenAmount"]["uiAmount"]
                            .as_f64()
                        {
                            total += amount;
                        }
                    }
                }

                self.cicada_balance = Some(total);
            }
        }
    }
}
