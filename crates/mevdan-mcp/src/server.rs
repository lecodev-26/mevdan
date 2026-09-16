//! `McpServer` — descriptor de un servidor MCP.

use crate::error::{McpError, McpResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Configuración de un servidor MCP.
///
/// Describe cómo lanzar y comunicarse con el servidor. Se guarda en
/// config del proyecto (`.mevdan/mcp.toml` o similar) en fases
/// posteriores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    /// Nombre único del servidor (ej. `"filesystem"`).
    pub name: String,

    /// Comando a ejecutar (ej. `"npx"`).
    pub command: String,

    /// Argumentos del comando.
    #[serde(default)]
    pub args: Vec<String>,

    /// Variables de entorno.
    #[serde(default)]
    pub env: BTreeMap<String, String>,

    /// Descripción opcional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl McpServer {
    /// Crea un descriptor de servidor MCP.
    pub fn new(name: impl Into<String>, command: impl Into<String>) -> McpResult<Self> {
        let name = name.into();
        validate_server_name(&name)?;
        Ok(Self {
            name,
            command: command.into(),
            args: Vec::new(),
            env: BTreeMap::new(),
            description: None,
        })
    }

    /// Añade args.
    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args = args.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Añade env.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Añade descripción.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Resumen textual.
    pub fn summary(&self) -> String {
        let mut s = format!("{} ({}", self.name, self.command);
        for arg in &self.args {
            s.push(' ');
            s.push_str(arg);
        }
        s.push(')');
        s
    }
}

/// Valida un nombre de servidor.
///
/// Reglas: 1-64 chars, solo `a-z`, `0-9`, `-`, `_`.
pub fn validate_server_name(name: &str) -> McpResult<()> {
    if name.is_empty() || name.len() > 64 {
        return Err(McpError::InvalidServerName(name.to_string()));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        return Err(McpError::InvalidServerName(name.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_server() {
        let s = McpServer::new("filesystem", "npx").unwrap();
        assert_eq!(s.name, "filesystem");
        assert_eq!(s.command, "npx");
        assert!(s.args.is_empty());
        assert!(s.env.is_empty());
        assert!(s.description.is_none());
    }

    #[test]
    fn new_validates_name() {
        assert!(McpServer::new("", "cmd").is_err());
        assert!(McpServer::new("Bad Name", "cmd").is_err());
        assert!(McpServer::new("bad/name", "cmd").is_err());
        assert!(McpServer::new("valid-name", "cmd").is_ok());
        assert!(McpServer::new("valid_name_2", "cmd").is_ok());
    }

    #[test]
    fn with_args_sets_args() {
        let s = McpServer::new("test", "cmd")
            .unwrap()
            .with_args(["-y", "package@latest"]);
        assert_eq!(s.args, vec!["-y", "package@latest"]);
    }

    #[test]
    fn with_env_sets_env() {
        let s = McpServer::new("test", "cmd")
            .unwrap()
            .with_env("KEY", "value")
            .with_env("OTHER", "x");
        assert_eq!(s.env.get("KEY").map(|s| s.as_str()), Some("value"));
        assert_eq!(s.env.get("OTHER").map(|s| s.as_str()), Some("x"));
    }

    #[test]
    fn with_description_sets_it() {
        let s = McpServer::new("test", "cmd")
            .unwrap()
            .with_description("A test server");
        assert_eq!(s.description.as_deref(), Some("A test server"));
    }

    #[test]
    fn summary_format() {
        let s = McpServer::new("fs", "npx")
            .unwrap()
            .with_args(["-y", "@modelcontextprotocol/server-filesystem"]);
        assert!(s.summary().contains("fs"));
        assert!(s.summary().contains("npx"));
        assert!(s
            .summary()
            .contains("@modelcontextprotocol/server-filesystem"));
    }

    #[test]
    fn server_roundtrips() {
        let s = McpServer::new("filesystem", "npx")
            .unwrap()
            .with_args(["-y", "pkg"])
            .with_env("FOO", "bar")
            .with_description("FS server");
        let json = serde_json::to_string(&s).unwrap();
        let back: McpServer = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, s.name);
        assert_eq!(back.args, s.args);
        assert_eq!(back.env, s.env);
        assert_eq!(back.description, s.description);
    }

    #[test]
    fn validate_server_name_rules() {
        assert!(validate_server_name("valid").is_ok());
        assert!(validate_server_name("valid-name_2").is_ok());
        assert!(validate_server_name("").is_err());
        assert!(validate_server_name("has space").is_err());
        assert!(validate_server_name(&"a".repeat(65)).is_err());
    }
}
