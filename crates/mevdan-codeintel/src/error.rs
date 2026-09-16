//! Errores del crate `mevdan-codeintel`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodeIntelError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("path not found: {0}")]
    PathNotFound(String),

    #[error("not a directory: {0}")]
    NotADirectory(String),

    #[error("max depth exceeded ({max}) at: {path}")]
    MaxDepthExceeded { max: usize, path: String },

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type CodeIntelResult<T> = Result<T, CodeIntelError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_not_found_displays_path() {
        let err = CodeIntelError::PathNotFound("/tmp/x".into());
        assert_eq!(err.to_string(), "path not found: /tmp/x");
    }

    #[test]
    fn not_a_directory_displays_path() {
        let err = CodeIntelError::NotADirectory("/tmp/file.txt".into());
        assert_eq!(err.to_string(), "not a directory: /tmp/file.txt");
    }

    #[test]
    fn max_depth_displays_fields() {
        let err = CodeIntelError::MaxDepthExceeded {
            max: 10,
            path: "/deep/path".into(),
        };
        assert_eq!(err.to_string(), "max depth exceeded (10) at: /deep/path");
    }

    #[test]
    fn io_error_converts() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err: CodeIntelError = io_err.into();
        assert!(matches!(err, CodeIntelError::Io(_)));
    }
}
