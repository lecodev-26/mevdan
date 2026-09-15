//! # mevdan-task
//!
//! Gestión de tareas de MEVDAN.
//!
//! ## Concepto
//!
//! Una **tarea** es una unidad de trabajo con un estado bien definido.
//! El `TaskEngine` gestiona:
//!
//! - Creación y eliminación.
//! - Transiciones de estado (con validación).
//! - Dependencias entre tareas (con detección de ciclos).
//! - Cálculo de tareas "listas" (dependencias satisfechas).
//! - Orden topológico.
//!
//! ## Estados y transiciones
//!
//! ```text
//! Pending ──► Ready ──► Running ──► Completed
//!    │         │          │
//!    │         │          ├──► WaitingApproval ──► Running
//!    │         │          │
//!    ▼         ▼          ▼
//! Blocked ◄────┴──────────┘
//!    │
//!    ▼
//!  Ready / Pending / Cancelled
//! ```
//!
//! ## Qué NO hace
//!
//! - No ejecuta tareas. La ejecución la hace `mevdan-agent`.
//! - No persiste. La persistencia la hará `mevdan-storage` en V3.9.
//! - No conoce al Work Graph. La conversión se hace en V3.9.

pub mod engine;
pub mod error;
pub mod id;
pub mod status;
pub mod task;

// Re-exports de conveniencia.
pub use engine::TaskEngine;
pub use error::{TaskError, TaskResult};
pub use id::TaskId;
pub use status::{TaskPriority, TaskStatus};
pub use task::Task;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_flow_task_lifecycle() {
        let mut engine = TaskEngine::new();

        let t = engine
            .add(Task::new("do something").with_priority(TaskPriority::High))
            .unwrap();
        assert_eq!(engine.len(), 1);

        engine.transition(t, TaskStatus::Ready).unwrap();
        engine.transition(t, TaskStatus::Running).unwrap();

        let task = engine.get_mut(t).unwrap();
        task.complete("done").unwrap();

        let task = engine.get(t).unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
        assert_eq!(task.result.as_deref(), Some("done"));
    }

    #[test]
    fn full_flow_with_dependencies() {
        let mut engine = TaskEngine::new();

        let a = engine.add(Task::new("A")).unwrap();
        let b = engine.add(Task::new("B")).unwrap();
        let c = engine.add(Task::new("C")).unwrap();

        engine.add_dependency(b, a).unwrap();
        engine.add_dependency(c, b).unwrap();

        // Resolución en cadena.
        engine.refresh_ready_tasks();
        assert_eq!(engine.get(a).unwrap().status, TaskStatus::Ready);

        engine.transition(a, TaskStatus::Running).unwrap();
        engine.transition(a, TaskStatus::Completed).unwrap();

        engine.refresh_ready_tasks();
        assert_eq!(engine.get(b).unwrap().status, TaskStatus::Ready);

        engine.transition(b, TaskStatus::Running).unwrap();
        engine.transition(b, TaskStatus::Completed).unwrap();

        engine.refresh_ready_tasks();
        assert_eq!(engine.get(c).unwrap().status, TaskStatus::Ready);

        engine.transition(c, TaskStatus::Running).unwrap();
        engine.transition(c, TaskStatus::Completed).unwrap();

        assert!(engine.all_terminal());
    }
}
