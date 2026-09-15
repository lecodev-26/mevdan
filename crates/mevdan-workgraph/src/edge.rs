//! Aristas del Work Graph.
//!
//! Una **arista** conecta dos nodos y describe la relación entre ellos.

use crate::node::NodeId;
use serde::{Deserialize, Serialize};

/// Tipo de relación entre dos nodos.
///
/// `PartialOrd` y `Ord` están derivados para permitir usar `EdgeKind`
/// como clave de `BTreeMap` (por ejemplo, en `GraphStats`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// `A` depende de `B` (no se puede hacer A hasta que B esté hecho).
    DependsOn,
    /// `A` produce `B` (artefacto, evidencia).
    Produces,
    /// `A` verifica `B`.
    Verifies,
    /// `A` es parte de `B` (sub-tarea, sub-requisito).
    PartOf,
    /// `A` satisface `B` (una tarea satisface un requisito).
    Satisfies,
    /// `A` precedió a `B` temporalmente (secuencia).
    Precedes,
    /// `A` refina `B` (más detallado).
    Refines,
}

impl EdgeKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            EdgeKind::DependsOn => "depends_on",
            EdgeKind::Produces => "produces",
            EdgeKind::Verifies => "verifies",
            EdgeKind::PartOf => "part_of",
            EdgeKind::Satisfies => "satisfies",
            EdgeKind::Precedes => "precedes",
            EdgeKind::Refines => "refines",
        }
    }

    /// ¿Esta relación crea dependencias ordenadas (ciclos importan)?
    pub fn is_ordering(&self) -> bool {
        matches!(
            self,
            EdgeKind::DependsOn | EdgeKind::Precedes | EdgeKind::PartOf | EdgeKind::Refines
        )
    }
}

/// Una arista dirigida: `from` → `to` con un tipo.
///
/// `PartialOrd` y `Ord` están derivados para poder almacenar `Edge` en
/// un `BTreeSet<Edge>` (deduplicación + orden determinista).
///
/// El orden natural sigue el orden de declaración de los campos
/// (`from`, `to`, `kind`), lo cual es útil para tests que dependen de
/// iteración estable.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub kind: EdgeKind,
}

impl Edge {
    /// Crea una arista.
    pub fn new(from: NodeId, to: NodeId, kind: EdgeKind) -> Self {
        Self { from, to, kind }
    }

    /// ¿Es un auto-loop?
    pub fn is_self_loop(&self) -> bool {
        self.from == self.to
    }

    /// Clave única para deduplicar: (from, to, kind).
    pub fn key(&self) -> (NodeId, NodeId, EdgeKind) {
        (self.from, self.to, self.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_kind_display_names() {
        assert_eq!(EdgeKind::DependsOn.display_name(), "depends_on");
        assert_eq!(EdgeKind::Produces.display_name(), "produces");
    }

    #[test]
    fn edge_kind_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&EdgeKind::DependsOn).unwrap(),
            "\"depends_on\""
        );
        assert_eq!(
            serde_json::to_string(&EdgeKind::PartOf).unwrap(),
            "\"part_of\""
        );
    }

    #[test]
    fn ordering_kinds() {
        assert!(EdgeKind::DependsOn.is_ordering());
        assert!(EdgeKind::Precedes.is_ordering());
        assert!(EdgeKind::PartOf.is_ordering());
        assert!(EdgeKind::Refines.is_ordering());
        assert!(!EdgeKind::Produces.is_ordering());
        assert!(!EdgeKind::Verifies.is_ordering());
        assert!(!EdgeKind::Satisfies.is_ordering());
    }

    #[test]
    fn self_loop_detection() {
        let a = NodeId::new();
        let b = NodeId::new();
        assert!(Edge::new(a, a, EdgeKind::DependsOn).is_self_loop());
        assert!(!Edge::new(a, b, EdgeKind::DependsOn).is_self_loop());
    }

    #[test]
    fn edge_key_is_stable() {
        let a = NodeId::new();
        let b = NodeId::new();
        let e1 = Edge::new(a, b, EdgeKind::DependsOn);
        let e2 = Edge::new(a, b, EdgeKind::DependsOn);
        assert_eq!(e1.key(), e2.key());
    }

    #[test]
    fn edge_key_distinguishes_kind() {
        let a = NodeId::new();
        let b = NodeId::new();
        let e1 = Edge::new(a, b, EdgeKind::DependsOn);
        let e2 = Edge::new(a, b, EdgeKind::Produces);
        assert_ne!(e1.key(), e2.key());
    }

    #[test]
    fn edge_roundtrips() {
        let a = NodeId::new();
        let b = NodeId::new();
        let e = Edge::new(a, b, EdgeKind::Satisfies);
        let json = serde_json::to_string(&e).unwrap();
        let back: Edge = serde_json::from_str(&json).unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn edge_can_be_used_in_btreeset() {
        use std::collections::BTreeSet;
        let a = NodeId::new();
        let b = NodeId::new();
        let mut set: BTreeSet<Edge> = BTreeSet::new();
        set.insert(Edge::new(a, b, EdgeKind::DependsOn));
        set.insert(Edge::new(a, b, EdgeKind::DependsOn)); // duplicado, no cuenta.
        set.insert(Edge::new(a, b, EdgeKind::Produces));
        assert_eq!(set.len(), 2);
    }
}
