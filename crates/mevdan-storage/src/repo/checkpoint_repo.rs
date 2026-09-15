//! Repositorio de checkpoints.
//!
//! Persiste `Checkpoint` como blob JSON.

use crate::error::{StorageError, StorageResult};
use chrono::{DateTime, Utc};
use mevdan_checkpoint::{Checkpoint, CheckpointId, WorkState};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

/// Versión actual del schema de persistencia de checkpoints.
pub const CHECKPOINT_SCHEMA_VERSION: &str = "0.1.0";

/// Metadata de un checkpoint persistido (sin el state).
#[derive(Debug, Clone)]
pub struct CheckpointMeta {
    pub id: CheckpointId,
    pub project_id: String,
    pub label: Option<String>,
    pub schema_version: String,
    pub created_at: DateTime<Utc>,
}

/// Inserta un checkpoint.
pub fn insert(conn: &Connection, checkpoint: &Checkpoint, project_id: &str) -> StorageResult<()> {
    let json = serde_json::to_string(checkpoint)
        .map_err(|e| StorageError::Migration(format!("serialize checkpoint: {}", e)))?;

    conn.execute(
        "INSERT INTO checkpoints
            (id, project_id, label, schema_version, json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            checkpoint.id.to_string(),
            project_id,
            checkpoint.label,
            CHECKPOINT_SCHEMA_VERSION,
            json,
            checkpoint.created_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Carga un checkpoint completo por ID.
pub fn get(conn: &Connection, id: CheckpointId) -> StorageResult<Option<Checkpoint>> {
    let row: Option<String> = conn
        .query_row(
            "SELECT json FROM checkpoints WHERE id = ?1",
            params![id.to_string()],
            |row| row.get(0),
        )
        .optional()?;

    let Some(json) = row else {
        return Ok(None);
    };

    let cp: Checkpoint = serde_json::from_str(&json)
        .map_err(|e| StorageError::Migration(format!("deserialize checkpoint: {}", e)))?;
    Ok(Some(cp))
}

/// Carga solo la metadata (sin json completo).
pub fn get_meta(conn: &Connection, id: CheckpointId) -> StorageResult<Option<CheckpointMeta>> {
    conn.query_row(
        "SELECT id, project_id, label, schema_version, created_at
         FROM checkpoints
         WHERE id = ?1",
        params![id.to_string()],
        row_to_meta,
    )
    .optional()
    .map_err(StorageError::from)
}

/// Lista los checkpoints de un proyecto (más recientes primero).
pub fn list_by_project(conn: &Connection, project_id: &str) -> StorageResult<Vec<CheckpointMeta>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, label, schema_version, created_at
         FROM checkpoints
         WHERE project_id = ?1
         ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(params![project_id], row_to_meta)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Lista los últimos N checkpoints de un proyecto.
pub fn list_recent(
    conn: &Connection,
    project_id: &str,
    limit: usize,
) -> StorageResult<Vec<CheckpointMeta>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, label, schema_version, created_at
         FROM checkpoints
         WHERE project_id = ?1
         ORDER BY created_at DESC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![project_id, limit as i64], row_to_meta)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Borra un checkpoint.
pub fn delete(conn: &Connection, id: CheckpointId) -> StorageResult<bool> {
    let affected = conn.execute(
        "DELETE FROM checkpoints WHERE id = ?1",
        params![id.to_string()],
    )?;
    Ok(affected > 0)
}

/// Cuenta los checkpoints de un proyecto.
pub fn count_by_project(conn: &Connection, project_id: &str) -> StorageResult<u64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM checkpoints WHERE project_id = ?1",
        params![project_id],
        |row| row.get(0),
    )?;
    Ok(count as u64)
}

/// Carga y devuelve solo el `WorkState` de un checkpoint.
pub fn get_state(conn: &Connection, id: CheckpointId) -> StorageResult<Option<WorkState>> {
    Ok(get(conn, id)?.map(|cp| cp.state))
}

fn row_to_meta(row: &rusqlite::Row<'_>) -> rusqlite::Result<CheckpointMeta> {
    let id_str: String = row.get(0)?;
    let uuid = Uuid::parse_str(&id_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })?;

    let created_str: String = row.get(4)?;
    let created_at = DateTime::parse_from_rfc3339(&created_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
        })?;

    Ok(CheckpointMeta {
        id: CheckpointId(uuid),
        project_id: row.get(1)?,
        label: row.get(2)?,
        schema_version: row.get(3)?,
        created_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db::Database, repo::project_repo};
    use mevdan_core::project::Project;
    use mevdan_task::Task;
    use std::fs;
    use tempfile::TempDir;

    fn fresh_db_with_project() -> (TempDir, Database, Project) {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".mevdan")).unwrap();
        let db = Database::open(dir.path()).unwrap();

        let p = Project::new("demo", "0.1.0");
        project_repo::insert(db.connection(), &p).unwrap();

        (dir, db, p)
    }

    fn sample_checkpoint() -> Checkpoint {
        let state = mevdan_checkpoint::WorkState::new()
            .with_task(Task::new("task 1"))
            .with_task(Task::new("task 2"));
        Checkpoint::new(state).with_label("sample")
    }

    #[test]
    fn insert_and_get() {
        let (_dir, db, project) = fresh_db_with_project();
        let cp = sample_checkpoint();
        let cp_id = cp.id;

        insert(db.connection(), &cp, &project.id.to_string()).unwrap();

        let loaded = get(db.connection(), cp_id).unwrap().unwrap();
        assert_eq!(loaded.id, cp_id);
        assert_eq!(loaded.label.as_deref(), Some("sample"));
        assert_eq!(loaded.state.tasks.len(), 2);
    }

    #[test]
    fn get_unknown_returns_none() {
        let (_dir, db, _project) = fresh_db_with_project();
        let loaded = get(db.connection(), CheckpointId::new()).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn get_meta_works() {
        let (_dir, db, project) = fresh_db_with_project();
        let cp = sample_checkpoint();
        let cp_id = cp.id;
        insert(db.connection(), &cp, &project.id.to_string()).unwrap();

        let meta = get_meta(db.connection(), cp_id).unwrap().unwrap();
        assert_eq!(meta.id, cp_id);
        assert_eq!(meta.project_id, project.id.to_string());
        assert_eq!(meta.label.as_deref(), Some("sample"));
    }

    #[test]
    fn list_by_project_returns_recent_first() {
        let (_dir, db, project) = fresh_db_with_project();

        let cp1 = Checkpoint::new(mevdan_checkpoint::WorkState::new()).with_label("first");
        insert(db.connection(), &cp1, &project.id.to_string()).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let cp2 = Checkpoint::new(mevdan_checkpoint::WorkState::new()).with_label("second");
        insert(db.connection(), &cp2, &project.id.to_string()).unwrap();

        let list = list_by_project(db.connection(), &project.id.to_string()).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].label.as_deref(), Some("second"));
        assert_eq!(list[1].label.as_deref(), Some("first"));
    }

    #[test]
    fn list_recent_limits() {
        let (_dir, db, project) = fresh_db_with_project();

        for i in 0..5 {
            let cp = Checkpoint::new(mevdan_checkpoint::WorkState::new())
                .with_label(format!("cp-{}", i));
            insert(db.connection(), &cp, &project.id.to_string()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let recent = list_recent(db.connection(), &project.id.to_string(), 2).unwrap();
        assert_eq!(recent.len(), 2);
    }

    #[test]
    fn delete_removes() {
        let (_dir, db, project) = fresh_db_with_project();
        let cp = sample_checkpoint();
        let cp_id = cp.id;
        insert(db.connection(), &cp, &project.id.to_string()).unwrap();

        assert!(delete(db.connection(), cp_id).unwrap());
        assert!(get(db.connection(), cp_id).unwrap().is_none());
    }

    #[test]
    fn delete_unknown_returns_false() {
        let (_dir, db, _project) = fresh_db_with_project();
        assert!(!delete(db.connection(), CheckpointId::new()).unwrap());
    }

    #[test]
    fn count_by_project_works() {
        let (_dir, db, project) = fresh_db_with_project();
        assert_eq!(
            count_by_project(db.connection(), &project.id.to_string()).unwrap(),
            0
        );

        let cp = sample_checkpoint();
        insert(db.connection(), &cp, &project.id.to_string()).unwrap();

        assert_eq!(
            count_by_project(db.connection(), &project.id.to_string()).unwrap(),
            1
        );
    }

    #[test]
    fn get_state_returns_only_state() {
        let (_dir, db, project) = fresh_db_with_project();
        let cp = sample_checkpoint();
        let cp_id = cp.id;
        insert(db.connection(), &cp, &project.id.to_string()).unwrap();

        let state = get_state(db.connection(), cp_id).unwrap().unwrap();
        assert_eq!(state.tasks.len(), 2);
    }

    #[test]
    fn full_roundtrip_preserves_complex_state() {
        let (_dir, db, project) = fresh_db_with_project();

        // Crea un estado más complejo.
        let state = mevdan_checkpoint::WorkState::new()
            .with_task(Task::new("build"))
            .with_artifact(mevdan_verification::Artifact::from_text("out.txt", "hello"))
            .with_claim(mevdan_verification::Claim::file_created("out.txt"))
            .with_metadata(serde_json::json!({"commit": "abc123"}));

        let cp = Checkpoint::new(state).with_label("complex");
        let cp_id = cp.id;
        insert(db.connection(), &cp, &project.id.to_string()).unwrap();

        let loaded = get(db.connection(), cp_id).unwrap().unwrap();
        assert_eq!(loaded.state.tasks.len(), 1);
        assert_eq!(loaded.state.artifacts.len(), 1);
        assert_eq!(loaded.state.claims.len(), 1);
        assert_eq!(loaded.state.metadata["commit"], "abc123");
    }
}
