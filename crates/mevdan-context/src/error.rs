//! Errores del crate `mevdan-context`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ContextError {
    #[error("context budget exceeded: {used} tokens used, {max} allowed")]
    BudgetExceeded { used: u32, max: u32 },

    #[error("budget too small: {budget} tokens cannot fit mandatory sections")]
    BudgetTooSmall { budget: u32 },

    #[error("invalid section: {0}")]
    InvalidSection(String),

    #[error("empty required field: {0}")]
    EmptyRequired(String),

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type ContextResult<T> = Result<T, ContextError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_exceeded_displays_counts() {
        let err = ContextError::BudgetExceeded {
            used: 5000,
            max: 4096,
        };
        assert_eq!(
            err.to_string(),
            "context budget exceeded: 5000 tokens used, 4096 allowed"
        );
    }

    #[test]
    fn budget_too_small_displays_budget() {
        let err = ContextError::BudgetTooSmall { budget: 10 };
        assert_eq!(
            err.to_string(),
            "budget too small: 10 tokens cannot fit mandatory sections"
        );
    }

    #[test]
    fn invalid_section_displays_reason() {
        let err = ContextError::InvalidSection("missing title".into());
        assert_eq!(err.to_string(), "invalid section: missing title");
    }

    #[test]
    fn empty_required_displays_field() {
        let err = ContextError::EmptyRequired("system prompt".into());
        assert_eq!(err.to_string(), "empty required field: system prompt");
    }
}
