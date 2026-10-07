use crate::CybOs;
use super::web_intent;
use super::web_planner;

impl CybOs {
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
            return self.direct_web_answer(&conversation, &web_query);
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

                "web_search" | "web_fetch" => {
                    match self.handle_web_tool(
                        &tool,
                        &arguments,
                        &conversation,
                    ) {
                        web_planner::WebToolOutcome::Answer(answer) => {
                            return answer;
                        }
                        web_planner::WebToolOutcome::Continue(result) => result,
                    }
                }

                _ => {
                    return format!("RobotCYB: неизвестный инструмент {}.", tool);
                }
            };

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
