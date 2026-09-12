//! Errores canónicos del core de MEVDAN.
//!
//! Regla: `mevdan-core` nunca depende de I/O (SQLite, red, filesystem).
//! Por tanto, estos errores describen problemas de dominio, no de
//! infraestructura. Los errores de infraestructura viven en sus
//! respectivos crates (`mevdan-storage`, `mevdan-provider`, etc.).

use thiserror::Error;

/// Errores del dominio de MEVDAN.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("project not found: {0}")]
    ProjectNotFound(String),

    #[error("session not found: {0}")]
    SessionNotFound(String),

    #[error("invalid project name: {0}")]
    InvalidProjectName(String),

    #[error("schema version mismatch: expected {expected}, found {found}")]
    SchemaVersionMismatch { expected: String, found: String },

    #[error("invalid version string: {0}")]
    InvalidVersion(String),

    #[error("internal error: {0}")]
    Internal(String),
}

/// Alias para resultados del core.
pub type CoreResult<T> = Result<T, CoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_not_found_displays_id() {
        let err = CoreError::ProjectNotFound("abc-123".into());
        assert_eq!(err.to_string(), "project not found: abc-123");
    }

    #[test]
    fn schema_mismatch_displays_both_versions() {
        let err = CoreError::SchemaVersionMismatch {
            expected: "0.1.0".into(),
            found: "0.0.9".into(),
        };
        assert_eq!(
            err.to_string(),
            "schema version mismatch: expected 0.1.0, found 0.0.9"
        );
    }

    #[test]
    fn invalid_project_name_displays_name() {
        let err = CoreError::InvalidProjectName("bad/name".into());
        assert_eq!(err.to_string(), "invalid project name: bad/name");
    }
}
