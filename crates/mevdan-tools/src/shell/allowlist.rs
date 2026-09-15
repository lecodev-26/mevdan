//! Lista blanca de comandos permitidos.
//!
//! Por defecto, MEVDAN permite comandos "seguros" y de desarrollo:
//! compiladores, gestores de paquetes, herramientas de git, tests...
//!
//! El usuario puede **ampliar** la lista blanca explícitamente. Pero
//! nunca se permite `sudo`, `rm -rf /`, etc.

use std::collections::BTreeSet;

/// Lista blanca de comandos.
#[derive(Debug, Clone)]
pub struct AllowList {
    commands: BTreeSet<String>,
}

impl AllowList {
    /// Lista vacía.
    pub fn new() -> Self {
        Self {
            commands: BTreeSet::new(),
        }
    }

    /// Lista con los comandos por defecto (seguros para desarrollo).
    pub fn defaults() -> Self {
        let mut list = Self::new();

        // Rust / Cargo
        for cmd in ["cargo", "rustc", "rustup", "rustfmt", "clippy-driver"] {
            list.allow(cmd);
        }

        // Node / JS
        for cmd in [
            "node", "npm", "pnpm", "yarn", "bun", "tsc", "eslint", "prettier",
        ] {
            list.allow(cmd);
        }

        // Python
        for cmd in ["python", "python3", "pip", "pip3", "pytest", "ruff", "mypy"] {
            list.allow(cmd);
        }

        // Go
        for cmd in ["go", "gofmt"] {
            list.allow(cmd);
        }

        // Git (read operations; write se controla en el tool de git)
        list.allow("git");

        // Build tools genéricos
        for cmd in ["make", "cmake", "ninja", "just"] {
            list.allow(cmd);
        }

        // Coreutils seguros (lectura)
        for cmd in [
            "ls", "cat", "head", "tail", "wc", "find", "grep", "sed", "awk", "echo", "pwd",
            "which", "file",
        ] {
            list.allow(cmd);
        }

        // Módulos de MEVDAN
        list.allow("mevdan");

        list
    }

    /// Añade un comando a la lista blanca.
    pub fn allow(&mut self, cmd: impl Into<String>) {
        self.commands.insert(cmd.into());
    }

    /// Elimina un comando de la lista blanca.
    pub fn deny(&mut self, cmd: &str) {
        self.commands.remove(cmd);
    }

    /// ¿Está permitido este comando?
    pub fn is_allowed(&self, cmd: &str) -> bool {
        self.commands.contains(cmd)
    }

    /// Lista todos los comandos permitidos.
    pub fn list(&self) -> Vec<&str> {
        self.commands.iter().map(|s| s.as_str()).collect()
    }

    /// Número de comandos permitidos.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// ¿Está vacía?
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

impl Default for AllowList {
    fn default() -> Self {
        Self::defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_empty() {
        let l = AllowList::new();
        assert!(l.is_empty());
        assert_eq!(l.len(), 0);
    }

    #[test]
    fn defaults_has_common_commands() {
        let l = AllowList::defaults();
        assert!(l.is_allowed("cargo"));
        assert!(l.is_allowed("git"));
        assert!(l.is_allowed("python3"));
        assert!(l.is_allowed("node"));
        assert!(l.is_allowed("npm"));
        assert!(l.is_allowed("make"));
        assert!(l.is_allowed("ls"));
    }

    #[test]
    fn defaults_rejects_dangerous_commands() {
        let l = AllowList::defaults();
        assert!(!l.is_allowed("sudo"));
        assert!(!l.is_allowed("rm"));
        assert!(!l.is_allowed("curl"));
        assert!(!l.is_allowed("wget"));
        assert!(!l.is_allowed("sh"));
        assert!(!l.is_allowed("bash"));
        assert!(!l.is_allowed("dd"));
        assert!(!l.is_allowed("mkfs"));
    }

    #[test]
    fn allow_adds_command() {
        let mut l = AllowList::new();
        assert!(!l.is_allowed("mycmd"));
        l.allow("mycmd");
        assert!(l.is_allowed("mycmd"));
    }

    #[test]
    fn deny_removes_command() {
        let mut l = AllowList::defaults();
        assert!(l.is_allowed("cargo"));
        l.deny("cargo");
        assert!(!l.is_allowed("cargo"));
    }

    #[test]
    fn list_is_sorted() {
        let mut l = AllowList::new();
        l.allow("zeta");
        l.allow("alpha");
        l.allow("beta");
        assert_eq!(l.list(), vec!["alpha", "beta", "zeta"]);
    }

    #[test]
    fn default_trait_uses_defaults() {
        let l = AllowList::default();
        assert!(l.is_allowed("cargo"));
    }
}
