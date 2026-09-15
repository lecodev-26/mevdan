//! Errores del crate `mevdan-provider`.
//!
//! Regla: los errores NUNCA incluyen API keys, tokens, ni contenido
//! completo de mensajes del usuario. Solo metadatos (nombres de modelo,
//! códigos de estado, descripciones).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider not configured: {0}")]
    NotConfigured(String),

    #[error("model not found: {0}")]
    ModelNotFound(String),

    #[error("capability not supported by provider {provider}: {capability}")]
    CapabilityNotSupported {
        provider: String,
        capability: String,
    },

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("provider returned error (status {status}): {message}")]
    ProviderError { status: u16, message: String },

    #[error("rate limited, retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("network error: {0}")]
    Network(String),

    #[error("timeout after {seconds}s")]
    Timeout { seconds: u64 },

    #[error("authentication failed: missing or invalid credentials for {0}")]
    AuthFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type ProviderResult<T> = Result<T, ProviderError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_configured_displays_name() {
        let err = ProviderError::NotConfigured("openai".into());
        assert_eq!(err.to_string(), "provider not configured: openai");
    }

    #[test]
    fn capability_not_supported_displays_details() {
        let err = ProviderError::CapabilityNotSupported {
            provider: "ollama".into(),
            capability: "vision".into(),
        };
        assert_eq!(
            err.to_string(),
            "capability not supported by provider ollama: vision"
        );
    }

    #[test]
    fn rate_limited_displays_retry_after() {
        let err = ProviderError::RateLimited {
            retry_after_secs: 30,
        };
        assert_eq!(err.to_string(), "rate limited, retry after 30s");
    }

    #[test]
    fn auth_failed_never_leaks_credentials() {
        let err = ProviderError::AuthFailed("openai".into());
        let msg = err.to_string();
        assert!(msg.contains("openai"));
        // No hay ninguna key en el mensaje.
        assert!(!msg.contains("sk-"));
    }

    #[test]
    fn core_error_converts() {
        let core_err = mevdan_core::CoreError::Internal("test".into());
        let provider_err: ProviderError = core_err.into();
        assert!(matches!(provider_err, ProviderError::Core(_)));
    }
}
