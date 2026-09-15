//! Errores del crate `mevdan-verification`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum VerificationError {
    #[error("artifact not found: {0}")]
    ArtifactNotFound(String),

    #[error("evidence not found: {0}")]
    EvidenceNotFound(String),

    #[error("claim not found: {0}")]
    ClaimNotFound(String),

    #[error("invalid hash format: {0}")]
    InvalidHash(String),

    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type VerificationResult<T> = Result<T, VerificationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_not_found_displays_id() {
        let err = VerificationError::ArtifactNotFound("a1".into());
        assert_eq!(err.to_string(), "artifact not found: a1");
    }

    #[test]
    fn hash_mismatch_displays_both() {
        let err = VerificationError::HashMismatch {
            expected: "abc".into(),
            actual: "def".into(),
        };
        assert_eq!(err.to_string(), "hash mismatch: expected abc, got def");
    }

    #[test]
    fn invalid_hash_displays_format() {
        let err = VerificationError::InvalidHash("not-hex".into());
        assert_eq!(err.to_string(), "invalid hash format: not-hex");
    }

    #[test]
    fn invalid_input_displays_reason() {
        let err = VerificationError::InvalidInput("empty".into());
        assert_eq!(err.to_string(), "invalid input: empty");
    }

    #[test]
    fn core_error_converts() {
        let core_err = mevdan_core::CoreError::Internal("x".into());
        let v_err: VerificationError = core_err.into();
        assert!(matches!(v_err, VerificationError::Core(_)));
    }
}
