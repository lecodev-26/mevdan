//! `SkillRegistry` — catálogo de skills cargadas.

use crate::{
    error::{SkillError, SkillResult},
    id::SkillId,
    skill::Skill,
};
use std::collections::BTreeMap;

/// Registro de skills cargadas.
#[derive(Debug, Default)]
pub struct SkillRegistry {
    skills: BTreeMap<SkillId, Skill>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra una skill. Falla si ya hay otra con el mismo id.
    pub fn register(&mut self, skill: Skill) -> SkillResult<()> {
        if self.skills.contains_key(&skill.id) {
            return Err(SkillError::Duplicate {
                name: skill.id.name.clone(),
                version: skill.id.version.clone(),
            });
        }
        self.skills.insert(skill.id.clone(), skill);
        Ok(())
    }

    /// Registra o reemplaza.
    pub fn register_or_replace(&mut self, skill: Skill) {
        self.skills.insert(skill.id.clone(), skill);
    }

    /// Registra múltiples skills. Falla si alguna colisiona.
    pub fn register_all(&mut self, skills: Vec<Skill>) -> SkillResult<()> {
        for s in skills {
            self.register(s)?;
        }
        Ok(())
    }

    /// Obtiene una skill por nombre (última versión registrada).
    ///
    /// Si hay varias versiones, devuelve la de mayor versión
    /// (comparación lexicográfica).
    pub fn get_by_name(&self, name: &str) -> Option<&Skill> {
        self.skills
            .values()
            .filter(|s| s.name() == name)
            .max_by(|a, b| a.version().cmp(b.version()))
    }

    /// Obtiene una skill por id exacto.
    pub fn get(&self, id: &SkillId) -> Option<&Skill> {
        self.skills.get(id)
    }

    /// Obtiene o error.
    pub fn require(&self, id: &SkillId) -> SkillResult<&Skill> {
        self.get(id).ok_or_else(|| SkillError::NotFound(id.full()))
    }

    /// Elimina una skill.
    pub fn remove(&mut self, id: &SkillId) -> SkillResult<Skill> {
        self.skills
            .remove(id)
            .ok_or_else(|| SkillError::NotFound(id.full()))
    }

    /// Número de skills.
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// ¿Está vacío?
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// Lista todas las skills (ordenadas por id).
    pub fn list(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    /// Lista los nombres distintos.
    pub fn names(&self) -> Vec<String> {
        let mut set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for s in self.skills.values() {
            set.insert(s.name().to_string());
        }
        set.into_iter().collect()
    }

    /// Limpia el registro.
    pub fn clear(&mut self) {
        self.skills.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::SkillManifest;
    use std::path::PathBuf;

    fn make_skill(name: &str, version: &str) -> Skill {
        let toml = format!(
            r#"
[skill]
name = "{}"
version = "{}"
description = "test"
"#,
            name, version
        );
        let m = SkillManifest::from_toml_str(&toml).unwrap();
        Skill::new(m, PathBuf::from(format!("/tmp/{}", name))).unwrap()
    }

    #[test]
    fn new_registry_is_empty() {
        let r = SkillRegistry::new();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn register_adds_skill() {
        let mut r = SkillRegistry::new();
        r.register(make_skill("coding", "0.1.0")).unwrap();
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn register_duplicate_fails() {
        let mut r = SkillRegistry::new();
        r.register(make_skill("coding", "0.1.0")).unwrap();
        let err = r.register(make_skill("coding", "0.1.0")).unwrap_err();
        assert!(matches!(err, SkillError::Duplicate { .. }));
    }

    #[test]
    fn register_different_versions_ok() {
        let mut r = SkillRegistry::new();
        r.register(make_skill("coding", "0.1.0")).unwrap();
        r.register(make_skill("coding", "0.2.0")).unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn register_or_replace_replaces() {
        let mut r = SkillRegistry::new();
        r.register(make_skill("coding", "0.1.0")).unwrap();
        r.register_or_replace(make_skill("coding", "0.1.0"));
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn register_all_works() {
        let mut r = SkillRegistry::new();
        r.register_all(vec![
            make_skill("coding", "0.1.0"),
            make_skill("research", "0.1.0"),
        ])
        .unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn get_by_name_returns_highest_version() {
        let mut r = SkillRegistry::new();
        r.register(make_skill("coding", "0.1.0")).unwrap();
        r.register(make_skill("coding", "0.2.0")).unwrap();
        r.register(make_skill("coding", "0.1.5")).unwrap();

        let s = r.get_by_name("coding").unwrap();
        assert_eq!(s.version(), "0.2.0");
    }

    #[test]
    fn get_by_name_unknown_returns_none() {
        let r = SkillRegistry::new();
        assert!(r.get_by_name("nope").is_none());
    }

    #[test]
    fn get_by_id_exact() {
        let mut r = SkillRegistry::new();
        r.register(make_skill("coding", "0.1.0")).unwrap();
        r.register(make_skill("coding", "0.2.0")).unwrap();

        let id = SkillId::new("coding", "0.1.0").unwrap();
        let s = r.get(&id).unwrap();
        assert_eq!(s.version(), "0.1.0");
    }

    #[test]
    fn require_unknown_fails() {
        let r = SkillRegistry::new();
        let id = SkillId::new("nope", "0.1.0").unwrap();
        let err = r.require(&id).unwrap_err();
        assert!(matches!(err, SkillError::NotFound(_)));
    }

    #[test]
    fn remove_works() {
        let mut r = SkillRegistry::new();
        r.register(make_skill("coding", "0.1.0")).unwrap();
        let id = SkillId::new("coding", "0.1.0").unwrap();
        r.remove(&id).unwrap();
        assert!(r.is_empty());
    }

    #[test]
    fn names_returns_distinct() {
        let mut r = SkillRegistry::new();
        r.register(make_skill("coding", "0.1.0")).unwrap();
        r.register(make_skill("coding", "0.2.0")).unwrap();
        r.register(make_skill("research", "0.1.0")).unwrap();

        let names = r.names();
        assert_eq!(names, vec!["coding", "research"]);
    }

    #[test]
    fn clear_removes_all() {
        let mut r = SkillRegistry::new();
        r.register(make_skill("a", "0.1.0")).unwrap();
        r.register(make_skill("b", "0.1.0")).unwrap();
        r.clear();
        assert!(r.is_empty());
    }

    #[test]
    fn list_returns_all() {
        let mut r = SkillRegistry::new();
        r.register(make_skill("a", "0.1.0")).unwrap();
        r.register(make_skill("b", "0.1.0")).unwrap();
        assert_eq!(r.list().len(), 2);
    }
}
