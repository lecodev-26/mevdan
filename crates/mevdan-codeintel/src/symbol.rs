//! Extracción de símbolos por parsing básico con regex.

use crate::language::Language;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Tipo de símbolo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Class,
    Enum,
    Trait,
    Interface,
    Module,
    Constant,
    Static,
    Variable,
    TypeAlias,
    Macro,
    Unknown,
}

impl SymbolKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Method => "method",
            SymbolKind::Struct => "struct",
            SymbolKind::Class => "class",
            SymbolKind::Enum => "enum",
            SymbolKind::Trait => "trait",
            SymbolKind::Interface => "interface",
            SymbolKind::Module => "module",
            SymbolKind::Constant => "constant",
            SymbolKind::Static => "static",
            SymbolKind::Variable => "variable",
            SymbolKind::TypeAlias => "type_alias",
            SymbolKind::Macro => "macro",
            SymbolKind::Unknown => "unknown",
        }
    }
}

/// Un símbolo extraído del código.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
    pub path: String,
}

impl Symbol {
    pub fn new(
        name: impl Into<String>,
        kind: SymbolKind,
        line: usize,
        path: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            line,
            path: path.into(),
        }
    }
}

/// Mapa de símbolos de un archivo o proyecto.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SymbolMap {
    pub symbols: Vec<Symbol>,
}

impl SymbolMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_file(path: &Path, language: Language) -> Self {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        Self::from_source(&content, language, &path.display().to_string())
    }

    pub fn from_source(source: &str, language: Language, path: &str) -> Self {
        let patterns = patterns_for(language);
        let mut symbols = Vec::new();

        for (kind, re) in patterns {
            for (line_idx, line) in source.lines().enumerate() {
                if let Some(caps) = re.captures(line) {
                    if let Some(name_match) = caps.get(1) {
                        let name = name_match.as_str().to_string();
                        if name.is_empty() {
                            continue;
                        }
                        symbols.push(Symbol::new(name, kind, line_idx + 1, path));
                    }
                }
            }
        }

        symbols.sort_by(|a, b| {
            a.line
                .cmp(&b.line)
                .then(a.name.cmp(&b.name))
                .then(a.kind.cmp(&b.kind))
        });
        symbols.dedup_by(|a, b| a.line == b.line && a.name == b.name && a.kind == b.kind);

        Self { symbols }
    }

    pub fn extend(&mut self, other: SymbolMap) {
        self.symbols.extend(other.symbols);
    }

    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    pub fn by_kind(&self, kind: SymbolKind) -> Vec<&Symbol> {
        self.symbols.iter().filter(|s| s.kind == kind).collect()
    }

    pub fn find_by_name(&self, name: &str) -> Vec<&Symbol> {
        self.symbols.iter().filter(|s| s.name == name).collect()
    }

    pub fn summary(&self) -> String {
        if self.symbols.is_empty() {
            return "no symbols".to_string();
        }
        let mut counts: std::collections::BTreeMap<SymbolKind, usize> =
            std::collections::BTreeMap::new();
        for s in &self.symbols {
            *counts.entry(s.kind).or_insert(0) += 1;
        }
        let parts: Vec<String> = counts
            .iter()
            .map(|(k, v)| format!("{}: {}", k.display_name(), v))
            .collect();
        format!("{} symbols ({})", self.symbols.len(), parts.join(", "))
    }
}

fn patterns_for(language: Language) -> Vec<(SymbolKind, Regex)> {
    match language {
        Language::Rust => rust_patterns(),
        Language::Python => python_patterns(),
        Language::JavaScript | Language::TypeScript => js_patterns(),
        Language::Go => go_patterns(),
        _ => Vec::new(),
    }
}

fn rust_patterns() -> Vec<(SymbolKind, Regex)> {
    vec![
        (
            SymbolKind::Function,
            Regex::new(r"^\s*(?:pub\s+)?(?:async\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
        ),
        (
            SymbolKind::Struct,
            Regex::new(r"^\s*(?:pub\s+)?struct\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
        ),
        (
            SymbolKind::Enum,
            Regex::new(r"^\s*(?:pub\s+)?enum\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
        ),
        (
            SymbolKind::Trait,
            Regex::new(r"^\s*(?:pub\s+)?trait\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
        ),
        (
            SymbolKind::Module,
            Regex::new(r"^\s*(?:pub\s+)?mod\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
        ),
        (
            SymbolKind::TypeAlias,
            Regex::new(r"^\s*(?:pub\s+)?type\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
        ),
        (
            SymbolKind::Constant,
            Regex::new(r"^\s*(?:pub\s+)?const\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
        ),
        (
            SymbolKind::Static,
            Regex::new(r"^\s*(?:pub\s+)?static\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
        ),
        (
            SymbolKind::Macro,
            Regex::new(r"^\s*macro_rules!\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
        ),
    ]
}

fn python_patterns() -> Vec<(SymbolKind, Regex)> {
    vec![
        (
            SymbolKind::Function,
            Regex::new(r"^\s*(?:async\s+)?def\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
        ),
        (
            SymbolKind::Class,
            Regex::new(r"^\s*class\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
        ),
    ]
}

fn js_patterns() -> Vec<(SymbolKind, Regex)> {
    vec![
        (
            SymbolKind::Function,
            Regex::new(r"^\s*(?:export\s+)?(?:async\s+)?function\s+([a-zA-Z_][a-zA-Z0-9_]*)")
                .unwrap(),
        ),
        (
            SymbolKind::Class,
            Regex::new(r"^\s*(?:export\s+)?class\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
        ),
        (
            SymbolKind::Interface,
            Regex::new(r"^\s*(?:export\s+)?interface\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
        ),
        (
            SymbolKind::TypeAlias,
            Regex::new(r"^\s*(?:export\s+)?type\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
        ),
        (
            SymbolKind::Constant,
            Regex::new(r"^\s*(?:export\s+)?const\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
        ),
    ]
}

fn go_patterns() -> Vec<(SymbolKind, Regex)> {
    vec![
        (
            SymbolKind::Function,
            Regex::new(r"^\s*func\s+(?:\([^)]*\)\s+)?([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
        ),
        (
            SymbolKind::Struct,
            Regex::new(r"^\s*type\s+([a-zA-Z_][a-zA-Z0-9_]*)\s+struct").unwrap(),
        ),
        (
            SymbolKind::Interface,
            Regex::new(r"^\s*type\s+([a-zA-Z_][a-zA-Z0-9_]*)\s+interface").unwrap(),
        ),
        (
            SymbolKind::TypeAlias,
            Regex::new(r"^\s*type\s+([a-zA-Z_][a-zA-Z0-9_]*)\s+").unwrap(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_kind_display_names() {
        assert_eq!(SymbolKind::Function.display_name(), "function");
        assert_eq!(SymbolKind::Struct.display_name(), "struct");
        assert_eq!(SymbolKind::TypeAlias.display_name(), "type_alias");
    }

    #[test]
    fn extract_rust_functions() {
        let source = r#"
fn main() {
    println!("hi");
}

pub fn hello() {}

async fn fetch() {}
"#;
        let map = SymbolMap::from_source(source, Language::Rust, "test.rs");
        let fns = map.by_kind(SymbolKind::Function);
        assert_eq!(fns.len(), 3);
        let names: Vec<&str> = fns.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"main"));
        assert!(names.contains(&"hello"));
        assert!(names.contains(&"fetch"));
    }

    #[test]
    fn extract_rust_structs_enums_traits() {
        let source = r#"
pub struct Point { x: i32, y: i32 }
enum Color { Red, Green }
trait Drawable {}
pub mod utils {}
"#;
        let map = SymbolMap::from_source(source, Language::Rust, "test.rs");
        assert_eq!(map.by_kind(SymbolKind::Struct).len(), 1);
        assert_eq!(map.by_kind(SymbolKind::Enum).len(), 1);
        assert_eq!(map.by_kind(SymbolKind::Trait).len(), 1);
        assert_eq!(map.by_kind(SymbolKind::Module).len(), 1);
    }

    #[test]
    fn extract_rust_constants_and_statics() {
        let source = r#"
const MAX_SIZE: usize = 100;
static NAME: &str = "test";
type Alias = String;
"#;
        let map = SymbolMap::from_source(source, Language::Rust, "test.rs");
        assert_eq!(map.by_kind(SymbolKind::Constant).len(), 1);
        assert_eq!(map.by_kind(SymbolKind::Static).len(), 1);
        assert_eq!(map.by_kind(SymbolKind::TypeAlias).len(), 1);
    }

    #[test]
    fn extract_rust_macro() {
        let source = r#"
macro_rules! my_macro {
    () => {};
}
"#;
        let map = SymbolMap::from_source(source, Language::Rust, "test.rs");
        assert_eq!(map.by_kind(SymbolKind::Macro).len(), 1);
    }

    #[test]
    fn extract_python_functions_and_classes() {
        let source = r#"
def hello():
    pass

async def fetch():
    pass

class Point:
    pass
"#;
        let map = SymbolMap::from_source(source, Language::Python, "test.py");
        assert_eq!(map.by_kind(SymbolKind::Function).len(), 2);
        assert_eq!(map.by_kind(SymbolKind::Class).len(), 1);
    }

    #[test]
    fn extract_javascript_functions() {
        let source = r#"
function hello() {}
export function hi() {}
async function fetchData() {}

class Point {}
export class Circle {}

const MAX = 100;
export const NAME = "test";
"#;
        let map = SymbolMap::from_source(source, Language::JavaScript, "test.js");
        let fns = map.by_kind(SymbolKind::Function);
        assert_eq!(fns.len(), 3);
        assert_eq!(map.by_kind(SymbolKind::Class).len(), 2);
        assert_eq!(map.by_kind(SymbolKind::Constant).len(), 2);
    }

    #[test]
    fn extract_typescript_interfaces() {
        let source = r#"
interface Drawable {
    draw(): void;
}

export interface Shape {}
"#;
        let map = SymbolMap::from_source(source, Language::TypeScript, "test.ts");
        assert_eq!(map.by_kind(SymbolKind::Interface).len(), 2);
    }

    #[test]
    fn extract_go_functions_and_structs() {
        let source = r#"
func main() {}

func (p *Point) Draw() {}

type Point struct {
    X, Y int
}

type Drawable interface {
    Draw()
}
"#;
        let map = SymbolMap::from_source(source, Language::Go, "test.go");
        let fns = map.by_kind(SymbolKind::Function);
        assert_eq!(fns.len(), 2);
        let names: Vec<&str> = fns.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"main"));
        assert!(names.contains(&"Draw"));

        assert_eq!(map.by_kind(SymbolKind::Struct).len(), 1);
        assert_eq!(map.by_kind(SymbolKind::Interface).len(), 1);
    }

    #[test]
    fn unknown_language_returns_empty() {
        let source = "fn main() {}";
        let map = SymbolMap::from_source(source, Language::Markdown, "test.md");
        assert!(map.is_empty());
    }

    #[test]
    fn symbol_lines_are_1_based() {
        let source = "fn first() {}\nfn second() {}\nfn third() {}";
        let map = SymbolMap::from_source(source, Language::Rust, "test.rs");
        assert_eq!(map.symbols[0].line, 1);
        assert_eq!(map.symbols[1].line, 2);
        assert_eq!(map.symbols[2].line, 3);
    }

    #[test]
    fn find_by_name_works() {
        let source = "fn foo() {}\nfn bar() {}\nfn foo() {}";
        let map = SymbolMap::from_source(source, Language::Rust, "test.rs");
        let foo = map.find_by_name("foo");
        assert_eq!(foo.len(), 2);
    }

    #[test]
    fn summary_contains_counts() {
        let source = "fn a() {}\nfn b() {}\nstruct C {}";
        let map = SymbolMap::from_source(source, Language::Rust, "test.rs");
        let s = map.summary();
        assert!(s.contains("3 symbols"));
        assert!(s.contains("function: 2"));
        assert!(s.contains("struct: 1"));
    }

    #[test]
    fn summary_empty() {
        let map = SymbolMap::new();
        assert_eq!(map.summary(), "no symbols");
    }

    #[test]
    fn deduplicates_repeated_matches() {
        let source = "pub fn test() {}";
        let map = SymbolMap::from_source(source, Language::Rust, "test.rs");
        assert_eq!(map.symbols.len(), 1);
    }

    #[test]
    fn symbol_map_serializes() {
        let source = "fn a() {}";
        let map = SymbolMap::from_source(source, Language::Rust, "test.rs");
        let json = serde_json::to_string(&map).unwrap();
        let back: SymbolMap = serde_json::from_str(&json).unwrap();
        assert_eq!(back.symbols.len(), 1);
    }

    #[test]
    fn extend_combines_maps() {
        let mut m1 = SymbolMap::from_source("fn a() {}", Language::Rust, "a.rs");
        let m2 = SymbolMap::from_source("fn b() {}", Language::Rust, "b.rs");
        m1.extend(m2);
        assert_eq!(m1.len(), 2);
    }
}
