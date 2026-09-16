//! `MultiAgentEngine` — orquestador de equipos y workflows.

use crate::{
    agent::{Agent, AgentOutcome, FinishKind},
    error::{AgentError, AgentResult},
    identity::AgentRole,
    step::{Step, StepKind, StepOutcome},
    team::{Team, TeamMember},
    workflow::{Workflow, WorkflowStatus},
};
use std::collections::BTreeMap;

/// Estado de ejecución de un workflow.
#[derive(Debug, Clone)]
pub struct TeamExecution {
    /// Workflow ejecutado.
    pub workflow: Workflow,
    /// Resultados por paso (índice → outcome).
    pub step_results: BTreeMap<usize, AgentOutcome>,
    /// Trazabilidad de la ejecución completa.
    pub trace: Vec<Step>,
    /// ¿Se completó con éxito?
    pub success: bool,
}

impl TeamExecution {
    /// Número de pasos ejecutados con éxito.
    pub fn successful_steps(&self) -> usize {
        self.step_results.values().filter(|o| o.success).count()
    }

    /// Número de pasos fallidos.
    pub fn failed_steps(&self) -> usize {
        self.step_results.values().filter(|o| !o.success).count()
    }

    /// Resumen textual.
    pub fn summary(&self) -> String {
        format!(
            "workflow '{}' [{}]: {}/{} steps ok, {} failed",
            self.workflow.name,
            self.workflow.status.display_name(),
            self.successful_steps(),
            self.workflow.step_count(),
            self.failed_steps(),
        )
    }
}

/// Motor multi-agente.
pub struct MultiAgentEngine {
    agents: BTreeMap<AgentRole, Box<dyn Agent>>,
}

impl std::fmt::Debug for MultiAgentEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiAgentEngine")
            .field("agents", &self.agents.len())
            .finish()
    }
}

impl Default for MultiAgentEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiAgentEngine {
    pub fn new() -> Self {
        Self {
            agents: BTreeMap::new(),
        }
    }

    /// Registra un agente. Si ya hay uno con su rol, lo reemplaza.
    pub fn register(&mut self, agent: Box<dyn Agent>) {
        let role = agent.identity().role;
        self.agents.insert(role, agent);
    }

    /// Número de agentes registrados.
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    /// ¿Tiene agente para este rol?
    pub fn has_role(&self, role: AgentRole) -> bool {
        self.agents.contains_key(&role)
    }

    /// Roles disponibles.
    pub fn roles(&self) -> Vec<AgentRole> {
        self.agents.keys().copied().collect()
    }

    /// Ejecuta un workflow usando los agentes registrados.
    pub fn execute(&self, mut workflow: Workflow) -> AgentResult<TeamExecution> {
        if workflow.is_empty() {
            return Err(AgentError::InvalidConfig("workflow has no steps".into()));
        }

        // Verificamos que todos los roles estén cubiertos.
        for step in &workflow.steps {
            if !self.has_role(step.role) {
                return Err(AgentError::InvalidConfig(format!(
                    "no agent registered for role {:?}",
                    step.role
                )));
            }
        }

        workflow.set_status(WorkflowStatus::Running);

        let mut trace: Vec<Step> = Vec::new();
        let mut step_results: BTreeMap<usize, AgentOutcome> = BTreeMap::new();

        trace.push(Step::new(
            StepKind::Plan,
            StepOutcome::Success,
            format!(
                "executing workflow '{}' ({} steps)",
                workflow.name,
                workflow.step_count()
            ),
        ));

        for (idx, step) in workflow.steps.iter().enumerate() {
            let agent = self.agents.get(&step.role).unwrap();

            trace.push(Step::new(
                StepKind::Act,
                StepOutcome::Success,
                format!(
                    "step {} → agent '{}' (role {:?})",
                    idx,
                    agent.identity().name,
                    step.role
                ),
            ));

            match agent.execute(&step.description) {
                Ok(outcome) => {
                    let success = outcome.success;
                    step_results.insert(idx, outcome.clone());

                    let outcome_kind = if success {
                        StepOutcome::Success
                    } else {
                        StepOutcome::Failure
                    };

                    trace.push(Step::new(
                        StepKind::Act,
                        outcome_kind,
                        format!(
                            "step {} {} ({} sub-steps)",
                            idx,
                            if success { "completed" } else { "failed" },
                            outcome.steps.len()
                        ),
                    ));

                    if !success && !step.continue_on_failure {
                        workflow.set_status(WorkflowStatus::Failed);
                        trace.push(Step::new(
                            StepKind::Finish,
                            StepOutcome::Failure,
                            format!("workflow aborted at step {}", idx),
                        ));
                        return Ok(TeamExecution {
                            workflow,
                            step_results,
                            trace,
                            success: false,
                        });
                    }
                }
                Err(e) => {
                    trace.push(Step::new(
                        StepKind::Act,
                        StepOutcome::Failure,
                        format!("step {} error: {}", idx, e),
                    ));

                    if !step.continue_on_failure {
                        workflow.set_status(WorkflowStatus::Failed);
                        return Ok(TeamExecution {
                            workflow,
                            step_results,
                            trace,
                            success: false,
                        });
                    }
                }
            }
        }

        workflow.set_status(WorkflowStatus::Completed);

        trace.push(Step::new(
            StepKind::Finish,
            StepOutcome::Success,
            format!("workflow '{}' completed", workflow.name),
        ));

        Ok(TeamExecution {
            workflow,
            step_results,
            trace,
            success: true,
        })
    }

    /// Construye un `Team` a partir de los agentes registrados.
    pub fn as_team(&self, name: impl Into<String>) -> Team {
        let mut team = Team::new(name);
        for (role, agent) in &self.agents {
            team.set_member(TeamMember::new(*role, agent.identity().name.clone()));
        }
        team
    }
}

// Permite convertir una `TeamExecution` en un `AgentOutcome` completo.
impl From<&TeamExecution> for AgentOutcome {
    fn from(exec: &TeamExecution) -> Self {
        AgentOutcome {
            response: exec.summary(),
            steps: exec.trace.clone(),
            success: exec.success,
            finish_reason: if exec.success {
                FinishKind::Completed
            } else {
                FinishKind::GaveUp
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        echo::EchoAgent,
        identity::AgentIdentity,
        workflow::{Workflow, WorkflowStep},
    };

    fn echo_for(role: AgentRole) -> Box<dyn Agent> {
        let identity = AgentIdentity::new(format!("echo-{:?}", role), role);
        Box::new(EchoAgent::new().with_identity(identity))
    }

    fn setup_engine() -> MultiAgentEngine {
        let mut engine = MultiAgentEngine::new();
        engine.register(echo_for(AgentRole::Planner));
        engine.register(echo_for(AgentRole::Coder));
        engine.register(echo_for(AgentRole::Reviewer));
        engine.register(echo_for(AgentRole::Tester));
        engine
    }

    #[test]
    fn new_engine_is_empty() {
        let e = MultiAgentEngine::new();
        assert_eq!(e.agent_count(), 0);
    }

    #[test]
    fn register_adds_agent() {
        let mut e = MultiAgentEngine::new();
        e.register(echo_for(AgentRole::Coder));
        assert_eq!(e.agent_count(), 1);
        assert!(e.has_role(AgentRole::Coder));
    }

    #[test]
    fn register_replaces_same_role() {
        let mut e = MultiAgentEngine::new();
        e.register(echo_for(AgentRole::Coder));
        e.register(echo_for(AgentRole::Coder));
        assert_eq!(e.agent_count(), 1);
    }

    #[test]
    fn roles_list() {
        let e = setup_engine();
        let roles = e.roles();
        assert_eq!(roles.len(), 4);
        assert!(roles.contains(&AgentRole::Planner));
        assert!(roles.contains(&AgentRole::Coder));
    }

    #[test]
    fn execute_empty_workflow_fails() {
        let e = setup_engine();
        let w = Workflow::new("empty", "nothing");
        let err = e.execute(w).unwrap_err();
        assert!(matches!(err, AgentError::InvalidConfig(_)));
    }

    #[test]
    fn execute_workflow_missing_role_fails() {
        let mut e = MultiAgentEngine::new();
        e.register(echo_for(AgentRole::Coder));

        let w = Workflow::new("w", "g").add_step(WorkflowStep::new("plan", AgentRole::Planner));

        let err = e.execute(w).unwrap_err();
        assert!(matches!(err, AgentError::InvalidConfig(_)));
    }

    #[test]
    fn execute_simple_workflow() {
        let e = setup_engine();
        let w = Workflow::new("build feature", "build a hello world")
            .add_step(WorkflowStep::new("plan the work", AgentRole::Planner))
            .add_step(WorkflowStep::new("write code", AgentRole::Coder))
            .add_step(WorkflowStep::new("review code", AgentRole::Reviewer));

        let exec = e.execute(w).unwrap();
        assert!(exec.success);
        assert_eq!(exec.workflow.status, WorkflowStatus::Completed);
        assert_eq!(exec.successful_steps(), 3);
        assert_eq!(exec.failed_steps(), 0);
        assert_eq!(exec.step_results.len(), 3);
    }

    #[test]
    fn execute_workflow_trace_contains_plan_and_finish() {
        let e = setup_engine();
        let w = Workflow::new("w", "g").add_step(WorkflowStep::new("a", AgentRole::Coder));

        let exec = e.execute(w).unwrap();
        let kinds: Vec<StepKind> = exec.trace.iter().map(|s| s.kind).collect();
        assert!(kinds.contains(&StepKind::Plan));
        assert!(kinds.contains(&StepKind::Act));
        assert!(kinds.contains(&StepKind::Finish));
    }

    #[test]
    fn summary_contains_info() {
        let e = setup_engine();
        let w =
            Workflow::new("test-workflow", "g").add_step(WorkflowStep::new("a", AgentRole::Coder));

        let exec = e.execute(w).unwrap();
        let s = exec.summary();
        assert!(s.contains("test-workflow"));
        assert!(s.contains("completed"));
        assert!(s.contains("1/1"));
    }

    #[test]
    fn as_team_works() {
        let e = setup_engine();
        let team = e.as_team("my-team");
        assert_eq!(team.name, "my-team");
        assert_eq!(team.len(), 4);
    }

    #[test]
    fn workflow_with_continue_on_failure_keeps_going() {
        let e = setup_engine();
        let w = Workflow::new("w", "g")
            .add_step(WorkflowStep::new("might fail", AgentRole::Coder).continue_on_failure())
            .add_step(WorkflowStep::new("always runs", AgentRole::Reviewer));

        let exec = e.execute(w).unwrap();
        assert!(exec.success);
    }

    #[test]
    fn team_execution_converts_to_agent_outcome() {
        let e = setup_engine();
        let w = Workflow::new("w", "g").add_step(WorkflowStep::new("a", AgentRole::Coder));

        let exec = e.execute(w).unwrap();
        let outcome: AgentOutcome = (&exec).into();
        assert!(outcome.success);
        assert_eq!(outcome.finish_reason, FinishKind::Completed);
        assert!(!outcome.steps.is_empty());
    }
}
