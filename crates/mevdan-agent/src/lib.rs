//! # mevdan-agent
//!
//! Motor de agentes de MEVDAN.
//!
//! ## Estado del proyecto
//!
//! - **12.1** ✅ — cimientos (identity, policy, session, error).
//! - **12.2** ✅ — loop de razonamiento (step, agent, loop_engine, echo).
//! - **12.3** ✅ — multi-agente (role, plan, multi).
//!
//! ## Qué hace el loop
//!
//! 1. `Understand` — interpreta la tarea.
//! 2. `Act` — llama al provider.
//! 3. `Observe` — procesa la respuesta.
//! 4. `Review` — auto-crítica (si la política lo permite).
//! 5. `Replan` — reintento con feedback (si la review falla).
//! 6. `Finish` — entregar resultado.
//!
//! ## Qué NO hace todavía
//!
//! - No invoca tools (Fase 15-17).
//! - No aplica permisos (Fase 18).
//! - No persiste sesiones ni planes (Fase 20+).
//! - No ejecuta agentes en paralelo (Fase 40).

pub mod agent;
pub mod echo;
pub mod error;
pub mod identity;
pub mod loop_engine;
pub mod multi;
pub mod plan;
pub mod policy;
pub mod role;
pub mod session;
pub mod step;

// Re-exports de conveniencia.
pub use agent::{Agent, AgentOutcome, FinishKind};
pub use echo::EchoAgent;
pub use error::{AgentError, AgentResult};
pub use identity::{AgentIdentity, AgentRole};
pub use multi::{MultiAgent, MultiAgentOutcome};
pub use plan::{Plan, PlanId, PlanStatus, PlanStep, PlanStepId, PlanStepStatus};
pub use policy::{ExecutionPolicy, ReviewMode};
pub use session::{AgentSession, AgentSessionId};
pub use step::{Step, StepId, StepKind, StepOutcome};

#[cfg(test)]
mod tests {
    use super::*;
    use mevdan_provider::Message;

    #[test]
    fn full_flow_create_agent_session() {
        let identity = AgentIdentity::new("default-coder", AgentRole::Coder);
        let policy = ExecutionPolicy::default();
        let mut session = AgentSession::new(policy);

        session.push_message(Message::system(&identity.system_prompt));
        session.push_message(Message::user("Write a hello world in Rust"));

        assert_eq!(session.message_count(), 2);
        assert!(session.can_take_step().is_ok());
        assert_eq!(identity.role, AgentRole::Coder);
    }

    #[test]
    fn policy_limits_are_enforced() {
        let policy = ExecutionPolicy::minimal();
        let mut session = AgentSession::new(policy);

        assert!(session.can_take_step().is_ok());
        session.record_step();
        assert!(session.can_take_step().is_err());
    }

    #[test]
    fn full_flow_agent_execution() {
        let agent = EchoAgent::new();
        let outcome = agent.execute("do something useful").unwrap();

        assert!(outcome.success);
        assert_eq!(outcome.finish_reason, FinishKind::Completed);
        assert!(outcome.steps.len() >= 4);
    }

    #[test]
    fn full_flow_multi_agent_plan() {
        let mut multi = MultiAgent::new();
        multi.register(Box::new(EchoAgent::new().with_identity(role::planner("p"))));
        multi.register(Box::new(EchoAgent::new().with_identity(role::coder("c"))));
        multi.register(Box::new(
            EchoAgent::new().with_identity(role::reviewer("r")),
        ));

        let plan = Plan::with_steps(
            "ship a feature",
            vec![
                PlanStep::new("plan").with_role(AgentRole::Planner),
                PlanStep::new("code").with_role(AgentRole::Coder),
                PlanStep::new("review").with_role(AgentRole::Reviewer),
            ],
        );

        let outcome = multi.execute(plan).unwrap();
        assert!(outcome.success());
        assert_eq!(outcome.plan.step_count(), 3);
    }
}
