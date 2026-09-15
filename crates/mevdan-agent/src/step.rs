//! Pasos del loop de razonamiento.
//!
//! Un `Step` representa una iteración del loop del agente: qué hizo,
//! con qué entrada, y qué resultado produjo. Los steps son la unidad
//! de trazabilidad del agente.

use chrono::{DateTime, Utc};
use mevdan_provider::{ChatRequest, ChatResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// ID de un paso.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StepId(pub Uuid);

impl StepId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for StepId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for StepId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Tipo de paso.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepKind {
    /// Paso inicial: interpretar la tarea.
    Understand,
    /// Paso de planificación.
    Plan,
    /// Paso de acción (invocación al provider).
    Act,
    /// Paso de observación (procesar respuesta).
    Observe,
    /// Auto-crítica de la respuesta.
    Review,
    /// Replanning tras un fallo.
    Replan,
    /// Paso final: entregar resultado.
    Finish,
}

/// Resultado de un paso.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepOutcome {
    /// El paso se completó con éxito.
    Success,
    /// El paso falló.
    Failure,
    /// El paso se completó parcialmente.
    Partial,
    /// El paso fue omitido (por política, por ejemplo).
    Skipped,
}

/// Un paso del agente.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub id: StepId,
    pub kind: StepKind,
    pub outcome: StepOutcome,
    pub occurred_at: DateTime<Utc>,
    /// Nota corta describiendo qué hizo este paso.
    pub note: String,
    /// Request que se hizo (si aplica).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<ChatRequest>,
    /// Response que se recibió (si aplica).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<ChatResponse>,
    /// Tokens consumidos en este paso (si aplica).
    #[serde(default)]
    pub tokens_used: u32,
}

impl Step {
    pub fn new(kind: StepKind, outcome: StepOutcome, note: impl Into<String>) -> Self {
        Self {
            id: StepId::new(),
            kind,
            outcome,
            occurred_at: Utc::now(),
            note: note.into(),
            request: None,
            response: None,
            tokens_used: 0,
        }
    }

    pub fn with_request(mut self, req: ChatRequest) -> Self {
        self.request = Some(req);
        self
    }

    pub fn with_response(mut self, resp: ChatResponse, tokens: u32) -> Self {
        self.response = Some(resp);
        self.tokens_used = tokens;
        self
    }

    /// Atajo: un paso de tipo `Act` con éxito.
    pub fn act_success(note: impl Into<String>) -> Self {
        Self::new(StepKind::Act, StepOutcome::Success, note)
    }

    /// Atajo: un paso de tipo `Review` con fallo.
    pub fn review_failure(note: impl Into<String>) -> Self {
        Self::new(StepKind::Review, StepOutcome::Failure, note)
    }

    pub fn is_success(&self) -> bool {
        matches!(self.outcome, StepOutcome::Success)
    }

    pub fn is_failure(&self) -> bool {
        matches!(self.outcome, StepOutcome::Failure)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_step_has_defaults() {
        let s = Step::new(StepKind::Act, StepOutcome::Success, "did something");
        assert_eq!(s.kind, StepKind::Act);
        assert_eq!(s.outcome, StepOutcome::Success);
        assert_eq!(s.note, "did something");
        assert_eq!(s.tokens_used, 0);
        assert!(s.request.is_none());
        assert!(s.response.is_none());
    }

    #[test]
    fn act_success_shortcut() {
        let s = Step::act_success("did act");
        assert_eq!(s.kind, StepKind::Act);
        assert!(s.is_success());
        assert!(!s.is_failure());
    }

    #[test]
    fn review_failure_shortcut() {
        let s = Step::review_failure("bad response");
        assert_eq!(s.kind, StepKind::Review);
        assert!(s.is_failure());
        assert!(!s.is_success());
    }

    #[test]
    fn step_ids_are_unique() {
        let a = Step::new(StepKind::Act, StepOutcome::Success, "a");
        let b = Step::new(StepKind::Act, StepOutcome::Success, "b");
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn step_kind_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&StepKind::Understand).unwrap(),
            "\"understand\""
        );
        assert_eq!(
            serde_json::to_string(&StepKind::Replan).unwrap(),
            "\"replan\""
        );
    }

    #[test]
    fn step_outcome_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&StepOutcome::Success).unwrap(),
            "\"success\""
        );
        assert_eq!(
            serde_json::to_string(&StepOutcome::Failure).unwrap(),
            "\"failure\""
        );
    }

    #[test]
    fn step_id_displays() {
        let id = StepId::new();
        let s = id.to_string();
        assert!(!s.is_empty());
        assert!(s.contains('-'));
    }
}
