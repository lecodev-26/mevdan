//! Planes y pasos de plan.
//!
//! Un `Plan` es una secuencia ordenada de pasos que un agente (o
//! varios) puede seguir. **No es el Work Graph** — el Work Graph real
//! llega en Fase 20. Esto es una estructura ligera para que un agente
//! planificador pueda descomponer una tarea.

use crate::identity::AgentRole;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// ID de un plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlanId(pub Uuid);

impl PlanId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for PlanId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PlanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// ID de un paso de plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlanStepId(pub Uuid);

impl PlanStepId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for PlanStepId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PlanStepId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Estado de un paso del plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// Un paso dentro del plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: PlanStepId,
    /// Descripción de qué hay que hacer.
    pub description: String,
    /// Rol del agente que debería ejecutarlo (si se especifica).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<AgentRole>,
    /// IDs de los pasos que deben completarse antes.
    #[serde(default)]
    pub depends_on: Vec<PlanStepId>,
    /// Estado actual.
    pub status: PlanStepStatus,
    /// Resultado textual (si se completó).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

impl PlanStep {
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            id: PlanStepId::new(),
            description: description.into(),
            role: None,
            depends_on: Vec::new(),
            status: PlanStepStatus::Pending,
            result: None,
        }
    }

    pub fn with_role(mut self, role: AgentRole) -> Self {
        self.role = Some(role);
        self
    }

    pub fn with_depends_on(mut self, deps: Vec<PlanStepId>) -> Self {
        self.depends_on = deps;
        self
    }

    pub fn is_ready(&self, completed: &[PlanStepId]) -> bool {
        self.depends_on.iter().all(|d| completed.contains(d))
    }

    pub fn mark_completed(&mut self, result: impl Into<String>) {
        self.status = PlanStepStatus::Completed;
        self.result = Some(result.into());
    }

    pub fn mark_failed(&mut self, reason: impl Into<String>) {
        self.status = PlanStepStatus::Failed;
        self.result = Some(reason.into());
    }
}

/// Estado global del plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Abandoned,
}

/// Un plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: PlanId,
    /// Objetivo del plan.
    pub goal: String,
    /// Pasos ordenados.
    pub steps: Vec<PlanStep>,
    /// Estado global.
    pub status: PlanStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Plan {
    /// Crea un plan vacío con un objetivo.
    pub fn new(goal: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: PlanId::new(),
            goal: goal.into(),
            steps: Vec::new(),
            status: PlanStatus::Pending,
            created_at: now,
            updated_at: now,
        }
    }

    /// Crea un plan con pasos.
    pub fn with_steps(goal: impl Into<String>, steps: Vec<PlanStep>) -> Self {
        let mut p = Self::new(goal);
        p.steps = steps;
        p
    }

    /// Añade un paso al final.
    pub fn push_step(&mut self, step: PlanStep) {
        self.steps.push(step);
        self.updated_at = Utc::now();
    }

    /// Número de pasos.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// IDs de pasos completados.
    pub fn completed_ids(&self) -> Vec<PlanStepId> {
        self.steps
            .iter()
            .filter(|s| s.status == PlanStepStatus::Completed)
            .map(|s| s.id)
            .collect()
    }

    /// ¿Todos los pasos están completados?
    pub fn is_complete(&self) -> bool {
        !self.steps.is_empty()
            && self.steps.iter().all(|s| {
                s.status == PlanStepStatus::Completed || s.status == PlanStepStatus::Skipped
            })
    }

    /// Próximo paso listo para ejecutar.
    ///
    /// Devuelve el primer paso `Pending` cuyas dependencias estén
    /// completadas.
    pub fn next_ready_step(&self) -> Option<&PlanStep> {
        let completed = self.completed_ids();
        self.steps
            .iter()
            .find(|s| s.status == PlanStepStatus::Pending && s.is_ready(&completed))
    }

    /// Marca el plan como completado si todos los pasos lo están.
    pub fn refresh_status(&mut self) {
        if self.is_complete() {
            self.status = PlanStatus::Completed;
        } else if self
            .steps
            .iter()
            .any(|s| s.status == PlanStepStatus::Failed)
        {
            self.status = PlanStatus::Failed;
        } else if self
            .steps
            .iter()
            .any(|s| s.status == PlanStepStatus::Running)
        {
            self.status = PlanStatus::Running;
        } else {
            self.status = PlanStatus::Pending;
        }
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_plan_has_goal_and_no_steps() {
        let p = Plan::new("build a thing");
        assert_eq!(p.goal, "build a thing");
        assert_eq!(p.step_count(), 0);
        assert_eq!(p.status, PlanStatus::Pending);
    }

    #[test]
    fn push_step_increments_count() {
        let mut p = Plan::new("goal");
        p.push_step(PlanStep::new("step 1"));
        p.push_step(PlanStep::new("step 2"));
        assert_eq!(p.step_count(), 2);
    }

    #[test]
    fn next_ready_step_first_when_no_deps() {
        let p = Plan::with_steps(
            "goal",
            vec![PlanStep::new("first"), PlanStep::new("second")],
        );
        let ready = p.next_ready_step().unwrap();
        assert_eq!(ready.description, "first");
    }

    #[test]
    fn next_ready_step_respects_dependencies() {
        let step1 = PlanStep::new("first");
        let step1_id = step1.id;
        let step2 = PlanStep::new("second").with_depends_on(vec![step1_id]);

        let mut p = Plan::with_steps("goal", vec![step1, step2]);
        // Antes de completar step1, el ready es step1.
        assert_eq!(p.next_ready_step().unwrap().description, "first");

        // Marca step1 como completado.
        p.steps[0].mark_completed("done");

        // Ahora el ready es step2.
        assert_eq!(p.next_ready_step().unwrap().description, "second");
    }

    #[test]
    fn next_ready_step_returns_none_when_all_done() {
        let mut p = Plan::with_steps("goal", vec![PlanStep::new("only")]);
        p.steps[0].mark_completed("done");
        assert!(p.next_ready_step().is_none());
    }

    #[test]
    fn is_complete_false_when_empty() {
        let p = Plan::new("empty");
        assert!(!p.is_complete());
    }

    #[test]
    fn is_complete_true_when_all_completed_or_skipped() {
        let s1 = PlanStep::new("a");
        let s2 = PlanStep::new("b");
        let mut p = Plan::with_steps("g", vec![s1, s2]);
        p.steps[0].mark_completed("done");
        p.steps[1].status = PlanStepStatus::Skipped;
        assert!(p.is_complete());
    }

    #[test]
    fn refresh_status_completes_when_all_done() {
        let mut p = Plan::with_steps("g", vec![PlanStep::new("only")]);
        p.steps[0].mark_completed("done");
        p.refresh_status();
        assert_eq!(p.status, PlanStatus::Completed);
    }

    #[test]
    fn refresh_status_failed_when_any_failed() {
        let mut p = Plan::with_steps("g", vec![PlanStep::new("only")]);
        p.steps[0].mark_failed("boom");
        p.refresh_status();
        assert_eq!(p.status, PlanStatus::Failed);
    }

    #[test]
    fn plan_step_status_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&PlanStepStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&PlanStepStatus::Completed).unwrap(),
            "\"completed\""
        );
    }

    #[test]
    fn plan_status_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&PlanStatus::Running).unwrap(),
            "\"running\""
        );
    }

    #[test]
    fn plan_roundtrips_through_json() {
        let p = Plan::with_steps(
            "goal",
            vec![
                PlanStep::new("a").with_role(AgentRole::Coder),
                PlanStep::new("b"),
            ],
        );
        let json = serde_json::to_string(&p).unwrap();
        let back: Plan = serde_json::from_str(&json).unwrap();
        assert_eq!(back.goal, p.goal);
        assert_eq!(back.steps.len(), 2);
        assert_eq!(back.steps[0].role, Some(AgentRole::Coder));
    }

    #[test]
    fn plan_id_unique() {
        let a = PlanId::new();
        let b = PlanId::new();
        assert_ne!(a, b);
    }
}
