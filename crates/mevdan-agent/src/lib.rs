//! # mevdan-agent
//!
//! Motor de agentes de MEVDAN.
//!
//! ## Estado del proyecto
//!
//! - **12.1** ✅ — cimientos (identity, policy, session, error).
//! - **12.2** ✅ — loop de razonamiento (step, agent, loop_engine, echo).
//! - **12.3** ✅ — multi-agente (role, plan, multi).
//! - **V4.7** ✅ — `Team`, `Workflow`, `MultiAgentEngine`.
//! - **V4.8** ⏳ — Router.

pub mod agent;
pub mod echo;
pub mod engine;
pub mod error;
pub mod identity;
pub mod loop_engine;
pub mod multi;
pub mod plan;
pub mod policy;
pub mod role;
pub mod session;
pub mod step;
pub mod team;
pub mod workflow;

// Re-exports de conveniencia.
pub use agent::{Agent, AgentOutcome, FinishKind};
pub use echo::EchoAgent;
pub use engine::{MultiAgentEngine, TeamExecution};
pub use error::{AgentError, AgentResult};
pub use identity::{AgentIdentity, AgentRole};
pub use multi::{MultiAgent, MultiAgentOutcome};
pub use plan::{Plan, PlanId, PlanStatus, PlanStep, PlanStepId, PlanStepStatus};
pub use policy::{ExecutionPolicy, ReviewMode};
pub use session::{AgentSession, AgentSessionId};
pub use step::{Step, StepId, StepKind, StepOutcome};
pub use team::{Team, TeamMember};
pub use workflow::{Workflow, WorkflowId, WorkflowStatus, WorkflowStep};

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
    }

    #[test]
    fn full_flow_agent_execution() {
        let agent = EchoAgent::new();
        let outcome = agent.execute("do something useful").unwrap();

        assert!(outcome.success);
        assert_eq!(outcome.finish_reason, FinishKind::Completed);
    }

    #[test]
    fn full_flow_multi_agent_team_workflow() {
        let mut engine = MultiAgentEngine::new();

        let planner =
            EchoAgent::new().with_identity(AgentIdentity::new("planner-1", AgentRole::Planner));
        let coder = EchoAgent::new().with_identity(AgentIdentity::new("coder-1", AgentRole::Coder));
        let reviewer =
            EchoAgent::new().with_identity(AgentIdentity::new("reviewer-1", AgentRole::Reviewer));

        engine.register(Box::new(planner));
        engine.register(Box::new(coder));
        engine.register(Box::new(reviewer));

        let workflow = Workflow::new("build feature", "build a calculator CLI")
            .add_step(WorkflowStep::new("plan the work", AgentRole::Planner))
            .add_step(WorkflowStep::new("write the code", AgentRole::Coder))
            .add_step(WorkflowStep::new("review the code", AgentRole::Reviewer));

        let exec = engine.execute(workflow).unwrap();

        assert!(exec.success);
        assert_eq!(exec.workflow.status, WorkflowStatus::Completed);
        assert_eq!(exec.successful_steps(), 3);
        assert_eq!(exec.failed_steps(), 0);

        let team = engine.as_team("my-team");
        assert_eq!(team.name, "my-team");
        assert_eq!(team.len(), 3);
        assert!(team.has_role(AgentRole::Planner));
        assert!(team.has_role(AgentRole::Coder));
        assert!(team.has_role(AgentRole::Reviewer));
    }

    #[test]
    fn full_flow_workflow_missing_role() {
        let mut engine = MultiAgentEngine::new();
        engine.register(Box::new(
            EchoAgent::new().with_identity(AgentIdentity::new("coder-1", AgentRole::Coder)),
        ));

        let workflow =
            Workflow::new("w", "g").add_step(WorkflowStep::new("plan", AgentRole::Planner));

        let err = engine.execute(workflow).unwrap_err();
        assert!(matches!(err, AgentError::InvalidConfig(_)));
    }
}
