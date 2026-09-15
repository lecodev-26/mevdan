//! Nivel de permiso y scope.
//!
//! Un permiso puede ser:
//! - `Allow` — permitido sin preguntar.
//! - `Ask` — preguntar al usuario antes de ejecutar.
//! - `Deny` — rechazado.
//!
//! Un **scope** determina a qué aplica una regla: qué paths, qué
//! comandos, qué acciones.

use serde::{Deserialize, Serialize};

/// Nivel de permiso.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Permitido sin preguntar.
    Allow,
    /// Preguntar al usuario antes de ejecutar.
    Ask,
    /// Rechazado.
    Deny,
}

impl Permission {
    /// Nombre legible.
    pub fn display_name(&self) -> &'static str {
        match self {
            Permission::Allow => "ALLOW",
            Permission::Ask => "ASK",
            Permission::Deny => "DENY",
        }
    }

    /// ¿Es la opción más restrictiva?
    pub fn is_deny(&self) -> bool {
        matches!(self, Permission::Deny)
    }

    /// ¿Requiere interacción del usuario?
    pub fn needs_user_input(&self) -> bool {
        matches!(self, Permission::Ask)
    }

    /// Combina dos permisos quedándose con el **más restrictivo**.
    ///
    /// Orden de restrictividad: `Deny > Ask > Allow`.
    pub fn combine(self, other: Permission) -> Permission {
        match (self, other) {
            (Permission::Deny, _) | (_, Permission::Deny) => Permission::Deny,
            (Permission::Ask, _) | (_, Permission::Ask) => Permission::Ask,
            _ => Permission::Allow,
        }
    }
}

/// Scope de una regla.
///
/// Determina a qué aplica la regla. Cada variante es un tipo de match
/// distinto.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Scope {
    /// Aplica a todo (sin filtro adicional).
    #[default]
    Any,

    /// Aplica a paths que coincidan con un glob.
    ///
    /// Ejemplos:
    /// - `"src/**"` — todo dentro de `src/`.
    /// - `"*.md"` — todos los `.md` en la raíz.
    /// - `"**/*.rs"` — todos los `.rs` recursivamente.
    PathGlob { pattern: String },

    /// Aplica a paths que empiecen con un prefijo.
    PathPrefix { prefix: String },

    /// Aplica a un path exacto.
    PathExact { path: String },

    /// Aplica a comandos de la shell que coincidan exactamente.
    ShellProgram { program: String },

    /// Aplica a una acción específica (ej. `"delete"`, `"commit"`).
    Action { name: String },
}

impl Scope {
    /// Constructor conveniente.
    pub fn glob(pattern: impl Into<String>) -> Self {
        Scope::PathGlob {
            pattern: pattern.into(),
        }
    }

    pub fn prefix(prefix: impl Into<String>) -> Self {
        Scope::PathPrefix {
            prefix: prefix.into(),
        }
    }

    pub fn exact(path: impl Into<String>) -> Self {
        Scope::PathExact { path: path.into() }
    }

    pub fn shell_program(program: impl Into<String>) -> Self {
        Scope::ShellProgram {
            program: program.into(),
        }
    }

    pub fn action(name: impl Into<String>) -> Self {
        Scope::Action { name: name.into() }
    }

    /// Etiqueta corta para logs.
    pub fn label(&self) -> String {
        match self {
            Scope::Any => "any".to_string(),
            Scope::PathGlob { pattern } => format!("path:{}", pattern),
            Scope::PathPrefix { prefix } => format!("prefix:{}", prefix),
            Scope::PathExact { path } => format!("exact:{}", path),
            Scope::ShellProgram { program } => format!("shell:{}", program),
            Scope::Action { name } => format!("action:{}", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ──────────────────────────────────────────────
    // Permission
    // ──────────────────────────────────────────────

    #[test]
    fn permission_display_names() {
        assert_eq!(Permission::Allow.display_name(), "ALLOW");
        assert_eq!(Permission::Ask.display_name(), "ASK");
        assert_eq!(Permission::Deny.display_name(), "DENY");
    }

    #[test]
    fn permission_predicates() {
        assert!(Permission::Deny.is_deny());
        assert!(!Permission::Allow.is_deny());
        assert!(Permission::Ask.needs_user_input());
        assert!(!Permission::Allow.needs_user_input());
    }

    #[test]
    fn permission_combine_keeps_most_restrictive() {
        assert_eq!(
            Permission::Allow.combine(Permission::Allow),
            Permission::Allow
        );
        assert_eq!(Permission::Allow.combine(Permission::Ask), Permission::Ask);
        assert_eq!(Permission::Ask.combine(Permission::Allow), Permission::Ask);
        assert_eq!(
            Permission::Allow.combine(Permission::Deny),
            Permission::Deny
        );
        assert_eq!(
            Permission::Deny.combine(Permission::Allow),
            Permission::Deny
        );
        assert_eq!(Permission::Ask.combine(Permission::Deny), Permission::Deny);
    }

    #[test]
    fn permission_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&Permission::Allow).unwrap(),
            "\"allow\""
        );
        assert_eq!(serde_json::to_string(&Permission::Ask).unwrap(), "\"ask\"");
        assert_eq!(
            serde_json::to_string(&Permission::Deny).unwrap(),
            "\"deny\""
        );
    }

    // ──────────────────────────────────────────────
    // Scope
    // ──────────────────────────────────────────────

    #[test]
    fn scope_constructors() {
        assert_eq!(
            Scope::glob("src/**"),
            Scope::PathGlob {
                pattern: "src/**".into()
            }
        );
        assert_eq!(
            Scope::prefix("src/"),
            Scope::PathPrefix {
                prefix: "src/".into()
            }
        );
        assert_eq!(
            Scope::exact("a.txt"),
            Scope::PathExact {
                path: "a.txt".into()
            }
        );
        assert_eq!(
            Scope::shell_program("cargo"),
            Scope::ShellProgram {
                program: "cargo".into()
            }
        );
        assert_eq!(
            Scope::action("delete"),
            Scope::Action {
                name: "delete".into()
            }
        );
    }

    #[test]
    fn scope_default_is_any() {
        assert_eq!(Scope::default(), Scope::Any);
    }

    #[test]
    fn scope_labels() {
        assert_eq!(Scope::Any.label(), "any");
        assert_eq!(Scope::glob("*.md").label(), "path:*.md");
        assert_eq!(Scope::prefix("src/").label(), "prefix:src/");
        assert_eq!(Scope::exact("a.txt").label(), "exact:a.txt");
        assert_eq!(Scope::shell_program("cargo").label(), "shell:cargo");
        assert_eq!(Scope::action("commit").label(), "action:commit");
    }

    #[test]
    fn scope_roundtrips() {
        let s = Scope::glob("src/**");
        let json = serde_json::to_string(&s).unwrap();
        let back: Scope = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }
}
