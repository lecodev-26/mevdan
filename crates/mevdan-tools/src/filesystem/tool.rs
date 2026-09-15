//! `FilesystemTool` — operaciones de filesystem con sandbox.
//!
//! ## Acciones soportadas
//!
//! - `read` — lee un archivo. Riesgo: LOW.
//! - `list` — lista un directorio. Riesgo: LOW.
//! - `exists` — comprueba si algo existe. Riesgo: LOW.
//! - `write` — escribe un archivo. Riesgo: MEDIUM.
//! - `delete` — borra un archivo. Riesgo: HIGH.
//!
//! ## Formato del input
//!
//! ```json
//! { "action": "read", "path": "subdir/file.txt" }
//! { "action": "write", "path": "new.txt", "content": "hello" }
//! ```
//!
//! ## Formato del output
//!
//! ```json
//! { "action": "read", "path": "subdir/file.txt", "content": "hello", "bytes": 5 }
//! { "action": "list", "path": "subdir", "entries": [...] }
//! { "action": "exists", "path": "x.txt", "exists": true }
//! { "action": "write", "path": "new.txt", "bytes_written": 5 }
//! { "action": "delete", "path": "old.txt", "deleted": true }
//! ```

use crate::{
    error::{ToolError, ToolResult},
    filesystem::sandbox::Sandbox,
    tool::{RiskLevel, Tool, ToolCategory, ToolMetadata},
};
use serde_json::{json, Value};
use std::fs;

/// Tool de filesystem con sandbox.
#[derive(Debug)]
pub struct FilesystemTool {
    metadata: ToolMetadata,
    sandbox: Sandbox,
}

impl FilesystemTool {
    /// Crea un tool de filesystem con un sandbox raíz.
    pub fn new(sandbox: Sandbox) -> Self {
        Self {
            metadata: ToolMetadata {
                name: "filesystem".into(),
                description: "Read, list, write, check existence, and delete files \
                              inside the project sandbox."
                    .into(),
                category: ToolCategory::Filesystem,
                risk: RiskLevel::Medium, // riesgo por defecto del tool; cada acción afina
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["read", "list", "exists", "write", "delete"]
                        },
                        "path": { "type": "string" },
                        "content": { "type": "string" }
                    },
                    "required": ["action", "path"]
                }),
                output_schema: json!({ "type": "object" }),
            },
            sandbox,
        }
    }

    /// Acceso al sandbox.
    pub fn sandbox(&self) -> &Sandbox {
        &self.sandbox
    }

    /// Riesgo según la acción.
    fn risk_for_action(action: &str) -> RiskLevel {
        match action {
            "read" | "list" | "exists" => RiskLevel::Low,
            "write" => RiskLevel::Medium,
            "delete" => RiskLevel::High,
            _ => RiskLevel::Critical,
        }
    }

    /// Despacha la acción.
    fn dispatch(&self, input: &Value) -> ToolResult<Value> {
        let action = input
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::MissingArgument("action".into()))?;

        let path_str = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::MissingArgument("path".into()))?;

        let resolved = self.sandbox.resolve(path_str)?;

        match action {
            "read" => self.op_read(path_str, &resolved),
            "list" => self.op_list(path_str, &resolved),
            "exists" => self.op_exists(path_str, &resolved),
            "write" => self.op_write(path_str, &resolved, input),
            "delete" => self.op_delete(path_str, &resolved),
            other => Err(ToolError::InvalidInput(format!(
                "unknown action: {}",
                other
            ))),
        }
    }

    fn op_read(&self, path_str: &str, resolved: &std::path::Path) -> ToolResult<Value> {
        if !resolved.exists() {
            return Err(ToolError::PathNotFound(path_str.to_string()));
        }
        if resolved.is_dir() {
            return Err(ToolError::NotAFile(path_str.to_string()));
        }
        let content = fs::read_to_string(resolved)?;
        Ok(json!({
            "action": "read",
            "path": path_str,
            "content": content,
            "bytes": content.len(),
        }))
    }

    fn op_list(&self, path_str: &str, resolved: &std::path::Path) -> ToolResult<Value> {
        if !resolved.exists() {
            return Err(ToolError::PathNotFound(path_str.to_string()));
        }
        if !resolved.is_dir() {
            return Err(ToolError::NotADirectory(path_str.to_string()));
        }

        let mut entries: Vec<Value> = Vec::new();
        for entry in fs::read_dir(resolved)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            let name = entry.file_name().to_string_lossy().to_string();
            entries.push(json!({
                "name": name,
                "is_dir": meta.is_dir(),
                "is_file": meta.is_file(),
                "size": meta.len(),
            }));
        }

        entries.sort_by(|a, b| {
            a["name"]
                .as_str()
                .unwrap_or("")
                .cmp(b["name"].as_str().unwrap_or(""))
        });

        Ok(json!({
            "action": "list",
            "path": path_str,
            "entries": entries,
            "count": entries.len(),
        }))
    }

    fn op_exists(&self, path_str: &str, resolved: &std::path::Path) -> ToolResult<Value> {
        Ok(json!({
            "action": "exists",
            "path": path_str,
            "exists": resolved.exists(),
        }))
    }

    fn op_write(
        &self,
        path_str: &str,
        resolved: &std::path::Path,
        input: &Value,
    ) -> ToolResult<Value> {
        let content = input
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::MissingArgument("content".into()))?;

        if resolved.is_dir() {
            return Err(ToolError::NotAFile(path_str.to_string()));
        }

        // Crea directorios intermedios si no existen.
        if let Some(parent) = resolved.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        fs::write(resolved, content)?;
        Ok(json!({
            "action": "write",
            "path": path_str,
            "bytes_written": content.len(),
        }))
    }

    fn op_delete(&self, path_str: &str, resolved: &std::path::Path) -> ToolResult<Value> {
        if !resolved.exists() {
            return Err(ToolError::PathNotFound(path_str.to_string()));
        }
        if resolved.is_dir() {
            return Err(ToolError::InvalidInput(
                "delete only supports files, not directories".into(),
            ));
        }
        fs::remove_file(resolved)?;
        Ok(json!({
            "action": "delete",
            "path": path_str,
            "deleted": true,
        }))
    }
}

impl Tool for FilesystemTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn invoke(&self, input: Value) -> ToolResult<Value> {
        // Valida que `action` y `path` están presentes antes de nada.
        if input.get("action").is_none() {
            return Err(ToolError::MissingArgument("action".into()));
        }
        if input.get("path").is_none() {
            return Err(ToolError::MissingArgument("path".into()));
        }
        // Nota: el riesgo por acción se consulta con
        // `FilesystemTool::risk_for_action` desde fuera (por el motor
        // de permisos en Fase 18).
        self.dispatch(&input)
    }
}

impl FilesystemTool {
    /// Riesgo de una acción concreta. Se usa desde el motor de permisos.
    pub fn action_risk(action: &str) -> RiskLevel {
        Self::risk_for_action(action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup() -> (TempDir, FilesystemTool) {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("hello.txt"), "hello world").unwrap();
        fs::create_dir_all(dir.path().join("subdir")).unwrap();
        fs::write(dir.path().join("subdir/nested.txt"), "nested content").unwrap();

        let sandbox = Sandbox::new(dir.path()).unwrap();
        let tool = FilesystemTool::new(sandbox);
        (dir, tool)
    }

    // ──────────────────────────────────────────────
    // Metadatos
    // ──────────────────────────────────────────────

    #[test]
    fn metadata_is_correct() {
        let (_dir, tool) = setup();
        let m = tool.metadata();
        assert_eq!(m.name, "filesystem");
        assert_eq!(m.category, ToolCategory::Filesystem);
    }

    #[test]
    fn risk_for_action_works() {
        assert_eq!(FilesystemTool::action_risk("read"), RiskLevel::Low);
        assert_eq!(FilesystemTool::action_risk("list"), RiskLevel::Low);
        assert_eq!(FilesystemTool::action_risk("exists"), RiskLevel::Low);
        assert_eq!(FilesystemTool::action_risk("write"), RiskLevel::Medium);
        assert_eq!(FilesystemTool::action_risk("delete"), RiskLevel::High);
    }

    // ──────────────────────────────────────────────
    // Validación de input
    // ──────────────────────────────────────────────

    #[test]
    fn missing_action_fails() {
        let (_dir, tool) = setup();
        let err = tool.invoke(json!({"path": "hello.txt"})).unwrap_err();
        assert!(matches!(err, ToolError::MissingArgument(_)));
    }

    #[test]
    fn missing_path_fails() {
        let (_dir, tool) = setup();
        let err = tool.invoke(json!({"action": "read"})).unwrap_err();
        assert!(matches!(err, ToolError::MissingArgument(_)));
    }

    #[test]
    fn unknown_action_fails() {
        let (_dir, tool) = setup();
        let err = tool
            .invoke(json!({"action": "yolo", "path": "hello.txt"}))
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    // ──────────────────────────────────────────────
    // read
    // ──────────────────────────────────────────────

    #[test]
    fn read_existing_file() {
        let (_dir, tool) = setup();
        let out = tool
            .invoke(json!({"action": "read", "path": "hello.txt"}))
            .unwrap();
        assert_eq!(out["action"], "read");
        assert_eq!(out["content"], "hello world");
        assert_eq!(out["bytes"], 11);
    }

    #[test]
    fn read_nested_file() {
        let (_dir, tool) = setup();
        let out = tool
            .invoke(json!({"action": "read", "path": "subdir/nested.txt"}))
            .unwrap();
        assert_eq!(out["content"], "nested content");
    }

    #[test]
    fn read_unknown_file_fails() {
        let (_dir, tool) = setup();
        let err = tool
            .invoke(json!({"action": "read", "path": "nonexistent.txt"}))
            .unwrap_err();
        assert!(matches!(err, ToolError::PathNotFound(_)));
    }

    #[test]
    fn read_directory_fails() {
        let (_dir, tool) = setup();
        let err = tool
            .invoke(json!({"action": "read", "path": "subdir"}))
            .unwrap_err();
        assert!(matches!(err, ToolError::NotAFile(_)));
    }

    // ──────────────────────────────────────────────
    // list
    // ──────────────────────────────────────────────

    #[test]
    fn list_directory() {
        let (_dir, tool) = setup();
        let out = tool.invoke(json!({"action": "list", "path": "."})).unwrap();
        assert_eq!(out["action"], "list");
        let entries = out["entries"].as_array().unwrap();
        assert!(entries.len() >= 2); // hello.txt + subdir
    }

    #[test]
    fn list_unknown_directory_fails() {
        let (_dir, tool) = setup();
        let err = tool
            .invoke(json!({"action": "list", "path": "nonexistent"}))
            .unwrap_err();
        assert!(matches!(err, ToolError::PathNotFound(_)));
    }

    #[test]
    fn list_file_fails() {
        let (_dir, tool) = setup();
        let err = tool
            .invoke(json!({"action": "list", "path": "hello.txt"}))
            .unwrap_err();
        assert!(matches!(err, ToolError::NotADirectory(_)));
    }

    // ──────────────────────────────────────────────
    // exists
    // ──────────────────────────────────────────────

    #[test]
    fn exists_returns_true_for_existing() {
        let (_dir, tool) = setup();
        let out = tool
            .invoke(json!({"action": "exists", "path": "hello.txt"}))
            .unwrap();
        assert_eq!(out["exists"], true);
    }

    #[test]
    fn exists_returns_false_for_missing() {
        let (_dir, tool) = setup();
        let out = tool
            .invoke(json!({"action": "exists", "path": "nope.txt"}))
            .unwrap();
        assert_eq!(out["exists"], false);
    }

    // ──────────────────────────────────────────────
    // write
    // ──────────────────────────────────────────────

    #[test]
    fn write_new_file() {
        let (dir, tool) = setup();
        let out = tool
            .invoke(json!({
                "action": "write",
                "path": "new.txt",
                "content": "hello new"
            }))
            .unwrap();
        assert_eq!(out["bytes_written"], 9);
        assert!(dir.path().join("new.txt").exists());
        assert_eq!(
            fs::read_to_string(dir.path().join("new.txt")).unwrap(),
            "hello new"
        );
    }

    #[test]
    fn write_creates_intermediate_dirs() {
        let (dir, tool) = setup();
        tool.invoke(json!({
            "action": "write",
            "path": "a/b/c/deep.txt",
            "content": "deep"
        }))
        .unwrap();
        assert!(dir.path().join("a/b/c/deep.txt").exists());
    }

    #[test]
    fn write_without_content_fails() {
        let (_dir, tool) = setup();
        let err = tool
            .invoke(json!({"action": "write", "path": "x.txt"}))
            .unwrap_err();
        assert!(matches!(err, ToolError::MissingArgument(_)));
    }

    #[test]
    fn write_overwrites_existing() {
        let (dir, tool) = setup();
        tool.invoke(json!({
            "action": "write",
            "path": "hello.txt",
            "content": "overwritten"
        }))
        .unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("hello.txt")).unwrap(),
            "overwritten"
        );
    }

    // ──────────────────────────────────────────────
    // delete
    // ──────────────────────────────────────────────

    #[test]
    fn delete_existing_file() {
        let (dir, tool) = setup();
        let out = tool
            .invoke(json!({"action": "delete", "path": "hello.txt"}))
            .unwrap();
        assert_eq!(out["deleted"], true);
        assert!(!dir.path().join("hello.txt").exists());
    }

    #[test]
    fn delete_unknown_fails() {
        let (_dir, tool) = setup();
        let err = tool
            .invoke(json!({"action": "delete", "path": "nope.txt"}))
            .unwrap_err();
        assert!(matches!(err, ToolError::PathNotFound(_)));
    }

    #[test]
    fn delete_directory_fails() {
        let (_dir, tool) = setup();
        let err = tool
            .invoke(json!({"action": "delete", "path": "subdir"}))
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    // ──────────────────────────────────────────────
    // Sandbox escapes
    // ──────────────────────────────────────────────

    #[test]
    fn read_blocks_parent_traversal() {
        let (_dir, tool) = setup();
        let err = tool
            .invoke(json!({"action": "read", "path": "../etc/passwd"}))
            .unwrap_err();
        assert!(matches!(err, ToolError::PathEscapesSandbox(_)));
    }

    #[test]
    fn write_blocks_parent_traversal() {
        let (_dir, tool) = setup();
        let err = tool
            .invoke(json!({
                "action": "write",
                "path": "../escaped.txt",
                "content": "hacked"
            }))
            .unwrap_err();
        assert!(matches!(err, ToolError::PathEscapesSandbox(_)));
    }

    #[test]
    fn delete_blocks_parent_traversal() {
        let (_dir, tool) = setup();
        let err = tool
            .invoke(json!({"action": "delete", "path": "../../etc/passwd"}))
            .unwrap_err();
        assert!(matches!(err, ToolError::PathEscapesSandbox(_)));
    }

    #[test]
    fn absolute_path_is_treated_as_relative() {
        let (_dir, tool) = setup();
        // `/hello.txt` debería resolver a `root/hello.txt`.
        let out = tool
            .invoke(json!({"action": "read", "path": "/hello.txt"}))
            .unwrap();
        assert_eq!(out["content"], "hello world");
    }
}
