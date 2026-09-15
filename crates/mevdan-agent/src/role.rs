//! Roles predefinidos con constructores de identidad.
//!
//! `identity::AgentRole` define el tipo. Este módulo ofrece
//! **constructores de conveniencia** y un **catálogo** de roles con
//! identidades listas para usar.

use crate::identity::{AgentIdentity, AgentRole};

/// Crea una identidad `General`.
pub fn general(name: impl Into<String>) -> AgentIdentity {
    AgentIdentity::new(name, AgentRole::General)
}

/// Crea una identidad `Planner`.
pub fn planner(name: impl Into<String>) -> AgentIdentity {
    AgentIdentity::new(name, AgentRole::Planner)
}

/// Crea una identidad `Researcher`.
pub fn researcher(name: impl Into<String>) -> AgentIdentity {
    AgentIdentity::new(name, AgentRole::Researcher)
}

/// Crea una identidad `Coder`.
pub fn coder(name: impl Into<String>) -> AgentIdentity {
    AgentIdentity::new(name, AgentRole::Coder)
}

/// Crea una identidad `Reviewer`.
pub fn reviewer(name: impl Into<String>) -> AgentIdentity {
    AgentIdentity::new(name, AgentRole::Reviewer)
}

/// Crea una identidad `Tester`.
pub fn tester(name: impl Into<String>) -> AgentIdentity {
    AgentIdentity::new(name, AgentRole::Tester)
}

/// Crea una identidad `Debugger`.
pub fn debugger(name: impl Into<String>) -> AgentIdentity {
    AgentIdentity::new(name, AgentRole::Debugger)
}

/// Crea una identidad `Documenter`.
pub fn documenter(name: impl Into<String>) -> AgentIdentity {
    AgentIdentity::new(name, AgentRole::Documenter)
}

/// Catálogo de identidades por defecto.
///
/// Nombres estables para usar en configs, tests y CLI:
/// - `"default-general"`
/// - `"default-planner"`
/// - `"default-coder"`
/// - `"default-reviewer"`
/// - `"default-tester"`
/// - `"default-researcher"`
/// - `"default-debugger"`
/// - `"default-documenter"`
pub struct Defaults;

impl Defaults {
    pub fn all() -> Vec<AgentIdentity> {
        vec![
            general("default-general"),
            planner("default-planner"),
            researcher("default-researcher"),
            coder("default-coder"),
            reviewer("default-reviewer"),
            tester("default-tester"),
            debugger("default-debugger"),
            documenter("default-documenter"),
        ]
    }

    pub fn by_role(role: AgentRole) -> AgentIdentity {
        let name = match role {
            AgentRole::General => "default-general",
            AgentRole::Planner => "default-planner",
            AgentRole::Researcher => "default-researcher",
            AgentRole::Coder => "default-coder",
            AgentRole::Reviewer => "default-reviewer",
            AgentRole::Tester => "default-tester",
            AgentRole::Debugger => "default-debugger",
            AgentRole::Documenter => "default-documenter",
        };
        AgentIdentity::new(name, role)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_set_correct_role() {
        assert_eq!(general("a").role, AgentRole::General);
        assert_eq!(planner("a").role, AgentRole::Planner);
        assert_eq!(researcher("a").role, AgentRole::Researcher);
        assert_eq!(coder("a").role, AgentRole::Coder);
        assert_eq!(reviewer("a").role, AgentRole::Reviewer);
        assert_eq!(tester("a").role, AgentRole::Tester);
        assert_eq!(debugger("a").role, AgentRole::Debugger);
        assert_eq!(documenter("a").role, AgentRole::Documenter);
    }

    #[test]
    fn constructors_preserve_name() {
        assert_eq!(coder("my-coder").name, "my-coder");
    }

    #[test]
    fn defaults_all_returns_eight() {
        let all = Defaults::all();
        assert_eq!(all.len(), 8);
    }

    #[test]
    fn defaults_all_have_prompts() {
        for id in Defaults::all() {
            assert!(!id.system_prompt.is_empty());
        }
    }

    #[test]
    fn defaults_by_role_returns_correct_name() {
        let id = Defaults::by_role(AgentRole::Coder);
        assert_eq!(id.name, "default-coder");
        assert_eq!(id.role, AgentRole::Coder);
    }

    #[test]
    fn defaults_by_role_covers_all_roles() {
        for role in [
            AgentRole::General,
            AgentRole::Planner,
            AgentRole::Researcher,
            AgentRole::Coder,
            AgentRole::Reviewer,
            AgentRole::Tester,
            AgentRole::Debugger,
            AgentRole::Documenter,
        ] {
            let id = Defaults::by_role(role);
            assert_eq!(id.role, role);
        }
    }
}
