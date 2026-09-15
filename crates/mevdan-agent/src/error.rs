//! Errores del crate `mevdan-agent`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("agent exceeded max steps ({max}): {reason}")]
    MaxStepsExceeded { max: u32, reason: String },

    #[error("agent exceeded max tokens ({max}): {reason}")]
    MaxTokensExceeded { max: u32, reason: String },

    #[error("agent exceeded time budget ({max_secs}s): {reason}")]
    TimeBudgetExceeded { max_secs: u64, reason: String },

    #[error("agent aborted by policy: {0}")]
    AbortedByPolicy(String),

    #[error("provider error: {0}")]
    Provider(#[from] mevdan_provider::ProviderError),

    #[error("model not found: {0}")]
    ModelNotFound(String),

    #[error("no provider configured for agent '{0}'")]
    NoProvider(String),

    #[error("invalid agent configuration: {0}")]
    InvalidConfig(String),

    #[error("sub-agent failed: {0}")]
    SubAgentFailed(String),

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type AgentResult<T> = Result<T, AgentError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_steps_displays_limit() {
        let err = AgentError::MaxStepsExceeded {
            max: 10,
            reason: "loop did not converge".into(),
        };
        assert_eq!(
            err.to_string(),
            "agent exceeded max steps (10): loop did not converge"
        );
    }

    #[test]
    fn aborted_by_policy_displays_reason() {
        let err = AgentError::AbortedByPolicy("user cancelled".into());
        assert_eq!(err.to_string(), "agent aborted by policy: user cancelled");
    }

    #[test]
    fn no_provider_displays_agent_name() {
        let err = AgentError::NoProvider("my-agent".into());
        assert_eq!(
            err.to_string(),
            "no provider configured for agent 'my-agent'"
        );
    }

    #[test]
    fn provider_error_converts() {
        let prov_err = mevdan_provider::ProviderError::ModelNotFound("m".into());
        let agent_err: AgentError = prov_err.into();
        assert!(matches!(agent_err, AgentError::Provider(_)));
    }

    #[test]
    fn core_error_converts() {
        let core_err = mevdan_core::CoreError::Internal("test".into());
        let agent_err: AgentError = core_err.into();
        assert!(matches!(agent_err, AgentError::Core(_)));
    }
}
