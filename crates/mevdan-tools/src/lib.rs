//! # mevdan-tools
//!
//! Herramientas que el agente puede invocar.
//!
//! ## Estado del proyecto
//!
//! - **15** ✅ — `FilesystemTool` + sandbox.
//! - **16** ✅ — `ShellTool` con lista blanca.
//! - **17** ✅ — `GitTool` con operaciones validadas.
//! - **18** ⏳ — Permission engine.
//!
//! ## Reglas
//!
//! 1. **Sin I/O directo.** Cada tool gestiona su propio sandbox.
//! 2. **Errores tipados.** `ToolError` cubre los casos comunes.
//! 3. **Metadatos explícitos.** Cada tool declara nombre, categoría,
//!    nivel de riesgo, y schemas JSON.
//! 4. **Sin permisos todavía.** Llegan en Fase 18.

pub mod error;
pub mod filesystem;
pub mod git;
pub mod registry;
pub mod shell;
pub mod tool;

// Re-exports de conveniencia.
pub use error::{ToolError, ToolResult};
pub use filesystem::{FilesystemTool, Sandbox};
pub use git::{GitResult, GitTool};
pub use registry::ToolRegistry;
pub use shell::{AllowList, ShellResult, ShellTool};
pub use tool::{RiskLevel, Tool, ToolCategory, ToolMetadata};

#[cfg(test)]
mod tests {
    use super::*;

    /// Tool mínimo para tests de integración.
    #[derive(Debug)]
    struct StaticTool {
        metadata: ToolMetadata,
    }

    impl StaticTool {
        fn new(name: &str, risk: RiskLevel) -> Self {
            Self {
                metadata: ToolMetadata {
                    name: name.into(),
                    description: format!("test tool {}", name),
                    category: ToolCategory::Other,
                    risk,
                    input_schema: serde_json::json!({}),
                    output_schema: serde_json::json!({}),
                },
            }
        }
    }

    impl Tool for StaticTool {
        fn metadata(&self) -> &ToolMetadata {
            &self.metadata
        }

        fn invoke(&self, input: serde_json::Value) -> ToolResult<serde_json::Value> {
            Ok(serde_json::json!({
                "received": input,
                "tool": self.metadata.name,
            }))
        }
    }

    fn init_git_repo(dir: &std::path::Path) {
        use std::process::Command;
        Command::new("git")
            .args(["init", "-q", "-b", "main"])
            .current_dir(dir)
            .output()
            .unwrap();
    }

    #[test]
    fn full_flow_register_and_invoke() {
        let mut registry = ToolRegistry::new();
        registry
            .register(Box::new(StaticTool::new("low_tool", RiskLevel::Low)))
            .unwrap();
        registry
            .register(Box::new(StaticTool::new("high_tool", RiskLevel::High)))
            .unwrap();

        assert_eq!(registry.len(), 2);

        let out = registry
            .invoke("low_tool", serde_json::json!({"x": 1}))
            .unwrap();
        assert_eq!(out["tool"], "low_tool");
        assert_eq!(out["received"]["x"], 1);
    }

    #[test]
    fn three_critical_tools_coexist() {
        use std::fs;
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        init_git_repo(dir.path());
        fs::write(dir.path().join("file.txt"), "content").unwrap();

        let sandbox = Sandbox::new(dir.path()).unwrap();
        let fs_tool = FilesystemTool::new(sandbox);
        let shell_tool = ShellTool::new(dir.path()).unwrap();
        let git_tool = GitTool::new(dir.path()).unwrap();

        let mut registry = ToolRegistry::new();
        registry.register(Box::new(fs_tool)).unwrap();
        registry.register(Box::new(shell_tool)).unwrap();
        registry.register(Box::new(git_tool)).unwrap();

        assert_eq!(registry.len(), 3);
        let names = registry.list_names();
        assert_eq!(names, vec!["filesystem", "git", "shell"]);
    }

    #[test]
    fn filesystem_shell_git_workflow() {
        use std::fs;
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        init_git_repo(dir.path());

        let sandbox = Sandbox::new(dir.path()).unwrap();
        let mut registry = ToolRegistry::new();
        registry
            .register(Box::new(FilesystemTool::new(sandbox)))
            .unwrap();
        registry
            .register(Box::new(ShellTool::new(dir.path()).unwrap()))
            .unwrap();
        registry
            .register(Box::new(GitTool::new(dir.path()).unwrap()))
            .unwrap();

        // 1. Escribir un archivo con el tool de filesystem.
        registry
            .invoke(
                "filesystem",
                serde_json::json!({
                    "action": "write",
                    "path": "hello.txt",
                    "content": "hello world"
                }),
            )
            .unwrap();

        // 2. Verificar con shell.
        let out = registry
            .invoke(
                "shell",
                serde_json::json!({"program": "cat", "args": ["hello.txt"]}),
            )
            .unwrap();
        assert_eq!(out["stdout"].as_str().unwrap().trim(), "hello world");

        // 3. Verificar con git status (archivo nuevo).
        let out = registry
            .invoke("git", serde_json::json!({"action": "status"}))
            .unwrap();
        assert_eq!(out["clean"], false);

        // 4. Confirmar que el archivo existe físicamente.
        assert!(fs::read_to_string(dir.path().join("hello.txt")).is_ok());
    }
}
