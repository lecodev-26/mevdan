//! Tool de Git.
//!
//! **No es "cualquier comando git".** Son operaciones específicas
//! validadas y capturadas.
//!
//! ## Operaciones soportadas
//!
//! ### Solo lectura (riesgo LOW)
//! - `status` — estado del repo.
//! - `log` — historial (limitado).
//! - `diff` — diferencias.
//! - `branch_list` — listar ramas.
//! - `show` — mostrar un objeto.
//!
//! ### Escritura (riesgo MEDIUM)
//! - `add` — añadir archivos al stage.
//! - `commit` — crear commit.
//! - `branch_create` — crear rama.
//! - `checkout` — cambiar rama o commit.
//! - `restore` — restaurar archivos.
//!
//! ### NO soportado (todavía)
//! - `push` — riesgo CRITICAL, requiere gestión de credenciales.
//! - `reset --hard`, `clean -fd`, `rebase` — destructivos o complejos.
//!
//! ## Reglas
//!
//! 1. El `cwd` debe estar dentro de un repositorio Git válido.
//! 2. Se ejecuta con `--no-pager` para evitar bloqueos interactivos.
//! 3. Los commits usan un autor configurable (por defecto: MEVDAN).
//! 4. Sin interacción: no se piden credenciales, no se abren editores.

pub mod tool;

pub use tool::{GitResult, GitTool};
