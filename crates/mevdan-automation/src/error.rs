//! Errores del crate `mevdan-automation`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AutomationError {
    #[error("rule not found: {0}")]
    RuleNotFound(String),

    #[error("duplicate rule name: {0}")]
    DuplicateRule(String),

    #[error("invalid rule name: {0}")]
    InvalidName(String),

    #[error("invalid rule: {0}")]
    InvalidRule(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type AutomationResult<T> = Result<T, AutomationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_not_found_displays() {
        let err = AutomationError::RuleNotFound("r1".into());
        assert_eq!(err.to_string(), "rule not found: r1");
    }

    #[test]
    fn duplicate_rule_displays() {
        let err = AutomationError::DuplicateRule("on-save".into());
        assert_eq!(err.to_string(), "duplicate rule name: on-save");
    }

    #[test]
    fn invalid_name_displays() {
        let err = AutomationError::InvalidName("Bad Name".into());
        assert_eq!(err.to_string(), "invalid rule name: Bad Name");
    }

    #[test]
    fn invalid_rule_displays() {
        let err = AutomationError::InvalidRule("empty action".into());
        assert_eq!(err.to_string(), "invalid rule: empty action");
    }

    #[test]
    fn serialization_displays() {
        let err = AutomationError::Serialization("bad json".into());
        assert_eq!(err.to_string(), "serialization error: bad json");
    }

    #[test]
    fn core_error_converts() {
        let core_err = mevdan_core::CoreError::Internal("x".into());
        let err: AutomationError = core_err.into();
        assert!(matches!(err, AutomationError::Core(_)));
    }
}
