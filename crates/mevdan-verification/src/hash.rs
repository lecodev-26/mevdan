//! Hash de contenido (SHA-256).
//!
//! Se usa para verificar integridad de artefactos y evidencia.
//! El hash se guarda como string hexadecimal de 64 caracteres.

use crate::error::{VerificationError, VerificationResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Hash SHA-256 de un contenido.
///
/// Se guarda como hex string (64 caracteres: 32 bytes en hex).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContentHash(pub String);

impl ContentHash {
    /// Calcula el hash de un slice de bytes.
    pub fn of_bytes(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        Self(hex_encode(&result))
    }

    /// Calcula el hash de un string.
    pub fn of_str(s: &str) -> Self {
        Self::of_bytes(s.as_bytes())
    }

    /// Parsea un hash desde hex string. Falla si no es hex de 64 chars.
    pub fn from_hex(s: impl Into<String>) -> VerificationResult<Self> {
        let s = s.into();
        if s.len() != 64 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(VerificationError::InvalidHash(s));
        }
        Ok(Self(s.to_lowercase()))
    }

    /// Devuelve el hash como hex string.
    pub fn as_hex(&self) -> &str {
        &self.0
    }

    /// Compara dos hashes en tiempo constante (evita timing attacks).
    pub fn matches(&self, other: &ContentHash) -> bool {
        if self.0.len() != other.0.len() {
            return false;
        }
        let mut diff = 0u8;
        for (a, b) in self.0.bytes().zip(other.0.bytes()) {
            diff |= a ^ b;
        }
        diff == 0
    }
}

/// Codifica bytes a hex string.
fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_bytes_hash() {
        // SHA-256 de cadena vacía es un valor conocido.
        let h = ContentHash::of_bytes(&[]);
        assert_eq!(
            h.as_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn hello_world_hash() {
        // SHA-256 de "hello world".
        let h = ContentHash::of_str("hello world");
        assert_eq!(
            h.as_hex(),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn same_content_same_hash() {
        let a = ContentHash::of_str("test");
        let b = ContentHash::of_str("test");
        assert_eq!(a, b);
    }

    #[test]
    fn different_content_different_hash() {
        let a = ContentHash::of_str("test1");
        let b = ContentHash::of_str("test2");
        assert_ne!(a, b);
    }

    #[test]
    fn from_hex_valid() {
        let h = ContentHash::of_str("x");
        let parsed = ContentHash::from_hex(h.as_hex()).unwrap();
        assert_eq!(parsed, h);
    }

    #[test]
    fn from_hex_rejects_short() {
        let err = ContentHash::from_hex("abcd").unwrap_err();
        assert!(matches!(err, VerificationError::InvalidHash(_)));
    }

    #[test]
    fn from_hex_rejects_non_hex() {
        let bad = "z".repeat(64);
        let err = ContentHash::from_hex(bad).unwrap_err();
        assert!(matches!(err, VerificationError::InvalidHash(_)));
    }

    #[test]
    fn from_hex_normalizes_to_lowercase() {
        let upper = "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855";
        let parsed = ContentHash::from_hex(upper).unwrap();
        assert_eq!(
            parsed.as_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn matches_returns_true_for_equal() {
        let a = ContentHash::of_str("x");
        let b = ContentHash::of_str("x");
        assert!(a.matches(&b));
    }

    #[test]
    fn matches_returns_false_for_different() {
        let a = ContentHash::of_str("x");
        let b = ContentHash::of_str("y");
        assert!(!a.matches(&b));
    }

    #[test]
    fn hash_roundtrips() {
        let h = ContentHash::of_str("data");
        let json = serde_json::to_string(&h).unwrap();
        // Se serializa como string plano.
        assert!(json.starts_with('"'));
        let back: ContentHash = serde_json::from_str(&json).unwrap();
        assert_eq!(back, h);
    }
}
