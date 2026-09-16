//! Errores del crate `mevdan-mcp`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid server name: {0}")]
    InvalidServerName(String),

    #[error("server not found: {0}")]
    ServerNotFound(String),

    #[error("transport error: {0}")]
    Transport(String),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("JSON-RPC error (code {code}): {message}")]
    RpcError { code: i32, message: String },

    #[error("handshake failed: {0}")]
    Handshake(String),

    #[error("timeout after {seconds}s")]
    Timeout { seconds: u64 },

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type McpResult<T> = Result<T, McpError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_server_name_displays() {
        let err = McpError::InvalidServerName("bad/name".into());
        assert_eq!(err.to_string(), "invalid server name: bad/name");
    }

    #[test]
    fn server_not_found_displays() {
        let err = McpError::ServerNotFound("filesystem".into());
        assert_eq!(err.to_string(), "server not found: filesystem");
    }

    #[test]
    fn rpc_error_displays_code_and_message() {
        let err = McpError::RpcError {
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
        let err = McpError::Handshake("bad version".into());
        assert_eq!(err.to_string(), "handshake failed: bad version");
    }

    #[test]
    fn transport_error_displays() {
        let err = McpError::Transport("stdio closed".into());
        assert_eq!(err.to_string(), "transport error: stdio closed");
    }

    #[test]
    fn timeout_error_displays() {
        let err = McpError::Timeout { seconds: 30 };
        assert_eq!(err.to_string(), "timeout after 30s");
    }
}
