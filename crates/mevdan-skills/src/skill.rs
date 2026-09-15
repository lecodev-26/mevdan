//! `Skill` — skill cargada.

use crate::{error::SkillResult, id::SkillId, manifest::SkillManifest};
use std::path::PathBuf;

/// Una skill cargada en memoria.
#[derive(Debug, Clone)]
pub struct Skill {
    /// ID (nombre + versión).
    pub id: SkillId,
    /// Manifest completo.
    pub manifest: SkillManifest,
    /// Directorio raíz de la skill (donde vive `skill.toml`).
    pub path: PathBuf,
}

impl Skill {
    /// Crea una skill desde manifest y path.
    pub fn new(manifest: SkillManifest, path: PathBuf) -> SkillResult<Self> {
        let id = manifest.id()?;
        Ok(Self { id, manifest, path })
    }

    /// Nombre.
    pub fn name(&self) -> &str {
        &self.id.name
    }

    /// Versión.
    pub fn version(&self) -> &str {
        &self.id.version
    }

    /// Descripción.
    pub fn description(&self) -> &str {
        &self.manifest.skill.description
    }

    /// System prompt adicional (si lo hay).
    pub fn system_prompt(&self) -> Option<&str> {
        self.manifest.skill.instructions.system_prompt.as_deref()
    }

    /// Tools requeridas.
    pub fn required_tools(&self) -> &[String] {
        &self.manifest.skill.permissions.required_tools
    }

    /// Permisos adicionales requeridos.
    pub fn required_permissions(&self) -> &[String] {
        &self.manifest.skill.permissions.required_permissions
    }

    /// ¿Tiene instrucciones de system prompt?
    pub fn has_system_prompt(&self) -> bool {
        self.system_prompt().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::SkillManifest;

    fn sample_skill() -> Skill {
        let toml = r#"
[skill]
name = "coding"
version = "0.1.0"
description = "Coding assistant"

[skill.instructions]
system_prompt = "You write clean code."

[skill.permissions]
required_tools = ["filesystem"]
"#;
        let manifest = SkillManifest::from_toml_str(toml).unwrap();
        Skill::new(manifest, PathBuf::from("/tmp/coding")).unwrap()
    }

    #[test]
    fn new_computes_id() {
        let s = sample_skill();
        assert_eq!(s.name(), "coding");
        assert_eq!(s.version(), "0.1.0");
    }

    #[test]
    fn description_accessible() {
        let s = sample_skill();
        assert_eq!(s.description(), "Coding assistant");
    }

    #[test]
    fn system_prompt_accessible() {
        let s = sample_skill();
        assert!(s.has_system_prompt());
        assert_eq!(s.system_prompt(), Some("You write clean code."));
    }

    #[test]
    fn required_tools_accessible() {
        let s = sample_skill();
        assert_eq!(s.required_tools(), &["filesystem"]);
    }

    #[test]
    fn required_permissions_empty_by_default() {
        let s = sample_skill();
        assert!(s.required_permissions().is_empty());
    }

    #[test]
    fn skill_without_system_prompt() {
        let toml = r#"
[skill]
name = "no-prompt"
version = "0.1.0"
description = "x"
"#;
        let manifest = SkillManifest::from_toml_str(toml).unwrap();
        let s = Skill::new(manifest, PathBuf::from("/tmp")).unwrap();
        assert!(!s.has_system_prompt());
    }
}
