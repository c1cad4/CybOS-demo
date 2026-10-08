//! cybOS graph domain and graph persistence.
//!
//! Owns graph bootstrap, graph loading and graph persistence.
//! SQLite access itself remains inside Store.

use crate::models::{GraphLink, GraphNode};
use crate::state::CybOs;

impl CybOs {
    fn default_graph() -> (Vec<GraphNode>, Vec<GraphLink>) {
        let farm = String::from("cicadafarm");
        let robot = String::from("robotcyb");
        let hive = String::from("hive-003");
        let apiary = String::from("apiary-north");

        let nodes = vec![
                GraphNode {
                    id: farm.clone(),
                    label: "CICADAFARM".into(),
                    kind: "FARM".into(),
                    x: 0.0,
                    y: 0.0,
                },
                GraphNode {
                    id: robot.clone(),
                    label: "ROBOTCYB".into(),
                    kind: "AGENT".into(),
                    x: -220.0,
                    y: -110.0,
                },
                GraphNode {
                    id: hive.clone(),
                    label: "HIVE #003".into(),
                    kind: "BEE".into(),
                    x: 220.0,
                    y: -110.0,
                },
                GraphNode {
                    id: apiary.clone(),
                    label: "NORTH APIARY".into(),
                    kind: "PLACE".into(),
                    x: 220.0,
                    y: 110.0,
                },
                GraphNode {
                    id: "weather-001".into(),
                    label: "WEATHER SENSOR".into(),
                    kind: "SENSOR".into(),
                    x: -220.0,
                    y: 120.0,
                },
                GraphNode {
                    id: "knowledge".into(),
                    label: "KNOWLEDGE".into(),
                    kind: "BRAIN".into(),
                    x: 0.0,
                    y: 210.0,
                },
            ];

        let links = vec![
                GraphLink {
                    from: farm.clone(),
                    to: robot,
                    relation: "has_agent".into(),
                },
                GraphLink {
                    from: farm.clone(),
                    to: hive.clone(),
                    relation: "contains".into(),
                },
                GraphLink {
                    from: hive,
                    to: apiary,
                    relation: "located_at".into(),
                },
                GraphLink {
                    from: farm.clone(),
                    to: "weather-001".into(),
                    relation: "observed_by".into(),
                },
                GraphLink {
                    from: farm,
                    to: "knowledge".into(),
                    relation: "feeds".into(),
                },
            ];

        (nodes, links)
    }

    pub(crate) fn initialize_graph(&mut self) {
        let stored_nodes = self.store.graph_nodes();
        let stored_links = self.store.graph_links();

        if stored_nodes.is_empty() {
            let (nodes, links) = Self::default_graph();

            self.nodes = nodes;
            self.links = links;

            self.persist_current_graph();
        } else {
            self.nodes = stored_nodes;
            self.links = stored_links;
        }
    }


    pub(crate) fn upsert_secure_peer(&mut self, peer_id: &str, fingerprint: &str, trusted: bool) {
        let node_id = format!("cyb:{}", peer_id);
        if !self.nodes.iter().any(|n| n.id == node_id) {
            self.nodes.push(GraphNode {
                id: node_id.clone(),
                label: format!("CYB {}", &peer_id.chars().take(8).collect::<String>()),
                kind: "CYB_NODE".into(),
                x: 0.0,
                y: 0.0,
            });
        }

        let relation = if trusted {
            "SECURE_CHAT_TRUSTED"
        } else {
            "SECURE_CHAT_TOFU"
        };

        let link = GraphLink {
            from: "robotcyb".into(),
            to: node_id,
            relation: format!("{} · {}", relation, fingerprint),
        };

        if !self.links.iter().any(|l| l.from == link.from && l.to == link.to && l.relation.starts_with(relation)) {
            self.links.push(link);
        }
        self.persist_current_graph();
    }

    pub(crate) fn persist_current_graph(&self) {
        for node in &self.nodes {
            self.store.save_graph_node(node);
        }

        for link in &self.links {
            self.store.save_graph_link(link);
        }
    }
}
