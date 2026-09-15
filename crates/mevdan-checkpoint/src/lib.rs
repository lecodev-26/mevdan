//! # mevdan-checkpoint
//!
//! Checkpoints y recuperación del trabajo.
//!
//! ## Concepto
//!
//! Un **checkpoint** captura el estado completo del trabajo:
//! Work Graph, tareas, artefactos, claims, metadatos.
//!
//! La **recuperación** permite volver a un estado anterior:
//!
//! - `resume` — continuar desde el último checkpoint.
//! - `retry` — reintentar una tarea fallida.
//! - `rollback` — volver a un checkpoint concreto.
//! - `continue` — avanzar desde el estado actual.
//!
//! ## Estado del proyecto
//!
//! - **V3.4** ✅ — `Checkpoint`, `WorkState`, `CheckpointEngine`, persistencia SQLite.
//! - **V3.5** ✅ — `RecoveryEngine`, `RecoveryAction`, `RecoveryResult`.
//! - **V3.6** ⏳ — Audit + Replay.

pub mod checkpoint;
pub mod engine;
pub mod error;
pub mod id;
pub mod recovery;
pub mod state;

// Re-exports de conveniencia.
pub use checkpoint::Checkpoint;
pub use engine::CheckpointEngine;
pub use error::{CheckpointError, CheckpointResult};
pub use id::CheckpointId;
pub use recovery::{RecoveryAction, RecoveryEngine, RecoveryResult};
pub use state::{WorkState, WORK_STATE_SCHEMA_VERSION};

#[cfg(test)]
mod tests {
    use super::*;
    use mevdan_task::Task;

    #[test]
    fn full_flow_create_and_restore() {
        let mut engine = CheckpointEngine::new();

        let state1 = WorkState::new()
            .with_task(Task::new("task 1"))
            .with_task(Task::new("task 2"));

        let cp1 = engine.create(state1, Some("initial".into()));
        assert_eq!(engine.len(), 1);

        let state2 = WorkState::new()
            .with_task(Task::new("task 1"))
            .with_task(Task::new("task 2"))
            .with_task(Task::new("task 3"));

        let _cp2 = engine.create(state2, Some("after-task3".into()));
        assert_eq!(engine.len(), 2);

        let restored = engine.restore(cp1).unwrap();
        assert_eq!(restored.tasks.len(), 2);
    }

    #[test]
    fn full_flow_recovery() {
        let mut engine = CheckpointEngine::new();

        // Estado inicial.
        let state = WorkState::new()
            .with_task(Task::new("a"))
            .with_task(Task::new("b"));
        let id = engine.create(state, Some("before-change".into()));

        // Recovery engine.
        let recovery = RecoveryEngine::new(&engine);

        // Rollback al checkpoint.
        let result = recovery.rollback(id).unwrap();
        assert_eq!(result.action, RecoveryAction::Rollback);
        assert_eq!(result.state.tasks.len(), 2);

        // Resume desde el último.
        let result = recovery.resume().unwrap();
        assert_eq!(result.action, RecoveryAction::Resume);
        assert!(result.recovered());
    }

    #[test]
    fn full_flow_retry() {
        use mevdan_task::TaskStatus;

        let mut engine = CheckpointEngine::new();

        let mut failed_task = Task::new("failing");
        failed_task.status = TaskStatus::Failed;
        failed_task.result = Some("error".into());
        let task_id = failed_task.id;

        engine.create(WorkState::new().with_task(failed_task), None);

        let recovery = RecoveryEngine::new(&engine);
        let result = recovery.retry(task_id).unwrap();

        assert_eq!(result.tasks_to_retry, vec![task_id]);
        let restored = result.state.tasks.iter().find(|t| t.id == task_id).unwrap();
        assert_eq!(restored.status, TaskStatus::Ready);
    }
}
