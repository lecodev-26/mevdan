//! Capacidades de un provider.
//!
//! No todos los providers ni todos los modelos soportan todo. Este
//! módulo define qué puede hacer un provider, para que el runtime pueda
//! adaptar su comportamiento (por ejemplo, caer a un modo texto cuando
//! no hay tool-calling nativo).

use serde::{Deserialize, Serialize};

/// Qué puede hacer un provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    /// Llamadas a herramientas nativas (formato del provider).
    /// Si es `false`, el runtime puede caer a un modo ReAct textual.
    pub native_tool_calling: bool,

    /// Puede ejecutar varias herramientas en una sola respuesta.
    pub parallel_tool_calls: bool,

    /// Puede devolver la respuesta por partes (streaming).
    pub streaming: bool,

    /// Puede forzar formato JSON en la respuesta.
    pub structured_output: bool,

    /// Puede procesar imágenes de entrada (visión).
    pub vision: bool,

    /// Puede generar imágenes.
    pub image_generation: bool,

    /// Puede procesar audio de entrada (speech-to-text).
    pub audio_input: bool,

    /// Puede generar audio de salida (text-to-speech).
    pub audio_output: bool,

    /// Puede generar embeddings.
    pub embeddings: bool,

    /// Expone razonamiento explícito (chain-of-thought separable).
    pub reasoning: bool,
}

impl ProviderCapabilities {
    /// Todas las capacidades desactivadas.
    ///
    /// Útil como base para providers que solo hacen chat texto básico.
    pub const fn none() -> Self {
        Self {
            native_tool_calling: false,
            parallel_tool_calls: false,
            streaming: false,
            structured_output: false,
            vision: false,
            image_generation: false,
            audio_input: false,
            audio_output: false,
            embeddings: false,
            reasoning: false,
        }
    }

    /// Capacidades típicas de un provider OpenAI-compatible moderno.
    pub const fn openai_compatible() -> Self {
        Self {
            native_tool_calling: true,
            parallel_tool_calls: true,
            streaming: true,
            structured_output: true,
            vision: true,
            image_generation: false,
            audio_input: false,
            audio_output: false,
            embeddings: true,
            reasoning: false,
        }
    }

    /// Capacidades típicas de un modelo local vía Ollama.
    pub const fn ollama() -> Self {
        Self {
            native_tool_calling: true,
            parallel_tool_calls: false,
            streaming: true,
            structured_output: true,
            vision: false,
            image_generation: false,
            audio_input: false,
            audio_output: false,
            embeddings: true,
            reasoning: false,
        }
    }

    /// Solo chat de texto. Ni tools, ni streaming, ni nada.
    pub const fn text_only() -> Self {
        Self::none()
    }

    /// ¿Soporta alguna capacidad de tool-calling?
    pub fn supports_tools(&self) -> bool {
        self.native_tool_calling || self.parallel_tool_calls
    }
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self::text_only()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_has_all_false() {
        let c = ProviderCapabilities::none();
        assert!(!c.native_tool_calling);
        assert!(!c.streaming);
        assert!(!c.vision);
        assert!(!c.embeddings);
        assert!(!c.supports_tools());
    }

    #[test]
    fn openai_compatible_has_tools_and_streaming() {
        let c = ProviderCapabilities::openai_compatible();
        assert!(c.native_tool_calling);
        assert!(c.parallel_tool_calls);
        assert!(c.streaming);
        assert!(c.structured_output);
        assert!(c.vision);
        assert!(c.supports_tools());
    }

    #[test]
    fn ollama_has_tools_but_no_parallel() {
        let c = ProviderCapabilities::ollama();
        assert!(c.native_tool_calling);
        assert!(!c.parallel_tool_calls);
        assert!(c.streaming);
        assert!(c.supports_tools());
    }

    #[test]
    fn default_is_text_only() {
        let c = ProviderCapabilities::default();
        assert_eq!(c, ProviderCapabilities::text_only());
    }

    #[test]
    fn roundtrip_through_json() {
        let c = ProviderCapabilities::openai_compatible();
        let json = serde_json::to_string(&c).unwrap();
        let back: ProviderCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }
}
