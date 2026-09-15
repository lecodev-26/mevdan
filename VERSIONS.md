# MEVDAN — Versions

Este documento lista las versiones del proyecto y qué contiene cada una.

## Versionado

- `0.x.y` → pre-release. La API puede cambiar sin previo aviso.
- `1.0.0` → primera versión estable. Compromiso de compatibilidad.

## Estado actual

**Versión activa:** `0.1.0` ✅ **COMPLETADA** (2026-09-15)

**Próxima versión:** `0.2.0` (en planificación)

---

## ✅ V0.1.0 — Foundation (COMPLETADA)

**Fecha de cierre:** 2026-09-15

**Objetivo:** tener un runtime mínimo donde `mevdan init` cree un proyecto local
y `mevdan status` lo inspeccione.

**Crates:**

- ✅ `mevdan-core` — Dominio puro: IDs tipados, errores, Project, Session, Event, versiones. **30 tests.**
- ✅ `mevdan-storage` — Persistencia local SQLite: migraciones, repos, event log append-only. **26 tests.**
- ✅ `mevdan-cli` — Interfaz de línea de comandos: `init`, `status`.

**Comandos:**

- ✅ `mevdan init <name>` — crea un proyecto MEVDAN.
- ✅ `mevdan status` — muestra el estado del proyecto actual.

**Contenido del proyecto:**

- ✅ `.mevdan/project.toml` — config legible por humanos + schema_version.
- ✅ `.mevdan/mevdan.db` — SQLite con estado canónico + event log.
- ✅ `.mevdan/artifacts/` — vacío en V0.1.0.
- ✅ `.mevdan/checkpoints/` — vacío en V0.1.0.
- ✅ `.mevdan/events/` — vacío en V0.1.0.

**Garantías:**

- ✅ Local-first: no requiere servidor ni cuenta.
- ✅ Append-only event log desde el día 1.
- ✅ Schema versioning desde el día 1.
- ✅ `mevdan-core` no depende de I/O.
- ✅ 56 tests pasando en Termux aarch64.

**Demo verificada:**

```bash
$ mevdan init demo-v010
✔ MEVDAN project initialized at ./demo-v010
  Project ID: 01a0a493-223b-7461-97fb-0a408cdf5dd4
  Session ID: 01a0a493-2252-77e0-8f56-ea73eb015a67

$ cd demo-v010
$ mevdan status
MEVDAN — Project Status
───────────────────────────────────────
Name:           demo-v010
Schema:         0.1.0
MEVDAN version: 0.1.0
Sessions:       1
Events:         1
```

---

Roadmap de versiones futuras

Versión Contenido Estado
V0.1.0 Foundation: core + storage + CLI (init, status) ✅ Completada
V0.2.0 Provider abstraction + OpenAI-compatible + Ollama ⏳ Pendiente
V0.3.0 Agent engine + tool-calling adapter ⏳ Pendiente
V0.4.0 Tools: filesystem, shell, git ⏳ Pendiente
V0.5.0 Permission engine (ALLOW/ASK/DENY) ⏳ Pendiente
V0.6.0 Work Graph básico ⏳ Pendiente
V0.7.0 Checkpoints + mevdan continue ⏳ Pendiente
V0.8.0 Verification engine ⏳ Pendiente
V0.9.0 Audit + Doctor + mevdan run real ⏳ Pendiente
V1.0.0 MVP cerrado, documentación, primer release ⏳ Pendiente

---

Principios rectores

1. The model is replaceable. The agent is replaceable. The work is not.
2. The model proposes; MEVDAN decides and executes through audited,
   permission-gated tools.
3. Verification over claims.
4. Local-first. Provider-agnostic. Agent-agnostic.
5. Everything observable.
   EOF

git add VERSIONS.md
git commit -m "docs: mark V0.1.0 as completed"
cat > VERSIONS.md << 'EOF'
# MEVDAN — Versions

Este documento lista las versiones del proyecto y qué contiene cada una.

## Versionado

- `0.x.y` → pre-release. La API puede cambiar sin previo aviso.
- `1.0.0` → primera versión estable. Compromiso de compatibilidad.

## Estado actual

**Versión activa:** `0.1.0` ✅ **COMPLETADA** (2026-09-15)

**Próxima versión:** `0.2.0` (en planificación)

---

## ✅ V0.1.0 — Foundation (COMPLETADA)

**Fecha de cierre:** 2026-09-15

**Objetivo:** tener un runtime mínimo donde `mevdan init` cree un proyecto local
y `mevdan status` lo inspeccione.

**Crates:**

- ✅ `mevdan-core` — Dominio puro: IDs tipados, errores, Project, Session, Event, versiones. **30 tests.**
- ✅ `mevdan-storage` — Persistencia local SQLite: migraciones, repos, event log append-only. **26 tests.**
- ✅ `mevdan-cli` — Interfaz de línea de comandos: `init`, `status`.

**Comandos:**

- ✅ `mevdan init <name>` — crea un proyecto MEVDAN.
- ✅ `mevdan status` — muestra el estado del proyecto actual.

**Contenido del proyecto:**

- ✅ `.mevdan/project.toml` — config legible por humanos + schema_version.
- ✅ `.mevdan/mevdan.db` — SQLite con estado canónico + event log.
- ✅ `.mevdan/artifacts/` — vacío en V0.1.0.
- ✅ `.mevdan/checkpoints/` — vacío en V0.1.0.
- ✅ `.mevdan/events/` — vacío en V0.1.0.

**Garantías:**

- ✅ Local-first: no requiere servidor ni cuenta.
- ✅ Append-only event log desde el día 1.
- ✅ Schema versioning desde el día 1.
- ✅ `mevdan-core` no depende de I/O.
- ✅ 56 tests pasando en Termux aarch64.

**Demo verificada:**

```bash
$ mevdan init demo-v010
✔ MEVDAN project initialized at ./demo-v010
  Project ID: 01a0a493-223b-7461-97fb-0a408cdf5dd4
  Session ID: 01a0a493-2252-77e0-8f56-ea73eb015a67

$ cd demo-v010
$ mevdan status
MEVDAN — Project Status
───────────────────────────────────────
Name:           demo-v010
Schema:         0.1.0
MEVDAN version: 0.1.0
Sessions:       1
Events:         1
```

---

Roadmap de versiones futuras

Versión Contenido Estado
V0.1.0 Foundation: core + storage + CLI (init, status) ✅ Completada
V0.2.0 Provider abstraction + OpenAI-compatible + Ollama ⏳ Pendiente
V0.3.0 Agent engine + tool-calling adapter ⏳ Pendiente
V0.4.0 Tools: filesystem, shell, git ⏳ Pendiente
V0.5.0 Permission engine (ALLOW/ASK/DENY) ⏳ Pendiente
V0.6.0 Work Graph básico ⏳ Pendiente
V0.7.0 Checkpoints + mevdan continue ⏳ Pendiente
V0.8.0 Verification engine ⏳ Pendiente
V0.9.0 Audit + Doctor + mevdan run real ⏳ Pendiente
V1.0.0 MVP cerrado, documentación, primer release ⏳ Pendiente

---

Principios rectores

1. The model is replaceable. The agent is replaceable. The work is not.
2. The model proposes; MEVDAN decides and executes through audited,
   permission-gated tools.
3. Verification over claims.
4. Local-first. Provider-agnostic. Agent-agnostic.
5. Everything observable.
   EOF

