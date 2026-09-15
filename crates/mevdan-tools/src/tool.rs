//! Trait `Tool` — contrato de una herramienta.
//!
//! Una herramienta es algo que el agente puede invocar. Tiene:
//! - Un **nombre** único y estable.
//! - Una **descripción** legible.
//! - Un **schema de entrada** (JSON schema).
//! - Un **schema de salida** (JSON schema).
//! - Un **nivel de riesgo** (para permisos futuros).
//! - Una función `invoke(input) -> output`.
//!
//! ## Regla fundamental
//!
//! Los tools NUNCA acceden al exterior sin pasar por el sandbox y los
//! permisos (Fase 18). El trait no expone I/O directo; cada tool
//! implementa su propia lógica pero confinada.

use crate::error::ToolResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;

/// Nivel de riesgo de un tool.
///
/// Se usa para decisiones de permisos en Fase 18.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// Solo lectura. Sin efectos secundarios.
    Low = 0,
    /// Escritura en workspace, sin borrados destructivos.
    Medium = 1,
    /// Borrado de archivos, ejecución de comandos.
    High = 2,
    /// Operaciones fuera del workspace, red, sudo.
    Critical = 3,
}

impl RiskLevel {
    pub fn display_name(&self) -> &'static str {
        match self {
            RiskLevel::Low => "low",
            RiskLevel::Medium => "medium",
            RiskLevel::High => "high",
            RiskLevel::Critical => "critical",
        }
    }
}

/// Categoría del tool (para agrupar en UI, docs, permisos).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    Filesystem,
    Shell,
    Git,
    Network,
    Documents,
    Image,
    Audio,
    Video,
    Data,
    Other,
}

/// Metadatos de un tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// Nombre único y estable (ej. `"filesystem"`).
    pub name: String,

    /// Descripción legible.
    pub description: String,

    /// Categoría.
    pub category: ToolCategory,

    /// Nivel de riesgo.
    pub risk: RiskLevel,

    /// JSON schema del input.
    pub input_schema: Value,

    /// JSON schema del output.
    pub output_schema: Value,
}

/// Un tool invocable.
///
/// Requiere `Debug` para permitir logs y debugging. Cada implementación
/// puede derivarlo trivialmente (los campos suelen ser `String` +
/// `ToolMetadata`, que ya es `Debug`).
pub trait Tool: Send + Sync + Debug {
    /// Metadatos del tool.
    fn metadata(&self) -> &ToolMetadata;

    /// Invoca el tool con un input JSON.
    ///
    /// Devuelve un output JSON o un error tipado.
    fn invoke(&self, input: Value) -> ToolResult<Value>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_level_is_ordered() {
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }

    #[test]
    fn risk_level_serializes_snake_case() {
        assert_eq!(serde_json::to_string(&RiskLevel::Low).unwrap(), "\"low\"");
        assert_eq!(
            serde_json::to_string(&RiskLevel::Critical).unwrap(),
            "\"critical\""
        );
    }

    #[test]
    fn risk_level_display_names() {
        assert_eq!(RiskLevel::Low.display_name(), "low");
        assert_eq!(RiskLevel::Critical.display_name(), "critical");
    }

    #[test]
    fn tool_category_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&ToolCategory::Filesystem).unwrap(),
            "\"filesystem\""
        );
        assert_eq!(
            serde_json::to_string(&ToolCategory::Git).unwrap(),
            "\"git\""
        );
    }

    #[test]
    fn metadata_roundtrips() {
        let m = ToolMetadata {
            name: "test".into(),
            description: "a test tool".into(),
            category: ToolCategory::Other,
            risk: RiskLevel::Low,
            input_schema: serde_json::json!({"type": "object"}),
            output_schema: serde_json::json!({"type": "string"}),
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: ToolMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "test");
        assert_eq!(back.risk, RiskLevel::Low);
    }
}
