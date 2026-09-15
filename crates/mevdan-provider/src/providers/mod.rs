//! Adapters de providers concretos.
//!
//! Cada adapter implementa el trait `Provider` de `crate::provider`.
//! Los adapters NO exponen tipos de vendor hacia fuera: traducen
//! internamente y devuelven los tipos neutros de MEVDAN.
//!
//! ## Adapters disponibles
//!
//! - `openai_compat` — cualquier API compatible con OpenAI
//!   (OpenAI, DeepSeek, OpenRouter, Groq, Together, LM Studio, vLLM,
//!   LocalAI, Jan, etc.).
//! - `ollama` — servidor local de Ollama.
//!
//! ## Adapters futuros
//!
//! - `anthropic` — API de Anthropic (formato distinto).
//! - `google` — API de Google Gemini (formato distinto).

pub mod ollama;
pub mod openai_compat;
