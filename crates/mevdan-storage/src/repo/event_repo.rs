//! Repositorio de `Event`.
//!
//! Regla dura (Regla 15 del prompt fundacional):
//!   Este módulo NO expone `update` ni `delete`.
//!   Los eventos son inmutables y append-only.
//!
//! Si en el futuro alguien necesita modificar el estado derivado de un
//! evento, debe emitir un EVENTO NUEVO que represente el cambio. El log
//! es la fuente de verdad, no un estado mutable.

use crate::error::{StorageError, StorageResult};
use chrono::{DateTime, Utc};
use mevdan_core::{
    event::{Event, EventKind},
    ids::{EventId, ProjectId, SessionId},
};
use rusqlite::{params, Connection};

/// Añade un evento al log. Devuelve el ID del evento creado.
pub fn append(
    conn: &Connection,
    project_id: ProjectId,
    session_id: Option<SessionId>,
    kind: EventKind,
    payload: serde_json::Value,
) -> StorageResult<EventId> {
    let event = Event::new(project_id, session_id, kind, payload);

    let kind_str = serde_json::to_string(&event.kind)?
        .trim_matches('"')
        .to_string();
    let payload_str = serde_json::to_string(&event.payload)?;

    conn.execute(
        "INSERT INTO events
            (id, project_id, session_id, occurred_at, kind, payload_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            event.id.to_string(),
            event.project_id.to_string(),
            event.session_id.map(|s| s.to_string()),
            event.occurred_at.to_rfc3339(),
            kind_str,
            payload_str,
        ],
    )?;

    Ok(event.id)
}

/// Lee un evento por su ID. Útil para auditoría.
pub fn get_by_id(conn: &Connection, id: EventId) -> StorageResult<Option<Event>> {
    use rusqlite::OptionalExtension;
    conn.query_row(
        "SELECT id, project_id, session_id, occurred_at, kind, payload_json
         FROM events
         WHERE id = ?1",
        params![id.to_string()],
        row_to_event,
    )
    .optional()
    .map_err(StorageError::from)
}

/// Lista todos los eventos de un proyecto, ordenados por `occurred_at`.
///
/// Nota: en proyectos grandes esto puede ser lento. En M4 añadiremos
/// paginación. Para M1 con pocos eventos, está bien.
pub fn list_by_project(conn: &Connection, project_id: ProjectId) -> StorageResult<Vec<Event>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, session_id, occurred_at, kind, payload_json
         FROM events
         WHERE project_id = ?1
         ORDER BY occurred_at ASC",
    )?;
    let rows = stmt.query_map(params![project_id.to_string()], row_to_event)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Cuenta cuántos eventos tiene un proyecto.
pub fn count_by_project(conn: &Connection, project_id: ProjectId) -> StorageResult<u64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM events WHERE project_id = ?1",
        params![project_id.to_string()],
        |row| row.get(0),
    )?;
    Ok(count as u64)
}

/// Convierte una fila SQLite a `Event`.
fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<Event> {
    let id_str: String = row.get(0)?;
    let id: EventId = id_str.parse().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })?;

    let project_id_str: String = row.get(1)?;
    let project_id: ProjectId = project_id_str.parse().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
    })?;

    let session_id: Option<SessionId> = match row.get::<_, Option<String>>(2)? {
        Some(s) => Some(s.parse().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
        })?),
        None => None,
    };

    let occurred_str: String = row.get(3)?;
    let occurred_at = DateTime::parse_from_rfc3339(&occurred_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e))
        })?;

    let kind_str: String = row.get(4)?;
    let kind: EventKind = serde_json::from_str(&format!("\"{}\"", kind_str)).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
    })?;

    let payload_str: String = row.get(5)?;
    let payload: serde_json::Value = serde_json::from_str(&payload_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e))
    })?;

    Ok(Event {
        id,
        project_id,
        session_id,
        occurred_at,
        kind,
        payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::repo::{project_repo, session_repo};
    use mevdan_core::{project::Project, session::Session};
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
    fn append_and_read_back() {
        let (_dir, db, p) = fresh_db_with_project();
        let payload = serde_json::json!({"name": "demo", "mevdan_version": "0.1.0"});

        let event_id = append(
            db.connection(),
            p.id,
            None,
            EventKind::ProjectCreated,
            payload.clone(),
        )
        .unwrap();

        let loaded = get_by_id(db.connection(), event_id).unwrap().unwrap();
        assert_eq!(loaded.id, event_id);
        assert_eq!(loaded.project_id, p.id);
        assert_eq!(loaded.session_id, None);
        assert_eq!(loaded.kind, EventKind::ProjectCreated);
        assert_eq!(loaded.payload, payload);
    }

    #[test]
    fn append_with_session_id() {
        let (_dir, db, p) = fresh_db_with_project();
        let s = Session::new(p.id, Some("test".into()));
        session_repo::insert(db.connection(), &s).unwrap();

        let event_id = append(
            db.connection(),
            p.id,
            Some(s.id),
            EventKind::SessionStarted,
            serde_json::json!({}),
        )
        .unwrap();

        let loaded = get_by_id(db.connection(), event_id).unwrap().unwrap();
        assert_eq!(loaded.session_id, Some(s.id));
    }

    #[test]
    fn list_by_project_returns_events_in_order() {
        let (_dir, db, p) = fresh_db_with_project();

        for _ in 0..3 {
            append(
                db.connection(),
                p.id,
                None,
                EventKind::ProjectCreated,
                serde_json::json!({}),
            )
            .unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        let events = list_by_project(db.connection(), p.id).unwrap();
        assert_eq!(events.len(), 3);

        // Verifica orden por occurred_at
        for i in 1..events.len() {
            assert!(events[i].occurred_at >= events[i - 1].occurred_at);
        }
    }

    #[test]
    fn count_by_project_works() {
        let (_dir, db, p) = fresh_db_with_project();
        assert_eq!(count_by_project(db.connection(), p.id).unwrap(), 0);

        append(
            db.connection(),
            p.id,
            None,
            EventKind::ProjectCreated,
            serde_json::json!({}),
        )
        .unwrap();

        assert_eq!(count_by_project(db.connection(), p.id).unwrap(), 1);
    }

    #[test]
    fn get_by_id_returns_none_for_unknown() {
        let (_dir, db, _p) = fresh_db_with_project();
        let result = get_by_id(db.connection(), EventId::new()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn append_duplicate_is_impossible() {
        // Este test es conceptual: cada append genera un EventId nuevo,
        // así que no puede haber colisión.
        let (_dir, db, p) = fresh_db_with_project();

        let a = append(
            db.connection(),
            p.id,
            None,
            EventKind::ProjectCreated,
            serde_json::json!({}),
        )
        .unwrap();
        let b = append(
            db.connection(),
            p.id,
            None,
            EventKind::ProjectCreated,
            serde_json::json!({}),
        )
        .unwrap();

        assert_ne!(a, b);
    }
}
