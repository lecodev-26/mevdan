//! # mevdan-context
//!
//! Motor de construcción y compactación de contexto.
//!
//! ## Qué hace
//!
//! - **Construcción** (Fase 13): recibe secciones de información
//!   (system prompt, tarea, proyecto, historial, archivos), las
//!   ordena por prioridad, y las recorta si exceden el presupuesto.
//! - **Compactación** (Fase 14): cuando un historial crece demasiado,
//!   lo reduce preservando información crítica (decisiones,
//!   restricciones, errores, resultados).

pub mod budget;
pub mod builder;
pub mod compaction;
pub mod error;
pub mod section;

// Re-exports de conveniencia.
pub use budget::{estimate_tokens, TokenBudget, BYTES_PER_TOKEN};
pub use builder::{Context, ContextBuilder};
pub use compaction::{
    CompactedHistory, CompactionPolicy, CompactionStrategy, Compactor, CONSTRAINT_MARKERS,
    DECISION_MARKERS, ERROR_MARKERS, RESULT_MARKERS,
};
pub use error::{ContextError, ContextResult};
pub use section::{Priority, Section};

#[cfg(test)]
mod tests {
    use super::*;
    use mevdan_provider::{Message, Role};

    #[test]
    fn full_flow_realistic_context() {
        let budget = TokenBudget::default_budget();
        let context = ContextBuilder::new(budget)
            .system("You are MEVDAN, an AI work runtime assistant.")
            .project("Project: demo. Language: Rust. Tests: 345 passing.")
            .task("Add a new function that calculates factorial.")
            .build()
            .unwrap();

        assert_eq!(context.section_count(), 3);
        assert!(context.fits());
        assert!(context.estimated_tokens > 0);

        let messages = context.to_messages();
        assert_eq!(messages.len(), 3);
    }

    #[test]
    fn budget_is_respected() {
        let budget = TokenBudget::small();
        let context = ContextBuilder::new(budget)
            .system("sys")
            .task("do something small")
            .build()
            .unwrap();

        assert!(context.estimated_tokens <= budget.available());
    }

    #[test]
    fn full_flow_compaction_with_markers() {
        // Usa `vec![]` directamente (Clippy: vec_init_then_push).
        let mut history: Vec<Message> = vec![
            Message {
                role: Role::System,
                content: "You are a helpful assistant.".into(),
            },
            Message::user("DECISION: use Rust"),
            Message::user("CONSTRAINT: must compile on Windows"),
            Message::user("ERROR: initial approach failed"),
        ];
        for i in 0..20 {
            history.push(Message::user(format!("chitchat {}", i)));
        }

        let policy = CompactionPolicy::aggressive(); // trigger_at = 15
        let compactor = Compactor::new(policy);
        let compacted = compactor.compact(&history);

        assert!(compacted.has_drops());
        assert_eq!(compacted.decisions.len(), 1);
        assert_eq!(compacted.constraints.len(), 1);
        assert_eq!(compacted.errors.len(), 1);
        assert_eq!(compacted.messages[0].role, Role::System);
    }
}
