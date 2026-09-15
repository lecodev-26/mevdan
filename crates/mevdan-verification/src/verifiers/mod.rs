//! Verificadores concretos.
//!
//! Cada verificador implementa `Verifier` y sabe comprobar un tipo
//! concreto de claim:
//!
//! - `FileExistsVerifier` — comprueba que un archivo existe.
//! - `HashVerifier` — compara el hash real con el esperado.
//! - `CommandExitVerifier` — ejecuta un comando y comprueba el exit code.

pub mod command_exit;
pub mod file_exists;
pub mod hash;

pub use command_exit::CommandExitVerifier;
pub use file_exists::FileExistsVerifier;
pub use hash::HashVerifier;
