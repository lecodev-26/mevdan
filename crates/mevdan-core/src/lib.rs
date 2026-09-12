//! # mevdan-core
//!
//! Dominio puro de MEVDAN: tipos, entidades y errores.
//!
//! ## Reglas de este crate
//!
//! 1. **Sin I/O.** Nada de filesystem, red, SQLite, procesos.
//! 2. **Sin async.** Solo tipos y lógica pura.
//! 3. **Sin dependencias de otros crates de MEVDAN.**
//! 4. **Con tests.** Cada archivo lleva sus tests al final.
//!
//! Este crate es la base sobre la que se construyen
//! `mevdan-storage`, `mevdan-provider`, `mevdan-agent`, etc.

// ─── Módulos ─────────────────────────────────────────────────
pub mod error;
pub mod event;
pub mod ids;
pub mod project;
pub mod session;
pub mod version;

// ─── Re-exports de conveniencia ──────────────────────────────
//
// Permiten escribir:
//     use mevdan_core::Project;
// en vez de:
//     use mevdan_core::project::Project;

pub use error::{CoreError, CoreResult};
pub use event::{Event, EventKind};
pub use ids::{EventId, ProjectId, SessionId};
pub use project::{Project, ProjectConfig, PROJECT_SCHEMA_VERSION};
pub use session::{Session, SessionStatus};

/// Versión de esta build de MEVDAN.
///
/// Se lee de `Cargo.toml` en tiempo de compilación.
pub const MEVDAN_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mevdan_version_is_not_empty() {
        assert!(!MEVDAN_VERSION.is_empty());
    }

    #[test]
    fn reexports_are_accessible() {
        // Verifica que los re-exports funcionan.
        let _p = Project::new("demo", MEVDAN_VERSION);
        let _s = Session::new(ProjectId::new(), None);
        let _e = Event::new(
            ProjectId::new(),
            None,
            EventKind::ProjectCreated,
            serde_json::json!({}),
        );
        let _err = CoreError::Internal("test".into());
    }
}
