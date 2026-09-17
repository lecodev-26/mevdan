//! Parallel Agents — ejecución de tareas independientes.
//!
//! Un **grupo paralelo** contiene tareas que no dependen entre sí.
//! Pueden ejecutarse a la vez sobre el mismo proyecto.
//!
//! ## Estado de implementación
//!
//! **V4.10** define la API y ejecuta **secuencialmente**. La API está
//! lista para cambiar a threads reales en V5 sin tocar el resto del
//! código.

use crate::{
    agent::{Agent, AgentOutcome},
    error::{AgentError, AgentResult},
    identity::AgentRole,
    step::{Step, StepKind, StepOutcome},
    workflow::WorkflowStatus,
};
use serde::{Deserialize, Serialize};

/// Una tarea paralelizable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelTask {
    /// Descripción de la tarea.
    pub description: String,
    /// Rol del agente que la ejecuta.
    pub role: AgentRole,
    /// Etiqueta corta para identificar la tarea.
    pub label: String,
    /// Si `true`, el grupo continúa aunque esta tarea falle.
    #[serde(default)]
    pub continue_on_failure: bool,
}

impl ParallelTask {
    pub fn new(label: impl Into<String>, description: impl Into<String>, role: AgentRole) -> Self {
        Self {
            label: label.into(),
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

/// Configuración de un grupo paralelo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelConfig {
    /// Máximo de tareas a ejecutar simultáneamente.
    pub max_parallel: usize,
    /// Timeout por tarea en segundos (0 = sin límite).
    pub task_timeout_secs: u64,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            max_parallel: 4,
            task_timeout_secs: 300,
        }
    }
}

impl ParallelConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_parallel(mut self, n: usize) -> Self {
        self.max_parallel = n.max(1);
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.task_timeout_secs = secs;
        self
    }
}

/// Un grupo de tareas paralelizables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelGroup {
    pub name: String,
    pub tasks: Vec<ParallelTask>,
    pub config: ParallelConfig,
}

impl ParallelGroup {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tasks: Vec::new(),
            config: ParallelConfig::default(),
        }
    }

    pub fn with_config(mut self, config: ParallelConfig) -> Self {
        self.config = config;
        self
    }

    /// Añade una tarea al grupo.
    ///
    /// Se llama `with_task` (no `add`) para no confundirse con
    /// `std::ops::Add::add`.
    pub fn with_task(mut self, task: ParallelTask) -> Self {
        self.tasks.push(task);
        self
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Roles únicos requeridos.
    pub fn required_roles(&self) -> Vec<AgentRole> {
        let mut set = std::collections::BTreeSet::new();
        for t in &self.tasks {
            set.insert(t.role);
        }
        set.into_iter().collect()
    }
}

/// Resultado de una tarea ejecutada.
#[derive(Debug, Clone)]
pub struct ParallelTaskResult {
    pub label: String,
    pub role: AgentRole,
    pub outcome: AgentOutcome,
    /// Orden de ejecución (0-based).
    pub order: usize,
}

/// Resultado agregado del grupo.
#[derive(Debug, Clone)]
pub struct ParallelOutcome {
    pub group_name: String,
    pub status: WorkflowStatus,
    pub results: Vec<ParallelTaskResult>,
    pub trace: Vec<Step>,
    pub success: bool,
}

impl ParallelOutcome {
    pub fn successful(&self) -> usize {
        self.results.iter().filter(|r| r.outcome.success).count()
    }

    pub fn failed(&self) -> usize {
        self.results.iter().filter(|r| !r.outcome.success).count()
    }

    pub fn summary(&self) -> String {
        format!(
            "parallel group '{}' [{}]: {}/{} ok, {} failed",
            self.group_name,
            self.status.display_name(),
            self.successful(),
            self.results.len(),
            self.failed(),
        )
    }
}

/// Ejecutor de grupos paralelos.
///
/// **V4.10:** ejecución secuencial. La API está preparada para que
/// en V5 se cambie el cuerpo de `execute` por threads reales sin
/// tocar la firma ni el resto del código.
pub struct ParallelExecutor<'a> {
    agents: &'a std::collections::BTreeMap<AgentRole, Box<dyn Agent>>,
}

impl<'a> std::fmt::Debug for ParallelExecutor<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParallelExecutor")
            .field("agents", &self.agents.len())
            .finish()
    }
}

impl<'a> ParallelExecutor<'a> {
    /// Crea un ejecutor sobre los agentes registrados.
    pub fn new(agents: &'a std::collections::BTreeMap<AgentRole, Box<dyn Agent>>) -> Self {
        Self { agents }
    }

    /// Ejecuta un grupo paralelo.
    ///
    /// **Nota V4:** ejecuta secuencialmente en el orden de las tareas.
    /// En V5 se paralelizará respetando `max_parallel`.
    pub fn execute(&self, group: ParallelGroup) -> AgentResult<ParallelOutcome> {
        if group.is_empty() {
            return Err(AgentError::InvalidConfig(
                "parallel group has no tasks".into(),
            ));
        }

        for task in &group.tasks {
            if !self.agents.contains_key(&task.role) {
                return Err(AgentError::InvalidConfig(format!(
                    "no agent registered for role {:?}",
                    task.role
                )));
            }
        }

        let mut trace: Vec<Step> = Vec::new();
        trace.push(Step::new(
            StepKind::Plan,
            StepOutcome::Success,
            format!(
                "executing parallel group '{}' ({} tasks)",
                group.name,
                group.len()
            ),
        ));

        let mut results: Vec<ParallelTaskResult> = Vec::new();
        let mut had_failure = false;
        let mut abort = false;

        for (idx, task) in group.tasks.iter().enumerate() {
            if abort {
                break;
            }

            trace.push(Step::new(
                StepKind::Act,
                StepOutcome::Success,
                format!(
                    "task {} '{}' → agent for role {:?}",
                    idx, task.label, task.role
                ),
            ));

            let agent = self.agents.get(&task.role).unwrap();

            match agent.execute(&task.description) {
                Ok(outcome) => {
                    let success = outcome.success;
                    if !success {
                        had_failure = true;
                    }

                    trace.push(Step::new(
                        StepKind::Act,
                        if success {
                            StepOutcome::Success
                        } else {
                            StepOutcome::Failure
                        },
                        format!(
                            "task '{}' {} ({} sub-steps)",
                            task.label,
                            if success { "completed" } else { "failed" },
                            outcome.steps.len()
                        ),
                    ));

                    results.push(ParallelTaskResult {
                        label: task.label.clone(),
                        role: task.role,
                        outcome,
                        order: idx,
                    });

                    if !success && !task.continue_on_failure {
                        abort = true;
                    }
                }
                Err(e) => {
                    had_failure = true;
                    trace.push(Step::new(
                        StepKind::Act,
                        StepOutcome::Failure,
                        format!("task '{}' error: {}", task.label, e),
                    ));

                    if !task.continue_on_failure {
                        abort = true;
                    }
                }
            }
        }

        let success = !had_failure && !abort;
        let status = if abort || !success {
            WorkflowStatus::Failed
        } else {
            WorkflowStatus::Completed
        };

        trace.push(Step::new(
            StepKind::Finish,
            if success {
                StepOutcome::Success
            } else {
                StepOutcome::Failure
            },
            format!(
                "parallel group '{}' {}",
                group.name,
                if success { "completed" } else { "aborted" }
            ),
        ));

        Ok(ParallelOutcome {
            group_name: group.name,
            status,
            results,
            trace,
            success,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{echo::EchoAgent, identity::AgentIdentity};
    use std::collections::BTreeMap;

    fn agents_with_roles(roles: &[AgentRole]) -> BTreeMap<AgentRole, Box<dyn Agent>> {
        let mut agents: BTreeMap<AgentRole, Box<dyn Agent>> = BTreeMap::new();
        for role in roles {
            let identity = AgentIdentity::new(format!("echo-{:?}", role), *role);
            agents.insert(*role, Box::new(EchoAgent::new().with_identity(identity)));
        }
        agents
    }

    #[test]
    fn task_new_creates() {
        let t = ParallelTask::new("auth", "implement authentication", AgentRole::Coder);
        assert_eq!(t.label, "auth");
        assert_eq!(t.role, AgentRole::Coder);
        assert!(!t.continue_on_failure);
    }

    #[test]
    fn task_continue_on_failure() {
        let t = ParallelTask::new("x", "y", AgentRole::Coder).continue_on_failure();
        assert!(t.continue_on_failure);
    }

    #[test]
    fn task_serializes() {
        let t = ParallelTask::new("a", "b", AgentRole::Coder);
        let json = serde_json::to_string(&t).unwrap();
        let back: ParallelTask = serde_json::from_str(&json).unwrap();
        assert_eq!(back.label, "a");
    }

    #[test]
    fn config_defaults() {
        let c = ParallelConfig::default();
        assert_eq!(c.max_parallel, 4);
        assert_eq!(c.task_timeout_secs, 300);
    }

    #[test]
    fn config_with_max_parallel_clamps() {
        let c = ParallelConfig::new().with_max_parallel(0);
        assert_eq!(c.max_parallel, 1);
    }

    #[test]
    fn config_with_timeout() {
        let c = ParallelConfig::new().with_timeout(60);
        assert_eq!(c.task_timeout_secs, 60);
    }

    #[test]
    fn group_new_is_empty() {
        let g = ParallelGroup::new("test");
        assert_eq!(g.name, "test");
        assert!(g.is_empty());
        assert_eq!(g.len(), 0);
    }

    #[test]
    fn group_with_task() {
        let g =
            ParallelGroup::new("g").with_task(ParallelTask::new("a", "task a", AgentRole::Coder));
        assert_eq!(g.len(), 1);
    }

    #[test]
    fn group_with_config() {
        let g = ParallelGroup::new("g").with_config(ParallelConfig::new().with_max_parallel(2));
        assert_eq!(g.config.max_parallel, 2);
    }

    #[test]
    fn group_required_roles_unique() {
        let g = ParallelGroup::new("g")
            .with_task(ParallelTask::new("a", "x", AgentRole::Coder))
            .with_task(ParallelTask::new("b", "y", AgentRole::Coder))
            .with_task(ParallelTask::new("c", "z", AgentRole::Reviewer));
        let roles = g.required_roles();
        assert_eq!(roles.len(), 2);
    }

    #[test]
    fn execute_empty_group_fails() {
        let agents = agents_with_roles(&[AgentRole::Coder]);
        let exec = ParallelExecutor::new(&agents);
        let g = ParallelGroup::new("empty");
        let err = exec.execute(g).unwrap_err();
        assert!(matches!(err, AgentError::InvalidConfig(_)));
    }

    #[test]
    fn execute_missing_role_fails() {
        let agents = agents_with_roles(&[AgentRole::Coder]);
        let exec = ParallelExecutor::new(&agents);
        let g =
            ParallelGroup::new("g").with_task(ParallelTask::new("a", "task", AgentRole::Reviewer));
        let err = exec.execute(g).unwrap_err();
        assert!(matches!(err, AgentError::InvalidConfig(_)));
    }

    #[test]
    fn execute_simple_group() {
        let agents = agents_with_roles(&[AgentRole::Coder, AgentRole::Reviewer]);
        let exec = ParallelExecutor::new(&agents);
        let g = ParallelGroup::new("build")
            .with_task(ParallelTask::new("a", "write code", AgentRole::Coder))
            .with_task(ParallelTask::new("b", "review code", AgentRole::Reviewer));

        let outcome = exec.execute(g).unwrap();
        assert!(outcome.success);
        assert_eq!(outcome.results.len(), 2);
        assert_eq!(outcome.successful(), 2);
        assert_eq!(outcome.failed(), 0);
        assert_eq!(outcome.status, WorkflowStatus::Completed);
    }

    #[test]
    fn execute_preserves_order() {
        let agents = agents_with_roles(&[AgentRole::Coder, AgentRole::Reviewer]);
        let exec = ParallelExecutor::new(&agents);
        let g = ParallelGroup::new("g")
            .with_task(ParallelTask::new("first", "x", AgentRole::Coder))
            .with_task(ParallelTask::new("second", "y", AgentRole::Reviewer));

        let outcome = exec.execute(g).unwrap();
        assert_eq!(outcome.results[0].label, "first");
        assert_eq!(outcome.results[1].label, "second");
        assert_eq!(outcome.results[0].order, 0);
        assert_eq!(outcome.results[1].order, 1);
    }

    #[test]
    fn execute_trace_contains_plan_and_finish() {
        let agents = agents_with_roles(&[AgentRole::Coder]);
        let exec = ParallelExecutor::new(&agents);
        let g = ParallelGroup::new("g").with_task(ParallelTask::new("a", "x", AgentRole::Coder));

        let outcome = exec.execute(g).unwrap();
        let kinds: Vec<StepKind> = outcome.trace.iter().map(|s| s.kind).collect();
        assert!(kinds.contains(&StepKind::Plan));
        assert!(kinds.contains(&StepKind::Act));
        assert!(kinds.contains(&StepKind::Finish));
    }

    #[test]
    fn summary_contains_info() {
        let agents = agents_with_roles(&[AgentRole::Coder]);
        let exec = ParallelExecutor::new(&agents);
        let g = ParallelGroup::new("test-group").with_task(ParallelTask::new(
            "a",
            "x",
            AgentRole::Coder,
        ));

        let outcome = exec.execute(g).unwrap();
        let s = outcome.summary();
        assert!(s.contains("test-group"));
        assert!(s.contains("completed"));
        assert!(s.contains("1/1"));
    }

    #[test]
    fn full_flow_three_parallel_tasks() {
        let agents = agents_with_roles(&[AgentRole::Coder, AgentRole::Tester, AgentRole::Reviewer]);
        let exec = ParallelExecutor::new(&agents);
        let g = ParallelGroup::new("feature-x")
            .with_config(ParallelConfig::new().with_max_parallel(3))
            .with_task(ParallelTask::new(
                "auth",
                "implement authentication",
                AgentRole::Coder,
            ))
            .with_task(ParallelTask::new("tests", "write tests", AgentRole::Tester))
            .with_task(ParallelTask::new(
                "review",
                "review the code",
                AgentRole::Reviewer,
            ));

        let outcome = exec.execute(g).unwrap();
        assert!(outcome.success);
        assert_eq!(outcome.results.len(), 3);
        assert_eq!(outcome.successful(), 3);
    }
}
