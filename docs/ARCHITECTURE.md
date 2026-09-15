# MEVDAN — Architecture

This document describes the overall architecture of MEVDAN: the
crates, their responsibilities, their dependencies, and how data
flows through the system.

## Core idea

MEVDAN is a **local-first AI work runtime**. It's not a chatbot. It's
not tied to any provider. The work belongs to the user, not to the
model or the agent.

> The model is replaceable. The agent is replaceable. The work is not.

## Design principles

1. **Local-first.** No server required. No account. No telemetry.
2. **Provider-agnostic.** Any model provider can be plugged in.
3. **Agent-agnostic.** Agents are interchangeable.
4. **Verification over claims.** Agents propose; MEVDAN verifies.
5. **Security by default.** Dangerous operations require permissions.
6. **Everything observable.** Every action produces events.
7. **Portable work.** Work can be exported and imported.

## Layered view

```

┌──────────────────────────────────────────────┐
│  INTERFACES                                  │
│  CLI  (crates/mevdan-cli)                    │
│  [future] Desktop, Android                   │
└────────────────┬─────────────────────────────┘
│
┌────────────────▼─────────────────────────────┐
│  RUNTIME LAYER                               │
│  Agent  Context  Task  WorkGraph  Tools      │
│  Permissions  Verification  Checkpoint       │
│  Skills  Models                              │
└────────────────┬─────────────────────────────┘
│
┌────────────────▼─────────────────────────────┐
│  FOUNDATION                                  │
│  Core (domain)  Storage (SQLite)             │
│  Config  Secrets                             │
└────────────────┬─────────────────────────────┘
│
┌────────────────▼─────────────────────────────┐
│  EXTERNAL                                    │
│  Providers (OpenAI-compat, Ollama, ...)      │
│  Filesystem  Shell  Git                      │
└──────────────────────────────────────────────┘

```

## Crates

### Foundation (no I/O in core)

#### `mevdan-core`
Pure domain: typed IDs, errors, `Project`, `Session`, `Event`,
schema versioning. **No I/O. No dependencies on other MEVDAN crates.**

#### `mevdan-storage`
SQLite persistence. Migrations (V001-V003), repositories,
append-only event log. Depends on `mevdan-core`,
`mevdan-workgraph`, `mevdan-checkpoint`.

#### `mevdan-config`
Configuration loading (global `~/.config/mevdan/` + per-project
`.mevdan/`).

#### `mevdan-secrets`
Secret storage (API keys). File-based with 0600 permissions.
Future: keyring integration.

### Runtime

#### `mevdan-provider`
Provider abstraction + adapters (OpenAI-compatible, Ollama).
Synchronous HTTP.

#### `mevdan-models`
Model registry with capabilities (context window, tool calling,
vision, etc.).

#### `mevdan-agent`
Agent engine: identity, roles, execution policy, reasoning loop
with auto-review and replanning, multi-agent orchestration.

#### `mevdan-context`
Context construction and compaction. Preserves decisions,
constraints, errors when compacting.

#### `mevdan-tools`
Tools the agent can invoke:
- `FilesystemTool` — sandboxed read/write/list/delete.
- `ShellTool` — allowlisted command execution.
- `GitTool` — validated git operations.

#### `mevdan-permissions`
Permission engine (ALLOW/ASK/DENY) + risk engine with
auto-escalation.

#### `mevdan-workgraph`
The Work Graph: typed nodes and edges, cycle detection,
topological sort, traversal.

#### `mevdan-task`
Task Engine: states, dependencies, transitions, topological order.

#### `mevdan-verification`
Claims, evidence, artifacts (SHA-256 hashed). Verifiers
(`FileExists`, `Hash`, `CommandExit`) + `VerificationEngine`.

#### `mevdan-checkpoint`
Checkpoints (WorkState snapshots), `CheckpointEngine`, and
`RecoveryEngine` (resume/retry/rollback/continue).

#### `mevdan-skills`
Skill system: `SkillManifest` (TOML), loader, registry, and
isolation (permission validation).

### Interface

#### `mevdan-cli`
Command-line interface. Commands: `init`, `status`, `chat`,
`tasks`, `audit`.

## Dependency graph

```

mevdan-cli
├── mevdan-core
├── mevdan-storage
│     ├── mevdan-core
│     ├── mevdan-workgraph
│     └── mevdan-checkpoint
│           ├── mevdan-core
│           ├── mevdan-workgraph
│           ├── mevdan-task
│           └── mevdan-verification
├── mevdan-config
├── mevdan-secrets
├── mevdan-provider
├── mevdan-workgraph
└── mevdan-task

mevdan-verification
├── mevdan-core
└── mevdan-tools

mevdan-skills
└── mevdan-core

```

Rule: **no cycles.** `mevdan-core` is the root of the dependency
tree.

## Data flow

### Creating a project

```

mevdan init demo
│
├── mevdan-cli parses args
├── mevdan-core::Project::new()
├── Creates .mevdan/ directory structure
├── mevdan-storage::Database::open()
│     └── applies migrations V001-V003
├── Persists Project, Session, ProjectCreated event
└── Prints summary

```

### Running an agent (V0.4+)

```

Agent task
│
├── Context construction (mevdan-context)
├── Reasoning loop (mevdan-agent)
│     ├── Build request → provider.chat()
│     ├── Observe response
│     ├── Review (auto-critique)
│     └── Replan if needed
├── Tool invocations
│     ├── Permission check (mevdan-permissions)
│     ├── Risk assessment (mevdan-permissions)
│     └── Execute tool (mevdan-tools)
├── Record actions as Work Graph nodes
├── Produce artifacts and evidence
├── Verify claims
└── Checkpoint state

```

## Storage model

Project directory structure:

```

demo/
└── .mevdan/
├── project.toml          # Config, schema version
├── mevdan.db             # SQLite: state + event log
├── artifacts/            # Large files (future)
├── checkpoints/          # Snapshots (future)
├── events/               # Export of event log (future)
└── skills/               # Installed skills

```

SQLite schema (version 0.3.0):
- `meta` — schema version, creation time.
- `projects` — one row per project.
- `sessions` — one row per session.
- `events` — append-only event log.
- `workgraphs` — Work Graph snapshots (JSON blobs).
- `checkpoints` — Work State snapshots (JSON blobs).

## Extensibility

- **New providers:** implement `mevdan_provider::Provider`.
- **New tools:** implement `mevdan_tools::Tool`.
- **New verifiers:** implement `mevdan_verification::Verifier`.
- **New skills:** add a `skill.toml` under `.mevdan/skills/`.

## Non-goals (for now)

- GUI (planned for V0.6).
- Cloud sync (user can use git or any sync tool).
- Multi-user accounts (local-first).
- Managed backend (there is no MEVDAN server).

## Further reading

- `docs/WORKGRAPH_SPEC.md` — Work Graph details.
- `docs/VERIFICATION_SPEC.md` — verification model.
- `docs/CHECKPOINT_SPEC.md` — checkpoints and recovery.
- `docs/TASK_SPEC.md` — Task Engine.
- `docs/SKILL_SPEC.md` — skill system.
- `docs/CLI_SPEC.md` — CLI commands.
