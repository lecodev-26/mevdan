//! `Buffer` — contenido de un archivo en memoria.
//!
//! Un buffer mantiene:
//!
//! - El texto actual.
//! - Una versión (se incrementa en cada edit).
//! - Si tiene cambios sin guardar (`dirty`).
//! - El contenido original (para calcular el diff, si se quiere).

use crate::error::{EditorError, EditorResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Versión de un buffer. Se incrementa en cada edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BufferVersion(pub u64);

impl BufferVersion {
    pub fn initial() -> Self {
        Self(0)
    }

    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

impl std::fmt::Display for BufferVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

/// Rango de texto (byte offsets, half-open [start, end)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    pub fn new(start: usize, end: usize) -> EditorResult<Self> {
        if start > end {
            return Err(EditorError::InvalidRange { start, end });
        }
        Ok(Self { start, end })
    }

    pub fn empty(at: usize) -> Self {
        Self { start: at, end: at }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// Un buffer abierto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Buffer {
    pub path: PathBuf,
    pub version: BufferVersion,
    text: String,
    /// ¿Tiene cambios sin guardar?
    pub dirty: bool,
    /// Contenido cuando se abrió.
    original: String,
}

impl Buffer {
    /// Crea un buffer desde un path y contenido.
    pub fn from_text(path: impl Into<PathBuf>, text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            path: path.into(),
            version: BufferVersion::initial(),
            original: text.clone(),
            text,
            dirty: false,
        }
    }

    /// Abre un buffer desde un archivo.
    pub fn open(path: impl AsRef<Path>) -> EditorResult<Self> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path)?;
        Ok(Self::from_text(path.to_path_buf(), text))
    }

    /// Texto actual.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Contenido original (cuando se abrió).
    pub fn original(&self) -> &str {
        &self.original
    }

    /// Longitud en bytes.
    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Aplica un reemplazo de bytes [start, end) por `replacement`.
    pub fn replace(&mut self, range: TextRange, replacement: &str) -> EditorResult<()> {
        self.check_range(range)?;
        self.check_boundary(range.start)?;
        self.check_boundary(range.end)?;

        self.text.replace_range(range.start..range.end, replacement);
        self.version = self.version.next();
        self.dirty = true;
        Ok(())
    }

    /// Inserta texto en la posición `at`.
    pub fn insert(&mut self, at: usize, text: &str) -> EditorResult<()> {
        self.replace(TextRange::empty(at), text)
    }

    /// Borra [start, end).
    pub fn delete(&mut self, range: TextRange) -> EditorResult<()> {
        self.replace(range, "")
    }

    /// Guarda el buffer en disco y limpia `dirty`.
    pub fn save(&mut self) -> EditorResult<()> {
        std::fs::write(&self.path, &self.text)?;
        self.original = self.text.clone();
        self.dirty = false;
        Ok(())
    }

    /// Descarta los cambios y vuelve al contenido original.
    pub fn revert(&mut self) {
        self.text = self.original.clone();
        self.version = self.version.next();
        self.dirty = false;
    }

    /// Comprueba que un rango está dentro de los límites.
    fn check_range(&self, range: TextRange) -> EditorResult<()> {
        if range.end > self.text.len() {
            return Err(EditorError::RangeOutOfBounds {
                range: range.end,
                size: self.text.len(),
            });
        }
        Ok(())
    }

    /// Comprueba que una posición está en un char boundary.
    fn check_boundary(&self, pos: usize) -> EditorResult<()> {
        if !self.text.is_char_boundary(pos) {
            return Err(EditorError::NotOnCharBoundary { position: pos });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Buffer {
        Buffer::from_text("/tmp/test.txt", "Hello world")
    }

    #[test]
    fn version_initial() {
        assert_eq!(BufferVersion::initial(), BufferVersion(0));
    }

    #[test]
    fn version_next() {
        assert_eq!(BufferVersion::initial().next(), BufferVersion(1));
    }

    #[test]
    fn version_display() {
        assert_eq!(BufferVersion(5).to_string(), "v5");
    }

    #[test]
    fn text_range_new() {
        let r = TextRange::new(0, 5).unwrap();
        assert_eq!(r.start, 0);
        assert_eq!(r.end, 5);
        assert_eq!(r.len(), 5);
        assert!(!r.is_empty());
    }

    #[test]
    fn text_range_invalid() {
        let err = TextRange::new(5, 0).unwrap_err();
        assert!(matches!(err, EditorError::InvalidRange { .. }));
    }

    #[test]
    fn text_range_empty() {
        let r = TextRange::empty(3);
        assert!(r.is_empty());
        assert_eq!(r.start, 3);
        assert_eq!(r.end, 3);
    }

    #[test]
    fn from_text_creates() {
        let b = Buffer::from_text("/tmp/test.txt", "Hello");
        assert_eq!(b.text(), "Hello");
        assert_eq!(b.version, BufferVersion::initial());
        assert!(!b.dirty);
    }

    #[test]
    fn len_and_empty() {
        let b = sample();
        assert_eq!(b.len(), 11);
        assert!(!b.is_empty());

        let b = Buffer::from_text("/tmp/x", "");
        assert!(b.is_empty());
        assert_eq!(b.len(), 0);
    }

    #[test]
    fn insert_at_start() {
        let mut b = sample();
        b.insert(0, "Say: ").unwrap();
        assert_eq!(b.text(), "Say: Hello world");
        assert_eq!(b.version, BufferVersion(1));
        assert!(b.dirty);
    }

    #[test]
    fn insert_at_end() {
        let mut b = sample();
        b.insert(11, "!").unwrap();
        assert_eq!(b.text(), "Hello world!");
    }

    #[test]
    fn insert_middle() {
        let mut b = sample();
        b.insert(6, "beautiful ").unwrap();
        assert_eq!(b.text(), "Hello beautiful world");
    }

    #[test]
    fn replace_range() {
        let mut b = sample();
        b.replace(TextRange::new(0, 5).unwrap(), "Goodbye").unwrap();
        assert_eq!(b.text(), "Goodbye world");
    }

    #[test]
    fn delete_range() {
        let mut b = sample();
        b.delete(TextRange::new(5, 11).unwrap()).unwrap();
        assert_eq!(b.text(), "Hello");
    }

    #[test]
    fn version_increments() {
        let mut b = sample();
        b.insert(0, "a").unwrap();
        b.insert(0, "b").unwrap();
        b.insert(0, "c").unwrap();
        assert_eq!(b.version, BufferVersion(3));
    }

    #[test]
    fn range_out_of_bounds_fails() {
        let mut b = sample();
        let err = b.replace(TextRange::new(0, 100).unwrap(), "x").unwrap_err();
        assert!(matches!(err, EditorError::RangeOutOfBounds { .. }));
    }

    #[test]
    fn range_not_on_char_boundary_fails() {
        let mut b = Buffer::from_text("/tmp/x", "café");
        // "é" ocupa 2 bytes (0xC3, 0xA9). Posición 4 está entre bytes.
        let err = b.replace(TextRange::new(0, 4).unwrap(), "x").unwrap_err();
        assert!(matches!(err, EditorError::NotOnCharBoundary { .. }));
    }

    #[test]
    fn revert_restores_original() {
        let mut b = sample();
        b.insert(0, "changed ").unwrap();
        assert!(b.dirty);
        b.revert();
        assert_eq!(b.text(), "Hello world");
        assert!(!b.dirty);
    }

    #[test]
    fn save_writes_and_clears_dirty() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "original").unwrap();

        let mut b = Buffer::open(&path).unwrap();
        b.insert(0, "modified ").unwrap();
        assert!(b.dirty);

        b.save().unwrap();
        assert!(!b.dirty);

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "modified original");
    }

    #[test]
    fn open_reads_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "file content").unwrap();

        let b = Buffer::open(&path).unwrap();
        assert_eq!(b.text(), "file content");
        assert!(!b.dirty);
    }

    #[test]
    fn open_missing_file_fails() {
        let err = Buffer::open("/definitely/not/a/file.txt").unwrap_err();
        assert!(matches!(err, EditorError::Io(_)));
    }

    #[test]
    fn buffer_serializes() {
        let b = sample();
        let json = serde_json::to_string(&b).unwrap();
        let back: Buffer = serde_json::from_str(&json).unwrap();
        assert_eq!(back.text(), b.text());
        assert_eq!(back.version, b.version);
    }

    #[test]
    fn utf8_edits_work() {
        // Usamos strings construidos para no depender de códigos exactos.
        let prefix = "caf\u{00e9} "; // "café " = 6 bytes
        let emoji = "\u{2615}"; // "☕" = 3 bytes (U+2615)
        let replacement = "\u{1F375}"; // "🍵" = 4 bytes (U+1F375)

        let source = format!("{}{}", prefix, emoji);
        let mut b = Buffer::from_text("/tmp/x", source);

        let start = prefix.len();
        let end = start + emoji.len();
        b.replace(TextRange::new(start, end).unwrap(), replacement)
            .unwrap();

        let expected = format!("{}{}", prefix, replacement);
        assert_eq!(b.text(), expected);
    }
}
