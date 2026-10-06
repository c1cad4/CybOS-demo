use crate::CybOs;
use crate::models::{GraphLink, GraphNode};

impl CybOs {
    pub(crate) fn add_graph_node(&mut self, id: &str, label: &str, kind: &str) -> bool {
        if self.nodes.iter().any(|n| n.id == id) {
            return false;
        }

        let index = self.nodes.len() as f32;

        let angle = index * 1.7;
        let radius = 280.0 + (index % 5.0) * 55.0;

        let x = angle.cos() * radius;
        let y = angle.sin() * radius;

        let node = GraphNode {
            id: id.to_string(),
            label: label.to_string(),
            kind: kind.to_string(),
            x,
            y,
        };

        self.store.save_graph_node(&node);
        self.nodes.push(node);

        self.add_event("GRAPH", format!("New node added: {} ({})", label, kind));

        true
    }

    pub(crate) fn add_graph_link(&mut self, from: &str, to: &str, relation: &str) -> bool {
        if !self.nodes.iter().any(|n| n.id == from) || !self.nodes.iter().any(|n| n.id == to) {
            return false;
        }

        if self
            .links
            .iter()
            .any(|l| l.from == from && l.to == to && l.relation == relation)
        {
            return false;
        }

        let link = GraphLink {
            from: from.to_string(),
            to: to.to_string(),
            relation: relation.to_string(),
        };

        self.store.save_graph_link(&link);
        self.links.push(link);

        self.add_event(
            "GRAPH",
            format!("New relationship: {} --{}--> {}", from, relation, to),
        );

        true
    }

    pub(crate) fn tool_system_status(&self) -> String {
        format!(
            "System status:
Node ID: {}
Status: {}
Temperature: {:.1} C
Battery: {:.0}%",
            self.node_id, self.status, self.temperature, self.battery
        )
    }

    pub(crate) fn tool_farm_status(&self) -> String {
        let mut result = String::new();

        result.push_str("CicadaFarm status:\n");
        result.push_str("Farm node: CicadaFarm\n");
        result.push_str("Knowledge Graph entities:\n");

        for node in &self.nodes {
            if node.id.contains("cicada") || node.id.contains("hive") || node.id.contains("apiary")
            {
                result.push_str(&format!("- {}: {}\n", node.id, node.label));
            }
        }

        result
    }

    pub(crate) fn tool_get_events(&self, limit: usize) -> String {
        let mut result = String::new();

        result.push_str(&format!("Recent cybOS events (limit {}):\n", limit));

        for event in self.events.iter().take(limit) {
            result.push_str(&format!(
                "- [{}] {}: {}\n",
                event.time, event.kind, event.text
            ));
        }

        result
    }

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

    pub(crate) fn tool_create_event(&mut self, kind: &str, text: &str) -> String {
        self.add_event(kind, text.to_string());

        format!(
            "Event created successfully.\nKind: {}\nText: {}",
            kind, text
        )
    }

    pub(crate) fn route_tool(&self, query: &str) -> Option<String> {
        let q = query.to_lowercase();

        // System / node status
        if q.contains("статус системы")
            || q.contains("состояние системы")
            || q.contains("статус cybos")
            || q.contains("состояние cybos")
            || q.contains("system status")
            || q.contains("cybos status")
            || q.contains("температур")
            || q.contains("батар")
        {
            return Some(self.tool_system_status());
        }

        // Recent events
        if q.contains("последние события")
            || q.contains("что произошло")
            || q.contains("события cybos")
            || q.contains("recent events")
            || q.contains("what happened")
        {
            return Some(self.tool_get_events(10));
        }

        // Entity lookup
        if q.contains("hive-")
            || q.contains("hive ")
            || q.contains("улей")
            || q.contains("пасек")
            || q.contains("apiary")
        {
            let candidates = ["Hive-003", "hive-003", "apiary-north", "hive"];

            for candidate in candidates {
                if q.contains(&candidate.to_lowercase()) {
                    return Some(self.tool_get_entity(candidate));
                }
            }

            return Some(self.tool_search_knowledge(query));
        }

        // CicadaFarm
        if q.contains("cicadafarm")
            || q.contains("цикадаферм")
            || q.contains("ферм")
            || q.contains("farm")
        {
            return Some(self.tool_farm_status());
        }

        // General knowledge search
        if q.contains("найди")
            || q.contains("поиск")
            || q.contains("информац")
            || q.contains("знаешь о")
            || q.contains("расскажи о")
            || q.contains("search")
            || q.contains("find information")
            || q.contains("what do you know about")
        {
            return Some(self.tool_search_knowledge(query));
        }

        None
    }
}
