//! Tool de filesystem con sandbox.
//!
//! Este módulo implementa:
//! - `Sandbox` — confina todas las operaciones a un directorio root.
//! - `FilesystemTool` — operaciones: read, list, write, exists, delete.
//!
//! ## Regla de seguridad
//!
//! NINGUNA operación puede salir del directorio raíz del sandbox. Se
//! canonicalizan las rutas y se verifica que estén dentro del root.
//! Path traversal (`../../../etc/passwd`) queda bloqueado.

pub mod sandbox;
pub mod tool;

pub use sandbox::Sandbox;
pub use tool::FilesystemTool;
