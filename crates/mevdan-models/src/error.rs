//! Errores del crate `mevdan-models`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModelsError {
    #[error("model not found in registry: {0}")]
    ModelNotFound(String),

    #[error("provider not found in registry: {0}")]
    ProviderNotFound(String),

    #[error("invalid model descriptor: {0}")]
    InvalidDescriptor(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type ModelsResult<T> = Result<T, ModelsError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_not_found_displays_name() {
        let err = ModelsError::ModelNotFound("gpt-99".into());
        assert_eq!(err.to_string(), "model not found in registry: gpt-99");
    }

    #[test]
    fn provider_not_found_displays_name() {
        let err = ModelsError::ProviderNotFound("some-provider".into());
        assert_eq!(
            err.to_string(),
            "provider not found in registry: some-provider"
        );
    }

    #[test]
    fn invalid_descriptor_displays_reason() {
        let err = ModelsError::InvalidDescriptor("missing name".into());
        assert_eq!(err.to_string(), "invalid model descriptor: missing name");
    }
}
