use crate::CybOs;
use serde_json::json;
use super::web_intent;

impl CybOs {
    fn parse_planner_json(content: &str) -> Option<serde_json::Value> {
        let clean = content.trim();

        if let Ok(value) = serde_json::from_str::<serde_json::Value>(clean) {
            return Some(value);
        }

        let stripped = clean
            .strip_prefix("```json")
            .or_else(|| clean.strip_prefix("```JSON"))
            .unwrap_or(clean)
            .trim();

        let stripped = stripped.strip_prefix("```").unwrap_or(stripped).trim();

        let stripped = stripped.strip_suffix("```").unwrap_or(stripped).trim();

        if let Ok(value) = serde_json::from_str::<serde_json::Value>(stripped) {
            return Some(value);
        }

        let start = stripped.find('{')?;
        let end = stripped.rfind('}')?;

        if end <= start {
            return None;
        }

        serde_json::from_str(&stripped[start..=end]).ok()
    }

    pub(crate) fn select_tool_with_qwen(&self, q: &str) -> Option<(String, String)> {
        let url = "http://127.0.0.1:8080/v1/chat/completions";

        let system_prompt = r#"
You are the action planner and autonomous brain of RobotCYB.

You have full freedom to answer using your own pretrained knowledge,
reasoning, local cybOS data, or available tools.

A tool is OPTIONAL, not mandatory.

Use a tool when it materially improves the answer:
- local tools for CicadaFarm, memories, Knowledge Graph, events and node state;
- web_search/web_fetch for current or external information;
- no tool for ordinary stable knowledge that you already know.

Do not refuse a normal question merely because the answer is absent
from the local database.

Available tools:

1. system_status
Use for current system status, temperature, battery or node state.

2. get_events
Use for recent cybOS events.
Argument should be a number such as "10".

3. get_entity
Use for a specific entity such as Hive-003, apiary-north,
Weather-001, RobotCYB or CicadaFarm.
Argument should be the entity name.

4. search_knowledge
Use for general searches through the local Knowledge Graph and events.
Argument should be the search query.

5. farm_status
Use for general CicadaFarm information.
Argument should normally be "CicadaFarm".

6. web_search
Use when local cybOS knowledge is insufficient or when the user asks for
public, current, external, legal, government, scientific, technical,
document, website, or other internet information.
Argument should be the complete web search query.

7. web_fetch
Use to open and read a specific URL returned by web_search.
Argument must be the complete URL.

Important web rule:
For factual, legal, governmental, scientific or document questions,
search results alone are not sufficient.
After web_search, use web_fetch on a relevant source before returning
a final answer whenever a readable source is available.

Prefer official or primary sources for legal and government questions.

Return ONLY valid JSON.

If a tool is required:
{"action":"tool","tool":"TOOL_NAME","arguments":"ARGUMENT"}

If the user can be answered without a tool:
{"action":"final","answer":"ANSWER"}

Never output explanations outside JSON.
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
                    "content": q
                }
            ],
            "max_tokens": 250,
            "temperature": 0.0,
            "chat_template_kwargs": {
                "enable_thinking": false
            }
        });

        let response = ureq::post(url)
            .header("Content-Type", "application/json")
            .send_json(&payload)
            .ok()?;

        let body = response.into_body().read_to_string().ok()?;
        let value: serde_json::Value = serde_json::from_str(&body).ok()?;

        let content = Self::qwen_visible_content(&value)?;
        let decision = Self::parse_planner_json(content)?;

        if decision["action"].as_str()? != "tool" {
            return None;
        }

        let tool = decision["tool"].as_str()?.to_string();

        let arguments = decision["arguments"].as_str().unwrap_or("").to_string();

        Some((tool, arguments))
    }

    pub(crate) fn planner_decision_from_observation(&self, observation: &str) -> Option<serde_json::Value> {
        let url = "http://127.0.0.1:8080/v1/chat/completions";

        let system_prompt = r#"
You are RobotCYB, the action planner of cybOS.

You receive:
1. ORIGINAL USER REQUEST.
2. TOOL RESULT from the last executed tool.

Your job is to complete ALL parts of the original request.

Tool protocol:

{"action":"tool","tool":"TOOL_NAME","arguments":"ARGUMENT"}

or:

{"action":"final","answer":"ANSWER"}

Available tools:
system_status
get_events
get_entity
search_knowledge
farm_status
web_search
web_fetch

WEB RULES:

web_search is only a discovery step.
It provides titles and URLs, not verified document contents.

For legal, governmental, scientific, technical, current or document questions:
1. Inspect the web_search results.
2. Choose a relevant source.
3. Use web_fetch on that URL.
4. Only then return final.

Prefer official or primary sources.
Do not treat a search-result title as the content of a document.
Do not invent facts absent from the supplied source text.

If web_fetch fails, say that the source could not be read.
Do not replace unread source content with guesses.

Never output intermediate narration such as:
"I am searching..."
"Process started..."
"I will look..."
Those are NOT valid responses.

If another tool is required, return the tool JSON.
If all requested tasks are completed, return final JSON.

Return ONLY valid JSON.
"#;

        for retry in 0..3 {
            let retry_note = if retry == 0 {
                ""
            } else {
                "\n\nRETRY: Your previous planner output was invalid. Return ONLY one valid JSON object with action=tool or action=final. No prose.\n"
            };

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
                            "{}{}",
                            observation,
                            retry_note
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
                Err(_) => continue,
            };

            let body = match response.into_body().read_to_string() {
                Ok(body) => body,
                Err(_) => continue,
            };

            let value: serde_json::Value = match serde_json::from_str(&body) {
                Ok(value) => value,
                Err(_) => continue,
            };

            let Some(content) = Self::qwen_visible_content(&value) else {
                continue;
            };

            let Some(decision) = Self::parse_planner_json(content) else {
                continue;
            };

            let Some(action) = decision["action"].as_str() else {
                continue;
            };

            match action {
                "tool" => {
                    let Some(tool) = decision["tool"].as_str() else {
                        continue;
                    };

                    let allowed = matches!(
                        tool,
                        "system_status"
                            | "get_events"
                            | "get_entity"
                            | "search_knowledge"
                            | "farm_status"
                            | "web_search"
                            | "web_fetch"
                    );

                    if !allowed {
                        continue;
                    }

                    if decision["arguments"].as_str().is_none() {
                        continue;
                    }

                    return Some(decision);
                }

                "final" => {
                    if let Some(answer) = decision["answer"].as_str() {
                        if !answer.trim().is_empty() {
                            return Some(decision);
                        }
                    }
                }

                _ => continue,
            }
        }

        None
    }

    pub(crate) fn agent_answer(&mut self, q: &str) -> String {
        let lower = q.trim().to_lowercase();

        let learning_request = lower.starts_with("запомни")
            || lower.starts_with("сохрани")
            || lower.starts_with("учти")
            || lower.starts_with("запиши")
            || lower.starts_with("remember")
            || lower.starts_with("save this")
            || lower.starts_with("store this");

        if learning_request {
            self.learn_from_user_message(q);

            return "Запомнил. Данные обработаны и сохранены в Knowledge Graph и локальной памяти cybOS.".into();
        }

        let conversation = self.conversation_context(q);

        // ----------------------------------------------------
        // Direct web path:
        // explicit internet request or follow-up to a web case
        // skips the initial planner call.
        // ----------------------------------------------------

        let forced_web_query = if let Some(query) = web_intent::web_followup_query(q, self.previous_user_query()) {
            Some(query)
        } else if web_intent::is_web_request(q) {
            Some(q.to_string())
        } else {
            None
        };

        if let Some(web_query) = forced_web_query {
            let search_result = self.tool_web_search(&web_query);

            let urls = Self::web_urls_from_result(&search_result);

            if urls.is_empty() {
                return "Веб-поиск не нашёл подходящих источников.".into();
            }

            for url in urls {
                let fetched = self.tool_web_fetch(&url);

                if !fetched.starts_with("WEB SOURCE\n") {
                    continue;
                }

                return self.web_answer_from_source(&conversation, &fetched);
            }

            return "Поиск выполнился, но cybOS не смог открыть источник. Непроверенная информация не была выдана как факт.".into();
        }

        // ----------------------------------------------------
        // Normal local agent path
        // ----------------------------------------------------

        let mut current_query = conversation.clone();
        let mut forced_tool: Option<(String, String)> = None;

        for step in 0..5 {
            let selection = if let Some(tool) = forced_tool.take() {
                Some(tool)
            } else {
                self.select_tool_with_qwen(&current_query)
            };

            let Some((tool, arguments)) = selection else {
                return self.robot_answer(q);
            };

            let result = match tool.as_str() {
                "system_status" => self.tool_system_status(),

                "get_events" => {
                    let limit = arguments.parse::<usize>().unwrap_or(10).clamp(1, 50);

                    self.tool_get_events(limit)
                }

                "get_entity" => self.tool_get_entity(&arguments),

                "search_knowledge" => self.tool_search_knowledge(&arguments),

                "farm_status" => self.tool_farm_status(),

                "web_search" => self.tool_web_search(&arguments),

                "web_fetch" => {
                    let fetched = self.tool_web_fetch(&arguments);

                    if fetched.starts_with("WEB SOURCE\n") {
                        return self.web_answer_from_source(&conversation, &fetched);
                    }

                    fetched
                }

                _ => {
                    return format!("RobotCYB: неизвестный инструмент {}.", tool);
                }
            };

            // If the planner itself selected web_search,
            // immediately fetch the best candidate.
            if tool == "web_search" {
                let urls = Self::web_urls_from_result(&result);

                for url in urls {
                    let fetched = self.tool_web_fetch(&url);

                    if fetched.starts_with("WEB SOURCE\n") {
                        return self.web_answer_from_source(&conversation, &fetched);
                    }
                }

                return "Поиск выполнился, но источник не удалось открыть. Непроверенная информация не была выдана как факт.".into();
            }

            let observation = format!(
                "ORIGINAL CONVERSATION:

{}

AGENT STEP:
{}

LAST TOOL:
{}

TOOL ARGUMENTS:
{}

TOOL RESULT:
{}
",
                conversation,
                step + 1,
                tool,
                arguments,
                result
            );

            let Some(decision) = self.planner_decision_from_observation(&observation) else {
                return "RobotCYB не смог завершить запрос в проверяемом режиме.".into();
            };

            match decision["action"].as_str() {
                Some("final") => {
                    if let Some(answer) = decision["answer"].as_str() {
                        if !answer.trim().is_empty() {
                            return answer.to_string();
                        }
                    }

                    return self.robot_answer(q);
                }

                Some("tool") => {
                    let next_tool = decision["tool"].as_str().unwrap_or("");

                    let next_arguments = decision["arguments"].as_str().unwrap_or("");

                    if next_tool.is_empty() {
                        return self.robot_answer(q);
                    }

                    forced_tool = Some((next_tool.to_string(), next_arguments.to_string()));

                    current_query = format!(
                        "{}\n\nPrevious tool result:\n{}\n\nRequested next tool: {}\nArguments: {}",
                        conversation, result, next_tool, next_arguments
                    );
                }

                _ => {
                    return self.robot_answer(q);
                }
            }
        }

        "RobotCYB достиг максимального числа шагов агента.".into()
    }






}
