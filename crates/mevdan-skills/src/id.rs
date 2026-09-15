//! ID tipado de skill (name + version).

use crate::error::{SkillError, SkillResult};
use serde::{Deserialize, Serialize};
use std::fmt;

/// ID único de una skill: nombre + versión.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SkillId {
    pub name: String,
    pub version: String,
}

impl SkillId {
    /// Crea un `SkillId` validando nombre y versión.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> SkillResult<Self> {
        let name = name.into();
        let version = version.into();
        validate_name(&name)?;
        validate_version(&version)?;
        Ok(Self { name, version })
    }

    /// Devuelve el id en formato `name@version`.
    pub fn full(&self) -> String {
        format!("{}@{}", self.name, self.version)
    }
}

impl fmt::Display for SkillId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.name, self.version)
    }
}

/// Valida el nombre de una skill.
///
/// Reglas: 1-64 chars, solo `a-z`, `0-9`, `-`, `_`.
pub fn validate_name(name: &str) -> SkillResult<()> {
    if name.is_empty() || name.len() > 64 {
        return Err(SkillError::InvalidName(name.to_string()));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        return Err(SkillError::InvalidName(name.to_string()));
    }
    Ok(())
}

/// Valida una versión semver-like.
///
/// Reglas: al menos `X.Y` o `X.Y.Z`. Solo dígitos y puntos.
pub fn validate_version(version: &str) -> SkillResult<()> {
    if version.is_empty() || version.len() > 32 {
        return Err(SkillError::InvalidVersion(version.to_string()));
    }
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() < 2 {
        return Err(SkillError::InvalidVersion(version.to_string()));
    }
    for p in &parts {
        if p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()) {
            return Err(SkillError::InvalidVersion(version.to_string()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_id() {
        let id = SkillId::new("coding", "0.1.0").unwrap();
        assert_eq!(id.name, "coding");
        assert_eq!(id.version, "0.1.0");
    }

    #[test]
    fn full_format() {
        let id = SkillId::new("coding", "0.1.0").unwrap();
        assert_eq!(id.full(), "coding@0.1.0");
    }

    #[test]
    fn display_format() {
        let id = SkillId::new("coding", "0.1.0").unwrap();
        assert_eq!(id.to_string(), "coding@0.1.0");
    }

    #[test]
    fn invalid_names_rejected() {
        assert!(SkillId::new("", "0.1.0").is_err());
        assert!(SkillId::new("Coding", "0.1.0").is_err());
        assert!(SkillId::new("bad/name", "0.1.0").is_err());
        assert!(SkillId::new("bad name", "0.1.0").is_err());
        assert!(SkillId::new("a".repeat(65), "0.1.0").is_err());
    }

    #[test]
    fn valid_names_accepted() {
        assert!(SkillId::new("coding", "0.1.0").is_ok());
        assert!(SkillId::new("data-analysis", "0.1.0").is_ok());
        assert!(SkillId::new("web_research", "0.1.0").is_ok());
        assert!(SkillId::new("skill123", "0.1.0").is_ok());
    }

    #[test]
    fn invalid_versions_rejected() {
        assert!(SkillId::new("coding", "").is_err());
        assert!(SkillId::new("coding", "1").is_err());
        assert!(SkillId::new("coding", "1.x.0").is_err());
        assert!(SkillId::new("coding", "1..0").is_err());
    }

    #[test]
    fn valid_versions_accepted() {
        assert!(SkillId::new("coding", "0.1").is_ok());
        assert!(SkillId::new("coding", "0.1.0").is_ok());
        assert!(SkillId::new("coding", "1.2.3").is_ok());
    }

    #[test]
    fn id_roundtrips() {
        let id = SkillId::new("coding", "0.1.0").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        let back: SkillId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }
}
