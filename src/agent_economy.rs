//! First vertical slice for autonomous work: task contracts and an auditable local ledger.
//!
//! This module deliberately does not execute payments. It models work proposals,
//! acceptance criteria, budget ceilings, and recorded outcomes before external
//! payment adapters are introduced.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(crate) const MAX_TITLE_CHARS: usize = 160;
pub(crate) const MAX_DESCRIPTION_CHARS: usize = 8_000;
pub(crate) const MAX_CRITERIA_CHARS: usize = 4_000;
pub(crate) const MAX_NOTE_CHARS: usize = 2_000;
pub(crate) const MAX_BUDGET: f64 = 1_000_000.0;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentTaskStatus {
    Proposed,
    Ready,
    InProgress,
    Submitted,
    Accepted,
    Rejected,
    Cancelled,
}

impl AgentTaskStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Ready => "ready",
            Self::InProgress => "in_progress",
            Self::Submitted => "submitted",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
        }
    }

    pub(crate) fn can_transition_to(&self, next: &Self) -> bool {
        use AgentTaskStatus::*;
        matches!(
            (self, next),
            (Proposed, Ready | Cancelled)
                | (Ready, InProgress | Cancelled)
                | (InProgress, Submitted | Cancelled)
                | (Submitted, Accepted | Rejected)
                | (Rejected, InProgress | Cancelled)
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AgentTask {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) acceptance_criteria: String,
    pub(crate) budget_limit: f64,
    pub(crate) estimated_cost: f64,
    pub(crate) status: AgentTaskStatus,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl AgentTask {
    pub(crate) fn proposal(
        title: impl Into<String>,
        description: impl Into<String>,
        acceptance_criteria: impl Into<String>,
        budget_limit: f64,
        estimated_cost: f64,
    ) -> Result<Self, String> {
        let now = chrono::Utc::now().to_rfc3339();
        let task = Self {
            id: Uuid::new_v4().to_string(),
            title: title.into().trim().to_string(),
            description: description.into().trim().to_string(),
            acceptance_criteria: acceptance_criteria.into().trim().to_string(),
            budget_limit,
            estimated_cost,
            status: AgentTaskStatus::Proposed,
            created_at: now.clone(),
            updated_at: now,
        };
        task.validate()?;
        Ok(task)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.title.is_empty() || self.title.chars().count() > MAX_TITLE_CHARS {
            return Err(format!("title must contain 1-{MAX_TITLE_CHARS} characters"));
        }
        if self.description.is_empty()
            || self.description.chars().count() > MAX_DESCRIPTION_CHARS
        {
            return Err(format!(
                "description must contain 1-{MAX_DESCRIPTION_CHARS} characters"
            ));
        }
        if self.acceptance_criteria.is_empty()
            || self.acceptance_criteria.chars().count() > MAX_CRITERIA_CHARS
        {
            return Err(format!(
                "acceptance criteria must contain 1-{MAX_CRITERIA_CHARS} characters"
            ));
        }
        if !self.budget_limit.is_finite()
            || self.budget_limit <= 0.0
            || self.budget_limit > MAX_BUDGET
        {
            return Err(format!("budget limit must be finite and between 0 and {MAX_BUDGET}"));
        }
        if !self.estimated_cost.is_finite()
            || self.estimated_cost < 0.0
            || self.estimated_cost > self.budget_limit
        {
            return Err("estimated cost must be finite, non-negative, and within the budget limit".into());
        }
        Ok(())
    }

    pub(crate) fn transition(&mut self, next: AgentTaskStatus) -> Result<(), String> {
        if !self.status.can_transition_to(&next) {
            return Err(format!(
                "invalid task transition: {} -> {}",
                self.status.as_str(),
                next.as_str()
            ));
        }
        self.status = next;
        self.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LedgerKind {
    Income,
    Expense,
    Refund,
    Adjustment,
}

impl LedgerKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Income => "income",
            Self::Expense => "expense",
            Self::Refund => "refund",
            Self::Adjustment => "adjustment",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LedgerEntry {
    pub(crate) id: String,
    pub(crate) timestamp: String,
    pub(crate) kind: LedgerKind,
    /// Positive amount in the explicitly named currency. This is an accounting
    /// record, not a payment instruction or proof that funds settled.
    pub(crate) amount: f64,
    pub(crate) currency: String,
    pub(crate) task_id: Option<String>,
    pub(crate) note: String,
}

impl LedgerEntry {
    pub(crate) fn new(
        kind: LedgerKind,
        amount: f64,
        currency: impl Into<String>,
        task_id: Option<String>,
        note: impl Into<String>,
    ) -> Result<Self, String> {
        let entry = Self {
            id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            kind,
            amount,
            currency: currency.into().trim().to_ascii_uppercase(),
            task_id,
            note: note.into().trim().to_string(),
        };
        entry.validate()?;
        Ok(entry)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if !self.amount.is_finite() || self.amount <= 0.0 || self.amount > MAX_BUDGET {
            return Err(format!("ledger amount must be finite and between 0 and {MAX_BUDGET}"));
        }
        if self.currency.is_empty()
            || self.currency.len() > 12
            || !self.currency.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err("currency must be 1-12 ASCII letters, digits, or hyphens".into());
        }
        if self.note.chars().count() > MAX_NOTE_CHARS {
            return Err(format!("ledger note cannot exceed {MAX_NOTE_CHARS} characters"));
        }
        if self.task_id.as_ref().is_some_and(|id| id.trim().is_empty() || id.len() > 128) {
            return Err("task id must be non-empty and at most 128 bytes".into());
        }
        Ok(())
    }
}

/// Declarative identity and capability manifest for one local agent instance.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AgentManifest {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) purpose: String,
    pub(crate) tools: Vec<String>,
    pub(crate) capabilities: Vec<String>,
    pub(crate) compute_budget_units: u64,
    pub(crate) network_access: bool,
    pub(crate) created_at: String,
}

impl AgentManifest {
    pub(crate) fn new(
        display_name: impl Into<String>,
        purpose: impl Into<String>,
        tools: Vec<String>,
        capabilities: Vec<String>,
        compute_budget_units: u64,
        network_access: bool,
    ) -> Result<Self, String> {
        let manifest = Self {
            id: Uuid::new_v4().to_string(),
            display_name: display_name.into().trim().to_string(),
            purpose: purpose.into().trim().to_string(),
            tools,
            capabilities,
            compute_budget_units,
            network_access,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        manifest.validate()?;
        Ok(manifest)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.display_name.is_empty() || self.display_name.chars().count() > 80 {
            return Err("agent display name must contain 1-80 characters".into());
        }
        if self.purpose.is_empty() || self.purpose.chars().count() > 1_000 {
            return Err("agent purpose must contain 1-1000 characters".into());
        }
        if self.compute_budget_units == 0 || self.compute_budget_units > 1_000_000_000 {
            return Err("compute budget must be between 1 and 1,000,000,000 units".into());
        }
        if self.tools.len() > 64 || self.capabilities.len() > 64 {
            return Err("agent may declare at most 64 tools and 64 capabilities".into());
        }
        for item in self.tools.iter().chain(self.capabilities.iter()) {
            if item.trim().is_empty() || item.chars().count() > 128 {
                return Err("tool and capability names must contain 1-128 characters".into());
            }
        }
        Ok(())
    }
}

/// Attributable knowledge metadata. Raw source content is not copied here.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct KnowledgeRecord {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) source_uri: String,
    pub(crate) source_type: String,
    pub(crate) license: String,
    pub(crate) collected_at: String,
    pub(crate) confidence: f32,
    pub(crate) content_hash: String,
    pub(crate) notes: String,
}

impl KnowledgeRecord {
    pub(crate) fn new(
        title: impl Into<String>,
        source_uri: impl Into<String>,
        source_type: impl Into<String>,
        license: impl Into<String>,
        confidence: f32,
        content_hash: impl Into<String>,
        notes: impl Into<String>,
    ) -> Result<Self, String> {
        let record = Self {
            id: Uuid::new_v4().to_string(),
            title: title.into().trim().to_string(),
            source_uri: source_uri.into().trim().to_string(),
            source_type: source_type.into().trim().to_string(),
            license: license.into().trim().to_string(),
            collected_at: chrono::Utc::now().to_rfc3339(),
            confidence,
            content_hash: content_hash.into().trim().to_ascii_lowercase(),
            notes: notes.into().trim().to_string(),
        };
        record.validate()?;
        Ok(record)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.title.is_empty() || self.title.chars().count() > 240 {
            return Err("knowledge title must contain 1-240 characters".into());
        }
        if !(self.source_uri.starts_with("https://")
            || self.source_uri.starts_with("http://")
            || self.source_uri.starts_with("local://")
            || self.source_uri.starts_with("cicadafarm://"))
        {
            return Err("source URI must use https://, http://, local://, or cicadafarm://".into());
        }
        if self.source_type.is_empty() || self.source_type.chars().count() > 64 {
            return Err("source type must contain 1-64 characters".into());
        }
        if self.license.is_empty() || self.license.chars().count() > 128 {
            return Err("license must be recorded (use 'unknown' if not yet verified)".into());
        }
        if !self.confidence.is_finite() || !(0.0..=1.0).contains(&self.confidence) {
            return Err("confidence must be finite and between 0 and 1".into());
        }
        if !self.content_hash.is_empty()
            && (self.content_hash.len() != 64
                || !self.content_hash.chars().all(|c| c.is_ascii_hexdigit()))
        {
            return Err("content hash must be empty or a 64-character SHA-256 hex digest".into());
        }
        if self.notes.chars().count() > 4_000 {
            return Err("knowledge notes cannot exceed 4000 characters".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_proposal_rejects_budget_overrun_and_non_finite_values() {
        assert!(AgentTask::proposal("Report", "Make a report", "File exists", 5.0, 6.0).is_err());
        assert!(AgentTask::proposal("Report", "Make a report", "File exists", 5.0, f64::NAN).is_err());
        assert!(AgentTask::proposal(" ", "Make a report", "File exists", 5.0, 1.0).is_err());
    }

    #[test]
    fn task_lifecycle_requires_a_submission_before_acceptance() {
        let mut task = AgentTask::proposal("Report", "Make a report", "Contains sources", 10.0, 2.0)
            .expect("valid task");
        assert!(task.transition(AgentTaskStatus::Accepted).is_err());
        task.transition(AgentTaskStatus::Ready).expect("ready");
        task.transition(AgentTaskStatus::InProgress).expect("in progress");
        task.transition(AgentTaskStatus::Submitted).expect("submitted");
        task.transition(AgentTaskStatus::Accepted).expect("accepted");
        assert_eq!(task.status, AgentTaskStatus::Accepted);
    }

    #[test]
    fn agent_manifest_requires_explicit_bounded_capabilities() {
        assert!(AgentManifest::new("Robot", "Assist", vec![], vec![], 0, false).is_err());
        let manifest = AgentManifest::new(
            "RobotCYB", "Prepare farm reports", vec!["read_sensors".into()],
            vec!["observe".into(), "create_report".into()], 100, false,
        ).expect("valid manifest");
        assert_eq!(manifest.display_name, "RobotCYB");
        assert!(!manifest.network_access);
    }

    #[test]
    fn knowledge_records_require_provenance_and_valid_confidence() {
        assert!(KnowledgeRecord::new(
            "Study", "javascript:alert(1)", "paper", "unknown", 0.8, "", "",
        ).is_err());
        assert!(KnowledgeRecord::new(
            "Study", "https://example.org/study", "paper", "CC-BY-4.0", f32::NAN, "", "",
        ).is_err());
        let record = KnowledgeRecord::new(
            "Farm log", "cicadafarm://sensor/soil", "observation", "owner-provided",
            0.95, "", "Observed locally",
        ).expect("valid knowledge record");
        assert_eq!(record.confidence, 0.95);
    }

    #[test]
    fn ledger_rejects_invalid_amounts_and_currency_codes() {
        assert!(LedgerEntry::new(LedgerKind::Income, f64::INFINITY, "USD", None, "paid").is_err());
        assert!(LedgerEntry::new(LedgerKind::Expense, 1.0, "US D", None, "compute").is_err());
        assert!(LedgerEntry::new(LedgerKind::Income, 0.0, "USD", None, "paid").is_err());
    }

    #[test]
    fn ledger_normalizes_currency_and_records_kind() {
        let entry = LedgerEntry::new(LedgerKind::Income, 12.5, "usd", None, "accepted task")
            .expect("valid ledger entry");
        assert_eq!(entry.currency, "USD");
        assert_eq!(entry.kind.as_str(), "income");
    }
}
