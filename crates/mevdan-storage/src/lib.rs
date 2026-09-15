//! # mevdan-storage
//!
//! Persistencia local de MEVDAN basada en SQLite.
//!
//! ## Módulos
//!
//! - `db` — conexión y migraciones.
//! - `repo` — repositorios (project, session, event, workgraph, checkpoint).
//! - `audit` — timeline y replay desde el Event Log.

pub mod audit;
pub mod db;
pub mod error;
pub mod repo;

// Re-exports de conveniencia.
pub use audit::{AuditTrail, EventCategory, Replay, TimelineEntry};
pub use db::Database;
pub use error::{StorageError, StorageResult};

#[cfg(test)]
mod tests {
    use super::*;
    use mevdan_core::project::Project;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn full_flow_init_project_session_event() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".mevdan")).unwrap();

        let db = Database::open(dir.path()).unwrap();

        let project = Project::new("demo", "0.1.0");
        repo::project_repo::insert(db.connection(), &project).unwrap();

        let session = mevdan_core::session::Session::new(project.id, Some("initial".to_string()));
        repo::session_repo::insert(db.connection(), &session).unwrap();

        repo::event_repo::append(
            db.connection(),
            project.id,
            Some(session.id),
            mevdan_core::event::EventKind::ProjectCreated,
            serde_json::json!({"name": "demo"}),
        )
        .unwrap();

        let loaded_project = repo::project_repo::get_first(db.connection())
            .unwrap()
            .unwrap();
        assert_eq!(loaded_project.id, project.id);

        let sessions = repo::session_repo::list_by_project(db.connection(), project.id).unwrap();
        assert_eq!(sessions.len(), 1);

        let event_count = repo::event_repo::count_by_project(db.connection(), project.id).unwrap();
        assert_eq!(event_count, 1);
    }

    #[test]
    fn full_flow_with_workgraph() {
        use mevdan_workgraph::{Edge, EdgeKind, Node, WorkGraph};

        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".mevdan")).unwrap();

        let db = Database::open(dir.path()).unwrap();

        let project = Project::new("demo", "0.1.0");
        repo::project_repo::insert(db.connection(), &project).unwrap();

        let mut graph = WorkGraph::new();
        let goal = graph.add_node(Node::goal("build feature")).unwrap();
        let task = graph.add_node(Node::task("implement")).unwrap();
        graph
            .add_edge(Edge::new(goal, task, EdgeKind::Refines))
            .unwrap();

        let wg_id = repo::workgraph_repo::WorkGraphId::new();
        repo::workgraph_repo::insert(
            db.connection(),
            wg_id,
            &project.id.to_string(),
            Some("main"),
            &graph,
        )
        .unwrap();

        let loaded = repo::workgraph_repo::get(db.connection(), wg_id)
            .unwrap()
            .unwrap();
        assert_eq!(loaded.graph.node_count(), 2);
        assert_eq!(loaded.graph.edge_count(), 1);
    }

    #[test]
    fn full_flow_with_checkpoint() {
        use mevdan_checkpoint::{Checkpoint, WorkState};
        use mevdan_task::Task;

        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".mevdan")).unwrap();

        let db = Database::open(dir.path()).unwrap();

        let project = Project::new("demo", "0.1.0");
        repo::project_repo::insert(db.connection(), &project).unwrap();

        let state = WorkState::new()
            .with_task(Task::new("task 1"))
            .with_task(Task::new("task 2"));
        let cp = Checkpoint::new(state).with_label("before-refactor");

        repo::checkpoint_repo::insert(db.connection(), &cp, &project.id.to_string()).unwrap();

        let list = repo::checkpoint_repo::list_by_project(db.connection(), &project.id.to_string())
            .unwrap();
        assert_eq!(list.len(), 1);

        let restored = repo::checkpoint_repo::get_state(db.connection(), cp.id)
            .unwrap()
            .unwrap();
        assert_eq!(restored.tasks.len(), 2);
    }

    #[test]
    fn full_flow_with_audit_trail() {
        use mevdan_core::event::EventKind;

        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".mevdan")).unwrap();

        let db = Database::open(dir.path()).unwrap();

        let project = Project::new("demo", "0.1.0");
        repo::project_repo::insert(db.connection(), &project).unwrap();

        repo::event_repo::append(
            db.connection(),
            project.id,
            None,
            EventKind::ProjectCreated,
            serde_json::json!({"name": "demo", "mevdan_version": "0.2.0"}),
        )
        .unwrap();

        let trail = AuditTrail::build(db.connection(), project.id).unwrap();
        assert_eq!(trail.len(), 1);
        assert!(trail.entries()[0].description.contains("demo"));
    }
}
