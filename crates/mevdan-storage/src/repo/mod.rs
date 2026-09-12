//! Repositorios de acceso a datos.
//!
//! Cada repo es un módulo con funciones libres que reciben `&Connection`.
//! No usamos traits `Repository` — eso sería sobre-ingeniería en M1.
//!
//! Reglas:
//!   - `event_repo` es APPEND-ONLY. No expone `update` ni `delete`.
//!   - Todos los repos devuelven `StorageResult<T>`.
//!   - Los timestamps se guardan y leen como ISO-8601 UTC.

pub mod event_repo;
pub mod project_repo;
pub mod session_repo;
