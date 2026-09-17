//! `Rule` y `RuleEngine` — automatización por reglas.

use crate::{
    action::{Action, ActionResult},
    error::{AutomationError, AutomationResult},
    trigger::{Trigger, TriggerFiring, TriggerInput},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

/// ID de una regla.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RuleId(pub Uuid);

impl RuleId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for RuleId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Una regla de automatización.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: RuleId,
    pub name: String,
    pub trigger: Trigger,
    pub actions: Vec<Action>,
    /// Si está deshabilitada, no matchea.
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

fn default_true() -> bool {
    true
}

impl Rule {
    /// Crea una regla nueva.
    pub fn new(
        name: impl Into<String>,
        trigger: Trigger,
        actions: Vec<Action>,
    ) -> AutomationResult<Self> {
        let name = name.into();
        validate_name(&name)?;
        if actions.is_empty() {
            return Err(AutomationError::InvalidRule(
                "rule must have at least one action".into(),
            ));
        }
        Ok(Self {
            id: RuleId::new(),
            name,
            trigger,
            actions,
            enabled: true,
            description: None,
            created_at: Utc::now(),
        })
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// ¿Esta regla matchea un input?
    pub fn matches(&self, input: &TriggerInput) -> bool {
        self.enabled && self.trigger.matches(input)
    }

    pub fn action_count(&self) -> usize {
        self.actions.len()
    }
}

/// Valida un nombre de regla.
pub fn validate_name(name: &str) -> AutomationResult<()> {
    if name.is_empty() || name.len() > 64 {
        return Err(AutomationError::InvalidName(name.to_string()));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        return Err(AutomationError::InvalidName(name.to_string()));
    }
    Ok(())
}

/// Motor de reglas.
#[derive(Debug, Default)]
pub struct RuleEngine {
    rules: BTreeMap<String, Rule>,
    firings: Vec<TriggerFiring>,
    results: Vec<ActionResult>,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra una regla. Falla si ya existe una con el mismo nombre.
    pub fn add_rule(&mut self, rule: Rule) -> AutomationResult<()> {
        if self.rules.contains_key(&rule.name) {
            return Err(AutomationError::DuplicateRule(rule.name));
        }
        self.rules.insert(rule.name.clone(), rule);
        Ok(())
    }

    /// Registra o reemplaza una regla.
    pub fn set_rule(&mut self, rule: Rule) {
        self.rules.insert(rule.name.clone(), rule);
    }

    /// Elimina una regla.
    pub fn remove_rule(&mut self, name: &str) -> AutomationResult<Rule> {
        self.rules
            .remove(name)
            .ok_or_else(|| AutomationError::RuleNotFound(name.to_string()))
    }

    pub fn get_rule(&self, name: &str) -> Option<&Rule> {
        self.rules.get(name)
    }

    pub fn list_rules(&self) -> Vec<&Rule> {
        self.rules.values().collect()
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    pub fn clear(&mut self) {
        self.rules.clear();
    }

    /// Evalúa todas las reglas contra un input.
    ///
    /// Devuelve las reglas que matchean. No ejecuta sus acciones.
    pub fn evaluate(&self, input: &TriggerInput) -> Vec<&Rule> {
        self.rules.values().filter(|r| r.matches(input)).collect()
    }

    /// Evalúa y registra los disparos.
    pub fn evaluate_and_record(&mut self, input: TriggerInput) -> Vec<TriggerFiring> {
        let matching: Vec<Rule> = self
            .rules
            .values()
            .filter(|r| r.matches(&input))
            .cloned()
            .collect();

        let firings: Vec<TriggerFiring> = matching
            .into_iter()
            .map(|rule| TriggerFiring {
                rule_name: rule.name,
                trigger: rule.trigger,
                input: input.clone(),
                fired_at: Utc::now(),
            })
            .collect();

        self.firings.extend(firings.clone());
        firings
    }

    /// Registra el resultado de ejecutar una acción.
    pub fn record_result(&mut self, result: ActionResult) {
        self.results.push(result);
    }

    /// Historial de disparos.
    pub fn firings(&self) -> &[TriggerFiring] {
        &self.firings
    }

    /// Resultados registrados.
    pub fn results(&self) -> &[ActionResult] {
        &self.results
    }

    /// Limpia historial de disparos y resultados.
    pub fn clear_history(&mut self) {
        self.firings.clear();
        self.results.clear();
    }

    /// Serializa a JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.rules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trigger::TriggerInput;

    fn sample_rule(name: &str) -> Rule {
        Rule::new(
            name,
            Trigger::file_change(".rs"),
            vec![Action::run_command_with_args("cargo", ["test"])],
        )
        .unwrap()
    }

    // ─────────────────────────────────────────────
    // Rule
    // ─────────────────────────────────────────────

    #[test]
    fn rule_new_creates() {
        let r = sample_rule("on-save");
        assert_eq!(r.name, "on-save");
        assert!(r.enabled);
        assert!(r.description.is_none());
        assert_eq!(r.action_count(), 1);
    }

    #[test]
    fn rule_validates_name() {
        assert!(Rule::new("", Trigger::manual(), vec![Action::notify("x")]).is_err());
        assert!(Rule::new("Bad Name", Trigger::manual(), vec![Action::notify("x")]).is_err());
    }

    #[test]
    fn rule_requires_actions() {
        let err = Rule::new("empty", Trigger::manual(), vec![]).unwrap_err();
        assert!(matches!(err, AutomationError::InvalidRule(_)));
    }

    #[test]
    fn rule_with_description() {
        let r = sample_rule("x").with_description("test");
        assert_eq!(r.description.as_deref(), Some("test"));
    }

    #[test]
    fn rule_disabled_does_not_match() {
        let r = sample_rule("x").disabled();
        assert!(!r.matches(&TriggerInput::file_changed("main.rs")));
    }

    #[test]
    fn rule_matches_when_enabled() {
        let r = sample_rule("x");
        assert!(r.matches(&TriggerInput::file_changed("main.rs")));
    }

    #[test]
    fn rule_enable_disable() {
        let mut r = sample_rule("x");
        r.disable();
        assert!(!r.enabled);
        r.enable();
        assert!(r.enabled);
    }

    #[test]
    fn rule_serializes() {
        let r = sample_rule("x");
        let json = serde_json::to_string(&r).unwrap();
        let back: Rule = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, r.id);
        assert_eq!(back.name, r.name);
    }

    // ─────────────────────────────────────────────
    // RuleEngine
    // ─────────────────────────────────────────────

    #[test]
    fn engine_new_is_empty() {
        let e = RuleEngine::new();
        assert!(e.is_empty());
        assert_eq!(e.len(), 0);
    }

    #[test]
    fn engine_add_rule() {
        let mut e = RuleEngine::new();
        e.add_rule(sample_rule("r1")).unwrap();
        assert_eq!(e.len(), 1);
    }

    #[test]
    fn engine_rejects_duplicate() {
        let mut e = RuleEngine::new();
        e.add_rule(sample_rule("r1")).unwrap();
        let err = e.add_rule(sample_rule("r1")).unwrap_err();
        assert!(matches!(err, AutomationError::DuplicateRule(_)));
    }

    #[test]
    fn engine_set_rule_replaces() {
        let mut e = RuleEngine::new();
        e.add_rule(sample_rule("r1")).unwrap();
        e.set_rule(sample_rule("r1"));
        assert_eq!(e.len(), 1);
    }

    #[test]
    fn engine_remove_rule() {
        let mut e = RuleEngine::new();
        e.add_rule(sample_rule("r1")).unwrap();
        e.remove_rule("r1").unwrap();
        assert!(e.is_empty());
    }

    #[test]
    fn engine_remove_unknown_fails() {
        let mut e = RuleEngine::new();
        let err = e.remove_rule("nope").unwrap_err();
        assert!(matches!(err, AutomationError::RuleNotFound(_)));
    }

    #[test]
    fn engine_evaluate_matches_rules() {
        let mut e = RuleEngine::new();
        e.add_rule(sample_rule("rs-files")).unwrap();
        e.add_rule(
            Rule::new(
                "py-files",
                Trigger::file_change(".py"),
                vec![Action::notify("python changed")],
            )
            .unwrap(),
        )
        .unwrap();

        let matching = e.evaluate(&TriggerInput::file_changed("main.rs"));
        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].name, "rs-files");

        let matching = e.evaluate(&TriggerInput::file_changed("script.py"));
        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].name, "py-files");
    }

    #[test]
    fn engine_evaluate_returns_empty_when_no_match() {
        let mut e = RuleEngine::new();
        e.add_rule(sample_rule("rs-files")).unwrap();

        let matching = e.evaluate(&TriggerInput::event("task.failed"));
        assert_eq!(matching.len(), 0);
    }

    #[test]
    fn engine_evaluate_and_record() {
        let mut e = RuleEngine::new();
        e.add_rule(sample_rule("rs-files")).unwrap();
        e.add_rule(
            Rule::new(
                "py-files",
                Trigger::file_change(".py"),
                vec![Action::notify("py")],
            )
            .unwrap(),
        )
        .unwrap();

        let firings = e.evaluate_and_record(TriggerInput::file_changed("main.rs"));
        assert_eq!(firings.len(), 1);
        assert_eq!(firings[0].rule_name, "rs-files");

        assert_eq!(e.firings().len(), 1);
    }

    #[test]
    fn engine_multiple_firings_accumulate() {
        let mut e = RuleEngine::new();
        e.add_rule(sample_rule("rs-files")).unwrap();

        e.evaluate_and_record(TriggerInput::file_changed("a.rs"));
        e.evaluate_and_record(TriggerInput::file_changed("b.rs"));

        assert_eq!(e.firings().len(), 2);
    }

    #[test]
    fn engine_record_result() {
        let mut e = RuleEngine::new();
        let result = ActionResult::ok(Action::notify("x"), "done");
        e.record_result(result);
        assert_eq!(e.results().len(), 1);
    }

    #[test]
    fn engine_clear_history() {
        let mut e = RuleEngine::new();
        e.add_rule(sample_rule("rs-files")).unwrap();
        e.evaluate_and_record(TriggerInput::file_changed("a.rs"));
        e.clear_history();
        assert_eq!(e.firings().len(), 0);
        assert_eq!(e.results().len(), 0);
        // Las reglas siguen.
        assert_eq!(e.len(), 1);
    }

    #[test]
    fn engine_clear_removes_rules() {
        let mut e = RuleEngine::new();
        e.add_rule(sample_rule("r1")).unwrap();
        e.clear();
        assert!(e.is_empty());
    }

    #[test]
    fn engine_get_and_list() {
        let mut e = RuleEngine::new();
        e.add_rule(sample_rule("a")).unwrap();
        e.add_rule(sample_rule("b")).unwrap();

        assert!(e.get_rule("a").is_some());
        assert!(e.get_rule("c").is_none());
        assert_eq!(e.list_rules().len(), 2);
    }

    #[test]
    fn engine_serializes_rules() {
        let mut e = RuleEngine::new();
        e.add_rule(sample_rule("a")).unwrap();
        let json = e.to_json().unwrap();
        assert!(json.contains("\"a\""));
    }

    #[test]
    fn full_flow_save_and_test() {
        let mut e = RuleEngine::new();

        e.add_rule(
            Rule::new(
                "test-on-rs-change",
                Trigger::file_change(".rs"),
                vec![Action::run_command_with_args("cargo", ["test"])],
            )
            .unwrap()
            .with_description("Run tests when a Rust file changes"),
        )
        .unwrap();

        e.add_rule(
            Rule::new(
                "checkpoint-before-refactor",
                Trigger::event("refactor.start"),
                vec![Action::create_checkpoint(Some("before-refactor".into()))],
            )
            .unwrap(),
        )
        .unwrap();

        e.add_rule(
            Rule::new(
                "handoff-on-cost",
                Trigger::event("budget.exceeded"),
                vec![Action::handoff("Planner", "Coder", "cost")],
            )
            .unwrap(),
        )
        .unwrap();

        // Cambia un .rs → match con la primera regla.
        let f = e.evaluate_and_record(TriggerInput::file_changed("src/main.rs"));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_name, "test-on-rs-change");

        // Evento refactor.start → match con la segunda.
        let f = e.evaluate_and_record(TriggerInput::event("refactor.start"));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_name, "checkpoint-before-refactor");

        // Evento budget.exceeded → match con la tercera.
        let f = e.evaluate_and_record(TriggerInput::event("budget.exceeded"));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_name, "handoff-on-cost");

        assert_eq!(e.firings().len(), 3);
    }
}
