//! IDs tipados del dominio de MEVDAN.
//!
//! Regla: cada entidad tiene su propio tipo de ID. Esto evita que el
//! compilador acepte intercambiar IDs por error.
//!
//! Ejemplo del bug que evitamos:
//!     fn link(project: ProjectId, session: SessionId) {}
//!     link(session_id, project_id);   // ❌ no compila
//!
//! Todos los IDs usan UUID v7 (ordenable por tiempo), ideal para el
//! Event Log append-only.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Genera un nuevo ID usando UUID v7 (ordenable por tiempo).
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            /// Devuelve el UUID interno.
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(Uuid::parse_str(s)?))
            }
        }
    };
}

typed_id!(ProjectId);
typed_id!(SessionId);
typed_id!(EventId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_ids_are_unique() {
        let a = ProjectId::new();
        let b = ProjectId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn ids_roundtrip_through_string() {
        let original = ProjectId::new();
        let s = original.to_string();
        let parsed: ProjectId = s.parse().unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn different_id_types_are_not_interchangeable() {
        // Este test es conceptual: el compilador ya lo garantiza.
        // Si intentaras `let _: SessionId = ProjectId::new();`,
        // no compilaría. Aquí solo verificamos que existen los tipos.
        let _p = ProjectId::new();
        let _s = SessionId::new();
        let _e = EventId::new();
    }

    #[test]
    fn ids_serialize_as_transparent_strings() {
        let id = ProjectId::new();
        let json = serde_json::to_string(&id).unwrap();
        // Debe ser un string entre comillas, no un objeto.
        assert!(json.starts_with('"'));
        assert!(json.ends_with('"'));
        let back: ProjectId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn invalid_uuid_string_fails() {
        let result: Result<ProjectId, _> = "not-a-uuid".parse();
        assert!(result.is_err());
    }
}
