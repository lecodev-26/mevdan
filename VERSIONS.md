# MEVDAN — Versions

## Versionado

- `0.x.y` → pre-release. La API puede cambiar sin previo aviso.
- `1.0.0` → primera versión estable.

## Estado actual

**Versión activa:** `0.3.0` ✅ **COMPLETADA** (2026-09-15)

**Próxima versión:** `0.4.0` (en planificación)

---

## ✅ V0.1.0 — Foundation (COMPLETADA 2026-09-15)

**Fases del roadmap cubiertas: 01-10**

### Crates

- `mevdan-core` — Dominio puro: IDs tipados, errores, Project, Session, Event.
- `mevdan-storage` — SQLite: migraciones, repos, event log append-only.
- `mevdan-config` — Configuración global + proyecto.
- `mevdan-secrets` — Gestión de secretos con permisos 0600.
- `mevdan-provider` — Abstracción + OpenAI-compatible + Ollama.
- `mevdan-cli` — `init`, `status`, `chat`.

---

## ✅ V0.2.0 — Agent, Tools, Permissions, Work Graph (COMPLETADA 2026-09-15)

**Fases del roadmap cubiertas: 11-20**

### Crates nuevos

- `mevdan-models` — Registro de modelos con capacidades.
- `mevdan-agent` — Motor de agentes con loop de razonamiento, auto-review, replanning, multi-agente.
- `mevdan-context` — Context Engine con compactación estructurada.
- `mevdan-tools` — Filesystem (sandboxed), Shell (allowlist), Git (validado).
- `mevdan-permissions` — Permission Engine + Risk Engine con auto-escalado.
- `mevdan-workgraph` — Work Graph con nodos tipados, aristas, ciclos, topological sort, SQLite.

### Schema

- Versión: `0.2.0`.
- Migraciones: V001, V002.

---

## ✅ V0.3.0 — Tasks, Verification, Checkpoints, Skills (COMPLETADA 2026-09-15)

**Fases del roadmap cubiertas: 21-30**

### Crates nuevos

- `mevdan-task` — Task Engine con estados, dependencias, ciclo detection, topological order.
- `mevdan-verification` — Artifacts (SHA-256), Evidence, Claims, Verificadores (FileExists, Hash, CommandExit), VerificationEngine.
- `mevdan-checkpoint` — Checkpoints con WorkState, CheckpointEngine, RecoveryEngine (resume/retry/rollback/continue), persistencia SQLite.
- `mevdan-skills` — SkillManifest (TOML), SkillLoader, SkillRegistry, SkillIsolation.

### Ampliaciones

- `mevdan-storage` — `checkpoint_repo`, `audit` (AuditTrail, Replay), migración V003.
- `mevdan-cli` — Nuevos comandos: `tasks`, `audit`.

### Comandos

- `mevdan init <name>` — crea un proyecto.
- `mevdan status` — estado completo (sessions, events, work graphs, checkpoints).
- `mevdan chat <message>` — habla con un provider.
- `mevdan tasks` — lista tareas del Work Graph.
- `mevdan audit` — timeline completo del proyecto.

### Schema

- Versión: `0.3.0`.
- Migraciones: V001, V002, V003.

### Garantías

- **940 tests pasando** en Linux, macOS, Windows.
- CI verde (fmt + clippy + tests + docs + audit).
- Verificación real con SHA-256.
- Checkpoints con rollback.
- Audit trail sobre el Event Log.
- Skills declarativas con isolation.

---

## Roadmap de versiones futuras

| Versión | Fases | Contenido |
|---------|-------|-----------|
| **V0.1.0** | 01–10 | ✅ Foundation |
| **V0.2.0** | 11–20 | ✅ Agent + Tools + Permissions + Work Graph |
| **V0.3.0** | 21–30 | ✅ Tasks + Verification + Checkpoints + Skills |
| **V0.4.0** | 31–40 | ⏳ MCP, LSP, Multi-Agent, Router, Handoff, Parallel |
| **V0.5.0** | 41–50 | ⏳ Worktree, Automation, Documents, Data, Image, Audio, Video, Browser, Web |
| **V0.6.0** | 51–60 | ⏳ Memory, Local Models, Offline, Desktop, Approval, Android |
| **V0.7.0** | 61–70 | ⏳ Import, Compatibility, Plugins, Extensions, Registry, Security, Trust |
| **V0.8.0** | 71–80 | ⏳ Limits, Cost, Fallback, Observability, Testing, Benchmarks, Install, Release |
| **V0.9.0** | 81–90 | ⏳ Crash, Telemetry, Privacy, Team, Remote, Containers, Computer Use, Autonomy |
| **V1.0.0** | 91–100 | ⏳ Policy, Templates, One-Click, Universal Project, Backup, Recovery, i18n, 1.0 |

---

## Principios rectores

1. **The model is replaceable. The agent is replaceable. The work is not.**
2. **The model proposes; MEVDAN decides and executes through audited, permission-gated tools.**
3. **Verification over claims.**
4. **Local-first. Provider-agnostic. Agent-agnostic.**
5. **Everything observable.**
