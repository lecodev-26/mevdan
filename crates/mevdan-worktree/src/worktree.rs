//! `Worktree` — espacio de trabajo aislado dentro de un proyecto.
//!
//! Un worktree es una rama de trabajo paralela. Permite probar varios
//! enfoques al mismo tiempo sin colisión.
//!
//! ```text
//! mi-proyecto/                    ← proyecto principal (main)
//! ├── .mevdan/
//! │   ├── worktrees/
//! │   │   ├── approach-a/        ← worktree A
//! │   │   └── approach-b/        ← worktree B
//! │   └── mevdan.db
//! ```

use crate::{
    error::{WorktreeError, WorktreeResult},
    id::WorktreeId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// Nombre reservado del worktree principal.
pub const MAIN_WORKTREE_NAME: &str = "main";

/// Un worktree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worktree {
    pub id: WorktreeId,
    /// Nombre único dentro del proyecto.
    pub name: String,
    /// Ruta del worktree (relativa al proyecto o absoluta).
    pub path: PathBuf,
    /// ¿Es el worktree principal?
    pub is_main: bool,
    /// Descripción opcional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    /// Metadatos libres.
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl Worktree {
    /// Crea un worktree nuevo (no lo materializa en disco).
    pub fn new(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        is_main: bool,
    ) -> WorktreeResult<Self> {
        let name = name.into();
        validate_name(&name)?;
        Ok(Self {
            id: WorktreeId::new(),
            name,
            path: path.into(),
            is_main,
            description: None,
            created_at: Utc::now(),
            metadata: serde_json::Value::Null,
        })
    }

    /// Crea el worktree principal.
    pub fn main(path: impl Into<PathBuf>) -> WorktreeResult<Self> {
        Self::new(MAIN_WORKTREE_NAME, path, true)
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// ¿Existe la ruta en disco?
    pub fn exists_on_disk(&self) -> bool {
        self.path.exists()
    }
}

/// Valida un nombre de worktree.
pub fn validate_name(name: &str) -> WorktreeResult<()> {
    if name.is_empty() || name.len() > 64 {
        return Err(WorktreeError::InvalidName(name.to_string()));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        return Err(WorktreeError::InvalidName(name.to_string()));
    }
    Ok(())
}

/// Gestor de worktrees de un proyecto.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorktreeManager {
    /// Raíz del proyecto (contiene `.mevdan/`).
    pub project_root: PathBuf,
    /// Worktrees registrados, por nombre.
    pub worktrees: BTreeMap<String, Worktree>,
}

impl WorktreeManager {
    /// Crea un gestor asociado a un proyecto.
    ///
    /// El worktree principal (`main`) apunta a la raíz del proyecto.
    /// Se registra automáticamente si no existe.
    pub fn new(project_root: impl Into<PathBuf>) -> WorktreeResult<Self> {
        let project_root = project_root.into();
        let mut manager = Self {
            project_root,
            worktrees: BTreeMap::new(),
        };
        manager.ensure_main()?;
        Ok(manager)
    }

    /// Directorio donde se guardan los worktrees.
    pub fn worktrees_dir(&self) -> PathBuf {
        self.project_root.join(".mevdan").join("worktrees")
    }

    /// Asegura que el worktree `main` está registrado.
    pub fn ensure_main(&mut self) -> WorktreeResult<()> {
        if !self.worktrees.contains_key(MAIN_WORKTREE_NAME) {
            let main = Worktree::main(self.project_root.clone())?;
            self.worktrees.insert(MAIN_WORKTREE_NAME.to_string(), main);
        }
        Ok(())
    }

    /// Crea un nuevo worktree.
    ///
    /// Crea el directorio en `<project_root>/.mevdan/worktrees/<name>/`.
    pub fn create(
        &mut self,
        name: impl Into<String>,
        description: Option<String>,
    ) -> WorktreeResult<Worktree> {
        let name = name.into();
        validate_name(&name)?;

        if name == MAIN_WORKTREE_NAME {
            return Err(WorktreeError::DuplicateName(name));
        }
        if self.worktrees.contains_key(&name) {
            return Err(WorktreeError::DuplicateName(name));
        }

        let path = self.worktrees_dir().join(&name);
        fs::create_dir_all(&path)?;

        let mut wt = Worktree::new(&name, path, false)?;
        if let Some(d) = description {
            wt = wt.with_description(d);
        }

        self.worktrees.insert(name, wt.clone());
        Ok(wt)
    }

    /// Registra un worktree existente (sin crear directorio).
    pub fn register(&mut self, worktree: Worktree) -> WorktreeResult<()> {
        if self.worktrees.contains_key(&worktree.name) {
            return Err(WorktreeError::DuplicateName(worktree.name));
        }
        self.worktrees.insert(worktree.name.clone(), worktree);
        Ok(())
    }

    /// Devuelve un worktree por nombre.
    pub fn get(&self, name: &str) -> Option<&Worktree> {
        self.worktrees.get(name)
    }

    /// Devuelve un worktree o error.
    pub fn require(&self, name: &str) -> WorktreeResult<&Worktree> {
        self.worktrees
            .get(name)
            .ok_or_else(|| WorktreeError::NotFound(name.to_string()))
    }

    /// Lista todos los worktrees (ordenados por nombre).
    pub fn list(&self) -> Vec<&Worktree> {
        self.worktrees.values().collect()
    }

    /// Número de worktrees.
    pub fn len(&self) -> usize {
        self.worktrees.len()
    }

    /// ¿Está vacío? (nunca, porque `main` siempre existe)
    pub fn is_empty(&self) -> bool {
        self.worktrees.is_empty()
    }

    /// Elimina un worktree (y su directorio si existe).
    pub fn delete(&mut self, name: &str) -> WorktreeResult<Worktree> {
        if name == MAIN_WORKTREE_NAME {
            return Err(WorktreeError::CannotDeleteMain);
        }

        let wt = self
            .worktrees
            .remove(name)
            .ok_or_else(|| WorktreeError::NotFound(name.to_string()))?;

        // Best-effort: borra el directorio si existe y está dentro del
        // directorio de worktrees del proyecto.
        let expected_root = self.worktrees_dir();
        if wt.path.starts_with(&expected_root) && wt.path.exists() {
            let _ = fs::remove_dir_all(&wt.path);
        }

        Ok(wt)
    }

    /// El worktree `main`.
    pub fn main(&self) -> &Worktree {
        // `ensure_main` garantiza que existe.
        self.worktrees.get(MAIN_WORKTREE_NAME).unwrap()
    }

    /// Serializa el gestor a JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserializa desde JSON.
    pub fn from_json(s: &str) -> WorktreeResult<Self> {
        serde_json::from_str(s).map_err(|e| WorktreeError::Serialization(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_manager() -> (TempDir, WorktreeManager) {
        let dir = TempDir::new().unwrap();
        let manager = WorktreeManager::new(dir.path()).unwrap();
        (dir, manager)
    }

    // ─────────────────────────────────────────────
    // Worktree
    // ─────────────────────────────────────────────

    #[test]
    fn worktree_new_creates() {
        let wt = Worktree::new("test", "/tmp/test", false).unwrap();
        assert_eq!(wt.name, "test");
        assert!(!wt.is_main);
        assert!(wt.description.is_none());
    }

    #[test]
    fn worktree_main_is_main() {
        let wt = Worktree::main("/tmp/project").unwrap();
        assert_eq!(wt.name, MAIN_WORKTREE_NAME);
        assert!(wt.is_main);
    }

    #[test]
    fn worktree_validates_name() {
        assert!(Worktree::new("", "/tmp", false).is_err());
        assert!(Worktree::new("Bad Name", "/tmp", false).is_err());
        assert!(Worktree::new("bad/name", "/tmp", false).is_err());
        assert!(Worktree::new("valid-name", "/tmp", false).is_ok());
    }

    #[test]
    fn worktree_with_description() {
        let wt = Worktree::new("test", "/tmp", false)
            .unwrap()
            .with_description("my test worktree");
        assert_eq!(wt.description.as_deref(), Some("my test worktree"));
    }

    #[test]
    fn worktree_serializes() {
        let wt = Worktree::main("/tmp/project").unwrap();
        let json = serde_json::to_string(&wt).unwrap();
        let back: Worktree = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, wt.id);
        assert_eq!(back.name, wt.name);
    }

    // ─────────────────────────────────────────────
    // WorktreeManager
    // ─────────────────────────────────────────────

    #[test]
    fn manager_starts_with_main() {
        let (_dir, manager) = setup_manager();
        assert_eq!(manager.len(), 1);
        assert!(manager.get(MAIN_WORKTREE_NAME).is_some());
        assert!(manager.main().is_main);
    }

    #[test]
    fn manager_create_creates_directory() {
        let (_dir, mut manager) = setup_manager();
        let wt = manager.create("experiment", None).unwrap();

        assert_eq!(wt.name, "experiment");
        assert!(!wt.is_main);
        assert!(wt.path.exists());
        assert!(wt.path.starts_with(manager.worktrees_dir()));
    }

    #[test]
    fn manager_create_validates_name() {
        let (_dir, mut manager) = setup_manager();
        assert!(manager.create("Bad Name", None).is_err());
        assert!(manager.create("", None).is_err());
    }

    #[test]
    fn manager_rejects_duplicate() {
        let (_dir, mut manager) = setup_manager();
        manager.create("test", None).unwrap();
        let err = manager.create("test", None).unwrap_err();
        assert!(matches!(err, WorktreeError::DuplicateName(_)));
    }

    #[test]
    fn manager_rejects_main_name() {
        let (_dir, mut manager) = setup_manager();
        let err = manager.create(MAIN_WORKTREE_NAME, None).unwrap_err();
        assert!(matches!(err, WorktreeError::DuplicateName(_)));
    }

    #[test]
    fn manager_create_with_description() {
        let (_dir, mut manager) = setup_manager();
        let wt = manager.create("x", Some("experiment".into())).unwrap();
        assert_eq!(wt.description.as_deref(), Some("experiment"));
    }

    #[test]
    fn manager_get_returns_none_when_missing() {
        let (_dir, manager) = setup_manager();
        assert!(manager.get("nope").is_none());
    }

    #[test]
    fn manager_require_fails_when_missing() {
        let (_dir, manager) = setup_manager();
        let err = manager.require("nope").unwrap_err();
        assert!(matches!(err, WorktreeError::NotFound(_)));
    }

    #[test]
    fn manager_list_includes_main() {
        let (_dir, mut manager) = setup_manager();
        manager.create("a", None).unwrap();
        manager.create("b", None).unwrap();

        let list = manager.list();
        assert_eq!(list.len(), 3);
        let names: Vec<&str> = list.iter().map(|w| w.name.as_str()).collect();
        assert!(names.contains(&"main"));
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }

    #[test]
    fn manager_delete_removes_directory() {
        let (_dir, mut manager) = setup_manager();
        let wt = manager.create("temp", None).unwrap();
        let path = wt.path.clone();
        assert!(path.exists());

        manager.delete("temp").unwrap();
        assert!(!path.exists());
        assert!(manager.get("temp").is_none());
    }

    #[test]
    fn manager_delete_unknown_fails() {
        let (_dir, mut manager) = setup_manager();
        let err = manager.delete("nope").unwrap_err();
        assert!(matches!(err, WorktreeError::NotFound(_)));
    }

    #[test]
    fn manager_cannot_delete_main() {
        let (_dir, mut manager) = setup_manager();
        let err = manager.delete(MAIN_WORKTREE_NAME).unwrap_err();
        assert!(matches!(err, WorktreeError::CannotDeleteMain));
    }

    #[test]
    fn manager_register_external() {
        let (_dir, mut manager) = setup_manager();
        let wt = Worktree::new("external", "/tmp/external", false).unwrap();
        manager.register(wt.clone()).unwrap();

        assert_eq!(manager.require("external").unwrap().id, wt.id);
    }

    #[test]
    fn manager_register_duplicate_fails() {
        let (_dir, mut manager) = setup_manager();
        let wt = Worktree::new("x", "/tmp/x", false).unwrap();
        manager.register(wt.clone()).unwrap();
        let err = manager.register(wt).unwrap_err();
        assert!(matches!(err, WorktreeError::DuplicateName(_)));
    }

    #[test]
    fn manager_roundtrips_through_json() {
        let (_dir, mut manager) = setup_manager();
        manager.create("a", None).unwrap();
        manager.create("b", Some("bee".into())).unwrap();

        let json = manager.to_json().unwrap();
        let back = WorktreeManager::from_json(&json).unwrap();

        assert_eq!(back.len(), manager.len());
        assert!(back.get("a").is_some());
        assert!(back.get("b").is_some());
        assert_eq!(back.get("b").unwrap().description.as_deref(), Some("bee"));
    }

    #[test]
    fn validate_name_rules() {
        assert!(validate_name("valid").is_ok());
        assert!(validate_name("valid-name").is_ok());
        assert!(validate_name("valid_name_2").is_ok());
        assert!(validate_name("").is_err());
        assert!(validate_name("has space").is_err());
        assert!(validate_name("UPPER").is_err());
        assert!(validate_name(&"a".repeat(65)).is_err());
    }

    #[test]
    fn full_flow_multi_approach_workflow() {
        let (_dir, mut manager) = setup_manager();

        // Simulamos: quiero probar dos enfoques.
        let approach_a = manager
            .create("approach-a", Some("first approach".into()))
            .unwrap();
        let approach_b = manager
            .create("approach-b", Some("second approach".into()))
            .unwrap();

        assert_ne!(approach_a.id, approach_b.id);
        assert_ne!(approach_a.path, approach_b.path);
        assert_eq!(manager.len(), 3); // main + a + b

        // Elijo uno y borro el otro.
        manager.delete("approach-b").unwrap();
        assert_eq!(manager.len(), 2);
        assert!(manager.get("approach-a").is_some());
        assert!(manager.get("approach-b").is_none());
    }
}
