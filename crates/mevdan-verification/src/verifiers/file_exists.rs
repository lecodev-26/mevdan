//! Verificador: comprueba que un archivo existe.

use crate::{
    claim::{Claim, ClaimKind},
    evidence::Evidence,
    verifier::{VerificationContext, VerificationOutcome, Verifier},
};

/// Verifica claims de tipo `FileCreated` / `FileModified` comprobando
/// que el archivo existe.
#[derive(Debug)]
pub struct FileExistsVerifier;

impl FileExistsVerifier {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileExistsVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Verifier for FileExistsVerifier {
    fn name(&self) -> &str {
        "file_exists"
    }

    fn can_verify(&self, claim: &Claim) -> bool {
        matches!(claim.kind, ClaimKind::FileCreated | ClaimKind::FileModified)
    }

    fn verify(&self, claim: &Claim, ctx: &VerificationContext) -> VerificationOutcome {
        let path = match claim.data.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => {
                return VerificationOutcome::unknown("claim does not specify a path");
            }
        };

        let resolved = ctx.resolve(path);

        if resolved.exists() {
            let ev = Evidence::file_exists(path);
            VerificationOutcome::verified(format!("file exists: {}", path)).with_evidence(ev)
        } else {
            VerificationOutcome::failed(format!("file does not exist: {}", path))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn can_verify_file_created() {
        let v = FileExistsVerifier::new();
        assert!(v.can_verify(&Claim::file_created("x")));
        assert!(v.can_verify(&Claim::new(ClaimKind::FileModified, "x")));
        assert!(!v.can_verify(&Claim::new(ClaimKind::Other, "x")));
        assert!(!v.can_verify(&Claim::command_ran("cargo", 0)));
    }

    #[test]
    fn verify_existing_file() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("test.txt"), "content").unwrap();

        let v = FileExistsVerifier::new();
        let ctx = VerificationContext::new(dir.path());
        let claim = Claim::file_created("test.txt");

        let outcome = v.verify(&claim, &ctx);
        assert!(outcome.is_verified());
        assert_eq!(outcome.evidence.len(), 1);
    }

    #[test]
    fn verify_missing_file() {
        let dir = TempDir::new().unwrap();
        let v = FileExistsVerifier::new();
        let ctx = VerificationContext::new(dir.path());
        let claim = Claim::file_created("nonexistent.txt");

        let outcome = v.verify(&claim, &ctx);
        assert!(outcome.is_failed());
    }

    #[test]
    fn verify_claim_without_path_is_unknown() {
        let dir = TempDir::new().unwrap();
        let v = FileExistsVerifier::new();
        let ctx = VerificationContext::new(dir.path());

        // Claim de tipo FileCreated sin path en data.
        let mut claim = Claim::new(ClaimKind::FileCreated, "x");
        claim.data = serde_json::json!({});

        let outcome = v.verify(&claim, &ctx);
        assert_eq!(outcome.status, crate::claim::ClaimStatus::Unknown);
    }

    #[test]
    fn verify_absolute_path_resolves_to_sandbox() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("abs.txt"), "content").unwrap();

        let v = FileExistsVerifier::new();
        let ctx = VerificationContext::new(dir.path());
        let claim = Claim::file_created("/abs.txt");

        let outcome = v.verify(&claim, &ctx);
        assert!(outcome.is_verified());
    }
}
