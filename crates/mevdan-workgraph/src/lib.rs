//! # mevdan-workgraph
//!
//! Work Graph: la estructura del trabajo en MEVDAN.
//!
//! ## Concepto
//!
//! En vez de "chat + respuesta", MEVDAN modela el trabajo como un
//! **grafo dirigido de nodos tipados**.
//!
//! ```text
//! Goal
//!  ├── Requirement
//!  ├── Constraint
//!  ├── Task
//!  │    ├── Action
//!  │    ├── Artifact
//!  │    └── Evidence
//!  ├── Verification
//!  ├── Decision
//!  └── Checkpoint
//! ```
//!
//! ## Estado del proyecto
//!
//! - **20.1** ✅ — cimientos: `NodeId`, `NodeKind`, `Node`, `Edge`,
//!   `EdgeKind`.
//! - **20.2** ✅ — `WorkGraph` con construcción, traversal, ciclos,
//!   topological order, stats.
//! - **20.3** ⏳ — persistencia con `mevdan-storage`.
//!
//! ## Reglas
//!
//! 1. **Un nodo es un nodo.** No hay subclases. `kind` + `data`
//!    distinguen.
//! 2. **Aristas tipadas.** Cada relación tiene su `EdgeKind`.
//! 3. **Sin ciclos en relaciones de orden** (`DependsOn`, `Precedes`,
//!    `PartOf`, `Refines`).
//! 4. **Inmutable respecto a IDs.** Un `NodeId` nunca cambia.

pub mod edge;
pub mod error;
pub mod graph;
pub mod node;

// Re-exports de conveniencia.
pub use edge::{Edge, EdgeKind};
pub use error::{WorkGraphError, WorkGraphResult};
pub use graph::{GraphStats, WorkGraph};
pub use node::{Node, NodeId, NodeKind};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_flow_build_simple_goal() {
        let mut graph = WorkGraph::new();

        let goal = graph
            .add_node(Node::goal("Build a hello-world CLI"))
            .unwrap();
        let req = graph
            .add_node(Node::requirement("Must print 'hello world'"))
            .unwrap();
        let task = graph.add_node(Node::task("Write main.rs")).unwrap();
        let artifact = graph.add_node(Node::artifact("src/main.rs")).unwrap();
        let evidence = graph
            .add_node(Node::evidence("cargo build succeeds"))
            .unwrap();

        graph
            .add_edge(Edge::new(req, goal, EdgeKind::PartOf))
            .unwrap();
        graph
            .add_edge(Edge::new(task, req, EdgeKind::Satisfies))
            .unwrap();
        graph
            .add_edge(Edge::new(task, artifact, EdgeKind::Produces))
            .unwrap();
        graph
            .add_edge(Edge::new(evidence, task, EdgeKind::Verifies))
            .unwrap();

        assert_eq!(graph.node_count(), 5);
        assert_eq!(graph.edge_count(), 4);
        assert_eq!(graph.stats().nodes_by_kind.get(&NodeKind::Goal), Some(&1));
    }

    #[test]
    fn kinds_and_edges_coexist() {
        let nodes = [
            Node::goal("g"),
            Node::requirement("r"),
            Node::constraint("c"),
            Node::task("t"),
            Node::action("a"),
            Node::artifact("ar"),
            Node::evidence("e"),
            Node::verification("v"),
            Node::decision("d"),
            Node::checkpoint("cp"),
        ];
        assert_eq!(nodes.len(), 10);

        let a = NodeId::new();
        let b = NodeId::new();
        let edges = [
            Edge::new(a, b, EdgeKind::DependsOn),
            Edge::new(a, b, EdgeKind::Produces),
            Edge::new(a, b, EdgeKind::Verifies),
            Edge::new(a, b, EdgeKind::PartOf),
            Edge::new(a, b, EdgeKind::Satisfies),
            Edge::new(a, b, EdgeKind::Precedes),
            Edge::new(a, b, EdgeKind::Refines),
        ];
        assert_eq!(edges.len(), 7);
    }
}
