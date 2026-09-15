//! `MockProvider` — implementación de `Provider` para tests.
//!
//! No hace red. Devuelve respuestas predecibles. Se usa en los tests
//! del trait y en fases posteriores (agentes, tools) para probar sin
//! depender de providers reales.

use crate::{
    capabilities::ProviderCapabilities,
    error::{ProviderError, ProviderResult},
    message::Message,
    provider::Provider,
    request::{ChatRequest, ChatResponse, FinishReason, Usage},
};
use std::sync::Mutex;

/// Provider de prueba.
///
/// Por defecto:
/// - Nombre: "mock"
/// - Capabilities: `openai_compatible()`
/// - Modelos: `["mock-model-1", "mock-model-2"]`
/// - `chat()` devuelve un mensaje de asistente con eco del último
///   mensaje del usuario.
///
/// Se puede personalizar con `with_response` para devolver una
/// respuesta fija.
#[derive(Debug)]
pub struct MockProvider {
    name: String,
    capabilities: ProviderCapabilities,
    models: Vec<String>,
    /// Si es `Some`, `chat()` devuelve esto en vez del eco.
    fixed_response: Mutex<Option<String>>,
    /// Registro de las últimas peticiones recibidas (para tests).
    requests: Mutex<Vec<ChatRequest>>,
}

impl Default for MockProvider {
    fn default() -> Self {
        Self {
            name: "mock".to_string(),
            capabilities: ProviderCapabilities::openai_compatible(),
            models: vec!["mock-model-1".to_string(), "mock-model-2".to_string()],
            fixed_response: Mutex::new(None),
            requests: Mutex::new(Vec::new()),
        }
    }
}

impl MockProvider {
    /// Crea un provider con un nombre personalizado.
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Fija una respuesta literal para `chat()`.
    pub fn with_response(self, response: impl Into<String>) -> Self {
        *self.fixed_response.lock().unwrap() = Some(response.into());
        self
    }

    /// Reemplaza las capacidades.
    pub fn with_capabilities(mut self, caps: ProviderCapabilities) -> Self {
        self.capabilities = caps;
        self
    }

    /// Devuelve cuántas peticiones ha recibido.
    pub fn request_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }

    /// Devuelve la última petición recibida (clonada).
    pub fn last_request(&self) -> Option<ChatRequest> {
        self.requests.lock().unwrap().last().cloned()
    }
}

impl Provider for MockProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> ProviderCapabilities {
        self.capabilities
    }

    fn models(&self) -> ProviderResult<Vec<String>> {
        Ok(self.models.clone())
    }

    fn chat(&self, request: ChatRequest) -> ProviderResult<ChatResponse> {
        // Validación básica
        if request.model.is_empty() {
            return Err(ProviderError::InvalidRequest(
                "model name cannot be empty".into(),
            ));
        }
        if request.messages.is_empty() {
            return Err(ProviderError::InvalidRequest(
                "at least one message is required".into(),
            ));
        }
        if !self.models.contains(&request.model) {
            return Err(ProviderError::ModelNotFound(request.model.clone()));
        }

        // Registra la petición
        self.requests.lock().unwrap().push(request.clone());

        // Respuesta: fija o eco
        let content = self
            .fixed_response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| {
                let last_user = request
                    .messages
                    .iter()
                    .rev()
                    .find(|m| m.role == crate::message::Role::User)
                    .map(|m| m.content.clone())
                    .unwrap_or_default();
                format!("echo: {}", last_user)
            });

        Ok(ChatResponse {
            message: Message::assistant(content),
            model: request.model,
            finish_reason: FinishReason::Stop,
            usage: Some(Usage {
                input_tokens: 10,
                output_tokens: 5,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mock_has_expected_metadata() {
        let m = MockProvider::default();
        assert_eq!(m.name(), "mock");
        assert!(m.capabilities().native_tool_calling);
        assert_eq!(m.models().unwrap().len(), 2);
    }

    #[test]
    fn chat_returns_echo_of_last_user_message() {
        let m = MockProvider::default();
        let req = ChatRequest::simple("mock-model-1", "hola mundo");
        let resp = m.chat(req).unwrap();
        assert_eq!(resp.message.content, "echo: hola mundo");
        assert_eq!(resp.finish_reason, FinishReason::Stop);
    }

    #[test]
    fn chat_with_fixed_response() {
        let m = MockProvider::default().with_response("respuesta fija");
        let req = ChatRequest::simple("mock-model-1", "cualquier cosa");
        let resp = m.chat(req).unwrap();
        assert_eq!(resp.message.content, "respuesta fija");
    }

    #[test]
    fn chat_fails_with_empty_model() {
        let m = MockProvider::default();
        let req = ChatRequest::simple("", "hi");
        let err = m.chat(req).unwrap_err();
        assert!(matches!(err, ProviderError::InvalidRequest(_)));
    }

    #[test]
    fn chat_fails_with_unknown_model() {
        let m = MockProvider::default();
        let req = ChatRequest::simple("nonexistent", "hi");
        let err = m.chat(req).unwrap_err();
        assert!(matches!(err, ProviderError::ModelNotFound(_)));
    }

    #[test]
    fn chat_fails_with_no_messages() {
        let m = MockProvider::default();
        let req = ChatRequest {
            model: "mock-model-1".into(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        let err = m.chat(req).unwrap_err();
        assert!(matches!(err, ProviderError::InvalidRequest(_)));
    }

    #[test]
    fn chat_records_requests() {
        let m = MockProvider::default();
        assert_eq!(m.request_count(), 0);

        let _ = m.chat(ChatRequest::simple("mock-model-1", "one"));
        let _ = m.chat(ChatRequest::simple("mock-model-1", "two"));

        assert_eq!(m.request_count(), 2);
        assert_eq!(m.last_request().unwrap().messages[0].content, "two");
    }

    #[test]
    fn custom_name_works() {
        let m = MockProvider::with_name("my-mock");
        assert_eq!(m.name(), "my-mock");
    }

    #[test]
    fn custom_capabilities_work() {
        let m = MockProvider::default().with_capabilities(ProviderCapabilities::text_only());
        assert!(!m.capabilities().native_tool_calling);
    }
}
