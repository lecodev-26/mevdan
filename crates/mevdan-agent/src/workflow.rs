//! `Workflow` — secuencia ordenada de pasos con roles asignados.

use crate::identity::AgentRole;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// ID de un workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkflowId(pub Uuid);

impl WorkflowId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for WorkflowId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WorkflowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Estado de un workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Aborted,
}

impl WorkflowStatus {
    pub fn display_name(&self) -> &'static str {
        match self {
            WorkflowStatus::Pending => "pending",
            WorkflowStatus::Running => "running",
            WorkflowStatus::Completed => "completed",
            WorkflowStatus::Failed => "failed",
            WorkflowStatus::Aborted => "aborted",
        }
    }
}

/// Un paso del workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Descripción de qué hay que hacer.
    pub description: String,
    /// Rol del agente que debe ejecutarlo.
    pub role: AgentRole,
    /// Si `true`, el workflow continúa aunque este paso falle.
    #[serde(default)]
    pub continue_on_failure: bool,
}

impl WorkflowStep {
    pub fn new(description: impl Into<String>, role: AgentRole) -> Self {
        Self {
            description: description.into(),
            role,
            continue_on_failure: false,
        }
    }

    pub fn continue_on_failure(mut self) -> Self {
        self.continue_on_failure = true;
        self
    }
}

/// Un workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: WorkflowId,
    pub name: String,
    pub goal: String,
    pub steps: Vec<WorkflowStep>,
    pub status: WorkflowStatus,
}

impl Workflow {
    pub fn new(name: impl Into<String>, goal: impl Into<String>) -> Self {
        Self {
            id: WorkflowId::new(),
            name: name.into(),
            goal: goal.into(),
            steps: Vec::new(),
            status: WorkflowStatus::Pending,
        }
    }

    /// Añade un paso.
    pub fn add_step(mut self, step: WorkflowStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Roles únicos requeridos por el workflow.
    pub fn required_roles(&self) -> Vec<AgentRole> {
        let mut set = std::collections::BTreeSet::new();
        for s in &self.steps {
            set.insert(s.role);
        }
        set.into_iter().collect()
    }

    /// Cambia el estado.
    pub fn set_status(&mut self, status: WorkflowStatus) {
        self.status = status;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_workflow_defaults() {
        let w = Workflow::new("test", "do something");
        assert_eq!(w.name, "test");
        assert_eq!(w.goal, "do something");
        assert_eq!(w.status, WorkflowStatus::Pending);
        assert!(w.is_empty());
    }

    #[test]
    fn add_step_increments_count() {
        let w = Workflow::new("w", "g")
            .add_step(WorkflowStep::new("step 1", AgentRole::Planner))
            .add_step(WorkflowStep::new("step 2", AgentRole::Coder));
        assert_eq!(w.step_count(), 2);
    }

    #[test]
    fn required_roles_unique() {
        let w = Workflow::new("w", "g")
            .add_step(WorkflowStep::new("a", AgentRole::Coder))
            .add_step(WorkflowStep::new("b", AgentRole::Coder))
            .add_step(WorkflowStep::new("c", AgentRole::Reviewer));
        let roles = w.required_roles();
        assert_eq!(roles.len(), 2);
        assert!(roles.contains(&AgentRole::Coder));
        assert!(roles.contains(&AgentRole::Reviewer));
    }

    #[test]
    fn continue_on_failure_flag() {
        let step = WorkflowStep::new("risky", AgentRole::Coder).continue_on_failure();
        assert!(step.continue_on_failure);

        let step = WorkflowStep::new("risky", AgentRole::Coder);
        assert!(!step.continue_on_failure);
    }

    #[test]
    fn set_status_works() {
        let mut w = Workflow::new("w", "g");
        w.set_status(WorkflowStatus::Running);
        assert_eq!(w.status, WorkflowStatus::Running);
        w.set_status(WorkflowStatus::Completed);
        assert_eq!(w.status, WorkflowStatus::Completed);
    }

    #[test]
    fn status_display_names() {
        assert_eq!(WorkflowStatus::Pending.display_name(), "pending");
        assert_eq!(WorkflowStatus::Running.display_name(), "running");
        assert_eq!(WorkflowStatus::Completed.display_name(), "completed");
        assert_eq!(WorkflowStatus::Failed.display_name(), "failed");
        assert_eq!(WorkflowStatus::Aborted.display_name(), "aborted");
    }

    #[test]
    fn status_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&WorkflowStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&WorkflowStatus::Failed).unwrap(),
            "\"failed\""
        );
    }

    #[test]
    fn workflow_id_unique() {
        let a = WorkflowId::new();
        let b = WorkflowId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn workflow_serializes() {
        let w = Workflow::new("w", "g").add_step(WorkflowStep::new("a", AgentRole::Coder));
        let json = serde_json::to_string(&w).unwrap();
        let back: Workflow = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "w");
        assert_eq!(back.step_count(), 1);
    }
}
