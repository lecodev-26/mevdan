//! `WorkState` — snapshot del trabajo.

use mevdan_task::Task;
use mevdan_verification::{Artifact, Claim};
use mevdan_workgraph::WorkGraph;
use serde::{Deserialize, Serialize};

/// Versión del esquema de `WorkState`. Incrementar si cambia el
/// formato de forma incompatible.
pub const WORK_STATE_SCHEMA_VERSION: &str = "0.1.0";

/// Snapshot del trabajo en un momento dado.
///
/// Contiene todo lo necesario para restaurar el estado:
/// - El Work Graph completo.
/// - Las tareas (serializadas de `TaskEngine`).
/// - Los artefactos producidos.
/// - Los claims emitidos.
/// - Metadatos libres (commit hash, mensajes, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkState {
    /// Versión del esquema.
    pub schema_version: String,

    /// Work Graph.
    #[serde(default)]
    pub workgraph: WorkGraph,

    /// Tareas (lista plana; el engine las reconstruye).
    #[serde(default)]
    pub tasks: Vec<Task>,

    /// Artefactos producidos.
    #[serde(default)]
    pub artifacts: Vec<Artifact>,

    /// Claims emitidos.
    #[serde(default)]
    pub claims: Vec<Claim>,

    /// Metadatos libres (commit, branch, notas, etc.).
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl Default for WorkState {
    fn default() -> Self {
        Self {
            schema_version: WORK_STATE_SCHEMA_VERSION.to_string(),
            workgraph: WorkGraph::new(),
            tasks: Vec::new(),
            artifacts: Vec::new(),
            claims: Vec::new(),
            metadata: serde_json::Value::Null,
        }
    }
}

impl WorkState {
    /// Crea un `WorkState` vacío.
    pub fn new() -> Self {
        Self::default()
    }

    /// Añade una tarea.
    pub fn with_task(mut self, task: Task) -> Self {
        self.tasks.push(task);
        self
    }

    /// Añade un artefacto.
    pub fn with_artifact(mut self, artifact: Artifact) -> Self {
        self.artifacts.push(artifact);
        self
    }

    /// Añade un claim.
    pub fn with_claim(mut self, claim: Claim) -> Self {
        self.claims.push(claim);
        self
    }

    /// Reemplaza el Work Graph.
    pub fn with_workgraph(mut self, wg: WorkGraph) -> Self {
        self.workgraph = wg;
        self
    }

    /// Añade metadatos.
    pub fn with_metadata(mut self, m: serde_json::Value) -> Self {
        self.metadata = m;
        self
    }

    /// Cuenta total de elementos.
    pub fn total_items(&self) -> usize {
        self.workgraph.node_count() + self.tasks.len() + self.artifacts.len() + self.claims.len()
    }

    /// ¿Está vacío?
    pub fn is_empty(&self) -> bool {
        self.total_items() == 0
    }

    /// Serializa a JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserializa de JSON.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mevdan_workgraph::Node;

    #[test]
    fn default_state_is_empty() {
        let s = WorkState::default();
        assert_eq!(s.schema_version, WORK_STATE_SCHEMA_VERSION);
        assert!(s.is_empty());
        assert_eq!(s.total_items(), 0);
    }

    #[test]
    fn with_task_adds_task() {
        let s = WorkState::new().with_task(Task::new("do something"));
        assert_eq!(s.tasks.len(), 1);
        assert_eq!(s.total_items(), 1);
    }

    #[test]
    fn with_artifact_adds_artifact() {
        let s = WorkState::new().with_artifact(Artifact::from_text("t", "x"));
        assert_eq!(s.artifacts.len(), 1);
    }

    #[test]
    fn with_claim_adds_claim() {
        let s = WorkState::new().with_claim(Claim::file_created("/x"));
        assert_eq!(s.claims.len(), 1);
    }

    #[test]
    fn with_workgraph_replaces() {
        let mut wg = WorkGraph::new();
        wg.add_node(Node::goal("g")).unwrap();
        let s = WorkState::new().with_workgraph(wg);
        assert_eq!(s.workgraph.node_count(), 1);
    }

    #[test]
    fn with_metadata_sets_it() {
        let s = WorkState::new().with_metadata(serde_json::json!({"commit": "abc"}));
        assert_eq!(s.metadata["commit"], "abc");
    }

    #[test]
    fn total_items_sums_everything() {
        let mut wg = WorkGraph::new();
        wg.add_node(Node::goal("g")).unwrap();
        wg.add_node(Node::task("t")).unwrap();

        let s = WorkState::new()
            .with_workgraph(wg)
            .with_task(Task::new("a"))
            .with_artifact(Artifact::from_text("b", "x"))
            .with_claim(Claim::file_created("/c"));

        assert_eq!(s.total_items(), 5);
    }

    #[test]
    fn state_roundtrips_through_json() {
        let s = WorkState::new()
            .with_task(Task::new("t"))
            .with_artifact(Artifact::from_text("a", "content"))
            .with_metadata(serde_json::json!({"key": "value"}));

        let json = s.to_json().unwrap();
        let back = WorkState::from_json(&json).unwrap();

        assert_eq!(back.tasks.len(), 1);
        assert_eq!(back.artifacts.len(), 1);
        assert_eq!(back.metadata["key"], "value");
    }
}
