//! Resultado de evaluar una invocación contra la política.
//!
//! Una `Decision` no es lo mismo que un `Permission`:
//! - `Permission` es lo que una **regla** declara.
//! - `Decision` es lo que el **motor** concluye tras evaluar todas
//!   las reglas. Incluye la razón (qué regla matcheó, o si se aplicó
//!   el default).

use crate::{permission::Permission, rule::Rule};
use serde::{Deserialize, Serialize};

/// Tipo de decisión.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionKind {
    /// Permitido. Se puede ejecutar sin preguntar.
    Allowed,
    /// Requiere confirmación del usuario.
    NeedsUserInput,
    /// Denegado.
    Denied,
}

impl DecisionKind {
    /// Nombre legible.
    pub fn display_name(&self) -> &'static str {
        match self {
            DecisionKind::Allowed => "ALLOWED",
            DecisionKind::NeedsUserInput => "ASK",
            DecisionKind::Denied => "DENIED",
        }
    }

    /// Convierte desde `Permission`.
    pub fn from_permission(p: Permission) -> Self {
        match p {
            Permission::Allow => DecisionKind::Allowed,
            Permission::Ask => DecisionKind::NeedsUserInput,
            Permission::Deny => DecisionKind::Denied,
        }
    }
}

/// Origen de la decisión.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum DecisionSource {
    /// Una regla específica matcheó.
    Rule {
        /// Índice de la regla en la política.
        index: usize,
        /// Etiqueta legible.
        label: String,
    },
    /// Ninguna regla matcheó; se aplicó el default.
    Default,
}

/// Decisión del motor de permisos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    /// Tipo de decisión.
    pub kind: DecisionKind,

    /// Permiso original.
    pub permission: Permission,

    /// Origen (regla o default).
    pub source: DecisionSource,

    /// Razón legible.
    pub reason: String,
}

impl Decision {
    /// Decisión de la regla.
    pub fn from_rule(rule: &Rule, index: usize) -> Self {
        let reason = rule
            .reason
            .clone()
            .unwrap_or_else(|| format!("matched rule #{}", index));

        Self {
            kind: DecisionKind::from_permission(rule.permission),
            permission: rule.permission,
            source: DecisionSource::Rule {
                index,
                label: rule.label(),
            },
            reason,
        }
    }

    /// Decisión por default.
    pub fn from_default(permission: Permission) -> Self {
        Self {
            kind: DecisionKind::from_permission(permission),
            permission,
            source: DecisionSource::Default,
            reason: format!("no rule matched; default is {}", permission.display_name()),
        }
    }

    /// ¿Permite la ejecución?
    pub fn is_allowed(&self) -> bool {
        matches!(self.kind, DecisionKind::Allowed)
    }

    /// ¿Requiere input del usuario?
    pub fn needs_user_input(&self) -> bool {
        matches!(self.kind, DecisionKind::NeedsUserInput)
    }

    /// ¿Deniega la ejecución?
    pub fn is_denied(&self) -> bool {
        matches!(self.kind, DecisionKind::Denied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        permission::{Permission, Scope},
        rule::Rule,
    };

    #[test]
    fn decision_kind_display_names() {
        assert_eq!(DecisionKind::Allowed.display_name(), "ALLOWED");
        assert_eq!(DecisionKind::NeedsUserInput.display_name(), "ASK");
        assert_eq!(DecisionKind::Denied.display_name(), "DENIED");
    }

    #[test]
    fn decision_kind_from_permission() {
        assert_eq!(
            DecisionKind::from_permission(Permission::Allow),
            DecisionKind::Allowed
        );
        assert_eq!(
            DecisionKind::from_permission(Permission::Ask),
            DecisionKind::NeedsUserInput
        );
        assert_eq!(
            DecisionKind::from_permission(Permission::Deny),
            DecisionKind::Denied
        );
    }

    #[test]
    fn from_rule_allow() {
        let rule = Rule::new("filesystem", Scope::Any, Permission::Allow).unwrap();
        let d = Decision::from_rule(&rule, 0);
        assert!(d.is_allowed());
        assert_eq!(d.permission, Permission::Allow);
        assert!(matches!(d.source, DecisionSource::Rule { index: 0, .. }));
    }

    #[test]
    fn from_rule_with_custom_reason() {
        let rule = Rule::new("filesystem", Scope::Any, Permission::Deny)
            .unwrap()
            .with_reason("no writes");
        let d = Decision::from_rule(&rule, 5);
        assert!(d.is_denied());
        assert_eq!(d.reason, "no writes");
    }

    #[test]
    fn from_default_deny() {
        let d = Decision::from_default(Permission::Deny);
        assert!(d.is_denied());
        assert!(matches!(d.source, DecisionSource::Default));
        assert!(d.reason.contains("no rule matched"));
    }

    #[test]
    fn predicates_are_mutually_exclusive() {
        let allow = Decision::from_default(Permission::Allow);
        assert!(allow.is_allowed());
        assert!(!allow.needs_user_input());
        assert!(!allow.is_denied());

        let ask = Decision::from_default(Permission::Ask);
        assert!(!ask.is_allowed());
        assert!(ask.needs_user_input());
        assert!(!ask.is_denied());

        let deny = Decision::from_default(Permission::Deny);
        assert!(!deny.is_allowed());
        assert!(!deny.needs_user_input());
        assert!(deny.is_denied());
    }

    #[test]
    fn decision_roundtrips() {
        let rule = Rule::new("shell", Scope::Any, Permission::Ask).unwrap();
        let d = Decision::from_rule(&rule, 3);
        let json = serde_json::to_string(&d).unwrap();
        let back: Decision = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind, d.kind);
        assert_eq!(back.permission, d.permission);
    }
}
