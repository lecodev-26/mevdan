//! Event Log append-only de MEVDAN.
//!
//! Reglas duras (Regla 15 del prompt fundacional):
//!   1. Los eventos son INMUTABLES. Nunca se editan.
//!   2. Los eventos son APPEND-ONLY. Nunca se borran.
//!   3. Todo cambio de estado pasa por un evento.
//!   4. Los tipos de evento se AÑADEN, nunca se reemplazan.
//!
//! El Event Log es la base de:
//!   - `mevdan audit`       (Fase 10)
//!   - `mevdan checkpoint`  (Fase 8)
//!   - `mevdan rollback`    (Fase 8)
//!   - Verificación de claims (Fase 9)

use crate::ids::{EventId, ProjectId, SessionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Un evento inmutable en el log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub project_id: ProjectId,
    /// Algunos eventos no pertenecen a una sesión (ej: ProjectCreated).
    pub session_id: Option<SessionId>,
    pub occurred_at: DateTime<Utc>,
    pub kind: EventKind,
    /// Payload libre. El contenido depende del `kind`.
    /// Se serializa como JSON opaco; el dominio no lo interpreta.
    pub payload: serde_json::Value,
}

/// Tipos de evento conocidos.
///
/// Regla: AÑADIR variantes. Nunca eliminar ni renombrar una variante
/// existente, porque rompería los logs históricos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    // ─── Ciclo de vida (activos en M1) ──────────────────────
    ProjectCreated,
    SessionStarted,
    SessionEnded,
    // ─── Placeholders para fases futuras ────────────────────
    // Se activan al implementar cada fase. No se emiten todavía.
    //
    // Fase 4 — Agent Engine
    // AgentStarted,
    // AgentStopped,
    //
    // Fase 5 — Tools
    // ToolInvoked,
    // ToolCompleted,
    // ToolFailed,
    //
    // Fase 6 — Permissions
    // PermissionRequested,
    // PermissionGranted,
    // PermissionDenied,
    //
    // Fase 7 — Work Graph
    // TaskCreated,
    // TaskCompleted,
    // ArtifactCreated,
    //
    // Fase 8 — Checkpoints
    // CheckpointCreated,
    // CheckpointRestored,
    //
    // Fase 9 — Verification
    // VerificationExecuted,
    // VerificationPassed,
    // VerificationFailed,
}

impl Event {
    /// Crea un evento nuevo. No lo persiste — eso lo hace `mevdan-storage`.
    pub fn new(
        project_id: ProjectId,
        session_id: Option<SessionId>,
        kind: EventKind,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: EventId::new(),
            project_id,
            session_id,
            occurred_at: Utc::now(),
            kind,
            payload,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_event_sets_fields() {
        let pid = ProjectId::new();
        let sid = SessionId::new();
        let e = Event::new(
            pid,
            Some(sid),
            EventKind::SessionStarted,
            serde_json::json!({"label": "test"}),
        );
        assert_eq!(e.project_id, pid);
        assert_eq!(e.session_id, Some(sid));
        assert_eq!(e.kind, EventKind::SessionStarted);
        assert_eq!(e.payload["label"], "test");
    }

    #[test]
    fn event_without_session_is_valid() {
        let pid = ProjectId::new();
        let e = Event::new(pid, None, EventKind::ProjectCreated, serde_json::json!({}));
        assert!(e.session_id.is_none());
    }

    #[test]
    fn event_kind_serializes_as_snake_case() {
        let json = serde_json::to_string(&EventKind::ProjectCreated).unwrap();
        assert_eq!(json, "\"project_created\"");
        let json = serde_json::to_string(&EventKind::SessionStarted).unwrap();
        assert_eq!(json, "\"session_started\"");
    }

    #[test]
    fn event_roundtrips_through_json() {
        let e = Event::new(
            ProjectId::new(),
            Some(SessionId::new()),
            EventKind::ProjectCreated,
            serde_json::json!({"name": "demo", "mevdan_version": "0.1.0"}),
        );
        let s = serde_json::to_string(&e).unwrap();
        let back: Event = serde_json::from_str(&s).unwrap();
        assert_eq!(e.id, back.id);
        assert_eq!(e.kind, back.kind);
        assert_eq!(e.payload, back.payload);
    }

    #[test]
    fn event_id_is_unique() {
        let pid = ProjectId::new();
        let a = Event::new(pid, None, EventKind::ProjectCreated, serde_json::json!({}));
        let b = Event::new(pid, None, EventKind::ProjectCreated, serde_json::json!({}));
        assert_ne!(a.id, b.id);
    }
}
