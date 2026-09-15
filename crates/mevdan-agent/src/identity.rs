//! Identidad de un agente.
//!
//! La identidad define **quién es** un agente: su nombre, su rol, su
//! prompt de sistema, y qué capacidades tiene declaradas.

use serde::{Deserialize, Serialize};

/// Rol del agente en el ecosistema.
///
/// El rol determina convenciones por defecto (por ejemplo, un
/// `Planner` tiende a devolver planes estructurados).
///
/// `PartialOrd` y `Ord` están derivados para permitir usar `AgentRole`
/// como clave de `BTreeMap` (por ejemplo, en `MultiAgent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    /// Agente genérico. Hace lo que se le pide, sin especialización.
    General,
    /// Planifica: descompone objetivos en tareas.
    Planner,
    /// Investiga: busca información y la sintetiza.
    Researcher,
    /// Escribe código.
    Coder,
    /// Revisa trabajo de otros (auto-crítica o revisión de pares).
    Reviewer,
    /// Escribe y ejecuta tests.
    Tester,
    /// Depura: encuentra y arregla bugs.
    Debugger,
    /// Documenta: escribe guías, READMEs, comentarios.
    Documenter,
}

impl AgentRole {
    /// Nombre legible del rol.
    pub fn display_name(&self) -> &'static str {
        match self {
            AgentRole::General => "General",
            AgentRole::Planner => "Planner",
            AgentRole::Researcher => "Researcher",
            AgentRole::Coder => "Coder",
            AgentRole::Reviewer => "Reviewer",
            AgentRole::Tester => "Tester",
            AgentRole::Debugger => "Debugger",
            AgentRole::Documenter => "Documenter",
        }
    }

    /// System prompt por defecto para este rol.
    ///
    /// Es conciso y sin florituras: la personalidad concreta se añade
    /// por encima si hace falta.
    pub fn default_system_prompt(&self) -> &'static str {
        match self {
            AgentRole::General => {
                "You are a helpful AI assistant operating inside MEVDAN, \
                 a local-first work runtime. Answer the user's request \
                 directly and concisely."
            }
            AgentRole::Planner => {
                "You are a planner. Break down the user's goal into \
                 concrete, ordered, actionable steps. Be specific. \
                 Do not execute; only plan."
            }
            AgentRole::Researcher => {
                "You are a researcher. Gather relevant information, \
                 cite your sources when possible, and synthesize a \
                 clear answer. Distinguish facts from speculation."
            }
            AgentRole::Coder => {
                "You are a coder. Write correct, idiomatic code. \
                 Prefer clarity over cleverness. Include error handling. \
                 Do not invent APIs that do not exist."
            }
            AgentRole::Reviewer => {
                "You are a reviewer. Critique the given work honestly. \
                 Point out bugs, unclear code, missing tests, and \
                 security issues. Suggest concrete improvements."
            }
            AgentRole::Tester => {
                "You are a tester. Write tests that would fail if the \
                 code under test were wrong. Cover edge cases, errors, \
                 and boundaries. Do not write tests that always pass."
            }
            AgentRole::Debugger => {
                "You are a debugger. Given a failure, form hypotheses, \
                 gather evidence, and identify the root cause. \
                 Do not fix before you understand."
            }
            AgentRole::Documenter => {
                "You are a technical writer. Write clear, accurate, \
                 concise documentation. Prefer examples over prose. \
                 Do not invent behavior."
            }
        }
    }
}

/// Identidad de un agente.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    /// Nombre único del agente (ej. `"default-coder"`).
    pub name: String,

    /// Rol del agente.
    pub role: AgentRole,

    /// System prompt efectivo. Puede ser el por defecto del rol o uno
    /// personalizado.
    pub system_prompt: String,

    /// Descripción opcional para logs/UI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl AgentIdentity {
    /// Crea una identidad con el prompt por defecto del rol.
    pub fn new(name: impl Into<String>, role: AgentRole) -> Self {
        Self {
            name: name.into(),
            role,
            system_prompt: role.default_system_prompt().to_string(),
            description: None,
        }
    }

    /// Sobrescribe el system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    /// Añade descripción.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_uses_role_default_prompt() {
        let id = AgentIdentity::new("a", AgentRole::Coder);
        assert_eq!(id.name, "a");
        assert_eq!(id.role, AgentRole::Coder);
        assert!(id.system_prompt.contains("coder"));
    }

    #[test]
    fn custom_prompt_overrides_default() {
        let id = AgentIdentity::new("a", AgentRole::Coder).with_system_prompt("custom prompt");
        assert_eq!(id.system_prompt, "custom prompt");
    }

    #[test]
    fn role_display_names() {
        assert_eq!(AgentRole::Planner.display_name(), "Planner");
        assert_eq!(AgentRole::Coder.display_name(), "Coder");
    }

    #[test]
    fn role_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&AgentRole::Planner).unwrap(),
            "\"planner\""
        );
        assert_eq!(
            serde_json::to_string(&AgentRole::Coder).unwrap(),
            "\"coder\""
        );
    }

    #[test]
    fn identity_roundtrips() {
        let id = AgentIdentity::new("test", AgentRole::Reviewer).with_description("A reviewer");
        let json = serde_json::to_string(&id).unwrap();
        let back: AgentIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, id.name);
        assert_eq!(back.role, id.role);
        assert_eq!(back.description, id.description);
    }

    #[test]
    fn all_roles_have_prompts() {
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
            assert!(!role.default_system_prompt().is_empty());
            assert!(!role.display_name().is_empty());
        }
    }

    #[test]
    fn role_can_be_used_as_btreemap_key() {
        use std::collections::BTreeMap;
        let mut map: BTreeMap<AgentRole, u32> = BTreeMap::new();
        map.insert(AgentRole::Coder, 1);
        map.insert(AgentRole::Planner, 2);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&AgentRole::Coder), Some(&1));
    }

    #[test]
    fn role_ord_is_stable() {
        // Como estamos usando BTreeMap, el orden debe ser determinista
        // (el orden de declaración en el enum).
        assert!(AgentRole::General < AgentRole::Planner);
        assert!(AgentRole::Planner < AgentRole::Researcher);
        assert!(AgentRole::Researcher < AgentRole::Coder);
    }
}
