//! Detección de formato de documento.

use crate::error::{DocumentError, DocumentResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Formato de un documento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentFormat {
    /// PDF (no extraído en V5.3, solo reconocido).
    Pdf,
    /// Microsoft Word (.docx).
    Docx,
    /// Markdown (.md).
    Markdown,
    /// Texto plano (.txt).
    Text,
    /// EPUB (no extraído en V5.3, solo reconocido).
    Epub,
    /// CSV.
    Csv,
    /// JSON.
    Json,
    /// Desconocido.
    Unknown,
}

impl DocumentFormat {
    pub fn display_name(&self) -> &'static str {
        match self {
            DocumentFormat::Pdf => "pdf",
            DocumentFormat::Docx => "docx",
            DocumentFormat::Markdown => "markdown",
            DocumentFormat::Text => "text",
            DocumentFormat::Epub => "epub",
            DocumentFormat::Csv => "csv",
            DocumentFormat::Json => "json",
            DocumentFormat::Unknown => "unknown",
        }
    }

    /// ¿Podemos extraer texto de este formato en V5.3?
    pub fn is_extractable(&self) -> bool {
        matches!(
            self,
            DocumentFormat::Markdown
                | DocumentFormat::Text
                | DocumentFormat::Csv
                | DocumentFormat::Json
        )
    }

    /// ¿Es un formato de texto que se lee como string?
    pub fn is_plain_text(&self) -> bool {
        matches!(self, DocumentFormat::Markdown | DocumentFormat::Text)
    }

    /// Extensión principal.
    pub fn extension(&self) -> &'static str {
        match self {
            DocumentFormat::Pdf => "pdf",
            DocumentFormat::Docx => "docx",
            DocumentFormat::Markdown => "md",
            DocumentFormat::Text => "txt",
            DocumentFormat::Epub => "epub",
            DocumentFormat::Csv => "csv",
            DocumentFormat::Json => "json",
            DocumentFormat::Unknown => "",
        }
    }

    /// Detecta el formato desde un path.
    pub fn from_path(path: &Path) -> DocumentResult<Self> {
        let ext = match path.extension().and_then(|s| s.to_str()) {
            Some(e) => e.to_lowercase(),
            None => return Err(DocumentError::UnsupportedFormat(path.display().to_string())),
        };

        let format = match ext.as_str() {
            "pdf" => DocumentFormat::Pdf,
            "docx" => DocumentFormat::Docx,
            "md" | "markdown" => DocumentFormat::Markdown,
            "txt" | "text" | "log" => DocumentFormat::Text,
            "epub" => DocumentFormat::Epub,
            "csv" => DocumentFormat::Csv,
            "json" => DocumentFormat::Json,
            _ => DocumentFormat::Unknown,
        };

        Ok(format)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn detect(path: &str) -> DocumentResult<DocumentFormat> {
        DocumentFormat::from_path(&PathBuf::from(path))
    }

    #[test]
    fn detect_pdf() {
        assert_eq!(detect("book.pdf").unwrap(), DocumentFormat::Pdf);
    }

    #[test]
    fn detect_docx() {
        assert_eq!(detect("report.docx").unwrap(), DocumentFormat::Docx);
    }

    #[test]
    fn detect_markdown() {
        assert_eq!(detect("README.md").unwrap(), DocumentFormat::Markdown);
        assert_eq!(detect("doc.markdown").unwrap(), DocumentFormat::Markdown);
    }

    #[test]
    fn detect_text() {
        assert_eq!(detect("notes.txt").unwrap(), DocumentFormat::Text);
        assert_eq!(detect("app.log").unwrap(), DocumentFormat::Text);
    }

    #[test]
    fn detect_epub() {
        assert_eq!(detect("book.epub").unwrap(), DocumentFormat::Epub);
    }

    #[test]
    fn detect_csv() {
        assert_eq!(detect("data.csv").unwrap(), DocumentFormat::Csv);
    }

    #[test]
    fn detect_json() {
        assert_eq!(detect("config.json").unwrap(), DocumentFormat::Json);
    }

    #[test]
    fn detect_unknown() {
        assert_eq!(detect("file.xyz").unwrap(), DocumentFormat::Unknown);
    }

    #[test]
    fn detect_no_extension_fails() {
        assert!(detect("README").is_err());
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(detect("BOOK.PDF").unwrap(), DocumentFormat::Pdf);
        assert_eq!(detect("Doc.MD").unwrap(), DocumentFormat::Markdown);
    }

    #[test]
    fn display_names() {
        assert_eq!(DocumentFormat::Pdf.display_name(), "pdf");
        assert_eq!(DocumentFormat::Docx.display_name(), "docx");
        assert_eq!(DocumentFormat::Markdown.display_name(), "markdown");
        assert_eq!(DocumentFormat::Unknown.display_name(), "unknown");
    }

    #[test]
    fn is_extractable() {
        assert!(DocumentFormat::Markdown.is_extractable());
        assert!(DocumentFormat::Text.is_extractable());
        assert!(DocumentFormat::Csv.is_extractable());
        assert!(DocumentFormat::Json.is_extractable());
        assert!(!DocumentFormat::Pdf.is_extractable());
        assert!(!DocumentFormat::Docx.is_extractable());
        assert!(!DocumentFormat::Epub.is_extractable());
        assert!(!DocumentFormat::Unknown.is_extractable());
    }

    #[test]
    fn is_plain_text() {
        assert!(DocumentFormat::Markdown.is_plain_text());
        assert!(DocumentFormat::Text.is_plain_text());
        assert!(!DocumentFormat::Csv.is_plain_text());
        assert!(!DocumentFormat::Json.is_plain_text());
    }

    #[test]
    fn extension() {
        assert_eq!(DocumentFormat::Pdf.extension(), "pdf");
        assert_eq!(DocumentFormat::Markdown.extension(), "md");
        assert_eq!(DocumentFormat::Unknown.extension(), "");
    }

    #[test]
    fn serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&DocumentFormat::Markdown).unwrap(),
            "\"markdown\""
        );
        assert_eq!(
            serde_json::to_string(&DocumentFormat::Pdf).unwrap(),
            "\"pdf\""
        );
    }
}
