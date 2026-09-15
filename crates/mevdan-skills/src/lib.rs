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
//!
//! ## Estado del proyecto
//!
//! - **V3.7** ✅ — `Skill`, `SkillManifest`, `SkillLoader`, `SkillRegistry`.
//! - **V3.8** ✅ — `SkillIsolation`, `IsolationPolicy`, `IsolationReport`.

pub mod error;
pub mod id;
pub mod isolation;
pub mod loader;
pub mod manifest;
pub mod registry;
pub mod skill;

// Re-exports de conveniencia.
pub use error::{SkillError, SkillResult};
pub use id::SkillId;
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
    fn full_flow_load_register_isolate() {
        let dir = TempDir::new().unwrap();

        // Skill 1: sin requisitos.
        let coding_dir = dir.path().join("coding");
        fs::create_dir_all(&coding_dir).unwrap();
        fs::write(
            coding_dir.join("skill.toml"),
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

        // Skill 2: requiere tools.
        let shell_dir = dir.path().join("shell-heavy");
        fs::create_dir_all(&shell_dir).unwrap();
        fs::write(
            shell_dir.join("skill.toml"),
            r#"
[skill]
name = "shell-heavy"
version = "0.1.0"
description = "Shell intensive"

[skill.permissions]
required_tools = ["shell"]
required_permissions = ["shell.execute"]
"#,
        )
        .unwrap();

        // Cargar.
        let skills = SkillLoader::load_from_dir(dir.path()).unwrap();
        assert_eq!(skills.len(), 2);

        // Registrar.
        let mut registry = SkillRegistry::new();
        registry.register_all(skills).unwrap();
        assert_eq!(registry.len(), 2);

        // Aislar con política restrictiva.
        let iso = SkillIsolation::strict();
        let reports = iso.analyze_all(&registry.list().into_iter().cloned().collect::<Vec<_>>());

        // Encontramos los dos reportes.
        let coding_report = reports
            .iter()
            .find(|r| r.skill_id.contains("coding"))
            .unwrap();
        assert!(coding_report.is_compatible());

        let shell_report = reports
            .iter()
            .find(|r| r.skill_id.contains("shell-heavy"))
            .unwrap();
        assert!(!shell_report.is_compatible());
        assert!(shell_report.missing_tools.contains(&"shell".to_string()));
    }

    #[test]
    fn full_flow_filter_compatible() {
        let toml_no_reqs = r#"
[skill]
name = "simple"
version = "0.1.0"
description = "Simple"

[skill.permissions]
required_tools = []
required_permissions = []
"#;
        let m1 = SkillManifest::from_toml_str(toml_no_reqs).unwrap();
        let s1 = Skill::new(m1, std::path::PathBuf::from("/tmp")).unwrap();

        let toml_with_reqs = r#"
[skill]
name = "complex"
version = "0.1.0"
description = "Complex"

[skill.permissions]
required_tools = ["shell"]
required_permissions = []
"#;
        let m2 = SkillManifest::from_toml_str(toml_with_reqs).unwrap();
        let s2 = Skill::new(m2, std::path::PathBuf::from("/tmp")).unwrap();

        let iso = SkillIsolation::strict();
        let compatible = iso.filter_compatible(vec![s1, s2]);
        assert_eq!(compatible.len(), 1);
        assert_eq!(compatible[0].name(), "simple");
    }
}
