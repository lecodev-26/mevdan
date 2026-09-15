//! Verificador: compara el hash real de un archivo con el esperado.

use crate::{
    claim::Claim,
    evidence::Evidence,
    hash::ContentHash,
    verifier::{VerificationContext, VerificationOutcome, Verifier},
};
use std::fs;

/// Verifica claims que tengan un `expected_hash` comparándolo con el
/// hash real del archivo referenciado.
#[derive(Debug)]
pub struct HashVerifier;

impl HashVerifier {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HashVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Verifier for HashVerifier {
    fn name(&self) -> &str {
        "hash"
    }

    fn can_verify(&self, claim: &Claim) -> bool {
        claim.expected_hash.is_some()
    }

    fn verify(&self, claim: &Claim, ctx: &VerificationContext) -> VerificationOutcome {
        // 1. Debe tener un hash esperado.
        let expected = match &claim.expected_hash {
            Some(h) => h,
            None => {
                return VerificationOutcome::unknown("no expected_hash");
            }
        };

        // 2. Debe tener un path en data.
        let path = match claim.data.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => {
                return VerificationOutcome::unknown("claim does not specify a path");
            }
        };

        // 3. El archivo debe existir.
        let resolved = ctx.resolve(path);
        if !resolved.exists() {
            return VerificationOutcome::failed(format!("file does not exist: {}", path));
        }
        if resolved.is_dir() {
            return VerificationOutcome::failed(format!("path is a directory: {}", path));
        }

        // 4. Leer bytes y calcular hash.
        let bytes = match fs::read(&resolved) {
            Ok(b) => b,
            Err(e) => {
                return VerificationOutcome::unknown(format!("failed to read file: {}", e));
            }
        };
        let actual = ContentHash::of_bytes(&bytes);

        // 5. Comparar.
        if actual.matches(expected) {
            let ev = Evidence::file_exists(path)
                .with_hash(actual)
                .with_data(serde_json::json!({
                    "path": path,
                    "hash_match": true,
                }));
            VerificationOutcome::verified(format!("hash matches for {}", path)).with_evidence(ev)
        } else {
            VerificationOutcome::failed(format!(
                "hash mismatch for {}: expected {}, got {}",
                path,
                expected.as_hex(),
                actual.as_hex(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claim::ClaimKind;
    use tempfile::TempDir;

    #[test]
    fn can_verify_claim_with_expected_hash() {
        let v = HashVerifier::new();
        let h = ContentHash::of_str("x");
        assert!(v.can_verify(&Claim::new(ClaimKind::Other, "x").with_expected_hash(h)));
    }

    #[test]
    fn cannot_verify_claim_without_expected_hash() {
        let v = HashVerifier::new();
        assert!(!v.can_verify(&Claim::new(ClaimKind::Other, "x")));
    }

    #[test]
    fn verify_hash_matches() {
        let dir = TempDir::new().unwrap();
        let content = "hello world";
        fs::write(dir.path().join("test.txt"), content).unwrap();

        let v = HashVerifier::new();
        let ctx = VerificationContext::new(dir.path());
        let claim =
            Claim::file_created("test.txt").with_expected_hash(ContentHash::of_str(content));

        let outcome = v.verify(&claim, &ctx);
        assert!(outcome.is_verified());
    }

    #[test]
    fn verify_hash_mismatch() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("test.txt"), "actual content").unwrap();

        let v = HashVerifier::new();
        let ctx = VerificationContext::new(dir.path());
        let claim = Claim::file_created("test.txt")
            .with_expected_hash(ContentHash::of_str("different content"));

        let outcome = v.verify(&claim, &ctx);
        assert!(outcome.is_failed());
        assert!(outcome.reason.contains("hash mismatch"));
    }

    #[test]
    fn verify_missing_file() {
        let dir = TempDir::new().unwrap();
        let v = HashVerifier::new();
        let ctx = VerificationContext::new(dir.path());
        let claim =
            Claim::file_created("nonexistent.txt").with_expected_hash(ContentHash::of_str("x"));

        let outcome = v.verify(&claim, &ctx);
        assert!(outcome.is_failed());
    }

    #[test]
    fn verify_directory_fails() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("subdir")).unwrap();

        let v = HashVerifier::new();
        let ctx = VerificationContext::new(dir.path());
        let claim = Claim::file_created("subdir").with_expected_hash(ContentHash::of_str("x"));

        let outcome = v.verify(&claim, &ctx);
        assert!(outcome.is_failed());
        assert!(outcome.reason.contains("directory"));
    }
}
