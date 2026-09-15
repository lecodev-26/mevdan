# MEVDAN — Versions

Este documento lista las versiones del proyecto y qué contiene cada una.

## Versionado

- `0.x.y` → pre-release. La API puede cambiar sin previo aviso.
- `1.0.0` → primera versión estable. Compromiso de compatibilidad.

## Estado actual

**Versión activa:** `0.1.0` (en desarrollo)

---

## V0.1.0 — Foundation (MVP inicial)

**Objetivo:** tener un runtime mínimo donde `mevdan init` cree un proyecto local
y `mevdan status` lo inspeccione.

**Crates:**

- `mevdan-core` — Dominio puro: IDs, errores, Project, Session, Event, versiones.
- `mevdan-storage` — Persistencia local SQLite: migraciones, repos, event log append-only.
- `mevdan-cli` — Interfaz de línea de comandos: `init`, `status`.

**Comandos:**

- `mevdan init <name>` — crea un proyecto MEVDAN.
- `mevdan status` — muestra el estado del proyecto actual.

**Contenido del proyecto:**

- `.mevdan/project.toml` — config legible por humanos + schema_version.
- `.mevdan/mevdan.db` — SQLite con estado canónico + event log.
- `.mevdan/artifacts/` — vacío en V0.1.0.
- `.mevdan/checkpoints/` — vacío en V0.1.0.
- `.mevdan/events/` — vacío en V0.1.0.

**Garantías:**

- Local-first: no requiere servidor ni cuenta.
- Append-only event log desde el día 1.
- Schema versioning desde el día 1.
- `mevdan-core` no depende de I/O.

---

## Roadmap de versiones futuras

| Versión | Contenido |
|---------|-----------|
| **V0.2.0** | Provider abstraction + OpenAI-compatible + Ollama |
| **V0.3.0** | Agent engine + tool-calling adapter |
| **V0.4.0** | Tools: filesystem, shell, git |
| **V0.5.0** | Permission engine (ALLOW/ASK/DENY) |
| **V0.6.0** | Work Graph básico |
| **V0.7.0** | Checkpoints + `mevdan continue` |
| **V0.8.0** | Verification engine |
| **V0.9.0** | Audit + Doctor + `mevdan run` real |
| **V1.0.0** | MVP cerrado, documentación, primer release |

---

## Principios rectores

1. **The model is replaceable. The agent is replaceable. The work is not.**
2. **The model proposes; MEVDAN decides and executes through audited,
   permission-gated tools.**
3. **Verification over claims.**
4. **Local-first. Provider-agnostic. Agent-agnostic.**
5. **Everything observable.**
