//! Trait `Agent` — contrato de un agente.
//!
//! Un agente es algo que:
//! 1. Tiene una **identidad** (nombre, rol, system prompt).
//! 2. Tiene una **política de ejecución** (límites).
//! 3. Puede **ejecutar una tarea** produciendo una respuesta y un
//!    registro de pasos.
//!
//! ## Regla fundamental
//!
//! El agente NO es dueño del estado del proyecto. El runtime
//! (`mevdan-runtime`, futura fase) coordina. El agente solo ejecuta
//! tareas y devuelve resultados.

use crate::{error::AgentResult, identity::AgentIdentity, policy::ExecutionPolicy, step::Step};
use serde::{Deserialize, Serialize};

/// Resultado de la ejecución de un agente.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutcome {
    /// Respuesta final (texto).
    pub response: String,
    /// Pasos dados durante la ejecución.
    pub steps: Vec<Step>,
    /// ¿Se alcanzó una respuesta satisfactoria?
    pub success: bool,
    /// Razón de terminación (por éxito, por límite, por error...).
    pub finish_reason: FinishKind,
}

/// Cómo terminó el agente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishKind {
    /// El agente terminó con éxito.
    Completed,
    /// El agente terminó tras agotar replans.
    GaveUp,
    /// El agente terminó porque alcanzó un límite (steps, tokens, tiempo).
    HitLimit,
    /// El agente terminó por un error externo.
    Errored,
}

/// Un agente.
pub trait Agent: Send + Sync {
    /// Identidad del agente.
    fn identity(&self) -> &AgentIdentity;

    /// Política de ejecución efectiva.
    fn policy(&self) -> &ExecutionPolicy;

    /// Ejecuta una tarea y devuelve el resultado.
    ///
    /// Este método es **bloqueante**. El agente hace todo el trabajo
    /// internamente: llamadas al provider, auto-crítica, replanning.
    fn execute(&self, task: &str) -> AgentResult<AgentOutcome>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::echo::EchoAgent;

    #[test]
    fn agent_outcome_construction() {
        let outcome = AgentOutcome {
            response: "done".into(),
            steps: vec![],
            success: true,
            finish_reason: FinishKind::Completed,
        };
        assert!(outcome.success);
        assert_eq!(outcome.finish_reason, FinishKind::Completed);
    }

    #[test]
    fn finish_kind_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&FinishKind::Completed).unwrap(),
            "\"completed\""
        );
        assert_eq!(
            serde_json::to_string(&FinishKind::HitLimit).unwrap(),
            "\"hit_limit\""
        );
    }

    #[test]
    fn echo_agent_implements_agent_trait() {
        let agent = EchoAgent::new();
        let _: &dyn Agent = &agent;
        assert_eq!(agent.identity().role, crate::identity::AgentRole::General);
    }

    #[test]
    fn outcome_roundtrips() {
        let outcome = AgentOutcome {
            response: "hello".into(),
            steps: vec![Step::act_success("did act")],
            success: true,
            finish_reason: FinishKind::Completed,
        };
        let json = serde_json::to_string(&outcome).unwrap();
        let back: AgentOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(back.response, "hello");
        assert_eq!(back.steps.len(), 1);
    }
}
