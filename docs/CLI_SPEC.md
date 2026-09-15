# MEVDAN — CLI Specification

The command-line interface. Everything MEVDAN can do is reachable
from the CLI.

## Binary

```

mevdan 0.3.0

```

## Global

```

mevdan --version
mevdan --help

```

## Commands

### `mevdan init <name>`

Create a new project.

```bash
mevdan init demo
mevdan init demo --path /tmp
```

Creates:

· <path>/<name>/.mevdan/project.toml
· <path>/<name>/.mevdan/mevdan.db
· <path>/<name>/.mevdan/artifacts/
· <path>/<name>/.mevdan/checkpoints/
· <path>/<name>/.mevdan/events/

Also inserts a Project, an initial Session, and a
ProjectCreated event.

mevdan status

Show the current project's status.

```bash
cd demo
mevdan status
```

Output:

```
MEVDAN — Project Status
───────────────────────────────────────
Name:           demo
ID:             01a0...
Schema:         0.3.0
MEVDAN version: 0.3.0
Created:        2026-09-15T10:00:00Z
Updated:        2026-09-15T10:00:00Z

Sessions:       1
Events:         1
Work Graphs:    0
Checkpoints:    0

Directory:      /home/user/demo
Database:       /home/user/demo/.mevdan/mevdan.db
```

mevdan chat <message>

Send a message to a provider.

```bash
mevdan chat "Hello" -m llama3.2 -p ollama

mevdan chat "Hello" -m gpt-4o-mini \
    -p openai-compatible \
    --base-url https://api.openai.com/v1 \
    --api-key-secret openai_api_key
```

Flags:

· -p, --provider — ollama (default) or openai-compatible.
· -m, --model — model name.
· --base-url — provider base URL.
· --api-key-secret — secret name (default openai_api_key).

mevdan tasks

List tasks from the project's Work Graph.

```bash
mevdan tasks
mevdan tasks --status ready
mevdan tasks --verbose
```

Flags:

· -s, --status — filter by status (pending, ready, running,
  blocked, waiting_approval, completed, failed, cancelled).
· -v, --verbose — show IDs, descriptions, priorities.

If no Work Graph exists yet, prints a note that tasks are created
when the agent runs (V0.4.0+).

mevdan audit

Show the audit trail of the project.

```bash
mevdan audit
mevdan audit --limit 20
mevdan audit --category lifecycle
mevdan audit --json
```

Flags:

· -l, --limit — show only last N entries.
· -c, --category — filter by category (lifecycle, session,
  agent, tool, task, permission, checkpoint,
  verification, unknown).
· --json — output as JSON.

Output example:

```
MEVDAN — Audit Trail
───────────────────────────────────────
Project: demo
Total events: 3

[2026-09-15 10:00:00] [lifecycle] project created: demo (MEVDAN 0.3.0)
[2026-09-15 10:00:01] [session] session started: initial
[2026-09-15 10:05:00] [session] session ended: completed

3 event(s) shown.
```

Working directory

All commands except init look for .mevdan/project.toml in the
current directory or any parent. If not found, the command fails
with a clear error.

Exit codes

· 0 — success.
· 1 — error (invalid args, missing project, provider error).

Planned commands (V0.4+)

· mevdan run <task> — execute a task with the agent.
· mevdan checkpoint — manage checkpoints.
· mevdan verify — verify claims.
· mevdan skills — list/install skills.
· mevdan doctor — diagnose environment.
· mevdan export / mevdan import — portable work package.

Environment

· HOME — used to find ~/.config/mevdan/.
· No other environment variables are required.
