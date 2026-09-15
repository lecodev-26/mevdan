# MEVDAN — Checkpoint and Recovery Specification

Checkpoints let you snapshot the entire work state and roll back to
it later.

## Work State

A checkpoint captures a `WorkState`:

```rust
pub struct WorkState {
    pub schema_version: String,
    pub workgraph: WorkGraph,
    pub tasks: Vec<Task>,
    pub artifacts: Vec<Artifact>,
    pub claims: Vec<Claim>,
    pub metadata: serde_json::Value,
}
```

metadata is a free-form JSON field for context (git commit, branch
name, notes).

Checkpoint

A checkpoint wraps a WorkState with metadata:

```rust
pub struct Checkpoint {
    pub id: CheckpointId,
    pub label: Option<String>,
    pub state: WorkState,
    pub created_at: DateTime<Utc>,
}
```

Checkpoint engine

```rust
let mut engine = CheckpointEngine::new();
let id = engine.create(state, Some("before-refactor".into()));

// Later:
let restored = engine.restore(id)?;
```

Operations:

· create(state, label) -> CheckpointId
· list() — all checkpoints (ordered by id → time)
· list_recent(n) — last N, most recent first
· get(id) / require(id)
· restore(id) -> WorkState (cloned)
· delete(id)
· find_by_label(label)
· clear()

Persistence

Checkpoints are stored in the checkpoints table (V003):

```sql
CREATE TABLE checkpoints (
    id              TEXT PRIMARY KEY,
    project_id      TEXT NOT NULL,
    label           TEXT,
    schema_version  TEXT NOT NULL,
    json            TEXT NOT NULL,      -- serialized Checkpoint
    created_at      TEXT NOT NULL
);
```

The whole Checkpoint is JSON-encoded. The json column carries
everything.

Recovery

RecoveryEngine uses checkpoints to recover from failures.

Actions

Action Purpose
Resume Continue from the last checkpoint.
Retry Retry a failed task from the last checkpoint.
Rollback Restore a specific checkpoint.
Continue Advance from the current in-memory state.

Resume

```rust
let recovery = RecoveryEngine::new(&checkpoint_engine);
let result = recovery.resume()?;
// result.state = last checkpoint's WorkState
```

Fails if no checkpoints exist.

Retry

```rust
let result = recovery.retry(task_id)?;
// Task is reset from Failed → Ready
```

Fails if:

· No checkpoints.
· Task not found in the checkpoint's state.
· Task is not in Failed status.

Rollback

```rust
let result = recovery.rollback(checkpoint_id)?;
// result.state = that checkpoint's WorkState
```

Continue

```rust
let result = recovery.continue_from(current_state);
// result.tasks_to_retry = tasks in Ready status
```

Useful when the runtime already has state in memory.

Recovery result

```rust
pub struct RecoveryResult {
    pub action: RecoveryAction,
    pub checkpoint_id: Option<CheckpointId>,
    pub state: WorkState,
    pub description: String,
    pub tasks_to_retry: Vec<TaskId>,
}
```

Typical workflow

```
1. Before risky operation:
   mevdan checkpoint create "before-refactor"

2. Agent works, changes state.

3. Something fails.

4. User decides:
   - Retry the failed task:
     mevdan continue --retry <task-id>
   - Or rollback to before:
     mevdan checkpoint restore <id>
```

Limitations

· Checkpoints are whole-state. There is no incremental diff.
· No automatic checkpointing yet. The runtime (V0.4+) will add
  policy-based auto-checkpoints.
· WorkState grows with the project. For large projects, storing
  many checkpoints could be heavy. Mitigation planned for V0.4+:
  compressed snapshots or differential storage.
