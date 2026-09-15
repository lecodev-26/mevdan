//! `SkillLoader` — carga skills desde disco.

use crate::{
    error::{SkillError, SkillResult},
    manifest::SkillManifest,
    skill::Skill,
};
use std::fs;
use std::path::{Path, PathBuf};

/// Carga skills desde un directorio.
///
/// Estructura esperada:
///
/// ```text
/// skills_root/
///   coding/
///     skill.toml
///   research/
///     skill.toml
/// ```
pub struct SkillLoader;

impl SkillLoader {
    /// Carga todas las skills de un directorio.
    ///
    /// Ignora subdirectorios sin `skill.toml`. Falla si un `skill.toml`
    /// existe pero es inválido.
    pub fn load_from_dir(root: impl AsRef<Path>) -> SkillResult<Vec<Skill>> {
        let root = root.as_ref();
        if !root.exists() {
            return Ok(Vec::new());
        }
        if !root.is_dir() {
            return Err(SkillError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("not a directory: {}", root.display()),
            )));
        }

        let mut skills = Vec::new();
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let manifest_path = path.join("skill.toml");
            if !manifest_path.exists() {
                continue;
            }
            let skill = Self::load_skill_dir(&path)?;
            skills.push(skill);
        }
        Ok(skills)
    }

    /// Carga una skill desde un directorio específico.
    pub fn load_skill_dir(dir: impl AsRef<Path>) -> SkillResult<Skill> {
        let dir = dir.as_ref();
        let manifest_path = dir.join("skill.toml");

        if !manifest_path.exists() {
            return Err(SkillError::ManifestNotFound(
                manifest_path.display().to_string(),
            ));
        }

        let content = fs::read_to_string(&manifest_path)?;
        let manifest = SkillManifest::from_toml_str(&content)?;
        Skill::new(manifest, dir.to_path_buf())
    }

    /// Encuentra la ruta por defecto de skills para un proyecto:
    /// `<project_root>/.mevdan/skills/`.
    pub fn default_skills_dir(project_root: impl AsRef<Path>) -> PathBuf {
        project_root.as_ref().join(".mevdan").join("skills")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_skill(dir: &Path, name: &str) {
        let skill_dir = dir.join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        let toml = format!(
            r#"
[skill]
name = "{}"
version = "0.1.0"
description = "Test skill {}"
"#,
            name, name
        );
        fs::write(skill_dir.join("skill.toml"), toml).unwrap();
    }

    #[test]
    fn load_from_missing_dir_returns_empty() {
        let result = SkillLoader::load_from_dir("/nonexistent/xyz123").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn load_from_empty_dir_returns_empty() {
        let dir = TempDir::new().unwrap();
        let result = SkillLoader::load_from_dir(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn load_finds_skills() {
        let dir = TempDir::new().unwrap();
        write_skill(dir.path(), "coding");
        write_skill(dir.path(), "research");

        let skills = SkillLoader::load_from_dir(dir.path()).unwrap();
        assert_eq!(skills.len(), 2);

        let names: Vec<&str> = skills.iter().map(|s| s.name()).collect();
        assert!(names.contains(&"coding"));
        assert!(names.contains(&"research"));
    }

    #[test]
    fn load_ignores_dirs_without_manifest() {
        let dir = TempDir::new().unwrap();
        write_skill(dir.path(), "coding");
        // Directorio sin skill.toml.
        fs::create_dir_all(dir.path().join("no-manifest")).unwrap();

        let skills = SkillLoader::load_from_dir(dir.path()).unwrap();
        assert_eq!(skills.len(), 1);
    }

    #[test]
    fn load_fails_on_invalid_manifest() {
        let dir = TempDir::new().unwrap();
        write_skill(dir.path(), "valid");

        let bad_dir = dir.path().join("bad");
        fs::create_dir_all(&bad_dir).unwrap();
        fs::write(bad_dir.join("skill.toml"), "invalid = = toml").unwrap();

        let result = SkillLoader::load_from_dir(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn load_skill_dir_works() {
        let dir = TempDir::new().unwrap();
        write_skill(dir.path(), "solo");

        let skill = SkillLoader::load_skill_dir(dir.path().join("solo")).unwrap();
        assert_eq!(skill.name(), "solo");
    }

    #[test]
    fn load_skill_dir_missing_manifest_fails() {
        let dir = TempDir::new().unwrap();
        let result = SkillLoader::load_skill_dir(dir.path());
        assert!(matches!(result, Err(SkillError::ManifestNotFound(_))));
    }

    #[test]
    fn default_skills_dir_joins_correctly() {
        let dir = SkillLoader::default_skills_dir("/tmp/project");
        assert_eq!(dir, PathBuf::from("/tmp/project/.mevdan/skills"));
    }

    #[test]
    fn load_fails_if_root_is_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("file.txt");
        fs::write(&file, "x").unwrap();
        let result = SkillLoader::load_from_dir(&file);
        assert!(result.is_err());
    }
}
