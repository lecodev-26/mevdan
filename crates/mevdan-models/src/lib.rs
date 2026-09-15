//! # mevdan-models
//!
//! Registro de modelos conocidos.
//!
//! ## Qué hace
//!
//! - Define `ModelDescriptor` (metadatos de un modelo).
//! - Define `ModelCapabilities` (qué puede hacer un modelo).
//! - Define `Registry` (catálogo consultable).
//!
//! ## Qué NO hace
//!
//! - No hace red. No consulta APIs de providers.
//! - No decide qué modelo usar (eso es el routing, Fase 38).
//! - No invoca modelos (eso es `mevdan-provider`).
//!
//! ## Uso básico
//!
//! ```
//! use mevdan_models::Registry;
//!
//! let registry = Registry::with_defaults();
//! let model = registry.find("gpt-4o").unwrap();
//! assert_eq!(model.provider, "openai");
//! assert!(model.capabilities.context_window > 0);
//! ```

pub mod capability;
pub mod error;
pub mod model;
pub mod registry;

// Re-exports de conveniencia.
pub use capability::{Modality, ModelCapabilities};
pub use error::{ModelsError, ModelsResult};
pub use model::ModelDescriptor;
pub use registry::Registry;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_flow_find_multimodal_model() {
        let registry = Registry::with_defaults();

        // Encuentra un modelo que soporte visión.
        let vision_models = registry.list_supporting(|m| m.capabilities.supports_vision());
        assert!(!vision_models.is_empty());

        // Toma el primero y verifica sus capacidades.
        let first = vision_models[0];
        assert!(first.capabilities.supports_vision());
        assert!(first.capabilities.context_window > 0);
    }

    #[test]
    fn register_custom_model() {
        let mut registry = Registry::with_defaults();
        let initial_count = registry.len();

        let custom = ModelDescriptor::new(
            "my-custom-model",
            "My Custom Model",
            "self-hosted",
            ModelCapabilities::modern_text(32_000),
        )
        .with_description("A local model I configured");

        registry.register(custom).unwrap();
        assert_eq!(registry.len(), initial_count + 1);

        let found = registry.find("my-custom-model").unwrap();
        assert_eq!(found.provider, "self-hosted");
    }

    #[test]
    fn empty_registry_can_be_populated() {
        let mut r = Registry::new();
        assert!(r.is_empty());

        r.register(ModelDescriptor::new(
            "only-model",
            "Only",
            "test",
            ModelCapabilities::basic_text(4000),
        ))
        .unwrap();

        assert_eq!(r.len(), 1);
        assert!(r.find("only-model").is_some());
    }
}
