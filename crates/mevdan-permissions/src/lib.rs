//! # mevdan-permissions
//!
//! Sistema de permisos y evaluación de riesgo de MEVDAN.
//!
//! ## Modelo
//!
//! - Cada tool puede invocarse solo si el motor de permisos lo
//!   autoriza.
//! - Las reglas se evalúan **en orden**: la primera que matchea gana.
//! - Si ninguna matchea, el default es **`Deny`** (safety by default).
//! - El **Risk Engine** evalúa el riesgo de una invocación y puede
//!   escalar `Allow` → `Ask` cuando el riesgo es alto.
//!
//! ## Permisos
//!
//! - `Allow` — ejecuta sin preguntar.
//! - `Ask` — pregunta al usuario antes de ejecutar.
//! - `Deny` — rechaza.
//!
//! ## Niveles de riesgo
//!
//! - `Low` — sin efectos relevantes.
//! - `Medium` — efectos dentro del workspace.
//! - `High` — destructivo o fuera del workspace.
//! - `Critical` — irreversible o peligroso (`sudo`, `rm -rf`).
//!
//! ## Estado del proyecto
//!
//! - **18.1** ✅ — `Permission`, `Scope`, `Rule`.
//! - **18.2** ✅ — `Policy`, `PermissionEngine`, `Decision`, `Invocation`.
//! - **19** ✅ — `RiskEngine`, `RiskReport`, `RiskFactor`, `CombinedDecision`.
//!
//! ## Ejemplo: permisos
//!
//! ```
//! use mevdan_permissions::{
//!     Invocation, Permission, PermissionEngine, Policy, Rule, Scope,
//! };
//!
//! let policy = Policy::new()
//!     .add_rule(Rule::new("filesystem", Scope::action("delete"), Permission::Deny).unwrap())
//!     .add_rule(Rule::new("filesystem", Scope::glob("src/**"), Permission::Allow).unwrap())
//!     .add_rule(Rule::new("filesystem", Scope::Any, Permission::Ask).unwrap());
//!
//! let engine = PermissionEngine::new(policy);
//!
//! let inv = Invocation::filesystem("read", "src/main.rs");
//! assert!(engine.evaluate(&inv).is_allowed());
//!
//! let inv = Invocation::filesystem("delete", "src/main.rs");
//! assert!(engine.evaluate(&inv).is_denied());
//! ```
//!
//! ## Ejemplo: riesgo
//!
//! ```
//! use mevdan_permissions::{Invocation, RiskEngine};
//!
//! let engine = RiskEngine::new();
//!
//! let inv = Invocation::filesystem("read", "src/main.rs");
//! let report = engine.evaluate(&inv);
//! assert!(!report.is_dangerous());
//!
//! let inv = Invocation::filesystem("delete", "src/main.rs");
//! let report = engine.evaluate(&inv);
//! assert!(report.is_dangerous());
//! ```

pub mod combined;
pub mod decision;
pub mod error;
pub mod invocation;
pub mod permission;
pub mod policy;
pub mod risk;
pub mod rule;

// Re-exports de conveniencia.
pub use combined::CombinedDecision;
pub use decision::{Decision, DecisionKind, DecisionSource};
pub use error::{PermissionError, PermissionResult};
pub use invocation::Invocation;
pub use permission::{Permission, Scope};
pub use policy::{PermissionEngine, Policy};
pub use risk::{RiskAssessment, RiskEngine, RiskFactor, RiskReport};
pub use rule::{Rule, ANY_TOOL};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_flow_allow_ask_deny() {
        let policy = Policy::new()
            .add_rule(
                Rule::new("filesystem", Scope::action("delete"), Permission::Deny)
                    .unwrap()
                    .with_reason("never delete automatically"),
            )
            .add_rule(Rule::new("filesystem", Scope::glob("src/**"), Permission::Allow).unwrap())
            .add_rule(Rule::new("filesystem", Scope::Any, Permission::Ask).unwrap());

        let engine = PermissionEngine::new(policy);

        assert!(engine
            .evaluate(&Invocation::filesystem("read", "src/main.rs"))
            .is_allowed());

        assert!(engine
            .evaluate(&Invocation::filesystem("read", "docs/x.md"))
            .needs_user_input());

        let d = engine.evaluate(&Invocation::filesystem("delete", "src/main.rs"));
        assert!(d.is_denied());
        assert_eq!(d.reason, "never delete automatically");
    }

    #[test]
    fn full_flow_combined_with_risk_escalation() {
        let policy = Policy::new()
            // Todo permitido por defecto.
            .add_rule(Rule::new("*", Scope::Any, Permission::Allow).unwrap());
        let perm_engine = PermissionEngine::new(policy);
        let risk_engine = RiskEngine::new();

        // 1. Lectura segura → Allow sin escalada.
        let inv = Invocation::filesystem("read", "src/main.rs");
        let perm = perm_engine.evaluate(&inv);
        let risk = risk_engine.evaluate(&inv);
        let combined = CombinedDecision::combine(perm, risk);
        assert!(combined.is_allowed());
        assert!(!combined.escalated_by_risk);

        // 2. Delete → Allow por permisos, pero riesgo HIGH → Ask.
        let inv = Invocation::filesystem("delete", "src/main.rs");
        let perm = perm_engine.evaluate(&inv);
        let risk = risk_engine.evaluate(&inv);
        let combined = CombinedDecision::combine(perm, risk);
        assert!(combined.needs_user_input());
        assert!(combined.escalated_by_risk);
        assert!(combined.is_high_risk());
    }
}
