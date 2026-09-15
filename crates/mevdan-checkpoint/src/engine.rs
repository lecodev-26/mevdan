//! `CheckpointEngine` — crear, listar, restaurar.

use crate::{
    checkpoint::Checkpoint,
    error::{CheckpointError, CheckpointResult},
    id::CheckpointId,
    state::WorkState,
};
use std::collections::BTreeMap;

/// Motor de checkpoints en memoria.
///
/// Los checkpoints se guardan en un `BTreeMap` ordenado por ID (que
/// es UUID v7 → ordenable por tiempo).
#[derive(Debug, Default)]
pub struct CheckpointEngine {
    checkpoints: BTreeMap<CheckpointId, Checkpoint>,
}

impl CheckpointEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Crea un checkpoint a partir de un `WorkState`.
    pub fn create(&mut self, state: WorkState, label: Option<String>) -> CheckpointId {
        let mut cp = Checkpoint::new(state);
        if let Some(l) = label {
            cp = cp.with_label(l);
        }
        let id = cp.id;
        self.checkpoints.insert(id, cp);
        id
    }

    /// Lista todos los checkpoints (ordenados por ID → por tiempo).
    pub fn list(&self) -> Vec<&Checkpoint> {
        self.checkpoints.values().collect()
    }

    /// Lista los últimos N checkpoints (más recientes primero).
    pub fn list_recent(&self, n: usize) -> Vec<&Checkpoint> {
        let mut v: Vec<&Checkpoint> = self.checkpoints.values().collect();
        v.reverse();
        v.truncate(n);
        v
    }

    /// Obtiene un checkpoint por ID.
    pub fn get(&self, id: CheckpointId) -> Option<&Checkpoint> {
        self.checkpoints.get(&id)
    }

    /// Obtiene un checkpoint o error.
    pub fn require(&self, id: CheckpointId) -> CheckpointResult<&Checkpoint> {
        self.get(id)
            .ok_or_else(|| CheckpointError::NotFound(id.to_string()))
    }

    /// Devuelve una copia del `WorkState` de un checkpoint.
    pub fn restore(&self, id: CheckpointId) -> CheckpointResult<WorkState> {
        Ok(self.require(id)?.state.clone())
    }

    /// Elimina un checkpoint.
    pub fn delete(&mut self, id: CheckpointId) -> CheckpointResult<Checkpoint> {
        self.checkpoints
            .remove(&id)
            .ok_or_else(|| CheckpointError::NotFound(id.to_string()))
    }

    /// Número de checkpoints.
    pub fn len(&self) -> usize {
        self.checkpoints.len()
    }

    /// ¿Está vacío?
    pub fn is_empty(&self) -> bool {
        self.checkpoints.is_empty()
    }

    /// Elimina todos los checkpoints.
    pub fn clear(&mut self) {
        self.checkpoints.clear();
    }

    /// Busca checkpoints por etiqueta (match exacto).
    pub fn find_by_label(&self, label: &str) -> Vec<&Checkpoint> {
        self.checkpoints
            .values()
            .filter(|cp| cp.label.as_deref() == Some(label))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::WorkState;
    use mevdan_task::Task;

    fn state_with_task(title: &str) -> WorkState {
        WorkState::new().with_task(Task::new(title))
    }

    #[test]
    fn new_engine_is_empty() {
        let e = CheckpointEngine::new();
        assert!(e.is_empty());
        assert_eq!(e.len(), 0);
    }

    #[test]
    fn create_adds_checkpoint() {
        let mut e = CheckpointEngine::new();
        let id = e.create(WorkState::new(), None);
        assert_eq!(e.len(), 1);
        assert!(e.get(id).is_some());
    }

    #[test]
    fn create_with_label() {
        let mut e = CheckpointEngine::new();
        let id = e.create(WorkState::new(), Some("before-refactor".into()));
        let cp = e.get(id).unwrap();
        assert_eq!(cp.label.as_deref(), Some("before-refactor"));
    }

    #[test]
    fn list_returns_all() {
        let mut e = CheckpointEngine::new();
        e.create(WorkState::new(), Some("a".into()));
        e.create(WorkState::new(), Some("b".into()));
        e.create(WorkState::new(), Some("c".into()));
        assert_eq!(e.list().len(), 3);
    }

    #[test]
    fn list_recent_returns_latest_first() {
        let mut e = CheckpointEngine::new();
        let _a = e.create(WorkState::new(), Some("a".into()));
        std::thread::sleep(std::time::Duration::from_millis(2));
        let _b = e.create(WorkState::new(), Some("b".into()));
        std::thread::sleep(std::time::Duration::from_millis(2));
        let _c = e.create(WorkState::new(), Some("c".into()));

        let recent = e.list_recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].label.as_deref(), Some("c"));
        assert_eq!(recent[1].label.as_deref(), Some("b"));
    }

    #[test]
    fn get_unknown_returns_none() {
        let e = CheckpointEngine::new();
        assert!(e.get(CheckpointId::new()).is_none());
    }

    #[test]
    fn require_unknown_fails() {
        let e = CheckpointEngine::new();
        let err = e.require(CheckpointId::new()).unwrap_err();
        assert!(matches!(err, CheckpointError::NotFound(_)));
    }

    #[test]
    fn restore_returns_state() {
        let mut e = CheckpointEngine::new();
        let id = e.create(state_with_task("t1"), None);

        let restored = e.restore(id).unwrap();
        assert_eq!(restored.tasks.len(), 1);
        assert_eq!(restored.tasks[0].title, "t1");
    }

    #[test]
    fn delete_removes() {
        let mut e = CheckpointEngine::new();
        let id = e.create(WorkState::new(), None);
        e.delete(id).unwrap();
        assert!(e.is_empty());
    }

    #[test]
    fn delete_unknown_fails() {
        let mut e = CheckpointEngine::new();
        let err = e.delete(CheckpointId::new()).unwrap_err();
        assert!(matches!(err, CheckpointError::NotFound(_)));
    }

    #[test]
    fn clear_removes_all() {
        let mut e = CheckpointEngine::new();
        e.create(WorkState::new(), None);
        e.create(WorkState::new(), None);
        e.clear();
        assert!(e.is_empty());
    }

    #[test]
    fn find_by_label_filters() {
        let mut e = CheckpointEngine::new();
        e.create(WorkState::new(), Some("alpha".into()));
        e.create(WorkState::new(), Some("beta".into()));
        e.create(WorkState::new(), Some("alpha".into()));

        let found = e.find_by_label("alpha");
        assert_eq!(found.len(), 2);

        let found = e.find_by_label("gamma");
        assert_eq!(found.len(), 0);
    }

    #[test]
    fn realistic_checkpoint_workflow() {
        let mut e = CheckpointEngine::new();

        // 1. Estado inicial.
        let s1 = WorkState::new().with_task(Task::new("setup"));
        let cp1 = e.create(s1, Some("initial".into()));

        // 2. Avanzamos el trabajo.
        let s2 = WorkState::new()
            .with_task(Task::new("setup"))
            .with_task(Task::new("implement"));
        let cp2 = e.create(s2, Some("after-setup".into()));

        // 3. Algo falla y queremos volver.
        let restored = e.restore(cp1).unwrap();
        assert_eq!(restored.tasks.len(), 1);

        // Pero también podemos volver al otro.
        let restored = e.restore(cp2).unwrap();
        assert_eq!(restored.tasks.len(), 2);

        // Y vemos el historial.
        let recent = e.list_recent(10);
        assert_eq!(recent.len(), 2);
    }
}
