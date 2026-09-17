//! ID tipado de worktree.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// ID único de un worktree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorktreeId(pub Uuid);

impl WorktreeId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for WorktreeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for WorktreeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for WorktreeId {
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
        let a = WorktreeId::new();
        let b = WorktreeId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn ids_roundtrip() {
        let original = WorktreeId::new();
        let s = original.to_string();
        let parsed: WorktreeId = s.parse().unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn ids_serialize_transparently() {
        let id = WorktreeId::new();
        let json = serde_json::to_string(&id).unwrap();
        assert!(json.starts_with('"'));
        assert!(json.ends_with('"'));
    }

    #[test]
    fn invalid_uuid_fails() {
        let result: Result<WorktreeId, _> = "not-a-uuid".parse();
        assert!(result.is_err());
    }

    #[test]
    fn ids_are_orderable() {
        let a = WorktreeId::new();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = WorktreeId::new();
        assert!(a < b);
    }
}
