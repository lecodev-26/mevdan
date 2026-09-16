//! # mevdan-codeintel
//!
//! Inteligencia de código para MEVDAN.
//!
//! ## Estado del proyecto
//!
//! - **V4.6.1** ✅ — `RepoMap`, `LanguageDetector`, `BuildSystemDetector`, `ProjectInfo`.
//! - **V4.6.2** ✅ — `SymbolMap`, `DependencyMap`, `CodeIntelEngine`.
//! - **V4.7** ⏳ — Multi-Agent Engine.
//!
//! ## Ejemplo
//!
//! ```no_run
//! use mevdan_codeintel::CodeIntelEngine;
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let engine = CodeIntelEngine::new();
//! let report = engine.analyze(Path::new("/home/user/project"))?;
//!
//! println!("{}", report.summary());
//! # Ok(())
//! # }
//! ```

pub mod build_system;
pub mod dependency;
pub mod engine;
pub mod error;
pub mod language;
pub mod repo_map;
pub mod symbol;

// Re-exports de conveniencia.
pub use build_system::{BuildSystem, BuildSystemDetector};
pub use dependency::{Dependency, DependencyKind, DependencyMap};
pub use engine::{CodeIntelEngine, CodeIntelReport};
pub use error::{CodeIntelError, CodeIntelResult};
pub use language::{Language, LanguageDetector};
pub use repo_map::{ProjectInfo, RepoMap, RepoNode, DEFAULT_MAX_DEPTH};
pub use symbol::{Symbol, SymbolKind, SymbolMap};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn full_flow_rust_project() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(
            dir.path().join("src/main.rs"),
            r#"
use std::io;

fn main() {}
struct Config {}
"#,
        )
        .unwrap();

        let engine = CodeIntelEngine::new();
        let report = engine.analyze(dir.path()).unwrap();

        assert_eq!(report.repo.info.primary_language(), Some(Language::Rust));
        assert_eq!(report.symbols.by_kind(SymbolKind::Function).len(), 1);
        assert_eq!(report.symbols.by_kind(SymbolKind::Struct).len(), 1);
        assert_eq!(report.dependencies.len(), 1);
    }

    #[test]
    fn full_flow_empty_project() {
        let dir = TempDir::new().unwrap();
        let engine = CodeIntelEngine::new();
        let report = engine.analyze(dir.path()).unwrap();

        assert_eq!(report.repo.file_count(), 0);
        assert!(report.symbols.is_empty());
        assert!(report.dependencies.is_empty());
        assert!(report.repo.info.primary_language().is_none());
    }
}
