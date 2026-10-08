use crate::CybOs;
use crate::brain::qwen_runtime::{QWEN_ADDRESS, QWEN_MODEL};
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
};

impl CybOs {
    pub(crate) fn ensure_qwen(&mut self) {
        let address = QWEN_ADDRESS;

        if Instant::now() < self.qwen_retry_after {
            return;
        }

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
                QWEN_MODEL,
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
                self.qwen_retry_after = Instant::now() + Duration::from_secs(2);
                self.qwen_status = "QWEN · STARTING".into();
                self.status = "LOCAL-FIRST · QWEN STARTING".into();
            }
            Err(error) => {
                self.qwen_status = format!("QWEN · START FAILED: {}", error);
                self.status = "LOCAL-FIRST · QWEN ERROR".into();
                self.qwen_retry_after = Instant::now() + Duration::from_secs(5);
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

        match self.qwen_chat_json_with_temperature(
            &system_prompt,
            q,
            400,
            0.7,
        ) {
            Ok(value) => match Self::qwen_visible_content(&value) {
                Some(content) if !content.trim().is_empty() => {
                    content.to_string()
                }
                _ => "Qwen returned no visible answer.".to_string(),
            },
            Err(error) => format!(
                "Qwen is offline. Start mlx_lm.server on {}.\n\nError: {}",
                QWEN_ADDRESS,
                error
            ),
        }
    }
}
