<div align="center">

# MEVDAN

### Open-source, local-first AI Work Runtime

**Any model. Any agent. Your work.**

<p>
  <img src="https://img.shields.io/badge/status-v0.5.0-7c3aed?style=for-the-badge" alt="Status">
  <img src="https://img.shields.io/badge/roadmap-50%25-2563eb?style=for-the-badge" alt="Roadmap">
  <img src="https://img.shields.io/badge/license-MIT-16a34a?style=for-the-badge" alt="License">
  <img src="https://img.shields.io/badge/rust-1.85%2B-orange?style=for-the-badge&logo=rust" alt="Rust">
</p>

<p>
  <a href="#-why-mevdan">Why MEVDAN</a> •
  <a href="#-features">Features</a> •
  <a href="#-architecture">Architecture</a> •
  <a href="#-roadmap">Roadmap</a> •
  <a href="#-development">Development</a> •
  <a href="#-contributing">Contributing</a>
</p>

</div>

---

> **Models are replaceable. Work is not.**

MEVDAN is an **open-source, local-first AI Work Runtime** for building, executing, verifying, and preserving real work with AI.

Instead of making your project depend on one chatbot, model, provider, or agent, MEVDAN puts the **work itself** at the center: goals, requirements, tasks, artifacts, evidence, decisions, checkpoints, permissions, and verification live in the runtime.

```text
                           ┌─────────────────┐
                           │      USER       │
                           └────────┬────────┘
                                    │
                                    ▼
                       ┌────────────────────────┐
                       │      MEVDAN CLIENT     │
                       │   CLI • Desktop •      │
                       │        Android        │
                       └───────────┬────────────┘
                                   │
                                   ▼
                       ┌────────────────────────┐
                       │     MEVDAN RUNTIME     │
                       └───────────┬────────────┘
                                   │
                                   ▼
                       ┌────────────────────────┐
                       │       WORK GRAPH       │
                       │ Goals • Tasks • State  │
                       └───────────┬────────────┘
                                   │
                ┌──────────────────┼──────────────────┐
                ▼                  ▼                  ▼
           ┌─────────┐       ┌──────────┐       ┌─────────┐
           │ Agents  │       │  Models  │       │  Tools  │
           └────┬────┘       └────┬─────┘       └────┬────┘
                └──────────────────┼──────────────────┘
                                   ▼
                     ┌──────────────────────────┐
                     │ Evidence • Verification  │
                     │ Artifacts • Checkpoints  │
                     └──────────────────────────┘
```

---

## ✨ Why MEVDAN?

AI tools are getting better at **doing things**, but the work they do is often trapped inside a session, a provider, or a particular agent.

MEVDAN takes a different approach:

| Traditional AI workflow | MEVDAN |
|---|---|
| Conversation is the project | **Work state is the project** |
| Agent owns the context | **Runtime owns durable state** |
| Model/provider lock-in | **Provider & model agnostic** |
| "The agent says it worked" | **Evidence + verification** |
| One agent per workflow | **Agents can hand off work** |
| Cloud-first by default | **Local-first architecture** |
| Hidden autonomy | **Explicit permissions & policies** |
| Hard to recover | **Checkpoints & recovery** |

### The core idea

```text
       MODEL
         │
       AGENT
         │
         ▼
   ┌───────────────┐
   │    MEVDAN     │
   │ WORK RUNTIME  │
   └───────┬───────┘
           │
           ▼
      YOUR WORK
```

The intelligence can change.

**The work does not have to.**

---

# 🚀 Features

## 🕸️ Agent-Agnostic Work Graph

MEVDAN models work as structured state rather than treating a chat transcript as the source of truth.

Core entities include:

- 🎯 Goals
- 📋 Requirements
- 🚧 Constraints
- ✅ Tasks
- ⚙️ Actions
- 📦 Artifacts
- 🔎 Evidence
- 🧪 Verification
- 🧠 Decisions
- 💾 Checkpoints

Relationships include:

```text
DependsOn
Produces
Verifies
PartOf
Satisfies
Precedes
Refines
```

This gives the runtime a durable representation of **what is being done, why it is being done, and what proves it is done**.

---

## 🤖 Provider & Model Agnostic

Bring the model you already use.

MEVDAN is built around provider abstraction rather than binding project state to one vendor.

Designed to work with:

- OpenAI-compatible APIs
- Ollama
- LM Studio
- llama.cpp
- vLLM
- Anthropic
- Google
- DeepSeek
- OpenRouter
- Other compatible providers

### BYOK — Bring Your Own Key

You control your credentials and provider choices.

Local models can also be used without cloud API keys.

---

## 🏠 Local-First

Your project belongs on your machine.

MEVDAN is designed around:

- Local project state
- SQLite + filesystem storage
- Local execution
- Local Git repositories
- Local checkpoints
- Optional local models
- No mandatory MEVDAN account
- No mandatory MEVDAN cloud
- No mandatory telemetry

The runtime should remain useful even when the internet is unavailable.

---

## 🔐 Permissions & Controlled Autonomy

AI agents can access powerful tools. MEVDAN therefore treats permissions as a runtime primitive.

Actions can be:

| Policy | Meaning |
|---|---|
| `ALLOW` | Execute automatically |
| `ASK` | Request user approval |
| `DENY` | Block the operation |

Risk levels:

```text
LOW
MEDIUM
HIGH
CRITICAL
```

Sensitive operations can include:

- Filesystem writes
- Shell commands
- Git operations
- Network access
- External tools
- MCP servers
- Skills
- Agent delegation

**Autonomy should be configurable, not assumed.**

---

## 🔎 Verification Over Claims

One of the fundamental MEVDAN rules:

> **An agent claim is not evidence.**

The runtime can turn execution into a verifiable chain:

```text
┌──────────────┐
│ Agent Claim  │
└──────┬───────┘
       ▼
┌──────────────┐
│   Evidence   │
└──────┬───────┘
       ▼
┌──────────────┐
│ Verification │
└──────┬───────┘
       ▼
┌──────────────┐
│   VERIFIED   │
└──────────────┘
```

Verification primitives include things such as:

- File existence
- Artifact hashes
- Command exit codes
- Test execution
- Structured verification results

---

## 📦 Artifacts & Evidence

A completed task should leave behind useful, inspectable output.

```text
Task
 │
 ├── Action
 │    └── Execution
 │
 ├── Artifact
 │
 ├── Evidence
 │
 └── Verification
```

This makes it possible to understand not only **what the agent said**, but what the runtime actually observed.

---

## 💾 Checkpoints & Recovery

Long-running AI work should not disappear when a session ends.

MEVDAN's checkpoint architecture provides foundations for:

- Recovering project state
- Preserving important execution states
- Auditing work
- Replaying workflows
- Continuing after failures
- Building reliable autonomous workflows

---

## 🤝 Multi-Agent Work

Different agents can participate in different parts of the same project.

```text
                       ┌─────────────┐
                       │   PROJECT   │
                       └──────┬──────┘
                              │
             ┌────────────────┼────────────────┐
             ▼                ▼                ▼
        ┌─────────┐      ┌─────────┐      ┌─────────┐
        │Research │      │  Code   │      │  Tests  │
        │  Agent  │      │  Agent  │      │  Agent  │
        └────┬────┘      └────┬────┘      └────┬────┘
             └────────────────┼────────────────┘
                              ▼
                        ┌───────────┐
                        │ VERIFY    │
                        └───────────┘
```

MEVDAN supports the architecture required for:

- Agent roles
- Delegation
- Handoff
- Parallel execution
- Shared project state
- Coordinated workflows

---

## 🔄 Agent Handoff

The project should never belong permanently to one agent.

A workflow can move between agents:

```text
Agent A
  │
  ├── Research
  ▼
Work Graph
  │
  ├── Handoff
  ▼
Agent B
  │
  ├── Implementation
  ▼
Agent C
  │
  └── Verification
```

The **work state survives the handoff**.

---

## 🧩 Skills

MEVDAN provides an extensible skill architecture for reusable capabilities and workflows.

The system includes foundations for:

- Skill discovery
- Skill installation
- Skill isolation
- Skill metadata
- Skill execution
- Skill security

---

## 🔌 MCP

MEVDAN integrates with the **Model Context Protocol** ecosystem.

MCP allows the runtime to connect agents with external tools and capabilities while MEVDAN remains responsible for the surrounding runtime concerns:

```text
Agent
  │
  ▼
MEVDAN Permissions
  │
  ▼
MCP / Tools
  │
  ▼
External Capability
```

---

## 🛠️ Developer Workflows

MEVDAN is designed to support serious software-engineering workflows.

Current architecture includes foundations for:

- Filesystem operations
- Shell execution
- Git
- Git worktrees
- LSP
- Code intelligence
- Testing
- Verification
- Artifacts
- Multi-agent execution
- Automation

Example:

```text
Analyze repository
       ↓
Create plan
       ↓
Inspect code
       ↓
Implement changes
       ↓
Run tests
       ↓
Collect evidence
       ↓
Verify results
       ↓
Create checkpoint
       ↓
Produce report
```

---

# 🏗️ Architecture

MEVDAN is implemented as a modular **Rust workspace**.

```text
┌────────────────────────────────────────────────────────┐
│                      MEVDAN                            │
├────────────────────────────────────────────────────────┤
│                                                        │
│  CLIENTS                                               │
│  ├── CLI                                               │
│  ├── Desktop                                           │
│  └── Android                                           │
│                                                        │
├────────────────────────────────────────────────────────┤
│                                                        │
│  RUNTIME                                                │
│  ├── Core                                               │
│  ├── Agent Engine                                       │
│  ├── Context                                            │
│  ├── Router                                             │
│  ├── Permissions                                        │
│  └── Policy                                             │
│                                                        │
├────────────────────────────────────────────────────────┤
│                                                        │
│  WORK                                                   │
│  ├── Work Graph                                         │
│  ├── Tasks                                              │
│  ├── Artifacts                                          │
│  ├── Evidence                                           │
│  ├── Verification                                       │
│  └── Checkpoints                                        │
│                                                        │
├────────────────────────────────────────────────────────┤
│                                                        │
│  INTELLIGENCE                                           │
│  ├── Providers                                          │
│  ├── Models                                             │
│  ├── Agents                                             │
│  ├── Skills                                             │
│  └── MCP                                                │
│                                                        │
├────────────────────────────────────────────────────────┤
│                                                        │
│  DEVELOPER & PRODUCTIVITY                              │
│  ├── Git / Worktrees                                    │
│  ├── LSP / Code Intelligence                            │
│  ├── Automation                                         │
│  ├── Documents / Data / Media                           │
│  ├── Web                                                │
│  └── Editor                                             │
│                                                        │
└────────────────────────────────────────────────────────┘
```

### Technology

| Layer | Technology |
|---|---|
| 🦀 Runtime | Rust |
| 💾 Storage | SQLite + Filesystem |
| 🖥️ Desktop | Tauri 2 |
| 📱 Android | Tauri 2 |
| 🌿 Version Control | Git |
| 🔌 Agent Connectivity | MCP |
| 🧠 Code Intelligence | LSP |
| 🏗️ CI/CD | GitHub Actions |
| 📜 License | MIT |

---

# 📁 Project Structure

```text
mevdan/
├── .github/
│   └── workflows/
├── crates/
│   ├── mevdan-core
│   ├── mevdan-storage
│   ├── mevdan-config
│   ├── mevdan-secrets
│   ├── mevdan-provider
│   ├── mevdan-models
│   ├── mevdan-agent
│   ├── mevdan-context
│   ├── mevdan-tools
│   ├── mevdan-permissions
│   ├── mevdan-workgraph
│   ├── mevdan-task
│   ├── mevdan-verification
│   ├── mevdan-checkpoint
│   ├── mevdan-skills
│   ├── mevdan-mcp
│   ├── mevdan-lsp
│   ├── mevdan-codeintel
│   ├── mevdan-router
│   ├── mevdan-worktree
│   ├── mevdan-automation
│   ├── mevdan-documents
│   ├── mevdan-data
│   ├── mevdan-media
│   ├── mevdan-web
│   ├── mevdan-editor
│   └── mevdan-cli
├── docs/
├── scripts/
├── Cargo.toml
├── CONTRIBUTING.md
├── GOVERNANCE.md
├── SECURITY.md
├── VERSIONS.md
└── LICENSE
```

> The workspace is intentionally modular so the runtime can grow without becoming a monolith.

---

# 🗺️ Roadmap

MEVDAN 1.0 is organized into **100 development phases**.

### Current progress

```text
0.1.0   ████████████████████  01–10  ✓
0.2.0   ████████████████████  11–20  ✓
0.3.0   ████████████████████  21–30  ✓
0.4.0   ████████████████████  31–40  ✓
0.5.0   ████████████████████  41–50  ✓
0.6.0   ░░░░░░░░░░░░░░░░░░░░  51–60
0.7.0   ░░░░░░░░░░░░░░░░░░░░  61–70
0.8.0   ░░░░░░░░░░░░░░░░░░░░  71–80
0.9.0   ░░░░░░░░░░░░░░░░░░░░  81–90
1.0.0   ░░░░░░░░░░░░░░░░░░░░  91–100
```

## Current milestone — `0.5.0`

The project has completed the first **50 phases** of the planned 100-phase roadmap.

The completed milestones establish the runtime foundation and expand it into:

- Agent execution
- Provider/model abstraction
- Context
- Tools
- Permissions
- Work Graph
- Tasks
- Verification
- Checkpoints
- Skills
- MCP
- LSP
- Code intelligence
- Routing
- Multi-agent workflows
- Handoff
- Parallel execution
- Worktrees
- Automation
- Documents
- Data
- Media
- Web
- Editor foundations

## Next milestones

The upcoming roadmap focuses on turning the runtime foundation into a more complete end-user product:

- 🧠 Memory
- 🏠 Local-model & offline experience
- 🖥️ Desktop UX
- ✅ Approval Center
- 📦 Import / Export
- 🔌 Plugin architecture
- 🛡️ Security hardening
- 🧬 Prompt-injection defense
- 🔐 Trust model
- 📏 Resource limits
- 💰 Cost controls
- 🔀 Provider fallback
- 📊 Observability
- 🧪 Testing & benchmarks
- 📦 Packaging & releases
- 🔄 Recovery & disaster recovery
- 🕵️ Privacy mode
- ♿ Accessibility
- 🌍 Internationalization

See [`VERSIONS.md`](VERSIONS.md) and the project documentation for the detailed roadmap.

---

# 🧪 Development

## Requirements

- Rust toolchain
- Cargo
- Git

Clone the repository:

```bash
git clone https://github.com/lecodev-26/mevdan.git
cd mevdan
```

Build the workspace:

```bash
cargo build --workspace
```

Run tests:

```bash
cargo test --workspace
```

Run Clippy:

```bash
cargo clippy --workspace --all-targets --all-features
```

Format:

```bash
cargo fmt --all
```

---

# 🔑 Providers & Secrets

MEVDAN follows a **Bring Your Own Key (BYOK)** philosophy for cloud providers.

Credentials should never be stored in:

- Git repositories
- Project files
- Logs
- Chat transcripts
- Committed configuration
- Source code

The architecture is designed to use the platform's secure credential mechanisms where available.

For local providers, users can run models on their own hardware.

---

# 🔒 Privacy Principles

MEVDAN is designed around user ownership.

### Principles

```text
LOCAL-FIRST
     │
     ├── Your files
     ├── Your project state
     ├── Your credentials
     ├── Your provider
     └── Your work
```

MEVDAN aims for:

- No mandatory account
- No mandatory cloud backend
- No mandatory subscription
- No mandatory telemetry
- Explicit permissions
- User-controlled providers
- Local project state

---

# 🛡️ Security

Security is treated as part of the architecture rather than an afterthought.

Important security surfaces include:

- Agent permissions
- Shell execution
- Filesystem access
- Network access
- MCP servers
- Skills
- External tools
- Untrusted files
- Web content
- Prompt injection
- Secrets

Please see [`SECURITY.md`](SECURITY.md) for the project's security policy.

---

# 🧭 The MEVDAN Work Loop

A typical autonomous workflow follows:

```text
                 USER TASK
                     │
                     ▼
                UNDERSTAND
                     │
                     ▼
                   PLAN
                     │
                     ▼
                SELECT TOOL
                     │
                     ▼
             REQUEST PERMISSION
                     │
                     ▼
                 EXECUTE
                     │
                     ▼
                 OBSERVE
                     │
                     ▼
             UPDATE WORK STATE
                     │
                     ▼
                 CONTINUE
                     │
                     ▼
                  VERIFY
                     │
                     ▼
                CHECKPOINT
                     │
                     ▼
                  FINISH
```

The runtime maintains the state throughout this loop.

---

# 📦 Universal Work Package

A long-term MEVDAN goal is to make AI work portable.

A project can ultimately be represented as a durable package containing concepts such as:

```text
Project
├── Goals
├── Requirements
├── Constraints
├── Tasks
├── Decisions
├── Artifacts
├── Evidence
├── Verification
├── Checkpoints
├── Agent state
└── Configuration
```

This makes it possible to preserve, back up, move, and continue work independently of a particular model or agent.

---

# 🌍 Open Source

MEVDAN is released under the **MIT License**.

The project is intended to remain:

- 🆓 Free to use
- 🔓 Open source
- 🏠 Local-first
- 🔀 Provider-agnostic
- 🧩 Extensible
- 👤 User-owned

---

# 🤝 Contributing

Contributions are welcome.

Before contributing, read:

- [`CONTRIBUTING.md`](CONTRIBUTING.md)
- [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)
- [`GOVERNANCE.md`](GOVERNANCE.md)
- [`SECURITY.md`](SECURITY.md)

### Development principles

1. Keep the architecture modular.
2. Add tests with new functionality.
3. Avoid unnecessary dependencies.
4. Preserve local-first behavior.
5. Never leak secrets.
6. Keep provider integrations replaceable.
7. Prefer evidence and verification over assumptions.
8. Avoid introducing mandatory cloud infrastructure.
9. Document important architectural decisions.
10. Keep the runtime independent from any single AI provider.

---

# ⭐ The Vision

MEVDAN is not trying to be another chat application.

It is being built as a runtime where AI can perform **real, persistent, verifiable work** while the user remains in control.

```text
          ANY MODEL
              │
          ANY AGENT
              │
              ▼
       ┌─────────────┐
       │   MEVDAN    │
       │   RUNTIME   │
       └──────┬──────┘
              │
              ▼
          YOUR WORK
```

The model can change.

The agent can change.

The provider can change.

The tools can change.

### The work remains.

---

<div align="center">

## MEVDAN

**Any model. Any agent. Your work.**

Open-source • Local-first • Verifiable • Extensible

<br>

[MIT License](LICENSE) · [Security](SECURITY.md) · [Contributing](CONTRIBUTING.md)

</div>
