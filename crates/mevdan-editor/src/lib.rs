//! # mevdan-editor
//!
//! Editor de texto programático para MEVDAN.
//!
//! ## Concepto
//!
//! Un **buffer** es un archivo abierto en memoria, con:
//!
//! - Texto actual.
//! - Versión (se incrementa en cada edit).
//! - Estado "dirty" (cambios sin guardar).
//! - Contenido original (para poder revertir).
//!
//! El `EditorEngine` gestiona múltiples buffers y permite aplicar
//! **edits** (reemplazos, inserciones, borrados).
//!
//! ## Qué NO hace
//!
//! - **No es un editor gráfico.** No hay UI.
//! - **No usa LSP.** La integración LSP llega en V5.9.
//! - **No tiene syntax highlighting.** Es un motor de edición programático.
//! - **No tiene undo/redo explícito.** El `revert` vuelve al estado
//!   original; el undo fino es responsabilidad del llamador.
//!
//! ## Estado del proyecto
//!
//! - **V5.7** ✅ — `Buffer`, `TextEdit`, `EditorEngine`.
//!
//! ## Ejemplo
//!
//! ```no_run
//! use mevdan_editor::{EditorEngine, TextEdit, TextRange};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut engine = EditorEngine::new();
//! engine.open_from_text("/path/to/file.rs", "fn main() {}\n")?;
//!
//! // Reemplaza "main" por "start".
//! let range = TextRange::new(3, 7)?;
//! engine.apply_edits(
//!     "/path/to/file.rs",
//!     vec![TextEdit::new(range, "start")],
//! )?;
//!
//! engine.save("/path/to/file.rs")?;
//! engine.close("/path/to/file.rs", false)?;
//! # Ok(())
//! # }
//! ```

pub mod buffer;
pub mod edit;
pub mod engine;
pub mod error;

// Re-exports de conveniencia.
pub use buffer::{Buffer, BufferVersion, TextRange};
pub use edit::{apply_edits, EditResult, TextEdit};
pub use engine::EditorEngine;
pub use error::{EditorError, EditorResult};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_flow_refactor_rename() {
        let mut engine = EditorEngine::new();
        let path = "/synthetic/main.rs";
        engine
            .open_from_text(
                path,
                "fn old_name() {\n    println!(\"hi\");\n}\n\nfn main() {\n    old_name();\n}\n",
            )
            .unwrap();

        // Encuentra todas las ocurrencias de "old_name".
        let text = engine.require(path).unwrap().text().to_string();
        let mut edits = Vec::new();
        let target = "old_name";
        let mut start = 0;
        while let Some(pos) = text[start..].find(target) {
            let abs_pos = start + pos;
            edits.push(TextEdit::new(
                TextRange::new(abs_pos, abs_pos + target.len()).unwrap(),
                "new_name",
            ));
            start = abs_pos + target.len();
        }

        let result = engine.apply_edits(path, edits).unwrap();
        assert_eq!(result.edits_applied, 2);
        assert_eq!(
            engine.require(path).unwrap().text(),
            "fn new_name() {\n    println!(\"hi\");\n}\n\nfn main() {\n    new_name();\n}\n"
        );
    }

    #[test]
    fn full_flow_multi_file_edit() {
        let mut engine = EditorEngine::new();

        engine
            .open_from_text("/synthetic/a.rs", "let x = 1;")
            .unwrap();
        engine
            .open_from_text("/synthetic/b.rs", "let y = 2;")
            .unwrap();

        engine
            .apply_edits(
                "/synthetic/a.rs",
                vec![TextEdit::new(TextRange::new(8, 9).unwrap(), "10")],
            )
            .unwrap();
        engine
            .apply_edits(
                "/synthetic/b.rs",
                vec![TextEdit::new(TextRange::new(8, 9).unwrap(), "20")],
            )
            .unwrap();

        assert_eq!(
            engine.require("/synthetic/a.rs").unwrap().text(),
            "let x = 10;"
        );
        assert_eq!(
            engine.require("/synthetic/b.rs").unwrap().text(),
            "let y = 20;"
        );

        let versions = engine.versions();
        assert_eq!(versions.len(), 2);
    }
}
