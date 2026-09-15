//! `WorkGraph` — la estructura del trabajo.

use crate::{
    edge::{Edge, EdgeKind},
    error::{WorkGraphError, WorkGraphResult},
    node::{Node, NodeId, NodeKind},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};

/// El Work Graph completo.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkGraph {
    nodes: BTreeMap<NodeId, Node>,
    edges: BTreeSet<Edge>,
}

impl WorkGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: Node) -> WorkGraphResult<NodeId> {
        if node.is_blank() {
            return Err(WorkGraphError::EmptyLabel(
                node.kind.display_name().to_string(),
            ));
        }
        let id = node.id;
        if self.nodes.contains_key(&id) {
            return Err(WorkGraphError::DuplicateNode(id.to_string()));
        }
        self.nodes.insert(id, node);
        Ok(id)
    }

    pub fn get_node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(&id)
    }

    pub fn require_node(&self, id: NodeId) -> WorkGraphResult<&Node> {
        self.get_node(id)
            .ok_or_else(|| WorkGraphError::NodeNotFound(id.to_string()))
    }

    pub fn remove_node(&mut self, id: NodeId) -> WorkGraphResult<Node> {
        let node = self
            .nodes
            .remove(&id)
            .ok_or_else(|| WorkGraphError::NodeNotFound(id.to_string()))?;
        self.edges.retain(|e| e.from != id && e.to != id);
        Ok(node)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    pub fn node_ids(&self) -> Vec<NodeId> {
        self.nodes.keys().copied().collect()
    }

    pub fn nodes_by_kind(&self, kind: NodeKind) -> Vec<&Node> {
        self.nodes.values().filter(|n| n.kind == kind).collect()
    }

    pub fn add_edge(&mut self, edge: Edge) -> WorkGraphResult<()> {
        if edge.is_self_loop() {
            return Err(WorkGraphError::InvalidEdge("self-loop".into()));
        }
        if !self.nodes.contains_key(&edge.from) {
            return Err(WorkGraphError::NodeNotFound(edge.from.to_string()));
        }
        if !self.nodes.contains_key(&edge.to) {
            return Err(WorkGraphError::NodeNotFound(edge.to.to_string()));
        }
        if self.edges.contains(&edge) {
            return Ok(());
        }
        if edge.kind.is_ordering() && self.would_create_cycle(edge.from, edge.to) {
            return Err(WorkGraphError::CycleDetected {
                from: edge.from.to_string(),
                to: edge.to.to_string(),
            });
        }
        self.edges.insert(edge);
        Ok(())
    }

    fn would_create_cycle(&self, from: NodeId, to: NodeId) -> bool {
        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut queue: VecDeque<NodeId> = VecDeque::new();
        queue.push_back(to);
        visited.insert(to);

        while let Some(current) = queue.pop_front() {
            if current == from {
                return true;
            }
            for edge in self.outgoing_ordering(current) {
                if visited.insert(edge.to) {
                    queue.push_back(edge.to);
                }
            }
        }
        false
    }

    pub fn remove_edge(&mut self, edge: &Edge) -> bool {
        self.edges.remove(edge)
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.edges.iter()
    }

    pub fn outgoing(&self, id: NodeId) -> Vec<&Edge> {
        self.edges.iter().filter(|e| e.from == id).collect()
    }

    pub fn incoming(&self, id: NodeId) -> Vec<&Edge> {
        self.edges.iter().filter(|e| e.to == id).collect()
    }

    fn outgoing_ordering(&self, id: NodeId) -> Vec<&Edge> {
        self.edges
            .iter()
            .filter(|e| e.from == id && e.kind.is_ordering())
            .collect()
    }

    pub fn neighbors(&self, id: NodeId) -> Vec<NodeId> {
        let mut set: BTreeSet<NodeId> = BTreeSet::new();
        for e in self.edges.iter() {
            if e.from == id {
                set.insert(e.to);
            }
            if e.to == id {
                set.insert(e.from);
            }
        }
        set.into_iter().collect()
    }

    pub fn dependencies(&self, id: NodeId) -> Vec<NodeId> {
        self.edges
            .iter()
            .filter(|e| e.from == id && e.kind == EdgeKind::DependsOn)
            .map(|e| e.to)
            .collect()
    }

    pub fn dependents(&self, id: NodeId) -> Vec<NodeId> {
        self.edges
            .iter()
            .filter(|e| e.to == id && e.kind == EdgeKind::DependsOn)
            .map(|e| e.from)
            .collect()
    }

    pub fn descendants(&self, id: NodeId) -> Vec<NodeId> {
        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut queue: VecDeque<NodeId> = VecDeque::new();
        let mut result: Vec<NodeId> = Vec::new();

        for e in self.edges.iter().filter(|e| e.from == id) {
            if visited.insert(e.to) {
                queue.push_back(e.to);
                result.push(e.to);
            }
        }

        while let Some(current) = queue.pop_front() {
            for e in self.edges.iter().filter(|e| e.from == current) {
                if visited.insert(e.to) {
                    queue.push_back(e.to);
                    result.push(e.to);
                }
            }
        }

        result
    }

    pub fn ancestors(&self, id: NodeId) -> Vec<NodeId> {
        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut queue: VecDeque<NodeId> = VecDeque::new();
        let mut result: Vec<NodeId> = Vec::new();

        for e in self.edges.iter().filter(|e| e.to == id) {
            if visited.insert(e.from) {
                queue.push_back(e.from);
                result.push(e.from);
            }
        }

        while let Some(current) = queue.pop_front() {
            for e in self.edges.iter().filter(|e| e.to == current) {
                if visited.insert(e.from) {
                    queue.push_back(e.from);
                    result.push(e.from);
                }
            }
        }

        result
    }

    pub fn roots(&self) -> Vec<NodeId> {
        self.nodes
            .keys()
            .copied()
            .filter(|id| {
                !self
                    .edges
                    .iter()
                    .any(|e| e.to == *id && e.kind.is_ordering())
            })
            .collect()
    }

    pub fn leaves(&self) -> Vec<NodeId> {
        self.nodes
            .keys()
            .copied()
            .filter(|id| {
                !self
                    .edges
                    .iter()
                    .any(|e| e.from == *id && e.kind.is_ordering())
            })
            .collect()
    }

    pub fn topological_order(&self) -> Option<Vec<NodeId>> {
        let mut in_degree: BTreeMap<NodeId, usize> = BTreeMap::new();
        for id in self.nodes.keys() {
            in_degree.insert(*id, 0);
        }
        for e in self.edges.iter() {
            if e.kind.is_ordering() {
                if let Some(d) = in_degree.get_mut(&e.to) {
                    *d += 1;
                }
            }
        }

        let mut queue: VecDeque<NodeId> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut result: Vec<NodeId> = Vec::with_capacity(self.nodes.len());

        while let Some(id) = queue.pop_front() {
            result.push(id);
            for e in self.outgoing_ordering(id) {
                if let Some(d) = in_degree.get_mut(&e.to) {
                    *d -= 1;
                    if *d == 0 {
                        queue.push_back(e.to);
                    }
                }
            }
        }

        if result.len() == self.nodes.len() {
            Some(result)
        } else {
            None
        }
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn stats(&self) -> GraphStats {
        let mut by_kind: BTreeMap<NodeKind, usize> = BTreeMap::new();
        for n in self.nodes.values() {
            *by_kind.entry(n.kind).or_insert(0) += 1;
        }

        let mut by_edge_kind: BTreeMap<EdgeKind, usize> = BTreeMap::new();
        for e in self.edges.iter() {
            *by_edge_kind.entry(e.kind).or_insert(0) += 1;
        }

        GraphStats {
            node_count: self.nodes.len(),
            edge_count: self.edges.len(),
            nodes_by_kind: by_kind,
            edges_by_kind: by_edge_kind,
        }
    }
}

/// Estadísticas del grafo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub nodes_by_kind: BTreeMap<NodeKind, usize>,
    pub edges_by_kind: BTreeMap<EdgeKind, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g() -> WorkGraph {
        WorkGraph::new()
    }

    #[test]
    fn new_graph_is_empty() {
        let graph = g();
        assert!(graph.is_empty());
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn add_node_returns_id() {
        let mut graph = g();
        let id = graph.add_node(Node::goal("test goal")).unwrap();
        assert!(graph.get_node(id).is_some());
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn add_node_with_empty_label_fails() {
        let mut graph = g();
        let err = graph.add_node(Node::goal("")).unwrap_err();
        assert!(matches!(err, WorkGraphError::EmptyLabel(_)));
    }

    #[test]
    fn add_node_with_whitespace_label_fails() {
        let mut graph = g();
        let err = graph.add_node(Node::task("   ")).unwrap_err();
        assert!(matches!(err, WorkGraphError::EmptyLabel(_)));
    }

    #[test]
    fn add_duplicate_node_fails() {
        let mut graph = g();
        let node = Node::goal("test");
        let id = node.id;
        graph.add_node(node).unwrap();

        let duplicate = Node {
            id,
            ..Node::goal("other")
        };
        let err = graph.add_node(duplicate).unwrap_err();
        assert!(matches!(err, WorkGraphError::DuplicateNode(_)));
    }

    #[test]
    fn get_node_unknown_returns_none() {
        let graph = g();
        assert!(graph.get_node(NodeId::new()).is_none());
    }

    #[test]
    fn require_node_unknown_fails() {
        let graph = g();
        let err = graph.require_node(NodeId::new()).unwrap_err();
        assert!(matches!(err, WorkGraphError::NodeNotFound(_)));
    }

    #[test]
    fn get_node_mut_allows_edit() {
        let mut graph = g();
        let id = graph.add_node(Node::task("original")).unwrap();
        if let Some(node) = graph.get_node_mut(id) {
            node.label = "modified".into();
            node.touch();
        }
        assert_eq!(graph.get_node(id).unwrap().label, "modified");
    }

    #[test]
    fn remove_node_removes_its_edges() {
        let mut graph = g();
        let a = graph.add_node(Node::task("a")).unwrap();
        let b = graph.add_node(Node::task("b")).unwrap();
        graph
            .add_edge(Edge::new(a, b, EdgeKind::DependsOn))
            .unwrap();

        assert_eq!(graph.edge_count(), 1);
        graph.remove_node(a).unwrap();
        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn remove_unknown_node_fails() {
        let mut graph = g();
        let err = graph.remove_node(NodeId::new()).unwrap_err();
        assert!(matches!(err, WorkGraphError::NodeNotFound(_)));
    }

    #[test]
    fn add_edge_between_existing_nodes() {
        let mut graph = g();
        let a = graph.add_node(Node::task("a")).unwrap();
        let b = graph.add_node(Node::task("b")).unwrap();
        graph
            .add_edge(Edge::new(a, b, EdgeKind::DependsOn))
            .unwrap();
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn add_edge_with_missing_from_fails() {
        let mut graph = g();
        let b = graph.add_node(Node::task("b")).unwrap();
        let fake = NodeId::new();
        let err = graph
            .add_edge(Edge::new(fake, b, EdgeKind::DependsOn))
            .unwrap_err();
        assert!(matches!(err, WorkGraphError::NodeNotFound(_)));
    }

    #[test]
    fn add_edge_with_missing_to_fails() {
        let mut graph = g();
        let a = graph.add_node(Node::task("a")).unwrap();
        let fake = NodeId::new();
        let err = graph
            .add_edge(Edge::new(a, fake, EdgeKind::DependsOn))
            .unwrap_err();
        assert!(matches!(err, WorkGraphError::NodeNotFound(_)));
    }

    #[test]
    fn add_self_loop_fails() {
        let mut graph = g();
        let a = graph.add_node(Node::task("a")).unwrap();
        let err = graph
            .add_edge(Edge::new(a, a, EdgeKind::DependsOn))
            .unwrap_err();
        assert!(matches!(err, WorkGraphError::InvalidEdge(_)));
    }

    #[test]
    fn add_duplicate_edge_is_idempotent() {
        let mut graph = g();
        let a = graph.add_node(Node::task("a")).unwrap();
        let b = graph.add_node(Node::task("b")).unwrap();
        let e = Edge::new(a, b, EdgeKind::DependsOn);
        graph.add_edge(e.clone()).unwrap();
        graph.add_edge(e).unwrap();
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn add_cycle_fails() {
        let mut graph = g();
        let a = graph.add_node(Node::task("a")).unwrap();
        let b = graph.add_node(Node::task("b")).unwrap();
        let c = graph.add_node(Node::task("c")).unwrap();

        graph
            .add_edge(Edge::new(a, b, EdgeKind::DependsOn))
            .unwrap();
        graph
            .add_edge(Edge::new(b, c, EdgeKind::DependsOn))
            .unwrap();

        let err = graph
            .add_edge(Edge::new(c, a, EdgeKind::DependsOn))
            .unwrap_err();
        assert!(matches!(err, WorkGraphError::CycleDetected { .. }));
    }

    #[test]
    fn non_ordering_edge_can_create_cycle() {
        let mut graph = g();
        let a = graph.add_node(Node::task("a")).unwrap();
        let b = graph.add_node(Node::task("b")).unwrap();
        graph.add_edge(Edge::new(a, b, EdgeKind::Produces)).unwrap();
        graph.add_edge(Edge::new(b, a, EdgeKind::Produces)).unwrap();
        assert_eq!(graph.edge_count(), 2);
    }

    #[test]
    fn remove_edge_works() {
        let mut graph = g();
        let a = graph.add_node(Node::task("a")).unwrap();
        let b = graph.add_node(Node::task("b")).unwrap();
        let e = Edge::new(a, b, EdgeKind::DependsOn);
        graph.add_edge(e.clone()).unwrap();
        assert!(graph.remove_edge(&e));
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn outgoing_and_incoming() {
        let mut graph = g();
        let a = graph.add_node(Node::task("a")).unwrap();
        let b = graph.add_node(Node::task("b")).unwrap();
        let c = graph.add_node(Node::task("c")).unwrap();

        graph.add_edge(Edge::new(a, b, EdgeKind::Produces)).unwrap();
        graph.add_edge(Edge::new(a, c, EdgeKind::Produces)).unwrap();
        graph.add_edge(Edge::new(b, c, EdgeKind::Produces)).unwrap();

        assert_eq!(graph.outgoing(a).len(), 2);
        assert_eq!(graph.incoming(c).len(), 2);
        assert_eq!(graph.outgoing(c).len(), 0);
    }

    #[test]
    fn neighbors_combines_both_directions() {
        let mut graph = g();
        let a = graph.add_node(Node::task("a")).unwrap();
        let b = graph.add_node(Node::task("b")).unwrap();
        let c = graph.add_node(Node::task("c")).unwrap();

        graph.add_edge(Edge::new(a, b, EdgeKind::Produces)).unwrap();
        graph.add_edge(Edge::new(c, a, EdgeKind::Produces)).unwrap();

        let neigh = graph.neighbors(a);
        assert!(neigh.contains(&b));
        assert!(neigh.contains(&c));
        assert_eq!(neigh.len(), 2);
    }

    #[test]
    fn dependencies_and_dependents() {
        let mut graph = g();
        let a = graph.add_node(Node::task("a")).unwrap();
        let b = graph.add_node(Node::task("b")).unwrap();
        let c = graph.add_node(Node::task("c")).unwrap();

        graph
            .add_edge(Edge::new(a, b, EdgeKind::DependsOn))
            .unwrap();
        graph
            .add_edge(Edge::new(a, c, EdgeKind::DependsOn))
            .unwrap();

        let deps = graph.dependencies(a);
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&b));
        assert!(deps.contains(&c));

        let dep_of_b = graph.dependents(b);
        assert_eq!(dep_of_b, vec![a]);
    }

    #[test]
    fn descendants_bfs() {
        let mut graph = g();
        let a = graph.add_node(Node::task("a")).unwrap();
        let b = graph.add_node(Node::task("b")).unwrap();
        let c = graph.add_node(Node::task("c")).unwrap();
        let d = graph.add_node(Node::task("d")).unwrap();

        graph.add_edge(Edge::new(a, b, EdgeKind::Produces)).unwrap();
        graph.add_edge(Edge::new(b, c, EdgeKind::Produces)).unwrap();
        graph.add_edge(Edge::new(b, d, EdgeKind::Produces)).unwrap();

        let desc = graph.descendants(a);
        assert_eq!(desc.len(), 3);
        assert!(desc.contains(&b));
        assert!(desc.contains(&c));
        assert!(desc.contains(&d));
    }

    #[test]
    fn ancestors_bfs() {
        let mut graph = g();
        let a = graph.add_node(Node::task("a")).unwrap();
        let b = graph.add_node(Node::task("b")).unwrap();
        let c = graph.add_node(Node::task("c")).unwrap();

        graph.add_edge(Edge::new(a, c, EdgeKind::Produces)).unwrap();
        graph.add_edge(Edge::new(b, c, EdgeKind::Produces)).unwrap();

        let anc = graph.ancestors(c);
        assert_eq!(anc.len(), 2);
        assert!(anc.contains(&a));
        assert!(anc.contains(&b));
    }

    #[test]
    fn roots_and_leaves() {
        let mut graph = g();
        let a = graph.add_node(Node::task("a")).unwrap();
        let b = graph.add_node(Node::task("b")).unwrap();
        let c = graph.add_node(Node::task("c")).unwrap();

        graph
            .add_edge(Edge::new(a, b, EdgeKind::DependsOn))
            .unwrap();
        graph
            .add_edge(Edge::new(b, c, EdgeKind::DependsOn))
            .unwrap();

        let roots = graph.roots();
        assert_eq!(roots, vec![a]);

        let leaves = graph.leaves();
        assert_eq!(leaves, vec![c]);
    }

    #[test]
    fn topological_order_linear() {
        let mut graph = g();
        let a = graph.add_node(Node::task("a")).unwrap();
        let b = graph.add_node(Node::task("b")).unwrap();
        let c = graph.add_node(Node::task("c")).unwrap();

        graph
            .add_edge(Edge::new(a, b, EdgeKind::DependsOn))
            .unwrap();
        graph
            .add_edge(Edge::new(b, c, EdgeKind::DependsOn))
            .unwrap();

        let order = graph.topological_order().unwrap();
        assert_eq!(order, vec![a, b, c]);
    }

    #[test]
    fn topological_order_branching() {
        let mut graph = g();
        let a = graph.add_node(Node::task("a")).unwrap();
        let b = graph.add_node(Node::task("b")).unwrap();
        let c = graph.add_node(Node::task("c")).unwrap();

        graph
            .add_edge(Edge::new(a, b, EdgeKind::DependsOn))
            .unwrap();
        graph
            .add_edge(Edge::new(a, c, EdgeKind::DependsOn))
            .unwrap();

        let order = graph.topological_order().unwrap();
        assert_eq!(order.len(), 3);
        assert_eq!(order[0], a);
    }

    #[test]
    fn nodes_by_kind_filters() {
        let mut graph = g();
        graph.add_node(Node::goal("g1")).unwrap();
        graph.add_node(Node::goal("g2")).unwrap();
        graph.add_node(Node::task("t1")).unwrap();

        assert_eq!(graph.nodes_by_kind(NodeKind::Goal).len(), 2);
        assert_eq!(graph.nodes_by_kind(NodeKind::Task).len(), 1);
        assert_eq!(graph.nodes_by_kind(NodeKind::Artifact).len(), 0);
    }

    #[test]
    fn stats_counts_correctly() {
        let mut graph = g();
        let a = graph.add_node(Node::goal("g")).unwrap();
        let b = graph.add_node(Node::task("t")).unwrap();
        graph.add_edge(Edge::new(a, b, EdgeKind::PartOf)).unwrap();

        let stats = graph.stats();
        assert_eq!(stats.node_count, 2);
        assert_eq!(stats.edge_count, 1);
        assert_eq!(stats.nodes_by_kind.get(&NodeKind::Goal), Some(&1));
        assert_eq!(stats.nodes_by_kind.get(&NodeKind::Task), Some(&1));
        assert_eq!(stats.edges_by_kind.get(&EdgeKind::PartOf), Some(&1));
    }

    /// Escenario realista con **dirección natural**: el goal es la
    /// raíz, y todo fluye hacia abajo.
    ///
    /// Direcciones de las aristas:
    /// - `goal → req1`, `goal → req2` (Refines).
    /// - `req1 → task1`, `req1 → task2`, `req2 → task3` (Refines).
    /// - `task1 → art1`, `task2 → art2`, `task3 → art3` (Produces).
    /// - `ev → task3` (Verifies).
    /// - `ver → req1`, `ver → req2` (Verifies).
    #[test]
    fn realistic_work_graph() {
        let mut graph = g();

        let goal = graph
            .add_node(Node::goal("Build a calculator CLI"))
            .unwrap();
        let req1 = graph
            .add_node(Node::requirement("support +, -, *, /"))
            .unwrap();
        let req2 = graph
            .add_node(Node::requirement("must have tests"))
            .unwrap();
        let task1 = graph.add_node(Node::task("implement parser")).unwrap();
        let task2 = graph.add_node(Node::task("implement evaluator")).unwrap();
        let task3 = graph.add_node(Node::task("write tests")).unwrap();
        let art1 = graph.add_node(Node::artifact("src/parser.rs")).unwrap();
        let art2 = graph.add_node(Node::artifact("src/eval.rs")).unwrap();
        let art3 = graph
            .add_node(Node::artifact("tests/eval_test.rs"))
            .unwrap();
        let ev = graph.add_node(Node::evidence("cargo test passes")).unwrap();
        let ver = graph
            .add_node(Node::verification("all requirements satisfied"))
            .unwrap();

        // Descomposición (goal → down).
        graph
            .add_edge(Edge::new(goal, req1, EdgeKind::Refines))
            .unwrap();
        graph
            .add_edge(Edge::new(goal, req2, EdgeKind::Refines))
            .unwrap();
        graph
            .add_edge(Edge::new(req1, task1, EdgeKind::Refines))
            .unwrap();
        graph
            .add_edge(Edge::new(req1, task2, EdgeKind::Refines))
            .unwrap();
        graph
            .add_edge(Edge::new(req2, task3, EdgeKind::Refines))
            .unwrap();
        // Producción (task → artifact).
        graph
            .add_edge(Edge::new(task1, art1, EdgeKind::Produces))
            .unwrap();
        graph
            .add_edge(Edge::new(task2, art2, EdgeKind::Produces))
            .unwrap();
        graph
            .add_edge(Edge::new(task3, art3, EdgeKind::Produces))
            .unwrap();
        // Verificación (evidence/verification → hacia el grafo).
        graph
            .add_edge(Edge::new(ev, task3, EdgeKind::Verifies))
            .unwrap();
        graph
            .add_edge(Edge::new(ver, req1, EdgeKind::Verifies))
            .unwrap();
        graph
            .add_edge(Edge::new(ver, req2, EdgeKind::Verifies))
            .unwrap();

        // ─────────────────────────────────────────
        // Stats
        // ─────────────────────────────────────────
        let stats = graph.stats();
        assert_eq!(stats.node_count, 11);
        assert_eq!(stats.nodes_by_kind.get(&NodeKind::Goal), Some(&1));
        assert_eq!(stats.nodes_by_kind.get(&NodeKind::Requirement), Some(&2));
        assert_eq!(stats.nodes_by_kind.get(&NodeKind::Task), Some(&3));
        assert_eq!(stats.nodes_by_kind.get(&NodeKind::Artifact), Some(&3));
        assert_eq!(stats.nodes_by_kind.get(&NodeKind::Evidence), Some(&1));
        assert_eq!(stats.nodes_by_kind.get(&NodeKind::Verification), Some(&1));

        // ─────────────────────────────────────────
        // Descendants del goal
        // ─────────────────────────────────────────
        // BFS desde goal: req1, req2, task1, task2, task3, art1,
        // art2, art3 → 8 nodos.
        // NO alcanzables: ev, ver (sus aristas van hacia el grafo,
        // no desde él).
        let desc = graph.descendants(goal);
        assert_eq!(desc.len(), 8);
        assert!(desc.contains(&req1));
        assert!(desc.contains(&req2));
        assert!(desc.contains(&task1));
        assert!(desc.contains(&task3));
        assert!(desc.contains(&art1));
        assert!(desc.contains(&art3));
        assert!(!desc.contains(&ev));
        assert!(!desc.contains(&ver));

        // ─────────────────────────────────────────
        // Ancestros de art1: task1, req1, goal
        // ─────────────────────────────────────────
        let anc_art1 = graph.ancestors(art1);
        assert!(anc_art1.contains(&task1));
        assert!(anc_art1.contains(&req1));
        assert!(anc_art1.contains(&goal));
        assert!(!anc_art1.contains(&ev));

        // ─────────────────────────────────────────
        // Descendants de ev: {task3, art3}
        // ─────────────────────────────────────────
        // BFS desde ev sigue TODAS las aristas:
        // - ev → task3 (Verifies)
        // - task3 → art3 (Produces)
        // Resultado: 2 nodos.
        let desc_ev = graph.descendants(ev);
        assert_eq!(desc_ev.len(), 2);
        assert!(desc_ev.contains(&task3));
        assert!(desc_ev.contains(&art3));

        // ─────────────────────────────────────────
        // Ancestros de task3: req2, goal, ev
        // ─────────────────────────────────────────
        let anc_task3 = graph.ancestors(task3);
        assert!(anc_task3.contains(&req2));
        assert!(anc_task3.contains(&goal));
        assert!(anc_task3.contains(&ev));
    }
}
