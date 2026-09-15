//! Presupuesto de tokens y estimación.
//!
//! Sin tokenizer real (eso vendrá cuando un provider exponga uno).
//! Usamos una heurística estándar: ~4 bytes por token en inglés.
//! Es una aproximación aceptable para planificar el contexto.
//!
//! **Importante:** el presupuesto aquí es solo para el contexto
//! (system + project + task + history + files). El espacio para la
//! respuesta del modelo se reserva aparte.

use serde::{Deserialize, Serialize};

/// Ratio de bytes por token aproximado.
///
/// 4.0 es la estimación estándar para inglés. Para código y otros
/// idiomas puede ser distinto, pero es suficientemente bueno para
/// presupuestar.
pub const BYTES_PER_TOKEN: f32 = 4.0;

/// Estima los tokens de un texto.
///
/// Devuelve al menos 1 si el texto no está vacío.
pub fn estimate_tokens(text: &str) -> u32 {
    if text.is_empty() {
        return 0;
    }
    let bytes = text.len() as f32;
    let tokens = (bytes / BYTES_PER_TOKEN).ceil() as u32;
    tokens.max(1)
}

/// Presupuesto de tokens para un contexto.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TokenBudget {
    /// Tokens máximos para el contexto completo.
    pub max_tokens: u32,

    /// Tokens reservados para la respuesta del modelo.
    pub reserve_for_response: u32,
}

impl TokenBudget {
    /// Presupuesto por defecto: 8K contexto, 2K respuesta.
    pub fn default_budget() -> Self {
        Self {
            max_tokens: 8_000,
            reserve_for_response: 2_000,
        }
    }

    /// Presupuesto pequeño: 2K contexto, 1K respuesta.
    pub fn small() -> Self {
        Self {
            max_tokens: 2_000,
            reserve_for_response: 1_000,
        }
    }

    /// Presupuesto grande: 32K contexto, 4K respuesta.
    pub fn large() -> Self {
        Self {
            max_tokens: 32_000,
            reserve_for_response: 4_000,
        }
    }

    /// Tokens disponibles realmente para el contexto.
    pub fn available(&self) -> u32 {
        self.max_tokens.saturating_sub(self.reserve_for_response)
    }
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self::default_budget()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_empty_is_zero() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn estimate_short_text() {
        assert_eq!(estimate_tokens("hi"), 1);
    }

    #[test]
    fn estimate_typical_text() {
        // 4 chars → 1 token (con BYTES_PER_TOKEN=4).
        assert_eq!(estimate_tokens("abcd"), 1);
        // 8 chars → 2 tokens.
        assert_eq!(estimate_tokens("abcdefgh"), 2);
        // 100 chars → 25 tokens.
        let s = "a".repeat(100);
        assert_eq!(estimate_tokens(&s), 25);
    }

    #[test]
    fn estimate_non_ascii_uses_bytes() {
        // 'á' es 2 bytes en UTF-8.
        assert_eq!(estimate_tokens("á"), 1); // ceil(2/4) = 1
        assert_eq!(estimate_tokens("áááá"), 2); // ceil(8/4) = 2
    }

    #[test]
    fn default_budget_has_reserve() {
        let b = TokenBudget::default_budget();
        assert_eq!(b.max_tokens, 8_000);
        assert_eq!(b.reserve_for_response, 2_000);
        assert_eq!(b.available(), 6_000);
    }

    #[test]
    fn small_budget() {
        let b = TokenBudget::small();
        assert_eq!(b.available(), 1_000);
    }

    #[test]
    fn large_budget() {
        let b = TokenBudget::large();
        assert_eq!(b.available(), 28_000);
    }

    #[test]
    fn available_never_underflows() {
        let b = TokenBudget {
            max_tokens: 100,
            reserve_for_response: 500,
        };
        assert_eq!(b.available(), 0);
    }

    #[test]
    fn budget_roundtrips() {
        let b = TokenBudget::default_budget();
        let json = serde_json::to_string(&b).unwrap();
        let back: TokenBudget = serde_json::from_str(&json).unwrap();
        assert_eq!(back.max_tokens, b.max_tokens);
        assert_eq!(back.reserve_for_response, b.reserve_for_response);
    }
}
