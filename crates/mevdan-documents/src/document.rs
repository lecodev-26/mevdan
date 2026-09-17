//! `Document` y `DocumentContent` — representación del documento.

use crate::format::DocumentFormat;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Descriptor de un documento.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Ruta al documento.
    pub path: PathBuf,
    /// Formato detectado.
    pub format: DocumentFormat,
    /// Tamaño en bytes.
    pub size_bytes: u64,
    /// Título (del primer heading Markdown, o del nombre de archivo).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Metadata libre.
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl Document {
    pub fn new(path: impl Into<PathBuf>, format: DocumentFormat, size_bytes: u64) -> Self {
        Self {
            path: path.into(),
            format,
            size_bytes,
            title: None,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Nombre de archivo.
    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string()
    }
}

/// Una sección de un documento (título + contenido).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    /// Nivel de heading (1 = H1, 2 = H2, ...). 0 = sin heading.
    pub level: u8,
    /// Título de la sección.
    pub title: String,
    /// Contenido de la sección.
    pub content: String,
    /// Línea donde empieza en el documento original.
    pub start_line: usize,
}

impl Section {
    pub fn new(
        level: u8,
        title: impl Into<String>,
        content: impl Into<String>,
        start_line: usize,
    ) -> Self {
        Self {
            level,
            title: title.into(),
            content: content.into(),
            start_line,
        }
    }

    pub fn is_heading(&self) -> bool {
        self.level > 0
    }
}

/// Contenido extraído de un documento.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentContent {
    /// Texto completo (concatenado).
    pub text: String,
    /// Número de líneas.
    pub line_count: usize,
    /// Número de palabras.
    pub word_count: usize,
    /// Secciones (para documentos con estructura, como Markdown).
    #[serde(default)]
    pub sections: Vec<Section>,
    /// ¿Se ha podido extraer el texto?
    pub extracted: bool,
}

impl DocumentContent {
    pub fn empty() -> Self {
        Self {
            text: String::new(),
            line_count: 0,
            word_count: 0,
            sections: Vec::new(),
            extracted: false,
        }
    }

    /// Construye desde texto plano (sin estructura).
    pub fn from_text(text: impl Into<String>) -> Self {
        let text = text.into();
        let line_count = text.lines().count();
        let word_count = text.split_whitespace().count();
        Self {
            text,
            line_count,
            word_count,
            sections: Vec::new(),
            extracted: true,
        }
    }

    /// Añade secciones.
    pub fn with_sections(mut self, sections: Vec<Section>) -> Self {
        self.sections = sections;
        self
    }

    /// Extrae las primeras N palabras como preview.
    pub fn preview(&self, max_chars: usize) -> String {
        if self.text.len() <= max_chars {
            self.text.clone()
        } else {
            let mut end = max_chars;
            while !self.text.is_char_boundary(end) && end > 0 {
                end -= 1;
            }
            format!("{}...", &self.text[..end])
        }
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

/// Resultado de cargar un documento: descriptor + contenido.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedDocument {
    pub document: Document,
    pub content: DocumentContent,
}

impl LoadedDocument {
    pub fn new(document: Document, content: DocumentContent) -> Self {
        Self { document, content }
    }

    /// Resumen textual.
    pub fn summary(&self) -> String {
        format!(
            "{} [{}] {} bytes, {} lines, {} words{}",
            self.document.file_name(),
            self.document.format.display_name(),
            self.document.size_bytes,
            self.content.line_count,
            self.content.word_count,
            if self.content.extracted {
                ""
            } else {
                " (text not extracted)"
            },
        )
    }
}

/// Helper para construir un Document desde un Path.
pub(crate) fn build_document(path: &Path, format: DocumentFormat) -> std::io::Result<Document> {
    let size = std::fs::metadata(path)?.len();
    Ok(Document::new(path.to_path_buf(), format, size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_new_creates() {
        let d = Document::new("/tmp/test.pdf", DocumentFormat::Pdf, 1234);
        assert_eq!(d.format, DocumentFormat::Pdf);
        assert_eq!(d.size_bytes, 1234);
        assert!(d.title.is_none());
    }

    #[test]
    fn document_with_title() {
        let d = Document::new("/tmp/x.md", DocumentFormat::Markdown, 100).with_title("Intro");
        assert_eq!(d.title.as_deref(), Some("Intro"));
    }

    #[test]
    fn document_file_name() {
        let d = Document::new("/tmp/sub/test.pdf", DocumentFormat::Pdf, 0);
        assert_eq!(d.file_name(), "test.pdf");
    }

    #[test]
    fn document_serializes() {
        let d = Document::new("/tmp/x.md", DocumentFormat::Markdown, 50);
        let json = serde_json::to_string(&d).unwrap();
        let back: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(back.format, d.format);
    }

    #[test]
    fn section_new_creates() {
        let s = Section::new(1, "Intro", "Hello world", 0);
        assert_eq!(s.level, 1);
        assert_eq!(s.title, "Intro");
        assert_eq!(s.content, "Hello world");
        assert!(s.is_heading());
    }

    #[test]
    fn section_level_zero_not_heading() {
        let s = Section::new(0, "", "text", 0);
        assert!(!s.is_heading());
    }

    #[test]
    fn content_empty() {
        let c = DocumentContent::empty();
        assert!(c.is_empty());
        assert!(!c.extracted);
        assert_eq!(c.line_count, 0);
    }

    #[test]
    fn content_from_text() {
        let c = DocumentContent::from_text("hello world\nsecond line");
        assert_eq!(c.line_count, 2);
        assert_eq!(c.word_count, 4);
        assert!(c.extracted);
    }

    #[test]
    fn content_from_text_empty() {
        let c = DocumentContent::from_text("");
        assert_eq!(c.line_count, 0);
        assert_eq!(c.word_count, 0);
    }

    #[test]
    fn content_with_sections() {
        let c = DocumentContent::from_text("hello")
            .with_sections(vec![Section::new(1, "Title", "body", 0)]);
        assert_eq!(c.sections.len(), 1);
    }

    #[test]
    fn content_preview_short() {
        let c = DocumentContent::from_text("short");
        assert_eq!(c.preview(100), "short");
    }

    #[test]
    fn content_preview_long() {
        let c = DocumentContent::from_text("this is a long string that should be truncated");
        let p = c.preview(10);
        assert!(p.ends_with("..."));
        assert!(p.len() <= 13); // 10 + "..."
    }

    #[test]
    fn content_preview_handles_utf8() {
        // Con acentos, hay que respetar char boundaries.
        let c = DocumentContent::from_text("café y té");
        let p = c.preview(4);
        // No debe paniquear.
        assert!(p.len() <= 7);
    }

    #[test]
    fn loaded_document_summary() {
        let d = Document::new("/tmp/test.md", DocumentFormat::Markdown, 100);
        let c = DocumentContent::from_text("hello world");
        let l = LoadedDocument::new(d, c);
        let s = l.summary();
        assert!(s.contains("test.md"));
        assert!(s.contains("markdown"));
        assert!(s.contains("2 words"));
    }

    #[test]
    fn loaded_document_summary_not_extracted() {
        let d = Document::new("/tmp/test.pdf", DocumentFormat::Pdf, 1000);
        let c = DocumentContent::empty();
        let l = LoadedDocument::new(d, c);
        let s = l.summary();
        assert!(s.contains("text not extracted"));
    }

    #[test]
    fn loaded_document_serializes() {
        let d = Document::new("/tmp/x.md", DocumentFormat::Markdown, 10);
        let c = DocumentContent::from_text("hi");
        let l = LoadedDocument::new(d, c);
        let json = serde_json::to_string(&l).unwrap();
        let back: LoadedDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(back.document.format, DocumentFormat::Markdown);
    }
}
