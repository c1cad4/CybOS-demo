//! Qwen planner protocol and decision logic.

use crate::CybOs;

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
        self.select_tool_with_qwen_deadline(q, std::time::Instant::now() + std::time::Duration::from_secs(30))
    }

    pub(crate) fn select_tool_with_qwen_deadline(
        &self,
        q: &str,
        deadline: std::time::Instant,
    ) -> Option<(String, String)> {

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

        let value = self.qwen_chat_json_with_deadline(
            system_prompt,
            q,
            250,
            0.0,
            deadline,
        ).ok()?;

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
        self.planner_decision_from_observation_deadline(
            observation,
            std::time::Instant::now() + std::time::Duration::from_secs(30),
        )
    }

    pub(crate) fn planner_decision_from_observation_deadline(
        &self,
        observation: &str,
        deadline: std::time::Instant,
    ) -> Option<serde_json::Value> {

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

            if deadline <= std::time::Instant::now() {
                return None;
            }

            let value = match self.qwen_chat_json_with_deadline(
                system_prompt,
                &format!("{}{}", observation, retry_note),
                350,
                0.0,
                deadline,
            ) {
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

}

#[cfg(test)]
mod tests {
    use super::CybOs;

    #[test]
    fn parses_plain_json() {
        let value = CybOs::parse_planner_json(
            r#"{"action":"final","answer":"ok"}"#,
        ).expect("plain JSON should parse");
        assert_eq!(value["action"].as_str(), Some("final"));
        assert_eq!(value["answer"].as_str(), Some("ok"));
    }

    #[test]
    fn parses_fenced_json() {
        let value = CybOs::parse_planner_json(
            "```json\n{\"action\":\"tool\",\"tool\":\"get_events\",\"arguments\":\"10\"}\n```",
        ).expect("fenced JSON should parse");
        assert_eq!(value["action"].as_str(), Some("tool"));
        assert_eq!(value["tool"].as_str(), Some("get_events"));
        assert_eq!(value["arguments"].as_str(), Some("10"));
    }

    #[test]
    fn extracts_json_from_surrounding_text() {
        let value = CybOs::parse_planner_json(
            "Planner result: {\"action\":\"final\",\"answer\":\"done\"}",
        ).expect("embedded JSON should parse");
        assert_eq!(value["action"].as_str(), Some("final"));
        assert_eq!(value["answer"].as_str(), Some("done"));
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(CybOs::parse_planner_json("not json").is_none());
    }
}
