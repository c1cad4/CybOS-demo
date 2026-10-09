//! Bounded, auditable workflow primitives for RobotCYB and native cybOS cells.
//!
//! This layer describes capabilities and records workflow progress; it does not
//! execute arbitrary commands, perform network calls, or move funds.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(crate) const MAX_CAPABILITY_ID: usize = 96;
pub(crate) const MAX_TEXT: usize = 8_000;
pub(crate) const MAX_CHECKPOINT_NOTE: usize = 4_000;
pub(crate) const MAX_INPUT_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CapabilityRisk {
    ReadOnly,
    LocalWrite,
    Network,
    ExternalSideEffect,
}

impl CapabilityRisk {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
            Self::LocalWrite => "local_write",
            Self::Network => "network",
            Self::ExternalSideEffect => "external_side_effect",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CapabilitySpec {
    pub(crate) id: String,
    pub(crate) description: String,
    pub(crate) risk: CapabilityRisk,
    pub(crate) requires_approval: bool,
    pub(crate) max_input_bytes: usize,
}

impl CapabilitySpec {
    pub(crate) fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        risk: CapabilityRisk,
        requires_approval: bool,
        max_input_bytes: usize,
    ) -> Result<Self, String> {
        let spec = Self {
            id: id.into().trim().to_ascii_lowercase(),
            description: description.into().trim().to_string(),
            risk,
            requires_approval,
            max_input_bytes,
        };
        spec.validate()?;
        Ok(spec)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.id.is_empty()
            || self.id.len() > MAX_CAPABILITY_ID
            || !self.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-' || c == '.')
        {
            return Err("capability id must use 1-96 lowercase ASCII letters, digits, '.', '_' or '-'".into());
        }
        if self.description.is_empty() || self.description.chars().count() > MAX_TEXT {
            return Err(format!("capability description must contain 1-{MAX_TEXT} characters"));
        }
        if self.max_input_bytes == 0 || self.max_input_bytes > MAX_INPUT_BYTES {
            return Err(format!("capability input limit must be 1-{MAX_INPUT_BYTES} bytes"));
        }
        if self.risk == CapabilityRisk::ExternalSideEffect && !self.requires_approval {
            return Err("external side-effect capabilities must require explicit approval".into());
        }
        Ok(())
    }

    pub(crate) fn validate_input(&self, input: &serde_json::Value) -> Result<(), String> {
        self.validate()?;
        let bytes = serde_json::to_vec(input)
            .map_err(|error| format!("cannot encode capability input: {error}"))?
            .len();
        if bytes > self.max_input_bytes {
            return Err(format!("capability input is {bytes} bytes; limit is {}", self.max_input_bytes));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorkflowStatus {
    Planned,
    Running,
    Checkpointed,
    Validating,
    Submitted,
    Accepted,
    Rejected,
    Failed,
    Cancelled,
}

impl WorkflowStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Running => "running",
            Self::Checkpointed => "checkpointed",
            Self::Validating => "validating",
            Self::Submitted => "submitted",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub(crate) fn can_transition_to(&self, next: &Self) -> bool {
        use WorkflowStatus::*;
        matches!(
            (self, next),
            (Planned, Running | Cancelled)
                | (Running, Checkpointed | Validating | Failed | Cancelled)
                | (Checkpointed, Running | Validating | Failed | Cancelled)
                | (Validating, Running | Submitted | Rejected | Failed | Cancelled)
                | (Submitted, Accepted | Rejected)
                | (Rejected, Running | Cancelled)
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct WorkflowRun {
    pub(crate) id: String,
    pub(crate) task_id: Option<String>,
    pub(crate) workflow_name: String,
    pub(crate) status: WorkflowStatus,
    pub(crate) step: u32,
    pub(crate) input: serde_json::Value,
    pub(crate) output: Option<serde_json::Value>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl WorkflowRun {
    pub(crate) fn new(
        workflow_name: impl Into<String>,
        task_id: Option<String>,
        input: serde_json::Value,
    ) -> Result<Self, String> {
        let now = chrono::Utc::now().to_rfc3339();
        let run = Self {
            id: Uuid::new_v4().to_string(),
            task_id,
            workflow_name: workflow_name.into().trim().to_string(),
            status: WorkflowStatus::Planned,
            step: 0,
            input,
            output: None,
            created_at: now.clone(),
            updated_at: now,
        };
        run.validate()?;
        Ok(run)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.workflow_name.is_empty() || self.workflow_name.chars().count() > 160 {
            return Err("workflow name must contain 1-160 characters".into());
        }
        if self.task_id.as_ref().is_some_and(|id| id.trim().is_empty() || id.len() > 128) {
            return Err("workflow task id must be non-empty and at most 128 bytes".into());
        }
        let input_len = serde_json::to_vec(&self.input)
            .map_err(|error| format!("cannot encode workflow input: {error}"))?.len();
        if input_len > MAX_INPUT_BYTES {
            return Err(format!("workflow input exceeds {MAX_INPUT_BYTES} bytes"));
        }
        if let Some(output) = &self.output {
            let output_len = serde_json::to_vec(output)
                .map_err(|error| format!("cannot encode workflow output: {error}"))?.len();
            if output_len > MAX_INPUT_BYTES {
                return Err(format!("workflow output exceeds {MAX_INPUT_BYTES} bytes"));
            }
        }
        Ok(())
    }

    pub(crate) fn transition(&mut self, next: WorkflowStatus) -> Result<(), String> {
        if !self.status.can_transition_to(&next) {
            return Err(format!("invalid workflow transition: {} -> {}", self.status.as_str(), next.as_str()));
        }
        self.status = next;
        self.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(())
    }

    pub(crate) fn checkpoint(&mut self, output: serde_json::Value) -> Result<(), String> {
        if self.status != WorkflowStatus::Running && self.status != WorkflowStatus::Checkpointed {
            return Err("checkpoints may only be recorded while running or resuming".into());
        }
        let bytes = serde_json::to_vec(&output)
            .map_err(|error| format!("cannot encode checkpoint output: {error}"))?.len();
        if bytes > MAX_INPUT_BYTES {
            return Err(format!("checkpoint output exceeds {MAX_INPUT_BYTES} bytes"));
        }
        self.step = self.step.saturating_add(1);
        self.output = Some(output);
        self.status = WorkflowStatus::Checkpointed;
        self.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct WorkflowCheckpoint {
    pub(crate) id: String,
    pub(crate) run_id: String,
    pub(crate) step: u32,
    pub(crate) stage: String,
    pub(crate) note: String,
    pub(crate) data: serde_json::Value,
    pub(crate) created_at: String,
}

impl WorkflowCheckpoint {
    pub(crate) fn new(
        run_id: impl Into<String>,
        step: u32,
        stage: impl Into<String>,
        note: impl Into<String>,
        data: serde_json::Value,
    ) -> Result<Self, String> {
        let checkpoint = Self {
            id: Uuid::new_v4().to_string(),
            run_id: run_id.into(),
            step,
            stage: stage.into().trim().to_string(),
            note: note.into().trim().to_string(),
            data,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        checkpoint.validate()?;
        Ok(checkpoint)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.run_id.trim().is_empty() || self.run_id.len() > 128 {
            return Err("checkpoint run id must be non-empty and at most 128 bytes".into());
        }
        if self.step == 0 {
            return Err("checkpoint step must be greater than zero".into());
        }
        if self.stage.is_empty() || self.stage.chars().count() > 160 {
            return Err("checkpoint stage must contain 1-160 characters".into());
        }
        if self.note.chars().count() > MAX_CHECKPOINT_NOTE {
            return Err(format!("checkpoint note cannot exceed {MAX_CHECKPOINT_NOTE} characters"));
        }
        let bytes = serde_json::to_vec(&self.data)
            .map_err(|error| format!("cannot encode checkpoint data: {error}"))?.len();
        if bytes > MAX_INPUT_BYTES {
            return Err(format!("checkpoint data exceeds {MAX_INPUT_BYTES} bytes"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_side_effects_cannot_be_registered_without_approval() {
        assert!(CapabilitySpec::new(
            "send_payment", "Send funds", CapabilityRisk::ExternalSideEffect, false, 1024
        ).is_err());
        assert!(CapabilitySpec::new(
            "read_sensor", "Read a sensor", CapabilityRisk::ReadOnly, false, 1024
        ).is_ok());
    }

    #[test]
    fn capability_input_is_bounded() {
        let capability = CapabilitySpec::new(
            "read_note", "Read one note", CapabilityRisk::ReadOnly, false, 8
        ).expect("valid capability");
        assert!(capability.validate_input(&serde_json::json!({"q":"ok"})).is_ok());
        assert!(capability.validate_input(&serde_json::json!({"q":"this is too long"})).is_err());
    }

    #[test]
    fn workflow_transitions_and_checkpoints_are_validated() {
        let mut run = WorkflowRun::new("farm_report", None, serde_json::json!({"zone":"north"}))
            .expect("valid workflow");
        assert!(run.transition(WorkflowStatus::Accepted).is_err());
        run.transition(WorkflowStatus::Running).expect("start");
        run.checkpoint(serde_json::json!({"rows":12})).expect("checkpoint");
        assert_eq!(run.step, 1);
        assert_eq!(run.status, WorkflowStatus::Checkpointed);
        run.transition(WorkflowStatus::Running).expect("resume");
        run.transition(WorkflowStatus::Validating).expect("validate");
        run.transition(WorkflowStatus::Submitted).expect("submit");
        run.transition(WorkflowStatus::Accepted).expect("accept");
        assert!(run.transition(WorkflowStatus::Running).is_err());
    }

    #[test]
    fn workflow_input_and_checkpoint_payloads_are_bounded() {
        let too_large = "x".repeat(MAX_INPUT_BYTES + 1);
        assert!(WorkflowRun::new("oversized", None, serde_json::json!({"payload":too_large})).is_err());
        assert!(WorkflowCheckpoint::new(
            "run-1", 1, "collect", "large", serde_json::json!({"payload":"x".repeat(MAX_INPUT_BYTES + 1)})
        ).is_err());
    }
}
