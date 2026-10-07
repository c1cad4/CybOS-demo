//! Graph mutation tools.

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
}
