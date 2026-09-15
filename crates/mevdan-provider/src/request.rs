//! Request y response del chat.
//!
//! Estos tipos son agnósticos de provider. Los adapters se encargan de
//! traducirlos a/desde el formato específico de cada uno.

use crate::message::Message;
use serde::{Deserialize, Serialize};

/// Petición de chat a un provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// Nombre del modelo (ej. "gpt-4o-mini", "llama3.2").
    pub model: String,

    /// Conversación en orden cronológico.
    pub messages: Vec<Message>,

    /// Temperatura (0.0–2.0). `None` = valor por defecto del provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Máximo de tokens a generar. `None` = valor por defecto del provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Si `true`, el provider debería devolver streaming.
    /// (La implementación del streaming llega en una fase posterior.)
    #[serde(default)]
    pub stream: bool,
}

impl ChatRequest {
    /// Crea una petición simple con un solo mensaje de usuario.
    pub fn simple(model: impl Into<String>, user_message: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: vec![Message::user(user_message)],
            temperature: None,
            max_tokens: None,
            stream: false,
        }
    }

    /// Constructor fluido.
    pub fn with_messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }

    pub fn with_temperature(mut self, t: f32) -> Self {
        self.temperature = Some(t);
        self
    }

    pub fn with_max_tokens(mut self, n: u32) -> Self {
        self.max_tokens = Some(n);
        self
    }

    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }
}

/// Respuesta de chat de un provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Mensaje del asistente.
    pub message: Message,

    /// Modelo que realmente respondió (puede diferir del solicitado si
    /// el provider redirige).
    pub model: String,

    /// Motivo por el que el modelo dejó de generar.
    pub finish_reason: FinishReason,

    /// Uso de tokens (si el provider lo reporta).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

/// Motivo por el que el modelo terminó de generar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// Terminó naturalmente.
    Stop,
    /// Alcanzó el límite de tokens.
    Length,
    /// Quiere llamar herramientas (en versiones posteriores).
    ToolCalls,
    /// Fue filtrado por contenido.
    ContentFilter,
    /// Motivo desconocido.
    Unknown,
}

/// Uso de tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

impl Usage {
    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_request_has_one_user_message() {
        let req = ChatRequest::simple("gpt-4o-mini", "hello");
        assert_eq!(req.model, "gpt-4o-mini");
        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.messages[0].content, "hello");
        assert!(!req.stream);
    }

    #[test]
    fn builder_methods_work() {
        let req = ChatRequest::simple("m", "u")
            .with_temperature(0.5)
            .with_max_tokens(100)
            .with_stream(true);
        assert_eq!(req.temperature, Some(0.5));
        assert_eq!(req.max_tokens, Some(100));
        assert!(req.stream);
    }

    #[test]
    fn finish_reason_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&FinishReason::Stop).unwrap(),
            "\"stop\""
        );
        assert_eq!(
            serde_json::to_string(&FinishReason::Length).unwrap(),
            "\"length\""
        );
        assert_eq!(
            serde_json::to_string(&FinishReason::ToolCalls).unwrap(),
            "\"tool_calls\""
        );
    }

    #[test]
    fn usage_total_is_sum() {
        let u = Usage {
            input_tokens: 10,
            output_tokens: 20,
        };
        assert_eq!(u.total(), 30);
    }

    #[test]
    fn chat_response_roundtrips() {
        let resp = ChatResponse {
            message: Message::assistant("hi"),
            model: "m".into(),
            finish_reason: FinishReason::Stop,
            usage: Some(Usage {
                input_tokens: 5,
                output_tokens: 3,
            }),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: ChatResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp.message, back.message);
        assert_eq!(resp.model, back.model);
        assert_eq!(resp.finish_reason, back.finish_reason);
        assert_eq!(resp.usage, back.usage);
    }
}
