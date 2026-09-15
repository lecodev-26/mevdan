# MEVDAN

**Any model. Any agent. Your work.**

MEVDAN is an open-source, local-first AI work runtime.

> The model proposes; MEVDAN decides and executes through audited,
> permission-gated tools.

## What this is

MEVDAN is **not another chatbot**. It's a runtime where:

- The **model** is replaceable.
- The **agent** is replaceable.
- **The work is not.**

A project's state, requirements, tasks, decisions, artifacts, evidence,
verification results and checkpoints belong to the user — not to any
provider, model, or agent.

## Current version

**V0.1.0 — Foundation**

Two commands working:

```bash
mevdan init <name>    # Create a new MEVDAN project
mevdan status         # Show current project state
```

Everything else is planned. See VERSIONS.md for the roadmap.

Philosophy

· Local-first — no servers required.
· No account — no sign-up, no telemetry.
· Provider-agnostic — bring your own model.
· Agent-agnostic — swap agents without losing work.
· Verification over claims — agents propose, MEVDAN verifies.
· User owns the work — export, inspect, and move freely.

Quick start

Requirements

· Rust 1.75 or later
· Git
· On Termux: pkg install rust git clang make pkg-config

Build

```bash
git clone https://github.com/lecodev-26/mevdan.git
cd mevdan
cargo build --release
```

Use

```bash
./target/release/mevdan init demo
cd demo
../target/release/mevdan status
```

Project layout

```
mevdan/
├── crates/
│   ├── mevdan-core/       # Pure domain: types, errors, entities
│   ├── mevdan-storage/    # SQLite persistence + event log
│   └── mevdan-cli/        # Command-line interface
├── Cargo.toml
├── VERSIONS.md
└── LICENSE
```

Development

```bash
cargo build              # compile
cargo test               # run all tests
cargo fmt                # format
cargo clippy             # lint
```

License

MIT — see LICENSE.

Status

Early development. API may change. Contributions welcome once the
foundation is stable.
