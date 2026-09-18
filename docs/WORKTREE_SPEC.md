# MEVDAN — Worktree Specification

A **worktree** is an isolated workspace inside a project. Multiple
worktrees can coexist, each with its own state, without colliding.

## Why

Sometimes you want to try multiple approaches in parallel. Instead
of picking one and losing the other, create two worktrees:

```text
mi-proyecto/                    ← main worktree
├── .mevdan/
│   ├── worktrees/
│   │   ├── approach-a/        ← worktree A
│   │   └── approach-b/        ← worktree B
│   └── mevdan.db
Each worktree has its own Work Graph, tasks and checkpoints. The
Project is shared.

Concepts
Worktree
rust
pub struct Worktree {
    pub id: WorktreeId,
    pub name: String,
    pub path: PathBuf,
    pub is_main: bool,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}
is_main = true for the main worktree. Only one per project.

name must match: 1-64 chars, a-z, 0-9, -, _.

The path for non-main worktrees lives under
<project_root>/.mevdan/worktrees/<name>/.

WorktreeId
WorktreeId is a UUID v7 (time-ordered). Serialized transparently
as a string.

WorktreeManager
Manages the set of worktrees for a project.

rust
let mut manager = WorktreeManager::new("/path/to/project")?;

// The main worktree is auto-registered.
assert_eq!(manager.main().name, "main");

// Create a parallel worktree.
let wt = manager.create("experiment", Some("try a new approach".into()))?;

// List, get, require.
let all = manager.list();
let one = manager.get("experiment");
let mandatory = manager.require("experiment")?;

// Delete (removes the directory too).
manager.delete("experiment")?;
Operations:

new(project_root) — create manager, auto-register main.

create(name, description) — create a new worktree on disk.

register(worktree) — register an existing worktree (no disk).

get(name) / require(name) — lookup.

list() — all worktrees.

delete(name) — remove (fails on main).

main() — the main worktree.

to_json() / from_json(s) — persistence.

Rules
There is always a main worktree. It points to the project
root.

main cannot be deleted.

Worktree names are unique per project.

Worktree names are validated (lowercase, digits, -, _).

Creating a worktree creates its directory.

What this is NOT
Not a git worktree. MEVDAN worktrees do not share .git. They
are MEVDAN-level concepts.

Not parallel execution. MEVDAN worktrees are containers;
parallel execution arrives later.

Not automatic isolation. Agents must be explicitly assigned
to a worktree.

Not persisted by default. Persistence is the caller's job
(via to_json).

Design notes
One manager per project. No global state.

Disk-backed. Creating a worktree creates its directory.

Deletion is careful. Only deletes if the path is under the
project's worktree directory.

Serializable. The whole manager can be JSON-encoded.
