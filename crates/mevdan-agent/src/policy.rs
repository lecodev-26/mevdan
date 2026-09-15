//! Política de ejecución de un agente.
//!
//! La política define **los límites** del agente: cuántos pasos puede
//! dar, cuántos tokens puede consumir, cuánto tiempo puede tardar, y
//! si tiene permitido auto-revisar y replanificar.

use serde::{Deserialize, Serialize};

/// Modo de auto-crítica.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewMode {
    /// Sin auto-revisión.
    None,
    /// El agente revisa su propia respuesta antes de devolverla.
    SelfReview,
    /// El agente revisa, y si algo falla, se le pide que replantee.
    SelfReviewWithReplan,
}

/// Política de ejecución.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPolicy {
    /// Máximo número de pasos del loop.
    pub max_steps: u32,

    /// Máximo número de tokens consumidos (suma de input + output).
    pub max_tokens: u32,

    /// Presupuesto de tiempo en segundos.
    pub max_seconds: u64,

    /// Máximo número de replannings permitidos.
    pub max_replans: u32,

    /// Modo de auto-crítica.
    pub review_mode: ReviewMode,

    /// Si `true`, el agente puede delegar a sub-agentes.
    pub allow_sub_agents: bool,
}

impl Default for ExecutionPolicy {
    fn default() -> Self {
        Self {
            max_steps: 20,
            max_tokens: 100_000,
            max_seconds: 300,
            max_replans: 3,
            // Por defecto, el agente puede replantear si su primera
            // respuesta no supera la auto-crítica. Es lo que espera
            // un usuario. Para un agente sin replanning, usar
            // `minimal()`.
            review_mode: ReviewMode::SelfReviewWithReplan,
            allow_sub_agents: false,
        }
    }
}

impl ExecutionPolicy {
    /// Política restrictiva: pocas iteraciones, sin auto-review.
    pub fn minimal() -> Self {
        Self {
            max_steps: 1,
            max_tokens: 4_000,
            max_seconds: 30,
            max_replans: 0,
            review_mode: ReviewMode::None,
            allow_sub_agents: false,
        }
    }

    /// Política permisiva: muchas iteraciones, auto-review y replanning.
    pub fn thorough() -> Self {
        Self {
            max_steps: 50,
            max_tokens: 500_000,
            max_seconds: 1800,
            max_replans: 10,
            review_mode: ReviewMode::SelfReviewWithReplan,
            allow_sub_agents: true,
        }
    }

    /// ¿Permite auto-revisión?
    pub fn has_review(&self) -> bool {
        !matches!(self.review_mode, ReviewMode::None)
    }

    /// ¿Permite replanning?
    pub fn has_replan(&self) -> bool {
        matches!(self.review_mode, ReviewMode::SelfReviewWithReplan) && self.max_replans > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_reasonable() {
        let p = ExecutionPolicy::default();
        assert_eq!(p.max_steps, 20);
        assert!(p.has_review());
        assert!(p.has_replan());
        assert!(!p.allow_sub_agents);
    }

    #[test]
    fn minimal_has_no_review() {
        let p = ExecutionPolicy::minimal();
        assert_eq!(p.max_steps, 1);
        assert!(!p.has_review());
        assert!(!p.has_replan());
    }

    #[test]
    fn thorough_has_all_features() {
        let p = ExecutionPolicy::thorough();
        assert!(p.max_steps >= 50);
        assert!(p.has_review());
        assert!(p.has_replan());
        assert!(p.allow_sub_agents);
    }

    #[test]
    fn review_mode_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&ReviewMode::None).unwrap(),
            "\"none\""
        );
        assert_eq!(
            serde_json::to_string(&ReviewMode::SelfReview).unwrap(),
            "\"self_review\""
        );
        assert_eq!(
            serde_json::to_string(&ReviewMode::SelfReviewWithReplan).unwrap(),
            "\"self_review_with_replan\""
        );
    }

    #[test]
    fn policy_roundtrips() {
        let p = ExecutionPolicy::thorough();
        let json = serde_json::to_string(&p).unwrap();
        let back: ExecutionPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(back.max_steps, p.max_steps);
        assert_eq!(back.review_mode, p.review_mode);
    }

    #[test]
    fn self_review_without_replan_has_no_replan() {
        let p = ExecutionPolicy {
            review_mode: ReviewMode::SelfReview,
            max_replans: 5,
            ..ExecutionPolicy::default()
        };
        assert!(p.has_review());
        assert!(!p.has_replan());
    }

    #[test]
    fn self_review_with_replan_but_zero_replans_has_no_replan() {
        let p = ExecutionPolicy {
            review_mode: ReviewMode::SelfReviewWithReplan,
            max_replans: 0,
            ..ExecutionPolicy::default()
        };
        assert!(p.has_review());
        assert!(!p.has_replan());
    }
}
