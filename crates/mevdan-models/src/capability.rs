//! Capacidades de un modelo.
//!
//! Además de las capacidades del provider (`ProviderCapabilities`),
//! cada modelo tiene sus propias capacidades específicas: context
//! window, precio, modalidades soportadas...

use serde::{Deserialize, Serialize};

/// Modalidad de entrada o salida.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    Text,
    Image,
    Audio,
    Video,
}

/// Capacidades específicas de un modelo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Tamaño de la ventana de contexto en tokens.
    pub context_window: u32,

    /// Máximo de tokens de salida en una sola respuesta.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,

    /// Modalidades de entrada que acepta.
    #[serde(default)]
    pub input_modalities: Vec<Modality>,

    /// Modalidades de salida que produce.
    #[serde(default)]
    pub output_modalities: Vec<Modality>,

    /// Soporta tool-calling nativo.
    #[serde(default)]
    pub native_tool_calling: bool,

    /// Soporta salida estructurada (JSON schema).
    #[serde(default)]
    pub structured_output: bool,

    /// Soporta razonamiento explícito (chain-of-thought separable).
    #[serde(default)]
    pub reasoning: bool,
}

impl ModelCapabilities {
    /// Modelo de texto moderno con contexto grande y tool-calling.
    pub fn modern_text(context_window: u32) -> Self {
        Self {
            context_window,
            max_output_tokens: Some(4096),
            input_modalities: vec![Modality::Text],
            output_modalities: vec![Modality::Text],
            native_tool_calling: true,
            structured_output: true,
            reasoning: false,
        }
    }

    /// Modelo multimodal (texto + imagen).
    pub fn multimodal(context_window: u32) -> Self {
        Self {
            context_window,
            max_output_tokens: Some(4096),
            input_modalities: vec![Modality::Text, Modality::Image],
            output_modalities: vec![Modality::Text],
            native_tool_calling: true,
            structured_output: true,
            reasoning: false,
        }
    }

    /// Modelo básico de texto sin tool-calling.
    pub fn basic_text(context_window: u32) -> Self {
        Self {
            context_window,
            max_output_tokens: Some(2048),
            input_modalities: vec![Modality::Text],
            output_modalities: vec![Modality::Text],
            native_tool_calling: false,
            structured_output: false,
            reasoning: false,
        }
    }

    /// ¿Acepta imágenes de entrada?
    pub fn supports_vision(&self) -> bool {
        self.input_modalities.contains(&Modality::Image)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modern_text_has_tools() {
        let c = ModelCapabilities::modern_text(128_000);
        assert_eq!(c.context_window, 128_000);
        assert!(c.native_tool_calling);
        assert!(c.structured_output);
        assert!(!c.supports_vision());
    }

    #[test]
    fn multimodal_supports_vision() {
        let c = ModelCapabilities::multimodal(200_000);
        assert!(c.supports_vision());
        assert!(c.input_modalities.contains(&Modality::Image));
    }

    #[test]
    fn basic_text_has_no_tools() {
        let c = ModelCapabilities::basic_text(4_096);
        assert!(!c.native_tool_calling);
        assert!(!c.structured_output);
    }

    #[test]
    fn modality_serializes_snake_case() {
        assert_eq!(serde_json::to_string(&Modality::Text).unwrap(), "\"text\"");
        assert_eq!(
            serde_json::to_string(&Modality::Image).unwrap(),
            "\"image\""
        );
    }

    #[test]
    fn capabilities_roundtrip() {
        let c = ModelCapabilities::multimodal(100_000);
        let json = serde_json::to_string(&c).unwrap();
        let back: ModelCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(back.context_window, 100_000);
        assert!(back.supports_vision());
    }
}
