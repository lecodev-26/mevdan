//! Detección de lenguaje por extensión de archivo.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Lenguaje de programación.
///
/// Algunas variantes (JavaScript, TypeScript, CSharp) se renombran
/// explícitamente porque `snake_case` las convertiría en
/// `java_script`, `type_script` y `c_sharp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Rust,
    Python,
    #[serde(rename = "javascript")]
    JavaScript,
    #[serde(rename = "typescript")]
    TypeScript,
    Go,
    Java,
    Kotlin,
    C,
    Cpp,
    #[serde(rename = "csharp")]
    CSharp,
    Ruby,
    Php,
    Swift,
    Shell,
    Html,
    Css,
    Sql,
    Yaml,
    Toml,
    Json,
    Markdown,
    Xml,
    Unknown,
}

impl Language {
    pub fn name(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::JavaScript => "javascript",
            Language::TypeScript => "typescript",
            Language::Go => "go",
            Language::Java => "java",
            Language::Kotlin => "kotlin",
            Language::C => "c",
            Language::Cpp => "cpp",
            Language::CSharp => "csharp",
            Language::Ruby => "ruby",
            Language::Php => "php",
            Language::Swift => "swift",
            Language::Shell => "shell",
            Language::Html => "html",
            Language::Css => "css",
            Language::Sql => "sql",
            Language::Yaml => "yaml",
            Language::Toml => "toml",
            Language::Json => "json",
            Language::Markdown => "markdown",
            Language::Xml => "xml",
            Language::Unknown => "unknown",
        }
    }

    pub fn is_programming(&self) -> bool {
        matches!(
            self,
            Language::Rust
                | Language::Python
                | Language::JavaScript
                | Language::TypeScript
                | Language::Go
                | Language::Java
                | Language::Kotlin
                | Language::C
                | Language::Cpp
                | Language::CSharp
                | Language::Ruby
                | Language::Php
                | Language::Swift
                | Language::Shell
        )
    }

    pub fn is_config(&self) -> bool {
        matches!(
            self,
            Language::Yaml | Language::Toml | Language::Json | Language::Xml
        )
    }

    pub fn is_documentation(&self) -> bool {
        matches!(self, Language::Markdown)
    }
}

/// Detector de lenguajes.
#[derive(Debug, Default)]
pub struct LanguageDetector;

impl LanguageDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect(&self, path: &Path) -> Language {
        let ext = match path.extension().and_then(|s| s.to_str()) {
            Some(e) => e.to_lowercase(),
            None => return Language::Unknown,
        };

        match ext.as_str() {
            "rs" => Language::Rust,
            "py" | "pyi" | "pyw" => Language::Python,
            "js" | "mjs" | "cjs" | "jsx" => Language::JavaScript,
            "ts" | "mts" | "cts" | "tsx" => Language::TypeScript,
            "go" => Language::Go,
            "java" => Language::Java,
            "kt" | "kts" => Language::Kotlin,
            "c" | "h" => Language::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => Language::Cpp,
            "cs" => Language::CSharp,
            "rb" => Language::Ruby,
            "php" => Language::Php,
            "swift" => Language::Swift,
            "sh" | "bash" | "zsh" | "fish" => Language::Shell,
            "html" | "htm" => Language::Html,
            "css" | "scss" | "sass" | "less" => Language::Css,
            "sql" => Language::Sql,
            "yaml" | "yml" => Language::Yaml,
            "toml" => Language::Toml,
            "json" => Language::Json,
            "md" | "markdown" => Language::Markdown,
            "xml" => Language::Xml,
            _ => Language::Unknown,
        }
    }

    pub fn detect_by_filename(&self, _filename: &str) -> Language {
        Language::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn detect(path: &str) -> Language {
        LanguageDetector::new().detect(&PathBuf::from(path))
    }

    #[test]
    fn detect_rust() {
        assert_eq!(detect("main.rs"), Language::Rust);
    }

    #[test]
    fn detect_python() {
        assert_eq!(detect("script.py"), Language::Python);
        assert_eq!(detect("module.pyi"), Language::Python);
    }

    #[test]
    fn detect_javascript() {
        assert_eq!(detect("app.js"), Language::JavaScript);
        assert_eq!(detect("app.mjs"), Language::JavaScript);
        assert_eq!(detect("App.jsx"), Language::JavaScript);
    }

    #[test]
    fn detect_typescript() {
        assert_eq!(detect("app.ts"), Language::TypeScript);
        assert_eq!(detect("App.tsx"), Language::TypeScript);
    }

    #[test]
    fn detect_go() {
        assert_eq!(detect("main.go"), Language::Go);
    }

    #[test]
    fn detect_c_and_cpp() {
        assert_eq!(detect("main.c"), Language::C);
        assert_eq!(detect("header.h"), Language::C);
        assert_eq!(detect("main.cpp"), Language::Cpp);
        assert_eq!(detect("header.hpp"), Language::Cpp);
    }

    #[test]
    fn detect_config_files() {
        assert_eq!(detect("config.toml"), Language::Toml);
        assert_eq!(detect("data.json"), Language::Json);
        assert_eq!(detect("config.yaml"), Language::Yaml);
        assert_eq!(detect("config.yml"), Language::Yaml);
    }

    #[test]
    fn detect_docs() {
        assert_eq!(detect("README.md"), Language::Markdown);
    }

    #[test]
    fn detect_shell() {
        assert_eq!(detect("script.sh"), Language::Shell);
        assert_eq!(detect("script.bash"), Language::Shell);
    }

    #[test]
    fn unknown_extension_returns_unknown() {
        assert_eq!(detect("file.xyz123"), Language::Unknown);
        assert_eq!(detect("noextension"), Language::Unknown);
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(detect("MAIN.RS"), Language::Rust);
        assert_eq!(detect("Script.PY"), Language::Python);
    }

    #[test]
    fn language_names() {
        assert_eq!(Language::Rust.name(), "rust");
        assert_eq!(Language::TypeScript.name(), "typescript");
        assert_eq!(Language::Unknown.name(), "unknown");
    }

    #[test]
    fn is_programming() {
        assert!(Language::Rust.is_programming());
        assert!(Language::Python.is_programming());
        assert!(!Language::Toml.is_programming());
        assert!(!Language::Markdown.is_programming());
        assert!(!Language::Unknown.is_programming());
    }

    #[test]
    fn is_config() {
        assert!(Language::Toml.is_config());
        assert!(Language::Json.is_config());
        assert!(!Language::Rust.is_config());
    }

    #[test]
    fn is_documentation() {
        assert!(Language::Markdown.is_documentation());
        assert!(!Language::Rust.is_documentation());
    }

    #[test]
    fn language_serializes_snake_case() {
        assert_eq!(serde_json::to_string(&Language::Rust).unwrap(), "\"rust\"");
        assert_eq!(
            serde_json::to_string(&Language::JavaScript).unwrap(),
            "\"javascript\""
        );
        assert_eq!(
            serde_json::to_string(&Language::TypeScript).unwrap(),
            "\"typescript\""
        );
        assert_eq!(
            serde_json::to_string(&Language::CSharp).unwrap(),
            "\"csharp\""
        );
    }
}
