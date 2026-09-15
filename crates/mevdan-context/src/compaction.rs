//! Compactación de contexto.
//!
//! Cuando un historial de conversación crece demasiado, hay que
//! **compactarlo** antes de enviarlo al provider. Pero compactar no
//! significa "resumir todo" — hay información que NO se puede perder:
//!
//! - **Decisiones** tomadas por el agente.
//! - **Restricciones** declaradas por el usuario.
//! - **Errores** encontrados.
//! - **Resultados de tools** (cuando existan).
//!
//! Este módulo define:
//! - `CompactionPolicy` — cuándo y cómo compactar.
//! - `Compactor` — el algoritmo.
//! - `CompactedHistory` — el resultado.

use chrono::{DateTime, Utc};
use mevdan_provider::{Message, Role};
use serde::{Deserialize, Serialize};

/// Marcadores que indican información crítica en un mensaje.
pub const DECISION_MARKERS: &[&str] = &["DECISION:", "DECIDED:", "DECISIÓN:", "DECIDIDO:"];

pub const CONSTRAINT_MARKERS: &[&str] = &[
    "CONSTRAINT:",
    "MUST:",
    "REQUIRED:",
    "RESTRICCIÓN:",
    "DEBE:",
    "OBLIGATORIO:",
];

pub const ERROR_MARKERS: &[&str] = &["ERROR:", "FAILED:", "FAILURE:", "FALLO:"];

pub const RESULT_MARKERS: &[&str] = &["RESULT:", "RESULTADO:", "OUTPUT:", "SALIDA:"];

/// Estrategia de compactación.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactionStrategy {
    /// No compactar. El historial se envía tal cual.
    None,
    /// Mantener solo los N mensajes más recientes.
    Recency,
    /// Extraer información estructurada de los mensajes antiguos.
    Structured,
    /// Combinación: N recientes + resumen estructurado de anteriores.
    Hybrid,
}

/// Política de compactación.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionPolicy {
    pub strategy: CompactionStrategy,
    pub keep_recent: usize,
    pub trigger_at: usize,
}

impl Default for CompactionPolicy {
    fn default() -> Self {
        Self {
            strategy: CompactionStrategy::Hybrid,
            keep_recent: 10,
            trigger_at: 30,
        }
    }
}

impl CompactionPolicy {
    /// No compactar nunca.
    pub fn disabled() -> Self {
        Self {
            strategy: CompactionStrategy::None,
            keep_recent: 0,
            trigger_at: usize::MAX,
        }
    }

    /// Compactación agresiva: solo los 5 más recientes + estructura.
    pub fn aggressive() -> Self {
        Self {
            strategy: CompactionStrategy::Hybrid,
            keep_recent: 5,
            trigger_at: 15,
        }
    }

    /// Compactación conservadora: 20 recientes + estructura.
    pub fn conservative() -> Self {
        Self {
            strategy: CompactionStrategy::Hybrid,
            keep_recent: 20,
            trigger_at: 50,
        }
    }

    /// ¿Debería compactarse un historial de este tamaño?
    pub fn should_compact(&self, history_len: usize) -> bool {
        // `!matches!(...)` en vez de `matches!(...) == false` (Clippy).
        !matches!(self.strategy, CompactionStrategy::None) && history_len >= self.trigger_at
    }
}

/// Historial compactado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactedHistory {
    pub messages: Vec<Message>,
    pub decisions: Vec<String>,
    pub constraints: Vec<String>,
    pub errors: Vec<String>,
    pub results: Vec<String>,
    pub original_count: usize,
    pub final_count: usize,
    pub dropped_count: usize,
    pub compacted_at: DateTime<Utc>,
}

impl CompactedHistory {
    pub fn summary(&self) -> String {
        format!(
            "compacted: {} → {} messages ({} dropped), \
             {} decisions, {} constraints, {} errors, {} results",
            self.original_count,
            self.final_count,
            self.dropped_count,
            self.decisions.len(),
            self.constraints.len(),
            self.errors.len(),
            self.results.len(),
        )
    }

    pub fn has_drops(&self) -> bool {
        self.dropped_count > 0
    }
}

/// Compactador de historial.
#[derive(Debug)]
pub struct Compactor {
    policy: CompactionPolicy,
}

impl Compactor {
    pub fn new(policy: CompactionPolicy) -> Self {
        Self { policy }
    }

    pub fn compact(&self, history: &[Message]) -> CompactedHistory {
        let original_count = history.len();

        if matches!(self.policy.strategy, CompactionStrategy::None)
            || original_count < self.policy.trigger_at
        {
            return CompactedHistory {
                messages: history.to_vec(),
                decisions: Vec::new(),
                constraints: Vec::new(),
                errors: Vec::new(),
                results: Vec::new(),
                original_count,
                final_count: original_count,
                dropped_count: 0,
                compacted_at: Utc::now(),
            };
        }

        let mut system_messages: Vec<Message> = Vec::new();
        let mut other_messages: Vec<Message> = Vec::new();

        for m in history {
            if m.role == Role::System {
                system_messages.push(m.clone());
            } else {
                other_messages.push(m.clone());
            }
        }

        let keep_recent = self.policy.keep_recent.min(other_messages.len());
        let split_at = other_messages.len() - keep_recent;
        let (old, recent) = other_messages.split_at(split_at);

        let (decisions, constraints, errors, results) = extract_markers(old);

        let mut messages = system_messages;
        messages.extend(recent.iter().cloned());

        let final_count = messages.len();
        let dropped_count = original_count - final_count;

        CompactedHistory {
            messages,
            decisions,
            constraints,
            errors,
            results,
            original_count,
            final_count,
            dropped_count,
            compacted_at: Utc::now(),
        }
    }
}

impl Default for Compactor {
    fn default() -> Self {
        Self::new(CompactionPolicy::default())
    }
}

fn extract_markers(messages: &[Message]) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let mut decisions = Vec::new();
    let mut constraints = Vec::new();
    let mut errors = Vec::new();
    let mut results = Vec::new();

    for m in messages {
        for line in m.content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let upper = trimmed.to_uppercase();
            if DECISION_MARKERS.iter().any(|m| upper.starts_with(m)) {
                decisions.push(trimmed.to_string());
            } else if CONSTRAINT_MARKERS.iter().any(|m| upper.starts_with(m)) {
                constraints.push(trimmed.to_string());
            } else if ERROR_MARKERS.iter().any(|m| upper.starts_with(m)) {
                errors.push(trimmed.to_string());
            } else if RESULT_MARKERS.iter().any(|m| upper.starts_with(m)) {
                results.push(trimmed.to_string());
            }
        }
    }

    (decisions, constraints, errors, results)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: Role, content: &str) -> Message {
        Message {
            role,
            content: content.into(),
        }
    }

    #[test]
    fn policy_default_is_hybrid() {
        let p = CompactionPolicy::default();
        assert_eq!(p.strategy, CompactionStrategy::Hybrid);
        assert_eq!(p.keep_recent, 10);
        assert_eq!(p.trigger_at, 30);
    }

    #[test]
    fn policy_disabled_never_compacts() {
        let p = CompactionPolicy::disabled();
        assert!(!p.should_compact(100));
    }

    #[test]
    fn policy_aggressive_triggers_early() {
        let p = CompactionPolicy::aggressive();
        assert!(p.should_compact(15));
        assert!(!p.should_compact(14));
    }

    #[test]
    fn strategy_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&CompactionStrategy::Hybrid).unwrap(),
            "\"hybrid\""
        );
        assert_eq!(
            serde_json::to_string(&CompactionStrategy::Recency).unwrap(),
            "\"recency\""
        );
    }

    #[test]
    fn compact_below_threshold_does_nothing() {
        let policy = CompactionPolicy {
            strategy: CompactionStrategy::Hybrid,
            keep_recent: 5,
            trigger_at: 10,
        };
        let compactor = Compactor::new(policy);

        let history: Vec<Message> = (0..5)
            .map(|i| msg(Role::User, &format!("msg {}", i)))
            .collect();

        let result = compactor.compact(&history);
        assert_eq!(result.final_count, 5);
        assert_eq!(result.dropped_count, 0);
        assert!(!result.has_drops());
    }

    #[test]
    fn compact_disabled_strategy_never_drops() {
        let policy = CompactionPolicy::disabled();
        let compactor = Compactor::new(policy);

        let history: Vec<Message> = (0..100)
            .map(|i| msg(Role::User, &format!("msg {}", i)))
            .collect();

        let result = compactor.compact(&history);
        assert_eq!(result.final_count, 100);
        assert_eq!(result.dropped_count, 0);
    }

    #[test]
    fn compact_hybrid_keeps_recent() {
        let policy = CompactionPolicy {
            strategy: CompactionStrategy::Hybrid,
            keep_recent: 3,
            trigger_at: 5,
        };
        let compactor = Compactor::new(policy);

        let history: Vec<Message> = (0..10)
            .map(|i| msg(Role::User, &format!("msg {}", i)))
            .collect();

        let result = compactor.compact(&history);
        assert_eq!(result.final_count, 3);
        assert_eq!(result.dropped_count, 7);
        assert_eq!(result.messages[0].content, "msg 7");
        assert_eq!(result.messages[2].content, "msg 9");
    }

    #[test]
    fn compact_preserves_system_messages() {
        let policy = CompactionPolicy {
            strategy: CompactionStrategy::Hybrid,
            keep_recent: 2,
            trigger_at: 5,
        };
        let compactor = Compactor::new(policy);

        let mut history = vec![msg(Role::System, "system prompt")];
        for i in 0..10 {
            history.push(msg(Role::User, &format!("msg {}", i)));
        }

        let result = compactor.compact(&history);
        assert_eq!(result.final_count, 3);
        assert_eq!(result.messages[0].role, Role::System);
        assert_eq!(result.messages[0].content, "system prompt");
    }

    #[test]
    fn compact_extracts_decisions() {
        let policy = CompactionPolicy {
            strategy: CompactionStrategy::Hybrid,
            keep_recent: 2,
            trigger_at: 5,
        };
        let compactor = Compactor::new(policy);

        let mut history = vec![
            msg(Role::User, "DECISION: use Rust for this project"),
            msg(Role::User, "DECISION: use SQLite for storage"),
        ];
        for i in 0..10 {
            history.push(msg(Role::User, &format!("msg {}", i)));
        }

        let result = compactor.compact(&history);
        assert_eq!(result.decisions.len(), 2);
        assert!(result.decisions[0].contains("use Rust"));
        assert!(result.decisions[1].contains("use SQLite"));
    }

    #[test]
    fn compact_extracts_constraints() {
        let policy = CompactionPolicy {
            strategy: CompactionStrategy::Hybrid,
            keep_recent: 2,
            trigger_at: 5,
        };
        let compactor = Compactor::new(policy);

        let mut history = vec![
            msg(Role::User, "CONSTRAINT: must work offline"),
            msg(Role::User, "MUST: support Windows"),
        ];
        for i in 0..10 {
            history.push(msg(Role::User, &format!("msg {}", i)));
        }

        let result = compactor.compact(&history);
        assert_eq!(result.constraints.len(), 2);
    }

    #[test]
    fn compact_extracts_errors() {
        let policy = CompactionPolicy {
            strategy: CompactionStrategy::Hybrid,
            keep_recent: 2,
            trigger_at: 5,
        };
        let compactor = Compactor::new(policy);

        let mut history = vec![
            msg(Role::User, "ERROR: failed to compile"),
            msg(Role::User, "FAILED: test_x did not pass"),
        ];
        for i in 0..10 {
            history.push(msg(Role::User, &format!("msg {}", i)));
        }

        let result = compactor.compact(&history);
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn compact_extracts_results() {
        let policy = CompactionPolicy {
            strategy: CompactionStrategy::Hybrid,
            keep_recent: 2,
            trigger_at: 5,
        };
        let compactor = Compactor::new(policy);

        let mut history = vec![
            msg(Role::User, "RESULT: 42"),
            msg(Role::User, "OUTPUT: hello world"),
        ];
        for i in 0..10 {
            history.push(msg(Role::User, &format!("msg {}", i)));
        }

        let result = compactor.compact(&history);
        assert_eq!(result.results.len(), 2);
    }

    #[test]
    fn compact_does_not_extract_from_recent() {
        let policy = CompactionPolicy {
            strategy: CompactionStrategy::Hybrid,
            keep_recent: 5,
            trigger_at: 5,
        };
        let compactor = Compactor::new(policy);

        let history: Vec<Message> = (0..10)
            .map(|i| msg(Role::User, &format!("DECISION: decision {}", i)))
            .collect();

        let result = compactor.compact(&history);
        assert_eq!(result.decisions.len(), 5);
    }

    #[test]
    fn summary_contains_key_info() {
        let policy = CompactionPolicy {
            strategy: CompactionStrategy::Hybrid,
            keep_recent: 2,
            trigger_at: 5,
        };
        let compactor = Compactor::new(policy);

        let history: Vec<Message> = (0..10)
            .map(|i| msg(Role::User, &format!("msg {}", i)))
            .collect();

        let result = compactor.compact(&history);
        let summary = result.summary();
        assert!(summary.contains("10 → 2"));
        assert!(summary.contains("8 dropped"));
    }

    #[test]
    fn compacted_history_roundtrips() {
        let policy = CompactionPolicy {
            strategy: CompactionStrategy::Hybrid,
            keep_recent: 2,
            trigger_at: 5,
        };
        let compactor = Compactor::new(policy);

        let history: Vec<Message> = (0..10)
            .map(|i| msg(Role::User, &format!("msg {}", i)))
            .collect();

        let result = compactor.compact(&history);
        let json = serde_json::to_string(&result).unwrap();
        let back: CompactedHistory = serde_json::from_str(&json).unwrap();
        assert_eq!(back.final_count, result.final_count);
        assert_eq!(back.original_count, result.original_count);
    }

    #[test]
    fn conservative_policy_keeps_more() {
        let p = CompactionPolicy::conservative();
        assert_eq!(p.keep_recent, 20);
        assert_eq!(p.trigger_at, 50);
    }

    #[test]
    fn marker_detection_is_case_insensitive() {
        let policy = CompactionPolicy {
            strategy: CompactionStrategy::Hybrid,
            keep_recent: 1,
            trigger_at: 3,
        };
        let compactor = Compactor::new(policy);

        let mut history = vec![
            msg(Role::User, "decision: lowercase marker"),
            msg(Role::User, "Decision: mixed case"),
        ];
        for i in 0..5 {
            history.push(msg(Role::User, &format!("msg {}", i)));
        }

        let result = compactor.compact(&history);
        assert_eq!(result.decisions.len(), 2);
    }
}
