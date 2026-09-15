//! Reglas individuales de permisos.
//!
//! Una regla combina:
//! - Un **tool** (o `"*"` para cualquiera).
//! - Un **scope** (a qué aplica).
//! - Un **permission** (Allow/Ask/Deny).
//! - Una **razón** (para logs).

use crate::{
    error::{PermissionError, PermissionResult},
    permission::{Permission, Scope},
};
use serde::{Deserialize, Serialize};

/// Comodín para "cualquier tool".
pub const ANY_TOOL: &str = "*";

/// Una regla de permisos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    /// Nombre del tool al que aplica. `"*"` para cualquiera.
    pub tool: String,

    /// Scope.
    pub scope: Scope,

    /// Permiso.
    pub permission: Permission,

    /// Razón legible para logs/UI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl Rule {
    /// Crea una regla.
    pub fn new(
        tool: impl Into<String>,
        scope: Scope,
        permission: Permission,
    ) -> PermissionResult<Self> {
        let tool = tool.into();
        validate_tool(&tool)?;
        Ok(Self {
            tool,
            scope,
            permission,
            reason: None,
        })
    }

    /// Añade razón.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// ¿Aplica a este tool?
    pub fn applies_to_tool(&self, tool_name: &str) -> bool {
        self.tool == ANY_TOOL || self.tool == tool_name
    }

    /// Etiqueta corta para logs.
    pub fn label(&self) -> String {
        format!(
            "{} {} → {}",
            self.tool,
            self.scope.label(),
            self.permission.display_name()
        )
    }
}

/// Valida un nombre de tool (o `"*"`).
fn validate_tool(tool: &str) -> PermissionResult<()> {
    if tool.is_empty() || tool.len() > 64 {
        return Err(PermissionError::InvalidRule(format!(
            "invalid tool name: {}",
            tool
        )));
    }
    if tool == ANY_TOOL {
        return Ok(());
    }
    if !tool
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        return Err(PermissionError::InvalidRule(format!(
            "invalid tool name: {}",
            tool
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_valid_rule() {
        let r = Rule::new("filesystem", Scope::Any, Permission::Allow).unwrap();
        assert_eq!(r.tool, "filesystem");
        assert_eq!(r.permission, Permission::Allow);
        assert!(r.reason.is_none());
    }

    #[test]
    fn new_with_wildcard_tool() {
        let r = Rule::new("*", Scope::Any, Permission::Deny).unwrap();
        assert_eq!(r.tool, "*");
    }

    #[test]
    fn new_fails_with_empty_tool() {
        let err = Rule::new("", Scope::Any, Permission::Allow).unwrap_err();
        assert!(matches!(err, PermissionError::InvalidRule(_)));
    }

    #[test]
    fn new_fails_with_uppercase_tool() {
        let err = Rule::new("Filesystem", Scope::Any, Permission::Allow).unwrap_err();
        assert!(matches!(err, PermissionError::InvalidRule(_)));
    }

    #[test]
    fn with_reason_adds_reason() {
        let r = Rule::new("filesystem", Scope::Any, Permission::Allow)
            .unwrap()
            .with_reason("read-only access");
        assert_eq!(r.reason.as_deref(), Some("read-only access"));
    }

    #[test]
    fn applies_to_tool_exact_match() {
        let r = Rule::new("filesystem", Scope::Any, Permission::Allow).unwrap();
        assert!(r.applies_to_tool("filesystem"));
        assert!(!r.applies_to_tool("shell"));
    }

    #[test]
    fn applies_to_tool_wildcard() {
        let r = Rule::new("*", Scope::Any, Permission::Deny).unwrap();
        assert!(r.applies_to_tool("filesystem"));
        assert!(r.applies_to_tool("shell"));
        assert!(r.applies_to_tool("anything"));
    }

    #[test]
    fn label_format() {
        let r = Rule::new("filesystem", Scope::glob("src/**"), Permission::Allow).unwrap();
        assert_eq!(r.label(), "filesystem path:src/** → ALLOW");
    }

    #[test]
    fn rule_roundtrips() {
        let r = Rule::new("shell", Scope::shell_program("cargo"), Permission::Ask)
            .unwrap()
            .with_reason("compile");
        let json = serde_json::to_string(&r).unwrap();
        let back: Rule = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
    }
}
