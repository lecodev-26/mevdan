//! Seguridad y permisos para servidores MCP.
//!
//! Un servidor MCP puede exponer tools peligrosas. Este módulo define
//! cómo se decide si una invocación está permitida.
//!
//! ## Modelo
//!
//! Cada servidor tiene un **nivel de confianza** y una **política**.
//! El `McpSecurityGuard` evalúa cada invocación contra la política del
//! servidor y produce un `Permission` (`Allow`/`Ask`/`Deny`).
//!
//! ## Regla por defecto
//!
//! **Todo denegado hasta que el usuario lo apruebe.** Un servidor sin
//! política explícita se considera `Unknown` y todo se deniega.
//!
//! ## Integración
//!
//! Reutilizamos `mevdan_permissions::Permission` como tipo común para
//! decisiones de permiso, para no inventar otro enum.

use crate::error::{McpError, McpResult};
use mevdan_permissions::Permission;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Nivel de confianza de un servidor MCP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTrustLevel {
    /// El usuario ha aprobado explícitamente este servidor.
    Trusted,
    /// El servidor es desconocido. Todo se deniega hasta que se
    /// apruebe explícitamente.
    Unknown,
    /// El servidor está marcado como no fiable. Todo se deniega.
    Untrusted,
}

impl McpTrustLevel {
    pub fn display_name(&self) -> &'static str {
        match self {
            McpTrustLevel::Trusted => "trusted",
            McpTrustLevel::Unknown => "unknown",
            McpTrustLevel::Untrusted => "untrusted",
        }
    }

    /// ¿Permite ejecutar tools?
    pub fn allows_execution(&self) -> bool {
        matches!(self, McpTrustLevel::Trusted)
    }
}

/// Política de una tool MCP concreta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolPolicy {
    /// Nombre de la tool (ej. `"read_file"`).
    pub tool_name: String,
    /// Permiso asociado.
    pub permission: Permission,
    /// Razón legible (para logs/auditoría).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl McpToolPolicy {
    pub fn new(tool_name: impl Into<String>, permission: Permission) -> Self {
        Self {
            tool_name: tool_name.into(),
            permission,
            reason: None,
        }
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// Política de un servidor MCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPolicy {
    /// Nombre del servidor.
    pub server_name: String,
    /// Nivel de confianza.
    pub trust: McpTrustLevel,
    /// Permiso por defecto para tools sin política específica.
    pub default_permission: Permission,
    /// Políticas específicas por tool.
    #[serde(default)]
    pub tool_policies: BTreeMap<String, McpToolPolicy>,
}

impl McpPolicy {
    /// Política por defecto para un servidor desconocido.
    ///
    /// Todo denegado.
    pub fn unknown(server_name: impl Into<String>) -> Self {
        Self {
            server_name: server_name.into(),
            trust: McpTrustLevel::Unknown,
            default_permission: Permission::Deny,
            tool_policies: BTreeMap::new(),
        }
    }

    /// Política para un servidor marcado como no fiable.
    pub fn untrusted(server_name: impl Into<String>) -> Self {
        Self {
            server_name: server_name.into(),
            trust: McpTrustLevel::Untrusted,
            default_permission: Permission::Deny,
            tool_policies: BTreeMap::new(),
        }
    }

    /// Política para un servidor de confianza pero con tools a `Ask`
    /// por defecto. Es la política recomendada cuando el usuario
    /// aprueba un servidor.
    pub fn trusted_ask(server_name: impl Into<String>) -> Self {
        Self {
            server_name: server_name.into(),
            trust: McpTrustLevel::Trusted,
            default_permission: Permission::Ask,
            tool_policies: BTreeMap::new(),
        }
    }

    /// Política para un servidor de confianza con tools a `Allow`
    /// por defecto.
    ///
    /// **Cuidado:** úsala solo si confías plenamente en el servidor.
    pub fn trusted_allow(server_name: impl Into<String>) -> Self {
        Self {
            server_name: server_name.into(),
            trust: McpTrustLevel::Trusted,
            default_permission: Permission::Allow,
            tool_policies: BTreeMap::new(),
        }
    }

    /// Añade una política específica para una tool.
    pub fn add_tool_policy(mut self, policy: McpToolPolicy) -> Self {
        self.tool_policies.insert(policy.tool_name.clone(), policy);
        self
    }

    /// Permiso efectivo para una tool concreta.
    pub fn permission_for(&self, tool_name: &str) -> Permission {
        // Si el servidor no es de confianza, DENY siempre.
        if !self.trust.allows_execution() {
            return Permission::Deny;
        }
        // Si hay política específica para la tool, la usamos.
        if let Some(p) = self.tool_policies.get(tool_name) {
            return p.permission;
        }
        // Si no, el default de la política.
        self.default_permission
    }

    /// Razón del permiso (para logs).
    pub fn reason_for(&self, tool_name: &str) -> String {
        if !self.trust.allows_execution() {
            return format!(
                "server '{}' has trust level '{}'",
                self.server_name,
                self.trust.display_name()
            );
        }
        if let Some(p) = self.tool_policies.get(tool_name) {
            return p
                .reason
                .clone()
                .unwrap_or_else(|| format!("tool policy for '{}'", tool_name));
        }
        format!("default policy for server '{}'", self.server_name)
    }
}

/// Decisión del guard para una invocación MCP.
#[derive(Debug, Clone)]
pub struct McpSecurityDecision {
    pub server_name: String,
    pub tool_name: String,
    pub permission: Permission,
    pub reason: String,
}

impl McpSecurityDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self.permission, Permission::Allow)
    }

    pub fn needs_user_input(&self) -> bool {
        matches!(self.permission, Permission::Ask)
    }

    pub fn is_denied(&self) -> bool {
        matches!(self.permission, Permission::Deny)
    }
}

/// Guard de seguridad para invocaciones MCP.
///
/// Mantiene las políticas por servidor y evalúa cada invocación.
#[derive(Debug, Default)]
pub struct McpSecurityGuard {
    policies: BTreeMap<String, McpPolicy>,
}

impl McpSecurityGuard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra una política.
    pub fn register_policy(&mut self, policy: McpPolicy) {
        self.policies.insert(policy.server_name.clone(), policy);
    }

    /// Obtiene la política de un servidor.
    pub fn policy(&self, server_name: &str) -> Option<&McpPolicy> {
        self.policies.get(server_name)
    }

    /// Evalúa si una invocación está permitida.
    ///
    /// Si no hay política registrada para el servidor, se usa una
    /// política `Unknown` (todo denegado).
    pub fn check(&self, server_name: &str, tool_name: &str) -> McpResult<McpSecurityDecision> {
        let default_policy;
        let policy = match self.policies.get(server_name) {
            Some(p) => p,
            None => {
                default_policy = McpPolicy::unknown(server_name);
                &default_policy
            }
        };

        let permission = policy.permission_for(tool_name);
        let reason = policy.reason_for(tool_name);

        Ok(McpSecurityDecision {
            server_name: server_name.to_string(),
            tool_name: tool_name.to_string(),
            permission,
            reason,
        })
    }

    /// Igual que `check` pero devuelve error si está denegado.
    ///
    /// Útil para forzar la decisión en un punto de invocación.
    pub fn require(&self, server_name: &str, tool_name: &str) -> McpResult<McpSecurityDecision> {
        let decision = self.check(server_name, tool_name)?;
        if decision.is_denied() {
            return Err(McpError::Protocol(format!(
                "MCP invocation denied: {} (server: {}, tool: {})",
                decision.reason, server_name, tool_name
            )));
        }
        Ok(decision)
    }

    /// Número de políticas registradas.
    pub fn len(&self) -> usize {
        self.policies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.policies.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_level_display_names() {
        assert_eq!(McpTrustLevel::Trusted.display_name(), "trusted");
        assert_eq!(McpTrustLevel::Unknown.display_name(), "unknown");
        assert_eq!(McpTrustLevel::Untrusted.display_name(), "untrusted");
    }

    #[test]
    fn trust_level_allows_execution() {
        assert!(McpTrustLevel::Trusted.allows_execution());
        assert!(!McpTrustLevel::Unknown.allows_execution());
        assert!(!McpTrustLevel::Untrusted.allows_execution());
    }

    #[test]
    fn trust_level_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&McpTrustLevel::Trusted).unwrap(),
            "\"trusted\""
        );
        assert_eq!(
            serde_json::to_string(&McpTrustLevel::Unknown).unwrap(),
            "\"unknown\""
        );
    }

    #[test]
    fn tool_policy_new_and_with_reason() {
        let p = McpToolPolicy::new("read_file", Permission::Ask).with_reason("needs review");
        assert_eq!(p.tool_name, "read_file");
        assert_eq!(p.permission, Permission::Ask);
        assert_eq!(p.reason.as_deref(), Some("needs review"));
    }

    #[test]
    fn policy_unknown_denies_everything() {
        let p = McpPolicy::unknown("test");
        assert_eq!(p.permission_for("anything"), Permission::Deny);
        assert_eq!(p.trust, McpTrustLevel::Unknown);
    }

    #[test]
    fn policy_untrusted_denies_everything() {
        let p = McpPolicy::untrusted("test");
        assert_eq!(p.permission_for("anything"), Permission::Deny);
    }

    #[test]
    fn policy_trusted_ask_returns_ask_by_default() {
        let p = McpPolicy::trusted_ask("test");
        assert_eq!(p.permission_for("any_tool"), Permission::Ask);
    }

    #[test]
    fn policy_trusted_allow_returns_allow_by_default() {
        let p = McpPolicy::trusted_allow("test");
        assert_eq!(p.permission_for("any_tool"), Permission::Allow);
    }

    #[test]
    fn specific_tool_policy_overrides_default() {
        let p = McpPolicy::trusted_ask("test")
            .add_tool_policy(McpToolPolicy::new("delete_file", Permission::Deny));
        assert_eq!(p.permission_for("delete_file"), Permission::Deny);
        assert_eq!(p.permission_for("read_file"), Permission::Ask);
    }

    #[test]
    fn untrusted_ignores_specific_tool_policy() {
        let p = McpPolicy::untrusted("test")
            .add_tool_policy(McpToolPolicy::new("read_file", Permission::Allow));
        // Aunque haya política específica, el servidor no es de
        // confianza → DENY.
        assert_eq!(p.permission_for("read_file"), Permission::Deny);
    }

    #[test]
    fn reason_for_unknown_server() {
        let p = McpPolicy::unknown("test");
        let reason = p.reason_for("any");
        assert!(reason.contains("unknown"));
    }

    #[test]
    fn reason_for_specific_tool() {
        let p = McpPolicy::trusted_ask("test").add_tool_policy(
            McpToolPolicy::new("dangerous", Permission::Deny).with_reason("too risky"),
        );
        assert_eq!(p.reason_for("dangerous"), "too risky");
    }

    #[test]
    fn reason_for_default_policy() {
        let p = McpPolicy::trusted_ask("test");
        let reason = p.reason_for("any");
        assert!(reason.contains("default policy"));
        assert!(reason.contains("test"));
    }

    #[test]
    fn guard_empty_denies_unknown_server() {
        let guard = McpSecurityGuard::new();
        let decision = guard.check("unknown-server", "any_tool").unwrap();
        assert!(decision.is_denied());
        assert!(decision.reason.contains("unknown"));
    }

    #[test]
    fn guard_with_trusted_policy() {
        let mut guard = McpSecurityGuard::new();
        guard.register_policy(McpPolicy::trusted_ask("trusted-server"));

        let decision = guard.check("trusted-server", "any_tool").unwrap();
        assert!(decision.needs_user_input());
    }

    #[test]
    fn guard_with_deny_policy() {
        let mut guard = McpSecurityGuard::new();
        guard.register_policy(McpPolicy::untrusted("bad-server"));

        let decision = guard.check("bad-server", "any_tool").unwrap();
        assert!(decision.is_denied());
    }

    #[test]
    fn guard_require_fails_on_deny() {
        let mut guard = McpSecurityGuard::new();
        guard.register_policy(McpPolicy::untrusted("bad-server"));

        let err = guard.require("bad-server", "any").unwrap_err();
        assert!(matches!(err, McpError::Protocol(_)));
    }

    #[test]
    fn guard_require_passes_on_ask() {
        let mut guard = McpSecurityGuard::new();
        guard.register_policy(McpPolicy::trusted_ask("ask-server"));

        let decision = guard.require("ask-server", "any").unwrap();
        assert!(decision.needs_user_input());
    }

    #[test]
    fn guard_require_passes_on_allow() {
        let mut guard = McpSecurityGuard::new();
        guard.register_policy(McpPolicy::trusted_allow("good-server"));

        let decision = guard.require("good-server", "any").unwrap();
        assert!(decision.is_allowed());
    }

    #[test]
    fn guard_len_and_is_empty() {
        let mut guard = McpSecurityGuard::new();
        assert!(guard.is_empty());

        guard.register_policy(McpPolicy::unknown("s1"));
        assert_eq!(guard.len(), 1);
        assert!(!guard.is_empty());

        guard.register_policy(McpPolicy::unknown("s2"));
        assert_eq!(guard.len(), 2);
    }

    #[test]
    fn guard_policy_lookup() {
        let mut guard = McpSecurityGuard::new();
        guard.register_policy(McpPolicy::trusted_allow("s1"));
        assert!(guard.policy("s1").is_some());
        assert!(guard.policy("s2").is_none());
    }

    #[test]
    fn realistic_scenario_filesystem_server() {
        // Servidor de filesystem: confiable, pero con tools concretas
        // restringidas.
        let mut guard = McpSecurityGuard::new();
        let policy = McpPolicy::trusted_ask("filesystem")
            .add_tool_policy(
                McpToolPolicy::new("read_file", Permission::Allow).with_reason("safe read"),
            )
            .add_tool_policy(
                McpToolPolicy::new("delete_file", Permission::Deny).with_reason("destructive"),
            );

        guard.register_policy(policy);

        // read_file: ALLOW.
        let d = guard.check("filesystem", "read_file").unwrap();
        assert!(d.is_allowed());
        assert_eq!(d.reason, "safe read");

        // write_file: ASK (default).
        let d = guard.check("filesystem", "write_file").unwrap();
        assert!(d.needs_user_input());

        // delete_file: DENY.
        let d = guard.check("filesystem", "delete_file").unwrap();
        assert!(d.is_denied());
        assert_eq!(d.reason, "destructive");
    }
}
