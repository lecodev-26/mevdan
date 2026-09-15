//! # mevdan-storage
//!
//! Persistencia local de MEVDAN basada en SQLite.

pub mod db;
pub mod error;
pub mod repo;

// Re-exports de conveniencia.
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

        // Crea un Work Graph.
        let mut graph = WorkGraph::new();
        let goal = graph.add_node(Node::goal("build feature")).unwrap();
        let task = graph.add_node(Node::task("implement")).unwrap();
        graph
            .add_edge(Edge::new(goal, task, EdgeKind::Refines))
            .unwrap();

        // Persiste.
        let wg_id = repo::workgraph_repo::WorkGraphId::new();
        repo::workgraph_repo::insert(
            db.connection(),
            wg_id,
            &project.id.to_string(),
            Some("main"),
            &graph,
        )
        .unwrap();

        // Carga y verifica.
        let loaded = repo::workgraph_repo::get(db.connection(), wg_id)
            .unwrap()
            .unwrap();
        assert_eq!(loaded.graph.node_count(), 2);
        assert_eq!(loaded.graph.edge_count(), 1);
    }
}
