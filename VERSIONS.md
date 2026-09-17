# MEVDAN — Versions

## Versionado

- `0.x.y` → pre-release. La API puede cambiar sin previo aviso.
- `1.0.0` → primera versión estable.

## Estado actual

**Versión activa:** `0.5.0` ✅ **COMPLETADA** (2026-09-18)

**Próxima versión:** `0.6.0` (en planificación)

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
- `mevdan-agent` — Motor de agentes con loop de razonamiento.
- `mevdan-context` — Context Engine con compactación.
- `mevdan-tools` — Filesystem, Shell, Git.
- `mevdan-permissions` — Permission Engine + Risk Engine.
- `mevdan-workgraph` — Work Graph con nodos tipados.

---

## ✅ V0.3.0 — Tasks, Verification, Checkpoints, Skills (COMPLETADA 2026-09-15)

**Fases del roadmap cubiertas: 21-30**

### Crates nuevos

- `mevdan-task` — Task Engine.
- `mevdan-verification` — Artifacts, Evidence, Claims, Verifiers.
- `mevdan-checkpoint` — Checkpoints, RecoveryEngine.
- `mevdan-skills` — SkillManifest, Loader, Registry, Isolation.

---

## ✅ V0.4.0 — Multi-Agent, MCP, LSP, Code Intel (COMPLETADA 2026-09-17)

**Fases del roadmap cubiertas: 31-40**

### Crates nuevos

- `mevdan-mcp` — Cliente MCP.
- `mevdan-lsp` — Cliente LSP.
- `mevdan-codeintel` — Repo map, símbolos, dependencias.
- `mevdan-router` — Routing (task, agent, model).

### Ampliaciones

- `mevdan-skills` — SkillDiscovery, SkillInstaller.
- `mevdan-agent` — Team, Workflow, MultiAgentEngine, Handoff, ParallelGroup.
- `mevdan-cli` — `skills`, `mcp`, `codeintel`, `route`.

---

## ✅ V0.5.0 — Worktree, Automation, Documents, Data, Media, Web, Editor (COMPLETADA 2026-09-18)

**Fases del roadmap cubiertas: 41-50**

### Crates nuevos

- `mevdan-worktree` — Worktree, WorktreeManager. Espacios de trabajo aislados.
- `mevdan-automation` — Trigger, Action, Rule, RuleEngine. Reglas de automatización.
- `mevdan-documents` — DocumentFormat, DocumentLoader. PDF, DOCX, Markdown, Text, CSV, JSON.
- `mevdan-data` — DataValue, DataTable, DataQuery, DataLoader. CSV, JSON, JSONL.
- `mevdan-media` — MediaFormat, MediaAsset, MediaLoader. Imagen, audio, vídeo.
- `mevdan-web` — WebUrl, WebPage, WebFetcher, WebSearchProvider, WebEngine.
- `mevdan-editor` — Buffer, TextEdit, EditorEngine. Edición programática.

### Conceptos nuevos

- **Worktree** — ramas de trabajo paralelas.
- **Automation** — reglas (trigger + acción) con evaluación en runtime.
- **Documents** — extracción de contenido de documentos.
- **Data** — tablas tipadas con queries.
- **Media** — metadata de imagen/audio/vídeo sin dependencias.
- **Web** — abstracciones de browser y search.
- **Editor** — edición programática de archivos.

### Comandos totales

- `mevdan init`, `status`, `chat`, `tasks`, `audit`
- `mevdan skills list`, `mcp list`, `codeintel map`, `route <text>`
- `mevdan worktree list`, `docs read <path>`, `data summary <path>`, `media info <path>`

### Schema

- Versión: `0.3.0` (sin migración nueva en V5).

---

## Roadmap de versiones futuras

| Versión | Fases | Contenido |
|---------|-------|-----------|
| **V0.1.0** | 01–10 | ✅ Foundation |
| **V0.2.0** | 11–20 | ✅ Agent + Tools + Permissions + Work Graph |
| **V0.3.0** | 21–30 | ✅ Tasks + Verification + Checkpoints + Skills |
| **V0.4.0** | 31–40 | ✅ MCP, LSP, Code Intel, Multi-Agent, Router |
| **V0.5.0** | 41–50 | ✅ Worktree, Automation, Documents, Data, Media, Web, Editor |
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
