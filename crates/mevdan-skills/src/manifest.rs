//! `SkillManifest` — definición declarativa de una skill.
//!
//! Vive en `<skill_dir>/skill.toml`.

use crate::error::{SkillError, SkillResult};
use serde::{Deserialize, Serialize};

/// Manifest de una skill.
///
/// Ejemplo de `skill.toml`:
///
/// ```toml
/// [skill]
/// name = "coding"
/// version = "0.1.0"
/// description = "Helps with writing and reviewing code"
/// author = "MEVDAN"
///
/// [skill.instructions]
/// system_prompt = "You are a coding assistant. Write clean, tested code."
///
/// [skill.permissions]
/// required_tools = ["filesystem", "shell", "git"]
///
/// [skill.metadata]
/// tags = ["dev", "coding"]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    pub skill: SkillSection,
}

/// Sección `[skill]` del manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSection {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default)]
    pub instructions: Instructions,
    #[serde(default)]
    pub permissions: Permissions,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Instrucciones que la skill añade al agente.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Instructions {
    /// Texto que se añade al system prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
}

/// Permisos y tools requeridos por la skill.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Permissions {
    /// Tools que la skill necesita.
    #[serde(default)]
    pub required_tools: Vec<String>,
    /// Permisos adicionales que la skill requiere.
    #[serde(default)]
    pub required_permissions: Vec<String>,
}

impl SkillManifest {
    /// Carga desde un archivo TOML.
    pub fn from_toml_str(s: &str) -> SkillResult<Self> {
        let manifest: SkillManifest = toml::from_str(s)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Valida el manifest.
    pub fn validate(&self) -> SkillResult<()> {
        crate::id::validate_name(&self.skill.name)?;
        crate::id::validate_version(&self.skill.version)?;
        if self.skill.description.trim().is_empty() {
            return Err(SkillError::InvalidManifest(
                "description cannot be empty".into(),
            ));
        }
        Ok(())
    }

    /// Genera el `SkillId`.
    pub fn id(&self) -> SkillResult<crate::id::SkillId> {
        crate::id::SkillId::new(&self.skill.name, &self.skill.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_TOML: &str = r#"
[skill]
name = "coding"
version = "0.1.0"
description = "Coding assistant"
"#;

    const FULL_TOML: &str = r#"
[skill]
name = "data-analysis"
version = "0.2.0"
description = "Analyze data and produce reports"
author = "MEVDAN"

[skill.instructions]
system_prompt = "You analyze data rigorously."

[skill.permissions]
required_tools = ["filesystem", "shell"]
required_permissions = ["filesystem.write", "shell.execute"]

[skill.metadata]
tags = ["data", "analysis"]
priority = 5
"#;

    #[test]
    fn parse_minimal_manifest() {
        let m = SkillManifest::from_toml_str(MINIMAL_TOML).unwrap();
        assert_eq!(m.skill.name, "coding");
        assert_eq!(m.skill.version, "0.1.0");
        assert!(m.skill.author.is_none());
        assert!(m.skill.instructions.system_prompt.is_none());
        assert!(m.skill.permissions.required_tools.is_empty());
    }

    #[test]
    fn parse_full_manifest() {
        let m = SkillManifest::from_toml_str(FULL_TOML).unwrap();
        assert_eq!(m.skill.name, "data-analysis");
        assert_eq!(m.skill.author.as_deref(), Some("MEVDAN"));
        assert!(m.skill.instructions.system_prompt.is_some());
        assert_eq!(m.skill.permissions.required_tools.len(), 2);
        assert_eq!(m.skill.permissions.required_permissions.len(), 2);
        assert_eq!(m.skill.metadata["priority"], 5);
    }

    #[test]
    fn parse_invalid_toml_fails() {
        let bad = "this is not = valid = toml";
        assert!(SkillManifest::from_toml_str(bad).is_err());
    }

    #[test]
    fn validate_rejects_empty_description() {
        let toml = r#"
[skill]
name = "test"
version = "0.1.0"
description = ""
"#;
        let err = SkillManifest::from_toml_str(toml).unwrap_err();
        assert!(matches!(err, SkillError::InvalidManifest(_)));
    }

    #[test]
    fn validate_rejects_invalid_name() {
        let toml = r#"
[skill]
name = "Bad Name"
version = "0.1.0"
description = "x"
"#;
        let err = SkillManifest::from_toml_str(toml).unwrap_err();
        assert!(matches!(err, SkillError::InvalidName(_)));
    }

    #[test]
    fn id_returns_skill_id() {
        let m = SkillManifest::from_toml_str(MINIMAL_TOML).unwrap();
        let id = m.id().unwrap();
        assert_eq!(id.name, "coding");
        assert_eq!(id.version, "0.1.0");
    }

    #[test]
    fn manifest_roundtrips_to_toml() {
        let m = SkillManifest::from_toml_str(FULL_TOML).unwrap();
        let back = toml::to_string(&m).unwrap();
        let m2 = SkillManifest::from_toml_str(&back).unwrap();
        assert_eq!(m2.skill.name, m.skill.name);
        assert_eq!(m2.skill.permissions.required_tools.len(), 2);
    }
}
