# MEVDAN

**Any model. Any agent. Your work.**

MEVDAN is an open-source, local-first AI work runtime.

> The model proposes; MEVDAN decides and executes through audited,
> permission-gated tools.

## What this is

MEVDAN is not another chatbot. It's a runtime where:

- **The model is replaceable.**
- **The agent is replaceable.**
- **The work is not.**

A project's state, requirements, tasks, decisions, artifacts, evidence,
verification results and checkpoints belong to the user — not to any
provider, model, or agent.

## Current version

**V0.4.0** — Multi-Agent, MCP, LSP, Code Intelligence.

**Status:** 40 of 100 roadmap phases complete. ~1500 tests passing on
Linux, macOS and Windows. CI green.

## What works

### CLI

```bash
mevdan init <name>       # Create a new project
mevdan status            # Show project state
mevdan chat <message>    # Talk to a provider (Ollama, OpenAI-compatible)
mevdan tasks             # List tasks from the Work Graph
mevdan audit             # Show the project's event timeline
mevdan skills list       # List installed skills
mevdan mcp list          # List configured MCP servers
mevdan codeintel map     # Show the repository map
mevdan route <text>      # Show agent + model routing for a piece of text
Crates (19)
Foundation:

mevdan-core — Pure domain: typed IDs, errors, Project, Session, Event.

mevdan-storage — SQLite persistence: migrations, repos, event log.

mevdan-config — Global and per-project configuration.

mevdan-secrets — Secret storage with 0600 permissions.

Runtime:

mevdan-provider — Provider abstraction (OpenAI-compatible, Ollama).

mevdan-models — Model registry with capabilities.

mevdan-agent — Agent engine, multi-agent, workflows, handoff, parallel.

mevdan-context — Context engine with structured compaction.

mevdan-tools — Sandboxed filesystem, shell, git.

mevdan-permissions — Permission engine + risk engine.

mevdan-workgraph — Typed Work Graph with cycle detection.

mevdan-task — Task engine with states and dependencies.

mevdan-verification — Claims, evidence, artifacts, verifiers.

mevdan-checkpoint — Checkpoints and recovery.

mevdan-skills — Skills with TOML manifests and isolation.

mevdan-mcp — MCP client with trust levels and security.

mevdan-lsp — LSP client with proper framing.

mevdan-codeintel — Repo map, symbols, dependencies.

mevdan-router — Task classification, agent + model routing (no ML).

Interface:

mevdan-cli — Command-line interface.

Install (from source)
Requires Rust 1.75+.

bash
git clone git@github.com:lecodev-26/mevdan.git
cd mevdan
cargo build --release
The binary will be at target/release/mevdan.

Quick start
bash
# Create a project.
mevdan init demo
cd demo

# Check status.
mevdan status

# Talk to a local model (requires Ollama running).
mevdan chat "Hello" -m llama3.2 -p ollama

# See the event timeline.
mevdan audit

# Check the repository map.
mevdan codeintel map

# See how a task would be routed.
mevdan route "write tests for the parser"
Principles
Local-first. No server required. No account. No telemetry.

Provider-agnostic. Any model provider can be plugged in.

Agent-agnostic. Agents are interchangeable.

Verification over claims. Agents propose; MEVDAN verifies.

Security by default. Risky operations require permissions.

Everything observable. Every action produces events.

Portable work. Work can be exported and imported.

Documentation
docs/ARCHITECTURE.md — overall architecture

docs/WORKGRAPH_SPEC.md — Work Graph

docs/VERIFICATION_SPEC.md — verification

docs/CHECKPOINT_SPEC.md — checkpoints and recovery

docs/TASK_SPEC.md — Task Engine

docs/SKILL_SPEC.md — skills

docs/CLI_SPEC.md — CLI reference

VERSIONS.md — version history and roadmap

Roadmap
MEVDAN is developed in public, one phase at a time.

Version	Phases	Content	Status
0.1.0	01–10	Foundation	✅
0.2.0	11–20	Agent, Tools, Permissions, Work Graph	✅
0.3.0	21–30	Tasks, Verification, Checkpoints, Skills	✅
0.4.0	31–40	MCP, LSP, Code Intel, Multi-Agent, Router	✅
0.5.0	41–50	Worktree, Automation, Documents, Data, Media, Browser, Web, Editor	⏳
0.6.0	51–60	Memory, Local Models, Offline, Desktop, Approval, Android	⏳
0.7.0	61–70	Import, Compatibility, Plugins, Extensions, Registry, Trust	⏳
0.8.0	71–80	Limits, Cost, Fallback, Observability, Benchmarks, Release	⏳
0.9.0	81–90	Crash, Telemetry, Privacy, Team, Remote, Containers, Autonomy	⏳
1.0.0	91–100	Policy, Templates, One-Click, Backup, Recovery, i18n	⏳
License
MIT. See LICENSE.

Contributing
See CONTRIBUTING.md.

Security
See SECURITY.md.
