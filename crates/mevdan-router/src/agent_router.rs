//! `AgentRouter` — decide qué agente usar para una tarea.

use crate::{error::RouterResult, task_kind::TaskKind};
use mevdan_agent::AgentRole;
use serde::{Deserialize, Serialize};

/// Decisión del AgentRouter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentDecision {
    /// Tipo de tarea clasificada.
    pub task_kind: TaskKind,
    /// Rol del agente elegido.
    pub role: AgentRole,
    /// Razón de la elección (para auditoría).
    pub reason: String,
}

/// Router de agentes.
///
/// Dado un tipo de tarea, decide qué rol de agente es el más adecuado.
/// La decisión se basa en reglas explícitas, no en ML.
#[derive(Debug, Default)]
pub struct AgentRouter;

impl AgentRouter {
    pub fn new() -> Self {
        Self
    }

    /// Decide el rol más adecuado para un tipo de tarea.
    pub fn route(&self, task_kind: TaskKind) -> RouterResult<AgentDecision> {
        let (role, reason) = match task_kind {
            TaskKind::Planning => (AgentRole::Planner, "planning tasks go to the planner"),
            TaskKind::Coding | TaskKind::Refactoring | TaskKind::Debugging => (
                AgentRole::Coder,
                "coding, refactoring and debugging go to the coder",
            ),
            TaskKind::Review => (AgentRole::Reviewer, "review tasks go to the reviewer"),
            TaskKind::Testing => (AgentRole::Tester, "testing tasks go to the tester"),
            TaskKind::Research | TaskKind::DataAnalysis => (
                AgentRole::Researcher,
                "research and data analysis go to the researcher",
            ),
            TaskKind::Documentation => (
                AgentRole::Documenter,
                "documentation tasks go to the documenter",
            ),
            TaskKind::Other => (
                AgentRole::Coder,
                "fallback: unknown tasks default to the coder",
            ),
        };

        Ok(AgentDecision {
            task_kind,
            role,
            reason: reason.to_string(),
        })
    }

    /// Igual que `route`, pero desde un texto crudo.
    /// Clasifica primero, luego decide.
    pub fn route_text(&self, text: &str) -> RouterResult<AgentDecision> {
        let kind = TaskKind::classify(text);
        self.route(kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_planning_to_planner() {
        let router = AgentRouter::new();
        let decision = router.route(TaskKind::Planning).unwrap();
        assert_eq!(decision.role, AgentRole::Planner);
        assert_eq!(decision.task_kind, TaskKind::Planning);
    }

    #[test]
    fn route_coding_to_coder() {
        let router = AgentRouter::new();
        let decision = router.route(TaskKind::Coding).unwrap();
        assert_eq!(decision.role, AgentRole::Coder);
    }

    #[test]
    fn route_refactoring_to_coder() {
        let router = AgentRouter::new();
        let decision = router.route(TaskKind::Refactoring).unwrap();
        assert_eq!(decision.role, AgentRole::Coder);
    }

    #[test]
    fn route_debugging_to_coder() {
        let router = AgentRouter::new();
        let decision = router.route(TaskKind::Debugging).unwrap();
        assert_eq!(decision.role, AgentRole::Coder);
    }

    #[test]
    fn route_review_to_reviewer() {
        let router = AgentRouter::new();
        let decision = router.route(TaskKind::Review).unwrap();
        assert_eq!(decision.role, AgentRole::Reviewer);
    }

    #[test]
    fn route_testing_to_tester() {
        let router = AgentRouter::new();
        let decision = router.route(TaskKind::Testing).unwrap();
        assert_eq!(decision.role, AgentRole::Tester);
    }

    #[test]
    fn route_research_to_researcher() {
        let router = AgentRouter::new();
        let decision = router.route(TaskKind::Research).unwrap();
        assert_eq!(decision.role, AgentRole::Researcher);
    }

    #[test]
    fn route_data_analysis_to_researcher() {
        let router = AgentRouter::new();
        let decision = router.route(TaskKind::DataAnalysis).unwrap();
        assert_eq!(decision.role, AgentRole::Researcher);
    }

    #[test]
    fn route_documentation_to_documenter() {
        let router = AgentRouter::new();
        let decision = router.route(TaskKind::Documentation).unwrap();
        assert_eq!(decision.role, AgentRole::Documenter);
    }

    #[test]
    fn route_other_to_coder_fallback() {
        let router = AgentRouter::new();
        let decision = router.route(TaskKind::Other).unwrap();
        assert_eq!(decision.role, AgentRole::Coder);
        assert!(decision.reason.contains("fallback"));
    }

    #[test]
    fn route_text_classifies_and_routes() {
        let router = AgentRouter::new();
        let decision = router.route_text("write tests for the parser").unwrap();
        assert_eq!(decision.task_kind, TaskKind::Testing);
        assert_eq!(decision.role, AgentRole::Tester);
    }

    #[test]
    fn route_text_unknown_falls_back_to_coder() {
        let router = AgentRouter::new();
        let decision = router.route_text("random string here").unwrap();
        assert_eq!(decision.task_kind, TaskKind::Other);
        assert_eq!(decision.role, AgentRole::Coder);
    }

    #[test]
    fn decision_serializes() {
        let router = AgentRouter::new();
        let decision = router.route(TaskKind::Coding).unwrap();
        let json = serde_json::to_string(&decision).unwrap();
        let back: AgentDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(back, decision);
    }

    #[test]
    fn default_impl_works() {
        let router = AgentRouter;
        assert_eq!(
            router.route(TaskKind::Coding).unwrap().role,
            AgentRole::Coder
        );
    }
}
