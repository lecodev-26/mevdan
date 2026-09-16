//! # mevdan-skills
//!
//! Sistema de skills de MEVDAN.
//!
//! ## Concepto
//!
//! Una **skill** es una capacidad instalable que añade:
//! instrucciones, referencias a tools, peticiones de permisos,
//! metadata.
//!
//! ## Componentes
//!
//! - **`Skill`** — skill cargada en memoria.
//! - **`SkillManifest`** — definición declarativa (TOML).
//! - **`SkillLoader`** — carga desde disco.
//! - **`SkillRegistry`** — catálogo de skills.
//! - **`SkillIsolation`** — validación de permisos y compatibilidad.
//! - **`SkillDiscovery`** — búsqueda multi-fuente.
//! - **`SkillInstaller`** — instalación local.
//!
//! ## Estado del proyecto
//!
//! - **V3.7** ✅ — `Skill`, `SkillManifest`, `SkillLoader`, `SkillRegistry`.
//! - **V3.8** ✅ — `SkillIsolation`, `IsolationPolicy`, `IsolationReport`.
//! - **V4.1** ✅ — `SkillDiscovery`, `SkillInstaller`, `SkillSource`.

pub mod discovery;
pub mod error;
pub mod id;
pub mod installer;
pub mod isolation;
pub mod loader;
pub mod manifest;
pub mod registry;
pub mod skill;

// Re-exports de conveniencia.
pub use discovery::{DiscoveredSkill, SkillDiscovery, SkillSource};
pub use error::{SkillError, SkillResult};
pub use id::SkillId;
pub use installer::{InstallResult, SkillInstaller};
pub use isolation::{IsolationPolicy, IsolationReport, IsolationVerdict, SkillIsolation};
pub use loader::SkillLoader;
pub use manifest::{Instructions, Permissions, SkillManifest, SkillSection};
pub use registry::SkillRegistry;
pub use skill::Skill;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn full_flow_discover_install_and_register() {
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
description = "Coding assistant"

[skill.instructions]
system_prompt = "You write clean code."
"#,
        )
        .unwrap();

        // 1. Instalar en el proyecto.
        let installer = SkillInstaller::new();
        installer
            .install_from_dir(&skill_src, project.path(), false)
            .unwrap();

        // 2. Descubrir desde el proyecto.
        let discovery = SkillDiscovery::new();
        let found = discovery.discover(Some(project.path())).unwrap();

        let coding = found
            .iter()
            .find(|s| s.name() == "coding")
            .expect("coding should be discovered");
        assert_eq!(coding.source, SkillSource::Project);

        // 3. Registrar.
        let mut registry = SkillRegistry::new();
        registry.register(coding.skill.clone()).unwrap();

        assert_eq!(registry.len(), 1);
        assert!(registry.get_by_name("coding").is_some());
    }

    #[test]
    fn full_flow_install_then_uninstall() {
        let source = TempDir::new().unwrap();
        let project = TempDir::new().unwrap();

        let skill_src = source.path().join("test");
        fs::create_dir_all(&skill_src).unwrap();
        fs::write(
            skill_src.join("skill.toml"),
            r#"
[skill]
name = "test"
version = "0.1.0"
description = "test skill"
"#,
        )
        .unwrap();

        let installer = SkillInstaller::new();
        installer
            .install_from_dir(&skill_src, project.path(), false)
            .unwrap();

        let dest = SkillLoader::default_skills_dir(project.path()).join("test");
        assert!(dest.exists());

        installer.uninstall("test", project.path()).unwrap();
        assert!(!dest.exists());
    }
}
