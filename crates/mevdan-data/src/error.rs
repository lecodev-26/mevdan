//! Errores del crate `mevdan-data`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DataError {
    #[error("dataset not found: {0}")]
    NotFound(String),

    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("parse error: {0}")]
    ParseError(String),

    #[error("column not found: {0}")]
    ColumnNotFound(String),

    #[error("row index out of bounds: {0}")]
    RowOutOfBounds(usize),

    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("empty dataset: {0}")]
    EmptyDataset(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type DataResult<T> = Result<T, DataError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_displays() {
        let err = DataError::NotFound("/tmp/x.csv".into());
        assert_eq!(err.to_string(), "dataset not found: /tmp/x.csv");
    }

    #[test]
    fn unsupported_format_displays() {
        let err = DataError::UnsupportedFormat("parquet".into());
        assert_eq!(err.to_string(), "unsupported format: parquet");
    }

    #[test]
    fn parse_error_displays() {
        let err = DataError::ParseError("bad csv".into());
        assert_eq!(err.to_string(), "parse error: bad csv");
    }

    #[test]
    fn column_not_found_displays() {
        let err = DataError::ColumnNotFound("age".into());
        assert_eq!(err.to_string(), "column not found: age");
    }

    #[test]
    fn row_out_of_bounds_displays() {
        let err = DataError::RowOutOfBounds(5);
        assert_eq!(err.to_string(), "row index out of bounds: 5");
    }

    #[test]
    fn type_mismatch_displays() {
        let err = DataError::TypeMismatch {
            expected: "int".into(),
            actual: "text".into(),
        };
        assert_eq!(err.to_string(), "type mismatch: expected int, got text");
    }

    #[test]
    fn empty_dataset_displays() {
        let err = DataError::EmptyDataset("test".into());
        assert_eq!(err.to_string(), "empty dataset: test");
    }

    #[test]
    fn io_error_converts() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "x");
        let err: DataError = io_err.into();
        assert!(matches!(err, DataError::Io(_)));
    }
}
