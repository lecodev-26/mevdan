//! Errores del crate `mevdan-worktree`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorktreeError {
    #[error("worktree not found: {0}")]
    NotFound(String),

    #[error("duplicate worktree name: {0}")]
    DuplicateName(String),

    #[error("invalid worktree name: {0}")]
    InvalidName(String),

    #[error("cannot delete the main worktree")]
    CannotDeleteMain,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type WorktreeResult<T> = Result<T, WorktreeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_displays_id() {
        let err = WorktreeError::NotFound("abc".into());
        assert_eq!(err.to_string(), "worktree not found: abc");
    }

    #[test]
    fn duplicate_name_displays() {
        let err = WorktreeError::DuplicateName("main".into());
        assert_eq!(err.to_string(), "duplicate worktree name: main");
    }

    #[test]
    fn invalid_name_displays() {
        let err = WorktreeError::InvalidName("bad/name".into());
        assert_eq!(err.to_string(), "invalid worktree name: bad/name");
    }

    #[test]
    fn cannot_delete_main_displays() {
        let err = WorktreeError::CannotDeleteMain;
        assert_eq!(err.to_string(), "cannot delete the main worktree");
    }

    #[test]
    fn serialization_displays() {
        let err = WorktreeError::Serialization("bad json".into());
        assert_eq!(err.to_string(), "serialization error: bad json");
    }

    #[test]
    fn io_error_converts() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err: WorktreeError = io_err.into();
        assert!(matches!(err, WorktreeError::Io(_)));
    }
}
