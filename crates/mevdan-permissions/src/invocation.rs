//! Contexto de una invocación a evaluar.
//!
//! El motor de permisos recibe un `Invocation` que describe **qué se
//! quiere hacer**: qué tool, con qué path, con qué comando, etc. Cada
//! scope de las reglas se compara contra este contexto.

use serde::{Deserialize, Serialize};

/// Contexto de una invocación.
///
/// No todos los campos aplican a todos los tools. Un tool de
/// filesystem usará `path`; un tool de shell usará `program` y `args`;
/// un tool de git usará `action` y quizá `path`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Invocation {
    /// Nombre del tool (ej. `"filesystem"`, `"shell"`).
    pub tool: String,

    /// Path involucrado (si aplica).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Programa invocado (si aplica, ej. `"cargo"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub program: Option<String>,

    /// Argumentos del programa (si aplica).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,

    /// Acción específica (ej. `"read"`, `"write"`, `"delete"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,

    /// Dominio de red (si aplica, Fase futura).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
}

impl Invocation {
    /// Constructor mínimo: solo el tool.
    pub fn tool(tool: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            ..Default::default()
        }
    }

    /// Constructor para filesystem.
    pub fn filesystem(action: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            tool: "filesystem".into(),
            path: Some(path.into()),
            action: Some(action.into()),
            ..Default::default()
        }
    }

    /// Constructor para shell.
    pub fn shell(program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            tool: "shell".into(),
            program: Some(program.into()),
            args,
            ..Default::default()
        }
    }

    /// Constructor para git.
    pub fn git(action: impl Into<String>) -> Self {
        Self {
            tool: "git".into(),
            action: Some(action.into()),
            ..Default::default()
        }
    }

    /// Añade path.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Añade acción.
    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_constructor() {
        let i = Invocation::tool("filesystem");
        assert_eq!(i.tool, "filesystem");
        assert!(i.path.is_none());
        assert!(i.program.is_none());
        assert!(i.action.is_none());
    }

    #[test]
    fn filesystem_constructor() {
        let i = Invocation::filesystem("read", "src/main.rs");
        assert_eq!(i.tool, "filesystem");
        assert_eq!(i.action.as_deref(), Some("read"));
        assert_eq!(i.path.as_deref(), Some("src/main.rs"));
    }

    #[test]
    fn shell_constructor() {
        let i = Invocation::shell("cargo", vec!["build".into(), "--release".into()]);
        assert_eq!(i.tool, "shell");
        assert_eq!(i.program.as_deref(), Some("cargo"));
        assert_eq!(i.args.len(), 2);
    }

    #[test]
    fn git_constructor() {
        let i = Invocation::git("commit");
        assert_eq!(i.tool, "git");
        assert_eq!(i.action.as_deref(), Some("commit"));
    }

    #[test]
    fn builder_methods() {
        let i = Invocation::tool("filesystem")
            .with_path("test.txt")
            .with_action("write");
        assert_eq!(i.path.as_deref(), Some("test.txt"));
        assert_eq!(i.action.as_deref(), Some("write"));
    }

    #[test]
    fn invocation_roundtrips() {
        let i = Invocation::filesystem("write", "new.txt");
        let json = serde_json::to_string(&i).unwrap();
        let back: Invocation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tool, i.tool);
        assert_eq!(back.path, i.path);
        assert_eq!(back.action, i.action);
    }
}
