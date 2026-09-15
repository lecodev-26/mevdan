//! Errores del crate `mevdan-task`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TaskError {
    #[error("task not found: {0}")]
    TaskNotFound(String),

    #[error("duplicate task id: {0}")]
    DuplicateTask(String),

    #[error("empty title: {0}")]
    EmptyTitle(String),

    #[error("invalid transition from {from} to {to} for task {task}")]
    InvalidTransition {
        task: String,
        from: String,
        to: String,
    },

    #[error("dependency not found: {0}")]
    DependencyNotFound(String),

    #[error("dependency cycle detected: {0}")]
    CycleDetected(String),

    #[error("cannot add dependency: task {task} depends on itself")]
    SelfDependency { task: String },

    #[error("task is blocked by unresolved dependencies: {0}")]
    Blocked(String),

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type TaskResult<T> = Result<T, TaskError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_not_found_displays_id() {
        let err = TaskError::TaskNotFound("abc".into());
        assert_eq!(err.to_string(), "task not found: abc");
    }

    #[test]
    fn empty_title_displays() {
        let err = TaskError::EmptyTitle("task".into());
        assert_eq!(err.to_string(), "empty title: task");
    }

    #[test]
    fn invalid_transition_displays_all_fields() {
        let err = TaskError::InvalidTransition {
            task: "t1".into(),
            from: "completed".into(),
            to: "running".into(),
        };
        assert_eq!(
            err.to_string(),
            "invalid transition from completed to running for task t1"
        );
    }

    #[test]
    fn self_dependency_displays() {
        let err = TaskError::SelfDependency { task: "t1".into() };
        assert_eq!(
            err.to_string(),
            "cannot add dependency: task t1 depends on itself"
        );
    }

    #[test]
    fn blocked_displays_id() {
        let err = TaskError::Blocked("t1".into());
        assert_eq!(
            err.to_string(),
            "task is blocked by unresolved dependencies: t1"
        );
    }
}
