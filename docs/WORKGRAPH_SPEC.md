# MEVDAN — Work Graph Specification

The Work Graph is the **central data structure** of MEVDAN. It models
work as a directed graph of typed nodes and edges.

## Why a graph?

Chat-based AI tools treat work as a linear conversation. MEVDAN treats
it as a **structure** that survives across agents, models, and
sessions.

When an agent says "I'm done", the Work Graph records:
- What goal was set.
- What requirements were derived.
- What tasks were executed.
- What artifacts were produced.
- What evidence supports the result.
- What verifications passed.

Any other agent can pick up where the previous one left off by
reading the graph — not by replaying chat history.

## Nodes

A node represents an element of work.

### Node types (`NodeKind`)

| Kind | Purpose |
|------|---------|
| `Goal` | High-level objective. |
| `Requirement` | What must be satisfied. |
| `Constraint` | What must NOT be violated. |
| `Task` | Executable unit of work. |
| `Action` | Concrete action (tool call, model invocation). |
| `Artifact` | Produced result (file, document, JSON...). |
| `Evidence` | Proof that something was done. |
| `Verification` | Result of checking a claim. |
| `Decision` | Architectural or design decision. |
| `Checkpoint` | Snapshot of the work state. |

### Node structure

```rust
pub struct Node {
    pub id: NodeId,          // UUID v7 (time-ordered)
    pub kind: NodeKind,
    pub label: String,       // Human-readable
    pub data: serde_json::Value,  // Kind-specific payload
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Design choice: one node type with a kind field, not subclasses.
This makes the graph uniform, serializable, and extensible.

Edges

An edge connects two nodes with a typed relation.

Edge types (EdgeKind)

Kind Meaning
DependsOn A depends on B.
Produces A produces B.
Verifies A verifies B.
PartOf A is part of B.
Satisfies A satisfies B.
Precedes A precedes B (temporal order).
Refines A refines B (more specific).

Ordering edges

Some edge kinds are ordering: DependsOn, Precedes, PartOf,
Refines. These must form a DAG (no cycles). Others
(Produces, Verifies, Satisfies) are free-form.

Edge structure

```rust
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub kind: EdgeKind,
}
```

Edges are stored in a BTreeSet for deduplication and stable order.

Invariants

1. IDs are unique. No two nodes share a NodeId.
2. Edges reference existing nodes. from and to must exist.
3. No self-loops. from != to.
4. No cycles in ordering edges.
5. Append-only semantics. Nodes and edges are added, not
   rewritten. Metadata like updated_at may change.

Operations

Adding nodes

```rust
let mut graph = WorkGraph::new();
let goal = graph.add_node(Node::goal("Build a CLI"))?;
let task = graph.add_node(Node::task("Implement parser"))?;
```

Empty labels are rejected.

Adding edges

```rust
graph.add_edge(Edge::new(goal, task, EdgeKind::Refines))?;
```

Validation:

· Both nodes must exist.
· No self-loops.
· No cycles in ordering edges.
· Duplicates are ignored (idempotent).

Traversal

· outgoing(id) / incoming(id) — edges from/to a node.
· neighbors(id) — all connected nodes.
· dependencies(id) — nodes this depends on (via DependsOn).
· dependents(id) — nodes depending on this.
· descendants(id) — all reachable nodes (BFS).
· ancestors(id) — all nodes that reach this (BFS).

Queries

· roots() — nodes with no incoming ordering edges.
· leaves() — nodes with no outgoing ordering edges.
· topological_order() — Kahn's algorithm. Returns None if a
  cycle exists (should not happen after validation).
· nodes_by_kind(kind) — filter by node type.
· stats() — counts by kind.

Typical shapes

Simple task

```
Goal ──Refines──► Requirement
                    ▲
                    │
                    │ Satisfies
                    │
                  Task ──Produces──► Artifact
                    ▲
                    │
                    │ Verifies
                    │
                Evidence
```

Multi-agent handoff

```
Agent A ──► [Tasks] ──► [Artifacts] ──► Checkpoint
                                            │
                                            ▼
Agent B ◄──[reads Work Graph from checkpoint]
   │
   └──► continues with new Tasks
```

Persistence

Work Graphs are stored in the workgraphs table (schema V002+):

```sql
CREATE TABLE workgraphs (
    id              TEXT PRIMARY KEY,
    project_id      TEXT NOT NULL,
    label           TEXT,
    schema_version  TEXT NOT NULL,
    json            TEXT NOT NULL,      -- serialized WorkGraph
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);
```

The graph is serialized as JSON. This trades off query flexibility
for simplicity. If performance requires, a future migration can
normalize nodes and edges into separate tables.

What the Work Graph is NOT

· It's not a chat log. Chat history is just one input.
· It's not tied to a specific model or agent.
· It's not an execution engine. It records state; the runtime
  executes.
  EOF
