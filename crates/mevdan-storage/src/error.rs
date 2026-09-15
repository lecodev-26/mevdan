//! Errores del crate `mevdan-storage`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("migration error: {0}")]
    Migration(String),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("project not found in {0}")]
    ProjectNotFound(String),

    #[error("invalid project directory: {0}")]
    InvalidProjectDir(String),

    #[error("project already exists at {0}")]
    ProjectAlreadyExists(String),

    #[error("audit error: {0}")]
    Audit(String),

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type StorageResult<T> = Result<T, StorageError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_not_found_displays_path() {
        let err = StorageError::ProjectNotFound("/tmp/foo".into());
        assert_eq!(err.to_string(), "project not found in /tmp/foo");
    }

    #[test]
    fn project_already_exists_displays_path() {
        let err = StorageError::ProjectAlreadyExists("/tmp/foo".into());
        assert_eq!(err.to_string(), "project already exists at /tmp/foo");
    }

    #[test]
    fn migration_error_displays_message() {
        let err = StorageError::Migration("V002 missing".into());
        assert_eq!(err.to_string(), "migration error: V002 missing");
    }

    #[test]
    fn audit_error_displays_message() {
        let err = StorageError::Audit("bad event".into());
        assert_eq!(err.to_string(), "audit error: bad event");
    }

    #[test]
    fn core_error_converts_to_storage_error() {
        let core_err = mevdan_core::CoreError::InvalidVersion("bad".into());
        let storage_err: StorageError = core_err.into();
        assert!(matches!(storage_err, StorageError::Core(_)));
    }
}
