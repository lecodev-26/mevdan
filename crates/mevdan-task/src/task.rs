//! Entidad `Task`.

use crate::{
    error::{TaskError, TaskResult},
    id::TaskId,
    status::{TaskPriority, TaskStatus},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Una tarea.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    /// Título corto.
    pub title: String,
    /// Descripción extendida (opcional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Estado actual.
    pub status: TaskStatus,
    /// Prioridad.
    pub priority: TaskPriority,
    /// IDs de las tareas de las que depende esta.
    #[serde(default)]
    pub depends_on: Vec<TaskId>,
    /// Resultado textual (al completar/fallar).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    /// Crea una tarea nueva en estado `Pending`.
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: TaskId::new(),
            title: title.into(),
            description: None,
            status: TaskStatus::Pending,
            priority: TaskPriority::Normal,
            depends_on: Vec::new(),
            result: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Sobrescribe la descripción.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sobrescribe la prioridad.
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Añade dependencias.
    pub fn with_depends_on(mut self, deps: Vec<TaskId>) -> Self {
        self.depends_on = deps;
        self
    }

    /// ¿El título está vacío?
    pub fn is_blank(&self) -> bool {
        self.title.trim().is_empty()
    }

    /// ¿Está en un estado terminal?
    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }

    /// ¿Está activa?
    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }

    /// Cambia el estado con validación.
    pub fn transition_to(&mut self, next: TaskStatus) -> TaskResult<()> {
        if !self.status.can_transition_to(next) {
            return Err(TaskError::InvalidTransition {
                task: self.id.to_string(),
                from: self.status.display_name().to_string(),
                to: next.display_name().to_string(),
            });
        }
        self.status = next;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Marca como completada con un resultado.
    pub fn complete(&mut self, result: impl Into<String>) -> TaskResult<()> {
        self.transition_to(TaskStatus::Completed)?;
        self.result = Some(result.into());
        Ok(())
    }

    /// Marca como fallida con una razón.
    pub fn fail(&mut self, reason: impl Into<String>) -> TaskResult<()> {
        self.transition_to(TaskStatus::Failed)?;
        self.result = Some(reason.into());
        Ok(())
    }

    /// Actualiza `updated_at`.
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_task_defaults() {
        let t = Task::new("do something");
        assert_eq!(t.title, "do something");
        assert_eq!(t.status, TaskStatus::Pending);
        assert_eq!(t.priority, TaskPriority::Normal);
        assert!(t.depends_on.is_empty());
        assert!(t.description.is_none());
        assert!(t.result.is_none());
    }

    #[test]
    fn with_description_sets_it() {
        let t = Task::new("t").with_description("longer text");
        assert_eq!(t.description.as_deref(), Some("longer text"));
    }

    #[test]
    fn with_priority_sets_it() {
        let t = Task::new("t").with_priority(TaskPriority::High);
        assert_eq!(t.priority, TaskPriority::High);
    }

    #[test]
    fn with_depends_on_sets_it() {
        let dep = TaskId::new();
        let t = Task::new("t").with_depends_on(vec![dep]);
        assert_eq!(t.depends_on, vec![dep]);
    }

    #[test]
    fn is_blank_works() {
        assert!(Task::new("").is_blank());
        assert!(Task::new("   ").is_blank());
        assert!(!Task::new("ok").is_blank());
    }

    #[test]
    fn is_terminal_works() {
        let mut t = Task::new("t");
        assert!(!t.is_terminal());
        t.status = TaskStatus::Completed;
        assert!(t.is_terminal());
    }

    #[test]
    fn is_active_works() {
        let mut t = Task::new("t");
        assert!(!t.is_active());
        t.status = TaskStatus::Running;
        assert!(t.is_active());
    }

    #[test]
    fn transition_to_valid_status() {
        let mut t = Task::new("t");
        assert!(t.transition_to(TaskStatus::Ready).is_ok());
        assert_eq!(t.status, TaskStatus::Ready);
    }

    #[test]
    fn transition_to_invalid_status_fails() {
        let mut t = Task::new("t");
        let err = t.transition_to(TaskStatus::Completed).unwrap_err();
        assert!(matches!(err, TaskError::InvalidTransition { .. }));
        // Status no cambió.
        assert_eq!(t.status, TaskStatus::Pending);
    }

    #[test]
    fn complete_works() {
        let mut t = Task::new("t");
        t.transition_to(TaskStatus::Ready).unwrap();
        t.transition_to(TaskStatus::Running).unwrap();
        t.complete("done").unwrap();
        assert_eq!(t.status, TaskStatus::Completed);
        assert_eq!(t.result.as_deref(), Some("done"));
    }

    #[test]
    fn fail_works() {
        let mut t = Task::new("t");
        t.transition_to(TaskStatus::Ready).unwrap();
        t.transition_to(TaskStatus::Running).unwrap();
        t.fail("boom").unwrap();
        assert_eq!(t.status, TaskStatus::Failed);
        assert_eq!(t.result.as_deref(), Some("boom"));
    }

    #[test]
    fn touch_updates_timestamp() {
        let mut t = Task::new("t");
        let original = t.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(5));
        t.touch();
        assert!(t.updated_at > original);
        assert_eq!(t.created_at, original);
    }

    #[test]
    fn task_roundtrips() {
        let t = Task::new("test")
            .with_description("desc")
            .with_priority(TaskPriority::High);
        let json = serde_json::to_string(&t).unwrap();
        let back: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, t.id);
        assert_eq!(back.title, t.title);
        assert_eq!(back.priority, t.priority);
    }
}
