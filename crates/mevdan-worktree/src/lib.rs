//! # mevdan-worktree
//!
//! Espacios de trabajo aislados dentro de un proyecto.
//!
//! ## Concepto
//!
//! Un **worktree** es una rama de trabajo paralela. Permite probar
//! varios enfoques al mismo tiempo sin que colisionen:
//!
//! ```text
//! mi-proyecto/                    ← main worktree
//! ├── .mevdan/
//! │   ├── worktrees/
//! │   │   ├── approach-a/        ← worktree A
//! │   │   └── approach-b/        ← worktree B
//! │   └── mevdan.db
//! ```
//!
//! Cada worktree tiene su propio Work Graph, sus tareas y sus
//! checkpoints. El `Project` es compartido.
//!
//! ## Estado del proyecto
//!
//! - **V5.1** ✅ — `Worktree`, `WorktreeManager`, `WorktreeId`.
//!
//! ## Ejemplo
//!
//! ```no_run
//! use mevdan_worktree::WorktreeManager;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut manager = WorktreeManager::new("/path/to/project")?;
//!
//! // El worktree "main" siempre existe (apunta a la raíz del proyecto).
//! assert_eq!(manager.main().name, "main");
//!
//! // Creamos una rama paralela de trabajo.
//! let wt = manager.create("experiment", Some("try a new approach".into()))?;
//! println!("Created worktree at {}", wt.path.display());
//!
//! // Más tarde, si no convence, la borramos.
//! manager.delete("experiment")?;
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod id;
pub mod worktree;

// Re-exports de conveniencia.
pub use error::{WorktreeError, WorktreeResult};
pub use id::WorktreeId;
pub use worktree::{validate_name, Worktree, WorktreeManager, MAIN_WORKTREE_NAME};

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn full_flow_isolated_experiment() {
        let dir = TempDir::new().unwrap();
        let mut manager = WorktreeManager::new(dir.path()).unwrap();

        // Estado inicial: solo `main`.
        assert_eq!(manager.len(), 1);

        // Creamos un worktree aislado.
        let wt = manager.create("experiment", Some("test".into())).unwrap();
        assert!(wt.path.exists());
        assert!(wt.path.starts_with(dir.path()));

        // El gestor puede listar, obtener y serializar.
        assert_eq!(manager.list().len(), 2);
        assert!(manager.get("experiment").is_some());

        let json = manager.to_json().unwrap();
        let back = WorktreeManager::from_json(&json).unwrap();
        assert_eq!(back.len(), 2);

        // Limpieza.
        manager.delete("experiment").unwrap();
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn main_worktree_points_to_project_root() {
        let dir = TempDir::new().unwrap();
        let manager = WorktreeManager::new(dir.path()).unwrap();
        let main = manager.main();
        assert_eq!(main.path, dir.path());
        assert!(main.is_main);
        assert_eq!(main.name, MAIN_WORKTREE_NAME);
    }
}
