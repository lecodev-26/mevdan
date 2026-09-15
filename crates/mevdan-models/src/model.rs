//! Descriptor de un modelo conocido.

use crate::capability::ModelCapabilities;
use serde::{Deserialize, Serialize};

/// Descriptor de un modelo.
///
/// No es el modelo en sí, es la **información sobre el modelo**: nombre,
/// proveedor, capacidades, alias.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDescriptor {
    /// Identificador único y estable del modelo
    /// (ej. `"gpt-4o-mini"`, `"llama3.2"`).
    pub id: String,

    /// Nombre legible para humanos.
    pub display_name: String,

    /// Proveedor al que pertenece (ej. `"openai"`, `"ollama"`,
    /// `"anthropic"`).
    pub provider: String,

    /// Alias alternativos para referirse al modelo
    /// (ej. `["gpt-4o", "gpt4o"]`).
    #[serde(default)]
    pub aliases: Vec<String>,

    /// Capacidades específicas del modelo.
    pub capabilities: ModelCapabilities,

    /// Descripción opcional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl ModelDescriptor {
    /// Crea un descriptor con campos mínimos.
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        provider: impl Into<String>,
        capabilities: ModelCapabilities,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            provider: provider.into(),
            aliases: Vec::new(),
            capabilities,
            description: None,
        }
    }

    /// Añade alias.
    pub fn with_aliases(mut self, aliases: Vec<String>) -> Self {
        self.aliases = aliases;
        self
    }

    /// Añade descripción.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// ¿Este modelo responde a ese nombre o a alguno de sus alias?
    pub fn matches(&self, name: &str) -> bool {
        self.id == name || self.aliases.iter().any(|a| a == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ModelDescriptor {
        ModelDescriptor::new(
            "gpt-4o-mini",
            "GPT-4o mini",
            "openai",
            ModelCapabilities::modern_text(128_000),
        )
        .with_aliases(vec!["4o-mini".into()])
    }

    #[test]
    fn constructor_sets_fields() {
        let m = sample();
        assert_eq!(m.id, "gpt-4o-mini");
        assert_eq!(m.provider, "openai");
        assert_eq!(m.aliases, vec!["4o-mini".to_string()]);
    }

    #[test]
    fn matches_id_and_aliases() {
        let m = sample();
        assert!(m.matches("gpt-4o-mini"));
        assert!(m.matches("4o-mini"));
        assert!(!m.matches("gpt-4"));
    }

    #[test]
    fn roundtrip() {
        let m = sample().with_description("Fast small model");
        let json = serde_json::to_string(&m).unwrap();
        let back: ModelDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, m.id);
        assert_eq!(back.aliases, m.aliases);
        assert_eq!(back.description, m.description);
    }
}
