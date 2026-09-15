//! Nodos del Work Graph.
//!
//! Un **nodo** representa un elemento del trabajo. Puede ser un
//! objetivo, un requisito, una tarea, un artefacto, etc.
//!
//! ## Modelo de property graph
//!
//! En vez de tener 8 structs distintos (Goal, Requirement, Task...),
//! usamos **un solo tipo `Node`** con un campo `kind` que distingue.
//! Cada `Node` tiene:
//! - `id` único.
//! - `kind` (tipo).
//! - `label` legible.
//! - `data` JSON libre (payload específico del kind).
//! - `created_at`, `updated_at`.
//!
//! Ventajas:
//! - Flexible: añadir un campo específico no rompe el tipo.
//! - Persistible: se serializa entero.
//! - Consultable: edges genéricos funcionan con cualquier nodo.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// ID único de un nodo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Tipo de nodo.
///
/// Regla: AÑADIR variantes. Nunca eliminar ni renombrar una existente
/// para no romper datos históricos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// Objetivo de alto nivel (lo que el usuario quiere lograr).
    Goal,
    /// Requisito: qué debe cumplirse.
    Requirement,
    /// Restricción: qué NO se puede hacer.
    Constraint,
    /// Tarea: unidad de trabajo ejecutable.
    Task,
    /// Acción concreta (invocación a tool, mensaje al modelo).
    Action,
    /// Artefacto producido (archivo, documento, imagen...).
    Artifact,
    /// Evidencia de que algo se hizo (para verificación).
    Evidence,
    /// Verificación: resultado de comprobar algo.
    Verification,
    /// Decisión tomada (arquitectura, diseño, política).
    Decision,
    /// Checkpoint: snapshot del trabajo.
    Checkpoint,
}

impl NodeKind {
    /// Nombre legible.
    pub fn display_name(&self) -> &'static str {
        match self {
            NodeKind::Goal => "Goal",
            NodeKind::Requirement => "Requirement",
            NodeKind::Constraint => "Constraint",
            NodeKind::Task => "Task",
            NodeKind::Action => "Action",
            NodeKind::Artifact => "Artifact",
            NodeKind::Evidence => "Evidence",
            NodeKind::Verification => "Verification",
            NodeKind::Decision => "Decision",
            NodeKind::Checkpoint => "Checkpoint",
        }
    }

    /// ¿Es un nodo terminal (no debería tener hijos)?
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            NodeKind::Artifact | NodeKind::Evidence | NodeKind::Verification
        )
    }

    /// ¿Es un nodo "vivo" (el trabajo puede continuar a partir de él)?
    pub fn is_active(&self) -> bool {
        matches!(self, NodeKind::Goal | NodeKind::Task | NodeKind::Decision)
    }
}

/// Un nodo del Work Graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    /// Etiqueta legible corta (ej. "Create CLI parser").
    pub label: String,
    /// Payload específico del kind (JSON libre).
    #[serde(default)]
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Node {
    /// Crea un nodo nuevo.
    pub fn new(kind: NodeKind, label: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: NodeId::new(),
            kind,
            label: label.into(),
            data: serde_json::Value::Null,
            created_at: now,
            updated_at: now,
        }
    }

    /// Añade un payload.
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    /// Atajo: nodo `Goal`.
    pub fn goal(label: impl Into<String>) -> Self {
        Self::new(NodeKind::Goal, label)
    }

    /// Atajo: nodo `Task`.
    pub fn task(label: impl Into<String>) -> Self {
        Self::new(NodeKind::Task, label)
    }

    /// Atajo: nodo `Requirement`.
    pub fn requirement(label: impl Into<String>) -> Self {
        Self::new(NodeKind::Requirement, label)
    }

    /// Atajo: nodo `Artifact`.
    pub fn artifact(label: impl Into<String>) -> Self {
        Self::new(NodeKind::Artifact, label)
    }

    /// Atajo: nodo `Evidence`.
    pub fn evidence(label: impl Into<String>) -> Self {
        Self::new(NodeKind::Evidence, label)
    }

    /// Atajo: nodo `Verification`.
    pub fn verification(label: impl Into<String>) -> Self {
        Self::new(NodeKind::Verification, label)
    }

    /// Atajo: nodo `Decision`.
    pub fn decision(label: impl Into<String>) -> Self {
        Self::new(NodeKind::Decision, label)
    }

    /// Atajo: nodo `Constraint`.
    pub fn constraint(label: impl Into<String>) -> Self {
        Self::new(NodeKind::Constraint, label)
    }

    /// Atajo: nodo `Action`.
    pub fn action(label: impl Into<String>) -> Self {
        Self::new(NodeKind::Action, label)
    }

    /// Atajo: nodo `Checkpoint`.
    pub fn checkpoint(label: impl Into<String>) -> Self {
        Self::new(NodeKind::Checkpoint, label)
    }

    /// Actualiza `updated_at`.
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// ¿Es un nodo vacío (label en blanco)?
    pub fn is_blank(&self) -> bool {
        self.label.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_ids_unique() {
        let a = NodeId::new();
        let b = NodeId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn node_id_displays_uuid() {
        let id = NodeId::new();
        let s = id.to_string();
        assert!(s.contains('-'));
    }

    #[test]
    fn kind_display_names() {
        assert_eq!(NodeKind::Goal.display_name(), "Goal");
        assert_eq!(NodeKind::Task.display_name(), "Task");
        assert_eq!(NodeKind::Verification.display_name(), "Verification");
    }

    #[test]
    fn kind_serializes_snake_case() {
        assert_eq!(serde_json::to_string(&NodeKind::Goal).unwrap(), "\"goal\"");
        assert_eq!(
            serde_json::to_string(&NodeKind::Requirement).unwrap(),
            "\"requirement\""
        );
    }

    #[test]
    fn kind_is_terminal() {
        assert!(NodeKind::Artifact.is_terminal());
        assert!(NodeKind::Evidence.is_terminal());
        assert!(NodeKind::Verification.is_terminal());
        assert!(!NodeKind::Goal.is_terminal());
        assert!(!NodeKind::Task.is_terminal());
    }

    #[test]
    fn kind_is_active() {
        assert!(NodeKind::Goal.is_active());
        assert!(NodeKind::Task.is_active());
        assert!(NodeKind::Decision.is_active());
        assert!(!NodeKind::Artifact.is_active());
    }

    #[test]
    fn new_node_has_defaults() {
        let n = Node::new(NodeKind::Task, "test task");
        assert_eq!(n.kind, NodeKind::Task);
        assert_eq!(n.label, "test task");
        assert_eq!(n.data, serde_json::Value::Null);
        assert_eq!(n.created_at, n.updated_at);
    }

    #[test]
    fn with_data_sets_payload() {
        let n = Node::task("t").with_data(serde_json::json!({"x": 1}));
        assert_eq!(n.data["x"], 1);
    }

    #[test]
    fn constructors_set_correct_kind() {
        assert_eq!(Node::goal("g").kind, NodeKind::Goal);
        assert_eq!(Node::task("t").kind, NodeKind::Task);
        assert_eq!(Node::requirement("r").kind, NodeKind::Requirement);
        assert_eq!(Node::artifact("a").kind, NodeKind::Artifact);
        assert_eq!(Node::evidence("e").kind, NodeKind::Evidence);
        assert_eq!(Node::verification("v").kind, NodeKind::Verification);
        assert_eq!(Node::decision("d").kind, NodeKind::Decision);
        assert_eq!(Node::constraint("c").kind, NodeKind::Constraint);
        assert_eq!(Node::action("a").kind, NodeKind::Action);
        assert_eq!(Node::checkpoint("c").kind, NodeKind::Checkpoint);
    }

    #[test]
    fn touch_updates_timestamp() {
        let mut n = Node::task("t");
        let original = n.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(5));
        n.touch();
        assert!(n.updated_at > original);
        assert_eq!(n.created_at, original);
    }

    #[test]
    fn is_blank_works() {
        assert!(Node::task("").is_blank());
        assert!(Node::task("   ").is_blank());
        assert!(!Node::task("ok").is_blank());
    }

    #[test]
    fn node_roundtrips() {
        let n = Node::task("build feature").with_data(serde_json::json!({"priority": "high"}));
        let json = serde_json::to_string(&n).unwrap();
        let back: Node = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, n.id);
        assert_eq!(back.kind, n.kind);
        assert_eq!(back.label, n.label);
        assert_eq!(back.data, n.data);
    }
}
