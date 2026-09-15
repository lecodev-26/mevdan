//! Artefactos: resultados concretos producidos durante el trabajo.
//!
//! Un artefacto puede ser un archivo, un directorio, un texto, un JSON,
//! etc. Siempre lleva un **hash** de su contenido para integridad.

use crate::hash::ContentHash;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// ID único de un artefacto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArtifactId(pub Uuid);

impl ArtifactId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for ArtifactId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Tipo de artefacto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// Archivo regular en disco.
    File,
    /// Directorio.
    Directory,
    /// Contenido textual inline.
    Text,
    /// Contenido JSON inline.
    Json,
    /// Contenido binario (referencia a un archivo).
    Binary,
    /// Output de un comando.
    CommandOutput,
}

impl ArtifactKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            ArtifactKind::File => "file",
            ArtifactKind::Directory => "directory",
            ArtifactKind::Text => "text",
            ArtifactKind::Json => "json",
            ArtifactKind::Binary => "binary",
            ArtifactKind::CommandOutput => "command_output",
        }
    }
}

/// Un artefacto producido.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: ArtifactId,
    /// Tipo.
    pub kind: ArtifactKind,
    /// Nombre o ruta legible.
    pub name: String,
    /// Hash del contenido.
    pub hash: ContentHash,
    /// Tamaño en bytes.
    pub size_bytes: u64,
    /// Ruta relativa (si aplica).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Metadatos adicionales.
    #[serde(default)]
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl Artifact {
    /// Crea un artefacto a partir de un contenido en memoria.
    pub fn from_content(kind: ArtifactKind, name: impl Into<String>, content: &[u8]) -> Self {
        Self {
            id: ArtifactId::new(),
            kind,
            name: name.into(),
            hash: ContentHash::of_bytes(content),
            size_bytes: content.len() as u64,
            path: None,
            metadata: serde_json::Value::Null,
            created_at: Utc::now(),
        }
    }

    /// Crea un artefacto de texto.
    pub fn from_text(name: impl Into<String>, text: &str) -> Self {
        Self::from_content(ArtifactKind::Text, name, text.as_bytes())
    }

    /// Crea un artefacto JSON.
    pub fn from_json(name: impl Into<String>, value: &serde_json::Value) -> Self {
        let bytes = serde_json::to_vec(value).unwrap_or_default();
        Self::from_content(ArtifactKind::Json, name, &bytes)
    }

    /// Añade una ruta.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Añade metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// ¿Coincide el hash con otro?
    pub fn hash_matches(&self, other: &ContentHash) -> bool {
        self.hash.matches(other)
    }

    /// ¿Es un artefacto de tipo archivo?
    pub fn is_file(&self) -> bool {
        matches!(self.kind, ArtifactKind::File)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_ids_unique() {
        let a = ArtifactId::new();
        let b = ArtifactId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn artifact_id_displays() {
        let id = ArtifactId::new();
        assert!(id.to_string().contains('-'));
    }

    #[test]
    fn kind_display_names() {
        assert_eq!(ArtifactKind::File.display_name(), "file");
        assert_eq!(ArtifactKind::Text.display_name(), "text");
        assert_eq!(ArtifactKind::CommandOutput.display_name(), "command_output");
    }

    #[test]
    fn kind_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&ArtifactKind::File).unwrap(),
            "\"file\""
        );
        assert_eq!(
            serde_json::to_string(&ArtifactKind::CommandOutput).unwrap(),
            "\"command_output\""
        );
    }

    #[test]
    fn from_text_creates_text_artifact() {
        let a = Artifact::from_text("hello.txt", "hello world");
        assert_eq!(a.kind, ArtifactKind::Text);
        assert_eq!(a.name, "hello.txt");
        assert_eq!(a.size_bytes, 11);
        assert_eq!(
            a.hash.as_hex(),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn from_json_creates_json_artifact() {
        let value = serde_json::json!({"x": 1});
        let a = Artifact::from_json("data.json", &value);
        assert_eq!(a.kind, ArtifactKind::Json);
        assert!(a.size_bytes > 0);
    }

    #[test]
    fn with_path_sets_path() {
        let a = Artifact::from_text("t", "x").with_path("src/t");
        assert_eq!(a.path.as_deref(), Some("src/t"));
    }

    #[test]
    fn with_metadata_sets_metadata() {
        let a = Artifact::from_text("t", "x").with_metadata(serde_json::json!({"k": "v"}));
        assert_eq!(a.metadata["k"], "v");
    }

    #[test]
    fn hash_matches_works() {
        let content = "hello";
        let a = Artifact::from_text("t", content);
        let h = ContentHash::of_str(content);
        assert!(a.hash_matches(&h));

        let other = ContentHash::of_str("different");
        assert!(!a.hash_matches(&other));
    }

    #[test]
    fn is_file_works() {
        let a = Artifact::from_text("t", "x");
        assert!(!a.is_file());

        let a = Artifact::from_content(ArtifactKind::File, "f", b"bytes");
        assert!(a.is_file());
    }

    #[test]
    fn artifact_roundtrips() {
        let a = Artifact::from_text("test", "content")
            .with_path("path/to/test")
            .with_metadata(serde_json::json!({"tag": "important"}));
        let json = serde_json::to_string(&a).unwrap();
        let back: Artifact = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, a.id);
        assert_eq!(back.name, a.name);
        assert_eq!(back.hash, a.hash);
        assert_eq!(back.path, a.path);
    }
}
