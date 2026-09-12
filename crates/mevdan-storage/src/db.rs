//! Conexión a SQLite y gestión de migraciones.
//!
//! Reglas:
//!   1. Cada conexión activa `PRAGMA foreign_keys = ON` y `WAL`.
//!   2. Las migraciones son idempotentes: correrlas dos veces es seguro.
//!   3. La versión de esquema vive en la tabla `meta`.
//!
//! Este archivo NO conoce el dominio. Solo gestiona la conexión.

use crate::error::{StorageError, StorageResult};
use rusqlite::Connection;
use std::fmt;
use std::path::{Path, PathBuf};

/// Migraciones embebidas en el binario.
///
/// Cada migración es un `(version, sql)` donde `version` es una
/// cadena semver-like. Se aplican en orden lexicográfico.
///
/// Regla: AÑADIR migraciones al final. Nunca modificar una existente.
const MIGRATIONS: &[(&str, &str)] = &[(
    "0.1.0",
    include_str!("migrations/V001__initial.sql"),
)];

/// Conexión a la base de datos de un proyecto MEVDAN.
pub struct Database {
    conn: Connection,
    path: PathBuf,
}

// `rusqlite::Connection` no implementa Debug, así que lo hacemos a mano
// mostrando solo la ruta (nunca contenido de la DB).
impl fmt::Debug for Database {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Database")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl Database {
    /// Abre o crea la base de datos en `<project_dir>/.mevdan/mevdan.db`.
    ///
    /// Si la base no existe, la crea y aplica todas las migraciones.
    /// Si existe, verifica que esté al día.
    pub fn open(project_dir: &Path) -> StorageResult<Self> {
        let mevdan_dir = project_dir.join(".mevdan");
        if !mevdan_dir.exists() {
            return Err(StorageError::InvalidProjectDir(format!(
                "no .mevdan/ directory in {}",
                project_dir.display()
            )));
        }

        let db_path = mevdan_dir.join("mevdan.db");
        let conn = Connection::open(&db_path)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;

        let db = Self {
            conn,
            path: db_path,
        };

        db.apply_migrations()?;

        Ok(db)
    }

    /// Acceso a la conexión subyacente.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Ruta de la base de datos.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Aplica las migraciones pendientes.
    ///
    /// - Si la tabla `meta` no existe → primera vez, aplica todo.
    /// - Si existe → aplica solo las migraciones con versión superior.
    fn apply_migrations(&self) -> StorageResult<()> {
        let current = self.current_schema_version()?;

        match current {
            None => {
                // Primera vez: aplicar todas las migraciones.
                for (version, sql) in MIGRATIONS {
                    self.conn.execute_batch(sql).map_err(|e| {
                        StorageError::Migration(format!(
                            "failed to apply migration {}: {}",
                            version, e
                        ))
                    })?;
                }
            }
            Some(v) => {
                // Futuro: aplicar migraciones posteriores a `v`.
                // En M1 solo existe V001, así que verificamos que sea la actual.
                let latest = MIGRATIONS.last().map(|(v, _)| *v).unwrap_or("0.0.0");
                if v != latest {
                    return Err(StorageError::Migration(format!(
                        "schema version mismatch: db has {}, code expects {}",
                        v, latest
                    )));
                }
            }
        }

        Ok(())
    }

    /// Lee la versión de esquema actual de la base. `None` si es nueva.
    fn current_schema_version(&self) -> StorageResult<Option<String>> {
        // Verifica si existe la tabla `meta`.
        let exists: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='meta'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|n| n > 0)?;

        if !exists {
            return Ok(None);
        }

        let version: String = self.conn.query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )?;

        Ok(Some(version))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_project_dir() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".mevdan")).unwrap();
        dir
    }

    #[test]
    fn open_fails_without_mevdan_dir() {
        let dir = TempDir::new().unwrap();
        let err = Database::open(dir.path()).unwrap_err();
        match err {
            StorageError::InvalidProjectDir(_) => {}
            _ => panic!("expected InvalidProjectDir, got {:?}", err),
        }
    }

    #[test]
    fn open_creates_db_and_applies_migrations() {
        let dir = setup_project_dir();
        let db = Database::open(dir.path()).unwrap();
        assert!(db.path().exists());

        let version = db.current_schema_version().unwrap();
        assert_eq!(version.as_deref(), Some("0.1.0"));
    }

    #[test]
    fn open_is_idempotent() {
        let dir = setup_project_dir();
        let _db1 = Database::open(dir.path()).unwrap();
        drop(_db1);

        // Segunda apertura no debe fallar ni re-aplicar migraciones.
        let db2 = Database::open(dir.path()).unwrap();
        let version = db2.current_schema_version().unwrap();
        assert_eq!(version.as_deref(), Some("0.1.0"));
    }

    #[test]
    fn tables_are_created() {
        let dir = setup_project_dir();
        let db = Database::open(dir.path()).unwrap();

        for table in ["meta", "projects", "sessions", "events"] {
            let exists: i64 = db
                .connection()
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "table {} should exist", table);
        }
    }

    #[test]
    fn foreign_keys_are_enabled() {
        let dir = setup_project_dir();
        let db = Database::open(dir.path()).unwrap();

        let fk: i64 = db
            .connection()
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fk, 1);
    }
}
