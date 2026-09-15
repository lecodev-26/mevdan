//! Sesión de un agente.
//!
//! La sesión contiene el **estado mutable** de un agente durante su
//! ejecución: historial de mensajes, contadores, tiempo de inicio, etc.

use crate::{
    error::{AgentError, AgentResult},
    policy::ExecutionPolicy,
};
use chrono::{DateTime, Utc};
use mevdan_provider::Message;
use uuid::Uuid;

/// ID de una sesión de agente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AgentSessionId(pub Uuid);

impl AgentSessionId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for AgentSessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AgentSessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Estado mutable de una sesión de agente.
#[derive(Debug)]
pub struct AgentSession {
    pub id: AgentSessionId,
    pub started_at: DateTime<Utc>,
    pub policy: ExecutionPolicy,
    pub messages: Vec<Message>,
    pub steps_taken: u32,
    pub tokens_used: u32,
    pub replans_used: u32,
}

impl AgentSession {
    /// Crea una nueva sesión con la política dada.
    pub fn new(policy: ExecutionPolicy) -> Self {
        Self {
            id: AgentSessionId::new(),
            started_at: Utc::now(),
            policy,
            messages: Vec::new(),
            steps_taken: 0,
            tokens_used: 0,
            replans_used: 0,
        }
    }

    /// Añade un mensaje al historial.
    pub fn push_message(&mut self, msg: Message) {
        self.messages.push(msg);
    }

    /// Registra un paso dado.
    pub fn record_step(&mut self) {
        self.steps_taken += 1;
    }

    /// Registra consumo de tokens.
    pub fn record_tokens(&mut self, n: u32) {
        self.tokens_used = self.tokens_used.saturating_add(n);
    }

    /// Registra un replanning.
    pub fn record_replan(&mut self) {
        self.replans_used += 1;
    }

    /// Comprueba si quedan pasos disponibles.
    pub fn can_take_step(&self) -> AgentResult<()> {
        if self.steps_taken >= self.policy.max_steps {
            return Err(AgentError::MaxStepsExceeded {
                max: self.policy.max_steps,
                reason: "reached step limit".into(),
            });
        }
        Ok(())
    }

    /// Comprueba si quedan tokens disponibles.
    pub fn can_consume_tokens(&self, additional: u32) -> AgentResult<()> {
        if self.tokens_used.saturating_add(additional) > self.policy.max_tokens {
            return Err(AgentError::MaxTokensExceeded {
                max: self.policy.max_tokens,
                reason: "would exceed token budget".into(),
            });
        }
        Ok(())
    }

    /// Comprueba si queda tiempo disponible.
    pub fn can_continue_in_time(&self) -> AgentResult<()> {
        let elapsed = (Utc::now() - self.started_at).num_seconds().max(0) as u64;
        if elapsed > self.policy.max_seconds {
            return Err(AgentError::TimeBudgetExceeded {
                max_secs: self.policy.max_seconds,
                reason: "time budget exhausted".into(),
            });
        }
        Ok(())
    }

    /// Comprueba si quedan replans disponibles.
    pub fn can_replan(&self) -> bool {
        self.policy.has_replan() && self.replans_used < self.policy.max_replans
    }

    /// Número de mensajes en el historial.
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_starts_empty() {
        let s = AgentSession::new(ExecutionPolicy::default());
        assert_eq!(s.message_count(), 0);
        assert_eq!(s.steps_taken, 0);
        assert_eq!(s.tokens_used, 0);
        assert_eq!(s.replans_used, 0);
    }

    #[test]
    fn push_message_increments_count() {
        let mut s = AgentSession::new(ExecutionPolicy::default());
        s.push_message(Message::user("hello"));
        s.push_message(Message::assistant("hi"));
        assert_eq!(s.message_count(), 2);
    }

    #[test]
    fn can_take_step_under_limit() {
        let mut s = AgentSession::new(ExecutionPolicy::default());
        s.record_step();
        assert!(s.can_take_step().is_ok());
    }

    #[test]
    fn can_take_step_at_limit_fails() {
        let policy = ExecutionPolicy {
            max_steps: 2,
            ..ExecutionPolicy::default()
        };
        let mut s = AgentSession::new(policy);
        s.record_step();
        s.record_step();
        let err = s.can_take_step().unwrap_err();
        assert!(matches!(err, AgentError::MaxStepsExceeded { .. }));
    }

    #[test]
    fn can_consume_tokens_within_budget() {
        let s = AgentSession::new(ExecutionPolicy::default());
        assert!(s.can_consume_tokens(1000).is_ok());
    }

    #[test]
    fn can_consume_tokens_exceeds_budget() {
        let policy = ExecutionPolicy {
            max_tokens: 100,
            ..ExecutionPolicy::default()
        };
        let s = AgentSession::new(policy);
        let err = s.can_consume_tokens(200).unwrap_err();
        assert!(matches!(err, AgentError::MaxTokensExceeded { .. }));
    }

    #[test]
    fn record_tokens_accumulates() {
        let mut s = AgentSession::new(ExecutionPolicy::default());
        s.record_tokens(100);
        s.record_tokens(50);
        assert_eq!(s.tokens_used, 150);
    }

    #[test]
    fn can_replan_within_limit() {
        let mut s = AgentSession::new(ExecutionPolicy::default());
        assert!(s.can_replan());
        s.record_replan();
        assert!(s.can_replan());
    }

    #[test]
    fn can_replan_exhausted() {
        let policy = ExecutionPolicy {
            max_replans: 1,
            ..ExecutionPolicy::default()
        };
        let mut s = AgentSession::new(policy);
        s.record_replan();
        assert!(!s.can_replan());
    }

    #[test]
    fn can_continue_in_time_fresh_session() {
        let s = AgentSession::new(ExecutionPolicy::default());
        assert!(s.can_continue_in_time().is_ok());
    }

    #[test]
    fn session_id_is_unique() {
        let a = AgentSessionId::new();
        let b = AgentSessionId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn session_id_displays() {
        let id = AgentSessionId::new();
        let s = id.to_string();
        assert!(!s.is_empty());
        assert!(s.contains('-'));
    }
}
