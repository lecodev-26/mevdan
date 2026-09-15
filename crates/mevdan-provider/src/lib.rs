//! # mevdan-provider
//!
//! Abstracción de providers de modelos de lenguaje.
//!
//! ## Principio fundamental
//!
//! MEVDAN es **provider-agnostic**. Este crate define el contrato que
//! cualquier provider debe cumplir. Los adapters concretos (OpenAI,
//! Ollama, Anthropic, etc.) se implementan en la Fase 10 y viven en
//! submódulos de este mismo crate (o en crates separados si crecen).
//!
//! ## Reglas
//!
//! 1. **Sin tipos de vendor en la API.** Nada de `openai::ChatCompletion`.
//!    MEVDAN define sus propios `Message`, `Role`, `ChatRequest`, etc.
//! 2. **Sin `async` en esta fase.** Cuando se necesite, se añadirá de
//!    forma coordinada.
//! 3. **Sin red en este crate.** Los adapters de red llegan en Fase 10.
//! 4. **Errores tipados.** `ProviderError` cubre los casos comunes.
//! 5. **Capacidades explícitas.** `ProviderCapabilities` evita asumir
//!    que todos los providers hacen lo mismo.

pub mod capabilities;
pub mod error;
pub mod message;
pub mod mock;
pub mod provider;
pub mod request;

// Re-exports de conveniencia.
pub use capabilities::ProviderCapabilities;
pub use error::{ProviderError, ProviderResult};
pub use message::{Message, Role};
pub use mock::MockProvider;
pub use provider::Provider;
pub use request::{ChatRequest, ChatResponse, FinishReason, Usage};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_flow_chat_with_mock_provider() {
        let provider = MockProvider::default().with_response("respuesta de prueba");

        let req = ChatRequest::simple("mock-model-1", "hola").with_temperature(0.7);
        let resp = provider.chat(req).unwrap();

        assert_eq!(resp.message.content, "respuesta de prueba");
        assert_eq!(resp.model, "mock-model-1");
        assert_eq!(resp.finish_reason, FinishReason::Stop);
        assert!(resp.usage.is_some());
    }

    #[test]
    fn provider_can_be_used_as_trait_object() {
        let providers: Vec<Box<dyn Provider>> = vec![
            Box::new(MockProvider::with_name("p1")),
            Box::new(MockProvider::with_name("p2")),
        ];

        for p in &providers {
            assert!(!p.name().is_empty());
            assert!(p.capabilities().streaming);
        }

        let names: Vec<String> = providers.iter().map(|p| p.name().to_string()).collect();
        assert_eq!(names, vec!["p1", "p2"]);
    }

    #[test]
    fn capabilities_drive_runtime_behavior() {
        let modern = MockProvider::default();
        // `with_name` es un constructor asociado; se encadena antes
        // del método fluido `with_capabilities`.
        let basic =
            MockProvider::with_name("basic").with_capabilities(ProviderCapabilities::text_only());

        assert!(modern.capabilities().supports_tools());
        assert!(!basic.capabilities().supports_tools());
    }
}
