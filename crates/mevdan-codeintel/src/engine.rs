//! `CodeIntelEngine` — fachada que combina RepoMap + SymbolMap +
//! DependencyMap.

use crate::{
    dependency::DependencyMap,
    error::CodeIntelResult,
    language::Language,
    repo_map::{RepoMap, RepoNode},
    symbol::SymbolMap,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Resultado de un análisis de codeintel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeIntelReport {
    pub repo: RepoMap,
    pub symbols: SymbolMap,
    pub dependencies: DependencyMap,
}

impl CodeIntelReport {
    /// Resumen textual.
    pub fn summary(&self) -> String {
        format!(
            "Repo: {} files, {} dirs | {}\n{}\nBuild systems: {}",
            self.repo.file_count(),
            self.repo.dir_count(),
            self.symbols.summary(),
            self.dependencies.summary(),
            if self.repo.info.build_systems.is_empty() {
                "none".to_string()
            } else {
                self.repo
                    .info
                    .build_systems
                    .iter()
                    .map(|b| b.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        )
    }
}

/// Motor de Code Intelligence.
#[derive(Debug, Default)]
pub struct CodeIntelEngine;

impl CodeIntelEngine {
    pub fn new() -> Self {
        Self
    }

    /// Analiza un proyecto completo.
    pub fn analyze(&self, project_root: &Path) -> CodeIntelResult<CodeIntelReport> {
        let repo = RepoMap::build(project_root)?;

        let mut symbols = SymbolMap::new();
        let mut dependencies = DependencyMap::new();

        collect_symbols_and_deps(&repo.root_node, &mut symbols, &mut dependencies);

        Ok(CodeIntelReport {
            repo,
            symbols,
            dependencies,
        })
    }
}

/// Recorre el árbol del repo extrayendo símbolos y deps.
fn collect_symbols_and_deps(
    node: &RepoNode,
    symbols: &mut SymbolMap,
    dependencies: &mut DependencyMap,
) {
    match node {
        RepoNode::File { path, language, .. } => {
            if *language == Language::Unknown {
                return;
            }
            let p = Path::new(path);
            let sym = SymbolMap::from_file(p, *language);
            let deps = DependencyMap::from_file(p, *language);
            symbols.extend(sym);
            dependencies.extend(deps);
        }
        RepoNode::Dir { children, .. } => {
            for c in children {
                collect_symbols_and_deps(c, symbols, dependencies);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_rust_project() -> TempDir {
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
use crate::utils;

fn main() {
    println!("hi");
}

struct Config {
    name: String,
}
"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("src/utils.rs"),
            r#"
pub fn helper() {}

pub fn another() {}
"#,
        )
        .unwrap();
        dir
    }

    #[test]
    fn analyze_rust_project() {
        let dir = setup_rust_project();
        let engine = CodeIntelEngine::new();
        let report = engine.analyze(dir.path()).unwrap();

        // RepoMap
        assert!(report.repo.file_count() >= 3); // Cargo.toml, main.rs, utils.rs
        assert_eq!(report.repo.info.primary_language(), Some(Language::Rust));

        // Symbols
        let fns = report.symbols.by_kind(crate::symbol::SymbolKind::Function);
        assert!(fns.len() >= 3); // main, helper, another
        let structs = report.symbols.by_kind(crate::symbol::SymbolKind::Struct);
        assert_eq!(structs.len(), 1); // Config

        // Dependencies
        assert_eq!(report.dependencies.len(), 2); // std::io, crate::utils
        assert_eq!(report.dependencies.internal().len(), 1);
        assert_eq!(report.dependencies.external().len(), 1);
    }

    #[test]
    fn analyze_empty_project() {
        let dir = TempDir::new().unwrap();
        let engine = CodeIntelEngine::new();
        let report = engine.analyze(dir.path()).unwrap();

        assert_eq!(report.repo.file_count(), 0);
        assert!(report.symbols.is_empty());
        assert!(report.dependencies.is_empty());
    }

    #[test]
    fn analyze_python_project() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("main.py"),
            r#"
import os
from pathlib import Path

def hello():
    pass

class Point:
    pass
"#,
        )
        .unwrap();
        let engine = CodeIntelEngine::new();
        let report = engine.analyze(dir.path()).unwrap();

        assert_eq!(report.repo.info.primary_language(), Some(Language::Python));
        assert_eq!(
            report
                .symbols
                .by_kind(crate::symbol::SymbolKind::Function)
                .len(),
            1
        );
        assert_eq!(
            report
                .symbols
                .by_kind(crate::symbol::SymbolKind::Class)
                .len(),
            1
        );
        assert_eq!(report.dependencies.len(), 2);
    }

    #[test]
    fn report_summary_contains_key_info() {
        let dir = setup_rust_project();
        let engine = CodeIntelEngine::new();
        let report = engine.analyze(dir.path()).unwrap();

        let summary = report.summary();
        assert!(summary.contains("Repo:"));
        assert!(summary.contains("symbols"));
        assert!(summary.contains("dependencies"));
        assert!(summary.contains("cargo"));
    }

    #[test]
    fn report_serializes() {
        let dir = setup_rust_project();
        let engine = CodeIntelEngine::new();
        let report = engine.analyze(dir.path()).unwrap();

        let json = serde_json::to_string(&report).unwrap();
        let back: CodeIntelReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.repo.file_count(), report.repo.file_count());
        assert_eq!(back.symbols.len(), report.symbols.len());
    }

    #[test]
    fn analyze_mixed_language_project() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.rs"), "fn rust_fn() {}").unwrap();
        fs::write(dir.path().join("main.py"), "def py_fn():\n    pass").unwrap();
        fs::write(dir.path().join("main.go"), "func go_fn() {}").unwrap();

        let engine = CodeIntelEngine::new();
        let report = engine.analyze(dir.path()).unwrap();

        let fns = report.symbols.by_kind(crate::symbol::SymbolKind::Function);
        assert_eq!(fns.len(), 3);

        let names: Vec<&str> = fns.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"rust_fn"));
        assert!(names.contains(&"py_fn"));
        assert!(names.contains(&"go_fn"));
    }
}
