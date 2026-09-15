//! Repositorio de Work Graphs.
//!
//! Guarda y carga `WorkGraph` como un blob JSON dentro de SQLite.
//!
//! ## Modelo
//!
//! Cada fila de `workgraphs` es un Work Graph completo:
//! - `id`: identificador del Work Graph (no el del proyecto).
//! - `project_id`: a qué proyecto pertenece.
//! - `label`: nombre opcional.
//! - `schema_version`: versión del schema del Work Graph.
//! - `json`: serialización JSON del `WorkGraph`.
//! - `created_at`, `updated_at`: timestamps.

use crate::error::{StorageError, StorageResult};
use chrono::{DateTime, Utc};
use mevdan_workgraph::WorkGraph;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

/// Versión actual del schema de persistencia de Work Graphs.
pub const WORKGRAPH_SCHEMA_VERSION: &str = "0.1.0";

/// ID de un Work Graph persistido.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkGraphId(pub Uuid);

impl WorkGraphId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for WorkGraphId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WorkGraphId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Metadata de un Work Graph persistido.
#[derive(Debug, Clone)]
pub struct WorkGraphRecord {
    pub id: WorkGraphId,
    pub project_id: String,
    pub label: Option<String>,
    pub schema_version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub graph: WorkGraph,
}

/// Inserta un Work Graph nuevo.
pub fn insert(
    conn: &Connection,
    id: WorkGraphId,
    project_id: &str,
    label: Option<&str>,
    graph: &WorkGraph,
) -> StorageResult<()> {
    let now = Utc::now();
    let json = serde_json::to_string(graph)?;
    conn.execute(
        "INSERT INTO workgraphs
            (id, project_id, label, schema_version, json, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id.to_string(),
            project_id,
            label,
            WORKGRAPH_SCHEMA_VERSION,
            json,
            now.to_rfc3339(),
            now.to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Reemplaza el contenido de un Work Graph existente.
pub fn update(conn: &Connection, id: WorkGraphId, graph: &WorkGraph) -> StorageResult<()> {
    let now = Utc::now();
    let json = serde_json::to_string(graph)?;
    let affected = conn.execute(
        "UPDATE workgraphs
         SET json = ?1, updated_at = ?2
         WHERE id = ?3",
        params![json, now.to_rfc3339(), id.to_string()],
    )?;
    if affected == 0 {
        return Err(StorageError::ProjectNotFound(format!(
            "workgraph {} not found",
            id
        )));
    }
    Ok(())
}

/// Carga un Work Graph por ID.
pub fn get(conn: &Connection, id: WorkGraphId) -> StorageResult<Option<WorkGraphRecord>> {
    conn.query_row(
        "SELECT id, project_id, label, schema_version, json, created_at, updated_at
         FROM workgraphs
         WHERE id = ?1",
        params![id.to_string()],
        row_to_record,
    )
    .optional()
    .map_err(StorageError::from)
}

/// Lista los Work Graphs de un proyecto.
pub fn list_by_project(conn: &Connection, project_id: &str) -> StorageResult<Vec<WorkGraphRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, label, schema_version, json, created_at, updated_at
         FROM workgraphs
         WHERE project_id = ?1
         ORDER BY updated_at DESC",
    )?;
    let rows = stmt.query_map(params![project_id], row_to_record)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Borra un Work Graph por ID. Devuelve `true` si existía.
pub fn delete(conn: &Connection, id: WorkGraphId) -> StorageResult<bool> {
    let affected = conn.execute(
        "DELETE FROM workgraphs WHERE id = ?1",
        params![id.to_string()],
    )?;
    Ok(affected > 0)
}

/// Cuenta los Work Graphs de un proyecto.
pub fn count_by_project(conn: &Connection, project_id: &str) -> StorageResult<u64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM workgraphs WHERE project_id = ?1",
        params![project_id],
        |row| row.get(0),
    )?;
    Ok(count as u64)
}

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkGraphRecord> {
    let id_str: String = row.get(0)?;
    let uuid = Uuid::parse_str(&id_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let id = WorkGraphId(uuid);

    let created_str: String = row.get(5)?;
    let created_at = DateTime::parse_from_rfc3339(&created_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e))
        })?;

    let updated_str: String = row.get(6)?;
    let updated_at = DateTime::parse_from_rfc3339(&updated_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(e))
        })?;

    let json_str: String = row.get(4)?;
    let graph: WorkGraph = serde_json::from_str(&json_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
    })?;

    Ok(WorkGraphRecord {
        id,
        project_id: row.get(1)?,
        label: row.get(2)?,
        schema_version: row.get(3)?,
        created_at,
        updated_at,
        graph,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db::Database, repo::project_repo};
    use mevdan_core::project::Project;
    use mevdan_workgraph::{Edge, EdgeKind, Node};
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

    fn sample_graph() -> WorkGraph {
        let mut g = WorkGraph::new();
        let goal = g.add_node(Node::goal("do something")).unwrap();
        let task = g.add_node(Node::task("subtask")).unwrap();
        g.add_edge(Edge::new(goal, task, EdgeKind::Refines))
            .unwrap();
        g
    }

    #[test]
    fn insert_and_get() {
        let (_dir, db, project) = fresh_db_with_project();
        let id = WorkGraphId::new();
        let graph = sample_graph();

        insert(
            db.connection(),
            id,
            &project.id.to_string(),
            Some("main"),
            &graph,
        )
        .unwrap();

        let loaded = get(db.connection(), id).unwrap().unwrap();
        assert_eq!(loaded.id, id);
        assert_eq!(loaded.project_id, project.id.to_string());
        assert_eq!(loaded.label.as_deref(), Some("main"));
        assert_eq!(loaded.schema_version, WORKGRAPH_SCHEMA_VERSION);
        assert_eq!(loaded.graph.node_count(), 2);
        assert_eq!(loaded.graph.edge_count(), 1);
    }

    #[test]
    fn insert_without_label() {
        let (_dir, db, project) = fresh_db_with_project();
        let id = WorkGraphId::new();
        let graph = sample_graph();

        insert(db.connection(), id, &project.id.to_string(), None, &graph).unwrap();

        let loaded = get(db.connection(), id).unwrap().unwrap();
        assert!(loaded.label.is_none());
    }

    #[test]
    fn get_unknown_returns_none() {
        let (_dir, db, _project) = fresh_db_with_project();
        let loaded = get(db.connection(), WorkGraphId::new()).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn update_modifies_json_and_updated_at() {
        let (_dir, db, project) = fresh_db_with_project();
        let id = WorkGraphId::new();
        let mut graph = sample_graph();

        insert(db.connection(), id, &project.id.to_string(), None, &graph).unwrap();
        let original = get(db.connection(), id).unwrap().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(5));

        // Modifica el grafo.
        graph.add_node(Node::artifact("output.txt")).unwrap();
        update(db.connection(), id, &graph).unwrap();

        let updated = get(db.connection(), id).unwrap().unwrap();
        assert_eq!(updated.graph.node_count(), 3);
        assert!(updated.updated_at > original.updated_at);
        assert_eq!(updated.created_at, original.created_at);
    }

    #[test]
    fn update_unknown_fails() {
        let (_dir, db, _project) = fresh_db_with_project();
        let id = WorkGraphId::new();
        let graph = sample_graph();
        let err = update(db.connection(), id, &graph).unwrap_err();
        assert!(matches!(err, StorageError::ProjectNotFound(_)));
    }

    #[test]
    fn list_by_project_returns_all() {
        let (_dir, db, project) = fresh_db_with_project();
        for i in 0..3 {
            let id = WorkGraphId::new();
            let graph = sample_graph();
            insert(
                db.connection(),
                id,
                &project.id.to_string(),
                Some(&format!("graph-{}", i)),
                &graph,
            )
            .unwrap();
        }

        let list = list_by_project(db.connection(), &project.id.to_string()).unwrap();
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn list_by_project_empty_for_unknown() {
        let (_dir, db, _project) = fresh_db_with_project();
        let list = list_by_project(db.connection(), "unknown-project").unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn delete_removes_workgraph() {
        let (_dir, db, project) = fresh_db_with_project();
        let id = WorkGraphId::new();
        let graph = sample_graph();
        insert(db.connection(), id, &project.id.to_string(), None, &graph).unwrap();

        assert!(delete(db.connection(), id).unwrap());
        assert!(get(db.connection(), id).unwrap().is_none());
    }

    #[test]
    fn delete_unknown_returns_false() {
        let (_dir, db, _project) = fresh_db_with_project();
        assert!(!delete(db.connection(), WorkGraphId::new()).unwrap());
    }

    #[test]
    fn count_by_project_works() {
        let (_dir, db, project) = fresh_db_with_project();
        assert_eq!(
            count_by_project(db.connection(), &project.id.to_string()).unwrap(),
            0
        );

        let id = WorkGraphId::new();
        let graph = sample_graph();
        insert(db.connection(), id, &project.id.to_string(), None, &graph).unwrap();

        assert_eq!(
            count_by_project(db.connection(), &project.id.to_string()).unwrap(),
            1
        );
    }

    #[test]
    fn roundtrip_preserves_full_graph() {
        let (_dir, db, project) = fresh_db_with_project();

        // Grafo más complejo.
        let mut graph = WorkGraph::new();
        let goal = graph.add_node(Node::goal("build")).unwrap();
        let req = graph
            .add_node(
                Node::requirement("must work").with_data(serde_json::json!({"priority": "high"})),
            )
            .unwrap();
        let task = graph.add_node(Node::task("implement")).unwrap();
        let art = graph.add_node(Node::artifact("src/main.rs")).unwrap();

        graph
            .add_edge(Edge::new(goal, req, EdgeKind::Refines))
            .unwrap();
        graph
            .add_edge(Edge::new(req, task, EdgeKind::Refines))
            .unwrap();
        graph
            .add_edge(Edge::new(task, art, EdgeKind::Produces))
            .unwrap();

        let id = WorkGraphId::new();
        insert(db.connection(), id, &project.id.to_string(), None, &graph).unwrap();

        let loaded = get(db.connection(), id).unwrap().unwrap();
        assert_eq!(loaded.graph.node_count(), 4);
        assert_eq!(loaded.graph.edge_count(), 3);
        assert_eq!(loaded.graph.stats().nodes_by_kind.len(), 4);

        // Verifica el payload JSON de un nodo con data.
        let nodes = loaded
            .graph
            .nodes_by_kind(mevdan_workgraph::NodeKind::Requirement);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].data["priority"], "high");
    }

    #[test]
    fn workgraph_id_is_unique() {
        let a = WorkGraphId::new();
        let b = WorkGraphId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn workgraph_id_displays() {
        let id = WorkGraphId::new();
        assert!(id.to_string().contains('-'));
    }
}
