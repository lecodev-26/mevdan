//! Entidad `Session` del dominio de MEVDAN.
//!
//! Una `Session` representa una unidad de trabajo dentro de un
//! `Project`. Un proyecto puede tener muchas sesiones.
//!
//! Ejemplos de uso futuro:
//!   - Una sesión por tarea.
//!   - Una sesión por agente (para handoff).
//!   - Una sesión por experimento.
//!
//! Los eventos se asocian opcionalmente a una sesión. Los eventos de
//! ciclo de vida del proyecto (ProjectCreated) no tienen sesión; los
//! eventos de trabajo sí.

use crate::ids::{ProjectId, SessionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Estado de una sesión.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// Sesión recién creada, lista para trabajar.
    Active,
    /// Sesión pausada por el usuario.
    Paused,
    /// Sesión terminada exitosamente.
    Completed,
    /// Sesión terminada con errores.
    Failed,
}

/// Una sesión de trabajo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub project_id: ProjectId,
    pub label: Option<String>,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Session {
    /// Crea una sesión nueva en estado `Active`.
    pub fn new(project_id: ProjectId, label: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            project_id,
            label,
            status: SessionStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    /// Cambia el estado de la sesión y actualiza `updated_at`.
    pub fn set_status(&mut self, status: SessionStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_starts_active() {
        let project_id = ProjectId::new();
        let s = Session::new(project_id, Some("test".into()));
        assert_eq!(s.status, SessionStatus::Active);
        assert_eq!(s.project_id, project_id);
        assert_eq!(s.label.as_deref(), Some("test"));
    }

    #[test]
    fn new_session_accepts_no_label() {
        let project_id = ProjectId::new();
        let s = Session::new(project_id, None);
        assert!(s.label.is_none());
    }

    #[test]
    fn set_status_updates_timestamp() {
        let project_id = ProjectId::new();
        let mut s = Session::new(project_id, None);
        let original = s.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(5));
        s.set_status(SessionStatus::Completed);
        assert_eq!(s.status, SessionStatus::Completed);
        assert!(s.updated_at > original);
        assert_eq!(s.created_at, original);
    }

    #[test]
    fn session_roundtrips_through_json() {
        let project_id = ProjectId::new();
        let s = Session::new(project_id, Some("initial".into()));
        let json = serde_json::to_string(&s).unwrap();
        let back: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(s.id, back.id);
        assert_eq!(s.project_id, back.project_id);
        assert_eq!(s.status, back.status);
    }

    #[test]
    fn status_serializes_as_snake_case() {
        let json = serde_json::to_string(&SessionStatus::Active).unwrap();
        assert_eq!(json, "\"active\"");
        let json = serde_json::to_string(&SessionStatus::Completed).unwrap();
        assert_eq!(json, "\"completed\"");
    }
}
