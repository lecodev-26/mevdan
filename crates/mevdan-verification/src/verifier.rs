//! Contrato de verificación.

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
    ///
    /// Reglas:
    /// - Cualquier path con `/` inicial se interpreta como relativo
    ///   al sandbox (cross-platform, incluye Windows).
    /// - Paths absolutos con prefijo de disco (`C:\`) también se
    ///   interpretan como relativos, cogiendo solo los componentes
    ///   `Normal`.
    /// - Paths relativos se juntan con el sandbox.
    pub fn resolve(&self, path: &str) -> PathBuf {
        // Caso 1: empieza por `/` → relativo al sandbox (Unix y Windows).
        if let Some(stripped) = path.strip_prefix('/') {
            return self.sandbox_root.join(stripped);
        }

        // Caso 2: path absoluto con prefijo de disco (Windows `C:\`).
        let p = PathBuf::from(path);
        if p.is_absolute() {
            let normal_only: PathBuf = p
                .components()
                .filter_map(|c| match c {
                    std::path::Component::Normal(s) => Some(s),
                    _ => None,
                })
                .collect();
            return self.sandbox_root.join(normal_only);
        }

        // Caso 3: relativo normal.
        self.sandbox_root.join(p)
    }
}

/// Resultado de una verificación.
#[derive(Debug, Clone)]
pub struct VerificationOutcome {
    pub status: ClaimStatus,
    pub reason: String,
    pub evidence: Vec<Evidence>,
}

impl VerificationOutcome {
    pub fn verified(reason: impl Into<String>) -> Self {
        Self {
            status: ClaimStatus::Verified,
            reason: reason.into(),
            evidence: Vec::new(),
        }
    }

    pub fn failed(reason: impl Into<String>) -> Self {
        Self {
            status: ClaimStatus::Failed,
            reason: reason.into(),
            evidence: Vec::new(),
        }
    }

    pub fn partial(reason: impl Into<String>) -> Self {
        Self {
            status: ClaimStatus::Partial,
            reason: reason.into(),
            evidence: Vec::new(),
        }
    }

    pub fn unknown(reason: impl Into<String>) -> Self {
        Self {
            status: ClaimStatus::Unknown,
            reason: reason.into(),
            evidence: Vec::new(),
        }
    }

    pub fn with_evidence(mut self, e: Evidence) -> Self {
        self.evidence.push(e);
        self
    }

    pub fn is_verified(&self) -> bool {
        self.status == ClaimStatus::Verified
    }

    pub fn is_failed(&self) -> bool {
        self.status == ClaimStatus::Failed
    }
}

/// Un verificador.
pub trait Verifier: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    fn can_verify(&self, claim: &Claim) -> bool;
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
