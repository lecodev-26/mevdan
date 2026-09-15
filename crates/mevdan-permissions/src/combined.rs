//! Combinación de decisión de permisos y evaluación de riesgo.
//!
//! Cuando el motor de permisos dice `Allow` pero el riesgo es `High`
//! o `Critical`, el resultado combinado puede subir la decisión a
//! `Ask`. Esto se llama **auto-ask por riesgo**.

use crate::{
    decision::{Decision, DecisionKind},
    risk::{RiskAssessment, RiskReport},
};
use serde::{Deserialize, Serialize};

/// Decisión combinada: permisos + riesgo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedDecision {
    /// Decisión original del motor de permisos.
    pub decision: Decision,

    /// Reporte de riesgo.
    pub risk: RiskReport,

    /// Decisión efectiva (puede haber subido por riesgo).
    pub effective_kind: DecisionKind,

    /// Si `true`, la decisión efectiva subió debido al riesgo.
    pub escalated_by_risk: bool,

    /// Razón final legible.
    pub final_reason: String,
}

impl CombinedDecision {
    /// Combina una decisión de permisos con un reporte de riesgo.
    ///
    /// Regla: si la decisión es `Allowed` pero el riesgo es `High` o
    /// `Critical`, sube a `NeedsUserInput`.
    ///
    /// Si la decisión es `Denied`, se queda en `Denied` sin importar
    /// el riesgo.
    pub fn combine(decision: Decision, risk: RiskReport) -> Self {
        let original_kind = decision.kind;
        let mut effective_kind = original_kind;
        let mut escalated_by_risk = false;

        // Auto-ask: Allow + riesgo alto → Ask.
        if matches!(original_kind, DecisionKind::Allowed) && risk.is_dangerous() {
            effective_kind = DecisionKind::NeedsUserInput;
            escalated_by_risk = true;
        }

        let final_reason = if escalated_by_risk {
            format!(
                "{} → escalated to ASK due to {} risk: {}",
                original_kind.display_name(),
                risk.level.display_name(),
                risk.reason,
            )
        } else {
            format!(
                "{} ({}): {}",
                effective_kind.display_name(),
                risk.level.display_name(),
                decision.reason,
            )
        };

        Self {
            decision,
            risk,
            effective_kind,
            escalated_by_risk,
            final_reason,
        }
    }

    /// ¿La decisión efectiva permite ejecutar?
    pub fn is_allowed(&self) -> bool {
        matches!(self.effective_kind, DecisionKind::Allowed)
    }

    /// ¿La decisión efectiva requiere confirmación?
    pub fn needs_user_input(&self) -> bool {
        matches!(self.effective_kind, DecisionKind::NeedsUserInput)
    }

    /// ¿La decisión efectiva deniega?
    pub fn is_denied(&self) -> bool {
        matches!(self.effective_kind, DecisionKind::Denied)
    }

    /// ¿Es un riesgo serio?
    pub fn is_high_risk(&self) -> bool {
        matches!(
            self.risk.level,
            RiskAssessment::High | RiskAssessment::Critical
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decision::Decision,
        permission::Permission,
        risk::{RiskFactor, RiskReport},
    };

    #[test]
    fn allow_with_low_risk_stays_allowed() {
        let d = Decision::from_default(Permission::Allow);
        let r = RiskReport::low();
        let c = CombinedDecision::combine(d, r);
        assert!(c.is_allowed());
        assert!(!c.escalated_by_risk);
    }

    #[test]
    fn allow_with_high_risk_escalates_to_ask() {
        let d = Decision::from_default(Permission::Allow);
        let r = RiskReport::from_factors(vec![RiskFactor::DestructiveAction]);
        let c = CombinedDecision::combine(d, r);
        assert!(c.needs_user_input());
        assert!(c.escalated_by_risk);
        assert!(c.final_reason.contains("escalated"));
    }

    #[test]
    fn allow_with_critical_risk_escalates_to_ask() {
        let d = Decision::from_default(Permission::Allow);
        let r = RiskReport::from_factors(vec![RiskFactor::PrivilegeEscalation]);
        let c = CombinedDecision::combine(d, r);
        assert!(c.needs_user_input());
        assert!(c.escalated_by_risk);
    }

    #[test]
    fn ask_stays_ask_regardless_of_risk() {
        let d = Decision::from_default(Permission::Ask);
        let r = RiskReport::from_factors(vec![RiskFactor::DestructiveAction]);
        let c = CombinedDecision::combine(d, r);
        assert!(c.needs_user_input());
        assert!(!c.escalated_by_risk);
    }

    #[test]
    fn deny_stays_deny_regardless_of_risk() {
        let d = Decision::from_default(Permission::Deny);
        let r = RiskReport::low();
        let c = CombinedDecision::combine(d, r);
        assert!(c.is_denied());
        assert!(!c.escalated_by_risk);
    }

    #[test]
    fn deny_with_high_risk_stays_deny() {
        let d = Decision::from_default(Permission::Deny);
        let r = RiskReport::from_factors(vec![RiskFactor::DestructiveAction]);
        let c = CombinedDecision::combine(d, r);
        assert!(c.is_denied());
        assert!(!c.escalated_by_risk);
    }

    #[test]
    fn combined_roundtrips() {
        let d = Decision::from_default(Permission::Allow);
        let r = RiskReport::from_factors(vec![RiskFactor::Network]);
        let c = CombinedDecision::combine(d, r);
        let json = serde_json::to_string(&c).unwrap();
        let back: CombinedDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(back.effective_kind, c.effective_kind);
        assert_eq!(back.escalated_by_risk, c.escalated_by_risk);
    }
}
