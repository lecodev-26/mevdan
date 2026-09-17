//! `RouterEngine` — fachada que combina AgentRouter y ModelRouter.

use crate::{
    agent_router::{AgentDecision, AgentRouter},
    error::RouterResult,
    model_router::{ModelCandidate, ModelDecision, ModelRouter, RouterPolicy},
    task_kind::TaskKind,
};
use mevdan_agent::AgentRole;
use serde::{Deserialize, Serialize};

/// Decisión combinada: agente + modelo para una tarea.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub task_kind: TaskKind,
    pub agent: AgentDecision,
    pub model: ModelDecision,
}

impl RoutingDecision {
    /// Resumen textual.
    pub fn summary(&self) -> String {
        format!(
            "task={} → agent={:?}, model={}",
            self.task_kind.display_name(),
            self.agent.role,
            self.model.model_name
        )
    }
}

/// Motor de routing: une AgentRouter y ModelRouter.
#[derive(Debug, Default)]
pub struct RouterEngine {
    agent_router: AgentRouter,
    model_router: ModelRouter,
}

impl RouterEngine {
    pub fn new() -> Self {
        Self {
            agent_router: AgentRouter::new(),
            model_router: ModelRouter::new(),
        }
    }

    /// Acceso al AgentRouter.
    pub fn agent_router(&self) -> &AgentRouter {
        &self.agent_router
    }

    /// Acceso al ModelRouter.
    pub fn model_router(&self) -> &ModelRouter {
        &self.model_router
    }

    /// Decide el agente y el modelo para una tarea.
    pub fn route(
        &self,
        task_kind: TaskKind,
        candidates: &[ModelCandidate],
        policy: RouterPolicy,
    ) -> RouterResult<RoutingDecision> {
        let agent = self.agent_router.route(task_kind)?;
        let model = self.model_router.route(task_kind, candidates, policy)?;
        Ok(RoutingDecision {
            task_kind,
            agent,
            model,
        })
    }

    /// Igual que `route`, pero clasifica la tarea desde un texto.
    pub fn route_text(
        &self,
        text: &str,
        candidates: &[ModelCandidate],
        policy: RouterPolicy,
    ) -> RouterResult<RoutingDecision> {
        let kind = TaskKind::classify(text);
        self.route(kind, candidates, policy)
    }

    /// Solo decide el agente (sin modelo).
    pub fn route_agent(&self, task_kind: TaskKind) -> RouterResult<AgentDecision> {
        self.agent_router.route(task_kind)
    }

    /// Solo decide el agente, clasificando primero desde un texto.
    pub fn route_agent_text(&self, text: &str) -> RouterResult<AgentDecision> {
        self.agent_router.route_text(text)
    }

    /// Decide solo el rol de agente, sin construir la decisión completa.
    pub fn pick_role(&self, task_kind: TaskKind) -> RouterResult<AgentRole> {
        Ok(self.agent_router.route(task_kind)?.role)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidates() -> Vec<ModelCandidate> {
        vec![
            ModelCandidate::new("gpt-4o", 128_000)
                .with_tools()
                .with_vision()
                .with_cost(2.5, 10.0),
            ModelCandidate::new("gpt-4o-mini", 128_000)
                .with_tools()
                .with_vision()
                .with_cost(0.15, 0.6),
        ]
    }

    #[test]
    fn new_engine_is_ready() {
        let engine = RouterEngine::new();
        assert!(engine.agent_router().route(TaskKind::Coding).is_ok());
    }

    #[test]
    fn route_coding_complete() {
        let engine = RouterEngine::new();
        let decision = engine
            .route(TaskKind::Coding, &candidates(), RouterPolicy::Automatic)
            .unwrap();

        assert_eq!(decision.task_kind, TaskKind::Coding);
        assert_eq!(decision.agent.role, AgentRole::Coder);
        assert_eq!(decision.model.model_name, "gpt-4o-mini");
    }

    #[test]
    fn route_review_complete() {
        let engine = RouterEngine::new();
        let decision = engine
            .route(TaskKind::Review, &candidates(), RouterPolicy::Automatic)
            .unwrap();

        assert_eq!(decision.agent.role, AgentRole::Reviewer);
        assert!(!decision.model.model_name.is_empty());
    }

    #[test]
    fn route_text_classifies_and_decides() {
        let engine = RouterEngine::new();
        let decision = engine
            .route_text(
                "write tests for the parser",
                &candidates(),
                RouterPolicy::Automatic,
            )
            .unwrap();

        assert_eq!(decision.task_kind, TaskKind::Testing);
        assert_eq!(decision.agent.role, AgentRole::Tester);
    }

    #[test]
    fn route_text_planning() {
        let engine = RouterEngine::new();
        let decision = engine
            .route_text(
                "plan the architecture of the system",
                &candidates(),
                RouterPolicy::Automatic,
            )
            .unwrap();

        assert_eq!(decision.task_kind, TaskKind::Planning);
        assert_eq!(decision.agent.role, AgentRole::Planner);
    }

    #[test]
    fn route_agent_only() {
        let engine = RouterEngine::new();
        let decision = engine.route_agent(TaskKind::Documentation).unwrap();
        assert_eq!(decision.role, AgentRole::Documenter);
    }

    #[test]
    fn route_agent_text_only() {
        let engine = RouterEngine::new();
        let decision = engine.route_agent_text("debug this crash").unwrap();
        assert_eq!(decision.role, AgentRole::Coder);
        assert_eq!(decision.task_kind, TaskKind::Debugging);
    }

    #[test]
    fn pick_role_simple() {
        let engine = RouterEngine::new();
        assert_eq!(
            engine.pick_role(TaskKind::Research).unwrap(),
            AgentRole::Researcher
        );
    }

    #[test]
    fn route_fails_without_candidates() {
        let engine = RouterEngine::new();
        let err = engine
            .route(TaskKind::Coding, &[], RouterPolicy::Automatic)
            .unwrap_err();
        assert!(matches!(err, crate::error::RouterError::NoCandidates));
    }

    #[test]
    fn summary_contains_info() {
        let engine = RouterEngine::new();
        let decision = engine
            .route(TaskKind::Coding, &candidates(), RouterPolicy::Automatic)
            .unwrap();

        let summary = decision.summary();
        assert!(summary.contains("coding"));
        assert!(summary.contains("Coder"));
        assert!(summary.contains("gpt-4o-mini"));
    }

    #[test]
    fn decision_serializes() {
        let engine = RouterEngine::new();
        let decision = engine
            .route(TaskKind::Coding, &candidates(), RouterPolicy::Automatic)
            .unwrap();

        let json = serde_json::to_string(&decision).unwrap();
        let back: RoutingDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(back.task_kind, decision.task_kind);
        assert_eq!(back.agent.role, decision.agent.role);
        assert_eq!(back.model.model_name, decision.model.model_name);
    }

    #[test]
    fn fallback_policy_works() {
        let engine = RouterEngine::new();
        let decision = engine
            .route(TaskKind::Coding, &candidates(), RouterPolicy::Fallback)
            .unwrap();

        assert_eq!(decision.model.policy, RouterPolicy::Fallback);
        assert!(!decision.model.alternatives.is_empty());
    }

    #[test]
    fn default_impl_works() {
        let engine = RouterEngine::default();
        assert_eq!(
            engine.pick_role(TaskKind::Coding).unwrap(),
            AgentRole::Coder
        );
    }
}
