//! # mevdan-automation
//!
//! Automatización por reglas para MEVDAN.
//!
//! ## Concepto
//!
//! Una **regla** combina un **trigger** (qué dispara) con una lista de
//! **acciones** (qué hacer). Ejemplos:
//!
//! - "Cuando se modifique un archivo `.rs`, corre `cargo test`."
//! - "Cuando ocurra `refactor.start`, crea un checkpoint."
//! - "Cuando `budget.exceeded`, haz handoff a un agente más barato."
//! - "Cada 60 minutos, crea un checkpoint."
//!
//! ```text
//!     Trigger              Actions
//!   ┌─────────┐          ┌──────────┐
//!   │ File .rs│  ──►     │ cargo    │
//!   └─────────┘          │ test     │
//!                        └──────────┘
//! ```
//!
//! ## Estado del proyecto
//!
//! - **V5.2** ✅ — `Trigger`, `Action`, `Rule`, `RuleEngine`.
//!
//! ## Ejemplo
//!
//! ```no_run
//! use mevdan_automation::{
//!     Action, Rule, RuleEngine, Trigger, TriggerInput,
//! };
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut engine = RuleEngine::new();
//!
//! engine.add_rule(
//!     Rule::new(
//!         "test-on-rs-change",
//!         Trigger::file_change(".rs"),
//!         vec![Action::run_command_with_args("cargo", ["test"])],
//!     )?,
//! )?;
//!
//! let firings = engine.evaluate_and_record(TriggerInput::file_changed("src/main.rs"));
//! assert_eq!(firings.len(), 1);
//! # Ok(())
//! # }
//! ```

pub mod action;
pub mod error;
pub mod rule;
pub mod trigger;

// Re-exports de conveniencia.
pub use action::{Action, ActionResult};
pub use error::{AutomationError, AutomationResult};
pub use rule::{validate_name, Rule, RuleEngine, RuleId};
pub use trigger::{Trigger, TriggerFiring, TriggerInput};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_flow_rule_lifecycle() {
        let mut engine = RuleEngine::new();

        let rule = Rule::new(
            "checkpoint-hourly",
            Trigger::schedule(60),
            vec![Action::create_checkpoint(Some("hourly".into()))],
        )
        .unwrap();

        engine.add_rule(rule).unwrap();
        assert_eq!(engine.len(), 1);

        // A los 30 minutos: no matchea.
        let firings = engine.evaluate_and_record(TriggerInput::tick(30));
        assert!(firings.is_empty());

        // A los 60 minutos: matchea.
        let firings = engine.evaluate_and_record(TriggerInput::tick(60));
        assert_eq!(firings.len(), 1);

        // Desactivar.
        let mut rule = engine.get_rule("checkpoint-hourly").unwrap().clone();
        rule.disable();
        engine.set_rule(rule);

        let firings = engine.evaluate_and_record(TriggerInput::tick(120));
        assert!(firings.is_empty());
    }

    #[test]
    fn full_flow_multi_action_rule() {
        let mut engine = RuleEngine::new();

        engine
            .add_rule(
                Rule::new(
                    "pre-commit",
                    Trigger::manual(),
                    vec![
                        Action::create_checkpoint(Some("pre-commit".into())),
                        Action::run_command_with_args("cargo", ["test"]),
                        Action::notify("tests starting"),
                    ],
                )
                .unwrap(),
            )
            .unwrap();

        let firings = engine.evaluate_and_record(TriggerInput::manual());
        assert_eq!(firings.len(), 1);

        let rule = engine.get_rule("pre-commit").unwrap();
        assert_eq!(rule.action_count(), 3);
    }
}
