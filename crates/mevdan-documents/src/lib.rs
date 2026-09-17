//! # mevdan-documents
//!
//! Procesamiento de documentos para MEVDAN.
//!
//! ## Concepto
//!
//! Un **documento** es un archivo con contenido textual estructurado
//! o no. `mevdan-documents` detecta el formato, extrae el texto y,
//! cuando aplica, la estructura (secciones, headings).
//!
//! ## Formatos soportados en V5.3
//!
//! | Formato | Detección | Extracción |
//! |---------|-----------|------------|
//! | Markdown | ✅ | ✅ (con secciones) |
//! | Text | ✅ | ✅ |
//! | CSV | ✅ | ✅ (metadata de filas) |
//! | JSON | ✅ | ✅ (metadata de items) |
//! | PDF | ✅ | ❌ (V5.5) |
//! | DOCX | ✅ | ❌ (V5.5) |
//! | EPUB | ✅ | ❌ (V5.5) |
//!
//! ## Estado del proyecto
//!
//! - **V5.3** ✅ — `DocumentFormat`, `Document`, `DocumentContent`, `DocumentLoader`.
//!
//! ## Ejemplo
//!
//! ```no_run
//! use mevdan_documents::DocumentLoader;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let loader = DocumentLoader::new();
//! let loaded = loader.load("/path/to/README.md")?;
//!
//! println!("{}", loaded.summary());
//! for section in &loaded.content.sections {
//!     if section.is_heading() {
//!         println!("  {} {}", "#".repeat(section.level as usize), section.title);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod document;
pub mod error;
pub mod format;
pub mod loader;

// Re-exports de conveniencia.
pub use document::{Document, DocumentContent, LoadedDocument, Section};
pub use error::{DocumentError, DocumentResult};
pub use format::DocumentFormat;
pub use loader::DocumentLoader;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn full_flow_markdown_analysis() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("README.md");
        fs::write(
            &path,
            "# Project\n\nIntro text here.\n\n## Usage\n\nHow to use it.\n\n### Examples\n\nCode samples.",
        )
        .unwrap();

        let loader = DocumentLoader::new();
        let loaded = loader.load(&path).unwrap();

        assert_eq!(loaded.document.format, DocumentFormat::Markdown);
        assert_eq!(loaded.document.title.as_deref(), Some("Project"));
        assert!(loaded.content.extracted);

        let headings: Vec<&Section> = loaded
            .content
            .sections
            .iter()
            .filter(|s| s.is_heading())
            .collect();
        assert_eq!(headings.len(), 3);
        assert_eq!(headings[0].title, "Project");
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[1].title, "Usage");
        assert_eq!(headings[1].level, 2);
        assert_eq!(headings[2].title, "Examples");
        assert_eq!(headings[2].level, 3);
    }

    #[test]
    fn full_flow_multiple_documents() {
        let dir = TempDir::new().unwrap();
        let loader = DocumentLoader::new();

        // Crea varios documentos.
        fs::write(dir.path().join("notes.txt"), "some notes").unwrap();
        fs::write(dir.path().join("config.json"), r#"{"key": "value"}"#).unwrap();
        fs::write(dir.path().join("data.csv"), "a,b\n1,2\n3,4").unwrap();
        fs::write(dir.path().join("readme.md"), "# Title\n\ntext").unwrap();

        let paths = ["notes.txt", "config.json", "data.csv", "readme.md"];
        let mut loaded_count = 0;

        for name in &paths {
            let loaded = loader.load(dir.path().join(name)).unwrap();
            assert!(loaded.content.extracted, "{} should be extracted", name);
            loaded_count += 1;
        }

        assert_eq!(loaded_count, 4);
    }

    #[test]
    fn full_flow_pdf_not_extracted() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("book.pdf");
        fs::write(&path, b"%PDF-1.4 fake").unwrap();

        let loader = DocumentLoader::new();
        let loaded = loader.load(&path).unwrap();

        assert_eq!(loaded.document.format, DocumentFormat::Pdf);
        assert!(!loaded.content.extracted);
        assert!(loaded.content.is_empty());
    }
}
