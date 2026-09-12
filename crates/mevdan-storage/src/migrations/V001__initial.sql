-- ============================================================
-- MEVDAN — Migración inicial V001
-- ============================================================
-- Esta migración crea el esquema base de un proyecto MEVDAN.
--
-- Reglas:
--   1. Los eventos son APPEND-ONLY (no triggers de UPDATE/DELETE
--      en esta versión; se añadirán en M4).
--   2. Los timestamps son TEXT en formato ISO-8601 UTC.
--   3. Los IDs son TEXT (UUID v7 como string).
--   4. Nada de BLOB para artefactos grandes; esos van a disco.
-- ============================================================

PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

-- ============================================================
-- meta: metadatos del esquema
-- ============================================================
CREATE TABLE meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT INTO meta (key, value) VALUES
    ('schema_version', '0.1.0'),
    ('created_at',     strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

-- ============================================================
-- projects
-- ============================================================
CREATE TABLE projects (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    schema_version  TEXT NOT NULL,
    mevdan_version  TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    config_json     TEXT NOT NULL DEFAULT '{}'
);

-- ============================================================
-- sessions
-- ============================================================
CREATE TABLE sessions (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    label       TEXT,
    status      TEXT NOT NULL CHECK (status IN ('active','paused','completed','failed')),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE INDEX idx_sessions_project_id ON sessions(project_id);
CREATE INDEX idx_sessions_status     ON sessions(status);

-- ============================================================
-- events (append-only)
-- ============================================================
CREATE TABLE events (
    id           TEXT PRIMARY KEY,
    project_id   TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    session_id   TEXT REFERENCES sessions(id) ON DELETE SET NULL,
    occurred_at  TEXT NOT NULL,
    kind         TEXT NOT NULL,
    payload_json TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX idx_events_project_time ON events(project_id, occurred_at);
CREATE INDEX idx_events_session      ON events(session_id);
CREATE INDEX idx_events_kind         ON events(kind);
