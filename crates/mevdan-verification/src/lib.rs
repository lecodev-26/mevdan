//! # mevdan-verification
//!
//! Verificación de trabajo en MEVDAN.
//!
//! ## Concepto
//!
//! **"Verification over claims."**
//!
//! Cuando un agente dice que ha hecho algo, eso es un **claim**. No es
//! prueba. MEVDAN tiene que **verificar**.
//!
//! ## Componentes
//!
//! - **`Artifact`** — resultado concreto producido (con hash SHA-256).
//! - **`Evidence`** — prueba concreta de que algo se hizo.
//! - **`Claim`** — afirmación del agente que debe ser verificada.
//! - **`Verifier`** — sabe comprobar un tipo de claim.
//! - **`VerificationEngine`** — coordina verificadores.
//!
//! ## Estado del proyecto
//!
//! - **V3.2** ✅ — tipos base: `Artifact`, `Evidence`, `Claim`, `ContentHash`.
//! - **V3.3** ✅ — `Verifier`, `VerificationEngine`, 3 verificadores.
//! - **V3.4** ⏳ — checkpoints.
//!
//! ## Uso
//!
//! ```no_run
//! use mevdan_verification::{
//!     Claim, VerificationContext, VerificationEngine,
//! };
//!
//! let engine = VerificationEngine::with_defaults();
//! let ctx = VerificationContext::new("/path/to/project");
//!
//! let claim = Claim::file_created("src/main.rs");
//! let outcome = engine.verify(&claim, &ctx);
//!
//! if outcome.is_verified() {
//!     println!("Verified: {}", outcome.reason);
//! } else {
//!     println!("Not verified: {}", outcome.reason);
//! }
//! ```

pub mod artifact;
pub mod claim;
pub mod engine;
pub mod error;
pub mod evidence;
pub mod hash;
pub mod verifier;
pub mod verifiers;

// Re-exports de conveniencia.
pub use artifact::{Artifact, ArtifactId, ArtifactKind};
pub use claim::{Claim, ClaimId, ClaimKind, ClaimStatus};
pub use engine::VerificationEngine;
pub use error::{VerificationError, VerificationResult};
pub use evidence::{Evidence, EvidenceId, EvidenceKind};
pub use hash::ContentHash;
pub use verifier::{VerificationContext, VerificationOutcome, Verifier};
pub use verifiers::{CommandExitVerifier, FileExistsVerifier, HashVerifier};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn full_flow_agent_claims_and_evidence() {
        let content = "print('hello')";
        let artifact = Artifact::from_text("main.py", content).with_path("src/main.py");
        let evidence = Evidence::file_exists("src/main.py")
            .with_artifact(artifact.id)
            .with_hash(artifact.hash.clone());
        let claim = Claim::file_created("src/main.py")
            .with_artifact(artifact.id)
            .with_evidence(evidence.id)
            .with_expected_hash(ContentHash::of_str(content));

        assert!(artifact.hash_matches(&claim.expected_hash.clone().unwrap()));
        assert_eq!(evidence.hash, Some(artifact.hash.clone()));
        assert!(claim.is_pending());
    }

    #[test]
    fn full_flow_engine_verification() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.py"), "print('hello')").unwrap();

        let engine = VerificationEngine::with_defaults();
        let ctx = VerificationContext::new(dir.path());

        let mut claim = Claim::file_created("main.py")
            .with_expected_hash(ContentHash::of_str("print('hello')"));

        let outcome = engine.verify_and_update(&mut claim, &ctx);

        assert!(outcome.is_verified());
        assert!(claim.is_verified());
        assert!(!outcome.evidence.is_empty());
    }
}
