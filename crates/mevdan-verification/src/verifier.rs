//! Contrato de verificación.
//!
//! Un `Verifier` sabe comprobar un tipo concreto de claim. El engine
//! le pregunta si puede verificar un claim, y si dice que sí, le pide
//! la verificación.

use crate::{
    claim::{Claim, ClaimStatus},
    evidence::Evidence,
};
use std::path::PathBuf;

/// Contexto para verificar.
#[derive(Debug, Clone)]
pub struct VerificationContext {
    /// Directorio raíz del sandbox. Todos los paths se resuelven
    /// relativos a esto.
    pub sandbox_root: PathBuf,
}

impl VerificationContext {
    pub fn new(sandbox_root: impl Into<PathBuf>) -> Self {
        Self {
            sandbox_root: sandbox_root.into(),
        }
    }

    /// Resuelve un path relativo al sandbox.
    pub fn resolve(&self, path: &str) -> PathBuf {
        let p = PathBuf::from(path);
        if p.is_absolute() {
            self.sandbox_root.join(path.trim_start_matches('/'))
        } else {
            self.sandbox_root.join(p)
        }
    }
}

/// Resultado de una verificación.
#[derive(Debug, Clone)]
pub struct VerificationOutcome {
    /// Nuevo estado del claim.
    pub status: ClaimStatus,
    /// Razón legible.
    pub reason: String,
    /// Evidencias generadas durante la verificación.
    pub evidence: Vec<Evidence>,
}

impl VerificationOutcome {
    /// Claim verificado con éxito.
    pub fn verified(reason: impl Into<String>) -> Self {
        Self {
            status: ClaimStatus::Verified,
            reason: reason.into(),
            evidence: Vec::new(),
        }
    }

    /// Claim fallido.
    pub fn failed(reason: impl Into<String>) -> Self {
        Self {
            status: ClaimStatus::Failed,
            reason: reason.into(),
            evidence: Vec::new(),
        }
    }

    /// Claim parcialmente verificado.
    pub fn partial(reason: impl Into<String>) -> Self {
        Self {
            status: ClaimStatus::Partial,
            reason: reason.into(),
            evidence: Vec::new(),
        }
    }

    /// No se puede verificar.
    pub fn unknown(reason: impl Into<String>) -> Self {
        Self {
            status: ClaimStatus::Unknown,
            reason: reason.into(),
            evidence: Vec::new(),
        }
    }

    /// Añade una evidencia.
    pub fn with_evidence(mut self, e: Evidence) -> Self {
        self.evidence.push(e);
        self
    }

    /// ¿Es verificado?
    pub fn is_verified(&self) -> bool {
        self.status == ClaimStatus::Verified
    }

    /// ¿Falló?
    pub fn is_failed(&self) -> bool {
        self.status == ClaimStatus::Failed
    }
}

/// Un verificador.
///
/// Implementaciones concretas viven en `crate::verifiers`.
pub trait Verifier: Send + Sync + std::fmt::Debug {
    /// Nombre corto del verificador (para logs).
    fn name(&self) -> &str;

    /// ¿Puede verificar este claim?
    fn can_verify(&self, claim: &Claim) -> bool;

    /// Verifica el claim.
    fn verify(&self, claim: &Claim, ctx: &VerificationContext) -> VerificationOutcome;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claim::{Claim, ClaimKind};

    #[test]
    fn context_resolves_relative() {
        let ctx = VerificationContext::new("/root");
        let p = ctx.resolve("src/main.rs");
        assert_eq!(p, PathBuf::from("/root/src/main.rs"));
    }

    #[test]
    fn context_resolves_absolute_as_relative() {
        let ctx = VerificationContext::new("/root");
        let p = ctx.resolve("/src/main.rs");
        assert_eq!(p, PathBuf::from("/root/src/main.rs"));
    }

    #[test]
    fn outcome_verified_works() {
        let o = VerificationOutcome::verified("ok");
        assert!(o.is_verified());
        assert!(!o.is_failed());
        assert_eq!(o.reason, "ok");
    }

    #[test]
    fn outcome_failed_works() {
        let o = VerificationOutcome::failed("nope");
        assert!(o.is_failed());
        assert_eq!(o.reason, "nope");
    }

    #[test]
    fn outcome_partial_works() {
        let o = VerificationOutcome::partial("some");
        assert_eq!(o.status, ClaimStatus::Partial);
    }

    #[test]
    fn outcome_unknown_works() {
        let o = VerificationOutcome::unknown("can't tell");
        assert_eq!(o.status, ClaimStatus::Unknown);
    }

    #[test]
    fn outcome_with_evidence_appends() {
        let e = Evidence::file_exists("/x");
        let o = VerificationOutcome::verified("ok").with_evidence(e);
        assert_eq!(o.evidence.len(), 1);
    }

    // Un verificador de test para comprobar el trait.
    #[derive(Debug)]
    struct AlwaysVerifier;

    impl Verifier for AlwaysVerifier {
        fn name(&self) -> &str {
            "always"
        }

        fn can_verify(&self, _claim: &Claim) -> bool {
            true
        }

        fn verify(&self, _claim: &Claim, _ctx: &VerificationContext) -> VerificationOutcome {
            VerificationOutcome::verified("always yes")
        }
    }

    #[test]
    fn custom_verifier_works() {
        let v = AlwaysVerifier;
        let claim = Claim::new(ClaimKind::Other, "x");
        let ctx = VerificationContext::new("/tmp");
        assert!(v.can_verify(&claim));
        let outcome = v.verify(&claim, &ctx);
        assert!(outcome.is_verified());
    }
}
