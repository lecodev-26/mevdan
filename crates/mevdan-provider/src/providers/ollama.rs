//! Adapter para Ollama (servidor local de modelos).
//!
//! Ollama expone su propia API en `http://localhost:11434` por defecto.
//! No es compatible con OpenAI, así que tiene su propio adapter.
//!
//! ## Configuración
//!
//! ```no_run
//! use mevdan_provider::providers::ollama::OllamaProvider;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = OllamaProvider::new_default();
//! // o con una URL personalizada:
//! let provider = OllamaProvider::new("http://192.168.1.10:11434");
//! # Ok(())
//! # }
//! ```

use crate::{
    capabilities::ProviderCapabilities,
    error::{ProviderError, ProviderResult},
    message::{Message, Role},
    provider::Provider,
    request::{ChatRequest, ChatResponse, FinishReason, Usage},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_URL: &str = "http://localhost:11434";
const DEFAULT_TIMEOUT_SECS: u64 = 300; // local models can be slow

/// Adapter para Ollama.
pub struct OllamaProvider {
    base_url: String,
    client: reqwest::blocking::Client,
    name: String,
}

impl OllamaProvider {
    /// Crea un adapter apuntando a la URL por defecto
    /// (`http://localhost:11434`).
    pub fn new_default() -> Self {
        Self::new(DEFAULT_URL)
    }

    /// Crea un adapter apuntando a una URL específica.
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .expect("failed to build HTTP client");

        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client,
            name: "ollama".to_string(),
        }
    }

    /// Sobrescribe el nombre (útil para logs).
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    fn chat_endpoint(&self) -> String {
        format!("{}/api/chat", self.base_url)
    }

    fn tags_endpoint(&self) -> String {
        format!("{}/api/tags", self.base_url)
    }

    fn to_ollama_messages(messages: &[Message]) -> Vec<OllamaMessage> {
        messages
            .iter()
            .map(|m| OllamaMessage {
                role: match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::Tool => "tool",
                }
                .to_string(),
                content: m.content.clone(),
            })
            .collect()
    }
}

impl Provider for OllamaProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::ollama()
    }

    fn models(&self) -> ProviderResult<Vec<String>> {
        let url = self.tags_endpoint();
        let resp = self
            .client
            .get(&url)
            .send()
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(ProviderError::ProviderError {
                status: resp.status().as_u16(),
                message: "failed to list models from Ollama".into(),
            });
        }

        let parsed: TagsResponse = resp.json().map_err(|e| ProviderError::ProviderError {
            status: 200,
            message: format!("failed to parse tags response: {}", e),
        })?;

        Ok(parsed.models.into_iter().map(|m| m.name).collect())
    }

    fn chat(&self, request: ChatRequest) -> ProviderResult<ChatResponse> {
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

        let body = OllamaRequest {
            model: request.model.clone(),
            messages: Self::to_ollama_messages(&request.messages),
            stream: false,
            options: OllamaOptions {
                temperature: request.temperature,
                num_predict: request.max_tokens,
            },
        };

        let url = self.chat_endpoint();
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().unwrap_or_default();
            let message: String = text.chars().take(300).collect();
            return Err(ProviderError::ProviderError { status, message });
        }

        let parsed: OllamaResponse = resp.json().map_err(|e| ProviderError::ProviderError {
            status: 200,
            message: format!("failed to parse chat response: {}", e),
        })?;

        let finish_reason = if parsed.done {
            FinishReason::Stop
        } else {
            FinishReason::Unknown
        };

        Ok(ChatResponse {
            message: Message {
                role: Role::Assistant,
                content: parsed.message.content,
            },
            model: parsed.model,
            finish_reason,
            usage: if parsed.prompt_eval_count > 0 || parsed.eval_count > 0 {
                Some(Usage {
                    input_tokens: parsed.prompt_eval_count,
                    output_tokens: parsed.eval_count,
                })
            } else {
                None
            },
        })
    }
}

// ─────────────────────────────────────────────
// Tipos internos de Ollama
// ─────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    model: String,
    message: OllamaResponseMessage,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    prompt_eval_count: u32,
    #[serde(default)]
    eval_count: u32,
}

#[derive(Debug, Deserialize)]
struct OllamaResponseMessage {
    #[serde(default)]
    content: String,
}

#[derive(Debug, Deserialize)]
struct TagsResponse {
    models: Vec<TagEntry>,
}

#[derive(Debug, Deserialize)]
struct TagEntry {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_default_uses_localhost() {
        let p = OllamaProvider::new_default();
        assert_eq!(p.base_url, "http://localhost:11434");
    }

    #[test]
    fn new_trims_trailing_slash() {
        let p = OllamaProvider::new("http://example.com:11434/");
        assert_eq!(p.base_url, "http://example.com:11434");
    }

    #[test]
    fn chat_endpoint_is_correct() {
        let p = OllamaProvider::new_default();
        assert_eq!(p.chat_endpoint(), "http://localhost:11434/api/chat");
    }

    #[test]
    fn tags_endpoint_is_correct() {
        let p = OllamaProvider::new_default();
        assert_eq!(p.tags_endpoint(), "http://localhost:11434/api/tags");
    }

    #[test]
    fn default_name() {
        let p = OllamaProvider::new_default();
        assert_eq!(p.name(), "ollama");
    }

    #[test]
    fn custom_name() {
        let p = OllamaProvider::new_default().with_name("my-ollama");
        assert_eq!(p.name(), "my-ollama");
    }

    #[test]
    fn capabilities_are_ollama() {
        let p = OllamaProvider::new_default();
        let caps = p.capabilities();
        assert!(caps.native_tool_calling);
        assert!(!caps.parallel_tool_calls);
        assert!(caps.streaming);
    }

    #[test]
    fn to_ollama_messages_translates_roles() {
        let msgs = vec![
            Message::system("sys"),
            Message::user("usr"),
            Message::assistant("ast"),
        ];
        let converted = OllamaProvider::to_ollama_messages(&msgs);
        assert_eq!(converted.len(), 3);
        assert_eq!(converted[0].role, "system");
        assert_eq!(converted[1].role, "user");
        assert_eq!(converted[2].role, "assistant");
    }

    #[test]
    fn empty_model_fails_before_network() {
        let p = OllamaProvider::new_default();
        let req = ChatRequest::simple("", "hi");
        let err = p.chat(req).unwrap_err();
        assert!(matches!(err, ProviderError::InvalidRequest(_)));
    }

    #[test]
    fn empty_messages_fails_before_network() {
        let p = OllamaProvider::new_default();
        let req = ChatRequest {
            model: "m".into(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        let err = p.chat(req).unwrap_err();
        assert!(matches!(err, ProviderError::InvalidRequest(_)));
    }

    #[test]
    fn request_serializes_correctly() {
        let body = OllamaRequest {
            model: "llama3.2".into(),
            messages: vec![OllamaMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            stream: false,
            options: OllamaOptions {
                temperature: Some(0.5),
                num_predict: Some(50),
            },
        };
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["model"], "llama3.2");
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["stream"], false);
        assert_eq!(json["options"]["temperature"], 0.5);
        assert_eq!(json["options"]["num_predict"], 50);
    }

    #[test]
    fn options_omit_when_none() {
        let body = OllamaRequest {
            model: "m".into(),
            messages: vec![],
            stream: false,
            options: OllamaOptions {
                temperature: None,
                num_predict: None,
            },
        };
        let json = serde_json::to_value(&body).unwrap();
        assert!(!json["options"]
            .as_object()
            .unwrap()
            .contains_key("temperature"));
        assert!(!json["options"]
            .as_object()
            .unwrap()
            .contains_key("num_predict"));
    }

    #[test]
    fn parse_ollama_response() {
        let json = r#"{
            "model": "llama3.2",
            "message": {"role": "assistant", "content": "hi there"},
            "done": true,
            "prompt_eval_count": 10,
            "eval_count": 5
        }"#;
        let parsed: OllamaResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.model, "llama3.2");
        assert_eq!(parsed.message.content, "hi there");
        assert!(parsed.done);
        assert_eq!(parsed.prompt_eval_count, 10);
        assert_eq!(parsed.eval_count, 5);
    }
}
