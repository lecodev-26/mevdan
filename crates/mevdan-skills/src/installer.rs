//! Skill Installer: instalar skills en un proyecto.

use crate::{
    error::{SkillError, SkillResult},
    loader::SkillLoader,
    manifest::SkillManifest,
    skill::Skill,
};
use std::fs;
use std::path::{Path, PathBuf};

/// Resultado de una instalación.
#[derive(Debug, Clone)]
pub struct InstallResult {
    /// Skill instalada.
    pub skill: Skill,
    /// Ruta destino donde se instaló.
    pub destination: PathBuf,
    /// Si la instalación sobreescribió una existente.
    pub overwrote: bool,
}

/// Instalador de skills.
#[derive(Debug, Default)]
pub struct SkillInstaller;

impl SkillInstaller {
    /// Crea un instalador.
    pub fn new() -> Self {
        Self
    }

    /// Instala una skill desde un directorio fuente al proyecto.
    ///
    /// El directorio fuente debe contener un `skill.toml` válido.
    /// Se copia el directorio completo (incluyendo recursos
    /// adicionales) al destino.
    ///
    /// Destino: `<project_root>/.mevdan/skills/<skill-name>/`.
    ///
    /// Si ya existe una skill con el mismo nombre, se requiere
    /// `overwrite = true`.
    pub fn install_from_dir(
        &self,
        source: &Path,
        project_root: &Path,
        overwrite: bool,
    ) -> SkillResult<InstallResult> {
        // 1. Validar el manifest de origen.
        let source_manifest_path = source.join("skill.toml");
        if !source_manifest_path.exists() {
            return Err(SkillError::ManifestNotFound(
                source_manifest_path.display().to_string(),
            ));
        }

        let content = fs::read_to_string(&source_manifest_path)?;
        let manifest = SkillManifest::from_toml_str(&content)?;
        let skill_name = &manifest.skill.name;

        // 2. Destino.
        let skills_dir = SkillLoader::default_skills_dir(project_root);
        fs::create_dir_all(&skills_dir)?;
        let destination = skills_dir.join(skill_name);

        let overwrote = destination.exists();
        if overwrote && !overwrite {
            return Err(SkillError::Duplicate {
                name: manifest.skill.name.clone(),
                version: manifest.skill.version.clone(),
            });
        }

        // 3. Copiar el directorio completo.
        copy_dir_recursive(source, &destination)?;

        // 4. Cargar la skill instalada.
        let installed = SkillLoader::load_skill_dir(&destination)?;

        Ok(InstallResult {
            skill: installed,
            destination,
            overwrote,
        })
    }

    /// Instala una skill desde una ruta que contiene el manifest
    /// directamente (no un subdirectorio de un directorio de skills).
    ///
    /// Equivalente a `install_from_dir` en la práctica.
    pub fn install_from_path(
        &self,
        source: &Path,
        project_root: &Path,
        overwrite: bool,
    ) -> SkillResult<InstallResult> {
        self.install_from_dir(source, project_root, overwrite)
    }

    /// Desinstala una skill del proyecto.
    ///
    /// Devuelve `true` si existía y se borró.
    pub fn uninstall(&self, skill_name: &str, project_root: &Path) -> SkillResult<bool> {
        let skills_dir = SkillLoader::default_skills_dir(project_root);
        let target = skills_dir.join(skill_name);

        if !target.exists() {
            return Ok(false);
        }

        fs::remove_dir_all(&target)?;
        Ok(true)
    }
}

/// Copia un directorio recursivamente.
fn copy_dir_recursive(src: &Path, dst: &Path) -> SkillResult<()> {
    if !src.is_dir() {
        return Err(SkillError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("source is not a directory: {}", src.display()),
        )));
    }

    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
        // Ignoramos symlinks por ahora.
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_skill_at(base: &Path, name: &str, version: &str) {
        let skill_dir = base.join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        let toml = format!(
            r#"
[skill]
name = "{}"
version = "{}"
description = "test skill"
"#,
            name, version
        );
        fs::write(skill_dir.join("skill.toml"), toml).unwrap();
    }

    #[test]
    fn install_copies_skill_to_project() {
        let source = TempDir::new().unwrap();
        let project = TempDir::new().unwrap();

        let skill_src = source.path().join("coding");
        fs::create_dir_all(&skill_src).unwrap();
        fs::write(
            skill_src.join("skill.toml"),
            r#"
[skill]
name = "coding"
version = "0.1.0"
description = "coding skill"
"#,
        )
        .unwrap();

        let installer = SkillInstaller::new();
        let result = installer
            .install_from_dir(&skill_src, project.path(), false)
            .unwrap();

        assert_eq!(result.skill.name(), "coding");
        assert!(!result.overwrote);

        let dest = SkillLoader::default_skills_dir(project.path()).join("coding");
        assert!(dest.exists());
        assert!(dest.join("skill.toml").exists());
    }

    #[test]
    fn install_copies_extra_files() {
        let source = TempDir::new().unwrap();
        let project = TempDir::new().unwrap();

        let skill_src = source.path().join("my-skill");
        fs::create_dir_all(&skill_src).unwrap();
        fs::write(
            skill_src.join("skill.toml"),
            r#"
[skill]
name = "my-skill"
version = "0.1.0"
description = "x"
"#,
        )
        .unwrap();
        fs::write(skill_src.join("README.md"), "# My skill\n").unwrap();
        fs::create_dir_all(skill_src.join("templates")).unwrap();
        fs::write(skill_src.join("templates/template.txt"), "hi").unwrap();

        let installer = SkillInstaller::new();
        installer
            .install_from_dir(&skill_src, project.path(), false)
            .unwrap();

        let dest = SkillLoader::default_skills_dir(project.path()).join("my-skill");
        assert!(dest.join("README.md").exists());
        assert!(dest.join("templates/template.txt").exists());
    }

    #[test]
    fn install_refuses_overwrite_by_default() {
        let source = TempDir::new().unwrap();
        let project = TempDir::new().unwrap();

        write_skill_at(source.path(), "coding", "0.1.0");

        let installer = SkillInstaller::new();
        installer
            .install_from_dir(&source.path().join("coding"), project.path(), false)
            .unwrap();

        let err = installer
            .install_from_dir(&source.path().join("coding"), project.path(), false)
            .unwrap_err();
        assert!(matches!(err, SkillError::Duplicate { .. }));
    }

    #[test]
    fn install_with_overwrite_replaces() {
        let source = TempDir::new().unwrap();
        let project = TempDir::new().unwrap();

        write_skill_at(source.path(), "coding", "0.1.0");

        let installer = SkillInstaller::new();
        installer
            .install_from_dir(&source.path().join("coding"), project.path(), false)
            .unwrap();

        let result = installer
            .install_from_dir(&source.path().join("coding"), project.path(), true)
            .unwrap();
        assert!(result.overwrote);
    }

    #[test]
    fn install_fails_if_source_has_no_manifest() {
        let source = TempDir::new().unwrap();
        let project = TempDir::new().unwrap();

        let empty_skill_dir = source.path().join("empty");
        fs::create_dir_all(&empty_skill_dir).unwrap();

        let installer = SkillInstaller::new();
        let err = installer
            .install_from_dir(&empty_skill_dir, project.path(), false)
            .unwrap_err();
        assert!(matches!(err, SkillError::ManifestNotFound(_)));
    }

    #[test]
    fn install_fails_with_invalid_manifest() {
        let source = TempDir::new().unwrap();
        let project = TempDir::new().unwrap();

        let bad_skill = source.path().join("bad");
        fs::create_dir_all(&bad_skill).unwrap();
        fs::write(bad_skill.join("skill.toml"), "not = valid = toml").unwrap();

        let installer = SkillInstaller::new();
        let err = installer
            .install_from_dir(&bad_skill, project.path(), false)
            .unwrap_err();
        assert!(matches!(err, SkillError::TomlParse(_)));
    }

    #[test]
    fn uninstall_removes_installed_skill() {
        let source = TempDir::new().unwrap();
        let project = TempDir::new().unwrap();

        write_skill_at(source.path(), "coding", "0.1.0");

        let installer = SkillInstaller::new();
        installer
            .install_from_dir(&source.path().join("coding"), project.path(), false)
            .unwrap();

        let removed = installer.uninstall("coding", project.path()).unwrap();
        assert!(removed);

        let dest = SkillLoader::default_skills_dir(project.path()).join("coding");
        assert!(!dest.exists());
    }

    #[test]
    fn uninstall_unknown_returns_false() {
        let project = TempDir::new().unwrap();
        let installer = SkillInstaller::new();
        let removed = installer.uninstall("nonexistent", project.path()).unwrap();
        assert!(!removed);
    }

    #[test]
    fn install_creates_skills_dir_if_missing() {
        let source = TempDir::new().unwrap();
        let project = TempDir::new().unwrap();

        write_skill_at(source.path(), "coding", "0.1.0");

        let installer = SkillInstaller::new();
        installer
            .install_from_dir(&source.path().join("coding"), project.path(), false)
            .unwrap();

        let skills_dir = SkillLoader::default_skills_dir(project.path());
        assert!(skills_dir.exists());
    }
}
