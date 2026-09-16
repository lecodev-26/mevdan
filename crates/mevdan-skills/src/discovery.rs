//! Skill Discovery: búsqueda de skills en directorios.
//!
//! Permite encontrar skills en múltiples ubicaciones (proyecto,
//! usuario, sistema) y devolverlas ordenadas por prioridad.

use crate::{error::SkillResult, id::SkillId, loader::SkillLoader, skill::Skill};
use std::path::{Path, PathBuf};

/// Fuente de skills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SkillSource {
    /// Skill en el proyecto actual (`.mevdan/skills/`).
    Project,
    /// Skill del usuario (`~/.config/mevdan/skills/`).
    User,
    /// Skill del sistema (instaladas globalmente).
    System,
    /// Skill en un directorio arbitrario pasado por el usuario.
    Custom,
}

impl SkillSource {
    pub fn display_name(&self) -> &'static str {
        match self {
            SkillSource::Project => "project",
            SkillSource::User => "user",
            SkillSource::System => "system",
            SkillSource::Custom => "custom",
        }
    }

    /// Prioridad (menor = mayor prioridad).
    pub fn priority(&self) -> u8 {
        match self {
            SkillSource::Project => 0,
            SkillSource::User => 1,
            SkillSource::System => 2,
            SkillSource::Custom => 3,
        }
    }
}

/// Una skill descubierta, con su fuente.
#[derive(Debug, Clone)]
pub struct DiscoveredSkill {
    pub skill: Skill,
    pub source: SkillSource,
}

impl DiscoveredSkill {
    pub fn new(skill: Skill, source: SkillSource) -> Self {
        Self { skill, source }
    }

    pub fn id(&self) -> &SkillId {
        &self.skill.id
    }

    pub fn name(&self) -> &str {
        self.skill.name()
    }

    pub fn version(&self) -> &str {
        self.skill.version()
    }
}

/// Descubridor de skills.
#[derive(Debug, Default)]
pub struct SkillDiscovery {
    /// Directorios adicionales a escanear.
    custom_paths: Vec<PathBuf>,
}

impl SkillDiscovery {
    pub fn new() -> Self {
        Self::default()
    }

    /// Añade un directorio custom para escanear.
    pub fn add_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.custom_paths.push(path.into());
        self
    }

    /// Descubre skills desde varias fuentes.
    pub fn discover(&self, project_root: Option<&Path>) -> SkillResult<Vec<DiscoveredSkill>> {
        let mut discovered = Vec::new();

        // 1. Project skills.
        if let Some(root) = project_root {
            let dir = SkillLoader::default_skills_dir(root);
            for skill in SkillLoader::load_from_dir(&dir)? {
                discovered.push(DiscoveredSkill::new(skill, SkillSource::Project));
            }
        }

        // 2. User skills.
        if let Some(user_dir) = user_skills_dir() {
            if user_dir.exists() {
                for skill in SkillLoader::load_from_dir(&user_dir)? {
                    discovered.push(DiscoveredSkill::new(skill, SkillSource::User));
                }
            }
        }

        // 3. Custom paths.
        for path in &self.custom_paths {
            for skill in SkillLoader::load_from_dir(path)? {
                discovered.push(DiscoveredSkill::new(skill, SkillSource::Custom));
            }
        }

        Ok(discovered)
    }

    /// Descubre y devuelve solo una skill por nombre, la de mayor
    /// prioridad (menor `SkillSource::priority`).
    pub fn discover_unique(
        &self,
        project_root: Option<&Path>,
    ) -> SkillResult<Vec<DiscoveredSkill>> {
        let all = self.discover(project_root)?;
        let mut by_name: std::collections::BTreeMap<String, DiscoveredSkill> =
            std::collections::BTreeMap::new();

        for ds in all {
            let name = ds.name().to_string();
            let should_replace = match by_name.get(&name) {
                None => true,
                Some(existing) => {
                    if ds.source.priority() < existing.source.priority() {
                        true
                    } else if ds.source.priority() == existing.source.priority() {
                        ds.version() > existing.version()
                    } else {
                        false
                    }
                }
            };
            if should_replace {
                by_name.insert(name, ds);
            }
        }

        Ok(by_name.into_values().collect())
    }
}

/// Directorio de skills del usuario.
///
/// `~/.config/mevdan/skills/`
fn user_skills_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let home = PathBuf::from(home);
    Some(home.join(".config").join("mevdan").join("skills"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Escribe `skill.toml` directamente en `dir`.
    ///
    /// El directorio `dir` se considera la raíz de la skill (donde vive
    /// `skill.toml`). El nombre del skill lo dicta `name` dentro del
    /// fichero, no el nombre del directorio.
    fn write_skill_in_dir(dir: &Path, name: &str, version: &str) {
        fs::create_dir_all(dir).unwrap();
        let toml = format!(
            r#"
[skill]
name = "{}"
version = "{}"
description = "test"
"#,
            name, version
        );
        fs::write(dir.join("skill.toml"), toml).unwrap();
    }

    /// Crea `dir/<subdir>/skill.toml` con la skill indicada.
    fn write_skill_at(base: &Path, subdir: &str, name: &str, version: &str) {
        let skill_dir = base.join(subdir);
        write_skill_in_dir(&skill_dir, name, version);
    }

    fn setup_project_with_skills() -> TempDir {
        let project = TempDir::new().unwrap();
        let skills_dir = SkillLoader::default_skills_dir(project.path());
        fs::create_dir_all(&skills_dir).unwrap();
        project
    }

    #[test]
    fn source_display_names() {
        assert_eq!(SkillSource::Project.display_name(), "project");
        assert_eq!(SkillSource::User.display_name(), "user");
        assert_eq!(SkillSource::System.display_name(), "system");
        assert_eq!(SkillSource::Custom.display_name(), "custom");
    }

    #[test]
    fn source_priorities() {
        assert!(SkillSource::Project.priority() < SkillSource::User.priority());
        assert!(SkillSource::User.priority() < SkillSource::System.priority());
        assert!(SkillSource::System.priority() < SkillSource::Custom.priority());
    }

    #[test]
    fn discovered_skill_accessors() {
        use crate::manifest::SkillManifest;
        let toml = r#"
[skill]
name = "test"
version = "1.0.0"
description = "x"
"#;
        let m = SkillManifest::from_toml_str(toml).unwrap();
        let s = Skill::new(m, PathBuf::from("/tmp")).unwrap();
        let ds = DiscoveredSkill::new(s, SkillSource::Project);
        assert_eq!(ds.name(), "test");
        assert_eq!(ds.version(), "1.0.0");
    }

    #[test]
    fn discovery_finds_project_skills() {
        let project = setup_project_with_skills();
        let skills_dir = SkillLoader::default_skills_dir(project.path());
        write_skill_at(&skills_dir, "coding", "coding", "0.1.0");
        write_skill_at(&skills_dir, "research", "research", "0.1.0");

        let discovery = SkillDiscovery::new();
        let found = discovery.discover(Some(project.path())).unwrap();

        let project_skills: Vec<_> = found
            .iter()
            .filter(|s| s.source == SkillSource::Project)
            .collect();
        assert_eq!(project_skills.len(), 2);
    }

    #[test]
    fn discovery_finds_custom_paths() {
        let dir = TempDir::new().unwrap();
        write_skill_at(dir.path(), "custom1", "custom1", "0.1.0");
        write_skill_at(dir.path(), "custom2", "custom2", "0.1.0");

        let discovery = SkillDiscovery::new().add_path(dir.path());
        let found = discovery.discover(None).unwrap();

        let custom: Vec<_> = found
            .iter()
            .filter(|s| s.source == SkillSource::Custom)
            .collect();
        assert_eq!(custom.len(), 2);
    }

    #[test]
    fn discovery_with_no_sources_returns_empty() {
        let discovery = SkillDiscovery::new();
        let found = discovery.discover(None).unwrap();
        assert!(found.is_empty());
    }

    #[test]
    fn discovery_unique_filters_by_name() {
        // Dos skills distintas: una del proyecto, otra custom.
        // Ambas tienen el mismo `skill.name = "coding"`.
        let project_dir = TempDir::new().unwrap();
        let custom_dir = TempDir::new().unwrap();

        let project_skills = SkillLoader::default_skills_dir(project_dir.path());
        fs::create_dir_all(&project_skills).unwrap();
        write_skill_at(&project_skills, "coding", "coding", "0.1.0");

        write_skill_at(custom_dir.path(), "coding-custom", "coding", "0.2.0");

        let discovery = SkillDiscovery::new().add_path(custom_dir.path());
        let unique = discovery.discover_unique(Some(project_dir.path())).unwrap();

        let coding: Vec<_> = unique.iter().filter(|s| s.name() == "coding").collect();
        assert_eq!(coding.len(), 1);
        assert_eq!(coding[0].source, SkillSource::Project);
    }

    #[test]
    fn discovery_unique_prefers_project_over_custom() {
        let project_dir = TempDir::new().unwrap();
        let custom_dir = TempDir::new().unwrap();

        let project_skills = SkillLoader::default_skills_dir(project_dir.path());
        fs::create_dir_all(&project_skills).unwrap();
        write_skill_at(&project_skills, "coding", "coding", "0.1.0");

        write_skill_at(custom_dir.path(), "coding-custom", "coding", "0.9.0");

        let discovery = SkillDiscovery::new().add_path(custom_dir.path());
        let unique = discovery.discover_unique(Some(project_dir.path())).unwrap();

        let coding = unique.iter().find(|s| s.name() == "coding").unwrap();
        assert_eq!(coding.source, SkillSource::Project);
        assert_eq!(coding.version(), "0.1.0");
    }

    /// Cuando dos skills del mismo source tienen el mismo nombre,
    /// gana la versión lexicográficamente mayor.
    ///
    /// Ejemplo: `coding/v1` (0.1.0) y `coding/v2` (0.2.0).
    #[test]
    fn discovery_unique_picks_highest_version_same_source() {
        let project_dir = TempDir::new().unwrap();
        let skills_dir = SkillLoader::default_skills_dir(project_dir.path());
        fs::create_dir_all(&skills_dir).unwrap();

        // Dos subdirectorios con nombre distinto, ambos con
        // `skill.name = "coding"`.
        write_skill_at(&skills_dir, "coding-v1", "coding", "0.1.0");
        write_skill_at(&skills_dir, "coding-v2", "coding", "0.2.0");

        let discovery = SkillDiscovery::new();
        let unique = discovery.discover_unique(Some(project_dir.path())).unwrap();

        let coding: Vec<_> = unique.iter().filter(|s| s.name() == "coding").collect();
        assert_eq!(coding.len(), 1);
        assert_eq!(coding[0].version(), "0.2.0");
    }
}
