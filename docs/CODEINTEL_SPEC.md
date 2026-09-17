# MEVDAN — Code Intelligence Specification

**Code Intelligence** is what lets MEVDAN understand the structure of
a project: which languages it uses, what it builds with, what symbols
exist, and who depends on whom.

The `mevdan-codeintel` crate provides these capabilities without
requiring a language server. When LSP is available (V5+), it enriches
the analysis.

## Pieces

- **Language detection** — pick a `Language` from a file extension.
- **Build system detection** — pick a `BuildSystem` from project files.
- **Repo map** — tree of files + languages + project markers.
- **Symbol map** — named entities extracted via regex.
- **Dependency map** — imports and includes extracted via regex.
- **Engine** — combines everything into a `CodeIntelReport`.

## Language detection

```rust
use mevdan_codeintel::LanguageDetector;
use std::path::Path;

let det = LanguageDetector::new();
assert_eq!(det.detect(Path::new("main.rs")), Language::Rust);
assert_eq!(det.detect(Path::new("app.ts")), Language::TypeScript);
assert_eq!(det.detect(Path::new("README.md")), Language::Markdown);
assert_eq!(det.detect(Path::new("weird.xyz")), Language::Unknown);
Supported languages:

Family	Extensions
Rust	rs
Python	py, pyi, pyw
JavaScript	js, mjs, cjs, jsx
TypeScript	ts, mts, cts, tsx
Go	go
Java	java
Kotlin	kt, kts
C	c, h
C++	cpp, cc, cxx, hpp, hh, hxx
C#	cs
Ruby	rb
PHP	php
Swift	swift
Shell	sh, bash, zsh, fish
HTML	html, htm
CSS	css, scss, sass, less
SQL	sql
YAML	yaml, yml
TOML	toml
JSON	json
Markdown	md, markdown
XML	xml
Classification helpers:

Language::is_programming() — Rust, Python, Go, ... (not config, not docs).

Language::is_config() — YAML, TOML, JSON, XML.

Language::is_documentation() — Markdown.

Build system detection
rust
use mevdan_codeintel::BuildSystemDetector;

let det = BuildSystemDetector::new();
let system = det.detect(project_root);
let all = det.detect_all(project_root);  // for monorepos
Detected build systems and the files they match:

System	Marker
Cargo	Cargo.toml
Npm	package.json (without lockfiles)
Pnpm	pnpm-lock.yaml
Yarn	yarn.lock
Bun	bun.lock / bun.lockb
Pip	requirements.txt, setup.py
Poetry	poetry.lock or pyproject.toml
Uv	uv.lock
GoModules	go.mod
Maven	pom.xml
Gradle	build.gradle / build.gradle.kts
Make	Makefile
Cmake	CMakeLists.txt
Ninja	build.ninja
Each system knows its typical commands:

rust
assert_eq!(BuildSystem::Cargo.test_command(),  Some("cargo test"));
assert_eq!(BuildSystem::Cargo.build_command(), Some("cargo build"));
assert_eq!(BuildSystem::GoModules.test_command(), Some("go test ./..."));
RepoMap
RepoMap walks a project directory and produces a tree of RepoNodes
(Dir or File), plus a ProjectInfo summary.

rust
use mevdan_codeintel::RepoMap;
use std::path::Path;

let map = RepoMap::build(Path::new("/path/to/project"))?;
println!("{} files, {} dirs", map.file_count(), map.dir_count());
println!("Primary language: {:?}", map.info.primary_language());
Ignored directories:

text
.git, .hg, .svn, target, node_modules, .venv, venv,
__pycache__, .next, .nuxt, dist, build, .cargo, .rustup,
.cache, .gradle, .idea, .vscode, .pytest_cache, .mypy_cache
Ignored dotfiles (except a small whitelist):

text
.github, .gitignore, .gitattributes, .editorconfig,
.env.example, .rustfmt.toml, .clippy.toml
Maximum depth defaults to DEFAULT_MAX_DEPTH = 10. It can be changed
with RepoMap::build_with_depth.

ProjectInfo
rust
pub struct ProjectInfo {
    pub languages: BTreeMap<Language, usize>,
    pub build_systems: Vec<BuildSystem>,
    pub has_tests: bool,
    pub has_ci: bool,
    pub has_readme: bool,
    pub has_license: bool,
    pub has_git: bool,
}
has_tests is a heuristic: a tests/, test/ or __tests__/
directory, or files matching common test patterns.

primary_language() returns the language with the most files
(considering only programming languages).

SymbolMap
Symbols are named entities: functions, structs, classes, modules, etc.
MEVDAN extracts them via regex, per language.

rust
use mevdan_codeintel::{Language, SymbolMap, SymbolKind};

let source = "fn main() {}\nstruct Config {}\n";
let map = SymbolMap::from_source(source, Language::Rust, "main.rs");

assert_eq!(map.by_kind(SymbolKind::Function).len(), 1);
assert_eq!(map.by_kind(SymbolKind::Struct).len(), 1);
Supported symbol kinds:

Function, Method, Struct, Class, Enum, Trait, Interface,
Module, Constant, Static, Variable, TypeAlias, Macro,
Unknown.

Per-language patterns (extracted from the source, group 1 is the name):

Rust: fn, struct, enum, trait, mod, type, const,
static, macro_rules!.

Python: def, class.

JavaScript / TypeScript: function, class, interface,
type, const.

Go: func (including methods), type X struct,
type X interface, type X ....

DependencyMap
Dependencies are imports or includes.

rust
use mevdan_codeintel::{DependencyMap, Language};

let source = "use std::io;\nuse crate::foo;\n";
let map = DependencyMap::from_source(source, Language::Rust);

assert_eq!(map.len(), 2);
assert_eq!(map.internal().len(), 1);  // crate::foo
assert_eq!(map.external().len(), 1);  // std::io
Dependency kinds:

Import — use, import, from ... import.

Include — #include (C/C++).

ReExport — pub use (Rust).

Unknown.

Internal vs external:

Internal: starts with ., /, crate::, super::, self::.

External: everything else.

Per-language patterns:

Rust: use ..., pub use ....

Python: import ..., from ... import.

JavaScript / TypeScript: import ... from "...",
require("...").

Go: import "...".

C / C++: #include <...> or #include "...".

CodeIntelEngine
The engine walks a repo and combines RepoMap + SymbolMap +
DependencyMap into a single report.

rust
use mevdan_codeintel::CodeIntelEngine;
use std::path::Path;

let engine = CodeIntelEngine::new();
let report = engine.analyze(Path::new("/path/to/project"))?;

println!("{}", report.summary());
The report contains:

repo — the full RepoMap.

symbols — aggregated SymbolMap across all programming files.

dependencies — aggregated DependencyMap.

What Code Intelligence is NOT
No AST parsing. Regex only.

No type resolution. We do not know what Foo refers to.

No cross-file symbol linking. Each file's symbols are extracted
independently.

No LSP dependency. The analysis works with no language server
running. LSP integration is added in V5.

Regex is a deliberate compromise for V4: fast, no external processes,
portable. In V5, LSP-based analysis will enrich (not replace) this.
