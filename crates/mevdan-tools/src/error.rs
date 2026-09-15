//! Errores del crate `mevdan-tools`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("tool not found: {0}")]
    ToolNotFound(String),

    #[error("duplicate tool name: {0}")]
    DuplicateTool(String),

    #[error("invalid tool name: {0}")]
    InvalidToolName(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("missing required argument: {0}")]
    MissingArgument(String),

    #[error("path escapes sandbox: {0}")]
    PathEscapesSandbox(String),

    #[error("path not found: {0}")]
    PathNotFound(String),

    #[error("path is not a directory: {0}")]
    NotADirectory(String),

    #[error("path is not a file: {0}")]
    NotAFile(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type ToolResult<T> = Result<T, ToolError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_not_found_displays_name() {
        let err = ToolError::ToolNotFound("filesystem".into());
        assert_eq!(err.to_string(), "tool not found: filesystem");
    }

    #[test]
    fn path_escapes_sandbox_displays_path() {
        let err = ToolError::PathEscapesSandbox("../../etc/passwd".into());
        assert_eq!(err.to_string(), "path escapes sandbox: ../../etc/passwd");
    }

    #[test]
    fn missing_argument_displays_arg() {
        let err = ToolError::MissingArgument("path".into());
        assert_eq!(err.to_string(), "missing required argument: path");
    }

    #[test]
    fn invalid_input_displays_reason() {
        let err = ToolError::InvalidInput("bad json".into());
        assert_eq!(err.to_string(), "invalid input: bad json");
    }

    #[test]
    fn io_error_converts() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file gone");
        let tool_err: ToolError = io_err.into();
        assert!(matches!(tool_err, ToolError::Io(_)));
    }
}
