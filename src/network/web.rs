use crate::CybOs;
use serde_json::json;
use super::web_html;
use super::web_urls;

impl CybOs {
    pub(crate) fn tool_web_search(&self, query: &str) -> String {
        fn percent_encode(input: &str) -> String {
            let mut out = String::new();

            for byte in input.bytes() {
                match byte {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                        out.push(byte as char)
                    }
                    b' ' => out.push('+'),
                    _ => {
                        out.push('%');
                        out.push_str(&format!("{:02X}", byte));
                    }
                }
            }

            out
        }



        fn percent_decode(input: &str) -> String {
            let bytes = input.as_bytes();
            let mut out = Vec::with_capacity(bytes.len());
            let mut i = 0;

            while i < bytes.len() {
                if bytes[i] == b'%' && i + 2 < bytes.len() {
                    let h1 = bytes[i + 1] as char;
                    let h2 = bytes[i + 2] as char;

                    let hex = format!("{}{}", h1, h2);

                    if let Ok(value) = u8::from_str_radix(&hex, 16) {
                        out.push(value);
                        i += 3;
                        continue;
                    }
                }

                if bytes[i] == b'+' {
                    out.push(b' ');
                } else {
                    out.push(bytes[i]);
                }

                i += 1;
            }

            String::from_utf8_lossy(&out).to_string()
        }

        fn resolve_url(href: &str) -> String {
            let href = web_html::html_unescape(href.trim());

            if let Some(pos) = href.find("uddg=") {
                let value = &href[pos + 5..];
                let value = value.split('&').next().unwrap_or(value);

                return percent_decode(value);
            }

            if href.starts_with("//") {
                return format!("https:{}", href);
            }

            href
        }

        fn source_class(url: &str) -> &'static str {
            if url.contains("government.ru")
                || url.contains("pravo.gov.ru")
                || url.contains("publication.pravo.gov.ru")
            {
                "OFFICIAL"
            } else if url.contains("consultant.ru") || url.contains("garant.ru") {
                "LEGAL DATABASE"
            } else {
                "OTHER"
            }
        }

        let query = query.trim();

        if query.is_empty() {
            return "Web search query is empty.".into();
        }

        let encoded = percent_encode(query);

        let url = format!("https://html.duckduckgo.com/html/?q={}", encoded);

        let response = match ureq::get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Macintosh; Intel Mac OS X) cybOS/0.6",
            )
            .call()
        {
            Ok(response) => response,
            Err(error) => {
                return format!("Web search failed: {}", error);
            }
        };

        let body = match response.into_body().read_to_string() {
            Ok(body) => body,
            Err(error) => {
                return format!("Web search response could not be read: {}", error);
            }
        };

        let mut results = Vec::new();
        let mut remaining = body.as_str();

        while let Some(start) = remaining.find("result__a") {
            remaining = &remaining[start..];

            let href_start = match remaining.find("href=\"") {
                Some(pos) => pos + 6,
                None => break,
            };

            let href_end = match remaining[href_start..].find('"') {
                Some(pos) => href_start + pos,
                None => break,
            };

            let raw_href = &remaining[href_start..href_end];
            let href = resolve_url(raw_href);

            let title_start = match remaining[href_end..].find('>') {
                Some(pos) => href_end + pos + 1,
                None => break,
            };

            let title_end = match remaining[title_start..].find("</a>") {
                Some(pos) => title_start + pos,
                None => break,
            };

            let title = web_html::html_unescape(&remaining[title_start..title_end])
                .trim()
                .to_string();

            if !href.is_empty() && !title.is_empty() {
                results.push(format!(
                    "{}. {}\nSOURCE: {}\nURL: {}",
                    results.len() + 1,
                    title,
                    source_class(&href),
                    href
                ));
            }

            remaining = &remaining[title_end..];

            if results.len() >= 5 {
                break;
            }
        }

        if results.is_empty() {
            return format!("Web search returned no parsed results for: {}", query);
        }

        format!(
            "WEB SEARCH RESULTS FOR: {}\n\n{}\n\nIMPORTANT:\nSearch results are discovery data only. Read a relevant source with web_fetch before making factual claims.",
            query,
            results.join("\n\n")
        )
    }

    pub(crate) fn tool_web_fetch(&self, url: &str) -> String {




        let url = url.trim();

        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return format!("Invalid web URL: {}", url);
        }

        let response = match ureq::get(url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Macintosh; Intel Mac OS X) cybOS/0.6",
            )
            .call()
        {
            Ok(response) => response,
            Err(error) => {
                return format!("Web fetch failed for {}: {}", url, error);
            }
        };

        let body = match response.into_body().read_to_string() {
            Ok(body) => body,
            Err(error) => {
                return format!("Web source could not be read as text: {}", error);
            }
        };

        let text = web_html::html_to_text(&body);

        if text.trim().is_empty() {
            return format!("Web source returned no readable text: {}", url);
        }

        let max_chars = 7000usize;

        let excerpt = if text.chars().count() > max_chars {
            text.chars().take(max_chars).collect::<String>()
        } else {
            text
        };

        format!("WEB SOURCE\nURL: {}\n\nSOURCE TEXT:\n{}", url, excerpt)
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
