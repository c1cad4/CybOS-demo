use std::time::{Duration, Instant};
use uuid::Uuid;

pub const DEFAULT_AGENT_BUDGET: Duration = Duration::from_secs(15);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentAction {
    CreateEvent { kind: String, text: String },
    SetPowerMode { mode: String },
    SendCyBChat { peer_id: String, message: String },
    StartRobotTask { task: String },
}

#[derive(Clone, Debug)]
pub struct AgentRequest {
    pub id: String,
    pub source: String,
    pub action: AgentAction,
    pub deadline: Instant,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentDecision {
    Accepted,
    Rejected(String),
    TimedOut,
}

#[derive(Default)]
pub struct AgentBus;

impl AgentBus {
    pub fn new() -> Self { Self }

    pub fn request(
        &self,
        source: impl Into<String>,
        action: AgentAction,
        budget: Duration,
    ) -> AgentRequest {
        let budget = budget.min(Duration::from_secs(60));
        AgentRequest {
            id: Uuid::new_v4().to_string(),
            source: source.into(),
            action,
            deadline: Instant::now() + budget,
        }
    }

    pub fn validate(&self, request: &AgentRequest) -> AgentDecision {
        if request.source.trim().is_empty() {
            return AgentDecision::Rejected("missing action source".into());
        }
        if Instant::now() >= request.deadline {
            return AgentDecision::TimedOut;
        }

        match &request.action {
            AgentAction::CreateEvent { kind, text } => {
                if kind.trim().is_empty() || text.trim().is_empty() {
                    return AgentDecision::Rejected("event kind/text is empty".into());
                }
                if text.len() > 16 * 1024 {
                    return AgentDecision::Rejected("event text exceeds 16 KiB".into());
                }
            }
            AgentAction::SetPowerMode { mode } => {
                if !matches!(mode.as_str(), "NORMAL" | "CONSERVE" | "CRITICAL") {
                    return AgentDecision::Rejected("unsupported power mode".into());
                }
            }
            AgentAction::SendCyBChat { peer_id, message } => {
                if peer_id.trim().is_empty() || message.trim().is_empty() {
                    return AgentDecision::Rejected("peer/message is empty".into());
                }
                if message.len() > 16 * 1024 {
                    return AgentDecision::Rejected("message exceeds 16 KiB".into());
                }
            }
            AgentAction::StartRobotTask { task } => {
                if task.trim().is_empty() || task.len() > 8 * 1024 {
                    return AgentDecision::Rejected("robot task is invalid".into());
                }
            }
        }

        AgentDecision::Accepted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_bounded_action() {
        let bus = AgentBus::new();
        let request = bus.request(
            "ROBOTCYB",
            AgentAction::SetPowerMode { mode: "CONSERVE".into() },
            Duration::from_secs(10),
        );
        assert_eq!(bus.validate(&request), AgentDecision::Accepted);
    }

    #[test]
    fn rejects_unknown_power_mode() {
        let bus = AgentBus::new();
        let request = bus.request(
            "ROBOTCYB",
            AgentAction::SetPowerMode { mode: "MAXIMUM".into() },
            Duration::from_secs(10),
        );
        assert!(matches!(bus.validate(&request), AgentDecision::Rejected(_)));
    }
}
