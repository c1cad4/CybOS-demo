//! Shared local Qwen runtime client.

use crate::CybOs;

use serde_json::{json, Value};

pub(crate) const QWEN_ADDRESS: &str = "127.0.0.1:8080";

pub(crate) const QWEN_CHAT_URL: &str = "http://127.0.0.1:8080/v1/chat/completions";

pub(crate) const QWEN_MODEL: &str =
    "mlx-community/Qwen3.5-9B-MLX-4bit";

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

        let response = ureq::post(QWEN_CHAT_URL)
            .header("Content-Type", "application/json")
            .send_json(&payload)
            .map_err(|error| error.to_string())?;

        let body = response
            .into_body()
            .read_to_string()
            .map_err(|error| error.to_string())?;

        serde_json::from_str(&body)
            .map_err(|error| error.to_string())
    }
}
