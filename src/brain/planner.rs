use crate::CybOs;
use super::web_intent;

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
        return self.run_agent_loop(q, conversation);
}






}
