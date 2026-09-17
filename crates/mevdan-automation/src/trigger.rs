//! `Trigger` — qué dispara una automatización.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Un trigger de automatización.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Trigger {
    /// Cuando un archivo cambia.
    FileChange {
        /// Ruta o patrón (simple: match por nombre de archivo o extensión).
        path: String,
    },
    /// Cuando ocurre un evento concreto (por nombre).
    Event {
        /// Nombre del evento (ej. "session.ended", "task.failed").
        name: String,
    },
    /// Programado (cron-like simplificado: cada N minutos).
    Schedule {
        /// Intervalo en minutos.
        every_minutes: u32,
    },
    /// Manual: solo cuando el usuario lo dispara.
    Manual,
}

impl Trigger {
    pub fn file_change(path: impl Into<String>) -> Self {
        Trigger::FileChange { path: path.into() }
    }

    pub fn event(name: impl Into<String>) -> Self {
        Trigger::Event { name: name.into() }
    }

    pub fn schedule(every_minutes: u32) -> Self {
        Trigger::Schedule { every_minutes }
    }

    pub fn manual() -> Self {
        Trigger::Manual
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Trigger::FileChange { .. } => "file_change",
            Trigger::Event { .. } => "event",
            Trigger::Schedule { .. } => "schedule",
            Trigger::Manual => "manual",
        }
    }

    /// Evalúa si el trigger matchea un evento de entrada.
    pub fn matches(&self, input: &TriggerInput) -> bool {
        match (self, input) {
            (Trigger::FileChange { path }, TriggerInput::FileChanged { path: changed }) => {
                path_matches(path, changed)
            }
            (Trigger::Event { name }, TriggerInput::Event { name: fired }) => name == fired,
            (Trigger::Schedule { every_minutes }, TriggerInput::Tick { elapsed_minutes }) => {
                *every_minutes > 0 && elapsed_minutes >= every_minutes
            }
            (Trigger::Manual, TriggerInput::Manual) => true,
            _ => false,
        }
    }
}

/// Coincidencia simple de path: exact match o extensión.
fn path_matches(pattern: &str, path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // Si el pattern es una extensión (empieza por ".").
    if let Some(ext) = pattern.strip_prefix('.') {
        if let Some(actual_ext) = path.extension().and_then(|e| e.to_str()) {
            return actual_ext == ext;
        }
        return false;
    }

    // Si contiene glob-like "*".
    if let Some(prefix) = pattern.strip_suffix('*') {
        return path_str.starts_with(prefix);
    }

    // Match exacto por nombre de archivo.
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        if file_name == pattern {
            return true;
        }
    }

    // Match por path completo.
    path_str == pattern
}

/// Entrada de evaluación de un trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerInput {
    FileChanged { path: PathBuf },
    Event { name: String },
    Tick { elapsed_minutes: u32 },
    Manual,
}

impl TriggerInput {
    pub fn file_changed(path: impl Into<PathBuf>) -> Self {
        TriggerInput::FileChanged { path: path.into() }
    }

    pub fn event(name: impl Into<String>) -> Self {
        TriggerInput::Event { name: name.into() }
    }

    pub fn tick(elapsed_minutes: u32) -> Self {
        TriggerInput::Tick { elapsed_minutes }
    }

    pub fn manual() -> Self {
        TriggerInput::Manual
    }
}

/// Un disparo registrado de un trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerFiring {
    pub rule_name: String,
    pub trigger: Trigger,
    pub input: TriggerInput,
    pub fired_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_constructors() {
        assert!(matches!(
            Trigger::file_change("*.rs"),
            Trigger::FileChange { .. }
        ));
        assert!(matches!(
            Trigger::event("task.failed"),
            Trigger::Event { .. }
        ));
        assert!(matches!(Trigger::schedule(60), Trigger::Schedule { .. }));
        assert_eq!(Trigger::manual(), Trigger::Manual);
    }

    #[test]
    fn trigger_display_names() {
        assert_eq!(Trigger::file_change("x").display_name(), "file_change");
        assert_eq!(Trigger::event("x").display_name(), "event");
        assert_eq!(Trigger::schedule(1).display_name(), "schedule");
        assert_eq!(Trigger::manual().display_name(), "manual");
    }

    #[test]
    fn trigger_serializes_tagged() {
        let t = Trigger::event("task.failed");
        let json = serde_json::to_value(&t).unwrap();
        assert_eq!(json["type"], "event");
        assert_eq!(json["name"], "task.failed");
    }

    #[test]
    fn file_change_matches_exact_name() {
        let t = Trigger::file_change("main.rs");
        assert!(t.matches(&TriggerInput::file_changed("/a/b/main.rs")));
        assert!(!t.matches(&TriggerInput::file_changed("/a/b/lib.rs")));
    }

    #[test]
    fn file_change_matches_extension() {
        let t = Trigger::file_change(".rs");
        assert!(t.matches(&TriggerInput::file_changed("/a/b/main.rs")));
        assert!(t.matches(&TriggerInput::file_changed("/a/lib.rs")));
        assert!(!t.matches(&TriggerInput::file_changed("/a/script.py")));
    }

    #[test]
    fn file_change_matches_prefix_wildcard() {
        let t = Trigger::file_change("src/*");
        assert!(t.matches(&TriggerInput::file_changed("src/main.rs")));
        assert!(t.matches(&TriggerInput::file_changed("src/lib.rs")));
        assert!(!t.matches(&TriggerInput::file_changed("tests/x.rs")));
    }

    #[test]
    fn file_change_does_not_match_event() {
        let t = Trigger::file_change("main.rs");
        assert!(!t.matches(&TriggerInput::event("task.failed")));
    }

    #[test]
    fn event_matches_exact_name() {
        let t = Trigger::event("task.failed");
        assert!(t.matches(&TriggerInput::event("task.failed")));
        assert!(!t.matches(&TriggerInput::event("task.completed")));
    }

    #[test]
    fn event_does_not_match_file() {
        let t = Trigger::event("x");
        assert!(!t.matches(&TriggerInput::file_changed("x")));
    }

    #[test]
    fn schedule_matches_when_elapsed_reached() {
        let t = Trigger::schedule(60);
        assert!(t.matches(&TriggerInput::tick(60)));
        assert!(t.matches(&TriggerInput::tick(120)));
        assert!(!t.matches(&TriggerInput::tick(59)));
    }

    #[test]
    fn schedule_zero_never_matches() {
        let t = Trigger::schedule(0);
        assert!(!t.matches(&TriggerInput::tick(1000)));
    }

    #[test]
    fn schedule_does_not_match_manual() {
        let t = Trigger::schedule(60);
        assert!(!t.matches(&TriggerInput::manual()));
    }

    #[test]
    fn manual_matches_manual_only() {
        let t = Trigger::manual();
        assert!(t.matches(&TriggerInput::manual()));
        assert!(!t.matches(&TriggerInput::tick(100)));
        assert!(!t.matches(&TriggerInput::event("x")));
    }

    #[test]
    fn firing_serializes() {
        let firing = TriggerFiring {
            rule_name: "on-save".into(),
            trigger: Trigger::file_change("*.rs"),
            input: TriggerInput::file_changed("/a/main.rs"),
            fired_at: Utc::now(),
        };
        let json = serde_json::to_string(&firing).unwrap();
        let back: TriggerFiring = serde_json::from_str(&json).unwrap();
        assert_eq!(back.rule_name, "on-save");
    }
}
