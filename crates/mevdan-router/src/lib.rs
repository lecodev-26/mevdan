//! # mevdan-router
//!
//! Sistema de routing de MEVDAN.
//!
//! ## Concepto
//!
//! El router decide **qué agente** y **qué modelo** usar para cada
//! tarea. Sin ML: reglas explícitas, auditables, testeables.
//!
//! ## Componentes
//!
//! - **`TaskKind`** — clasifica tareas por palabras clave.
//! - **`AgentRouter`** — elige el rol de agente adecuado.
//! - **`ModelRouter`** — elige el modelo según capacidades y coste.
//! - **`RouterEngine`** — fachada que une ambos.
//!
//! ## Estado del proyecto
//!
//! - **V4.8** ✅ — `TaskKind`, `AgentRouter`, `ModelRouter`, `RouterEngine`.
//! - **V4.9** ⏳ — Agent Handoff.
//!
//! ## Ejemplo
//!
//! ```no_run
//! use mevdan_router::{
//!     ModelCandidate, RouterEngine, RouterPolicy, TaskKind,
//! };
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let engine = RouterEngine::new();
//!
//! let candidates = vec![
//!     ModelCandidate::new("gpt-4o", 128_000).with_tools().with_cost(2.5, 10.0),
//!     ModelCandidate::new("gpt-4o-mini", 128_000).with_tools().with_cost(0.15, 0.6),
//! ];
//!
//! let decision = engine.route(
//!     TaskKind::Coding,
//!     &candidates,
//!     RouterPolicy::Automatic,
//! )?;
//!
//! println!("{}", decision.summary());
//! # Ok(())
//! # }
//! ```

pub mod agent_router;
pub mod engine;
pub mod error;
pub mod model_router;
pub mod task_kind;

// Re-exports de conveniencia.
pub use agent_router::{AgentDecision, AgentRouter};
pub use engine::{RouterEngine, RoutingDecision};
pub use error::{RouterError, RouterResult};
pub use model_router::{
    ModelCandidate, ModelDecision, ModelRequirements, ModelRouter, RouterPolicy,
};
pub use task_kind::TaskKind;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_flow_route_coding_task() {
        let engine = RouterEngine::new();

        let candidates = vec![
            ModelCandidate::new("gpt-4o", 128_000)
                .with_tools()
                .with_vision()
                .with_cost(2.5, 10.0),
            ModelCandidate::new("gpt-4o-mini", 128_000)
                .with_tools()
                .with_vision()
                .with_cost(0.15, 0.6),
        ];

        let decision = engine
            .route(TaskKind::Coding, &candidates, RouterPolicy::Automatic)
            .unwrap();

        assert_eq!(decision.task_kind, TaskKind::Coding);
        assert_eq!(decision.agent.role, mevdan_agent::AgentRole::Coder);
        assert_eq!(decision.model.model_name, "gpt-4o-mini");
        assert!(decision.summary().contains("coding"));
    }

    #[test]
    fn full_flow_route_from_text() {
        let engine = RouterEngine::new();

        let candidates = vec![ModelCandidate::new("model-x", 128_000).with_tools()];

        let decision = engine
            .route_text(
                "review the pull request",
                &candidates,
                RouterPolicy::Automatic,
            )
            .unwrap();

        assert_eq!(decision.task_kind, TaskKind::Review);
        assert_eq!(decision.agent.role, mevdan_agent::AgentRole::Reviewer);
    }

    #[test]
    fn full_flow_all_task_kinds_have_agent() {
        let engine = RouterEngine::new();
        for kind in [
            TaskKind::Planning,
            TaskKind::Coding,
            TaskKind::Review,
            TaskKind::Research,
            TaskKind::Testing,
            TaskKind::Documentation,
            TaskKind::DataAnalysis,
            TaskKind::Refactoring,
            TaskKind::Debugging,
            TaskKind::Other,
        ] {
            assert!(engine.pick_role(kind).is_ok(), "failed for {:?}", kind);
        }
    }
}
