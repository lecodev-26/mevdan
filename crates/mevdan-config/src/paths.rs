//! Descubrimiento de rutas de configuración.
//!
//! MEVDAN usa:
//!   - Global: `<config_dir>/mevdan/config.toml`
//!     - Linux/Termux: `~/.config/mevdan/config.toml`
//!     - macOS: `~/Library/Application Support/mevdan/config.toml`
//!     - Windows: `%APPDATA%\mevdan\config.toml`
//!   - Proyecto: `<proyecto>/.mevdan/project.toml`
//!
//! Este módulo NUNCA crea directorios. Solo calcula rutas. La creación
//! la hacen `global::GlobalConfig::save()` y `mevdan init`.

use crate::error::{ConfigError, ConfigResult};
use std::path::PathBuf;

/// Directorio global de configuración de MEVDAN.
///
/// Se calcula como `<config_dir>/mevdan/`.
pub fn global_config_dir() -> ConfigResult<PathBuf> {
    let base = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
    Ok(base.join("mevdan"))
}

/// Ruta del archivo de configuración global.
pub fn global_config_file() -> ConfigResult<PathBuf> {
    Ok(global_config_dir()?.join("config.toml"))
}

/// Directorio de configuración de un proyecto MEVDAN.
///
/// Se calcula como `<proyecto>/.mevdan/`.
pub fn project_config_dir(project_root: &std::path::Path) -> PathBuf {
    project_root.join(".mevdan")
}

/// Ruta del archivo `project.toml` de un proyecto.
pub fn project_config_file(project_root: &std::path::Path) -> PathBuf {
    project_config_dir(project_root).join("project.toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn global_config_dir_has_mevdan_suffix() {
        // En CI y Termux hay HOME, así que debería resolverse.
        let dir = global_config_dir().expect("config dir should resolve");
        assert!(
            dir.ends_with("mevdan"),
            "expected path ending in 'mevdan', got {:?}",
            dir
        );
    }

    #[test]
    fn global_config_file_ends_with_config_toml() {
        let file = global_config_file().expect("config file should resolve");
        assert!(file.ends_with("config.toml"));
        assert!(file.parent().unwrap().ends_with("mevdan"));
    }

    #[test]
    fn project_config_dir_joins_mevdan() {
        let root = Path::new("/tmp/my-project");
        let dir = project_config_dir(root);
        assert_eq!(dir, Path::new("/tmp/my-project/.mevdan"));
    }

    #[test]
    fn project_config_file_is_project_toml() {
        let root = Path::new("/tmp/my-project");
        let file = project_config_file(root);
        assert_eq!(file, Path::new("/tmp/my-project/.mevdan/project.toml"));
    }
}
