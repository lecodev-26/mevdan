//! Configuración global del usuario.
//!
//! Vive en `<config_dir>/mevdan/config.toml`. Contiene preferencias
//! del usuario y referencias a providers. NO contiene secretos
//! (API keys van en Fase 05 con un gestor de secretos del sistema).
//!
//! El archivo es creado con valores por defecto si no existe. Si existe,
//! se hace merge: los campos nuevos se añaden sin borrar los del usuario.

use crate::{error::ConfigResult, paths};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Versión actual del esquema de configuración global.
pub const GLOBAL_CONFIG_SCHEMA_VERSION: &str = "0.1.0";

/// Preferencias generales del usuario.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Preferences {
    /// Idioma preferido (ej. "es", "en"). `None` = auto.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Tema de UI (ej. "dark", "light", "auto"). `None` = auto.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,

    /// Nombre para mostrar en la UI (opcional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
}

/// Configuración global del usuario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Versión del esquema. Permite migraciones futuras.
    pub schema_version: String,

    /// Preferencias generales.
    #[serde(default)]
    pub preferences: Preferences,

    /// Campos extra que no conocemos todavía. Se preservan al guardar.
    /// Esto evita perder config de versiones futuras al leer con una
    /// versión vieja.
    #[serde(default, flatten)]
    pub extra: std::collections::BTreeMap<String, toml::Value>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            schema_version: GLOBAL_CONFIG_SCHEMA_VERSION.to_string(),
            preferences: Preferences::default(),
            extra: Default::default(),
        }
    }
}

impl GlobalConfig {
    /// Carga la configuración global desde disco.
    ///
    /// - Si el archivo no existe, devuelve `GlobalConfig::default()`.
    /// - Si existe pero está corrupto, devuelve error (no inventa).
    pub fn load() -> ConfigResult<Self> {
        let path = paths::global_config_file()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)?;
        let config: GlobalConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Guarda la configuración en disco.
    ///
    /// Crea el directorio padre si no existe.
    pub fn save(&self) -> ConfigResult<PathBuf> {
        let path = paths::global_config_file()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let toml_str = toml::to_string_pretty(self)?;
        fs::write(&path, toml_str)?;
        Ok(path)
    }

    /// Guarda en una ruta específica (útil para tests).
    pub fn save_to(&self, path: &std::path::Path) -> ConfigResult<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let toml_str = toml::to_string_pretty(self)?;
        fs::write(path, toml_str)?;
        Ok(())
    }

    /// Carga desde una ruta específica (útil para tests).
    pub fn load_from(path: &std::path::Path) -> ConfigResult<Self> {
        let content = fs::read_to_string(path)?;
        let config: GlobalConfig = toml::from_str(&content)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ConfigError;
    use tempfile::TempDir;

    #[test]
    fn default_config_has_schema_version() {
        let cfg = GlobalConfig::default();
        assert_eq!(cfg.schema_version, GLOBAL_CONFIG_SCHEMA_VERSION);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        let mut cfg = GlobalConfig::default();
        cfg.preferences.language = Some("es".into());
        cfg.preferences.theme = Some("dark".into());

        cfg.save_to(&path).unwrap();
        assert!(path.exists());

        let loaded = GlobalConfig::load_from(&path).unwrap();
        assert_eq!(loaded.preferences.language.as_deref(), Some("es"));
        assert_eq!(loaded.preferences.theme.as_deref(), Some("dark"));
    }

    #[test]
    fn unknown_fields_are_preserved() {
        // Simula un config de una versión futura con un campo extra.
        let toml_str = r#"
schema_version = "0.1.0"

[preferences]
language = "en"

[future_section]
future_key = "future_value"
"#;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, toml_str).unwrap();

        let cfg = GlobalConfig::load_from(&path).unwrap();
        assert_eq!(cfg.preferences.language.as_deref(), Some("en"));
        assert!(cfg.extra.contains_key("future_section"));
    }

    #[test]
    fn load_missing_file_returns_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("does-not-exist.toml");
        // load_from falla porque el archivo no existe.
        assert!(GlobalConfig::load_from(&path).is_err());
    }

    #[test]
    fn malformed_toml_fails() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "this is not valid = = toml").unwrap();
        let result = GlobalConfig::load_from(&path);
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::TomlParse(_))));
    }
}
