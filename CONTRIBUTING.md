# Contributing to MEVDAN

Thank you for your interest in MEVDAN. This document explains how to
contribute.

## Before you start

MEVDAN is in **early development** (V0.1.0). The public API may change
without notice. If you plan to contribute something substantial, please
open an issue first to discuss.

## Ground rules

1. **Respect the architecture.** Read `docs/ARCHITECTURE.md` (once it
   exists) and the principles in `VERSIONS.md`.
2. **Small, focused PRs.** One change per PR. No massive rewrites.
3. **Tests are mandatory.** Every new feature needs tests.
4. **No breaking changes** to persistent formats without a migration and
   a schema version bump.
5. **No vendor lock-in.** No hard-coded assumptions about a single
   provider, model, or service.

## Development setup

### Requirements

- Rust 1.75 or later
- Git
- On Termux (Android): `pkg install rust git clang make pkg-config`

### Build and test

```bash
git clone https://github.com/lecodev-26/mevdan.git
cd mevdan
cargo build
cargo test
cargo fmt --all
cargo clippy --all-targets -- -D warnings
```

All four commands must pass before you submit a PR.

Code style

· Rust 2021 edition.
· cargo fmt — formatting is non-negotiable.
· cargo clippy -D warnings — no warnings allowed.
· Test coverage for new functionality.
· Doc comments (///) on public items.

Commit messages

Use clear, imperative commit messages:

```
feat: add Ollama provider adapter
fix: correct timestamp parsing in event_repo
docs: update README quick start
test: add tests for provider capabilities
refactor: split agent engine into modules
chore: bump dependencies
```

Format: <type>: <short description>

Types: feat, fix, docs, test, refactor, chore, perf, ci.

Pull request process

1. Fork the repository.
2. Create a branch: git checkout -b feat/your-feature
3. Make your changes with tests.
4. Ensure cargo fmt, cargo clippy -D warnings, cargo test all pass.
5. Push and open a PR against main.
6. A maintainer will review. Expect feedback.

What we will not accept

· Code copied from projects with incompatible licenses.
· Features that require a MEVDAN-hosted server.
· Telemetry enabled by default.
· Anything that breaks the "local-first" principle.

Code of Conduct

By participating, you agree to abide by our
Code of Conduct.

Security

To report a security vulnerability, see SECURITY.md.

License

By contributing, you agree that your contributions will be licensed
under the MIT License (see LICENSE).
