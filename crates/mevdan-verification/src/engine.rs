//! `VerificationEngine` — coordina verificadores.
//!
//! El engine mantiene una lista ordenada de `Verifier`. Cuando se le
//! pide verificar un claim:
//!
//! 1. Itera los verificadores en orden.
//! 2. El primero que diga `can_verify == true` se usa.
//! 3. Si ninguno aplica, devuelve `Unknown`.

use crate::{
    claim::{Claim, ClaimStatus},
    verifier::{VerificationContext, VerificationOutcome, Verifier},
};

/// Motor de verificación.
pub struct VerificationEngine {
    verifiers: Vec<Box<dyn Verifier>>,
}

impl std::fmt::Debug for VerificationEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerificationEngine")
            .field("verifier_count", &self.verifiers.len())
            .finish()
    }
}

impl Default for VerificationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl VerificationEngine {
    /// Crea un engine vacío.
    pub fn new() -> Self {
        Self {
            verifiers: Vec::new(),
        }
    }

    /// Crea un engine con los verificadores por defecto:
    /// - `HashVerifier` (primero: es el más específico).
    /// - `FileExistsVerifier`.
    /// - `CommandExitVerifier`.
    pub fn with_defaults() -> Self {
        use crate::verifiers::{CommandExitVerifier, FileExistsVerifier, HashVerifier};
        let mut e = Self::new();
        e.register(Box::new(HashVerifier::new()));
        e.register(Box::new(FileExistsVerifier::new()));
        e.register(Box::new(CommandExitVerifier::new()));
        e
    }

    /// Registra un verificador al final de la lista.
    pub fn register(&mut self, verifier: Box<dyn Verifier>) {
        self.verifiers.push(verifier);
    }

    /// Número de verificadores.
    pub fn len(&self) -> usize {
        self.verifiers.len()
    }

    /// ¿Está vacío?
    pub fn is_empty(&self) -> bool {
        self.verifiers.is_empty()
    }

    /// Lista los nombres de los verificadores.
    pub fn verifier_names(&self) -> Vec<&str> {
        self.verifiers.iter().map(|v| v.name()).collect()
    }

    /// Verifica un claim.
    ///
    /// Devuelve el `VerificationOutcome`. Nunca falla por sí mismo: si
    /// no hay verificador aplicable, devuelve `Unknown`.
    pub fn verify(&self, claim: &Claim, ctx: &VerificationContext) -> VerificationOutcome {
        for v in &self.verifiers {
            if v.can_verify(claim) {
                return v.verify(claim, ctx);
            }
        }
        VerificationOutcome {
            status: ClaimStatus::Unknown,
            reason: "no verifier available for this claim".to_string(),
            evidence: Vec::new(),
        }
    }

    /// Verifica un claim **y actualiza su estado**.
    pub fn verify_and_update(
        &self,
        claim: &mut Claim,
        ctx: &VerificationContext,
    ) -> VerificationOutcome {
        let outcome = self.verify(claim, ctx);
        match outcome.status {
            ClaimStatus::Verified => claim.mark_verified(),
            ClaimStatus::Failed => claim.mark_failed(),
            ClaimStatus::Partial => claim.mark_partial(),
            ClaimStatus::Unknown => claim.mark_unknown(),
            ClaimStatus::Unverified => {}
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        claim::{Claim, ClaimKind},
        hash::ContentHash,
    };
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn new_engine_is_empty() {
        let e = VerificationEngine::new();
        assert!(e.is_empty());
    }

    #[test]
    fn with_defaults_has_three() {
        let e = VerificationEngine::with_defaults();
        assert_eq!(e.len(), 3);
        let names = e.verifier_names();
        assert!(names.contains(&"hash"));
        assert!(names.contains(&"file_exists"));
        assert!(names.contains(&"command_exit"));
    }

    #[test]
    fn verify_file_created() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("test.txt"), "content").unwrap();

        let e = VerificationEngine::with_defaults();
        let ctx = VerificationContext::new(dir.path());
        let claim = Claim::file_created("test.txt");

        let outcome = e.verify(&claim, &ctx);
        assert!(outcome.is_verified());
    }

    #[test]
    fn verify_file_missing() {
        let dir = TempDir::new().unwrap();
        let e = VerificationEngine::with_defaults();
        let ctx = VerificationContext::new(dir.path());
        let claim = Claim::file_created("missing.txt");

        let outcome = e.verify(&claim, &ctx);
        assert!(outcome.is_failed());
    }

    #[test]
    fn verify_hash_takes_priority() {
        let dir = TempDir::new().unwrap();
        let content = "hello";
        fs::write(dir.path().join("f.txt"), content).unwrap();

        let e = VerificationEngine::with_defaults();
        let ctx = VerificationContext::new(dir.path());

        // El claim tiene expected_hash → HashVerifier gana.
        let claim = Claim::file_created("f.txt").with_expected_hash(ContentHash::of_str(content));

        let outcome = e.verify(&claim, &ctx);
        assert!(outcome.is_verified());
        // La evidencia generada por HashVerifier tiene `hash_match`.
        assert!(outcome.evidence[0].data["hash_match"] == true);
    }

    #[test]
    fn verify_hash_mismatch() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("f.txt"), "real").unwrap();

        let e = VerificationEngine::with_defaults();
        let ctx = VerificationContext::new(dir.path());
        let claim = Claim::file_created("f.txt").with_expected_hash(ContentHash::of_str("fake"));

        let outcome = e.verify(&claim, &ctx);
        assert!(outcome.is_failed());
    }

    #[test]
    fn verify_command() {
        let dir = TempDir::new().unwrap();
        let e = VerificationEngine::with_defaults();
        let ctx = VerificationContext::new(dir.path());

        let mut claim = Claim::new(ClaimKind::CommandRan, "echo");
        claim.data = serde_json::json!({
            "program": "echo",
            "args": ["hi"],
            "exit_code": 0,
        });

        let outcome = e.verify(&claim, &ctx);
        assert!(outcome.is_verified());
    }

    #[test]
    fn verify_unknown_claim_type_returns_unknown() {
        let dir = TempDir::new().unwrap();
        let e = VerificationEngine::with_defaults();
        let ctx = VerificationContext::new(dir.path());
        let claim = Claim::new(ClaimKind::Other, "weird");

        let outcome = e.verify(&claim, &ctx);
        assert_eq!(outcome.status, ClaimStatus::Unknown);
    }

    #[test]
    fn verify_and_update_sets_claim_status() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("f.txt"), "x").unwrap();

        let e = VerificationEngine::with_defaults();
        let ctx = VerificationContext::new(dir.path());
        let mut claim = Claim::file_created("f.txt");

        assert!(claim.is_pending());
        e.verify_and_update(&mut claim, &ctx);
        assert!(claim.is_verified());
    }

    #[test]
    fn verify_and_update_on_failure() {
        let dir = TempDir::new().unwrap();
        let e = VerificationEngine::with_defaults();
        let ctx = VerificationContext::new(dir.path());
        let mut claim = Claim::file_created("missing.txt");

        e.verify_and_update(&mut claim, &ctx);
        assert!(claim.is_failed());
    }
}
