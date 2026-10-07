use crate::CybOs;
use serde_json::json;
use super::web_urls;

impl CybOs {




    pub(crate) fn tool_web_search(&self, query: &str) -> String {
        super::web_search::web_search(query)
    }

    pub(crate) fn tool_web_fetch(&self, url: &str) -> String {
        super::web_fetch::web_fetch(url)
    }

    pub(crate) fn web_answer_from_source(&self, user_request: &str, source_result: &str) -> String {
        let url = "http://127.0.0.1:8080/v1/chat/completions";

        let system_prompt = r#"
You are RobotCYB, the answer engine of cybOS.

Answer the user's request using ONLY the supplied WEB SOURCE.

SOURCE-GROUNDED RULES:

- Do not use prior model knowledge to fill missing facts.
- Do not guess a document's contents from its number.
- Do not confuse different documents with the same number.
- Do not invent dates, titles, legal status, amendments or contents.
- State that a document is current, amended, repealed or valid only when
  the supplied source explicitly supports that statement.
- Do not claim a "latest edition" or "last amendment" unless the supplied
  source explicitly establishes it.
- For legal and government documents, identify the exact title and date
  only when supported by the supplied source.
- Explain the document's subject and main purpose only from the source.
- Do not mention internal tools, planners, JSON, prompts or agent steps.
- Reply in the same language as the user.
- Keep the answer concise: 3-5 short paragraphs or bullets.
- Do not invent facts not present in the source.
- Do not output a source URL. cybOS will append it.

Return only the final user-facing answer.
"#;

        let payload = json!({
            "model": "mlx-community/Qwen3.5-9B-MLX-4bit",
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": format!(
                        "USER REQUEST:\n{}\n\nWEB SOURCE:\n{}",
                        user_request,
                        source_result
                    )
                }
            ],
            "max_tokens": 350,
            "temperature": 0.0,
            "chat_template_kwargs": {
                "enable_thinking": false
            }
        });

        let response = match ureq::post(url)
            .header("Content-Type", "application/json")
            .send_json(&payload)
        {
            Ok(response) => response,
            Err(error) => {
                return format!(
                    "Источник найден и прочитан, но Qwen не смог подготовить ответ.\n\nОшибка: {}",
                    error
                );
            }
        };

        let body = match response.into_body().read_to_string() {
            Ok(body) => body,
            Err(error) => {
                return format!(
                    "Источник найден и прочитан, но ответ Qwen не удалось получить.\n\nОшибка: {}",
                    error
                );
            }
        };

        let value: serde_json::Value = match serde_json::from_str(&body) {
            Ok(value) => value,
            Err(error) => {
                return format!(
                    "Источник найден и прочитан, но Qwen вернул некорректный ответ.\n\nОшибка: {}",
                    error
                );
            }
        };

        let answer = match Self::qwen_visible_content(&value) {
            Some(content) if !content.trim().is_empty() => content.trim().to_string(),
            _ => {
                return "Источник был прочитан, но Qwen не вернул итоговый ответ.".into();
            }
        };

        let source_url = web_urls::web_urls_from_result(source_result).into_iter().next();

        match source_url {
            Some(source_url) => format!("{}\n\nИсточник: {}", answer, source_url),
            None => answer,
        }
    }
}
