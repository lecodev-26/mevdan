//! Claims: lo que el agente **dice** que ha hecho.
//!
//! Un claim NO es prueba. Es una afirmación que el motor de
//! verificación debe comprobar. Ejemplos:
//!
//! - "He creado el archivo src/main.rs".
//! - "Ejecuté cargo test y pasó".
//! - "El output contiene 'OK'".
//!
//! Un claim se resuelve a `Verified`, `Failed`, `Partial` o `Unknown`
//! en la fase de verificación.

use crate::{artifact::ArtifactId, evidence::EvidenceId, hash::ContentHash};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// ID único de un claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ClaimId(pub Uuid);

impl ClaimId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for ClaimId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ClaimId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Tipo de claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    /// El agente dice que ha creado un archivo.
    FileCreated,
    /// El agente dice que ha modificado un archivo.
    FileModified,
    /// El agente dice que un comando se ejecutó con éxito.
    CommandRan,
    /// El agente dice que un test pasó.
    TestPassed,
    /// El agente dice que un recurso HTTP respondió bien.
    HttpSucceeded,
    /// El agente dice que ha completado una tarea.
    TaskCompleted,
    /// Otro tipo.
    Other,
}

impl ClaimKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            ClaimKind::FileCreated => "file_created",
            ClaimKind::FileModified => "file_modified",
            ClaimKind::CommandRan => "command_ran",
            ClaimKind::TestPassed => "test_passed",
            ClaimKind::HttpSucceeded => "http_succeeded",
            ClaimKind::TaskCompleted => "task_completed",
            ClaimKind::Other => "other",
        }
    }
}

/// Estado de un claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    /// Aún no verificado.
    Unverified,
    /// Verificado: hay evidencia que lo respalda.
    Verified,
    /// Falló: la evidencia contradice el claim.
    Failed,
    /// Parcial: parte se verificó, parte no.
    Partial,
    /// Desconocido: no se puede verificar.
    Unknown,
}

impl ClaimStatus {
    pub fn display_name(&self) -> &'static str {
        match self {
            ClaimStatus::Unverified => "unverified",
            ClaimStatus::Verified => "verified",
            ClaimStatus::Failed => "failed",
            ClaimStatus::Partial => "partial",
            ClaimStatus::Unknown => "unknown",
        }
    }

    /// ¿Es un estado final (ya no cambiará)?
    pub fn is_final(&self) -> bool {
        !matches!(self, ClaimStatus::Unverified)
    }
}

/// Una afirmación del agente.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: ClaimId,
    /// Tipo de claim.
    pub kind: ClaimKind,
    /// Descripción legible.
    pub description: String,
    /// Estado de verificación.
    pub status: ClaimStatus,
    /// IDs de las evidencias que lo respaldan.
    #[serde(default)]
    pub evidence_ids: Vec<EvidenceId>,
    /// Artefactos relacionados.
    #[serde(default)]
    pub artifact_ids: Vec<ArtifactId>,
    /// Hash esperado (si el claim involucra un archivo).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_hash: Option<ContentHash>,
    /// Datos adicionales.
    #[serde(default)]
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
    /// Cuándo se verificó (si aplica).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<DateTime<Utc>>,
}

impl Claim {
    /// Crea un claim sin verificar.
    pub fn new(kind: ClaimKind, description: impl Into<String>) -> Self {
        Self {
            id: ClaimId::new(),
            kind,
            description: description.into(),
            status: ClaimStatus::Unverified,
            evidence_ids: Vec::new(),
            artifact_ids: Vec::new(),
            expected_hash: None,
            data: serde_json::Value::Null,
            created_at: Utc::now(),
            verified_at: None,
        }
    }

    /// Atajo: claim de "archivo creado".
    pub fn file_created(path: impl Into<String>) -> Self {
        let path = path.into();
        Self::new(ClaimKind::FileCreated, format!("file created: {}", path))
            .with_data(serde_json::json!({ "path": path }))
    }

    /// Atajo: claim de "comando ejecutado".
    pub fn command_ran(program: impl Into<String>, exit_code: i32) -> Self {
        let program = program.into();
        Self::new(
            ClaimKind::CommandRan,
            format!("{} ran with exit code {}", program, exit_code),
        )
        .with_data(serde_json::json!({
            "program": program,
            "exit_code": exit_code,
        }))
    }

    /// Atajo: claim de "test pasado".
    pub fn test_passed(name: impl Into<String>) -> Self {
        let name = name.into();
        Self::new(ClaimKind::TestPassed, format!("test passed: {}", name))
            .with_data(serde_json::json!({ "test": name }))
    }

    /// Añade una evidencia.
    pub fn with_evidence(mut self, id: EvidenceId) -> Self {
        self.evidence_ids.push(id);
        self
    }

    /// Añade un artefacto.
    pub fn with_artifact(mut self, id: ArtifactId) -> Self {
        self.artifact_ids.push(id);
        self
    }

    /// Fija el hash esperado.
    pub fn with_expected_hash(mut self, hash: ContentHash) -> Self {
        self.expected_hash = Some(hash);
        self
    }

    /// Añade datos.
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    /// Marca como verificado.
    pub fn mark_verified(&mut self) {
        self.status = ClaimStatus::Verified;
        self.verified_at = Some(Utc::now());
    }

    /// Marca como fallido.
    pub fn mark_failed(&mut self) {
        self.status = ClaimStatus::Failed;
        self.verified_at = Some(Utc::now());
    }

    /// Marca como parcial.
    pub fn mark_partial(&mut self) {
        self.status = ClaimStatus::Partial;
        self.verified_at = Some(Utc::now());
    }

    /// Marca como desconocido.
    pub fn mark_unknown(&mut self) {
        self.status = ClaimStatus::Unknown;
        self.verified_at = Some(Utc::now());
    }

    /// ¿Está verificado?
    pub fn is_verified(&self) -> bool {
        self.status == ClaimStatus::Verified
    }

    /// ¿Falló?
    pub fn is_failed(&self) -> bool {
        self.status == ClaimStatus::Failed
    }

    /// ¿Es todavía un claim sin resolver?
    pub fn is_pending(&self) -> bool {
        self.status == ClaimStatus::Unverified
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claim_ids_unique() {
        let a = ClaimId::new();
        let b = ClaimId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn kind_display_names() {
        assert_eq!(ClaimKind::FileCreated.display_name(), "file_created");
        assert_eq!(ClaimKind::CommandRan.display_name(), "command_ran");
        assert_eq!(ClaimKind::TaskCompleted.display_name(), "task_completed");
    }

    #[test]
    fn status_display_names() {
        assert_eq!(ClaimStatus::Unverified.display_name(), "unverified");
        assert_eq!(ClaimStatus::Verified.display_name(), "verified");
    }

    #[test]
    fn status_is_final() {
        assert!(!ClaimStatus::Unverified.is_final());
        assert!(ClaimStatus::Verified.is_final());
        assert!(ClaimStatus::Failed.is_final());
        assert!(ClaimStatus::Partial.is_final());
        assert!(ClaimStatus::Unknown.is_final());
    }

    #[test]
    fn new_claim_starts_unverified() {
        let c = Claim::new(ClaimKind::Other, "x");
        assert_eq!(c.status, ClaimStatus::Unverified);
        assert!(c.is_pending());
        assert!(!c.is_verified());
        assert!(!c.is_failed());
        assert!(c.verified_at.is_none());
    }

    #[test]
    fn file_created_shortcut() {
        let c = Claim::file_created("src/main.rs");
        assert_eq!(c.kind, ClaimKind::FileCreated);
        assert_eq!(c.data["path"], "src/main.rs");
    }

    #[test]
    fn command_ran_shortcut() {
        let c = Claim::command_ran("cargo", 0);
        assert_eq!(c.kind, ClaimKind::CommandRan);
        assert_eq!(c.data["program"], "cargo");
        assert_eq!(c.data["exit_code"], 0);
    }

    #[test]
    fn test_passed_shortcut() {
        let c = Claim::test_passed("my_test");
        assert_eq!(c.kind, ClaimKind::TestPassed);
    }

    #[test]
    fn with_evidence_appends() {
        let e1 = EvidenceId::new();
        let e2 = EvidenceId::new();
        let c = Claim::new(ClaimKind::Other, "x")
            .with_evidence(e1)
            .with_evidence(e2);
        assert_eq!(c.evidence_ids, vec![e1, e2]);
    }

    #[test]
    fn with_artifact_appends() {
        let a1 = ArtifactId::new();
        let c = Claim::file_created("/x").with_artifact(a1);
        assert_eq!(c.artifact_ids, vec![a1]);
    }

    #[test]
    fn with_expected_hash_sets() {
        let h = ContentHash::of_str("data");
        let c = Claim::file_created("/x").with_expected_hash(h.clone());
        assert_eq!(c.expected_hash, Some(h));
    }

    #[test]
    fn mark_verified_sets_status_and_time() {
        let mut c = Claim::new(ClaimKind::Other, "x");
        c.mark_verified();
        assert!(c.is_verified());
        assert!(c.verified_at.is_some());
    }

    #[test]
    fn mark_failed_sets_status_and_time() {
        let mut c = Claim::new(ClaimKind::Other, "x");
        c.mark_failed();
        assert!(c.is_failed());
        assert!(c.verified_at.is_some());
    }

    #[test]
    fn mark_partial_sets_status() {
        let mut c = Claim::new(ClaimKind::Other, "x");
        c.mark_partial();
        assert_eq!(c.status, ClaimStatus::Partial);
    }

    #[test]
    fn mark_unknown_sets_status() {
        let mut c = Claim::new(ClaimKind::Other, "x");
        c.mark_unknown();
        assert_eq!(c.status, ClaimStatus::Unknown);
    }

    #[test]
    fn claim_roundtrips() {
        let mut c = Claim::file_created("/x")
            .with_expected_hash(ContentHash::of_str("h"))
            .with_evidence(EvidenceId::new());
        c.mark_verified();

        let json = serde_json::to_string(&c).unwrap();
        let back: Claim = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, c.id);
        assert_eq!(back.status, c.status);
        assert_eq!(back.expected_hash, c.expected_hash);
        assert_eq!(back.evidence_ids, c.evidence_ids);
    }
}
