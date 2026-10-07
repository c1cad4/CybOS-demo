use crate::CybOs;
use serde_json::json;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
};

impl CybOs {
    pub(crate) fn ensure_qwen(&mut self) {
        let address = "127.0.0.1:8080";

        // Qwen уже работает — ничего не запускаем повторно.
        if let Ok(addr) = address.parse() {
            if std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(50)).is_ok() {
                self.qwen_status = "QWEN · ONLINE".into();
                self.status = "LOCAL-FIRST · QWEN ONLINE".into();
                return;
            }
        }

        // Если процесс уже был запущен cybOS, ждём его загрузки.
        if let Some(child) = &mut self.qwen_child {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.qwen_child = None;
                    self.qwen_status = "QWEN · RESTARTING".into();
                }
                Ok(None) => {
                    self.qwen_status = "QWEN · STARTING".into();
                    return;
                }
                Err(_) => {
                    self.qwen_child = None;
                }
            }
        }

        let home = match std::env::var("HOME") {
            Ok(value) => PathBuf::from(value),
            Err(_) => {
                self.qwen_status = "QWEN · HOME ERROR".into();
                return;
            }
        };

        let executable = home.join("cybAI/.venv/bin/mlx_lm.server");

        match Command::new(&executable)
            .args([
                "--model",
                "mlx-community/Qwen3.5-9B-MLX-4bit",
                "--host",
                "127.0.0.1",
                "--port",
                "8080",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => {
                self.qwen_child = Some(child);
                self.qwen_status = "QWEN · STARTING".into();
                self.status = "LOCAL-FIRST · QWEN STARTING".into();
            }
            Err(error) => {
                self.qwen_status = format!("QWEN · START FAILED: {}", error);
                self.status = "LOCAL-FIRST · QWEN ERROR".into();
            }
        }
    }

    pub(crate) fn qwen_visible_content<'a>(value: &'a serde_json::Value) -> Option<&'a str> {
        let message = &value["choices"][0]["message"];

        if let Some(content) = message["content"].as_str() {
            if !content.trim().is_empty() {
                return Some(content.trim());
            }
        }

        if let Some(reasoning) = message["reasoning"].as_str() {
            if !reasoning.trim().is_empty() {
                return Some(reasoning.trim());
            }
        }

        if let Some(text) = value["choices"][0]["text"].as_str() {
            if !text.trim().is_empty() {
                return Some(text.trim());
            }
        }

        None
    }

    pub(crate) fn robot_answer(&self, q: &str) -> String {
        let url = "http://127.0.0.1:8080/v1/chat/completions";

        let brain_context = self.build_brain_context(q);

        // Local Tool Router
        let tool_result = self.route_tool(q);

        let tool_context = match tool_result {
            Some(result) => format!(
                "\n\n=== LOCAL TOOL RESULT ===\n{}\n=== END LOCAL TOOL RESULT ===",
                result
            ),
            None => String::from(
                "\n\n=== LOCAL TOOL RESULT ===\nNo local tool was selected.\n=== END LOCAL TOOL RESULT ===",
            ),
        };

        let system_prompt = format!(
            "You are RobotCYB, the autonomous AI brain of cybOS.

You are a general-purpose intelligent assistant connected to a local
cybOS node and CicadaFarm.

You have freedom to:
- use your pretrained knowledge;
- reason about the user's question;
- use local memory and Knowledge Graph;
- use system state and farm tools;
- use internet tools when they are available and useful.

IMPORTANT:
The local database is NOT the limit of your knowledge.
Do not say that you cannot answer simply because something is absent
from SQLite or the Knowledge Graph.

For local farm facts, prefer the actual local data when it exists.
For current or externally changing information, internet information
may be more appropriate.
For ordinary stable knowledge, answer directly from your knowledge.

Think for yourself and choose the best available source.
Do not invent facts when you genuinely do not know them.

Answer naturally, directly and completely.
Reply in the same language as the user.

=== LIVE CONTEXT ===
{}

=== TOOL CONTEXT ===
{}
",
            brain_context, tool_context
        );

        let payload = json!({
            "model": "mlx-community/Qwen3.5-9B-MLX-4bit",
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": q
                }
            ],
            "max_tokens": 400,
            "temperature": 0.7,
            "chat_template_kwargs": {
                "enable_thinking": false
            }
        });

        match ureq::post(url)
            .header("Content-Type", "application/json")
            .send_json(&payload)
        {
            Ok(resp) => match resp.into_body().read_to_string() {
                Ok(body) => match serde_json::from_str::<serde_json::Value>(&body) {
                    Ok(value) => {
                        let message = &value["choices"][0]["message"];

                        if let Some(content) = message["content"].as_str() {
                            if !content.trim().is_empty() {
                                return content.to_string();
                            }
                        }

                        if let Some(reasoning) = message["reasoning"].as_str() {
                            if !reasoning.trim().is_empty() {
                                return reasoning.to_string();
                            }
                        }

                        if let Some(text) = value["choices"][0]["text"].as_str() {
                            if !text.trim().is_empty() {
                                return text.to_string();
                            }
                        }

                        "Qwen returned no visible answer.".to_string()
                    }
                    Err(e) => format!("Qwen returned invalid JSON: {}", e),
                },
                Err(e) => format!("Failed to read Qwen response: {}", e),
            },
            Err(e) => format!(
                "Qwen is offline. Start mlx_lm.server on 127.0.0.1:8080.\n\nError: {}",
                e
            ),
        }
    }
}
