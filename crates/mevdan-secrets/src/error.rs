//! Errores del crate `mevdan-secrets`.
//!
//! Regla: los errores NUNCA incluyen el valor del secreto. Solo el
//! nombre del secreto puede aparecer en un mensaje de error.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SecretsError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("secret not found: {0}")]
    NotFound(String),

    #[error("invalid secret name: {0}")]
    InvalidName(String),

    #[error("no home directory available")]
    NoHomeDir,

    #[error("config error: {0}")]
    Config(#[from] mevdan_config::ConfigError),
}

pub type SecretsResult<T> = Result<T, SecretsError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_displays_name_not_value() {
        // Regla de oro: solo el NOMBRE aparece, nunca el valor.
        let err = SecretsError::NotFound("openai_api_key".into());
        assert_eq!(err.to_string(), "secret not found: openai_api_key");
    }

    #[test]
    fn invalid_name_displays_name() {
        let err = SecretsError::InvalidName("bad/name".into());
        assert_eq!(err.to_string(), "invalid secret name: bad/name");
    }

    #[test]
    fn no_home_dir_displays() {
        let err = SecretsError::NoHomeDir;
        assert_eq!(err.to_string(), "no home directory available");
    }

    #[test]
    fn config_error_converts() {
        let config_err = mevdan_config::ConfigError::NoHomeDir;
        let secrets_err: SecretsError = config_err.into();
        assert!(matches!(secrets_err, SecretsError::Config(_)));
    }
}
