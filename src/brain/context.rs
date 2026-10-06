use crate::CybOs;

impl CybOs {
    pub(crate) fn build_brain_context(&self, query: &str) -> String {
        let mut context = String::new();

        context.push_str("=== cybOS LIVE CONTEXT ===\n");

        context.push_str("\nSYSTEM:\n");
        context.push_str(&format!(
            "Node ID: {}\nStatus: {}\nTemperature: {:.1} C\nBattery: {:.0}%\n",
            self.node_id, self.status, self.temperature, self.battery
        ));

        context.push_str("\nKNOWLEDGE GRAPH NODES:\n");
        for node in &self.nodes {
            context.push_str(&format!("- {} | {}\n", node.id, node.label));
        }

        context.push_str("\nKNOWLEDGE GRAPH RELATIONSHIPS:\n");
        for link in &self.links {
            context.push_str(&format!(
                "- {} --{}--> {}\n",
                link.from, link.relation, link.to
            ));
        }

        context.push_str("\nRELEVANT MEMORY:\n");

        let memories = self.store.search_memories(query);

        if memories.is_empty() {
            context.push_str("- No relevant stored memory found.\n");
        } else {
            for memory in memories {
                context.push_str(&format!(
                    "- [{}] source={} importance={:.1}: {}\n",
                    memory.time, memory.source, memory.importance, memory.text
                ));
            }
        }

        context.push_str("\nRECENT EVENTS:\n");
        for event in self.events.iter().take(12) {
            context.push_str(&format!(
                "- [{}] {}: {}\n",
                event.time, event.kind, event.text
            ));
        }

        context.push_str("\n=== END cybOS LIVE CONTEXT ===");

        context
    }

    pub(crate) fn conversation_context(&self, current_question: &str) -> String {
        let mut lines = Vec::new();

        for (who, message, _) in self.chat.iter().rev().take(10).rev() {
            lines.push(format!("{}: {}", who, message));
        }

        let history = if lines.is_empty() {
            "No previous conversation.".to_string()
        } else {
            lines.join("\n")
        };

        format!(
            "=== CONVERSATION CONTEXT ===\n{}\n=== CURRENT USER REQUEST ===\n{}\n=== END CONVERSATION CONTEXT ===",
            history, current_question
        )
    }

    pub(crate) fn previous_user_query(&self) -> Option<String> {
        // Prefer the last substantive web request, not a follow-up
        // such as "а действует ли он сейчас?".
        for (who, message, _) in self.chat.iter().rev() {
            if who != "YOU" || message.trim().is_empty() {
                continue;
            }

            let message = message.trim();

            if Self::is_web_request(message) && !Self::is_web_followup(message) {
                return Some(message.to_string());
            }
        }

        // Fallback: any previous user message.
        for (who, message, _) in self.chat.iter().rev() {
            if who == "YOU" && !message.trim().is_empty() {
                return Some(message.trim().to_string());
            }
        }

        None
    }
}
