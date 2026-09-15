//! # mevdan-config
//!
//! Gestión de configuración de MEVDAN.
//!
//! ## Niveles de configuración
//!
//! 1. **Global** (`~/.config/mevdan/config.toml` o equivalente por SO):
//!    preferencias del usuario, referencias a providers.
//! 2. **Proyecto** (`<proyecto>/.mevdan/project.toml`):
//!    metadatos del proyecto, config de storage.
//!
//! ## Reglas
//!
//! 1. **NUNCA secretos aquí.** API keys van en Fase 05 con el
//!    gestor de secretos del sistema (keyring).
//! 2. **Preservar campos desconocidos.** Si la config tiene un campo
//!    que no conocemos, se conserva al leer y guardar. Esto evita perder
//!    config de versiones futuras.
//! 3. **Sin I/O de red.** Solo filesystem local.
//! 4. **Errores explícitos.** No se inventan defaults para archivos
//!    corruptos (excepto el archivo global, que puede no existir).

pub mod error;
pub mod global;
pub mod paths;
pub mod project;

// Re-exports de conveniencia.
pub use error::{ConfigError, ConfigResult};
pub use global::{GlobalConfig, Preferences, GLOBAL_CONFIG_SCHEMA_VERSION};
pub use project::{ProjectConfig, ProjectSection, StorageSection};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn full_flow_global_and_project_config() {
        // Crea config global
        let global_dir = TempDir::new().unwrap();
        let global_path = global_dir.path().join("config.toml");
        let mut global = GlobalConfig::default();
        global.preferences.language = Some("es".into());
        global.save_to(&global_path).unwrap();

        // Crea proyecto
        let project_dir = TempDir::new().unwrap();
        let mevdan_dir = project_dir.path().join(".mevdan");
        fs::create_dir_all(&mevdan_dir).unwrap();
        let project_toml = r#"
[project]
name = "full-flow"
schema_version = "0.1.0"
mevdan_version = "0.1.0"
created_at = "2026-09-15T10:00:00Z"

[storage]
db_file = "mevdan.db"
"#;
        fs::write(mevdan_dir.join("project.toml"), project_toml).unwrap();

        // Lee ambos
        let loaded_global = GlobalConfig::load_from(&global_path).unwrap();
        let loaded_project = ProjectConfig::load(project_dir.path()).unwrap();

        // Verifica
        assert_eq!(loaded_global.preferences.language.as_deref(), Some("es"));
        assert_eq!(loaded_project.project.name, "full-flow");
        assert_eq!(
            loaded_project.db_path(project_dir.path()),
            mevdan_dir.join("mevdan.db")
        );
    }
}
