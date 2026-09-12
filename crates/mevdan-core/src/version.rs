//! Helpers de versión y compatibilidad de esquemas.
//!
//! Cuando un proyecto se abre, comparamos:
//!   - `project.schema_version` (lo que dice el proyecto)
//!   - `PROJECT_SCHEMA_VERSION` (lo que soporta esta build)
//!
//! Si el proyecto está en versión superior → no podemos abrirlo.
//! Si el proyecto está en versión inferior → migración necesaria.

use crate::error::{CoreError, CoreResult};
use semver::Version;

/// Parsea una versión semver, devolviendo un error de dominio si falla.
pub fn parse(s: &str) -> CoreResult<Version> {
    Version::parse(s).map_err(|_| CoreError::InvalidVersion(s.to_string()))
}

/// Verifica que una versión de proyecto sea compatible con la actual.
///
/// Reglas:
///   - Si `project_version` > `current_version` → incompatible.
///   - Si `project_version` == `current_version` → compatible.
///   - Si `project_version` < `current_version` → compatible pero
///     requiere migración (lo gestionará `mevdan-storage` en el futuro).
pub fn check_compatibility(project_version: &str, current_version: &str) -> CoreResult<()> {
    let project = parse(project_version)?;
    let current = parse(current_version)?;

    if project > current {
        return Err(CoreError::SchemaVersionMismatch {
            expected: current.to_string(),
            found: project.to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_version() {
        let v = parse("0.1.0").unwrap();
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 1);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn parse_invalid_version_fails() {
        let err = parse("not-a-version").unwrap_err();
        matches!(err, CoreError::InvalidVersion(_));
    }

    #[test]
    fn same_version_is_compatible() {
        assert!(check_compatibility("0.1.0", "0.1.0").is_ok());
    }

    #[test]
    fn older_project_version_is_compatible() {
        // Proyecto en 0.0.9, nosotros en 0.1.0 → migración necesaria pero OK.
        assert!(check_compatibility("0.0.9", "0.1.0").is_ok());
    }

    #[test]
    fn newer_project_version_is_incompatible() {
        // Proyecto en 0.2.0, nosotros en 0.1.0 → no podemos abrirlo.
        let err = check_compatibility("0.2.0", "0.1.0").unwrap_err();
        match err {
            CoreError::SchemaVersionMismatch { expected, found } => {
                assert_eq!(expected, "0.1.0");
                assert_eq!(found, "0.2.0");
            }
            _ => panic!("expected SchemaVersionMismatch"),
        }
    }
}
