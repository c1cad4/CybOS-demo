//! Shared local Qwen runtime client.

use crate::CybOs;

use serde_json::{json, Value};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

pub(crate) const QWEN_ADDRESS: &str = "127.0.0.1:8080";

pub(crate) const QWEN_CHAT_URL: &str =
    "http://127.0.0.1:8080/v1/chat/completions";

pub(crate) const QWEN_MODEL: &str =
    "mlx-community/Qwen3.5-9B-MLX-4bit";

const QWEN_REQUEST_BUDGET: Duration = Duration::from_secs(30);
const QWEN_MAX_PROMPT_BYTES: usize = 64 * 1024;

fn qwen_agent(budget: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(budget))
        .build()
        .into()
}

fn bounded_qwen_request(payload: Value, budget: Duration) -> Result<Value, String> {
    let prompt_bytes = payload["messages"]
        .as_array()
        .map(|messages| {
            messages
                .iter()
                .filter_map(|message| message["content"].as_str())
                .map(str::len)
                .sum::<usize>()
        })
        .unwrap_or(0);

    if prompt_bytes > QWEN_MAX_PROMPT_BYTES {
        return Err(format!(
            "Qwen prompt exceeds the {} KiB input limit.",
            QWEN_MAX_PROMPT_BYTES / 1024
        ));
    }

    let budget = budget.min(QWEN_REQUEST_BUDGET);
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let result = (|| {
            let agent = qwen_agent(budget);
            let response = agent
                .post(QWEN_CHAT_URL)
                .header("Content-Type", "application/json")
                .send_json(&payload)
                .map_err(|error| error.to_string())?;

            let body = response
                .into_body()
                .read_to_string()
                .map_err(|error| error.to_string())?;

            serde_json::from_str::<Value>(&body)
                .map_err(|error| error.to_string())
        })();

        let _ = tx.send(result);
    });

    rx.recv_timeout(budget)
        .map_err(|_| format!("Qwen request exceeded its {} second runtime budget.", budget.as_secs()))?
}

impl CybOs {
    pub(crate) fn qwen_chat_json(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        max_tokens: u64,
    ) -> Option<Value> {
        self.qwen_chat_json_with_temperature(
            system_prompt,
            user_prompt,
            max_tokens,
            0.0,
        )
        .ok()
    }

    pub(crate) fn qwen_chat_json_with_temperature(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        max_tokens: u64,
        temperature: f64,
    ) -> Result<Value, String> {
        let payload = json!({
            "model": QWEN_MODEL,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": user_prompt
                }
            ],
            "max_tokens": max_tokens,
            "temperature": temperature,
            "chat_template_kwargs": {
                "enable_thinking": false
            }
        });

        bounded_qwen_request(payload, QWEN_REQUEST_BUDGET)
    }

    pub(crate) fn qwen_chat_json_with_deadline(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        max_tokens: u64,
        temperature: f64,
        deadline: Instant,
    ) -> Result<Value, String> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("Qwen worker deadline expired before request start.".into());
        }

        let payload = json!({
            "model": QWEN_MODEL,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": user_prompt
                }
            ],
            "max_tokens": max_tokens,
            "temperature": temperature,
            "chat_template_kwargs": {
                "enable_thinking": false
            }
        });

        bounded_qwen_request(payload, remaining)
    }
}
