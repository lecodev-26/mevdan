//! `MultiAgentEngine` — orquestador de equipos y workflows.

use crate::{
    agent::{Agent, AgentOutcome, FinishKind},
    error::{AgentError, AgentResult},
    handoff::{Handoff, HandoffHistory, HandoffRequest},
    identity::AgentRole,
    parallel::{ParallelExecutor, ParallelGroup, ParallelOutcome},
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
    handoff_history: HandoffHistory,
}

impl std::fmt::Debug for MultiAgentEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiAgentEngine")
            .field("agents", &self.agents.len())
            .field("handoffs", &self.handoff_history.len())
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
            handoff_history: HandoffHistory::new(),
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

    // ─────────────────────────────────────────────
    // Handoff
    // ─────────────────────────────────────────────

    /// ¿Se puede hacer handoff entre dos roles?
    pub fn can_handoff(&self, from: AgentRole, to: AgentRole) -> bool {
        self.has_role(from) && self.has_role(to)
    }

    /// Registra un handoff entre dos agentes.
    pub fn handoff(&mut self, request: HandoffRequest) -> AgentResult<Handoff> {
        if !self.has_role(request.from_role) {
            return Err(AgentError::InvalidConfig(format!(
                "no agent registered for from-role {:?}",
                request.from_role
            )));
        }
        if !self.has_role(request.to_role) {
            return Err(AgentError::InvalidConfig(format!(
                "no agent registered for to-role {:?}",
                request.to_role
            )));
        }

        let handoff = Handoff::from_request(request);
        self.handoff_history.record(handoff.clone());
        Ok(handoff)
    }

    /// Historial de handoffs.
    pub fn handoff_history(&self) -> &HandoffHistory {
        &self.handoff_history
    }

    /// ¿Se ha hecho algún handoff?
    pub fn has_handoffs(&self) -> bool {
        !self.handoff_history.is_empty()
    }

    /// Último handoff (si hay).
    pub fn last_handoff(&self) -> Option<&Handoff> {
        self.handoff_history.last()
    }

    /// Número de handoffs registrados.
    pub fn handoff_count(&self) -> usize {
        self.handoff_history.len()
    }

    // ─────────────────────────────────────────────
    // Ejecución secuencial
    // ─────────────────────────────────────────────

    /// Ejecuta un workflow usando los agentes registrados.
    pub fn execute(&self, mut workflow: Workflow) -> AgentResult<TeamExecution> {
        if workflow.is_empty() {
            return Err(AgentError::InvalidConfig("workflow has no steps".into()));
        }

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

    // ─────────────────────────────────────────────
    // Ejecución paralela
    // ─────────────────────────────────────────────

    /// Ejecuta un grupo paralelo.
    ///
    /// **Nota V4:** ejecución secuencial en el orden de las tareas.
    /// La API está preparada para paralelización real en V5.
    pub fn execute_parallel(&self, group: ParallelGroup) -> AgentResult<ParallelOutcome> {
        let executor = ParallelExecutor::new(&self.agents);
        executor.execute(group)
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
        handoff::HandoffReason,
        identity::AgentIdentity,
        parallel::{ParallelConfig, ParallelGroup, ParallelTask},
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

    // ─────────────────────────────────────────────
    // Motor básico
    // ─────────────────────────────────────────────

    #[test]
    fn new_engine_is_empty() {
        let e = MultiAgentEngine::new();
        assert_eq!(e.agent_count(), 0);
        assert!(!e.has_handoffs());
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

    // ─────────────────────────────────────────────
    // Handoff
    // ─────────────────────────────────────────────

    #[test]
    fn can_handoff_requires_both_roles() {
        let e = setup_engine();
        assert!(e.can_handoff(AgentRole::Planner, AgentRole::Coder));
        assert!(!e.can_handoff(AgentRole::Planner, AgentRole::Documenter));
    }

    #[test]
    fn handoff_registers_and_returns() {
        let mut e = setup_engine();
        let req = HandoffRequest::new(
            AgentRole::Planner,
            "planner-1",
            AgentRole::Coder,
            "coder-1",
            HandoffReason::Cost,
        );

        let handoff = e.handoff(req).unwrap();
        assert_eq!(handoff.from_role, AgentRole::Planner);
        assert_eq!(handoff.to_role, AgentRole::Coder);
        assert_eq!(handoff.reason, HandoffReason::Cost);
        assert!(e.has_handoffs());
        assert_eq!(e.handoff_count(), 1);
    }

    #[test]
    fn handoff_fails_without_from_role() {
        let mut e = setup_engine();
        let req = HandoffRequest::new(
            AgentRole::Documenter,
            "doc-1",
            AgentRole::Coder,
            "coder-1",
            HandoffReason::Manual,
        );
        let err = e.handoff(req).unwrap_err();
        assert!(matches!(err, AgentError::InvalidConfig(_)));
    }

    #[test]
    fn handoff_fails_without_to_role() {
        let mut e = setup_engine();
        let req = HandoffRequest::new(
            AgentRole::Planner,
            "planner-1",
            AgentRole::Documenter,
            "doc-1",
            HandoffReason::Manual,
        );
        let err = e.handoff(req).unwrap_err();
        assert!(matches!(err, AgentError::InvalidConfig(_)));
    }

    #[test]
    fn last_handoff_returns_most_recent() {
        let mut e = setup_engine();
        e.handoff(HandoffRequest::new(
            AgentRole::Planner,
            "p",
            AgentRole::Coder,
            "c",
            HandoffReason::Cost,
        ))
        .unwrap();
        e.handoff(HandoffRequest::new(
            AgentRole::Coder,
            "c",
            AgentRole::Reviewer,
            "r",
            HandoffReason::Quality,
        ))
        .unwrap();

        assert_eq!(e.last_handoff().unwrap().reason, HandoffReason::Quality);
        assert_eq!(e.handoff_count(), 2);
    }

    #[test]
    fn handoff_history_accessible() {
        let mut e = setup_engine();
        e.handoff(HandoffRequest::new(
            AgentRole::Planner,
            "p",
            AgentRole::Coder,
            "c",
            HandoffReason::Manual,
        ))
        .unwrap();

        let history = e.handoff_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history.by_reason(HandoffReason::Manual).len(), 1);
    }

    #[test]
    fn full_flow_handoff_sequence() {
        let mut e = setup_engine();

        e.handoff(HandoffRequest::new(
            AgentRole::Planner,
            "gpt-4o",
            AgentRole::Coder,
            "llama3.2",
            HandoffReason::Cost,
        ))
        .unwrap();

        e.handoff(HandoffRequest::new(
            AgentRole::Coder,
            "llama3.2",
            AgentRole::Reviewer,
            "claude-sonnet",
            HandoffReason::Quality,
        ))
        .unwrap();

        e.handoff(HandoffRequest::new(
            AgentRole::Reviewer,
            "claude-sonnet",
            AgentRole::Tester,
            "local-tester",
            HandoffReason::Manual,
        ))
        .unwrap();

        assert_eq!(e.handoff_count(), 3);
        assert_eq!(
            e.handoff_history()
                .involving_role(AgentRole::Reviewer)
                .len(),
            2
        );
        assert_eq!(e.handoff_history().by_reason(HandoffReason::Cost).len(), 1);
    }

    // ─────────────────────────────────────────────
    // Parallel
    // ─────────────────────────────────────────────

    #[test]
    fn execute_parallel_empty_group_fails() {
        let e = setup_engine();
        let g = ParallelGroup::new("empty");
        let err = e.execute_parallel(g).unwrap_err();
        assert!(matches!(err, AgentError::InvalidConfig(_)));
    }

    #[test]
    fn execute_parallel_missing_role_fails() {
        let e = setup_engine();
        let g =
            ParallelGroup::new("g").with_task(ParallelTask::new("a", "x", AgentRole::Documenter));
        let err = e.execute_parallel(g).unwrap_err();
        assert!(matches!(err, AgentError::InvalidConfig(_)));
    }

    #[test]
    fn execute_parallel_simple() {
        let e = setup_engine();
        let g = ParallelGroup::new("build")
            .with_config(ParallelConfig::new().with_max_parallel(2))
            .with_task(ParallelTask::new("code", "write code", AgentRole::Coder))
            .with_task(ParallelTask::new(
                "review",
                "review code",
                AgentRole::Reviewer,
            ));

        let outcome = e.execute_parallel(g).unwrap();
        assert!(outcome.success);
        assert_eq!(outcome.results.len(), 2);
        assert_eq!(outcome.successful(), 2);
    }

    #[test]
    fn execute_parallel_three_tasks() {
        let e = setup_engine();
        let g = ParallelGroup::new("feature")
            .with_task(ParallelTask::new(
                "auth",
                "implement authentication",
                AgentRole::Coder,
            ))
            .with_task(ParallelTask::new("tests", "write tests", AgentRole::Tester))
            .with_task(ParallelTask::new(
                "review",
                "review code",
                AgentRole::Reviewer,
            ));

        let outcome = e.execute_parallel(g).unwrap();
        assert!(outcome.success);
        assert_eq!(outcome.successful(), 3);
        assert_eq!(outcome.failed(), 0);
    }

    #[test]
    fn parallel_summary_contains_info() {
        let e = setup_engine();
        let g =
            ParallelGroup::new("my-group").with_task(ParallelTask::new("a", "x", AgentRole::Coder));

        let outcome = e.execute_parallel(g).unwrap();
        let s = outcome.summary();
        assert!(s.contains("my-group"));
        assert!(s.contains("completed"));
    }
}
