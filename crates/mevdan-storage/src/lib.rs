//! # mevdan-storage
//!
//! Persistencia local de MEVDAN basada en SQLite.
//!
//! ## Responsabilidades
//!
//! - Abrir/crear la base de datos de un proyecto (`.mevdan/mevdan.db`).
//! - Aplicar migraciones de esquema.
//! - Persistir y leer entidades del dominio (`Project`, `Session`, `Event`).
//!
//! ## Reglas de este crate
//!
//! 1. **Solo I/O local.** Nada de red, ni procesos externos.
//! 2. **`mevdan-core` no depende de este crate.** La relación es unidireccional.
//! 3. **`event_repo` es append-only.** Nunca expone `update` ni `delete`.
//! 4. **Las migraciones son idempotentes.** Correrlas dos veces no rompe nada.

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
        // Este test simula lo que hará `mevdan init demo` en el CLI.
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".mevdan")).unwrap();

        // 1. Abrir DB (crea archivo + migraciones)
        let db = Database::open(dir.path()).unwrap();

        // 2. Insertar proyecto
        let project = Project::new("demo", "0.1.0");
        repo::project_repo::insert(db.connection(), &project).unwrap();

        // 3. Insertar sesión inicial
        let session = mevdan_core::session::Session::new(project.id, Some("initial".to_string()));
        repo::session_repo::insert(db.connection(), &session).unwrap();

        // 4. Emitir evento de creación
        repo::event_repo::append(
            db.connection(),
            project.id,
            Some(session.id),
            mevdan_core::event::EventKind::ProjectCreated,
            serde_json::json!({"name": "demo"}),
        )
        .unwrap();

        // 5. Verificar estado
        let loaded_project = repo::project_repo::get_first(db.connection())
            .unwrap()
            .unwrap();
        assert_eq!(loaded_project.id, project.id);

        let sessions = repo::session_repo::list_by_project(db.connection(), project.id).unwrap();
        assert_eq!(sessions.len(), 1);

        let event_count = repo::event_repo::count_by_project(db.connection(), project.id).unwrap();
        assert_eq!(event_count, 1);
    }
}
