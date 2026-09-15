//! Errores del crate `mevdan-checkpoint`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CheckpointError {
    #[error("checkpoint not found: {0}")]
    NotFound(String),

    #[error("invalid label: {0}")]
    InvalidLabel(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("no checkpoints available")]
    NoCheckpointsAvailable,

    #[error("task not found in state: {0}")]
    TaskNotFoundInState(String),

    #[error("cannot rollback: checkpoint is not in a restorable state")]
    CannotRollback,

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type CheckpointResult<T> = Result<T, CheckpointError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_displays_id() {
        let err = CheckpointError::NotFound("c1".into());
        assert_eq!(err.to_string(), "checkpoint not found: c1");
    }

    #[test]
    fn invalid_label_displays() {
        let err = CheckpointError::InvalidLabel("".into());
        assert_eq!(err.to_string(), "invalid label: ");
    }

    #[test]
    fn serialization_displays_reason() {
        let err = CheckpointError::Serialization("bad json".into());
        assert_eq!(err.to_string(), "serialization error: bad json");
    }

    #[test]
    fn no_checkpoints_displays() {
        let err = CheckpointError::NoCheckpointsAvailable;
        assert_eq!(err.to_string(), "no checkpoints available");
    }

    #[test]
    fn task_not_found_in_state_displays() {
        let err = CheckpointError::TaskNotFoundInState("t1".into());
        assert_eq!(err.to_string(), "task not found in state: t1");
    }

    #[test]
    fn cannot_rollback_displays() {
        let err = CheckpointError::CannotRollback;
        assert_eq!(
            err.to_string(),
            "cannot rollback: checkpoint is not in a restorable state"
        );
    }

    #[test]
    fn core_error_converts() {
        let core_err = mevdan_core::CoreError::Internal("x".into());
        let cp_err: CheckpointError = core_err.into();
        assert!(matches!(cp_err, CheckpointError::Core(_)));
    }
}
