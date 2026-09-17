//! `TextEdit` y `EditResult` — operaciones de edición.

use crate::{
    buffer::{Buffer, TextRange},
    error::{EditorError, EditorResult},
};
use serde::{Deserialize, Serialize};

/// Una operación de edición: reemplazar un rango por nuevo texto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEdit {
    pub range: TextRange,
    pub new_text: String,
}

impl TextEdit {
    pub fn new(range: TextRange, new_text: impl Into<String>) -> Self {
        Self {
            range,
            new_text: new_text.into(),
        }
    }

    /// Inserta texto en una posición.
    pub fn insert(at: usize, text: impl Into<String>) -> Self {
        Self {
            range: TextRange::empty(at),
            new_text: text.into(),
        }
    }

    /// Borra un rango.
    pub fn delete(range: TextRange) -> Self {
        Self {
            range,
            new_text: String::new(),
        }
    }

    /// Reemplaza un rango.
    pub fn replace(range: TextRange, text: impl Into<String>) -> Self {
        Self::new(range, text)
    }

    /// Longitud del rango afectado.
    pub fn deleted_len(&self) -> usize {
        self.range.len()
    }

    /// Longitud del texto insertado.
    pub fn inserted_len(&self) -> usize {
        self.new_text.len()
    }

    /// Aplica el edit a un buffer.
    pub fn apply(&self, buffer: &mut Buffer) -> EditorResult<()> {
        buffer.replace(self.range, &self.new_text)
    }
}

/// Resultado de aplicar un conjunto de edits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditResult {
    /// Número de edits aplicados.
    pub edits_applied: usize,
    /// Bytes eliminados en total.
    pub bytes_removed: usize,
    /// Bytes insertados en total.
    pub bytes_inserted: usize,
    /// Versión final del buffer.
    pub final_version: crate::buffer::BufferVersion,
}

impl EditResult {
    pub fn new() -> Self {
        Self {
            edits_applied: 0,
            bytes_removed: 0,
            bytes_inserted: 0,
            final_version: crate::buffer::BufferVersion::initial(),
        }
    }

    /// Balance neto de bytes (inserted - removed).
    pub fn net_bytes(&self) -> isize {
        self.bytes_inserted as isize - self.bytes_removed as isize
    }

    /// Resumen textual.
    pub fn summary(&self) -> String {
        format!(
            "{} edits applied: {} bytes removed, {} bytes inserted (net {:+}), {}",
            self.edits_applied,
            self.bytes_removed,
            self.bytes_inserted,
            self.net_bytes(),
            self.final_version,
        )
    }
}

impl Default for EditResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Aplica una lista de edits a un buffer.
///
/// Los edits se aplican en orden de rango descendente para no
/// invalidar posiciones. Si dos edits se solapan, devuelve error.
pub fn apply_edits(buffer: &mut Buffer, mut edits: Vec<TextEdit>) -> EditorResult<EditResult> {
    // Ordena por rango descendente (end, luego start) para aplicar
    // de atrás hacia adelante.
    edits.sort_by(|a, b| {
        b.range
            .start
            .cmp(&a.range.start)
            .then(b.range.end.cmp(&a.range.end))
    });

    // Comprueba solapamientos.
    for pair in edits.windows(2) {
        if pair[1].range.end > pair[0].range.start {
            return Err(EditorError::InvalidRange {
                start: pair[1].range.start,
                end: pair[0].range.end,
            });
        }
    }

    let mut result = EditResult::new();

    for edit in &edits {
        edit.apply(buffer)?;
        result.edits_applied += 1;
        result.bytes_removed += edit.deleted_len();
        result.bytes_inserted += edit.inserted_len();
    }

    result.final_version = buffer.version;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_buffer() -> Buffer {
        Buffer::from_text("/tmp/test.txt", "Hello world")
    }

    #[test]
    fn edit_new() {
        let e = TextEdit::new(TextRange::new(0, 5).unwrap(), "Goodbye");
        assert_eq!(e.range.start, 0);
        assert_eq!(e.new_text, "Goodbye");
    }

    #[test]
    fn edit_insert() {
        let e = TextEdit::insert(0, "hi");
        assert_eq!(e.range.start, 0);
        assert_eq!(e.range.end, 0);
        assert_eq!(e.new_text, "hi");
    }

    #[test]
    fn edit_delete() {
        let e = TextEdit::delete(TextRange::new(0, 5).unwrap());
        assert_eq!(e.new_text, "");
        assert_eq!(e.deleted_len(), 5);
    }

    #[test]
    fn edit_lengths() {
        let e = TextEdit::replace(TextRange::new(0, 5).unwrap(), "hi");
        assert_eq!(e.deleted_len(), 5);
        assert_eq!(e.inserted_len(), 2);
    }

    #[test]
    fn edit_apply() {
        let mut b = sample_buffer();
        let e = TextEdit::new(TextRange::new(0, 5).unwrap(), "Goodbye");
        e.apply(&mut b).unwrap();
        assert_eq!(b.text(), "Goodbye world");
    }

    #[test]
    fn result_new() {
        let r = EditResult::new();
        assert_eq!(r.edits_applied, 0);
        assert_eq!(r.net_bytes(), 0);
    }

    #[test]
    fn result_net_bytes() {
        let mut r = EditResult::new();
        r.bytes_removed = 10;
        r.bytes_inserted = 15;
        assert_eq!(r.net_bytes(), 5);
    }

    #[test]
    fn result_summary() {
        let mut r = EditResult::new();
        r.edits_applied = 3;
        r.bytes_removed = 10;
        r.bytes_inserted = 20;
        let s = r.summary();
        assert!(s.contains("3 edits"));
        assert!(s.contains("10 bytes removed"));
        assert!(s.contains("20 bytes inserted"));
    }

    #[test]
    fn apply_edits_single() {
        let mut b = sample_buffer();
        let edits = vec![TextEdit::new(TextRange::new(0, 5).unwrap(), "Goodbye")];
        let result = apply_edits(&mut b, edits).unwrap();
        assert_eq!(result.edits_applied, 1);
        assert_eq!(result.bytes_removed, 5);
        assert_eq!(result.bytes_inserted, 7);
        assert_eq!(b.text(), "Goodbye world");
    }

    #[test]
    fn apply_edits_multiple_non_overlapping() {
        let mut b = sample_buffer();
        let edits = vec![
            TextEdit::new(TextRange::new(0, 5).unwrap(), "Hi"),
            TextEdit::new(TextRange::new(6, 11).unwrap(), "earth"),
        ];
        let result = apply_edits(&mut b, edits).unwrap();
        assert_eq!(result.edits_applied, 2);
        assert_eq!(b.text(), "Hi earth");
    }

    #[test]
    fn apply_edits_inserts_at_multiple_positions() {
        let mut b = Buffer::from_text("/tmp/x", "abc");
        let edits = vec![
            TextEdit::insert(0, "X"),
            TextEdit::insert(1, "Y"),
            TextEdit::insert(2, "Z"),
        ];
        apply_edits(&mut b, edits).unwrap();
        // Inserts en orden descendente: posición 2 ("Z"), posición 1 ("Y"),
        // posición 0 ("X"). Resultado: "XaYbZc".
        assert_eq!(b.text(), "XaYbZc");
    }

    #[test]
    fn apply_edits_rejects_overlap() {
        let mut b = sample_buffer();
        let edits = vec![
            TextEdit::new(TextRange::new(0, 5).unwrap(), "x"),
            TextEdit::new(TextRange::new(3, 8).unwrap(), "y"),
        ];
        let err = apply_edits(&mut b, edits).unwrap_err();
        assert!(matches!(err, EditorError::InvalidRange { .. }));
    }

    #[test]
    fn apply_edits_empty() {
        let mut b = sample_buffer();
        let result = apply_edits(&mut b, vec![]).unwrap();
        assert_eq!(result.edits_applied, 0);
        assert_eq!(b.text(), "Hello world");
    }

    #[test]
    fn apply_edits_delete_all() {
        let mut b = sample_buffer();
        let edits = vec![TextEdit::delete(TextRange::new(0, 11).unwrap())];
        apply_edits(&mut b, edits).unwrap();
        assert_eq!(b.text(), "");
    }

    #[test]
    fn edit_result_serializes() {
        let mut r = EditResult::new();
        r.edits_applied = 2;
        let json = serde_json::to_string(&r).unwrap();
        let back: EditResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.edits_applied, 2);
    }

    #[test]
    fn text_edit_serializes() {
        let e = TextEdit::new(TextRange::new(0, 5).unwrap(), "x");
        let json = serde_json::to_string(&e).unwrap();
        let back: TextEdit = serde_json::from_str(&json).unwrap();
        assert_eq!(back, e);
    }
}
