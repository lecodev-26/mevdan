//! Recovery: recuperación desde checkpoints.
//!
//! Usando los checkpoints guardados, este módulo permite:
//!
//! - **`resume`** — continuar desde el último checkpoint.
//! - **`retry`** — reintentar una tarea fallida desde un checkpoint.
//! - **`rollback`** — volver atrás a un checkpoint (restaura estado).
//! - **`continue`** — avanzar desde donde se quedó, sin checkpoint.
//!
//! ## Modelo
//!
//! El `RecoveryEngine` recibe un `CheckpointEngine` como fuente de
//! checkpoints. Las operaciones **no ejecutan trabajo real**; solo
//! producen el `WorkState` que el runtime debe adoptar.

use crate::{
    engine::CheckpointEngine,
    error::{CheckpointError, CheckpointResult},
    id::CheckpointId,
    state::WorkState,
};
use mevdan_task::{TaskId, TaskStatus};
use serde::{Deserialize, Serialize};

/// Acción de recuperación.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryAction {
    /// Continuar desde el último checkpoint disponible.
    Resume,
    /// Reintentar una tarea fallida.
    Retry,
    /// Volver a un checkpoint concreto.
    Rollback,
    /// Avanzar desde el estado actual.
    Continue,
}

impl RecoveryAction {
    pub fn display_name(&self) -> &'static str {
        match self {
            RecoveryAction::Resume => "resume",
            RecoveryAction::Retry => "retry",
            RecoveryAction::Rollback => "rollback",
            RecoveryAction::Continue => "continue",
        }
    }
}

/// Resultado de una operación de recuperación.
#[derive(Debug, Clone)]
pub struct RecoveryResult {
    /// Acción ejecutada.
    pub action: RecoveryAction,
    /// Checkpoint usado (si aplica).
    pub checkpoint_id: Option<CheckpointId>,
    /// Estado restaurado (o el estado actual en `Continue`).
    pub state: WorkState,
    /// Descripción legible.
    pub description: String,
    /// Tareas que quedaron listas para reintentar.
    pub tasks_to_retry: Vec<TaskId>,
}

impl RecoveryResult {
    /// ¿Se pudo recuperar?
    pub fn recovered(&self) -> bool {
        !self.state.is_empty()
    }
}

/// Motor de recuperación.
pub struct RecoveryEngine<'a> {
    checkpoints: &'a CheckpointEngine,
}

impl<'a> RecoveryEngine<'a> {
    /// Crea un motor a partir de un `CheckpointEngine` prestado.
    pub fn new(checkpoints: &'a CheckpointEngine) -> Self {
        Self { checkpoints }
    }

    /// Continuar desde el último checkpoint.
    pub fn resume(&self) -> CheckpointResult<RecoveryResult> {
        let cp = self
            .checkpoints
            .list_recent(1)
            .into_iter()
            .next()
            .ok_or(CheckpointError::NoCheckpointsAvailable)?;

        Ok(RecoveryResult {
            action: RecoveryAction::Resume,
            checkpoint_id: Some(cp.id),
            state: cp.state.clone(),
            description: format!("resumed from checkpoint '{}'", cp.display_label()),
            tasks_to_retry: Vec::new(),
        })
    }

    /// Reintentar una tarea fallida.
    ///
    /// Busca la tarea en el estado del último checkpoint. Si existe y
    /// está `Failed`, la prepara para reintentar (la marca como
    /// `Ready`).
    pub fn retry(&self, task_id: TaskId) -> CheckpointResult<RecoveryResult> {
        let cp = self
            .checkpoints
            .list_recent(1)
            .into_iter()
            .next()
            .ok_or(CheckpointError::NoCheckpointsAvailable)?;

        let mut state = cp.state.clone();

        // Encontrar la tarea.
        let task = state
            .tasks
            .iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| CheckpointError::TaskNotFoundInState(task_id.to_string()))?;

        // Solo se pueden reintentar tareas fallidas.
        if task.status != TaskStatus::Failed {
            return Err(CheckpointError::CannotRollback);
        }

        // Resetear la tarea a Ready para reintentar.
        task.status = TaskStatus::Ready;
        task.result = None;
        task.touch();

        Ok(RecoveryResult {
            action: RecoveryAction::Retry,
            checkpoint_id: Some(cp.id),
            state,
            description: format!("task {} marked for retry", task_id),
            tasks_to_retry: vec![task_id],
        })
    }

    /// Volver a un checkpoint concreto.
    pub fn rollback(&self, id: CheckpointId) -> CheckpointResult<RecoveryResult> {
        let cp = self.checkpoints.require(id)?;
        Ok(RecoveryResult {
            action: RecoveryAction::Rollback,
            checkpoint_id: Some(cp.id),
            state: cp.state.clone(),
            description: format!("rolled back to checkpoint '{}'", cp.display_label()),
            tasks_to_retry: Vec::new(),
        })
    }

    /// Continuar desde el estado actual (sin checkpoint).
    ///
    /// Útil cuando el runtime ya tiene el estado en memoria y solo
    /// quiere saber qué tareas están listas para seguir.
    pub fn continue_from(&self, state: WorkState) -> RecoveryResult {
        // Identifica tareas en estado `Ready` para ejecutar.
        let ready: Vec<TaskId> = state
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Ready)
            .map(|t| t.id)
            .collect();

        RecoveryResult {
            action: RecoveryAction::Continue,
            checkpoint_id: None,
            state,
            description: format!(
                "continuing from current state ({} ready tasks)",
                ready.len()
            ),
            tasks_to_retry: ready,
        }
    }

    /// Estrategia de recuperación automática:
    /// si hay checkpoints, hace `resume`; si no, error.
    pub fn auto_resume(&self) -> CheckpointResult<RecoveryResult> {
        self.resume()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{engine::CheckpointEngine, state::WorkState};
    use mevdan_task::{Task, TaskStatus};

    fn setup_with_checkpoint() -> (CheckpointEngine, CheckpointId) {
        let mut e = CheckpointEngine::new();
        let state = WorkState::new()
            .with_task(Task::new("task 1"))
            .with_task(Task::new("task 2"));
        let id = e.create(state, Some("initial".into()));
        (e, id)
    }

    #[test]
    fn action_display_names() {
        assert_eq!(RecoveryAction::Resume.display_name(), "resume");
        assert_eq!(RecoveryAction::Retry.display_name(), "retry");
        assert_eq!(RecoveryAction::Rollback.display_name(), "rollback");
        assert_eq!(RecoveryAction::Continue.display_name(), "continue");
    }

    #[test]
    fn action_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&RecoveryAction::Resume).unwrap(),
            "\"resume\""
        );
        assert_eq!(
            serde_json::to_string(&RecoveryAction::Rollback).unwrap(),
            "\"rollback\""
        );
    }

    #[test]
    fn resume_with_no_checkpoints_fails() {
        let e = CheckpointEngine::new();
        let recovery = RecoveryEngine::new(&e);
        let err = recovery.resume().unwrap_err();
        assert!(matches!(err, CheckpointError::NoCheckpointsAvailable));
    }

    #[test]
    fn resume_returns_last_checkpoint() {
        let (e, _id) = setup_with_checkpoint();
        let recovery = RecoveryEngine::new(&e);
        let result = recovery.resume().unwrap();
        assert_eq!(result.action, RecoveryAction::Resume);
        assert_eq!(result.state.tasks.len(), 2);
        assert!(result.recovered());
    }

    #[test]
    fn rollback_to_specific_checkpoint() {
        let (e, id) = setup_with_checkpoint();
        let recovery = RecoveryEngine::new(&e);
        let result = recovery.rollback(id).unwrap();
        assert_eq!(result.action, RecoveryAction::Rollback);
        assert_eq!(result.checkpoint_id, Some(id));
        assert_eq!(result.state.tasks.len(), 2);
    }

    #[test]
    fn rollback_to_unknown_fails() {
        let e = CheckpointEngine::new();
        let recovery = RecoveryEngine::new(&e);
        let err = recovery.rollback(CheckpointId::new()).unwrap_err();
        assert!(matches!(err, CheckpointError::NotFound(_)));
    }

    #[test]
    fn retry_failed_task() {
        // Crea un checkpoint con una tarea fallida.
        let mut e = CheckpointEngine::new();
        let mut task = Task::new("will fail");
        task.status = TaskStatus::Failed;
        task.result = Some("boom".into());
        let task_id = task.id;

        let state = WorkState::new().with_task(task);
        e.create(state, Some("with-failure".into()));

        let recovery = RecoveryEngine::new(&e);
        let result = recovery.retry(task_id).unwrap();

        assert_eq!(result.action, RecoveryAction::Retry);
        assert_eq!(result.tasks_to_retry, vec![task_id]);

        // La tarea está ahora en Ready.
        let restored_task = result.state.tasks.iter().find(|t| t.id == task_id).unwrap();
        assert_eq!(restored_task.status, TaskStatus::Ready);
        assert!(restored_task.result.is_none());
    }

    #[test]
    fn retry_unknown_task_fails() {
        let (e, _id) = setup_with_checkpoint();
        let recovery = RecoveryEngine::new(&e);
        let err = recovery.retry(TaskId::new()).unwrap_err();
        assert!(matches!(err, CheckpointError::TaskNotFoundInState(_)));
    }

    #[test]
    fn retry_non_failed_task_fails() {
        let (e, _id) = setup_with_checkpoint();
        let task_id = e.list()[0].state.tasks[0].id;
        let recovery = RecoveryEngine::new(&e);
        // La tarea está Pending, no Failed.
        let err = recovery.retry(task_id).unwrap_err();
        assert!(matches!(err, CheckpointError::CannotRollback));
    }

    #[test]
    fn continue_from_state() {
        let mut task = Task::new("ready task");
        task.status = TaskStatus::Ready;
        let task_id = task.id;

        let state = WorkState::new()
            .with_task(task)
            .with_task(Task::new("pending task"));

        let e = CheckpointEngine::new();
        let recovery = RecoveryEngine::new(&e);
        let result = recovery.continue_from(state);

        assert_eq!(result.action, RecoveryAction::Continue);
        assert!(result.checkpoint_id.is_none());
        assert_eq!(result.tasks_to_retry, vec![task_id]);
    }

    #[test]
    fn continue_with_no_ready_tasks() {
        let state = WorkState::new().with_task(Task::new("pending"));
        let e = CheckpointEngine::new();
        let recovery = RecoveryEngine::new(&e);
        let result = recovery.continue_from(state);
        assert!(result.tasks_to_retry.is_empty());
    }

    #[test]
    fn auto_resume_works() {
        let (e, _id) = setup_with_checkpoint();
        let recovery = RecoveryEngine::new(&e);
        let result = recovery.auto_resume().unwrap();
        assert_eq!(result.action, RecoveryAction::Resume);
    }

    #[test]
    fn realistic_rollback_scenario() {
        let mut e = CheckpointEngine::new();

        // 1. Estado inicial.
        let initial = WorkState::new()
            .with_task(Task::new("setup"))
            .with_task(Task::new("implement"));
        let _cp1 = e.create(initial, Some("initial".into()));

        // 2. Estado tras avanzar (imagina que algo se rompió).
        let broken = WorkState::new()
            .with_task(Task::new("setup"))
            .with_task(Task::new("implement"))
            .with_task(Task::new("deploy - broken"));
        let _cp2 = e.create(broken, Some("broken".into()));

        // 3. El usuario decide volver al inicio.
        let recovery = RecoveryEngine::new(&e);
        let result = recovery.auto_resume().unwrap();

        // `resume` devuelve el último, así que obtiene el "broken".
        assert_eq!(result.state.tasks.len(), 3);

        // Pero podemos hacer rollback al primero.
        let first_cp = e.list()[0].id;
        let result = recovery.rollback(first_cp).unwrap();
        assert_eq!(result.state.tasks.len(), 2);
    }
}
