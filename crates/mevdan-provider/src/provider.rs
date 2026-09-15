//! Trait `Provider`.
//!
//! Este es el corazón del sistema de providers. Cualquier adapter
//! (OpenAI, Ollama, Anthropic, Google, etc.) implementa este trait.
//!
//! ## Reglas
//!
//! 1. El trait NO menciona tipos específicos de ningún vendor.
//! 2. `chat()` es el único método obligatorio además de metadatos.
//! 3. Las capacidades opcionales (tool-calling, streaming, vision) se
//!    anuncian en `capabilities()`.
//! 4. Si una capacidad no está soportada y se invoca, el provider debe
//!    devolver `ProviderError::CapabilityNotSupported`.
//!
//! ## Async
//!
//! El trait es síncrono en esta fase. Cuando integremos providers
//! reales (Fase 10), evaluaremos si necesitamos async y haremos la
//! migración coordinada.

use crate::{
    capabilities::ProviderCapabilities,
    error::ProviderResult,
    request::{ChatRequest, ChatResponse},
};

/// Un proveedor de modelos de lenguaje.
pub trait Provider: Send + Sync {
    /// Identificador único del provider (ej. "openai", "ollama").
    ///
    /// Se usa en logs, config, y mensajes de error. Debe ser estable.
    fn name(&self) -> &str;

    /// Capacidades que soporta este provider.
    fn capabilities(&self) -> ProviderCapabilities;

    /// Lista de modelos disponibles.
    ///
    /// Algunos providers (ej. OpenAI) devuelven una lista dinámica
    /// consultando una API. Otros (ej. Ollama) consultan el servidor
    /// local. Los adapters pueden cachear si es necesario.
    fn models(&self) -> ProviderResult<Vec<String>>;

    /// Ejecuta una petición de chat y devuelve la respuesta.
    fn chat(&self, request: ChatRequest) -> ProviderResult<ChatResponse>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockProvider;

    #[test]
    fn provider_trait_is_object_safe() {
        // Verifica que se puede usar como trait object.
        let p: Box<dyn Provider> = Box::new(MockProvider::default());
        assert_eq!(p.name(), "mock");
    }

    #[test]
    fn mock_provider_reports_capabilities() {
        let p = MockProvider::default();
        let caps = p.capabilities();
        assert!(caps.streaming);
        assert!(caps.native_tool_calling);
    }
}
