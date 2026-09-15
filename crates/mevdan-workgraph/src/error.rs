//! Errores del crate `mevdan-workgraph`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkGraphError {
    #[error("node not found: {0}")]
    NodeNotFound(String),

    #[error("duplicate node id: {0}")]
    DuplicateNode(String),

    #[error("invalid edge: {0}")]
    InvalidEdge(String),

    #[error("edge would create a cycle: {from} → {to}")]
    CycleDetected { from: String, to: String },

    #[error("invalid node kind transition: {0}")]
    InvalidTransition(String),

    #[error("empty label: {0}")]
    EmptyLabel(String),

    #[error("core error: {0}")]
    Core(#[from] mevdan_core::CoreError),
}

pub type WorkGraphResult<T> = Result<T, WorkGraphError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_not_found_displays_id() {
        let err = WorkGraphError::NodeNotFound("abc-123".into());
        assert_eq!(err.to_string(), "node not found: abc-123");
    }

    #[test]
    fn duplicate_node_displays_id() {
        let err = WorkGraphError::DuplicateNode("x".into());
        assert_eq!(err.to_string(), "duplicate node id: x");
    }

    #[test]
    fn cycle_detected_displays_path() {
        let err = WorkGraphError::CycleDetected {
            from: "a".into(),
            to: "b".into(),
        };
        assert_eq!(err.to_string(), "edge would create a cycle: a → b");
    }

    #[test]
    fn empty_label_displays() {
        let err = WorkGraphError::EmptyLabel("goal".into());
        assert_eq!(err.to_string(), "empty label: goal");
    }

    #[test]
    fn invalid_edge_displays() {
        let err = WorkGraphError::InvalidEdge("self-loop".into());
        assert_eq!(err.to_string(), "invalid edge: self-loop");
    }
}
