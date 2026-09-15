//! Audit trail y replay.
//!
//! Construye un timeline legible a partir del Event Log existente
//! (tabla `events`, creada en V001). No añade tablas nuevas.

use crate::{error::StorageResult, repo::event_repo};
use chrono::{DateTime, Utc};
use mevdan_core::{
    event::{Event, EventKind},
    ids::ProjectId,
};
use serde::{Deserialize, Serialize};

/// Categoría de un evento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    Lifecycle,
    Session,
    Agent,
    Tool,
    Task,
    Permission,
    Checkpoint,
    Verification,
    Unknown,
}

impl EventCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            EventCategory::Lifecycle => "lifecycle",
            EventCategory::Session => "session",
            EventCategory::Agent => "agent",
            EventCategory::Tool => "tool",
            EventCategory::Task => "task",
            EventCategory::Permission => "permission",
            EventCategory::Checkpoint => "checkpoint",
            EventCategory::Verification => "verification",
            EventCategory::Unknown => "unknown",
        }
    }

    pub fn from_kind(kind: EventKind) -> Self {
        match kind {
            EventKind::ProjectCreated => EventCategory::Lifecycle,
            EventKind::SessionStarted | EventKind::SessionEnded => EventCategory::Session,
        }
    }
}

/// Un evento enriquecido con categoría y descripción legible.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub occurred_at: DateTime<Utc>,
    pub kind: EventKind,
    pub category: EventCategory,
    pub description: String,
    pub payload: serde_json::Value,
}

impl TimelineEntry {
    pub fn from_event(event: &Event) -> Self {
        let category = EventCategory::from_kind(event.kind);
        let description = describe(event);
        Self {
            occurred_at: event.occurred_at,
            kind: event.kind,
            category,
            description,
            payload: event.payload.clone(),
        }
    }

    pub fn human_line(&self) -> String {
        format!(
            "[{}] [{}] {}",
            self.occurred_at.format("%Y-%m-%d %H:%M:%S"),
            self.category.display_name(),
            self.description,
        )
    }
}

/// Audit trail: secuencia ordenada de entradas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTrail {
    pub project_id: ProjectId,
    pub entries: Vec<TimelineEntry>,
}

impl AuditTrail {
    pub fn build(conn: &rusqlite::Connection, project_id: ProjectId) -> StorageResult<Self> {
        let events = event_repo::list_by_project(conn, project_id)?;
        let entries = events.iter().map(TimelineEntry::from_event).collect();
        Ok(Self {
            project_id,
            entries,
        })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[TimelineEntry] {
        &self.entries
    }

    pub fn by_category(&self, category: EventCategory) -> Vec<&TimelineEntry> {
        self.entries
            .iter()
            .filter(|e| e.category == category)
            .collect()
    }

    pub fn between(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<&TimelineEntry> {
        self.entries
            .iter()
            .filter(|e| e.occurred_at >= start && e.occurred_at <= end)
            .collect()
    }

    pub fn summary(&self) -> String {
        let mut by_cat: std::collections::BTreeMap<EventCategory, usize> =
            std::collections::BTreeMap::new();
        for e in &self.entries {
            *by_cat.entry(e.category).or_insert(0) += 1;
        }

        let parts: Vec<String> = by_cat
            .iter()
            .map(|(k, v)| format!("{}: {}", k.display_name(), v))
            .collect();

        if parts.is_empty() {
            "audit trail: no events".to_string()
        } else {
            format!("audit trail ({} events): {}", self.len(), parts.join(", "))
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Replay: reconstrucción del estado hasta un punto.
pub struct Replay;

impl Replay {
    pub fn until(
        conn: &rusqlite::Connection,
        project_id: ProjectId,
        timestamp: DateTime<Utc>,
    ) -> StorageResult<AuditTrail> {
        let full = AuditTrail::build(conn, project_id)?;
        let entries: Vec<TimelineEntry> = full
            .entries
            .into_iter()
            .filter(|e| e.occurred_at <= timestamp)
            .collect();
        Ok(AuditTrail {
            project_id,
            entries,
        })
    }

    pub fn between(
        conn: &rusqlite::Connection,
        project_id: ProjectId,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> StorageResult<AuditTrail> {
        let full = AuditTrail::build(conn, project_id)?;
        let entries: Vec<TimelineEntry> = full
            .entries
            .into_iter()
            .filter(|e| e.occurred_at >= start && e.occurred_at <= end)
            .collect();
        Ok(AuditTrail {
            project_id,
            entries,
        })
    }
}

/// Genera una descripción legible de un evento.
fn describe(event: &Event) -> String {
    match event.kind {
        EventKind::ProjectCreated => {
            let name = event
                .payload
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let version = event
                .payload
                .get("mevdan_version")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            format!("project created: {} (MEVDAN {})", name, version)
        }
        EventKind::SessionStarted => {
            let label = event
                .payload
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("unnamed");
            format!("session started: {}", label)
        }
        EventKind::SessionEnded => {
            let reason = event
                .payload
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("completed");
            format!("session ended: {}", reason)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        db::Database,
        repo::{event_repo, project_repo, session_repo},
    };
    use mevdan_core::{project::Project, session::Session};
    use std::fs;
    use tempfile::TempDir;

    fn setup_with_events() -> (TempDir, Database, Project) {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".mevdan")).unwrap();
        let db = Database::open(dir.path()).unwrap();

        let p = Project::new("demo", "0.1.0");
        project_repo::insert(db.connection(), &p).unwrap();

        event_repo::append(
            db.connection(),
            p.id,
            None,
            EventKind::ProjectCreated,
            serde_json::json!({"name": "demo", "mevdan_version": "0.1.0"}),
        )
        .unwrap();

        let s = Session::new(p.id, Some("initial".into()));
        session_repo::insert(db.connection(), &s).unwrap();

        event_repo::append(
            db.connection(),
            p.id,
            Some(s.id),
            EventKind::SessionStarted,
            serde_json::json!({"label": "initial"}),
        )
        .unwrap();

        (dir, db, p)
    }

    #[test]
    fn category_display_names() {
        assert_eq!(EventCategory::Lifecycle.display_name(), "lifecycle");
        assert_eq!(EventCategory::Tool.display_name(), "tool");
        assert_eq!(EventCategory::Unknown.display_name(), "unknown");
    }

    #[test]
    fn category_from_kind() {
        assert_eq!(
            EventCategory::from_kind(EventKind::ProjectCreated),
            EventCategory::Lifecycle
        );
        assert_eq!(
            EventCategory::from_kind(EventKind::SessionStarted),
            EventCategory::Session
        );
        assert_eq!(
            EventCategory::from_kind(EventKind::SessionEnded),
            EventCategory::Session
        );
    }

    #[test]
    fn category_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&EventCategory::Lifecycle).unwrap(),
            "\"lifecycle\""
        );
        assert_eq!(
            serde_json::to_string(&EventCategory::Tool).unwrap(),
            "\"tool\""
        );
    }

    #[test]
    fn audit_trail_from_events() {
        let (_dir, db, project) = setup_with_events();
        let trail = AuditTrail::build(db.connection(), project.id).unwrap();
        assert_eq!(trail.len(), 2);
        assert!(!trail.is_empty());
    }

    #[test]
    fn audit_trail_empty_for_unknown_project() {
        let (_dir, db, _project) = setup_with_events();
        let trail = AuditTrail::build(db.connection(), ProjectId::new()).unwrap();
        assert!(trail.is_empty());
    }

    #[test]
    fn audit_trail_orders_entries() {
        let (_dir, db, project) = setup_with_events();
        let trail = AuditTrail::build(db.connection(), project.id).unwrap();
        for i in 1..trail.entries.len() {
            assert!(trail.entries[i].occurred_at >= trail.entries[i - 1].occurred_at);
        }
    }

    #[test]
    fn by_category_filters() {
        let (_dir, db, project) = setup_with_events();
        let trail = AuditTrail::build(db.connection(), project.id).unwrap();

        let lifecycle = trail.by_category(EventCategory::Lifecycle);
        assert_eq!(lifecycle.len(), 1);
        assert_eq!(lifecycle[0].kind, EventKind::ProjectCreated);

        let session = trail.by_category(EventCategory::Session);
        assert_eq!(session.len(), 1);
        assert_eq!(session[0].kind, EventKind::SessionStarted);

        let tool = trail.by_category(EventCategory::Tool);
        assert_eq!(tool.len(), 0);
    }

    #[test]
    fn between_filters_by_time() {
        let (_dir, db, project) = setup_with_events();
        let trail = AuditTrail::build(db.connection(), project.id).unwrap();

        let first = trail.entries[0].occurred_at;
        let last = trail.entries[trail.entries.len() - 1].occurred_at;

        let all = trail.between(first, last);
        assert_eq!(all.len(), 2);

        let only_first = trail.between(first, first);
        assert_eq!(only_first.len(), 1);
    }

    #[test]
    fn summary_contains_categories() {
        let (_dir, db, project) = setup_with_events();
        let trail = AuditTrail::build(db.connection(), project.id).unwrap();
        let summary = trail.summary();
        assert!(summary.contains("2 events"));
        assert!(summary.contains("lifecycle"));
        assert!(summary.contains("session"));
    }

    #[test]
    fn summary_empty_trail() {
        let (_dir, db, _project) = setup_with_events();
        let trail = AuditTrail::build(db.connection(), ProjectId::new()).unwrap();
        assert_eq!(trail.summary(), "audit trail: no events");
    }

    #[test]
    fn timeline_entry_human_line() {
        let (_dir, db, project) = setup_with_events();
        let trail = AuditTrail::build(db.connection(), project.id).unwrap();
        let line = trail.entries[0].human_line();
        assert!(line.contains("[lifecycle]"));
        assert!(line.contains("project created: demo"));
    }

    #[test]
    fn timeline_entry_describes_session_started() {
        let (_dir, db, project) = setup_with_events();
        let trail = AuditTrail::build(db.connection(), project.id).unwrap();
        let session_entry = trail
            .entries
            .iter()
            .find(|e| e.kind == EventKind::SessionStarted)
            .unwrap();
        assert!(session_entry
            .description
            .contains("session started: initial"));
    }

    #[test]
    fn replay_until() {
        let (_dir, db, project) = setup_with_events();
        let full = AuditTrail::build(db.connection(), project.id).unwrap();
        let first_time = full.entries[0].occurred_at;

        let replay = Replay::until(db.connection(), project.id, first_time).unwrap();
        assert_eq!(replay.len(), 1);
        assert_eq!(replay.entries[0].kind, EventKind::ProjectCreated);
    }

    #[test]
    fn replay_until_returns_empty_for_old_timestamp() {
        let (_dir, db, project) = setup_with_events();
        let old = DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let replay = Replay::until(db.connection(), project.id, old).unwrap();
        assert!(replay.is_empty());
    }

    #[test]
    fn replay_between() {
        let (_dir, db, project) = setup_with_events();
        let full = AuditTrail::build(db.connection(), project.id).unwrap();
        let first = full.entries[0].occurred_at;
        let last = full.entries[full.entries.len() - 1].occurred_at;

        let replay = Replay::between(db.connection(), project.id, first, last).unwrap();
        assert_eq!(replay.len(), 2);
    }

    #[test]
    fn replay_between_narrow_range() {
        let (_dir, db, project) = setup_with_events();
        let full = AuditTrail::build(db.connection(), project.id).unwrap();
        let first = full.entries[0].occurred_at;

        let replay = Replay::between(db.connection(), project.id, first, first).unwrap();
        assert_eq!(replay.len(), 1);
    }

    #[test]
    fn audit_trail_roundtrips() {
        let (_dir, db, project) = setup_with_events();
        let trail = AuditTrail::build(db.connection(), project.id).unwrap();
        let json = trail.to_json().unwrap();
        let back: AuditTrail = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), trail.len());
        assert_eq!(back.project_id, trail.project_id);
    }
}
