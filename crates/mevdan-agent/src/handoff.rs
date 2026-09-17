//! Agent Handoff — pasar el testigo entre agentes sin perder el trabajo.
//!
//! Un **handoff** es un cambio de agente preservando el estado:
//!
//! ```text
//! Cloud model (gpt-4o) planifica
//!     ↓
//! checkpoint
//!     ↓
//! Local model (llama3.2) ejecuta
//!     ↓
//! checkpoint
//!     ↓
//! Cloud model (claude) revisa
//! ```
//!
//! El handoff no ejecuta trabajo en sí. Registra el cambio, captura
//! el estado actual (checkpoint) y lo deja disponible para el
//! siguiente agente.

use crate::identity::AgentRole;
use chrono::{DateTime, Utc};
use mevdan_checkpoint::CheckpointId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// ID único de un handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HandoffId(pub Uuid);

impl HandoffId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for HandoffId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for HandoffId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Razón por la que se hace un handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffReason {
    /// El agente actual cuesta demasiado para esta tarea.
    Cost,
    /// El siguiente agente es mejor para esta tarea.
    Quality,
    /// El siguiente agente es local (offline).
    Offline,
    /// El agente actual falló.
    Failure,
    /// El usuario pidió el cambio explícitamente.
    Manual,
    /// El siguiente agente tiene una capacidad que el actual no.
    Capability,
}

impl HandoffReason {
    pub fn display_name(&self) -> &'static str {
        match self {
            HandoffReason::Cost => "cost",
            HandoffReason::Quality => "quality",
            HandoffReason::Offline => "offline",
            HandoffReason::Failure => "failure",
            HandoffReason::Manual => "manual",
            HandoffReason::Capability => "capability",
        }
    }
}

/// Petición de handoff (input del usuario o del runtime).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffRequest {
    /// Rol del agente que entrega.
    pub from_role: AgentRole,
    /// Nombre del agente que entrega (informativo).
    pub from_agent_name: String,
    /// Rol del agente que recibe.
    pub to_role: AgentRole,
    /// Nombre del agente que recibe.
    pub to_agent_name: String,
    /// Por qué se hace el cambio.
    pub reason: HandoffReason,
    /// Contexto adicional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl HandoffRequest {
    pub fn new(
        from_role: AgentRole,
        from_agent_name: impl Into<String>,
        to_role: AgentRole,
        to_agent_name: impl Into<String>,
        reason: HandoffReason,
    ) -> Self {
        Self {
            from_role,
            from_agent_name: from_agent_name.into(),
            to_role,
            to_agent_name: to_agent_name.into(),
            reason,
            note: None,
        }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// Un handoff registrado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handoff {
    pub id: HandoffId,
    pub from_role: AgentRole,
    pub from_agent_name: String,
    pub to_role: AgentRole,
    pub to_agent_name: String,
    pub reason: HandoffReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Checkpoint creado en el momento del handoff (si aplica).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint_id: Option<CheckpointId>,
    pub occurred_at: DateTime<Utc>,
}

impl Handoff {
    /// Crea un handoff a partir de una petición.
    pub fn from_request(request: HandoffRequest) -> Self {
        Self {
            id: HandoffId::new(),
            from_role: request.from_role,
            from_agent_name: request.from_agent_name,
            to_role: request.to_role,
            to_agent_name: request.to_agent_name,
            reason: request.reason,
            note: request.note,
            checkpoint_id: None,
            occurred_at: Utc::now(),
        }
    }

    /// Asocia un checkpoint.
    pub fn with_checkpoint(mut self, id: CheckpointId) -> Self {
        self.checkpoint_id = Some(id);
        self
    }

    /// Resumen textual.
    ///
    /// Formato: `from_role (from_agent_name) → to_role (to_agent_name) [reason, with checkpoint?]`
    ///
    /// Los roles se muestran en minúscula para facilitar la búsqueda
    /// en logs y matches de texto.
    pub fn summary(&self) -> String {
        format!(
            "{} ({}) → {} ({}) [{}{}]",
            self.from_role.display_name().to_lowercase(),
            self.from_agent_name,
            self.to_role.display_name().to_lowercase(),
            self.to_agent_name,
            self.reason.display_name(),
            if self.checkpoint_id.is_some() {
                ", with checkpoint"
            } else {
                ""
            },
        )
    }
}

/// Historial de handoffs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HandoffHistory {
    handoffs: Vec<Handoff>,
}

impl HandoffHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra un handoff.
    pub fn record(&mut self, handoff: Handoff) {
        self.handoffs.push(handoff);
    }

    pub fn len(&self) -> usize {
        self.handoffs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.handoffs.is_empty()
    }

    /// Todos los handoffs registrados.
    pub fn all(&self) -> &[Handoff] {
        &self.handoffs
    }

    /// Último handoff (si hay).
    pub fn last(&self) -> Option<&Handoff> {
        self.handoffs.last()
    }

    /// Handoffs que involucran un rol (como origen o destino).
    pub fn involving_role(&self, role: AgentRole) -> Vec<&Handoff> {
        self.handoffs
            .iter()
            .filter(|h| h.from_role == role || h.to_role == role)
            .collect()
    }

    /// Handoffs con una razón concreta.
    pub fn by_reason(&self, reason: HandoffReason) -> Vec<&Handoff> {
        self.handoffs
            .iter()
            .filter(|h| h.reason == reason)
            .collect()
    }

    /// Limpia el historial.
    pub fn clear(&mut self) {
        self.handoffs.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_id_unique() {
        let a = HandoffId::new();
        let b = HandoffId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn reason_display_names() {
        assert_eq!(HandoffReason::Cost.display_name(), "cost");
        assert_eq!(HandoffReason::Failure.display_name(), "failure");
        assert_eq!(HandoffReason::Capability.display_name(), "capability");
    }

    #[test]
    fn reason_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&HandoffReason::Cost).unwrap(),
            "\"cost\""
        );
        assert_eq!(
            serde_json::to_string(&HandoffReason::Offline).unwrap(),
            "\"offline\""
        );
    }

    #[test]
    fn request_new_creates() {
        let r = HandoffRequest::new(
            AgentRole::Planner,
            "gpt-4o",
            AgentRole::Coder,
            "llama3.2",
            HandoffReason::Cost,
        );
        assert_eq!(r.from_role, AgentRole::Planner);
        assert_eq!(r.to_role, AgentRole::Coder);
        assert_eq!(r.reason, HandoffReason::Cost);
        assert!(r.note.is_none());
    }

    #[test]
    fn request_with_note() {
        let r = HandoffRequest::new(
            AgentRole::Planner,
            "a",
            AgentRole::Coder,
            "b",
            HandoffReason::Manual,
        )
        .with_note("user asked");
        assert_eq!(r.note.as_deref(), Some("user asked"));
    }

    #[test]
    fn handoff_from_request() {
        let r = HandoffRequest::new(
            AgentRole::Planner,
            "gpt-4o",
            AgentRole::Coder,
            "llama3.2",
            HandoffReason::Cost,
        );
        let h = Handoff::from_request(r);
        assert_eq!(h.from_role, AgentRole::Planner);
        assert_eq!(h.to_role, AgentRole::Coder);
        assert_eq!(h.reason, HandoffReason::Cost);
        assert!(h.checkpoint_id.is_none());
    }

    #[test]
    fn handoff_with_checkpoint() {
        let r = HandoffRequest::new(
            AgentRole::Planner,
            "a",
            AgentRole::Coder,
            "b",
            HandoffReason::Quality,
        );
        let cp_id = CheckpointId::new();
        let h = Handoff::from_request(r).with_checkpoint(cp_id);
        assert_eq!(h.checkpoint_id, Some(cp_id));
    }

    #[test]
    fn handoff_summary() {
        let r = HandoffRequest::new(
            AgentRole::Planner,
            "gpt-4o",
            AgentRole::Coder,
            "llama3.2",
            HandoffReason::Cost,
        );
        let h = Handoff::from_request(r);
        let s = h.summary();
        assert!(s.contains("planner"), "summary: {}", s);
        assert!(s.contains("coder"), "summary: {}", s);
        assert!(s.contains("cost"), "summary: {}", s);
    }

    #[test]
    fn history_new_is_empty() {
        let h = HandoffHistory::new();
        assert!(h.is_empty());
        assert_eq!(h.len(), 0);
    }

    #[test]
    fn history_record_increments() {
        let mut h = HandoffHistory::new();
        let req = HandoffRequest::new(
            AgentRole::Planner,
            "a",
            AgentRole::Coder,
            "b",
            HandoffReason::Manual,
        );
        h.record(Handoff::from_request(req));
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn history_last_returns_most_recent() {
        let mut h = HandoffHistory::new();
        for reason in [HandoffReason::Cost, HandoffReason::Quality] {
            let req = HandoffRequest::new(AgentRole::Planner, "a", AgentRole::Coder, "b", reason);
            h.record(Handoff::from_request(req));
        }
        assert_eq!(h.last().unwrap().reason, HandoffReason::Quality);
    }

    #[test]
    fn history_involving_role() {
        let mut h = HandoffHistory::new();
        h.record(Handoff::from_request(HandoffRequest::new(
            AgentRole::Planner,
            "a",
            AgentRole::Coder,
            "b",
            HandoffReason::Cost,
        )));
        h.record(Handoff::from_request(HandoffRequest::new(
            AgentRole::Coder,
            "b",
            AgentRole::Reviewer,
            "c",
            HandoffReason::Quality,
        )));

        let coder_hs = h.involving_role(AgentRole::Coder);
        assert_eq!(coder_hs.len(), 2);

        let tester_hs = h.involving_role(AgentRole::Tester);
        assert_eq!(tester_hs.len(), 0);
    }

    #[test]
    fn history_by_reason() {
        let mut h = HandoffHistory::new();
        h.record(Handoff::from_request(HandoffRequest::new(
            AgentRole::Planner,
            "a",
            AgentRole::Coder,
            "b",
            HandoffReason::Cost,
        )));
        h.record(Handoff::from_request(HandoffRequest::new(
            AgentRole::Coder,
            "b",
            AgentRole::Reviewer,
            "c",
            HandoffReason::Cost,
        )));
        h.record(Handoff::from_request(HandoffRequest::new(
            AgentRole::Reviewer,
            "c",
            AgentRole::Tester,
            "d",
            HandoffReason::Quality,
        )));

        assert_eq!(h.by_reason(HandoffReason::Cost).len(), 2);
        assert_eq!(h.by_reason(HandoffReason::Quality).len(), 1);
        assert_eq!(h.by_reason(HandoffReason::Failure).len(), 0);
    }

    #[test]
    fn history_clear() {
        let mut h = HandoffHistory::new();
        h.record(Handoff::from_request(HandoffRequest::new(
            AgentRole::Planner,
            "a",
            AgentRole::Coder,
            "b",
            HandoffReason::Manual,
        )));
        h.clear();
        assert!(h.is_empty());
    }

    #[test]
    fn history_all_returns_slice() {
        let mut h = HandoffHistory::new();
        h.record(Handoff::from_request(HandoffRequest::new(
            AgentRole::Planner,
            "a",
            AgentRole::Coder,
            "b",
            HandoffReason::Manual,
        )));
        assert_eq!(h.all().len(), 1);
    }

    #[test]
    fn handoff_serializes() {
        let req = HandoffRequest::new(
            AgentRole::Planner,
            "a",
            AgentRole::Coder,
            "b",
            HandoffReason::Cost,
        );
        let h = Handoff::from_request(req);
        let json = serde_json::to_string(&h).unwrap();
        let back: Handoff = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, h.id);
        assert_eq!(back.reason, h.reason);
    }

    #[test]
    fn history_serializes() {
        let mut h = HandoffHistory::new();
        h.record(Handoff::from_request(HandoffRequest::new(
            AgentRole::Planner,
            "a",
            AgentRole::Coder,
            "b",
            HandoffReason::Manual,
        )));
        let json = serde_json::to_string(&h).unwrap();
        let back: HandoffHistory = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), 1);
    }
}
