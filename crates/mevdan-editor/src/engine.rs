//! `EditorEngine` — gestión de buffers.

use crate::{
    buffer::{Buffer, BufferVersion},
    edit::{apply_edits, EditResult, TextEdit},
    error::{EditorError, EditorResult},
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Motor de edición: mantiene buffers abiertos.
#[derive(Debug, Default)]
pub struct EditorEngine {
    buffers: BTreeMap<PathBuf, Buffer>,
}

impl EditorEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Abre un archivo como buffer.
    ///
    /// Falla si ya está abierto.
    pub fn open(&mut self, path: impl AsRef<Path>) -> EditorResult<&Buffer> {
        let path = path.as_ref().to_path_buf();
        if self.buffers.contains_key(&path) {
            return Err(EditorError::BufferAlreadyOpen(path.display().to_string()));
        }
        let buffer = Buffer::open(&path)?;
        self.buffers.insert(path.clone(), buffer);
        Ok(self.buffers.get(&path).unwrap())
    }

    /// Abre un buffer desde texto (útil para tests y ediciones sintéticas).
    pub fn open_from_text(
        &mut self,
        path: impl Into<PathBuf>,
        text: impl Into<String>,
    ) -> EditorResult<()> {
        let path = path.into();
        if self.buffers.contains_key(&path) {
            return Err(EditorError::BufferAlreadyOpen(path.display().to_string()));
        }
        self.buffers
            .insert(path.clone(), Buffer::from_text(path, text));
        Ok(())
    }

    /// Cierra un buffer. Falla si tiene cambios sin guardar.
    ///
    /// Usa `force=true` para cerrar sin guardar.
    pub fn close(&mut self, path: impl AsRef<Path>, force: bool) -> EditorResult<Buffer> {
        let path = path.as_ref();
        let buffer = self
            .buffers
            .get(path)
            .ok_or_else(|| EditorError::BufferNotFound(path.display().to_string()))?;

        if buffer.dirty && !force {
            return Err(EditorError::UnsavedChanges(path.display().to_string()));
        }

        Ok(self.buffers.remove(path).unwrap())
    }

    /// Obtiene un buffer.
    pub fn get(&self, path: impl AsRef<Path>) -> Option<&Buffer> {
        self.buffers.get(path.as_ref())
    }

    /// Obtiene un buffer mutable.
    pub fn get_mut(&mut self, path: impl AsRef<Path>) -> Option<&mut Buffer> {
        self.buffers.get_mut(path.as_ref())
    }

    /// Obtiene un buffer o error.
    pub fn require(&self, path: impl AsRef<Path>) -> EditorResult<&Buffer> {
        let path = path.as_ref();
        self.buffers
            .get(path)
            .ok_or_else(|| EditorError::BufferNotFound(path.display().to_string()))
    }

    /// Aplica edits a un buffer abierto.
    pub fn apply_edits(
        &mut self,
        path: impl AsRef<Path>,
        edits: Vec<TextEdit>,
    ) -> EditorResult<EditResult> {
        let path = path.as_ref();
        let buffer = self
            .buffers
            .get_mut(path)
            .ok_or_else(|| EditorError::BufferNotFound(path.display().to_string()))?;

        let start_version = buffer.version;
        let result = apply_edits(buffer, edits)?;

        // Si algo falló a mitad, el buffer queda en un estado intermedio,
        // pero como el error se propaga, el llamador sabe que hubo problema.
        debug_assert!(result.final_version >= start_version);

        Ok(result)
    }

    /// Guarda un buffer en disco.
    pub fn save(&mut self, path: impl AsRef<Path>) -> EditorResult<()> {
        let path = path.as_ref();
        let buffer = self
            .buffers
            .get_mut(path)
            .ok_or_else(|| EditorError::BufferNotFound(path.display().to_string()))?;
        buffer.save()
    }

    /// Guarda todos los buffers con cambios.
    pub fn save_all(&mut self) -> EditorResult<usize> {
        let mut saved = 0;
        for buffer in self.buffers.values_mut() {
            if buffer.dirty {
                buffer.save()?;
                saved += 1;
            }
        }
        Ok(saved)
    }

    /// Revierte un buffer a su estado original.
    pub fn revert(&mut self, path: impl AsRef<Path>) -> EditorResult<()> {
        let path = path.as_ref();
        let buffer = self
            .buffers
            .get_mut(path)
            .ok_or_else(|| EditorError::BufferNotFound(path.display().to_string()))?;
        buffer.revert();
        Ok(())
    }

    /// Número de buffers abiertos.
    pub fn len(&self) -> usize {
        self.buffers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }

    /// Lista de paths abiertos.
    pub fn paths(&self) -> Vec<&Path> {
        self.buffers.keys().map(|p| p.as_path()).collect()
    }

    /// Lista de buffers con cambios sin guardar.
    pub fn dirty_paths(&self) -> Vec<&Path> {
        self.buffers
            .iter()
            .filter(|(_, b)| b.dirty)
            .map(|(p, _)| p.as_path())
            .collect()
    }

    /// Cierra todos los buffers. Falla si alguno tiene cambios.
    pub fn close_all(&mut self, force: bool) -> EditorResult<usize> {
        if !force {
            let dirty = self.dirty_paths();
            if !dirty.is_empty() {
                return Err(EditorError::UnsavedChanges(format!(
                    "{} buffer(s) with unsaved changes",
                    dirty.len()
                )));
            }
        }
        let n = self.buffers.len();
        self.buffers.clear();
        Ok(n)
    }

    /// Versiones de todos los buffers (para auditoría).
    pub fn versions(&self) -> BTreeMap<PathBuf, BufferVersion> {
        self.buffers
            .iter()
            .map(|(p, b)| (p.clone(), b.version))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::TextRange;
    use tempfile::TempDir;

    fn setup() -> (TempDir, EditorEngine) {
        let dir = TempDir::new().unwrap();
        let engine = EditorEngine::new();
        (dir, engine)
    }

    fn write_and_open(
        dir: &TempDir,
        engine: &mut EditorEngine,
        name: &str,
        content: &str,
    ) -> PathBuf {
        let path = dir.path().join(name);
        std::fs::write(&path, content).unwrap();
        engine.open(&path).unwrap();
        path
    }

    #[test]
    fn new_engine_is_empty() {
        let engine = EditorEngine::new();
        assert!(engine.is_empty());
        assert_eq!(engine.len(), 0);
    }

    #[test]
    fn open_reads_file() {
        let (dir, mut engine) = setup();
        let path = write_and_open(&dir, &mut engine, "test.txt", "Hello");

        assert_eq!(engine.len(), 1);
        assert_eq!(engine.get(&path).unwrap().text(), "Hello");
    }

    #[test]
    fn open_duplicate_fails() {
        let (dir, mut engine) = setup();
        let path = write_and_open(&dir, &mut engine, "test.txt", "Hello");

        let err = engine.open(&path).unwrap_err();
        assert!(matches!(err, EditorError::BufferAlreadyOpen(_)));
    }

    #[test]
    fn open_from_text_creates_buffer() {
        let mut engine = EditorEngine::new();
        engine
            .open_from_text("/synthetic/test.txt", "hello")
            .unwrap();

        assert_eq!(engine.len(), 1);
        assert_eq!(
            engine.require("/synthetic/test.txt").unwrap().text(),
            "hello"
        );
    }

    #[test]
    fn open_from_text_duplicate_fails() {
        let mut engine = EditorEngine::new();
        engine.open_from_text("/x", "a").unwrap();
        let err = engine.open_from_text("/x", "b").unwrap_err();
        assert!(matches!(err, EditorError::BufferAlreadyOpen(_)));
    }

    #[test]
    fn close_removes_buffer() {
        let (dir, mut engine) = setup();
        let path = write_and_open(&dir, &mut engine, "test.txt", "Hello");

        engine.close(&path, false).unwrap();
        assert!(engine.is_empty());
    }

    #[test]
    fn close_unknown_fails() {
        let mut engine = EditorEngine::new();
        let err = engine.close("/nope", false).unwrap_err();
        assert!(matches!(err, EditorError::BufferNotFound(_)));
    }

    #[test]
    fn close_dirty_fails_without_force() {
        let (dir, mut engine) = setup();
        let path = write_and_open(&dir, &mut engine, "test.txt", "Hello");

        engine
            .apply_edits(&path, vec![TextEdit::insert(0, "X")])
            .unwrap();

        let err = engine.close(&path, false).unwrap_err();
        assert!(matches!(err, EditorError::UnsavedChanges(_)));

        // Con force, funciona.
        engine.close(&path, true).unwrap();
        assert!(engine.is_empty());
    }

    #[test]
    fn require_returns_buffer() {
        let (dir, mut engine) = setup();
        let path = write_and_open(&dir, &mut engine, "test.txt", "Hello");

        let buffer = engine.require(&path).unwrap();
        assert_eq!(buffer.text(), "Hello");
    }

    #[test]
    fn require_unknown_fails() {
        let engine = EditorEngine::new();
        let err = engine.require("/nope").unwrap_err();
        assert!(matches!(err, EditorError::BufferNotFound(_)));
    }

    #[test]
    fn apply_edits_to_buffer() {
        let (dir, mut engine) = setup();
        let path = write_and_open(&dir, &mut engine, "test.txt", "Hello world");

        let result = engine
            .apply_edits(
                &path,
                vec![TextEdit::new(TextRange::new(0, 5).unwrap(), "Goodbye")],
            )
            .unwrap();

        assert_eq!(result.edits_applied, 1);
        assert_eq!(engine.require(&path).unwrap().text(), "Goodbye world");
    }

    #[test]
    fn apply_edits_unknown_buffer_fails() {
        let mut engine = EditorEngine::new();
        let err = engine
            .apply_edits("/nope", vec![TextEdit::insert(0, "x")])
            .unwrap_err();
        assert!(matches!(err, EditorError::BufferNotFound(_)));
    }

    #[test]
    fn save_writes_to_disk() {
        let (dir, mut engine) = setup();
        let path = write_and_open(&dir, &mut engine, "test.txt", "Hello");

        engine
            .apply_edits(&path, vec![TextEdit::insert(0, "Changed: ")])
            .unwrap();
        engine.save(&path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "Changed: Hello");
        assert!(!engine.require(&path).unwrap().dirty);
    }

    #[test]
    fn save_all_saves_dirty_buffers() {
        let (dir, mut engine) = setup();
        let p1 = write_and_open(&dir, &mut engine, "a.txt", "A");
        let p2 = write_and_open(&dir, &mut engine, "b.txt", "B");

        engine
            .apply_edits(&p1, vec![TextEdit::insert(0, "X")])
            .unwrap();
        engine
            .apply_edits(&p2, vec![TextEdit::insert(0, "Y")])
            .unwrap();

        let saved = engine.save_all().unwrap();
        assert_eq!(saved, 2);
        assert!(engine.dirty_paths().is_empty());
    }

    #[test]
    fn revert_undoes_changes() {
        let (dir, mut engine) = setup();
        let path = write_and_open(&dir, &mut engine, "test.txt", "Hello");

        engine
            .apply_edits(&path, vec![TextEdit::insert(0, "X")])
            .unwrap();
        engine.revert(&path).unwrap();
        assert_eq!(engine.require(&path).unwrap().text(), "Hello");
    }

    #[test]
    fn paths_lists_open_buffers() {
        let (dir, mut engine) = setup();
        write_and_open(&dir, &mut engine, "a.txt", "A");
        write_and_open(&dir, &mut engine, "b.txt", "B");

        let paths = engine.paths();
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn dirty_paths_filters() {
        let (dir, mut engine) = setup();
        let p1 = write_and_open(&dir, &mut engine, "a.txt", "A");
        let _p2 = write_and_open(&dir, &mut engine, "b.txt", "B");

        engine
            .apply_edits(&p1, vec![TextEdit::insert(0, "X")])
            .unwrap();

        let dirty = engine.dirty_paths();
        assert_eq!(dirty.len(), 1);
        assert_eq!(dirty[0], p1.as_path());
    }

    #[test]
    fn close_all_fails_with_dirty() {
        let (dir, mut engine) = setup();
        let path = write_and_open(&dir, &mut engine, "a.txt", "A");
        engine
            .apply_edits(&path, vec![TextEdit::insert(0, "X")])
            .unwrap();

        let err = engine.close_all(false).unwrap_err();
        assert!(matches!(err, EditorError::UnsavedChanges(_)));

        let n = engine.close_all(true).unwrap();
        assert_eq!(n, 1);
        assert!(engine.is_empty());
    }

    #[test]
    fn versions_returns_all() {
        let (dir, mut engine) = setup();
        let p1 = write_and_open(&dir, &mut engine, "a.txt", "A");
        let p2 = write_and_open(&dir, &mut engine, "b.txt", "B");

        engine
            .apply_edits(&p1, vec![TextEdit::insert(0, "X")])
            .unwrap();
        engine
            .apply_edits(&p2, vec![TextEdit::insert(0, "Y")])
            .unwrap();
        engine
            .apply_edits(&p2, vec![TextEdit::insert(0, "Z")])
            .unwrap();

        let versions = engine.versions();
        assert_eq!(versions.get(&p1), Some(&BufferVersion(1)));
        assert_eq!(versions.get(&p2), Some(&BufferVersion(2)));
    }

    #[test]
    fn full_flow_open_edit_save_close() {
        let (dir, mut engine) = setup();
        let path = write_and_open(&dir, &mut engine, "code.rs", "fn main() {}\n");

        // Reemplaza "main" por "start".
        engine
            .apply_edits(
                &path,
                vec![TextEdit::new(TextRange::new(3, 7).unwrap(), "start")],
            )
            .unwrap();

        engine.save(&path).unwrap();
        engine.close(&path, false).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "fn start() {}\n");
    }
}
