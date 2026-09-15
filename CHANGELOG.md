# Changelog

All notable changes to MEVDAN are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Planned
- Provider abstraction (trait + OpenAI-compatible + Ollama adapters)
- Configuration system (global + per-project)
- Secret management (keyring integration)

---

## [0.1.0] - 2026-09-15

### Added
- **`mevdan-core`** — pure domain model:
  - Typed IDs (`ProjectId`, `SessionId`, `EventId`) using UUID v7
  - `Project`, `Session`, `Event` entities
  - `CoreError` and `CoreResult`
  - Schema versioning helpers
  - 30 unit tests

- **`mevdan-storage`** — local SQLite persistence:
  - `Database` with migration system
  - Schema versioning from day 1
  - Append-only event log
  - Repositories: `project_repo`, `session_repo`, `event_repo`
  - 26 unit tests

- **`mevdan-cli`** — command-line interface:
  - `mevdan init <name>` — creates a new project
  - `mevdan status` — shows current project state

- **Documentation:**
  - `README.md` with philosophy and quick start
  - `VERSIONS.md` with roadmap
  - `LICENSE` (MIT)
  - `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`,
    `GOVERNANCE.md`, `CHANGELOG.md`

### Notes
- First usable version.
- Works on Termux (Android aarch64) using system SQLite.
- No network required.
- No telemetry.
