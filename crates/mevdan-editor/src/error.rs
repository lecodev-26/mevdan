//! Errores del crate `mevdan-editor`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EditorError {
    #[error("buffer not found: {0}")]
    BufferNotFound(String),

    #[error("buffer already open: {0}")]
    BufferAlreadyOpen(String),

    #[error("invalid range: start {start} > end {end}")]
    InvalidRange { start: usize, end: usize },

    #[error("range out of bounds: {range} (buffer size: {size})")]
    RangeOutOfBounds { range: usize, size: usize },

    #[error("range not on char boundary: {position}")]
    NotOnCharBoundary { position: usize },

    #[error("buffer has unsaved changes: {0}")]
    UnsavedChanges(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type EditorResult<T> = Result<T, EditorError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_not_found_displays() {
        let err = EditorError::BufferNotFound("/tmp/x.rs".into());
        assert_eq!(err.to_string(), "buffer not found: /tmp/x.rs");
    }

    #[test]
    fn buffer_already_open_displays() {
        let err = EditorError::BufferAlreadyOpen("/tmp/x.rs".into());
        assert_eq!(err.to_string(), "buffer already open: /tmp/x.rs");
    }

    #[test]
    fn invalid_range_displays() {
        let err = EditorError::InvalidRange { start: 10, end: 5 };
        assert_eq!(err.to_string(), "invalid range: start 10 > end 5");
    }

    #[test]
    fn range_out_of_bounds_displays() {
        let err = EditorError::RangeOutOfBounds {
            range: 100,
            size: 50,
        };
        assert_eq!(
            err.to_string(),
            "range out of bounds: 100 (buffer size: 50)"
        );
    }

    #[test]
    fn not_on_char_boundary_displays() {
        let err = EditorError::NotOnCharBoundary { position: 3 };
        assert_eq!(err.to_string(), "range not on char boundary: 3");
    }

    #[test]
    fn unsaved_changes_displays() {
        let err = EditorError::UnsavedChanges("/tmp/x.rs".into());
        assert_eq!(err.to_string(), "buffer has unsaved changes: /tmp/x.rs");
    }

    #[test]
    fn io_error_converts() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "x");
        let err: EditorError = io_err.into();
        assert!(matches!(err, EditorError::Io(_)));
    }
}
