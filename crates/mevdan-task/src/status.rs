//! Estado y prioridad de tareas.

use serde::{Deserialize, Serialize};

/// Estado de una tarea.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Creada, esperando a estar lista (puede tener dependencias).
    Pending,
    /// Todas las dependencias están resueltas; lista para ejecutarse.
    Ready,
    /// En ejecución.
    Running,
    /// Bloqueada (dependencias no resueltas, o esperando algo).
    Blocked,
    /// Esperando aprobación del usuario.
    WaitingApproval,
    /// Completada con éxito.
    Completed,
    /// Falló.
    Failed,
    /// Cancelada por el usuario o el runtime.
    Cancelled,
}

impl TaskStatus {
    pub fn display_name(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Ready => "ready",
            TaskStatus::Running => "running",
            TaskStatus::Blocked => "blocked",
            TaskStatus::WaitingApproval => "waiting_approval",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
        }
    }

    /// ¿Es un estado terminal (no puede cambiar más)?
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
        )
    }

    /// ¿Es un estado activo (la tarea está en juego)?
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            TaskStatus::Ready | TaskStatus::Running | TaskStatus::WaitingApproval
        )
    }

    /// ¿Se puede transicionar desde este estado a `next`?
    ///
    /// Reglas:
    /// - `Pending` → `Ready`, `Blocked`, `Cancelled`.
    /// - `Ready` → `Running`, `Blocked`, `Cancelled`.
    /// - `Running` → `Completed`, `Failed`, `WaitingApproval`, `Blocked`, `Cancelled`.
    /// - `Blocked` → `Ready`, `Pending`, `Cancelled`.
    /// - `WaitingApproval` → `Running`, `Failed`, `Cancelled`.
    /// - Estados terminales → nada.
    pub fn can_transition_to(&self, next: TaskStatus) -> bool {
        use TaskStatus::*;
        matches!(
            (self, next),
            // Desde Pending
            (Pending, Ready)
                | (Pending, Blocked)
                | (Pending, Cancelled)
            // Desde Ready
                | (Ready, Running)
                | (Ready, Blocked)
                | (Ready, Cancelled)
            // Desde Running
                | (Running, Completed)
                | (Running, Failed)
                | (Running, WaitingApproval)
                | (Running, Blocked)
                | (Running, Cancelled)
            // Desde Blocked
                | (Blocked, Ready)
                | (Blocked, Pending)
                | (Blocked, Cancelled)
            // Desde WaitingApproval
                | (WaitingApproval, Running)
                | (WaitingApproval, Failed)
                | (WaitingApproval, Cancelled)
        )
    }
}

/// Prioridad de una tarea.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

impl TaskPriority {
    pub fn display_name(&self) -> &'static str {
        match self {
            TaskPriority::Low => "low",
            TaskPriority::Normal => "normal",
            TaskPriority::High => "high",
            TaskPriority::Critical => "critical",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_display_names() {
        assert_eq!(TaskStatus::Pending.display_name(), "pending");
        assert_eq!(
            TaskStatus::WaitingApproval.display_name(),
            "waiting_approval"
        );
        assert_eq!(TaskStatus::Completed.display_name(), "completed");
    }

    #[test]
    fn status_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&TaskStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::WaitingApproval).unwrap(),
            "\"waiting_approval\""
        );
    }

    #[test]
    fn terminal_states() {
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
        assert!(!TaskStatus::Pending.is_terminal());
        assert!(!TaskStatus::Running.is_terminal());
    }

    #[test]
    fn active_states() {
        assert!(TaskStatus::Ready.is_active());
        assert!(TaskStatus::Running.is_active());
        assert!(TaskStatus::WaitingApproval.is_active());
        assert!(!TaskStatus::Pending.is_active());
        assert!(!TaskStatus::Completed.is_active());
    }

    #[test]
    fn transition_pending_to_ready() {
        assert!(TaskStatus::Pending.can_transition_to(TaskStatus::Ready));
    }

    #[test]
    fn transition_pending_to_completed_is_invalid() {
        assert!(!TaskStatus::Pending.can_transition_to(TaskStatus::Completed));
    }

    #[test]
    fn transition_running_to_completed() {
        assert!(TaskStatus::Running.can_transition_to(TaskStatus::Completed));
        assert!(TaskStatus::Running.can_transition_to(TaskStatus::Failed));
    }

    #[test]
    fn transition_terminal_to_anything_is_invalid() {
        for status in [
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
        ] {
            assert!(!status.can_transition_to(TaskStatus::Running));
            assert!(!status.can_transition_to(TaskStatus::Pending));
            assert!(!status.can_transition_to(TaskStatus::Ready));
        }
    }

    #[test]
    fn transition_blocked_to_ready() {
        assert!(TaskStatus::Blocked.can_transition_to(TaskStatus::Ready));
    }

    #[test]
    fn transition_waiting_approval_to_running() {
        assert!(TaskStatus::WaitingApproval.can_transition_to(TaskStatus::Running));
    }

    #[test]
    fn priority_default_is_normal() {
        assert_eq!(TaskPriority::default(), TaskPriority::Normal);
    }

    #[test]
    fn priority_ordering() {
        assert!(TaskPriority::Low < TaskPriority::Normal);
        assert!(TaskPriority::Normal < TaskPriority::High);
        assert!(TaskPriority::High < TaskPriority::Critical);
    }

    #[test]
    fn priority_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&TaskPriority::Low).unwrap(),
            "\"low\""
        );
        assert_eq!(
            serde_json::to_string(&TaskPriority::Critical).unwrap(),
            "\"critical\""
        );
    }
}
