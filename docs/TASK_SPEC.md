# MEVDAN — Task Engine Specification

The Task Engine manages units of work with well-defined states and
dependencies.

## Task

```rust
pub struct Task {
    pub id: TaskId,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub depends_on: Vec<TaskId>,
    pub result: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

States

```rust
pub enum TaskStatus {
    Pending,          // Created, waiting
    Ready,            // Dependencies satisfied
    Running,          // In progress
    Blocked,          // Cannot progress
    WaitingApproval,  // Waiting for user
    Completed,        // Done
    Failed,           // Failed
    Cancelled,        // Cancelled
}
```

Valid transitions

```
Pending      → Ready, Blocked, Cancelled
Ready        → Running, Blocked, Cancelled
Running      → Completed, Failed, WaitingApproval, Blocked, Cancelled
Blocked      → Ready, Pending, Cancelled
WaitingApproval → Running, Failed, Cancelled
Completed    → (terminal)
Failed       → (terminal)
Cancelled    → (terminal)
```

Invalid transitions are rejected with TaskError::InvalidTransition.

Priority

```rust
pub enum TaskPriority {
    Low,
    Normal,     // default
    High,
    Critical,
}
```

Higher priority tasks are returned first by ready_tasks().

Task Engine

```rust
let mut engine = TaskEngine::new();

let setup = engine.add(Task::new("setup").with_priority(TaskPriority::High))?;
let build = engine.add(Task::new("build"))?;
engine.add_dependency(build, setup)?;

engine.refresh_ready_tasks();   // setup → Ready
engine.transition(setup, TaskStatus::Running)?;
engine.transition(setup, TaskStatus::Completed)?;

engine.refresh_ready_tasks();   // build → Ready
```

Dependencies

add_dependency(task, depends_on):

· task depends on depends_on.
· Both must exist.
· No self-dependencies.
· No cycles (detected via BFS).

Dependencies must be Completed before a task can transition to
Running. The engine enforces this in transition().

Queries

· dependencies_of(id) — direct dependencies.
· dependents_of(id) — direct dependents.
· dependencies_resolved(id) — all deps completed?
· ready_tasks() — tasks in Ready, sorted by priority.
· all_terminal() — all tasks in terminal state?
· has_failures() — any task Failed?
· topological_order() — DAG order (Kahn's algorithm).

Refreshing

refresh_ready_tasks() walks all Pending/Blocked tasks and
promotes to Ready any whose dependencies are now satisfied.
Returns the list of IDs that changed.

The runtime calls this after each task completes.

Lifecycle example

```
1. Create tasks: setup, build, test, deploy
2. add_dependency(build, setup)
3. add_dependency(test, build)
4. add_dependency(deploy, test)

5. refresh_ready_tasks() → [setup]
6. transition(setup, Running)
7. transition(setup, Completed)

8. refresh_ready_tasks() → [build]
9. ... and so on ...
```

Relation to Work Graph

TaskEngine operates in memory. Tasks map to NodeKind::Task nodes
in the Work Graph:

· Task.id ↔ Node.id
· Task.title ↔ Node.label
· Task.status ↔ Node.data["status"]
· Dependencies ↔ EdgeKind::DependsOn

Persistence of task state happens via checkpoints (WorkState.tasks)
and future Work Graph nodes. mevdan-task itself is I/O-free.
