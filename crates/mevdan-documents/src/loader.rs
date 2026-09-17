//! `DocumentLoader` — carga documentos desde disco.

use crate::{
    document::{build_document, DocumentContent, LoadedDocument, Section},
    error::{DocumentError, DocumentResult},
    format::DocumentFormat,
};
use std::fs;
use std::path::Path;

/// Cargador de documentos.
#[derive(Debug, Default)]
pub struct DocumentLoader;

impl DocumentLoader {
    pub fn new() -> Self {
        Self
    }

    /// Carga un documento desde una ruta.
    pub fn load(&self, path: impl AsRef<Path>) -> DocumentResult<LoadedDocument> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(DocumentError::NotFound(path.display().to_string()));
        }
        if !path.is_file() {
            return Err(DocumentError::InvalidPath(path.display().to_string()));
        }

        let format = DocumentFormat::from_path(path)?;
        let document = build_document(path, format)?;

        let content = match format {
            DocumentFormat::Markdown => self.extract_markdown(path)?,
            DocumentFormat::Text => self.extract_plain_text(path)?,
            DocumentFormat::Csv => self.extract_csv(path)?,
            DocumentFormat::Json => self.extract_json(path)?,
            DocumentFormat::Pdf | DocumentFormat::Docx | DocumentFormat::Epub => {
                DocumentContent::empty()
            }
            DocumentFormat::Unknown => {
                return Err(DocumentError::UnsupportedFormat(path.display().to_string()));
            }
        };

        // Extrae el título si no lo tiene.
        // Calculamos el fallback ANTES de mover `document`.
        let fallback_title = document.file_name();
        let title = document
            .title
            .clone()
            .or_else(|| {
                content
                    .sections
                    .iter()
                    .find(|s| s.is_heading())
                    .map(|s| s.title.clone())
            })
            .unwrap_or(fallback_title);

        let document = document.with_title(title);

        Ok(LoadedDocument::new(document, content))
    }

    fn extract_plain_text(&self, path: &Path) -> DocumentResult<DocumentContent> {
        let text = fs::read_to_string(path)
            .map_err(|e| DocumentError::ReadError(format!("{}: {}", path.display(), e)))?;
        Ok(DocumentContent::from_text(text))
    }

    fn extract_markdown(&self, path: &Path) -> DocumentResult<DocumentContent> {
        let text = fs::read_to_string(path)
            .map_err(|e| DocumentError::ReadError(format!("{}: {}", path.display(), e)))?;

        let sections = parse_markdown_sections(&text);
        Ok(DocumentContent::from_text(text).with_sections(sections))
    }

    fn extract_csv(&self, path: &Path) -> DocumentResult<DocumentContent> {
        let text = fs::read_to_string(path)
            .map_err(|e| DocumentError::ReadError(format!("{}: {}", path.display(), e)))?;

        let row_count = text.lines().count().saturating_sub(1);

        let mut content = DocumentContent::from_text(text);
        content = content.with_sections(vec![Section::new(
            1,
            format!("CSV: {} rows", row_count),
            format!("{} rows detected", row_count),
            0,
        )]);
        Ok(content)
    }

    fn extract_json(&self, path: &Path) -> DocumentResult<DocumentContent> {
        let text = fs::read_to_string(path)
            .map_err(|e| DocumentError::ReadError(format!("{}: {}", path.display(), e)))?;

        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| DocumentError::ParseError(format!("invalid JSON: {}", e)))?;

        let item_count = match &value {
            serde_json::Value::Array(arr) => arr.len(),
            serde_json::Value::Object(map) => map.len(),
            _ => 1,
        };

        let mut content = DocumentContent::from_text(text);
        content = content.with_sections(vec![Section::new(
            1,
            format!("JSON: {} top-level items", item_count),
            format!("{} top-level items", item_count),
            0,
        )]);
        Ok(content)
    }
}

/// Parsea las secciones de un Markdown (headings + contenido).
fn parse_markdown_sections(text: &str) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    let mut current_level = 0u8;
    let mut current_title = String::new();
    let mut current_content = String::new();
    let mut current_start = 0usize;

    for (idx, line) in text.lines().enumerate() {
        if let Some((level, title)) = parse_heading(line) {
            if current_level > 0 || !current_content.trim().is_empty() {
                sections.push(Section::new(
                    current_level,
                    current_title.clone(),
                    current_content.trim().to_string(),
                    current_start,
                ));
            }

            current_level = level;
            current_title = title;
            current_content.clear();
            current_start = idx;
        } else {
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(line);
        }
    }

    if current_level > 0 || !current_content.trim().is_empty() {
        sections.push(Section::new(
            current_level,
            current_title,
            current_content.trim().to_string(),
            current_start,
        ));
    }

    sections.retain(|s| s.is_heading() || !s.content.is_empty());
    sections
}

/// Parsea un heading Markdown (`# Title`).
fn parse_heading(line: &str) -> Option<(u8, String)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }

    let mut level = 0u8;
    for c in trimmed.chars() {
        if c == '#' {
            level += 1;
        } else {
            break;
        }
    }

    if level == 0 || level > 6 {
        return None;
    }

    let rest = &trimmed[level as usize..];
    if !rest.starts_with(' ') {
        return None;
    }

    let title = rest.trim_start().trim_end_matches('#').trim().to_string();
    Some((level, title))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn load_fails_when_not_found() {
        let loader = DocumentLoader::new();
        let err = loader.load("/definitely/not/a/file.txt").unwrap_err();
        assert!(matches!(err, DocumentError::NotFound(_)));
    }

    #[test]
    fn load_fails_on_directory() {
        let dir = TempDir::new().unwrap();
        let loader = DocumentLoader::new();
        let err = loader.load(dir.path()).unwrap_err();
        assert!(matches!(err, DocumentError::InvalidPath(_)));
    }

    #[test]
    fn load_text_file() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "notes.txt", "hello world\nsecond line");

        let loader = DocumentLoader::new();
        let loaded = loader.load(&path).unwrap();

        assert_eq!(loaded.document.format, DocumentFormat::Text);
        assert_eq!(loaded.content.line_count, 2);
        assert!(loaded.content.extracted);
    }

    #[test]
    fn load_markdown_file() {
        let dir = TempDir::new().unwrap();
        let path = write(
            dir.path(),
            "README.md",
            "# Intro\n\nHello world\n\n## Section 2\n\nMore content.",
        );

        let loader = DocumentLoader::new();
        let loaded = loader.load(&path).unwrap();

        assert_eq!(loaded.document.format, DocumentFormat::Markdown);
        assert!(loaded.content.extracted);
        assert_eq!(loaded.document.title.as_deref(), Some("Intro"));

        let headings: Vec<_> = loaded
            .content
            .sections
            .iter()
            .filter(|s| s.is_heading())
            .collect();
        assert!(headings.len() >= 2);
    }

    #[test]
    fn load_markdown_detects_section_levels() {
        let dir = TempDir::new().unwrap();
        let path = write(
            dir.path(),
            "doc.md",
            "# H1\n\ncontent\n\n## H2\n\nmore\n\n### H3\n\ndetail",
        );

        let loader = DocumentLoader::new();
        let loaded = loader.load(&path).unwrap();

        let levels: Vec<u8> = loaded
            .content
            .sections
            .iter()
            .filter(|s| s.is_heading())
            .map(|s| s.level)
            .collect();

        assert!(levels.contains(&1));
        assert!(levels.contains(&2));
        assert!(levels.contains(&3));
    }

    #[test]
    fn load_csv_file() {
        let dir = TempDir::new().unwrap();
        let path = write(
            dir.path(),
            "data.csv",
            "name,age\nAlice,30\nBob,25\nCharlie,40",
        );

        let loader = DocumentLoader::new();
        let loaded = loader.load(&path).unwrap();

        assert_eq!(loaded.document.format, DocumentFormat::Csv);
        assert!(loaded.content.extracted);
        assert!(!loaded.content.sections.is_empty());
    }

    #[test]
    fn load_json_file() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "config.json", r#"{"a": 1, "b": 2, "c": 3}"#);

        let loader = DocumentLoader::new();
        let loaded = loader.load(&path).unwrap();

        assert_eq!(loaded.document.format, DocumentFormat::Json);
        assert!(loaded.content.extracted);
        assert!(!loaded.content.sections.is_empty());
    }

    #[test]
    fn load_json_invalid_fails() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "bad.json", "not json at all");

        let loader = DocumentLoader::new();
        let err = loader.load(&path).unwrap_err();
        assert!(matches!(err, DocumentError::ParseError(_)));
    }

    #[test]
    fn load_pdf_returns_empty_content() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "book.pdf", "fake pdf content");

        let loader = DocumentLoader::new();
        let loaded = loader.load(&path).unwrap();

        assert_eq!(loaded.document.format, DocumentFormat::Pdf);
        assert!(!loaded.content.extracted);
    }

    #[test]
    fn load_unknown_format_fails() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "file.xyz", "content");

        let loader = DocumentLoader::new();
        let err = loader.load(&path).unwrap_err();
        assert!(matches!(err, DocumentError::UnsupportedFormat(_)));
    }

    #[test]
    fn title_falls_back_to_filename() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "notes.txt", "no headings here");

        let loader = DocumentLoader::new();
        let loaded = loader.load(&path).unwrap();
        assert_eq!(loaded.document.title.as_deref(), Some("notes.txt"));
    }

    #[test]
    fn parse_heading_works() {
        assert_eq!(parse_heading("# Title"), Some((1, "Title".to_string())));
        assert_eq!(parse_heading("## Sub"), Some((2, "Sub".to_string())));
        assert_eq!(parse_heading("### Deep"), Some((3, "Deep".to_string())));
        assert_eq!(parse_heading("#NoSpace"), None);
        assert_eq!(parse_heading("not a heading"), None);
        assert_eq!(parse_heading(""), None);
    }

    #[test]
    fn parse_heading_strips_trailing_hashes() {
        assert_eq!(parse_heading("## Title ##"), Some((2, "Title".to_string())));
    }

    #[test]
    fn parse_heading_ignores_level_seven() {
        assert_eq!(parse_heading("####### x"), None);
    }

    #[test]
    fn parse_markdown_sections_empty() {
        let sections = parse_markdown_sections("");
        assert!(sections.is_empty());
    }

    #[test]
    fn parse_markdown_sections_only_text() {
        let sections = parse_markdown_sections("just plain text");
        assert_eq!(sections.len(), 1);
        assert!(!sections[0].is_heading());
    }

    #[test]
    fn parse_markdown_sections_only_headings() {
        let sections = parse_markdown_sections("# A\n## B\n### C");
        assert_eq!(sections.len(), 3);
    }

    #[test]
    fn loaded_document_summary_works() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "test.md", "# Title\n\ncontent");

        let loader = DocumentLoader::new();
        let loaded = loader.load(&path).unwrap();
        let s = loaded.summary();
        assert!(s.contains("test.md"));
        assert!(s.contains("markdown"));
    }
}
