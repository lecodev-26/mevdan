//! `mevdan chat <message>` — envía un mensaje a un provider.
//!
//! Comandos disponibles:
//! - `--provider openai-compatible` (por defecto): usa una API
//!   compatible con OpenAI. Necesita `--base-url` y una API key
//!   guardada en el store de secretos.
//! - `--provider ollama`: usa el servidor local de Ollama.
//!
//! Ejemplos:
//!
//! ```bash
//! # Ollama local
//! mevdan chat "Hola" -m llama3.2
//!
//! # OpenAI
//! mevdan chat "Hola" -m gpt-4o-mini \
//!     --provider openai-compatible \
//!     --base-url https://api.openai.com/v1 \
//!     --api-key-secret openai_api_key
//! ```

use mevdan_provider::{
    providers::{ollama::OllamaProvider, openai_compat::OpenAiCompatibleProvider},
    ChatRequest, Provider,
};
use mevdan_secrets::{FileSecretStore, SecretStore};

pub fn run(
    message: &str,
    provider_name: &str,
    model: &str,
    base_url: Option<&str>,
    api_key_secret: &str,
) -> anyhow::Result<()> {
    let provider: Box<dyn Provider> = match provider_name {
        "ollama" => {
            let url = base_url.unwrap_or("http://localhost:11434");
            Box::new(OllamaProvider::new(url))
        }
        "openai-compatible" => {
            let url = base_url.ok_or_else(|| {
                anyhow::anyhow!(
                    "--base-url is required for provider 'openai-compatible' \
                     (e.g. https://api.openai.com/v1)"
                )
            })?;

            // Read API key from secrets store.
            let store = FileSecretStore::open_default()?;
            let api_key = store.get(api_key_secret).map_err(|_| {
                anyhow::anyhow!(
                    "API key not found in secrets store (name: {}).\n\
                     Note: adding secrets via CLI will be available in a future version.",
                    api_key_secret
                )
            })?;

            Box::new(OpenAiCompatibleProvider::new(url, api_key))
        }
        other => {
            anyhow::bail!(
                "unknown provider '{}'. Supported: ollama, openai-compatible",
                other
            );
        }
    };

    let req = ChatRequest::simple(model, message);
    let response = provider.chat(req)?;

    println!("{}", response.message.content);
    if let Some(usage) = response.usage {
        eprintln!(
            "\n[tokens: {} in / {} out = {} total]",
            usage.input_tokens,
            usage.output_tokens,
            usage.total()
        );
    }

    Ok(())
}
