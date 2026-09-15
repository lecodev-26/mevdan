//! Orquestación multi-agente.
//!
//! Un `MultiAgent` coordina varios agentes sobre un mismo plan.
//! Ejecución **secuencial** por ahora (paralelo llega en Fase 40).
//!
//! ## Flujo
//!
//! 1. Se construye un plan con pasos.
//! 2. Cada paso tiene un rol asignado (opcional).
//! 3. `MultiAgent::execute(plan)` recorre los pasos en orden,
//!    resolviendo el agente adecuado para cada uno.
//! 4. Cada paso produce un `Step` de trazabilidad.

use crate::{
    agent::{Agent, AgentOutcome},
    error::{AgentError, AgentResult},
    identity::AgentRole,
    plan::{Plan, PlanStatus, PlanStepStatus},
    step::{Step, StepKind, StepOutcome},
};
use std::collections::BTreeMap;

/// Registro de agentes por rol.
///
/// `MultiAgent` mantiene un agente por rol. Cuando ejecuta un paso,
/// busca el agente cuyo rol coincide con el del paso.
pub struct MultiAgent {
    agents: BTreeMap<AgentRole, Box<dyn Agent>>,
    fallback: Option<Box<dyn Agent>>,
}

impl MultiAgent {
    /// Crea un orquestador vacío.
    pub fn new() -> Self {
        Self {
            agents: BTreeMap::new(),
            fallback: None,
        }
    }

    /// Registra un agente para su rol (según su identidad).
    pub fn register(&mut self, agent: Box<dyn Agent>) {
        let role = agent.identity().role;
        self.agents.insert(role, agent);
    }

    /// Registra un agente de fallback para pasos cuyo rol no tenga
    /// agente específico.
    pub fn with_fallback(mut self, agent: Box<dyn Agent>) -> Self {
        self.fallback = Some(agent);
        self
    }

    /// Número de agentes registrados.
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    /// ¿Hay un agente para este rol?
    pub fn has_role(&self, role: AgentRole) -> bool {
        self.agents.contains_key(&role)
    }

    /// Devuelve el agente para un rol, o el fallback.
    fn resolve(&self, role: Option<AgentRole>) -> AgentResult<&dyn Agent> {
        if let Some(r) = role {
            if let Some(a) = self.agents.get(&r) {
                return Ok(a.as_ref());
            }
        }
        self.fallback
            .as_deref()
            .or_else(|| self.agents.values().next().map(|a| a.as_ref()))
            .ok_or_else(|| AgentError::NoProvider("<multi>".to_string()))
    }

    /// Ejecuta un plan paso a paso.
    ///
    /// Devuelve los `Step`s de trazabilidad y el plan actualizado.
    pub fn execute(&self, mut plan: Plan) -> AgentResult<MultiAgentOutcome> {
        let mut trace: Vec<Step> = Vec::new();
        plan.status = PlanStatus::Running;

        trace.push(Step::new(
            StepKind::Plan,
            StepOutcome::Success,
            format!("executing plan with {} steps", plan.step_count()),
        ));

        while let Some(idx) = self.next_step_index(&plan) {
            // Marca el paso como running.
            plan.steps[idx].status = PlanStepStatus::Running;

            let role = plan.steps[idx].role;
            let description = plan.steps[idx].description.clone();

            let agent = self.resolve(role)?;
            let agent_name = agent.identity().name.to_string();

            trace.push(Step::new(
                StepKind::Act,
                StepOutcome::Success,
                format!("step {} → agent '{}' (role {:?})", idx, agent_name, role),
            ));

            // Ejecuta el agente sobre la descripción del paso.
            match agent.execute(&description) {
                Ok(outcome) => {
                    plan.steps[idx].mark_completed(outcome.response.clone());
                    trace.push(Step::act_success(format!(
                        "step {} completed by '{}' ({} sub-steps)",
                        idx,
                        agent_name,
                        outcome.steps.len()
                    )));
                }
                Err(e) => {
                    plan.steps[idx].mark_failed(e.to_string());
                    trace.push(Step::new(
                        StepKind::Act,
                        StepOutcome::Failure,
                        format!("step {} failed: {}", idx, e),
                    ));
                    plan.refresh_status();
                    return Err(e);
                }
            }

            plan.refresh_status();
        }

        trace.push(Step::new(
            StepKind::Finish,
            StepOutcome::Success,
            format!("plan finished with status {:?}", plan.status),
        ));

        Ok(MultiAgentOutcome { plan, trace })
    }

    /// Devuelve el índice del siguiente paso a ejecutar (respetando
    /// dependencias), o `None` si no queda ninguno listo.
    fn next_step_index(&self, plan: &Plan) -> Option<usize> {
        let completed = plan.completed_ids();
        plan.steps
            .iter()
            .enumerate()
            .find(|(_, s)| s.status == PlanStepStatus::Pending && s.is_ready(&completed))
            .map(|(i, _)| i)
    }
}

impl Default for MultiAgent {
    fn default() -> Self {
        Self::new()
    }
}

/// Resultado de una ejecución multi-agente.
#[derive(Debug)]
pub struct MultiAgentOutcome {
    pub plan: Plan,
    pub trace: Vec<Step>,
}

impl MultiAgentOutcome {
    /// ¿El plan terminó exitosamente?
    pub fn success(&self) -> bool {
        self.plan.status == PlanStatus::Completed
    }

    /// Resumen textual.
    pub fn summary(&self) -> String {
        format!(
            "plan '{}': {} steps, status={:?}",
            self.plan.goal,
            self.plan.step_count(),
            self.plan.status
        )
    }
}

// Re-export para quien use `MultiAgentOutcome::success` como
// discriminante principal.
impl From<&MultiAgentOutcome> for AgentOutcome {
    fn from(o: &MultiAgentOutcome) -> Self {
        AgentOutcome {
            response: o.summary(),
            steps: o.trace.clone(),
            success: o.success(),
            finish_reason: if o.success() {
                crate::agent::FinishKind::Completed
            } else {
                crate::agent::FinishKind::GaveUp
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
        plan::{Plan, PlanStep},
    };

    fn echo_for(role: AgentRole) -> Box<dyn Agent> {
        let identity = AgentIdentity::new(format!("echo-{:?}", role), role);
        Box::new(EchoAgent::new().with_identity(identity))
    }

    #[test]
    fn new_is_empty() {
        let m = MultiAgent::new();
        assert_eq!(m.agent_count(), 0);
    }

    #[test]
    fn register_adds_agent() {
        let mut m = MultiAgent::new();
        m.register(echo_for(AgentRole::Coder));
        assert_eq!(m.agent_count(), 1);
        assert!(m.has_role(AgentRole::Coder));
        assert!(!m.has_role(AgentRole::Planner));
    }

    #[test]
    fn register_replaces_same_role() {
        let mut m = MultiAgent::new();
        m.register(echo_for(AgentRole::Coder));
        m.register(echo_for(AgentRole::Coder));
        assert_eq!(m.agent_count(), 1);
    }

    #[test]
    fn execute_empty_plan_succeeds() {
        let m = MultiAgent::new().with_fallback(echo_for(AgentRole::General));
        let plan = Plan::new("empty");
        let outcome = m.execute(plan).unwrap();
        // Un plan vacío no es "completo" según `is_complete`,
        // así que el status queda como estaba.
        assert_eq!(outcome.trace.len(), 2); // Plan + Finish
    }

    #[test]
    fn execute_single_step_plan() {
        let m = MultiAgent::new().with_fallback(echo_for(AgentRole::General));
        let plan = Plan::with_steps("do one", vec![PlanStep::new("say hi")]);
        let outcome = m.execute(plan).unwrap();

        assert!(outcome.success());
        assert_eq!(outcome.plan.step_count(), 1);
        assert_eq!(outcome.plan.steps[0].status, PlanStepStatus::Completed);
    }

    #[test]
    fn execute_multi_step_plan_with_roles() {
        let mut m = MultiAgent::new();
        m.register(echo_for(AgentRole::Planner));
        m.register(echo_for(AgentRole::Coder));
        m.register(echo_for(AgentRole::Reviewer));

        let plan = Plan::with_steps(
            "build feature",
            vec![
                PlanStep::new("plan the work").with_role(AgentRole::Planner),
                PlanStep::new("write code").with_role(AgentRole::Coder),
                PlanStep::new("review code").with_role(AgentRole::Reviewer),
            ],
        );

        let outcome = m.execute(plan).unwrap();

        assert!(outcome.success());
        for step in &outcome.plan.steps {
            assert_eq!(step.status, PlanStepStatus::Completed);
            assert!(step.result.is_some());
        }
    }

    #[test]
    fn execute_respects_dependencies() {
        let step1 = PlanStep::new("first");
        let step1_id = step1.id;
        let step2 = PlanStep::new("second").with_depends_on(vec![step1_id]);

        let m = MultiAgent::new().with_fallback(echo_for(AgentRole::General));
        let plan = Plan::with_steps("deps", vec![step1, step2]);

        let outcome = m.execute(plan).unwrap();

        assert!(outcome.success());
        assert_eq!(outcome.plan.steps[0].status, PlanStepStatus::Completed);
        assert_eq!(outcome.plan.steps[1].status, PlanStepStatus::Completed);
    }

    #[test]
    fn execute_without_any_agent_fails() {
        let m = MultiAgent::new();
        let plan = Plan::with_steps("g", vec![PlanStep::new("do something")]);
        let result = m.execute(plan);
        assert!(result.is_err());
    }

    #[test]
    fn trace_contains_plan_act_finish() {
        let m = MultiAgent::new().with_fallback(echo_for(AgentRole::General));
        let plan = Plan::with_steps("g", vec![PlanStep::new("only")]);
        let outcome = m.execute(plan).unwrap();

        let kinds: Vec<StepKind> = outcome.trace.iter().map(|s| s.kind).collect();
        assert!(kinds.contains(&StepKind::Plan));
        assert!(kinds.contains(&StepKind::Act));
        assert!(kinds.contains(&StepKind::Finish));
    }

    #[test]
    fn summary_is_non_empty() {
        let m = MultiAgent::new().with_fallback(echo_for(AgentRole::General));
        let plan = Plan::with_steps("g", vec![PlanStep::new("a")]);
        let outcome = m.execute(plan).unwrap();
        assert!(!outcome.summary().is_empty());
    }

    #[test]
    fn outcome_converts_to_agent_outcome() {
        let m = MultiAgent::new().with_fallback(echo_for(AgentRole::General));
        let plan = Plan::with_steps("g", vec![PlanStep::new("a")]);
        let outcome = m.execute(plan).unwrap();

        let agent_outcome: AgentOutcome = (&outcome).into();
        assert!(agent_outcome.success);
        assert!(!agent_outcome.response.is_empty());
    }
}
