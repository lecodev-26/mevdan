//! Errores del crate `mevdan-config`.
//!
//! Regla: los errores de lectura/escritura de configuración viven aquí,
//! no en `mevdan-core`. El core no sabe que existe TOML ni `~/.config`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("no home directory found (HOME not set)")]
    NoHomeDir,

    #[error("no config directory available for this platform")]
    NoConfigDir,

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type ConfigResult<T> = Result<T, ConfigError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_displays() {
        let err = ConfigError::NoHomeDir;
        assert_eq!(err.to_string(), "no home directory found (HOME not set)");
    }

    #[test]
    fn no_config_dir_displays() {
        let err = ConfigError::NoConfigDir;
        assert_eq!(
            err.to_string(),
            "no config directory available for this platform"
        );
    }

    #[test]
    fn core_error_converts() {
        let core_err = mevdan_core::CoreError::InvalidVersion("bad".into());
        let config_err: ConfigError = core_err.into();
        assert!(matches!(config_err, ConfigError::Core(_)));
    }
}
