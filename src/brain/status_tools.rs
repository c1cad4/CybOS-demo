//! System, farm and event tools.

use crate::CybOs;

impl CybOs {
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
}
