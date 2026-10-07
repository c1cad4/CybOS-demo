//! Shared local Qwen runtime client.

use crate::CybOs;
use serde_json::{json, Value};

const QWEN_CHAT_URL: &str =
    "http://127.0.0.1:8080/v1/chat/completions";

const QWEN_MODEL: &str =
    "mlx-community/Qwen3.5-9B-MLX-4bit";

impl CybOs {
    pub(crate) fn qwen_chat_json(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        max_tokens: u64,
    ) -> Option<Value> {
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
            "temperature": 0.0,
            "chat_template_kwargs": {
                "enable_thinking": false
            }
        });

        let response = ureq::post(QWEN_CHAT_URL)
            .header("Content-Type", "application/json")
            .send_json(&payload)
            .ok()?;

        let body = response.into_body().read_to_string().ok()?;

        serde_json::from_str(&body).ok()
    }
}
