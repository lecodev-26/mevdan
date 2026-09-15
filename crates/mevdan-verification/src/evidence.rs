//! Evidencia: prueba concreta de que algo se hizo.
//!
//! A diferencia de un **claim** (que es lo que el agente *dice*), la
//! evidencia es un **hecho comprobable**: el hash de un archivo, el
//! output de un comando, un código HTTP, etc.

use crate::{artifact::ArtifactId, hash::ContentHash};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// ID único de evidencia.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EvidenceId(pub Uuid);

impl EvidenceId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for EvidenceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EvidenceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Tipo de evidencia.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// Un archivo existe (referencia a un artifact File).
    FileExists,
    /// El hash de un archivo coincide con el esperado.
    HashMatch,
    /// Un comando se ejecutó con éxito (exit code 0).
    CommandSuccess,
    /// El output de un comando contiene algo concreto.
    CommandOutputContains,
    /// Un test pasó.
    TestPassed,
    /// Una verificación HTTP devolvió 2xx.
    HttpSuccess,
    /// Otro tipo.
    Other,
}

impl EvidenceKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            EvidenceKind::FileExists => "file_exists",
            EvidenceKind::HashMatch => "hash_match",
            EvidenceKind::CommandSuccess => "command_success",
            EvidenceKind::CommandOutputContains => "command_output_contains",
            EvidenceKind::TestPassed => "test_passed",
            EvidenceKind::HttpSuccess => "http_success",
            EvidenceKind::Other => "other",
        }
    }
}

/// Evidencia concreta de que algo se hizo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: EvidenceId,
    /// Tipo de evidencia.
    pub kind: EvidenceKind,
    /// Descripción legible.
    pub description: String,
    /// Artefacto relacionado (si aplica).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<ArtifactId>,
    /// Hash relacionado (si aplica).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<ContentHash>,
    /// Datos adicionales (output, código, etc.).
    #[serde(default)]
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl Evidence {
    /// Crea evidencia genérica.
    pub fn new(kind: EvidenceKind, description: impl Into<String>) -> Self {
        Self {
            id: EvidenceId::new(),
            kind,
            description: description.into(),
            artifact_id: None,
            hash: None,
            data: serde_json::Value::Null,
            created_at: Utc::now(),
        }
    }

    /// Atajo: evidencia de que un archivo existe.
    pub fn file_exists(path: impl Into<String>) -> Self {
        let path = path.into();
        Self::new(EvidenceKind::FileExists, format!("file exists: {}", path))
            .with_data(serde_json::json!({ "path": path }))
    }

    /// Atajo: evidencia de que un comando salió con éxito.
    pub fn command_success(program: &str, exit_code: i32) -> Self {
        Self::new(
            EvidenceKind::CommandSuccess,
            format!("{} exited with {}", program, exit_code),
        )
        .with_data(serde_json::json!({
            "program": program,
            "exit_code": exit_code,
        }))
    }

    /// Atajo: evidencia de que un test pasó.
    pub fn test_passed(name: impl Into<String>) -> Self {
        let name = name.into();
        Self::new(EvidenceKind::TestPassed, format!("test passed: {}", name))
            .with_data(serde_json::json!({ "test": name }))
    }

    /// Asocia a un artifact.
    pub fn with_artifact(mut self, id: ArtifactId) -> Self {
        self.artifact_id = Some(id);
        self
    }

    /// Asocia un hash.
    pub fn with_hash(mut self, hash: ContentHash) -> Self {
        self.hash = Some(hash);
        self
    }

    /// Añade datos.
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    /// ¿Es una evidencia fuerte (verificable con hash)?
    pub fn is_strong(&self) -> bool {
        matches!(
            self.kind,
            EvidenceKind::HashMatch
                | EvidenceKind::CommandSuccess
                | EvidenceKind::TestPassed
                | EvidenceKind::HttpSuccess
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_ids_unique() {
        let a = EvidenceId::new();
        let b = EvidenceId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn kind_display_names() {
        assert_eq!(EvidenceKind::FileExists.display_name(), "file_exists");
        assert_eq!(
            EvidenceKind::CommandSuccess.display_name(),
            "command_success"
        );
        assert_eq!(EvidenceKind::TestPassed.display_name(), "test_passed");
    }

    #[test]
    fn kind_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&EvidenceKind::FileExists).unwrap(),
            "\"file_exists\""
        );
        assert_eq!(
            serde_json::to_string(&EvidenceKind::HashMatch).unwrap(),
            "\"hash_match\""
        );
    }

    #[test]
    fn new_creates_evidence() {
        let e = Evidence::new(EvidenceKind::Other, "something");
        assert_eq!(e.kind, EvidenceKind::Other);
        assert_eq!(e.description, "something");
        assert!(e.artifact_id.is_none());
        assert!(e.hash.is_none());
    }

    #[test]
    fn file_exists_shortcut() {
        let e = Evidence::file_exists("/tmp/foo");
        assert_eq!(e.kind, EvidenceKind::FileExists);
        assert_eq!(e.data["path"], "/tmp/foo");
    }

    #[test]
    fn command_success_shortcut() {
        let e = Evidence::command_success("cargo", 0);
        assert_eq!(e.kind, EvidenceKind::CommandSuccess);
        assert_eq!(e.data["program"], "cargo");
        assert_eq!(e.data["exit_code"], 0);
    }

    #[test]
    fn test_passed_shortcut() {
        let e = Evidence::test_passed("my_test");
        assert_eq!(e.kind, EvidenceKind::TestPassed);
        assert_eq!(e.data["test"], "my_test");
    }

    #[test]
    fn with_artifact_sets_id() {
        let aid = ArtifactId::new();
        let e = Evidence::file_exists("/x").with_artifact(aid);
        assert_eq!(e.artifact_id, Some(aid));
    }

    #[test]
    fn with_hash_sets_hash() {
        let h = ContentHash::of_str("data");
        let e = Evidence::file_exists("/x").with_hash(h.clone());
        assert_eq!(e.hash, Some(h));
    }

    #[test]
    fn is_strong_works() {
        assert!(Evidence::command_success("cargo", 0).is_strong());
        assert!(Evidence::test_passed("t").is_strong());
        assert!(!Evidence::file_exists("/x").is_strong());
        assert!(!Evidence::new(EvidenceKind::Other, "x").is_strong());
    }

    #[test]
    fn evidence_roundtrips() {
        let e = Evidence::test_passed("t1")
            .with_artifact(ArtifactId::new())
            .with_hash(ContentHash::of_str("x"));
        let json = serde_json::to_string(&e).unwrap();
        let back: Evidence = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, e.id);
        assert_eq!(back.kind, e.kind);
        assert_eq!(back.artifact_id, e.artifact_id);
    }
}
