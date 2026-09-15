//! Conexión a SQLite y gestión de migraciones.

use crate::error::{StorageError, StorageResult};
use rusqlite::Connection;
use std::fmt;
use std::path::{Path, PathBuf};

/// Migraciones embebidas en el binario.
const MIGRATIONS: &[(&str, &str)] = &[
    ("0.1.0", include_str!("migrations/V001__initial.sql")),
    ("0.2.0", include_str!("migrations/V002__workgraph.sql")),
    ("0.3.0", include_str!("migrations/V003__checkpoints.sql")),
];

/// Conexión a la base de datos de un proyecto MEVDAN.
pub struct Database {
    conn: Connection,
    path: PathBuf,
}

impl fmt::Debug for Database {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Database")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl Database {
    /// Abre o crea la base de datos en `<project_dir>/.mevdan/mevdan.db`.
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

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Aplica las migraciones pendientes.
    fn apply_migrations(&self) -> StorageResult<()> {
        let current = self.current_schema_version()?;

        match current {
            None => {
                // BD nueva: aplicar todas las migraciones.
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
                // BD existente: aplicar solo las migraciones con
                // versión mayor que la actual.
                for (version, sql) in MIGRATIONS {
                    if *version > v.as_str() {
                        self.conn.execute_batch(sql).map_err(|e| {
                            StorageError::Migration(format!(
                                "failed to apply migration {}: {}",
                                version, e
                            ))
                        })?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Lee la versión de esquema actual. `None` si es nueva.
    fn current_schema_version(&self) -> StorageResult<Option<String>> {
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
        assert_eq!(version.as_deref(), Some("0.3.0"));
    }

    #[test]
    fn open_is_idempotent() {
        let dir = setup_project_dir();
        let _db1 = Database::open(dir.path()).unwrap();
        drop(_db1);

        let db2 = Database::open(dir.path()).unwrap();
        let version = db2.current_schema_version().unwrap();
        assert_eq!(version.as_deref(), Some("0.3.0"));
    }

    #[test]
    fn tables_are_created() {
        let dir = setup_project_dir();
        let db = Database::open(dir.path()).unwrap();

        for table in [
            "meta",
            "projects",
            "sessions",
            "events",
            "workgraphs",
            "checkpoints",
        ] {
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
