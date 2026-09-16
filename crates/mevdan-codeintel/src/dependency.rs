//! Extracción de dependencias (imports) por parsing básico.

use crate::language::Language;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Tipo de dependencia.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    Import,
    Include,
    ReExport,
    Unknown,
}

impl DependencyKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            DependencyKind::Import => "import",
            DependencyKind::Include => "include",
            DependencyKind::ReExport => "re_export",
            DependencyKind::Unknown => "unknown",
        }
    }
}

/// Una dependencia.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub target: String,
    pub kind: DependencyKind,
    pub line: usize,
    pub is_internal: bool,
}

impl Dependency {
    pub fn new(target: impl Into<String>, kind: DependencyKind, line: usize) -> Self {
        let target = target.into();
        let is_internal = is_internal_target(&target);
        Self {
            target,
            kind,
            line,
            is_internal,
        }
    }
}

fn is_internal_target(target: &str) -> bool {
    target.starts_with('.')
        || target.starts_with('/')
        || target.starts_with("crate::")
        || target.starts_with("super::")
        || target.starts_with("self::")
        || target.starts_with("crate ")
}

/// Mapa de dependencias.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyMap {
    pub dependencies: Vec<Dependency>,
}

impl DependencyMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_file(path: &Path, language: Language) -> Self {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };
        Self::from_source(&content, language)
    }

    pub fn from_source(source: &str, language: Language) -> Self {
        let patterns = patterns_for(language);
        let mut dependencies = Vec::new();

        for (kind, re) in patterns {
            for (line_idx, line) in source.lines().enumerate() {
                if let Some(caps) = re.captures(line) {
                    if let Some(target_match) = caps.get(1) {
                        let target = target_match.as_str().trim().to_string();
                        if target.is_empty() {
                            continue;
                        }
                        dependencies.push(Dependency::new(target, kind, line_idx + 1));
                    }
                }
            }
        }

        dependencies.sort_by(|a, b| a.line.cmp(&b.line).then(a.target.cmp(&b.target)));
        dependencies.dedup_by(|a, b| a.line == b.line && a.target == b.target);

        Self { dependencies }
    }

    pub fn len(&self) -> usize {
        self.dependencies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dependencies.is_empty()
    }

    pub fn internal(&self) -> Vec<&Dependency> {
        self.dependencies.iter().filter(|d| d.is_internal).collect()
    }

    pub fn external(&self) -> Vec<&Dependency> {
        self.dependencies
            .iter()
            .filter(|d| !d.is_internal)
            .collect()
    }

    pub fn find_by_target(&self, target: &str) -> Vec<&Dependency> {
        self.dependencies
            .iter()
            .filter(|d| d.target == target)
            .collect()
    }

    pub fn extend(&mut self, other: DependencyMap) {
        self.dependencies.extend(other.dependencies);
    }

    pub fn summary(&self) -> String {
        if self.dependencies.is_empty() {
            return "no dependencies".to_string();
        }
        format!(
            "{} dependencies ({} internal, {} external)",
            self.dependencies.len(),
            self.internal().len(),
            self.external().len()
        )
    }
}

fn patterns_for(language: Language) -> Vec<(DependencyKind, Regex)> {
    match language {
        Language::Rust => rust_patterns(),
        Language::Python => python_patterns(),
        Language::JavaScript | Language::TypeScript => js_patterns(),
        Language::Go => go_patterns(),
        Language::C | Language::Cpp => c_patterns(),
        _ => Vec::new(),
    }
}

fn rust_patterns() -> Vec<(DependencyKind, Regex)> {
    vec![
        (
            DependencyKind::Import,
            Regex::new(r"^\s*use\s+([a-zA-Z_][a-zA-Z0-9_:]*)").unwrap(),
        ),
        (
            DependencyKind::ReExport,
            Regex::new(r"^\s*pub\s+use\s+([a-zA-Z_][a-zA-Z0-9_:]*)").unwrap(),
        ),
    ]
}

fn python_patterns() -> Vec<(DependencyKind, Regex)> {
    vec![
        // `import os`, `import sys`, `import a.b.c`
        (
            DependencyKind::Import,
            Regex::new(r"^\s*import\s+([a-zA-Z_][a-zA-Z0-9_.]*)").unwrap(),
        ),
        // `from pathlib import Path`
        // `from . import local`
        // `from ..utils import helper`
        // El target puede empezar por `.` (relativo).
        (
            DependencyKind::Import,
            Regex::new(r"^\s*from\s+(\.*[a-zA-Z_][a-zA-Z0-9_.]*)\s+import").unwrap(),
        ),
        // `from . import local` (caso especial: solo puntos, sin nombre)
        (
            DependencyKind::Import,
            Regex::new(r"^\s*from\s+(\.+)\s+import").unwrap(),
        ),
    ]
}

fn js_patterns() -> Vec<(DependencyKind, Regex)> {
    vec![
        (
            DependencyKind::Import,
            Regex::new(r#"^\s*import\s+.*?\s+from\s+['"]([^'"]+)['"]"#).unwrap(),
        ),
        (
            DependencyKind::Import,
            Regex::new(r#"^\s*import\s+['"]([^'"]+)['"]"#).unwrap(),
        ),
        (
            DependencyKind::Import,
            Regex::new(r#"require\s*\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap(),
        ),
    ]
}

fn go_patterns() -> Vec<(DependencyKind, Regex)> {
    vec![(
        DependencyKind::Import,
        Regex::new(r#"^\s*(?:import\s+)?(?:\w+\s+)?"([^"]+)""#).unwrap(),
    )]
}

fn c_patterns() -> Vec<(DependencyKind, Regex)> {
    vec![(
        DependencyKind::Include,
        Regex::new(r#"^\s*#include\s*[<"]([^>"]+)[>"]"#).unwrap(),
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_kind_display_names() {
        assert_eq!(DependencyKind::Import.display_name(), "import");
        assert_eq!(DependencyKind::Include.display_name(), "include");
        assert_eq!(DependencyKind::ReExport.display_name(), "re_export");
    }

    #[test]
    fn extract_rust_use() {
        let source = r#"
use std::collections::HashMap;
use crate::foo;
use super::bar;
pub use crate::Baz;
"#;
        let map = DependencyMap::from_source(source, Language::Rust);
        assert_eq!(map.len(), 4);
        assert_eq!(map.find_by_target("std::collections::HashMap").len(), 1);
        assert_eq!(map.find_by_target("crate::foo").len(), 1);
        assert_eq!(map.find_by_target("super::bar").len(), 1);
        assert_eq!(map.find_by_target("crate::Baz").len(), 1);
    }

    #[test]
    fn rust_classifies_internal_vs_external() {
        let source = r#"
use std::collections::HashMap;
use crate::foo;
use super::bar;
use self::baz;
use external_crate::Thing;
"#;
        let map = DependencyMap::from_source(source, Language::Rust);
        assert_eq!(map.internal().len(), 3);
        assert_eq!(map.external().len(), 2);
    }

    #[test]
    fn extract_python_imports() {
        let source = r#"
import os
import sys
from pathlib import Path
from . import local
"#;
        let map = DependencyMap::from_source(source, Language::Python);
        assert_eq!(map.len(), 4);
        assert!(!map.find_by_target("os").is_empty());
        assert!(!map.find_by_target("pathlib").is_empty());
        assert!(!map.find_by_target(".").is_empty());
    }

    #[test]
    fn extract_python_relative_imports() {
        let source = r#"
from . import local
from ..utils import helper
from .module import thing
"#;
        let map = DependencyMap::from_source(source, Language::Python);
        assert_eq!(map.len(), 3);
        assert!(!map.find_by_target(".").is_empty());
        assert!(!map.find_by_target("..utils").is_empty());
        assert!(!map.find_by_target(".module").is_empty());
    }

    #[test]
    fn extract_js_imports() {
        let source = r#"
import React from 'react';
import { useState } from "react";
import './styles.css';
const fs = require('fs');
"#;
        let map = DependencyMap::from_source(source, Language::JavaScript);
        assert_eq!(map.find_by_target("react").len(), 2);
        assert_eq!(map.find_by_target("./styles.css").len(), 1);
        assert_eq!(map.find_by_target("fs").len(), 1);
    }

    #[test]
    fn extract_go_imports() {
        let source = r#"
import "fmt"
import (
    "os"
    "strings"
)
"#;
        let map = DependencyMap::from_source(source, Language::Go);
        assert!(!map.find_by_target("fmt").is_empty());
        assert!(!map.find_by_target("os").is_empty());
    }

    #[test]
    fn extract_c_includes() {
        let source = r#"
#include <stdio.h>
#include <stdlib.h>
#include "myheader.h"
"#;
        let map = DependencyMap::from_source(source, Language::C);
        assert_eq!(map.len(), 3);
        assert_eq!(map.find_by_target("stdio.h").len(), 1);
        assert_eq!(map.find_by_target("myheader.h").len(), 1);
    }

    #[test]
    fn unknown_language_returns_empty() {
        let map = DependencyMap::from_source("any", Language::Markdown);
        assert!(map.is_empty());
    }

    #[test]
    fn summary_contains_counts() {
        let source = "use std::io;\nuse crate::foo;";
        let map = DependencyMap::from_source(source, Language::Rust);
        let s = map.summary();
        assert!(s.contains("2 dependencies"));
        assert!(s.contains("1 internal"));
        assert!(s.contains("1 external"));
    }

    #[test]
    fn summary_empty() {
        let map = DependencyMap::new();
        assert_eq!(map.summary(), "no dependencies");
    }

    #[test]
    fn dependency_map_serializes() {
        let map = DependencyMap::from_source("use std::io;", Language::Rust);
        let json = serde_json::to_string(&map).unwrap();
        let back: DependencyMap = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), 1);
    }

    #[test]
    fn extend_combines_maps() {
        let mut m1 = DependencyMap::from_source("use std::io;", Language::Rust);
        let m2 = DependencyMap::from_source("use std::fmt;", Language::Rust);
        m1.extend(m2);
        assert_eq!(m1.len(), 2);
    }

    #[test]
    fn lines_are_1_based() {
        let source = "use std::io;\nuse std::fmt;\nuse std::collections::HashMap;";
        let map = DependencyMap::from_source(source, Language::Rust);
        assert_eq!(map.dependencies[0].line, 1);
        assert_eq!(map.dependencies[1].line, 2);
        assert_eq!(map.dependencies[2].line, 3);
    }

    #[test]
    fn is_internal_detection() {
        assert!(is_internal_target("./foo"));
        assert!(is_internal_target("../bar"));
        assert!(is_internal_target("/abs"));
        assert!(is_internal_target("crate::x"));
        assert!(is_internal_target("super::y"));
        assert!(is_internal_target("self::z"));
        assert!(is_internal_target("."));

        assert!(!is_internal_target("std::io"));
        assert!(!is_internal_target("react"));
        assert!(!is_internal_target("fmt"));
    }
}
