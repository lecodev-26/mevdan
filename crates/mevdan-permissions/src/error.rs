//! Errores del crate `mevdan-permissions`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PermissionError {
    #[error("invalid rule: {0}")]
    InvalidRule(String),

    #[error("invalid scope: {0}")]
    InvalidScope(String),

    #[error("invalid glob pattern: {0}")]
    InvalidGlob(String),

    #[error("policy parse error: {0}")]
    PolicyParse(String),

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type PermissionResult<T> = Result<T, PermissionError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_rule_displays_reason() {
        let err = PermissionError::InvalidRule("empty tool name".into());
        assert_eq!(err.to_string(), "invalid rule: empty tool name");
    }

    #[test]
    fn invalid_scope_displays_reason() {
        let err = PermissionError::InvalidScope("bad pattern".into());
        assert_eq!(err.to_string(), "invalid scope: bad pattern");
    }

    #[test]
    fn invalid_glob_displays_pattern() {
        let err = PermissionError::InvalidGlob("***".into());
        assert_eq!(err.to_string(), "invalid glob pattern: ***");
    }

    #[test]
    fn policy_parse_displays_reason() {
        let err = PermissionError::PolicyParse("missing field".into());
        assert_eq!(err.to_string(), "policy parse error: missing field");
    }
}
