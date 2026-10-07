//! Knowledge Graph query tools.

use crate::CybOs;
use crate::models::GraphNode;

impl CybOs {
pub(crate) fn tool_get_entity(&self, query: &str) -> String {
        let q = query.to_lowercase();
        let mut result = String::new();

        for node in &self.nodes {
            if node.id.to_lowercase().contains(&q) || node.label.to_lowercase().contains(&q) {
                result.push_str(&format!(
                    "Entity found:\nID: {}\nLabel: {}\n",
                    node.id, node.label
                ));

                for link in &self.links {
                    if link.from == node.id {
                        result.push_str(&format!(
                            "Relationship: {} --{}--> {}\n",
                            link.from, link.relation, link.to
                        ));
                    }

                    if link.to == node.id {
                        result.push_str(&format!(
                            "Relationship: {} --{}--> {}\n",
                            link.from, link.relation, link.to
                        ));
                    }
                }
            }
        }

        if result.is_empty() {
            format!("No entity found matching '{}'.", query)
        } else {
            result
        }
    }

pub(crate) fn tool_search_knowledge(&self, query: &str) -> String {
        let q = query.to_lowercase();

        let mut terms: Vec<String> = q
            .split(|c: char| {
                c.is_whitespace()
                    || matches!(
                        c,
                        ',' | '.' | ':' | ';' | '!' | '?' | '(' | ')' | '"' | '\''
                    )
            })
            .filter(|term| term.len() >= 3)
            .map(|term| term.to_string())
            .collect();

        terms.sort();
        terms.dedup();

        let mut matched_nodes: Vec<&GraphNode> = Vec::new();

        for node in &self.nodes {
            let node_id = node.id.to_lowercase();
            let node_label = node.label.to_lowercase();

            let matched = terms
                .iter()
                .any(|term| node_id.contains(term) || node_label.contains(term));

            if matched {
                matched_nodes.push(node);
            }
        }

        let mut result = String::new();

        result.push_str(&format!("Knowledge search results for '{}':\n", query));

        if matched_nodes.is_empty() {
            result.push_str("No matching local knowledge found.\n");
            return result;
        }

        result.push_str("\nMATCHED ENTITIES:\n");

        for node in &matched_nodes {
            result.push_str(&format!("- ENTITY: {} | {}\n", node.id, node.label));
        }

        result.push_str("\nRELATIONSHIPS BETWEEN MATCHED OR RELATED ENTITIES:\n");

        let matched_ids: std::collections::HashSet<&str> =
            matched_nodes.iter().map(|node| node.id.as_str()).collect();

        let mut relationship_found = false;

        for link in &self.links {
            if matched_ids.contains(link.from.as_str()) || matched_ids.contains(link.to.as_str()) {
                result.push_str(&format!(
                    "- {} --{}--> {}\n",
                    link.from, link.relation, link.to
                ));

                relationship_found = true;
            }
        }

        if !relationship_found {
            result.push_str("No relationships found for matched entities.\n");
        }

        result.push_str("\nMATCHING EVENTS:\n");

        let mut event_found = false;

        for event in &self.events {
            let event_text = event.text.to_lowercase();
            let event_kind = event.kind.to_lowercase();

            if terms
                .iter()
                .any(|term| event_text.contains(term) || event_kind.contains(term))
            {
                result.push_str(&format!(
                    "- [{}] {}: {}\n",
                    event.time, event.kind, event.text
                ));

                event_found = true;
            }
        }

        if !event_found {
            result.push_str("No matching events found.\n");
        }

        result
    }
}
