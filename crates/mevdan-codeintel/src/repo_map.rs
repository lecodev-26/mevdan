//! `RepoMap` — estructura del repositorio.
//!
//! Recorre un directorio y produce un árbol con lenguajes detectados.

use crate::{
    build_system::{BuildSystem, BuildSystemDetector},
    error::{CodeIntelError, CodeIntelResult},
    language::{Language, LanguageDetector},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Nodos que se ignoran siempre durante el recorrido.
const IGNORED_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "target",
    "node_modules",
    ".venv",
    "venv",
    "__pycache__",
    ".next",
    ".nuxt",
    "dist",
    "build",
    ".cargo",
    ".rustup",
    ".cache",
    ".gradle",
    ".idea",
    ".vscode",
    ".pytest_cache",
    ".mypy_cache",
];

/// Dotfiles que **NO** se ignoran (whitelist).
const WHITELISTED_DOTFILES: &[&str] = &[
    ".github",
    ".gitignore",
    ".gitattributes",
    ".editorconfig",
    ".env.example",
    ".rustfmt.toml",
    ".clippy.toml",
];

/// Límite por defecto de profundidad.
pub const DEFAULT_MAX_DEPTH: usize = 10;

/// Nodo del árbol del repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RepoNode {
    Dir {
        name: String,
        path: String,
        children: Vec<RepoNode>,
    },
    File {
        name: String,
        path: String,
        language: Language,
        size_bytes: u64,
    },
}

impl RepoNode {
    pub fn name(&self) -> &str {
        match self {
            RepoNode::Dir { name, .. } => name,
            RepoNode::File { name, .. } => name,
        }
    }

    pub fn path(&self) -> &str {
        match self {
            RepoNode::Dir { path, .. } => path,
            RepoNode::File { path, .. } => path,
        }
    }

    pub fn is_dir(&self) -> bool {
        matches!(self, RepoNode::Dir { .. })
    }

    pub fn is_file(&self) -> bool {
        matches!(self, RepoNode::File { .. })
    }

    pub fn children(&self) -> Option<&[RepoNode]> {
        match self {
            RepoNode::Dir { children, .. } => Some(children),
            RepoNode::File { .. } => None,
        }
    }
}

/// Mapa completo del repositorio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoMap {
    pub root: String,
    pub root_node: RepoNode,
    pub info: ProjectInfo,
}

impl RepoMap {
    pub fn build(project_root: &Path) -> CodeIntelResult<Self> {
        Self::build_with_depth(project_root, DEFAULT_MAX_DEPTH)
    }

    pub fn build_with_depth(project_root: &Path, max_depth: usize) -> CodeIntelResult<Self> {
        if !project_root.exists() {
            return Err(CodeIntelError::PathNotFound(
                project_root.display().to_string(),
            ));
        }
        if !project_root.is_dir() {
            return Err(CodeIntelError::NotADirectory(
                project_root.display().to_string(),
            ));
        }

        let canonical = project_root
            .canonicalize()
            .unwrap_or_else(|_| project_root.to_path_buf());

        let detector = LanguageDetector::new();
        let root_node = walk(&canonical, 0, max_depth, &detector)?;

        let info = compute_project_info(&canonical, &root_node);

        Ok(Self {
            root: canonical.display().to_string(),
            root_node,
            info,
        })
    }

    pub fn file_count(&self) -> usize {
        count_files(&self.root_node)
    }

    pub fn dir_count(&self) -> usize {
        count_dirs(&self.root_node).saturating_sub(1)
    }
}

/// Resumen del proyecto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub languages: BTreeMap<Language, usize>,
    pub build_systems: Vec<BuildSystem>,
    pub has_tests: bool,
    pub has_ci: bool,
    pub has_readme: bool,
    pub has_license: bool,
    pub has_git: bool,
}

impl ProjectInfo {
    pub fn primary_language(&self) -> Option<Language> {
        self.languages
            .iter()
            .filter(|(lang, _)| lang.is_programming())
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| *lang)
    }
}

/// ¿Está en la whitelist de dotfiles?
fn is_whitelisted_dotfile(name: &str) -> bool {
    WHITELISTED_DOTFILES.contains(&name)
}

/// Recorre un directorio recursivamente.
fn walk(
    current: &Path,
    depth: usize,
    max_depth: usize,
    detector: &LanguageDetector,
) -> CodeIntelResult<RepoNode> {
    if depth > max_depth {
        return Err(CodeIntelError::MaxDepthExceeded {
            max: max_depth,
            path: current.display().to_string(),
        });
    }

    let name = current
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    let mut children = Vec::new();

    let entries = match fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => {
            return Ok(RepoNode::Dir {
                name,
                path: current.display().to_string(),
                children,
            });
        }
    };

    let mut entry_list: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    entry_list.sort_by_key(|e| e.file_name());

    for entry in entry_list {
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        // Ignoramos cualquier dotfile que no esté en la whitelist.
        if file_name.starts_with('.') && !is_whitelisted_dotfile(&file_name) {
            continue;
        }

        if path.is_dir() {
            if IGNORED_DIRS.contains(&file_name.as_str()) {
                continue;
            }
            match walk(&path, depth + 1, max_depth, detector) {
                Ok(node) => children.push(node),
                Err(CodeIntelError::MaxDepthExceeded { .. }) => {
                    continue;
                }
                Err(e) => return Err(e),
            }
        } else if path.is_file() {
            let language = detector.detect(&path);
            let size_bytes = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

            children.push(RepoNode::File {
                name: file_name,
                path: path.display().to_string(),
                language,
                size_bytes,
            });
        }
    }

    Ok(RepoNode::Dir {
        name,
        path: current.display().to_string(),
        children,
    })
}

fn count_files(node: &RepoNode) -> usize {
    match node {
        RepoNode::File { .. } => 1,
        RepoNode::Dir { children, .. } => children.iter().map(count_files).sum(),
    }
}

fn count_dirs(node: &RepoNode) -> usize {
    match node {
        RepoNode::File { .. } => 0,
        RepoNode::Dir { children, .. } => 1 + children.iter().map(count_dirs).sum::<usize>(),
    }
}

fn compute_project_info(root: &Path, root_node: &RepoNode) -> ProjectInfo {
    let mut languages: BTreeMap<Language, usize> = BTreeMap::new();
    collect_languages(root_node, &mut languages);

    let build_systems = BuildSystemDetector::new().detect_all(root);

    let has_tests = root.join("tests").exists()
        || root.join("test").exists()
        || root.join("__tests__").exists()
        || find_test_file(root_node).is_some();

    let has_ci = root.join(".github").join("workflows").exists()
        || root.join(".gitlab-ci.yml").exists()
        || root.join(".travis.yml").exists()
        || root.join("azure-pipelines.yml").exists();

    let has_readme = root.join("README.md").exists()
        || root.join("README.rst").exists()
        || root.join("README.txt").exists()
        || root.join("README").exists();

    let has_license = root.join("LICENSE").exists()
        || root.join("LICENSE.md").exists()
        || root.join("LICENSE.txt").exists()
        || root.join("COPYING").exists();

    let has_git = root.join(".git").exists();

    ProjectInfo {
        languages,
        build_systems,
        has_tests,
        has_ci,
        has_readme,
        has_license,
        has_git,
    }
}

fn collect_languages(node: &RepoNode, out: &mut BTreeMap<Language, usize>) {
    match node {
        RepoNode::File { language, .. } => {
            if *language != Language::Unknown {
                *out.entry(*language).or_insert(0) += 1;
            }
        }
        RepoNode::Dir { children, .. } => {
            for c in children {
                collect_languages(c, out);
            }
        }
    }
}

fn find_test_file(node: &RepoNode) -> Option<&RepoNode> {
    match node {
        RepoNode::File { name, .. } => {
            let lower = name.to_lowercase();
            if lower.contains("_test.")
                || lower.contains(".test.")
                || lower.contains("_spec.")
                || lower.contains(".spec.")
                || lower.starts_with("test_")
            {
                Some(node)
            } else {
                None
            }
        }
        RepoNode::Dir { children, .. } => children.iter().find_map(find_test_file),
    }
}

#[allow(dead_code)]
pub(crate) fn make_file_node(name: &str, language: Language) -> RepoNode {
    RepoNode::File {
        name: name.to_string(),
        path: name.to_string(),
        language,
        size_bytes: 0,
    }
}

#[allow(dead_code)]
pub(crate) fn make_dir_node(name: &str, children: Vec<RepoNode>) -> RepoNode {
    RepoNode::Dir {
        name: name.to_string(),
        path: name.to_string(),
        children,
    }
}

#[allow(dead_code)]
pub(crate) fn _absolute(p: &str) -> PathBuf {
    PathBuf::from(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_project() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(dir.path().join("README.md"), "# Test").unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("src/lib.rs"), "// lib").unwrap();
        fs::create_dir_all(dir.path().join("tests")).unwrap();
        fs::write(dir.path().join("tests/integration.rs"), "// test").unwrap();
        dir
    }

    #[test]
    fn build_succeeds() {
        let dir = setup_project();
        let map = RepoMap::build(dir.path()).unwrap();
        assert!(map.root.contains(dir.path().to_str().unwrap()));
    }

    #[test]
    fn build_fails_on_missing_path() {
        let err = RepoMap::build(Path::new("/definitely/not/real")).unwrap_err();
        assert!(matches!(err, CodeIntelError::PathNotFound(_)));
    }

    #[test]
    fn build_fails_on_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("file.txt");
        fs::write(&file, "x").unwrap();
        let err = RepoMap::build(&file).unwrap_err();
        assert!(matches!(err, CodeIntelError::NotADirectory(_)));
    }

    #[test]
    fn file_count_correct() {
        let dir = setup_project();
        let map = RepoMap::build(dir.path()).unwrap();
        assert_eq!(map.file_count(), 5);
    }

    #[test]
    fn dir_count_correct() {
        let dir = setup_project();
        let map = RepoMap::build(dir.path()).unwrap();
        assert_eq!(map.dir_count(), 2);
    }

    #[test]
    fn languages_detected() {
        let dir = setup_project();
        let map = RepoMap::build(dir.path()).unwrap();
        let langs = &map.info.languages;
        assert_eq!(langs.get(&Language::Rust), Some(&3));
        assert_eq!(langs.get(&Language::Toml), Some(&1));
        assert_eq!(langs.get(&Language::Markdown), Some(&1));
    }

    #[test]
    fn primary_language_is_rust() {
        let dir = setup_project();
        let map = RepoMap::build(dir.path()).unwrap();
        assert_eq!(map.info.primary_language(), Some(Language::Rust));
    }

    #[test]
    fn build_system_detected() {
        let dir = setup_project();
        let map = RepoMap::build(dir.path()).unwrap();
        assert!(map.info.build_systems.contains(&BuildSystem::Cargo));
    }

    #[test]
    fn has_readme_detected() {
        let dir = setup_project();
        let map = RepoMap::build(dir.path()).unwrap();
        assert!(map.info.has_readme);
    }

    #[test]
    fn has_tests_detected_via_dir() {
        let dir = setup_project();
        let map = RepoMap::build(dir.path()).unwrap();
        assert!(map.info.has_tests);
    }

    #[test]
    fn has_tests_false_when_no_tests() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        let map = RepoMap::build(dir.path()).unwrap();
        assert!(!map.info.has_tests);
    }

    #[test]
    fn has_ci_detected() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".github").join("workflows")).unwrap();
        fs::write(dir.path().join(".github/workflows/ci.yml"), "name: CI").unwrap();
        let map = RepoMap::build(dir.path()).unwrap();
        assert!(map.info.has_ci);
    }

    #[test]
    fn has_license_detected() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("LICENSE"), "MIT").unwrap();
        let map = RepoMap::build(dir.path()).unwrap();
        assert!(map.info.has_license);
    }

    #[test]
    fn ignored_dirs_are_skipped() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::create_dir_all(dir.path().join("target")).unwrap();
        fs::write(dir.path().join("target/big.rs"), "x").unwrap();
        fs::create_dir_all(dir.path().join("node_modules")).unwrap();
        fs::write(dir.path().join("node_modules/x.js"), "x").unwrap();

        let map = RepoMap::build(dir.path()).unwrap();
        assert_eq!(map.file_count(), 1);
    }

    #[test]
    fn ignored_dotfiles() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join(".gitignore"), "target/").unwrap();
        fs::write(dir.path().join(".env"), "SECRET=x").unwrap();
        fs::write(dir.path().join(".my_secret"), "hidden").unwrap();

        let map = RepoMap::build(dir.path()).unwrap();

        // main.rs → cuenta (normal).
        // .gitignore → cuenta (whitelist).
        // .env → se ignora (no whitelist).
        // .my_secret → se ignora (no whitelist).
        assert_eq!(map.file_count(), 2);
    }

    #[test]
    fn max_depth_respected() {
        let dir = TempDir::new().unwrap();
        let mut current = dir.path().to_path_buf();
        for i in 0..20 {
            current = current.join(format!("level{}", i));
            fs::create_dir_all(&current).unwrap();
        }
        fs::write(current.join("deep.rs"), "fn main() {}").unwrap();

        let map = RepoMap::build_with_depth(dir.path(), 5).unwrap();
        assert_eq!(map.file_count(), 0);
    }

    #[test]
    fn repo_map_serializes() {
        let dir = setup_project();
        let map = RepoMap::build(dir.path()).unwrap();
        let json = serde_json::to_string(&map).unwrap();
        let back: RepoMap = serde_json::from_str(&json).unwrap();
        assert_eq!(back.root, map.root);
        assert_eq!(back.file_count(), map.file_count());
    }

    #[test]
    fn repo_node_accessors() {
        let file = make_file_node("test.rs", Language::Rust);
        assert_eq!(file.name(), "test.rs");
        assert!(file.is_file());
        assert!(!file.is_dir());
        assert!(file.children().is_none());

        let dir = make_dir_node("src", vec![]);
        assert_eq!(dir.name(), "src");
        assert!(dir.is_dir());
        assert!(!dir.is_file());
        assert_eq!(dir.children().unwrap().len(), 0);
    }
}
