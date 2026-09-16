//! `Team` — conjunto de agentes organizados por rol.

use crate::{
    error::{AgentError, AgentResult},
    identity::AgentRole,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Un miembro del equipo: descripción de un agente asignado a un rol.
///
/// No contiene el agente en sí (que no es serializable), sino su
/// identidad. El `MultiAgentEngine` resuelve el agente real en
/// ejecución.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub role: AgentRole,
    pub agent_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl TeamMember {
    pub fn new(role: AgentRole, agent_name: impl Into<String>) -> Self {
        Self {
            role,
            agent_name: agent_name.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Un equipo de agentes, uno por rol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Team {
    pub name: String,
    pub members: BTreeMap<AgentRole, TeamMember>,
}

impl Team {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            members: BTreeMap::new(),
        }
    }

    /// Añade un miembro. Si ya hay uno con el mismo rol, falla.
    pub fn add_member(&mut self, member: TeamMember) -> AgentResult<()> {
        if self.members.contains_key(&member.role) {
            return Err(AgentError::InvalidConfig(format!(
                "team '{}' already has a member for role {:?}",
                self.name, member.role
            )));
        }
        self.members.insert(member.role, member);
        Ok(())
    }

    /// Añade o reemplaza un miembro.
    pub fn set_member(&mut self, member: TeamMember) {
        self.members.insert(member.role, member);
    }

    /// Devuelve el miembro para un rol.
    pub fn member_for(&self, role: AgentRole) -> Option<&TeamMember> {
        self.members.get(&role)
    }

    /// ¿Tiene miembro para este rol?
    pub fn has_role(&self, role: AgentRole) -> bool {
        self.members.contains_key(&role)
    }

    /// Número de miembros.
    pub fn len(&self) -> usize {
        self.members.len()
    }

    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Roles disponibles.
    pub fn roles(&self) -> Vec<AgentRole> {
        self.members.keys().copied().collect()
    }

    /// Lista de miembros.
    pub fn members(&self) -> Vec<&TeamMember> {
        self.members.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_team_is_empty() {
        let t = Team::new("test");
        assert_eq!(t.name, "test");
        assert!(t.is_empty());
        assert_eq!(t.len(), 0);
    }

    #[test]
    fn add_member_works() {
        let mut t = Team::new("team");
        t.add_member(TeamMember::new(AgentRole::Coder, "coder-1"))
            .unwrap();
        assert_eq!(t.len(), 1);
        assert!(t.has_role(AgentRole::Coder));
    }

    #[test]
    fn add_duplicate_role_fails() {
        let mut t = Team::new("team");
        t.add_member(TeamMember::new(AgentRole::Coder, "coder-1"))
            .unwrap();
        let err = t
            .add_member(TeamMember::new(AgentRole::Coder, "coder-2"))
            .unwrap_err();
        assert!(matches!(err, AgentError::InvalidConfig(_)));
    }

    #[test]
    fn set_member_replaces() {
        let mut t = Team::new("team");
        t.set_member(TeamMember::new(AgentRole::Coder, "coder-1"));
        t.set_member(TeamMember::new(AgentRole::Coder, "coder-2"));
        assert_eq!(t.len(), 1);
        assert_eq!(
            t.member_for(AgentRole::Coder).unwrap().agent_name,
            "coder-2"
        );
    }

    #[test]
    fn member_for_returns_none_when_missing() {
        let t = Team::new("team");
        assert!(t.member_for(AgentRole::Coder).is_none());
    }

    #[test]
    fn roles_and_members_lists() {
        let mut t = Team::new("team");
        t.add_member(TeamMember::new(AgentRole::Coder, "c"))
            .unwrap();
        t.add_member(TeamMember::new(AgentRole::Reviewer, "r"))
            .unwrap();

        let roles = t.roles();
        assert_eq!(roles.len(), 2);
        assert!(roles.contains(&AgentRole::Coder));
        assert!(roles.contains(&AgentRole::Reviewer));

        let members = t.members();
        assert_eq!(members.len(), 2);
    }

    #[test]
    fn member_with_description() {
        let m = TeamMember::new(AgentRole::Coder, "c").with_description("writes code");
        assert_eq!(m.description.as_deref(), Some("writes code"));
    }

    #[test]
    fn team_serializes() {
        let mut t = Team::new("team");
        t.add_member(TeamMember::new(AgentRole::Coder, "c"))
            .unwrap();
        let json = serde_json::to_string(&t).unwrap();
        let back: Team = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "team");
        assert_eq!(back.len(), 1);
    }
}
