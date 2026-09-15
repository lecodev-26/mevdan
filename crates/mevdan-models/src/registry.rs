//! Registro de modelos conocidos.
//!
//! El registro es una colección de `ModelDescriptor` indexados por
//! `id`, con búsqueda por alias, proveedor o capacidad.

use crate::{
    capability::ModelCapabilities,
    error::{ModelsError, ModelsResult},
    model::ModelDescriptor,
};
use std::collections::BTreeMap;

/// Registro de modelos.
///
/// Contiene descriptors de modelos conocidos. Se puede construir con
/// `Registry::with_defaults()` (catálogo incluido) o vacío
/// (`Registry::new()`) y poblar a mano.
#[derive(Debug, Clone, Default)]
pub struct Registry {
    models: BTreeMap<String, ModelDescriptor>,
}

impl Registry {
    /// Crea un registro vacío.
    pub fn new() -> Self {
        Self::default()
    }

    /// Crea un registro con un catálogo por defecto de modelos comunes.
    pub fn with_defaults() -> Self {
        let mut reg = Self::new();
        reg.add_defaults();
        reg
    }

    /// Añade el catálogo por defecto.
    ///
    /// Los IDs son estáticos y no colisionan entre sí, así que
    /// ignoramos el `Result` de `register`. Si algún día colisionan,
    /// el modelo nuevo no se inserta y el catálogo sigue siendo válido.
    fn add_defaults(&mut self) {
        // OpenAI
        let _ = self.register(
            ModelDescriptor::new(
                "gpt-4o",
                "GPT-4o",
                "openai",
                ModelCapabilities::multimodal(128_000),
            )
            .with_aliases(vec!["4o".into(), "gpt4o".into()])
            .with_description("OpenAI's flagship multimodal model"),
        );

        let _ = self.register(
            ModelDescriptor::new(
                "gpt-4o-mini",
                "GPT-4o mini",
                "openai",
                ModelCapabilities::multimodal(128_000),
            )
            .with_aliases(vec!["4o-mini".into()])
            .with_description("Fast and cheap multimodal model"),
        );

        let _ = self.register(ModelDescriptor::new(
            "gpt-4-turbo",
            "GPT-4 Turbo",
            "openai",
            ModelCapabilities::multimodal(128_000),
        ));

        // Anthropic
        let _ = self.register(
            ModelDescriptor::new(
                "claude-3-5-sonnet",
                "Claude 3.5 Sonnet",
                "anthropic",
                ModelCapabilities::multimodal(200_000),
            )
            .with_aliases(vec!["claude-sonnet".into()]),
        );

        let _ = self.register(ModelDescriptor::new(
            "claude-3-5-haiku",
            "Claude 3.5 Haiku",
            "anthropic",
            ModelCapabilities::multimodal(200_000),
        ));

        // Ollama common
        let _ = self.register(ModelDescriptor::new(
            "llama3.2",
            "Llama 3.2",
            "ollama",
            ModelCapabilities::modern_text(128_000),
        ));

        let _ = self.register(ModelDescriptor::new(
            "qwen2.5",
            "Qwen 2.5",
            "ollama",
            ModelCapabilities::modern_text(32_000),
        ));

        let _ = self.register(ModelDescriptor::new(
            "mistral",
            "Mistral",
            "ollama",
            ModelCapabilities::modern_text(32_000),
        ));

        // DeepSeek
        let _ = self.register(ModelDescriptor::new(
            "deepseek-chat",
            "DeepSeek Chat",
            "deepseek",
            ModelCapabilities::modern_text(64_000),
        ));
    }

    /// Registra un modelo. Falla si ya existe un modelo con el mismo id.
    pub fn register(&mut self, model: ModelDescriptor) -> ModelsResult<()> {
        if model.id.is_empty() {
            return Err(ModelsError::InvalidDescriptor(
                "model id cannot be empty".into(),
            ));
        }
        if self.models.contains_key(&model.id) {
            return Err(ModelsError::InvalidDescriptor(format!(
                "duplicate model id: {}",
                model.id
            )));
        }
        self.models.insert(model.id.clone(), model);
        Ok(())
    }

    /// Registra o reemplaza un modelo.
    pub fn register_or_replace(&mut self, model: ModelDescriptor) {
        if model.id.is_empty() {
            return;
        }
        self.models.insert(model.id.clone(), model);
    }

    /// Número de modelos registrados.
    pub fn len(&self) -> usize {
        self.models.len()
    }

    /// ¿Está vacío el registro?
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    /// Busca un modelo por id o alias.
    pub fn find(&self, name: &str) -> Option<&ModelDescriptor> {
        if let Some(m) = self.models.get(name) {
            return Some(m);
        }
        self.models.values().find(|m| m.matches(name))
    }

    /// Busca o devuelve error.
    pub fn require(&self, name: &str) -> ModelsResult<&ModelDescriptor> {
        self.find(name)
            .ok_or_else(|| ModelsError::ModelNotFound(name.to_string()))
    }

    /// Lista todos los modelos.
    pub fn list(&self) -> Vec<&ModelDescriptor> {
        self.models.values().collect()
    }

    /// Lista modelos por proveedor.
    pub fn list_by_provider(&self, provider: &str) -> Vec<&ModelDescriptor> {
        self.models
            .values()
            .filter(|m| m.provider == provider)
            .collect()
    }

    /// Lista modelos que cumplen un predicado.
    pub fn list_supporting<F>(&self, predicate: F) -> Vec<&ModelDescriptor>
    where
        F: Fn(&ModelDescriptor) -> bool,
    {
        self.models.values().filter(|m| predicate(m)).collect()
    }

    /// Lista todos los proveedores conocidos.
    pub fn providers(&self) -> Vec<String> {
        let mut set: std::collections::BTreeSet<String> = Default::default();
        for m in self.models.values() {
            set.insert(m.provider.clone());
        }
        set.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_registry_is_empty() {
        let r = Registry::new();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn with_defaults_has_models() {
        let r = Registry::with_defaults();
        assert!(r.len() >= 8);
        assert!(!r.is_empty());
    }

    #[test]
    fn find_by_id() {
        let r = Registry::with_defaults();
        let m = r.find("gpt-4o").unwrap();
        assert_eq!(m.id, "gpt-4o");
        assert_eq!(m.provider, "openai");
    }

    #[test]
    fn find_by_alias() {
        let r = Registry::with_defaults();
        let m = r.find("4o").unwrap();
        assert_eq!(m.id, "gpt-4o");

        let m2 = r.find("4o-mini").unwrap();
        assert_eq!(m2.id, "gpt-4o-mini");
    }

    #[test]
    fn find_unknown_returns_none() {
        let r = Registry::with_defaults();
        assert!(r.find("does-not-exist").is_none());
    }

    #[test]
    fn require_unknown_returns_error() {
        let r = Registry::with_defaults();
        let err = r.require("nope").unwrap_err();
        assert!(matches!(err, ModelsError::ModelNotFound(_)));
    }

    #[test]
    fn register_duplicate_fails() {
        let mut r = Registry::new();
        let m = ModelDescriptor::new(
            "test-model",
            "Test",
            "test",
            ModelCapabilities::basic_text(1000),
        );
        r.register(m.clone()).unwrap();
        assert!(r.register(m).is_err());
    }

    #[test]
    fn register_empty_id_fails() {
        let mut r = Registry::new();
        let m = ModelDescriptor::new("", "Test", "test", ModelCapabilities::basic_text(1000));
        assert!(r.register(m).is_err());
    }

    #[test]
    fn register_or_replace_works() {
        let mut r = Registry::new();
        let m1 = ModelDescriptor::new("m", "Old", "test", ModelCapabilities::basic_text(1000));
        r.register(m1).unwrap();

        let m2 = ModelDescriptor::new("m", "New", "test", ModelCapabilities::basic_text(2000));
        r.register_or_replace(m2);

        let found = r.find("m").unwrap();
        assert_eq!(found.display_name, "New");
        assert_eq!(found.capabilities.context_window, 2000);
    }

    #[test]
    fn list_by_provider_filters() {
        let r = Registry::with_defaults();
        let openai = r.list_by_provider("openai");
        assert!(openai.iter().all(|m| m.provider == "openai"));
        assert!(openai.len() >= 3);
    }

    #[test]
    fn list_supporting_vision() {
        let r = Registry::with_defaults();
        let vision_models = r.list_supporting(|m| m.capabilities.supports_vision());
        assert!(!vision_models.is_empty());
        assert!(vision_models
            .iter()
            .all(|m| m.capabilities.supports_vision()));
    }

    #[test]
    fn providers_returns_all_distinct() {
        let r = Registry::with_defaults();
        let providers = r.providers();
        assert!(providers.contains(&"openai".to_string()));
        assert!(providers.contains(&"anthropic".to_string()));
        assert!(providers.contains(&"ollama".to_string()));
    }

    #[test]
    fn list_returns_all() {
        let r = Registry::with_defaults();
        assert_eq!(r.list().len(), r.len());
    }
}
