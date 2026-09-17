//! Errores del crate `mevdan-router`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RouterError {
    #[error("no agent available for task kind: {0}")]
    NoAgentForTask(String),

    #[error("no model available matching criteria: {0}")]
    NoModelForTask(String),

    #[error("invalid policy: {0}")]
    InvalidPolicy(String),

    #[error("no candidates provided")]
    NoCandidates,

    #[error("manual selection required but no explicit choice given")]
    ManualSelectionRequired,

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type RouterResult<T> = Result<T, RouterError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_agent_displays_kind() {
        let err = RouterError::NoAgentForTask("coding".into());
        assert_eq!(err.to_string(), "no agent available for task kind: coding");
    }

    #[test]
    fn no_model_displays_criteria() {
        let err = RouterError::NoModelForTask("context > 100k".into());
        assert_eq!(
            err.to_string(),
            "no model available matching criteria: context > 100k"
        );
    }

    #[test]
    fn invalid_policy_displays_reason() {
        let err = RouterError::InvalidPolicy("bad".into());
        assert_eq!(err.to_string(), "invalid policy: bad");
    }

    #[test]
    fn no_candidates_displays() {
        let err = RouterError::NoCandidates;
        assert_eq!(err.to_string(), "no candidates provided");
    }

    #[test]
    fn manual_selection_displays() {
        let err = RouterError::ManualSelectionRequired;
        assert_eq!(
            err.to_string(),
            "manual selection required but no explicit choice given"
        );
    }

    #[test]
    fn core_error_converts() {
        let core_err = mevdan_core::CoreError::Internal("x".into());
        let err: RouterError = core_err.into();
        assert!(matches!(err, RouterError::Core(_)));
    }
}
