//! Sandbox de filesystem.
//!
//! Confina todas las operaciones a un directorio raíz. Cualquier ruta
//! que intente salir del root (incluyendo vía symlinks o `..`) es
//! rechazada.
//!
//! ## Comportamiento cross-platform
//!
//! - Cualquier path que empiece por `/` se interpreta como
//!   **relativo al root del sandbox**, independientemente del OS.
//!   Esto incluye Windows, donde `/foo` técnicamente no es absoluto
//!   pero lo tratamos como si lo fuera.
//! - Los paths con prefijo de disco en Windows (`C:\foo`) se
//!   interpretan también como relativos al root, cogiendo solo los
//!   componentes `Normal`.

use crate::error::{ToolError, ToolResult};
use std::path::{Component, Path, PathBuf};

/// Confinamiento de operaciones a un directorio raíz.
#[derive(Debug, Clone)]
pub struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    /// Crea un sandbox a partir de un directorio raíz.
    pub fn new(root: impl AsRef<Path>) -> ToolResult<Self> {
        let root = root.as_ref();
        if !root.exists() {
            return Err(ToolError::PathNotFound(format!(
                "sandbox root does not exist: {}",
                root.display()
            )));
        }
        let canonical = root.canonicalize()?;
        Ok(Self { root: canonical })
    }

    /// Root del sandbox (canonicalizado).
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resuelve una ruta dentro del sandbox.
    ///
    /// Reglas:
    /// - Cualquier path con `/` inicial se interpreta como relativo al
    ///   root (funciona igual en Unix y Windows).
    /// - Paths con prefijo de disco (`C:\`) también se interpretan como
    ///   relativos, cogiendo solo los componentes `Normal`.
    /// - Se rechazan componentes `..` explícitos.
    /// - Se verifica que la ruta (o su ancestro existente más cercano)
    ///   esté dentro del root.
    pub fn resolve(&self, path: impl AsRef<Path>) -> ToolResult<PathBuf> {
        let path = path.as_ref();

        // Convertimos a relativo al root.
        let path_str = path.to_string_lossy();
        let relative: PathBuf = if let Some(stripped) = path_str.strip_prefix('/') {
            // `/foo` → `foo` (válido tanto en Unix como en Windows).
            PathBuf::from(stripped)
        } else if path.is_absolute() {
            // Windows: `C:\foo` → `foo`. Unix: esto nunca entra porque
            // todo path absoluto en Unix empieza por `/` (ya cogido
            // arriba).
            path.components()
                .filter_map(|c| match c {
                    Component::Normal(s) => Some(s),
                    _ => None,
                })
                .collect()
        } else {
            path.to_path_buf()
        };

        // Defensa: rechazamos `..` en cualquier parte.
        if relative
            .components()
            .any(|c| matches!(c, Component::ParentDir))
        {
            return Err(ToolError::PathEscapesSandbox(path.display().to_string()));
        }

        // Construimos el path completo.
        let joined = self.root.join(&relative);

        // Verificamos seguridad: el ancestro existente más cercano debe
        // estar dentro del root.
        if joined.exists() {
            let canonical = joined.canonicalize()?;
            self.check_inside(&canonical, path)?;
            return Ok(canonical);
        }

        let existing_ancestor = find_existing_ancestor(&joined);
        match existing_ancestor {
            Some(ancestor) => {
                let canonical_ancestor = ancestor.canonicalize()?;
                self.check_inside(&canonical_ancestor, path)?;
            }
            None => {
                return Err(ToolError::PathNotFound(format!(
                    "no existing ancestor for: {}",
                    path.display()
                )));
            }
        }

        Ok(joined)
    }

    /// Verifica que una ruta canonicalizada esté dentro del root.
    fn check_inside(&self, canonical: &Path, original: impl AsRef<Path>) -> ToolResult<()> {
        if !canonical.starts_with(&self.root) {
            return Err(ToolError::PathEscapesSandbox(
                original.as_ref().display().to_string(),
            ));
        }
        Ok(())
    }
}

/// Encuentra el ancestro existente más cercano de un path.
fn find_existing_ancestor(path: &Path) -> Option<&Path> {
    let mut current = Some(path);
    while let Some(p) = current {
        if p.exists() {
            return Some(p);
        }
        current = p.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_sandbox() -> (TempDir, Sandbox) {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("existing.txt"), "hello").unwrap();
        fs::create_dir_all(dir.path().join("subdir")).unwrap();
        fs::write(dir.path().join("subdir/inside.txt"), "world").unwrap();

        let sandbox = Sandbox::new(dir.path()).unwrap();
        (dir, sandbox)
    }

    #[test]
    fn new_fails_if_root_does_not_exist() {
        let result = Sandbox::new("/nonexistent/path/xyz123");
        assert!(result.is_err());
    }

    #[test]
    fn new_succeeds_with_existing_root() {
        let dir = TempDir::new().unwrap();
        let sandbox = Sandbox::new(dir.path()).unwrap();
        assert!(sandbox.root().exists());
    }

    #[test]
    fn resolve_relative_file() {
        let (_dir, sandbox) = setup_sandbox();
        let path = sandbox.resolve("existing.txt").unwrap();
        assert!(path.ends_with("existing.txt"));
        assert!(path.exists());
    }

    #[test]
    fn resolve_nested_file() {
        let (_dir, sandbox) = setup_sandbox();
        let path = sandbox.resolve("subdir/inside.txt").unwrap();
        assert!(path.exists());
        assert!(path.ends_with("subdir/inside.txt"));
    }

    #[test]
    fn resolve_blocks_parent_dir_traversal() {
        let (_dir, sandbox) = setup_sandbox();
        let result = sandbox.resolve("../etc/passwd");
        assert!(result.is_err());
        assert!(matches!(result, Err(ToolError::PathEscapesSandbox(_))));
    }

    #[test]
    fn resolve_blocks_double_parent_traversal() {
        let (_dir, sandbox) = setup_sandbox();
        let result = sandbox.resolve("../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn resolve_blocks_embedded_traversal() {
        let (_dir, sandbox) = setup_sandbox();
        let result = sandbox.resolve("subdir/../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn resolve_treats_absolute_path_as_relative_to_root() {
        let (_dir, sandbox) = setup_sandbox();
        // `/existing.txt` → `root/existing.txt` (cross-platform).
        let path = sandbox.resolve("/existing.txt").unwrap();
        assert!(path.exists());
        assert!(path.starts_with(sandbox.root()));
    }

    #[test]
    fn resolve_new_file_inside_existing_dir() {
        let (_dir, sandbox) = setup_sandbox();
        let path = sandbox.resolve("subdir/new_file.txt").unwrap();
        assert!(!path.exists());
        assert!(path.starts_with(sandbox.root()));
        assert!(path.ends_with("subdir/new_file.txt"));
    }

    #[test]
    fn resolve_new_file_in_root() {
        let (_dir, sandbox) = setup_sandbox();
        let path = sandbox.resolve("brand_new.txt").unwrap();
        assert!(!path.exists());
        assert!(path.starts_with(sandbox.root()));
    }

    #[test]
    fn resolve_preserves_intermediate_dirs_for_new_file() {
        let (_dir, sandbox) = setup_sandbox();
        let path = sandbox.resolve("a/b/c/deep.txt").unwrap();
        assert!(path.starts_with(sandbox.root()));
        assert!(path.ends_with("a/b/c/deep.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_blocks_symlink_escape() {
        use std::os::unix::fs::symlink;
        let dir = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        fs::write(outside.path().join("secret.txt"), "top secret").unwrap();

        symlink(
            outside.path().join("secret.txt"),
            dir.path().join("link_to_outside"),
        )
        .unwrap();

        let sandbox = Sandbox::new(dir.path()).unwrap();
        let result = sandbox.resolve("link_to_outside");
        assert!(result.is_err(), "symlink escape should be blocked");
    }

    #[test]
    fn resolve_simple_filename() {
        let (_dir, sandbox) = setup_sandbox();
        let path = sandbox.resolve("new.txt").unwrap();
        assert!(path.starts_with(sandbox.root()));
    }
}
