//! Errores del crate `mevdan-documents`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DocumentError {
    #[error("document not found: {0}")]
    NotFound(String),

    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("failed to read document: {0}")]
    ReadError(String),

    #[error("failed to parse document: {0}")]
    ParseError(String),

    #[error("invalid path: {0}")]
    InvalidPath(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type DocumentResult<T> = Result<T, DocumentError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_displays() {
        let err = DocumentError::NotFound("/tmp/x.pdf".into());
        assert_eq!(err.to_string(), "document not found: /tmp/x.pdf");
    }

    #[test]
    fn unsupported_format_displays() {
        let err = DocumentError::UnsupportedFormat("xyz".into());
        assert_eq!(err.to_string(), "unsupported format: xyz");
    }

    #[test]
    fn read_error_displays() {
        let err = DocumentError::ReadError("permission denied".into());
        assert_eq!(
            err.to_string(),
            "failed to read document: permission denied"
        );
    }

    #[test]
    fn parse_error_displays() {
        let err = DocumentError::ParseError("bad header".into());
        assert_eq!(err.to_string(), "failed to parse document: bad header");
    }

    #[test]
    fn invalid_path_displays() {
        let err = DocumentError::InvalidPath("..".into());
        assert_eq!(err.to_string(), "invalid path: ..");
    }

    #[test]
    fn io_error_converts() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "x");
        let err: DocumentError = io_err.into();
        assert!(matches!(err, DocumentError::Io(_)));
    }
}
