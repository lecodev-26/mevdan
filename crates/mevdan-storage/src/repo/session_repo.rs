//! Repositorio de `Session`.
//!
//! Operaciones mínimas para M1:
//!   - insert: guarda una sesión
//!   - list_by_project: todas las sesiones de un proyecto
//!   - get_by_id: lee una sesión por su ID
//!
//! El estado se serializa en `snake_case` (coherente con el core).

use crate::error::{StorageError, StorageResult};
use chrono::{DateTime, Utc};
use mevdan_core::{
    ids::{ProjectId, SessionId},
    session::{Session, SessionStatus},
};
use rusqlite::{params, Connection, OptionalExtension};

/// Inserta una sesión.
pub fn insert(conn: &Connection, session: &Session) -> StorageResult<()> {
    let status_str = status_to_str(session.status);
    conn.execute(
        "INSERT INTO sessions
            (id, project_id, label, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            session.id.to_string(),
            session.project_id.to_string(),
            session.label,
            status_str,
            session.created_at.to_rfc3339(),
            session.updated_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Lista todas las sesiones de un proyecto, ordenadas por `created_at`.
pub fn list_by_project(conn: &Connection, project_id: ProjectId) -> StorageResult<Vec<Session>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, label, status, created_at, updated_at
         FROM sessions
         WHERE project_id = ?1
         ORDER BY created_at ASC",
    )?;

    let rows = stmt.query_map(params![project_id.to_string()], row_to_session)?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Lee una sesión por su ID.
pub fn get_by_id(conn: &Connection, id: SessionId) -> StorageResult<Option<Session>> {
    conn.query_row(
        "SELECT id, project_id, label, status, created_at, updated_at
         FROM sessions
         WHERE id = ?1",
        params![id.to_string()],
        row_to_session,
    )
    .optional()
    .map_err(StorageError::from)
}

/// Actualiza el estado de una sesión y su `updated_at`.
pub fn update_status(
    conn: &Connection,
    id: SessionId,
    status: SessionStatus,
) -> StorageResult<()> {
    let affected = conn.execute(
        "UPDATE sessions
         SET status = ?1, updated_at = ?2
         WHERE id = ?3",
        params![
            status_to_str(status),
            Utc::now().to_rfc3339(),
            id.to_string(),
        ],
    )?;

    if affected == 0 {
        return Err(StorageError::ProjectNotFound(format!(
            "session {} not found",
            id
        )));
    }
    Ok(())
}

/// Convierte una fila SQLite a `Session`.
fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<Session> {
    let id_str: String = row.get(0)?;
    let id: SessionId = id_str.parse().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })?;

    let project_id_str: String = row.get(1)?;
    let project_id: ProjectId = project_id_str.parse().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
    })?;

    let status_str: String = row.get(3)?;
    let status = str_to_status(&status_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown session status: {}", status_str),
            )),
        )
    })?;

    let created_at = parse_rfc3339(row, 4)?;
    let updated_at = parse_rfc3339(row, 5)?;

    Ok(Session {
        id,
        project_id,
        label: row.get(2)?,
        status,
        created_at,
        updated_at,
    })
}

fn parse_rfc3339(row: &rusqlite::Row<'_>, idx: usize) -> rusqlite::Result<DateTime<Utc>> {
    let s: String = row.get(idx)?;
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                idx,
                rusqlite::types::Type::Text,
                Box::new(e),
            )
        })
}

fn status_to_str(s: SessionStatus) -> &'static str {
    match s {
        SessionStatus::Active => "active",
        SessionStatus::Paused => "paused",
        SessionStatus::Completed => "completed",
        SessionStatus::Failed => "failed",
    }
}

fn str_to_status(s: &str) -> Option<SessionStatus> {
    match s {
        "active" => Some(SessionStatus::Active),
        "paused" => Some(SessionStatus::Paused),
        "completed" => Some(SessionStatus::Completed),
        "failed" => Some(SessionStatus::Failed),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::repo::project_repo;
    use mevdan_core::project::Project;
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

    #[test]
    fn insert_and_get_by_id() {
        let (_dir, db, p) = fresh_db_with_project();
        let s = Session::new(p.id, Some("initial".into()));

        insert(db.connection(), &s).unwrap();
        let loaded = get_by_id(db.connection(), s.id).unwrap().unwrap();

        assert_eq!(loaded.id, s.id);
        assert_eq!(loaded.project_id, p.id);
        assert_eq!(loaded.label.as_deref(), Some("initial"));
        assert_eq!(loaded.status, SessionStatus::Active);
    }

    #[test]
    fn list_by_project_returns_all_sessions() {
        let (_dir, db, p) = fresh_db_with_project();

        for i in 0..3 {
            let s = Session::new(p.id, Some(format!("s{}", i)));
            insert(db.connection(), &s).unwrap();
        }

        let sessions = list_by_project(db.connection(), p.id).unwrap();
        assert_eq!(sessions.len(), 3);
    }

    #[test]
    fn list_by_project_returns_empty_for_unknown_project() {
        let (_dir, db, _p) = fresh_db_with_project();
        let sessions = list_by_project(db.connection(), ProjectId::new()).unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn update_status_changes_status_and_timestamp() {
        let (_dir, db, p) = fresh_db_with_project();
        let s = Session::new(p.id, None);
        insert(db.connection(), &s).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(5));
        update_status(db.connection(), s.id, SessionStatus::Completed).unwrap();

        let loaded = get_by_id(db.connection(), s.id).unwrap().unwrap();
        assert_eq!(loaded.status, SessionStatus::Completed);
        assert!(loaded.updated_at > s.updated_at);
    }

    #[test]
    fn update_status_fails_for_unknown_session() {
        let (_dir, db, _p) = fresh_db_with_project();
        let result = update_status(db.connection(), SessionId::new(), SessionStatus::Completed);
        assert!(result.is_err());
    }
}
