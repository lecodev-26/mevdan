//! Entidad `Project` del dominio de MEVDAN.
//!
//! Un `Project` es la unidad de trabajo del usuario. Contiene la
//! configuración, el Work Graph (en fases futuras) y los checkpoints.
//!
//! Regla desde el día 1: `schema_version` y `mevdan_version` viven
//! aquí. Nunca se eliminan. Si el formato persistido cambia de forma
//! incompatible, se incrementa `schema_version`.

use crate::ids::ProjectId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Versión actual del esquema de datos de un `Project`.
///
/// Cualquier cambio incompatible con versiones previas DEBE
/// incrementar esta constante. Ejemplo: `"0.1.0"` -> `"0.2.0"`.
pub const PROJECT_SCHEMA_VERSION: &str = "0.1.0";

/// Entidad principal del dominio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub schema_version: String,
    pub mevdan_version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub config: ProjectConfig,
}

/// Configuración del proyecto. Extensible sin romper compatibilidad.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Proveedor por defecto para nuevas sesiones (Fase 3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_provider: Option<String>,

    /// Modelo por defecto para nuevas sesiones (Fase 3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,

    /// Políticas de permisos (Fase 6). Vacío por ahora.
    #[serde(default, flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl Project {
    /// Crea un `Project` nuevo con la versión de esquema actual.
    pub fn new(name: impl Into<String>, mevdan_version: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: ProjectId::new(),
            name: name.into(),
            schema_version: PROJECT_SCHEMA_VERSION.to_string(),
            mevdan_version: mevdan_version.into(),
            created_at: now,
            updated_at: now,
            config: ProjectConfig::default(),
        }
    }

    /// Actualiza `updated_at` al momento actual.
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_project_has_schema_version() {
        let p = Project::new("demo", "0.1.0");
        assert_eq!(p.schema_version, PROJECT_SCHEMA_VERSION);
        assert_eq!(p.name, "demo");
        assert_eq!(p.mevdan_version, "0.1.0");
    }

    #[test]
    fn new_project_has_timestamps_equal() {
        let p = Project::new("demo", "0.1.0");
        assert_eq!(p.created_at, p.updated_at);
    }

    #[test]
    fn touch_updates_updated_at() {
        let mut p = Project::new("demo", "0.1.0");
        let original = p.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(5));
        p.touch();
        assert!(p.updated_at > original);
        assert_eq!(p.created_at, original);
    }

    #[test]
    fn project_roundtrips_through_json() {
        let p = Project::new("demo", "0.1.0");
        let s = serde_json::to_string(&p).unwrap();
        let back: Project = serde_json::from_str(&s).unwrap();
        assert_eq!(p.id, back.id);
        assert_eq!(p.name, back.name);
        assert_eq!(p.schema_version, back.schema_version);
        assert_eq!(p.created_at, back.created_at);
    }

    #[test]
    fn config_extra_fields_do_not_break_deserialization() {
        // Simula un JSON con un campo desconocido en config.
        let json = r#"{
            "id": "0191a3f2-0000-7000-8000-000000000000",
            "name": "demo",
            "schema_version": "0.1.0",
            "mevdan_version": "0.1.0",
            "created_at": "2026-09-12T10:00:00Z",
            "updated_at": "2026-09-12T10:00:00Z",
            "config": {
                "future_field": "some_value",
                "another": 42
            }
        }"#;
        let p: Project = serde_json::from_str(json).unwrap();
        assert_eq!(p.name, "demo");
        assert!(p.config.extra.contains_key("future_field"));
        assert!(p.config.extra.contains_key("another"));
    }
}
