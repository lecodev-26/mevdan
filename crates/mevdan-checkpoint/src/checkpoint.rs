//! `Checkpoint` — snapshot con metadata.

use crate::{id::CheckpointId, state::WorkState};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Un checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: CheckpointId,
    /// Etiqueta legible (ej. "before-refactor").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Estado del trabajo en el momento del snapshot.
    pub state: WorkState,
    pub created_at: DateTime<Utc>,
}

impl Checkpoint {
    /// Crea un checkpoint.
    pub fn new(state: WorkState) -> Self {
        Self {
            id: CheckpointId::new(),
            label: None,
            state,
            created_at: Utc::now(),
        }
    }

    /// Sobrescribe la etiqueta.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Etiqueta o representación corta del ID (para logs).
    pub fn display_label(&self) -> String {
        match &self.label {
            Some(l) => l.clone(),
            None => {
                let s = self.id.to_string();
                format!("checkpoint-{}", &s[..8.min(s.len())])
            }
        }
    }

    /// Resumen breve.
    pub fn summary(&self) -> String {
        format!(
            "{} ({} items, {})",
            self.display_label(),
            self.state.total_items(),
            self.created_at.format("%Y-%m-%d %H:%M:%S")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::WorkState;
    use mevdan_task::Task;

    #[test]
    fn new_checkpoint_has_defaults() {
        let cp = Checkpoint::new(WorkState::new());
        assert!(cp.label.is_none());
        assert!(cp.state.is_empty());
    }

    #[test]
    fn with_label_sets_it() {
        let cp = Checkpoint::new(WorkState::new()).with_label("v1");
        assert_eq!(cp.label.as_deref(), Some("v1"));
    }

    #[test]
    fn display_label_uses_label_if_present() {
        let cp = Checkpoint::new(WorkState::new()).with_label("test");
        assert_eq!(cp.display_label(), "test");
    }

    #[test]
    fn display_label_falls_back_to_id() {
        let cp = Checkpoint::new(WorkState::new());
        let l = cp.display_label();
        assert!(l.starts_with("checkpoint-"));
    }

    #[test]
    fn summary_contains_info() {
        let cp = Checkpoint::new(WorkState::new().with_task(Task::new("t"))).with_label("test");
        let s = cp.summary();
        assert!(s.contains("test"));
        assert!(s.contains("1 items"));
    }

    #[test]
    fn checkpoint_roundtrips() {
        let cp = Checkpoint::new(WorkState::new().with_task(Task::new("t"))).with_label("v1");
        let json = serde_json::to_string(&cp).unwrap();
        let back: Checkpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, cp.id);
        assert_eq!(back.label, cp.label);
        assert_eq!(back.state.tasks.len(), 1);
    }
}
