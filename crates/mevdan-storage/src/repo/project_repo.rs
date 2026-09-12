//! Repositorio de `Project`.
//!
//! Operaciones mínimas para M1:
//!   - insert: guarda un proyecto
//!   - get_first: lee el primer proyecto (un proyecto por .mevdan/)
//!   - get_by_id: lee un proyecto por su ID
//!
//! Los timestamps se convierten a/desde ISO-8601 UTC.

use crate::error::{StorageError, StorageResult};
use chrono::{DateTime, Utc};
use mevdan_core::{
    ids::ProjectId,
    project::{Project, ProjectConfig},
};
use rusqlite::{params, Connection, OptionalExtension};

/// Inserta un proyecto. Falla si ya existe un proyecto con el mismo ID.
pub fn insert(conn: &Connection, project: &Project) -> StorageResult<()> {
    let config_json = serde_json::to_string(&project.config)?;
    conn.execute(
        "INSERT INTO projects
            (id, name, schema_version, mevdan_version, created_at, updated_at, config_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            project.id.to_string(),
            project.name,
            project.schema_version,
            project.mevdan_version,
            project.created_at.to_rfc3339(),
            project.updated_at.to_rfc3339(),
            config_json,
        ],
    )?;
    Ok(())
}

/// Lee el primer proyecto de la base (un `.mevdan/` tiene exactamente uno).
pub fn get_first(conn: &Connection) -> StorageResult<Option<Project>> {
    conn.query_row(
        "SELECT id, name, schema_version, mevdan_version, created_at, updated_at, config_json
         FROM projects
         ORDER BY created_at ASC
         LIMIT 1",
        [],
        row_to_project,
    )
    .optional()
    .map_err(StorageError::from)
}

/// Lee un proyecto por su ID.
pub fn get_by_id(conn: &Connection, id: ProjectId) -> StorageResult<Option<Project>> {
    conn.query_row(
        "SELECT id, name, schema_version, mevdan_version, created_at, updated_at, config_json
         FROM projects
         WHERE id = ?1",
        params![id.to_string()],
        row_to_project,
    )
    .optional()
    .map_err(StorageError::from)
}

/// Convierte una fila de SQLite en `Project`.
fn row_to_project(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    let id_str: String = row.get(0)?;
    let id: ProjectId = id_str
        .parse()
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(e),
        ))?;

    let created_str: String = row.get(4)?;
    let created_at = DateTime::parse_from_rfc3339(&created_str)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
            4,
            rusqlite::types::Type::Text,
            Box::new(e),
        ))?
        .with_timezone(&Utc);

    let updated_str: String = row.get(5)?;
    let updated_at = DateTime::parse_from_rfc3339(&updated_str)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
            5,
            rusqlite::types::Type::Text,
            Box::new(e),
        ))?
        .with_timezone(&Utc);

    let config_str: String = row.get(6)?;
    let config: ProjectConfig = serde_json::from_str(&config_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(e))
    })?;

    Ok(Project {
        id,
        name: row.get(1)?,
        schema_version: row.get(2)?,
        mevdan_version: row.get(3)?,
        created_at,
        updated_at,
        config,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use std::fs;
    use tempfile::TempDir;

    fn fresh_db() -> (TempDir, Database) {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".mevdan")).unwrap();
        let db = Database::open(dir.path()).unwrap();
        (dir, db)
    }

    #[test]
    fn insert_and_get_first() {
        let (_dir, db) = fresh_db();
        let p = Project::new("demo", "0.1.0");

        insert(db.connection(), &p).unwrap();
        let loaded = get_first(db.connection()).unwrap().unwrap();

        assert_eq!(loaded.id, p.id);
        assert_eq!(loaded.name, "demo");
        assert_eq!(loaded.schema_version, "0.1.0");
    }

    #[test]
    fn get_first_returns_none_when_empty() {
        let (_dir, db) = fresh_db();
        let loaded = get_first(db.connection()).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn get_by_id_returns_none_for_unknown() {
        let (_dir, db) = fresh_db();
        let result = get_by_id(db.connection(), ProjectId::new()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn roundtrip_preserves_config() {
        let (_dir, db) = fresh_db();
        let mut p = Project::new("demo", "0.1.0");
        p.config.default_provider = Some("openai".into());
        p.config.default_model = Some("gpt-4o-mini".into());

        insert(db.connection(), &p).unwrap();
        let loaded = get_first(db.connection()).unwrap().unwrap();

        assert_eq!(loaded.config.default_provider.as_deref(), Some("openai"));
        assert_eq!(loaded.config.default_model.as_deref(), Some("gpt-4o-mini"));
    }

    #[test]
    fn insert_duplicate_id_fails() {
        let (_dir, db) = fresh_db();
        let p = Project::new("demo", "0.1.0");

        insert(db.connection(), &p).unwrap();
        let result = insert(db.connection(), &p);
        assert!(result.is_err());
    }
}
