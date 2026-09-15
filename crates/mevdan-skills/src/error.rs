//! Errores del crate `mevdan-skills`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SkillError {
    #[error("skill not found: {0}")]
    NotFound(String),

    #[error("duplicate skill: {name}@{version}")]
    Duplicate { name: String, version: String },

    #[error("invalid skill name: {0}")]
    InvalidName(String),

    #[error("invalid version: {0}")]
    InvalidVersion(String),

    #[error("invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("manifest not found at {0}")]
    ManifestNotFound(String),

    #[error("skill is incompatible: {0}")]
    Incompatible(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type SkillResult<T> = Result<T, SkillError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_displays_id() {
        let err = SkillError::NotFound("coding@0.1.0".into());
        assert_eq!(err.to_string(), "skill not found: coding@0.1.0");
    }

    #[test]
    fn duplicate_displays_name_version() {
        let err = SkillError::Duplicate {
            name: "coding".into(),
            version: "0.1.0".into(),
        };
        assert_eq!(err.to_string(), "duplicate skill: coding@0.1.0");
    }

    #[test]
    fn invalid_name_displays() {
        let err = SkillError::InvalidName("bad/name".into());
        assert_eq!(err.to_string(), "invalid skill name: bad/name");
    }

    #[test]
    fn manifest_not_found_displays_path() {
        let err = SkillError::ManifestNotFound("/tmp/skill.toml".into());
        assert_eq!(err.to_string(), "manifest not found at /tmp/skill.toml");
    }

    #[test]
    fn incompatible_displays_reason() {
        let err = SkillError::Incompatible("missing tool: x".into());
        assert_eq!(err.to_string(), "skill is incompatible: missing tool: x");
    }
}
