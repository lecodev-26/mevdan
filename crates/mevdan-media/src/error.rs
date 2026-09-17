//! Errores del crate `mevdan-media`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MediaError {
    #[error("media not found: {0}")]
    NotFound(String),

    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("invalid header: {0}")]
    InvalidHeader(String),

    #[error("invalid path: {0}")]
    InvalidPath(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type MediaResult<T> = Result<T, MediaError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_displays() {
        let err = MediaError::NotFound("/tmp/x.png".into());
        assert_eq!(err.to_string(), "media not found: /tmp/x.png");
    }

    #[test]
    fn unsupported_format_displays() {
        let err = MediaError::UnsupportedFormat("xyz".into());
        assert_eq!(err.to_string(), "unsupported format: xyz");
    }

    #[test]
    fn invalid_header_displays() {
        let err = MediaError::InvalidHeader("bad PNG magic".into());
        assert_eq!(err.to_string(), "invalid header: bad PNG magic");
    }

    #[test]
    fn invalid_path_displays() {
        let err = MediaError::InvalidPath("/tmp".into());
        assert_eq!(err.to_string(), "invalid path: /tmp");
    }

    #[test]
    fn io_error_converts() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "x");
        let err: MediaError = io_err.into();
        assert!(matches!(err, MediaError::Io(_)));
    }
}
