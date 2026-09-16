//! Detección de build systems.
//!
//! Detecta el sistema de construcción de un proyecto mirando los
//! archivos presentes en la raíz.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Sistema de construcción.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildSystem {
    Cargo,
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Pip,
    Poetry,
    Uv,
    GoModules,
    Maven,
    Gradle,
    Make,
    Cmake,
    Ninja,
    Unknown,
}

impl BuildSystem {
    pub fn name(&self) -> &'static str {
        match self {
            BuildSystem::Cargo => "cargo",
            BuildSystem::Npm => "npm",
            BuildSystem::Pnpm => "pnpm",
            BuildSystem::Yarn => "yarn",
            BuildSystem::Bun => "bun",
            BuildSystem::Pip => "pip",
            BuildSystem::Poetry => "poetry",
            BuildSystem::Uv => "uv",
            BuildSystem::GoModules => "go_modules",
            BuildSystem::Maven => "maven",
            BuildSystem::Gradle => "gradle",
            BuildSystem::Make => "make",
            BuildSystem::Cmake => "cmake",
            BuildSystem::Ninja => "ninja",
            BuildSystem::Unknown => "unknown",
        }
    }

    /// Comando típico para ejecutar tests.
    pub fn test_command(&self) -> Option<&'static str> {
        match self {
            BuildSystem::Cargo => Some("cargo test"),
            BuildSystem::Npm => Some("npm test"),
            BuildSystem::Pnpm => Some("pnpm test"),
            BuildSystem::Yarn => Some("yarn test"),
            BuildSystem::Bun => Some("bun test"),
            BuildSystem::Pip | BuildSystem::Poetry | BuildSystem::Uv => Some("pytest"),
            BuildSystem::GoModules => Some("go test ./..."),
            BuildSystem::Maven => Some("mvn test"),
            BuildSystem::Gradle => Some("gradle test"),
            BuildSystem::Make => Some("make test"),
            BuildSystem::Cmake | BuildSystem::Ninja | BuildSystem::Unknown => None,
        }
    }

    /// Comando típico para construir.
    pub fn build_command(&self) -> Option<&'static str> {
        match self {
            BuildSystem::Cargo => Some("cargo build"),
            BuildSystem::Npm => Some("npm run build"),
            BuildSystem::Pnpm => Some("pnpm build"),
            BuildSystem::Yarn => Some("yarn build"),
            BuildSystem::Bun => Some("bun run build"),
            BuildSystem::GoModules => Some("go build ./..."),
            BuildSystem::Maven => Some("mvn package"),
            BuildSystem::Gradle => Some("gradle build"),
            BuildSystem::Make => Some("make"),
            BuildSystem::Cmake => Some("cmake --build ."),
            BuildSystem::Ninja => Some("ninja"),
            BuildSystem::Pip | BuildSystem::Poetry | BuildSystem::Uv | BuildSystem::Unknown => None,
        }
    }
}

/// Detector de build systems.
#[derive(Debug, Default)]
pub struct BuildSystemDetector;

impl BuildSystemDetector {
    pub fn new() -> Self {
        Self
    }

    /// Detecta el build system de un directorio.
    ///
    /// Devuelve el primer match según prioridad. Si no hay ninguno,
    /// devuelve `Unknown`.
    pub fn detect(&self, project_root: &Path) -> BuildSystem {
        if !project_root.is_dir() {
            return BuildSystem::Unknown;
        }

        // Orden de prioridad: los más específicos primero.
        if project_root.join("Cargo.toml").exists() {
            return BuildSystem::Cargo;
        }
        if project_root.join("bun.lockb").exists() || project_root.join("bun.lock").exists() {
            return BuildSystem::Bun;
        }
        if project_root.join("pnpm-lock.yaml").exists() {
            return BuildSystem::Pnpm;
        }
        if project_root.join("yarn.lock").exists() {
            return BuildSystem::Yarn;
        }
        if project_root.join("package.json").exists() {
            return BuildSystem::Npm;
        }
        if project_root.join("poetry.lock").exists() || project_root.join("pyproject.toml").exists()
        {
            // Distinguimos poetry vs uv por presencia de poetry.lock.
            if project_root.join("poetry.lock").exists() {
                return BuildSystem::Poetry;
            }
            // `uv.lock` es específico de uv.
            if project_root.join("uv.lock").exists() {
                return BuildSystem::Uv;
            }
            return BuildSystem::Poetry;
        }
        if project_root.join("requirements.txt").exists() || project_root.join("setup.py").exists()
        {
            return BuildSystem::Pip;
        }
        if project_root.join("go.mod").exists() {
            return BuildSystem::GoModules;
        }
        if project_root.join("pom.xml").exists() {
            return BuildSystem::Maven;
        }
        if project_root.join("build.gradle").exists()
            || project_root.join("build.gradle.kts").exists()
        {
            return BuildSystem::Gradle;
        }
        if project_root.join("CMakeLists.txt").exists() {
            return BuildSystem::Cmake;
        }
        if project_root.join("build.ninja").exists() {
            return BuildSystem::Ninja;
        }
        if project_root.join("Makefile").exists() || project_root.join("makefile").exists() {
            return BuildSystem::Make;
        }

        BuildSystem::Unknown
    }

    /// Detecta **todos** los build systems presentes (puede haber varios
    /// en monorepos).
    pub fn detect_all(&self, project_root: &Path) -> Vec<BuildSystem> {
        if !project_root.is_dir() {
            return Vec::new();
        }

        let mut found = Vec::new();

        let checks: &[(&str, BuildSystem)] = &[
            ("Cargo.toml", BuildSystem::Cargo),
            ("bun.lockb", BuildSystem::Bun),
            ("bun.lock", BuildSystem::Bun),
            ("pnpm-lock.yaml", BuildSystem::Pnpm),
            ("yarn.lock", BuildSystem::Yarn),
            ("package.json", BuildSystem::Npm),
            ("poetry.lock", BuildSystem::Poetry),
            ("uv.lock", BuildSystem::Uv),
            ("pyproject.toml", BuildSystem::Poetry),
            ("requirements.txt", BuildSystem::Pip),
            ("setup.py", BuildSystem::Pip),
            ("go.mod", BuildSystem::GoModules),
            ("pom.xml", BuildSystem::Maven),
            ("build.gradle", BuildSystem::Gradle),
            ("build.gradle.kts", BuildSystem::Gradle),
            ("CMakeLists.txt", BuildSystem::Cmake),
            ("build.ninja", BuildSystem::Ninja),
            ("Makefile", BuildSystem::Make),
        ];

        for (file, system) in checks {
            if project_root.join(file).exists() && !found.contains(system) {
                found.push(*system);
            }
        }

        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn tmp_with_file(name: &str) -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(name), "").unwrap();
        dir
    }

    #[test]
    fn detect_cargo() {
        let dir = tmp_with_file("Cargo.toml");
        assert_eq!(
            BuildSystemDetector::new().detect(dir.path()),
            BuildSystem::Cargo
        );
    }

    #[test]
    fn detect_npm() {
        let dir = tmp_with_file("package.json");
        assert_eq!(
            BuildSystemDetector::new().detect(dir.path()),
            BuildSystem::Npm
        );
    }

    #[test]
    fn detect_pnpm_over_npm() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), "").unwrap();
        fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
        assert_eq!(
            BuildSystemDetector::new().detect(dir.path()),
            BuildSystem::Pnpm
        );
    }

    #[test]
    fn detect_yarn_over_npm() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), "").unwrap();
        fs::write(dir.path().join("yarn.lock"), "").unwrap();
        assert_eq!(
            BuildSystemDetector::new().detect(dir.path()),
            BuildSystem::Yarn
        );
    }

    #[test]
    fn detect_bun() {
        let dir = tmp_with_file("bun.lockb");
        assert_eq!(
            BuildSystemDetector::new().detect(dir.path()),
            BuildSystem::Bun
        );
    }

    #[test]
    fn detect_pip() {
        let dir = tmp_with_file("requirements.txt");
        assert_eq!(
            BuildSystemDetector::new().detect(dir.path()),
            BuildSystem::Pip
        );
    }

    #[test]
    fn detect_poetry() {
        let dir = tmp_with_file("poetry.lock");
        assert_eq!(
            BuildSystemDetector::new().detect(dir.path()),
            BuildSystem::Poetry
        );
    }

    #[test]
    fn detect_uv() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("pyproject.toml"), "").unwrap();
        fs::write(dir.path().join("uv.lock"), "").unwrap();
        assert_eq!(
            BuildSystemDetector::new().detect(dir.path()),
            BuildSystem::Uv
        );
    }

    #[test]
    fn detect_go_modules() {
        let dir = tmp_with_file("go.mod");
        assert_eq!(
            BuildSystemDetector::new().detect(dir.path()),
            BuildSystem::GoModules
        );
    }

    #[test]
    fn detect_maven() {
        let dir = tmp_with_file("pom.xml");
        assert_eq!(
            BuildSystemDetector::new().detect(dir.path()),
            BuildSystem::Maven
        );
    }

    #[test]
    fn detect_gradle() {
        let dir = tmp_with_file("build.gradle");
        assert_eq!(
            BuildSystemDetector::new().detect(dir.path()),
            BuildSystem::Gradle
        );
    }

    #[test]
    fn detect_cmake() {
        let dir = tmp_with_file("CMakeLists.txt");
        assert_eq!(
            BuildSystemDetector::new().detect(dir.path()),
            BuildSystem::Cmake
        );
    }

    #[test]
    fn detect_make() {
        let dir = tmp_with_file("Makefile");
        assert_eq!(
            BuildSystemDetector::new().detect(dir.path()),
            BuildSystem::Make
        );
    }

    #[test]
    fn detect_unknown_for_empty_dir() {
        let dir = TempDir::new().unwrap();
        assert_eq!(
            BuildSystemDetector::new().detect(dir.path()),
            BuildSystem::Unknown
        );
    }

    #[test]
    fn detect_all_returns_multiple() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        fs::write(dir.path().join("package.json"), "").unwrap();
        fs::write(dir.path().join("Makefile"), "").unwrap();

        let all = BuildSystemDetector::new().detect_all(dir.path());
        assert_eq!(all.len(), 3);
        assert!(all.contains(&BuildSystem::Cargo));
        assert!(all.contains(&BuildSystem::Npm));
        assert!(all.contains(&BuildSystem::Make));
    }

    #[test]
    fn detect_all_no_duplicates() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("requirements.txt"), "").unwrap();
        fs::write(dir.path().join("setup.py"), "").unwrap();

        let all = BuildSystemDetector::new().detect_all(dir.path());
        // Ambos son `Pip` → solo un resultado.
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn detect_on_missing_dir_returns_unknown() {
        let detector = BuildSystemDetector::new();
        assert_eq!(
            detector.detect(Path::new("/definitely/not/real")),
            BuildSystem::Unknown
        );
    }

    #[test]
    fn test_commands() {
        assert_eq!(BuildSystem::Cargo.test_command(), Some("cargo test"));
        assert_eq!(BuildSystem::Npm.test_command(), Some("npm test"));
        assert_eq!(BuildSystem::GoModules.test_command(), Some("go test ./..."));
        assert_eq!(BuildSystem::Unknown.test_command(), None);
    }

    #[test]
    fn build_commands() {
        assert_eq!(BuildSystem::Cargo.build_command(), Some("cargo build"));
        assert_eq!(BuildSystem::Npm.build_command(), Some("npm run build"));
        assert_eq!(BuildSystem::Unknown.build_command(), None);
    }

    #[test]
    fn system_names() {
        assert_eq!(BuildSystem::Cargo.name(), "cargo");
        assert_eq!(BuildSystem::GoModules.name(), "go_modules");
        assert_eq!(BuildSystem::Unknown.name(), "unknown");
    }

    #[test]
    fn system_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&BuildSystem::Cargo).unwrap(),
            "\"cargo\""
        );
        assert_eq!(
            serde_json::to_string(&BuildSystem::GoModules).unwrap(),
            "\"go_modules\""
        );
    }
}
