-- ============================================================
-- MEVDAN — Migración V002
-- ============================================================
-- Añade la tabla `workgraphs` para persistir el Work Graph.
--
-- Decisión de diseño: el Work Graph se guarda como un blob JSON
-- en una sola columna. Esto:
--   1. Simplifica el esquema (no normalizamos nodos/aristas).
--   2. Permite que los `data` de cada nodo sean JSON libre.
--   3. Facilita export/import.
--   4. Se puede migrar a tablas normalizadas en V003+ si el
--      rendimiento lo requiere.
-- ============================================================

CREATE TABLE workgraphs (
    id              TEXT PRIMARY KEY,
    project_id      TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    label           TEXT,
    schema_version  TEXT NOT NULL,
    json            TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE INDEX idx_workgraphs_project_id ON workgraphs(project_id);
CREATE INDEX idx_workgraphs_updated_at ON workgraphs(updated_at);

-- Actualizamos el schema_version del meta.
UPDATE meta SET value = '0.2.0' WHERE key = 'schema_version';
