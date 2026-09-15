//! Adapter para APIs compatibles con OpenAI.
//!
//! Cubre: OpenAI, DeepSeek, OpenRouter, Groq, Together, Fireworks,
//! LM Studio, vLLM, LocalAI, Jan, y cualquier otro servidor que
//! implemente `POST /v1/chat/completions`.

use crate::{
    capabilities::ProviderCapabilities,
    error::{ProviderError, ProviderResult},
    message::{Message, Role},
    provider::Provider,
    request::{ChatRequest, ChatResponse, FinishReason, Usage},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Timeout por defecto para llamadas HTTP.
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Adapter para APIs compatibles con OpenAI.
pub struct OpenAiCompatibleProvider {
    base_url: String,
    api_key: String,
    client: reqwest::blocking::Client,
    name: String,
}

impl OpenAiCompatibleProvider {
    /// Crea un adapter apuntando a `base_url` (debe incluir `/v1`
    /// cuando corresponda).
    ///
    /// Ejemplos de `base_url`:
    /// - OpenAI: `https://api.openai.com/v1`
    /// - DeepSeek: `https://api.deepseek.com/v1`
    /// - OpenRouter: `https://openrouter.ai/api/v1`
    /// - LM Studio local: `http://localhost:1234/v1`
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .expect("failed to build HTTP client");

        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            client,
            name: "openai-compatible".to_string(),
        }
    }

    /// Sobrescribe el nombre del provider (útil para logs).
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Construye el endpoint de chat.
    fn chat_endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }

    /// Construye el endpoint de listado de modelos.
    fn models_endpoint(&self) -> String {
        format!("{}/models", self.base_url)
    }

    /// Convierte mensajes de MEVDAN al formato OpenAI.
    fn to_openai_messages(messages: &[Message]) -> Vec<OpenAiMessage> {
        messages
            .iter()
            .map(|m| OpenAiMessage {
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

    /// Traduce la respuesta de OpenAI a los tipos de MEVDAN.
    fn from_openai_response(resp: OpenAiResponse) -> ProviderResult<ChatResponse> {
        let choice =
            resp.choices
                .into_iter()
                .next()
                .ok_or_else(|| ProviderError::ProviderError {
                    status: 200,
                    message: "no choices in response".into(),
                })?;

        let role = match choice.message.role.as_str() {
            "assistant" => Role::Assistant,
            "user" => Role::User,
            "system" => Role::System,
            "tool" => Role::Tool,
            _ => Role::Assistant,
        };

        let finish_reason = match choice.finish_reason.as_deref() {
            Some("stop") => FinishReason::Stop,
            Some("length") => FinishReason::Length,
            Some("tool_calls") => FinishReason::ToolCalls,
            Some("content_filter") => FinishReason::ContentFilter,
            _ => FinishReason::Unknown,
        };

        Ok(ChatResponse {
            message: Message {
                role,
                content: choice.message.content.unwrap_or_default(),
            },
            model: resp.model,
            finish_reason,
            usage: resp.usage.map(|u| Usage {
                input_tokens: u.prompt_tokens,
                output_tokens: u.completion_tokens,
            }),
        })
    }
}

impl Provider for OpenAiCompatibleProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::openai_compatible()
    }

    fn models(&self) -> ProviderResult<Vec<String>> {
        let url = self.models_endpoint();
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().unwrap_or_default();
            return Err(ProviderError::ProviderError {
                status,
                message: text.chars().take(200).collect(),
            });
        }

        let parsed: ModelsResponse = resp.json().map_err(|e| ProviderError::ProviderError {
            status: 200,
            message: format!("failed to parse models response: {}", e),
        })?;

        Ok(parsed.data.into_iter().map(|m| m.id).collect())
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

        let body = OpenAiRequest {
            model: request.model.clone(),
            messages: Self::to_openai_messages(&request.messages),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: false, // streaming not implemented yet
        };

        let url = self.chat_endpoint();
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let code = status.as_u16();
            let text = resp.text().unwrap_or_default();
            // Never leak API key. Truncate response body.
            let message: String = text.chars().take(300).collect();
            if code == 401 || code == 403 {
                return Err(ProviderError::AuthFailed(self.name.clone()));
            }
            if code == 429 {
                return Err(ProviderError::RateLimited {
                    retry_after_secs: 60,
                });
            }
            return Err(ProviderError::ProviderError {
                status: code,
                message,
            });
        }

        let parsed: OpenAiResponse = resp.json().map_err(|e| ProviderError::ProviderError {
            status: 200,
            message: format!("failed to parse response: {}", e),
        })?;

        Self::from_openai_response(parsed)
    }
}

// ─────────────────────────────────────────────
// Tipos internos del protocolo OpenAI
// ─────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    model: String,
    choices: Vec<OpenAiChoice>,
    #[serde(default)]
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponseMessage {
    #[serde(default = "default_role")]
    role: String,
    #[serde(default)]
    content: Option<String>,
}

fn default_role() -> String {
    "assistant".to_string()
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_trims_trailing_slash() {
        let p = OpenAiCompatibleProvider::new("https://api.example.com/v1/", "key");
        assert_eq!(p.base_url, "https://api.example.com/v1");
    }

    #[test]
    fn chat_endpoint_is_correct() {
        let p = OpenAiCompatibleProvider::new("https://api.example.com/v1", "key");
        assert_eq!(
            p.chat_endpoint(),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn models_endpoint_is_correct() {
        let p = OpenAiCompatibleProvider::new("https://api.example.com/v1", "key");
        assert_eq!(p.models_endpoint(), "https://api.example.com/v1/models");
    }

    #[test]
    fn default_name() {
        let p = OpenAiCompatibleProvider::new("https://x.com/v1", "k");
        assert_eq!(p.name(), "openai-compatible");
    }

    #[test]
    fn custom_name() {
        let p = OpenAiCompatibleProvider::new("https://x.com/v1", "k").with_name("my-openai");
        assert_eq!(p.name(), "my-openai");
    }

    #[test]
    fn capabilities_are_openai_compatible() {
        let p = OpenAiCompatibleProvider::new("https://x.com/v1", "k");
        let caps = p.capabilities();
        assert!(caps.native_tool_calling);
        assert!(caps.streaming);
        assert!(caps.vision);
    }

    #[test]
    fn to_openai_messages_translates_roles() {
        let msgs = vec![
            Message::system("sys"),
            Message::user("usr"),
            Message::assistant("ast"),
        ];
        let converted = OpenAiCompatibleProvider::to_openai_messages(&msgs);
        assert_eq!(converted.len(), 3);
        assert_eq!(converted[0].role, "system");
        assert_eq!(converted[1].role, "user");
        assert_eq!(converted[2].role, "assistant");
        assert_eq!(converted[1].content, "usr");
    }

    #[test]
    fn empty_model_fails_before_network() {
        let p = OpenAiCompatibleProvider::new("https://x.com/v1", "k");
        let req = ChatRequest::simple("", "hi");
        let err = p.chat(req).unwrap_err();
        assert!(matches!(err, ProviderError::InvalidRequest(_)));
    }

    #[test]
    fn empty_messages_fails_before_network() {
        let p = OpenAiCompatibleProvider::new("https://x.com/v1", "k");
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
    fn parse_openai_response_ok() {
        let json = r#"{
            "model": "gpt-4o-mini",
            "choices": [{
                "message": {"role": "assistant", "content": "hi there"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 5, "completion_tokens": 3}
        }"#;
        let parsed: OpenAiResponse = serde_json::from_str(json).unwrap();
        let resp = OpenAiCompatibleProvider::from_openai_response(parsed).unwrap();
        assert_eq!(resp.model, "gpt-4o-mini");
        assert_eq!(resp.message.content, "hi there");
        assert_eq!(resp.finish_reason, FinishReason::Stop);
        assert_eq!(resp.usage.unwrap().total(), 8);
    }

    #[test]
    fn parse_openai_response_without_usage() {
        let json = r#"{
            "model": "m",
            "choices": [{"message": {"role": "assistant", "content": "x"}}]
        }"#;
        let parsed: OpenAiResponse = serde_json::from_str(json).unwrap();
        let resp = OpenAiCompatibleProvider::from_openai_response(parsed).unwrap();
        assert!(resp.usage.is_none());
        assert_eq!(resp.finish_reason, FinishReason::Unknown);
    }

    #[test]
    fn parse_openai_response_empty_choices_fails() {
        let json = r#"{"model": "m", "choices": []}"#;
        let parsed: OpenAiResponse = serde_json::from_str(json).unwrap();
        let err = OpenAiCompatibleProvider::from_openai_response(parsed).unwrap_err();
        assert!(matches!(err, ProviderError::ProviderError { .. }));
    }

    #[test]
    fn request_serializes_correctly() {
        let body = OpenAiRequest {
            model: "gpt-4o-mini".into(),
            messages: vec![OpenAiMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: false,
        };
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["model"], "gpt-4o-mini");
        assert_eq!(json["messages"][0]["role"], "user");
        // f32 → f64 en JSON introduce imprecisión; comparar con tolerancia.
        let temp = json["temperature"].as_f64().unwrap();
        assert!((temp - 0.7).abs() < 1e-6, "expected ~0.7, got {}", temp);
        assert_eq!(json["max_tokens"], 100);
        assert_eq!(json["stream"], false);
    }

    #[test]
    fn request_omits_optional_fields_when_none() {
        let body = OpenAiRequest {
            model: "m".into(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        let json = serde_json::to_value(&body).unwrap();
        assert!(!json.as_object().unwrap().contains_key("temperature"));
        assert!(!json.as_object().unwrap().contains_key("max_tokens"));
    }
}
