//! `TaskKind` — clasificación de tareas para routing.

use serde::{Deserialize, Serialize};

/// Tipo de tarea. Se usa para decidir qué agente y qué modelo usar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// Planificación, descomposición de objetivos.
    Planning,
    /// Escritura de código.
    Coding,
    /// Revisión de código o trabajo.
    Review,
    /// Investigación, búsqueda de información.
    Research,
    /// Testing, verificación.
    Testing,
    /// Documentación.
    Documentation,
    /// Análisis de datos.
    DataAnalysis,
    /// Refactorización.
    Refactoring,
    /// Debugging.
    Debugging,
    /// Otra cosa.
    Other,
}

impl TaskKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            TaskKind::Planning => "planning",
            TaskKind::Coding => "coding",
            TaskKind::Review => "review",
            TaskKind::Research => "research",
            TaskKind::Testing => "testing",
            TaskKind::Documentation => "documentation",
            TaskKind::DataAnalysis => "data_analysis",
            TaskKind::Refactoring => "refactoring",
            TaskKind::Debugging => "debugging",
            TaskKind::Other => "other",
        }
    }

    /// Clasifica una descripción textual en un `TaskKind`.
    ///
    /// Heurística simple por palabras clave. Sin ML.
    pub fn classify(text: &str) -> Self {
        let lower = text.to_lowercase();

        // Orden importa: los más específicos primero.
        if contains_any(&lower, &["debug", "bug", "error", "fix crash", "traceback"]) {
            return TaskKind::Debugging;
        }
        if contains_any(
            &lower,
            &["refactor", "clean up", "reorganize", "restructure"],
        ) {
            return TaskKind::Refactoring;
        }
        if contains_any(&lower, &["test", "assert", "coverage", "verify"]) {
            return TaskKind::Testing;
        }
        if contains_any(
            &lower,
            &["review", "audit", "critique", "check quality", "feedback"],
        ) {
            return TaskKind::Review;
        }
        if contains_any(
            &lower,
            &["plan", "design", "architect", "break down", "strategy"],
        ) {
            return TaskKind::Planning;
        }
        if contains_any(&lower, &["document", "readme", "docstring", "comment"]) {
            return TaskKind::Documentation;
        }
        if contains_any(
            &lower,
            &["research", "find", "search", "investigate", "explore"],
        ) {
            return TaskKind::Research;
        }
        if contains_any(
            &lower,
            &["analyze", "analysis", "data", "statistics", "dataset"],
        ) {
            return TaskKind::DataAnalysis;
        }
        if contains_any(
            &lower,
            &[
                "code",
                "implement",
                "write a function",
                "create a class",
                "build",
                "compile",
                "cargo",
                "program",
            ],
        ) {
            return TaskKind::Coding;
        }

        TaskKind::Other
    }
}

/// ¿El texto contiene alguna de las palabras clave?
fn contains_any(text: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|k| text.contains(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names() {
        assert_eq!(TaskKind::Coding.display_name(), "coding");
        assert_eq!(TaskKind::DataAnalysis.display_name(), "data_analysis");
        assert_eq!(TaskKind::Other.display_name(), "other");
    }

    #[test]
    fn serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&TaskKind::Coding).unwrap(),
            "\"coding\""
        );
        assert_eq!(
            serde_json::to_string(&TaskKind::DataAnalysis).unwrap(),
            "\"data_analysis\""
        );
    }

    #[test]
    fn classify_debugging() {
        assert_eq!(
            TaskKind::classify("fix the bug in main"),
            TaskKind::Debugging
        );
        assert_eq!(TaskKind::classify("there is an error"), TaskKind::Debugging);
    }

    #[test]
    fn classify_refactoring() {
        assert_eq!(
            TaskKind::classify("refactor the auth module"),
            TaskKind::Refactoring
        );
    }

    #[test]
    fn classify_testing() {
        assert_eq!(
            TaskKind::classify("write tests for parser"),
            TaskKind::Testing
        );
        assert_eq!(TaskKind::classify("add coverage"), TaskKind::Testing);
    }

    #[test]
    fn classify_review() {
        assert_eq!(
            TaskKind::classify("review this pull request"),
            TaskKind::Review
        );
    }

    #[test]
    fn classify_planning() {
        assert_eq!(
            TaskKind::classify("plan the architecture"),
            TaskKind::Planning
        );
    }

    #[test]
    fn classify_documentation() {
        assert_eq!(
            TaskKind::classify("write the README"),
            TaskKind::Documentation
        );
    }

    #[test]
    fn classify_research() {
        assert_eq!(
            TaskKind::classify("research how to use async"),
            TaskKind::Research
        );
    }

    #[test]
    fn classify_data_analysis() {
        assert_eq!(
            TaskKind::classify("analyze this dataset"),
            TaskKind::DataAnalysis
        );
    }

    #[test]
    fn classify_coding() {
        assert_eq!(TaskKind::classify("implement a parser"), TaskKind::Coding);
        assert_eq!(TaskKind::classify("write a function"), TaskKind::Coding);
    }

    #[test]
    fn classify_other() {
        assert_eq!(TaskKind::classify("hello world"), TaskKind::Other);
    }

    #[test]
    fn classify_case_insensitive() {
        assert_eq!(TaskKind::classify("DEBUG THE CODE"), TaskKind::Debugging);
        assert_eq!(TaskKind::classify("Refactor"), TaskKind::Refactoring);
    }

    #[test]
    fn classify_priority_specific_first() {
        // "test the bug fix" → debug (bug está primero).
        assert_eq!(
            TaskKind::classify("debug and test the bug"),
            TaskKind::Debugging
        );
    }
}
