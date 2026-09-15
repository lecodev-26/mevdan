//! Configuración de un proyecto MEVDAN.
//!
//! Vive en `<proyecto>/.mevdan/project.toml`. Es creado por `mevdan init`.
//!
//! Este módulo SOLO lee y valida el archivo. La escritura la hace el CLI
//! durante `init` (para tener control sobre qué se escribe).
//!
//! A diferencia de `global.rs`, aquí NO inventamos defaults: si el
//! archivo existe pero está mal, devolvemos error.

use crate::error::ConfigResult;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Contenido de `.mevdan/project.toml` tal como lo escribe `mevdan init`.
///
/// Es la fuente de verdad para saber si un directorio es un proyecto
/// MEVDAN válido.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Sección `[project]`.
    pub project: ProjectSection,

    /// Sección `[storage]`.
    #[serde(default)]
    pub storage: StorageSection,

    /// Campos extra. Se preservan al leer y guardar.
    #[serde(default, flatten)]
    pub extra: std::collections::BTreeMap<String, toml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSection {
    pub name: String,
    pub schema_version: String,
    pub mevdan_version: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSection {
    #[serde(default = "default_db_file")]
    pub db_file: String,
}

impl Default for StorageSection {
    fn default() -> Self {
        Self {
            db_file: default_db_file(),
        }
    }
}

fn default_db_file() -> String {
    "mevdan.db".to_string()
}

impl ProjectConfig {
    /// Carga el `project.toml` desde el directorio raíz de un proyecto.
    ///
    /// Falla si el archivo no existe o está mal formado.
    pub fn load(project_root: &Path) -> ConfigResult<Self> {
        let path = crate::paths::project_config_file(project_root);
        Self::load_from(&path)
    }

    /// Carga desde una ruta específica (útil para tests).
    pub fn load_from(path: &Path) -> ConfigResult<Self> {
        let content = fs::read_to_string(path)?;
        let config: ProjectConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Devuelve el directorio `.mevdan/` de un proyecto.
    pub fn mevdan_dir(project_root: &Path) -> std::path::PathBuf {
        crate::paths::project_config_dir(project_root)
    }

    /// Ruta al archivo de base de datos SQLite.
    pub fn db_path(&self, project_root: &Path) -> std::path::PathBuf {
        Self::mevdan_dir(project_root).join(&self.storage.db_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ConfigError;
    use tempfile::TempDir;

    const SAMPLE_TOML: &str = r#"
[project]
name = "demo"
schema_version = "0.1.0"
mevdan_version = "0.1.0"
created_at = "2026-09-15T10:00:00Z"

[storage]
db_file = "mevdan.db"
"#;

    #[test]
    fn load_valid_project_toml() {
        let dir = TempDir::new().unwrap();
        let mevdan_dir = dir.path().join(".mevdan");
        fs::create_dir_all(&mevdan_dir).unwrap();
        fs::write(mevdan_dir.join("project.toml"), SAMPLE_TOML).unwrap();

        let cfg = ProjectConfig::load(dir.path()).unwrap();
        assert_eq!(cfg.project.name, "demo");
        assert_eq!(cfg.project.schema_version, "0.1.0");
        assert_eq!(cfg.storage.db_file, "mevdan.db");
    }

    #[test]
    fn db_path_joins_correctly() {
        let dir = TempDir::new().unwrap();
        let mevdan_dir = dir.path().join(".mevdan");
        fs::create_dir_all(&mevdan_dir).unwrap();
        fs::write(mevdan_dir.join("project.toml"), SAMPLE_TOML).unwrap();

        let cfg = ProjectConfig::load(dir.path()).unwrap();
        let db = cfg.db_path(dir.path());
        assert!(db.ends_with(".mevdan/mevdan.db"));
    }

    #[test]
    fn missing_file_returns_error() {
        let dir = TempDir::new().unwrap();
        let result = ProjectConfig::load(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn malformed_toml_returns_error() {
        let dir = TempDir::new().unwrap();
        let mevdan_dir = dir.path().join(".mevdan");
        fs::create_dir_all(&mevdan_dir).unwrap();
        fs::write(mevdan_dir.join("project.toml"), "not = valid = toml").unwrap();

        let result = ProjectConfig::load(dir.path());
        assert!(matches!(result, Err(ConfigError::TomlParse(_))));
    }

    #[test]
    fn missing_storage_section_uses_default() {
        let toml_no_storage = r#"
[project]
name = "demo"
schema_version = "0.1.0"
mevdan_version = "0.1.0"
created_at = "2026-09-15T10:00:00Z"
"#;
        let dir = TempDir::new().unwrap();
        let mevdan_dir = dir.path().join(".mevdan");
        fs::create_dir_all(&mevdan_dir).unwrap();
        fs::write(mevdan_dir.join("project.toml"), toml_no_storage).unwrap();

        let cfg = ProjectConfig::load(dir.path()).unwrap();
        assert_eq!(cfg.storage.db_file, "mevdan.db");
    }

    #[test]
    fn extra_fields_are_preserved() {
        let toml_extra = r#"
[project]
name = "demo"
schema_version = "0.1.0"
mevdan_version = "0.1.0"
created_at = "2026-09-15T10:00:00Z"

[storage]
db_file = "mevdan.db"

[future]
key = "value"
"#;
        let dir = TempDir::new().unwrap();
        let mevdan_dir = dir.path().join(".mevdan");
        fs::create_dir_all(&mevdan_dir).unwrap();
        fs::write(mevdan_dir.join("project.toml"), toml_extra).unwrap();

        let cfg = ProjectConfig::load(dir.path()).unwrap();
        assert!(cfg.extra.contains_key("future"));
    }
}
