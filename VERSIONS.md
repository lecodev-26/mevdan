# MEVDAN — Versions

## Versionado

- `0.x.y` → pre-release. La API puede cambiar sin previo aviso.
- `1.0.0` → primera versión estable.

## Estado actual

**Versión activa:** `0.1.0` ✅ **COMPLETADA** (2026-09-15)

**Próxima versión:** `0.2.0` (planificada)

---

## ✅ V0.1.0 — Foundation (COMPLETADA 2026-09-15)

**Fases del roadmap cubiertas: 01, 02, 03, 04, 05, 06, 07, 08, 09, 10**

### Crates

- ✅ `mevdan-core` — dominio puro: IDs tipados, errores, Project, Session, Event. **30 tests.**
- ✅ `mevdan-storage` — SQLite: migraciones, repos, event log append-only. **26 tests.**
- ✅ `mevdan-config` — configuración global + proyecto. **19 tests.**
- ✅ `mevdan-secrets` — gestión de secretos con permisos 0600. **18 tests.**
- ✅ `mevdan-provider` — abstracción + OpenAI-compatible + Ollama. **61 tests.**
- ✅ `mevdan-cli` — `init`, `status`, `chat`.

### Comandos

- ✅ `mevdan init <name>` — crea un proyecto.
- ✅ `mevdan status` — muestra el estado del proyecto actual.
- ✅ `mevdan chat <message>` — habla con un provider (Ollama o compatible con OpenAI).

### Garantías

- ✅ Local-first. No requiere servidor ni cuenta.
- ✅ Event log append-only desde el día 1.
- ✅ Schema versioning desde el día 1.
- ✅ `mevdan-core` no depende de I/O.
- ✅ Permisos `0600` para archivos de secretos.
- ✅ Provider-agnostic (abstracción + 2 adapters).
- ✅ **156 tests pasando** en Linux/macOS/Windows.
- ✅ CI verde (fmt + clippy + tests + docs + audit).

### Demos verificadas

```bash
$ mevdan init demo
✔ MEVDAN project initialized at ./demo

$ cd demo && mevdan status
MEVDAN — Project Status
Name:           demo
Sessions:       1
Events:         1
```

```bash
$ mevdan chat 'Hola' -m llama3.2 -p ollama
¡Hola! ¿En qué puedo ayudarte?
[tokens: 15 in / 8 out = 23 total]
```

---

Roadmap de versiones futuras

Versión Fases Contenido
V0.1.0 01–10 ✅ COMPLETADA
V0.2.0 11–20 Model Registry, Agent Engine, Context Engine, Context Compaction, Filesystem, Shell, Git, Permissions, Risk, Work Graph
V0.3.0 21–30 Task Engine, Artifacts, Evidence, Verification, Checkpoints, Recovery, Audit, Replay, Skills, Skill Isolation
V0.4.0 31–40 Skill Discovery, MCP, LSP, Code Intelligence, Multi-Agent, Router, Handoff, Parallel
V0.5.0 41–50 Worktree, Automation, Scheduling, Documents, Data, Image, Audio, Video, Browser, Web Research
V0.6.0 51–60 Citation, Memory, Local Models, Offline, Desktop, Approval, Artifact Center, Android
V0.7.0 61–70 Import, Compatibility, Plugins, Extensions, Registry, Skills, Security, Prompt Injection, Trust
V0.8.0 71–80 Limits, Cost, Fallback, Observability, Testing, Benchmarks, Docs, Install, Release, Update
V0.9.0 81–90 Crash, Telemetry, Privacy, Team, Remote, Containers, Computer Use, Automation, Router, Autonomy
V1.0.0 91–100 Policy, Templates, One-Click, Universal Project, Backup, Recovery, Accessibility, i18n, 1.0

---

Principios rectores

1. The model is replaceable. The agent is replaceable. The work is not.
2. The model proposes; MEVDAN decides and executes through audited, permission-gated tools.
3. Verification over claims.
4. Local-first. Provider-agnostic. Agent-agnostic.
5. Everything observable.
   EOF
