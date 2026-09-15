-- ============================================================
-- MEVDAN — Migración V003
-- ============================================================
-- Añade la tabla `checkpoints` para persistir snapshots de trabajo.
--
-- Decisión de diseño: `WorkState` se guarda como blob JSON, igual
-- que `WorkGraph`. Simplifica el esquema.
-- ============================================================

CREATE TABLE checkpoints (
    id              TEXT PRIMARY KEY,
    project_id      TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    label           TEXT,
    schema_version  TEXT NOT NULL,
    json            TEXT NOT NULL,
    created_at      TEXT NOT NULL
);

CREATE INDEX idx_checkpoints_project_created
    ON checkpoints(project_id, created_at DESC);

-- Actualizamos el schema_version del meta.
UPDATE meta SET value = '0.3.0' WHERE key = 'schema_version';
