//! ID tipado de checkpoint.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// ID único de un checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CheckpointId(pub Uuid);

impl CheckpointId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for CheckpointId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CheckpointId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for CheckpointId {
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
        let a = CheckpointId::new();
        let b = CheckpointId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn ids_roundtrip() {
        let original = CheckpointId::new();
        let s = original.to_string();
        let parsed: CheckpointId = s.parse().unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn ids_serialize_transparently() {
        let id = CheckpointId::new();
        let json = serde_json::to_string(&id).unwrap();
        assert!(json.starts_with('"'));
    }
}
