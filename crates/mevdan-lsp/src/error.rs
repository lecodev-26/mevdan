//! Errores del crate `mevdan-lsp`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LspError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("transport error: {0}")]
    Transport(String),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("framing error: {0}")]
    Framing(String),

    #[error("JSON-RPC error (code {code}): {message}")]
    RpcError { code: i32, message: String },

    #[error("handshake failed: {0}")]
    Handshake(String),

    #[error("language server not initialized")]
    NotInitialized,

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type LspResult<T> = Result<T, LspError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framing_error_displays() {
        let err = LspError::Framing("bad header".into());
        assert_eq!(err.to_string(), "framing error: bad header");
    }

    #[test]
    fn rpc_error_displays() {
        let err = LspError::RpcError {
            code: -32601,
            message: "Method not found".into(),
        };
        assert_eq!(
            err.to_string(),
            "JSON-RPC error (code -32601): Method not found"
        );
    }

    #[test]
    fn handshake_error_displays() {
        let err = LspError::Handshake("bad version".into());
        assert_eq!(err.to_string(), "handshake failed: bad version");
    }

    #[test]
    fn not_initialized_displays() {
        let err = LspError::NotInitialized;
        assert_eq!(err.to_string(), "language server not initialized");
    }

    #[test]
    fn protocol_error_displays() {
        let err = LspError::Protocol("missing field".into());
        assert_eq!(err.to_string(), "protocol error: missing field");
    }
}
