//! `EchoAgent` — agente mínimo para tests.
//!
//! No usa provider real. Devuelve un eco del prompt del usuario.
//! Útil para tests de integración donde no queremos depender de red
//! ni de un modelo concreto.

use crate::{
    agent::{Agent, AgentOutcome, FinishKind},
    error::AgentResult,
    identity::{AgentIdentity, AgentRole},
    loop_engine,
    policy::ExecutionPolicy,
    session::AgentSession,
    step::Step,
};
use mevdan_provider::{mock::MockProvider, Provider};

/// Agente que hace eco de la tarea del usuario.
///
/// Por defecto usa un `MockProvider` interno. También se puede
/// inyectar un provider externo para tests más elaborados.
pub struct EchoAgent {
    identity: AgentIdentity,
    policy: ExecutionPolicy,
    provider: Box<dyn Provider>,
}

impl EchoAgent {
    /// Crea un `EchoAgent` con valores por defecto.
    pub fn new() -> Self {
        Self {
            identity: AgentIdentity::new("echo", AgentRole::General),
            policy: ExecutionPolicy::default(),
            provider: Box::new(MockProvider::default()),
        }
    }

    /// Sobrescribe la identidad.
    pub fn with_identity(mut self, identity: AgentIdentity) -> Self {
        self.identity = identity;
        self
    }

    /// Sobrescribe la política.
    pub fn with_policy(mut self, policy: ExecutionPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Inyecta un provider externo.
    pub fn with_provider(mut self, provider: Box<dyn Provider>) -> Self {
        self.provider = provider;
        self
    }

    /// Ejecución simplificada sin pasar por el loop completo.
    ///
    /// Útil cuando quieres solo un eco, sin reviews ni replans.
    pub fn simple_echo(&self, task: &str) -> AgentOutcome {
        AgentOutcome {
            response: format!("echo: {}", task),
            steps: vec![Step::act_success("echoed task")],
            success: true,
            finish_reason: FinishKind::Completed,
        }
    }
}

impl Default for EchoAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for EchoAgent {
    fn identity(&self) -> &AgentIdentity {
        &self.identity
    }

    fn policy(&self) -> &ExecutionPolicy {
        &self.policy
    }

    fn execute(&self, task: &str) -> AgentResult<AgentOutcome> {
        let session = AgentSession::new(self.policy.clone());
        loop_engine::run(&*self.provider, &self.identity, session, task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::StepKind;

    #[test]
    fn new_has_expected_defaults() {
        let a = EchoAgent::new();
        assert_eq!(a.identity().name, "echo");
        assert_eq!(a.identity().role, AgentRole::General);
    }

    #[test]
    fn default_impl_works() {
        let a = EchoAgent::default();
        assert_eq!(a.identity().name, "echo");
    }

    #[test]
    fn simple_echo_returns_prefixed_response() {
        let a = EchoAgent::new();
        let outcome = a.simple_echo("hello");
        assert_eq!(outcome.response, "echo: hello");
        assert!(outcome.success);
    }

    #[test]
    fn execute_runs_full_loop() {
        let a = EchoAgent::new();
        let outcome = a.execute("do something").unwrap();
        assert!(outcome.success);
        assert!(outcome.steps.iter().any(|s| s.kind == StepKind::Act));
        assert!(outcome.steps.iter().any(|s| s.kind == StepKind::Finish));
    }

    #[test]
    fn with_identity_overrides() {
        let custom = AgentIdentity::new("custom", AgentRole::Coder);
        let a = EchoAgent::new().with_identity(custom);
        assert_eq!(a.identity().name, "custom");
        assert_eq!(a.identity().role, AgentRole::Coder);
    }

    #[test]
    fn with_policy_overrides() {
        let a = EchoAgent::new().with_policy(ExecutionPolicy::minimal());
        assert_eq!(a.policy().max_steps, 1);
    }

    #[test]
    fn with_custom_provider() {
        let provider = MockProvider::default().with_response("custom response");
        let a = EchoAgent::new().with_provider(Box::new(provider));
        let outcome = a.execute("anything").unwrap();
        assert_eq!(outcome.response, "custom response");
    }

    #[test]
    fn echo_agent_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<EchoAgent>();
    }
}
