//! Bounded, auditable workflow primitives for RobotCYB and native cybOS cells.
//!
//! This layer describes capabilities and records workflow progress; it does not
//! execute arbitrary commands, perform network calls, or move funds.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
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


/// Built-in, deterministic contract check used to validate a task before real work.
pub(crate) struct TaskContractValidator;

impl CapabilityHandler for TaskContractValidator {
    fn execute(&self, input: &serde_json::Value) -> Result<serde_json::Value, String> {
        let required = ["title", "description", "acceptance_criteria"];
        let mut missing = Vec::new();
        for field in required {
            if input.get(field).and_then(serde_json::Value::as_str)
                .is_none_or(|value| value.trim().is_empty())
            {
                missing.push(field);
            }
        }
        Ok(serde_json::json!({
            "check": "task_contract",
            "valid": missing.is_empty(),
            "missing_fields": missing,
            "message": if missing.is_empty() {
                "Task has a title, description, and acceptance criteria."
            } else {
                "Task is incomplete; fill in the missing contract fields before execution."
            }
        }))
    }
}

/// Synchronous tool contract. Implementations should be short-running and side-effect
/// free unless their capability is explicitly classified and approved.
pub(crate) trait CapabilityHandler: Send + Sync {
    fn execute(&self, input: &serde_json::Value) -> Result<serde_json::Value, String>;
}

#[derive(Clone)]
struct RegisteredCapability {
    spec: CapabilitySpec,
    handler: Arc<dyn CapabilityHandler>,
}

/// In-process allow-list. Unknown capability IDs are never dispatched.
#[derive(Default, Clone)]
pub(crate) struct CapabilityRegistry {
    entries: HashMap<String, RegisteredCapability>,
}

impl CapabilityRegistry {
    pub(crate) fn register(
        &mut self,
        spec: CapabilitySpec,
        handler: Arc<dyn CapabilityHandler>,
    ) -> Result<(), String> {
        spec.validate()?;
        if self.entries.contains_key(&spec.id) {
            return Err(format!("capability '{}' is already registered", spec.id));
        }
        self.entries.insert(spec.id.clone(), RegisteredCapability { spec, handler });
        Ok(())
    }

    pub(crate) fn list(&self) -> Vec<CapabilitySpec> {
        let mut specs: Vec<_> = self.entries.values().map(|entry| entry.spec.clone()).collect();
        specs.sort_by(|a, b| a.id.cmp(&b.id));
        specs
    }

    pub(crate) fn execute(
        &self,
        capability_id: &str,
        input: &serde_json::Value,
        explicitly_approved: bool,
    ) -> Result<serde_json::Value, String> {
        let entry = self.entries.get(capability_id)
            .ok_or_else(|| format!("capability '{capability_id}' is not registered"))?;
        entry.spec.validate_input(input)?;
        if entry.spec.requires_approval && !explicitly_approved {
            return Err(format!("capability '{capability_id}' requires explicit approval"));
        }
        if entry.spec.risk == CapabilityRisk::ExternalSideEffect && !explicitly_approved {
            return Err(format!("external side effect '{capability_id}' was not approved"));
        }
        let output = entry.handler.execute(input)?;
        let bytes = serde_json::to_vec(&output)
            .map_err(|error| format!("cannot encode capability output: {error}"))?.len();
        if bytes > MAX_INPUT_BYTES {
            return Err(format!("capability output exceeds {MAX_INPUT_BYTES} bytes"));
        }
        Ok(output)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct WorkflowStep {
    pub(crate) capability_id: String,
    pub(crate) input: serde_json::Value,
    /// Side-effect capabilities are blocked unless this flag is set by an explicit
    /// user approval flow; plans loaded from disk must not be trusted blindly.
    pub(crate) approved: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct WorkflowPlan {
    pub(crate) name: String,
    pub(crate) steps: Vec<WorkflowStep>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct WorkflowExecutionReport {
    pub(crate) workflow_name: String,
    pub(crate) completed_steps: usize,
    pub(crate) outputs: Vec<serde_json::Value>,
    pub(crate) failed_step: Option<usize>,
    pub(crate) error: Option<String>,
}

/// A bounded, local workflow output with explicit provenance.
/// Artifacts are records only; creating one does not publish or transmit its content.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct WorkflowArtifact {
    pub(crate) id: String,
    pub(crate) run_id: String,
    pub(crate) name: String,
    pub(crate) media_type: String,
    pub(crate) content: String,
    pub(crate) created_at: String,
}

impl WorkflowArtifact {
    pub(crate) const MAX_CONTENT_BYTES: usize = 48 * 1024;

    pub(crate) fn new(
        run_id: impl Into<String>,
        name: impl Into<String>,
        media_type: impl Into<String>,
        content: impl Into<String>,
    ) -> Result<Self, String> {
        let artifact = Self {
            id: Uuid::new_v4().to_string(),
            run_id: run_id.into().trim().to_string(),
            name: name.into().trim().to_string(),
            media_type: media_type.into().trim().to_ascii_lowercase(),
            content: content.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        artifact.validate()?;
        Ok(artifact)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.run_id.is_empty() || self.run_id.len() > 128 {
            return Err("artifact run_id must contain 1-128 bytes".into());
        }
        if self.name.is_empty() || self.name.chars().count() > 160
            || self.name.chars().any(char::is_control)
        {
            return Err("artifact name must contain 1-160 printable characters".into());
        }
        if !matches!(self.media_type.as_str(),
            "text/plain" | "text/markdown" | "application/json")
        {
            return Err("artifact media type must be text/plain, text/markdown, or application/json".into());
        }
        if self.content.len() > Self::MAX_CONTENT_BYTES {
            return Err(format!("artifact content exceeds {} bytes", Self::MAX_CONTENT_BYTES));
        }
        if self.created_at.trim().is_empty() || self.created_at.len() > 64 {
            return Err("artifact timestamp must contain 1-64 bytes".into());
        }
        if self.media_type == "application/json" {
            serde_json::from_str::<serde_json::Value>(&self.content)
                .map_err(|error| format!("JSON artifact is invalid: {error}"))?;
        }
        Ok(())
    }
}

/// Executes only the finite, ordered plan supplied by the caller. It does not
/// generate commands, retry implicitly, or execute unregistered tools.
pub(crate) struct WorkflowRunner<'a> {
    registry: &'a CapabilityRegistry,
    max_steps: usize,
}

impl<'a> WorkflowRunner<'a> {
    pub(crate) fn new(registry: &'a CapabilityRegistry, max_steps: usize) -> Result<Self, String> {
        if max_steps == 0 || max_steps > 256 {
            return Err("workflow max_steps must be between 1 and 256".into());
        }
        Ok(Self { registry, max_steps })
    }

    pub(crate) fn run(&self, plan: &WorkflowPlan) -> Result<WorkflowExecutionReport, String> {
        if plan.name.trim().is_empty() || plan.name.chars().count() > 160 {
            return Err("workflow plan name must contain 1-160 characters".into());
        }
        if plan.steps.is_empty() || plan.steps.len() > self.max_steps {
            return Err(format!("workflow must contain 1-{} steps", self.max_steps));
        }
        let mut outputs = Vec::with_capacity(plan.steps.len());
        for (index, step) in plan.steps.iter().enumerate() {
            match self.registry.execute(&step.capability_id, &step.input, step.approved) {
                Ok(output) => outputs.push(output),
                Err(error) => {
                    return Ok(WorkflowExecutionReport {
                        workflow_name: plan.name.clone(),
                        completed_steps: outputs.len(),
                        outputs,
                        failed_step: Some(index),
                        error: Some(error),
                    });
                }
            }
        }
        Ok(WorkflowExecutionReport {
            workflow_name: plan.name.clone(),
            completed_steps: outputs.len(),
            outputs,
            failed_step: None,
            error: None,
        })
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

    struct EchoHandler;
    impl CapabilityHandler for EchoHandler {
        fn execute(&self, input: &serde_json::Value) -> Result<serde_json::Value, String> {
            Ok(input.clone())
        }
    }

    #[test]
    fn runner_executes_allowlisted_steps_in_order_and_stops_on_error() {
        let mut registry = CapabilityRegistry::default();
        registry.register(
            CapabilitySpec::new("echo", "Return input for test", CapabilityRisk::ReadOnly, false, 1024).unwrap(),
            Arc::new(EchoHandler),
        ).unwrap();
        let runner = WorkflowRunner::new(&registry, 4).unwrap();
        let plan = WorkflowPlan {
            name: "test".into(),
            steps: vec![
                WorkflowStep { capability_id: "echo".into(), input: serde_json::json!({"n":1}), approved: false },
                WorkflowStep { capability_id: "missing".into(), input: serde_json::json!({}), approved: false },
                WorkflowStep { capability_id: "echo".into(), input: serde_json::json!({"n":3}), approved: false },
            ],
        };
        let report = runner.run(&plan).unwrap();
        assert_eq!(report.completed_steps, 1);
        assert_eq!(report.failed_step, Some(1));
        assert!(report.error.as_deref().unwrap().contains("not registered"));
    }

    #[test]
    fn runner_requires_explicit_approval_for_external_side_effects() {
        let mut registry = CapabilityRegistry::default();
        registry.register(
            CapabilitySpec::new("publish", "Publish externally", CapabilityRisk::ExternalSideEffect, true, 1024).unwrap(),
            Arc::new(EchoHandler),
        ).unwrap();
        let runner = WorkflowRunner::new(&registry, 2).unwrap();
        let plan = WorkflowPlan {
            name: "approval-test".into(),
            steps: vec![WorkflowStep { capability_id: "publish".into(), input: serde_json::json!({}), approved: false }],
        };
        let report = runner.run(&plan).unwrap();
        assert_eq!(report.completed_steps, 0);
        assert!(report.error.as_deref().unwrap().contains("requires explicit approval"));
    }

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
            "read_note", "Read one note", CapabilityRisk::ReadOnly, false, 16
        ).expect("valid capability");
        assert!(capability.validate_input(&serde_json::json!({"q":"ok"})).is_ok());
        assert!(capability.validate_input(&serde_json::json!({"q":"this is too long"})).is_err());
    }

    #[test]
    fn workflow_transition_matrix_rejects_undeclared_edges() {
        use WorkflowStatus::*;

        let states = [
            Planned, Running, Checkpointed, Validating, Submitted,
            Accepted, Rejected, Failed, Cancelled,
        ];
        let allowed: &[(WorkflowStatus, WorkflowStatus)] = &[
            (Planned, Running), (Planned, Cancelled),
            (Running, Checkpointed), (Running, Validating),
            (Running, Failed), (Running, Cancelled),
            (Checkpointed, Running), (Checkpointed, Validating),
            (Checkpointed, Failed), (Checkpointed, Cancelled),
            (Validating, Running), (Validating, Submitted),
            (Validating, Rejected), (Validating, Failed), (Validating, Cancelled),
            (Submitted, Accepted), (Submitted, Rejected),
            (Rejected, Running), (Rejected, Cancelled),
        ];

        for from in states {
            for to in states {
                let expected = allowed.iter().any(|(source, target)| *source == from && *target == to);
                assert_eq!(
                    from.can_transition_to(&to),
                    expected,
                    "unexpected workflow transition: {} -> {}",
                    from.as_str(),
                    to.as_str(),
                );
            }
        }
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
