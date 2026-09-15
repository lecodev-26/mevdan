//! ID tipado de tarea.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// ID único de una tarea.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TaskId(pub Uuid);

impl TaskId {
    /// Genera un ID nuevo (UUID v7, ordenable por tiempo).
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// UUID interno.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for TaskId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_ids_are_unique() {
        let a = TaskId::new();
        let b = TaskId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn ids_roundtrip_through_string() {
        let original = TaskId::new();
        let s = original.to_string();
        let parsed: TaskId = s.parse().unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn ids_serialize_as_transparent_strings() {
        let id = TaskId::new();
        let json = serde_json::to_string(&id).unwrap();
        assert!(json.starts_with('"'));
        assert!(json.ends_with('"'));
    }

    #[test]
    fn invalid_uuid_fails() {
        let result: Result<TaskId, _> = "not-a-uuid".parse();
        assert!(result.is_err());
    }

    #[test]
    fn ids_are_orderable() {
        let a = TaskId::new();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = TaskId::new();
        // UUID v7 es ordenable por tiempo.
        assert!(a < b);
    }
}
