//! Almacenamiento de secretos.
//!
//! ## Diseño
//!
//! Se define un trait `SecretStore` que abstrae el mecanismo de
//! almacenamiento. La implementación por defecto es `FileSecretStore`,
//! que guarda secretos en un archivo JSON con permisos `0600` (solo el
//! usuario propietario puede leerlo).
//!
//! El archivo NUNCA vive dentro de un proyecto MEVDAN. Vive en el
//! directorio de config global del usuario (`~/.config/mevdan/` en
//! Linux/macOS, `%APPDATA%\mevdan\` en Windows). Esto garantiza que
//! `git add .` dentro de un proyecto NUNCA incluya secretos.
//!
//! ## Nombres de secretos
//!
//! Los nombres deben ser identificadores simples:
//! - Solo `a-z`, `A-Z`, `0-9`, `_`, `-`, `.`
//! - No vacíos, máximo 128 caracteres
//!
//! Ejemplos válidos:
//! - `openai_api_key`
//! - `anthropic.api-key`
//! - `providers.deepseek.key`

use crate::error::{SecretsError, SecretsResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Nombres de secretos válidos.
pub fn validate_name(name: &str) -> SecretsResult<()> {
    if name.is_empty() || name.len() > 128 {
        return Err(SecretsError::InvalidName(name.to_string()));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err(SecretsError::InvalidName(name.to_string()));
    }
    Ok(())
}

/// Interfaz de almacenamiento de secretos.
///
/// Implementaciones posibles (futuro):
/// - `FileSecretStore` (por defecto)
/// - `KeyringSecretStore` (Linux/macOS/Windows, requiere deps nativas)
/// - `AndroidKeystoreStore` (Android)
pub trait SecretStore {
    /// Lee un secreto. Devuelve `NotFound` si no existe.
    fn get(&self, name: &str) -> SecretsResult<String>;

    /// Guarda (o reemplaza) un secreto.
    fn set(&mut self, name: &str, value: &str) -> SecretsResult<()>;

    /// Elimina un secreto. Devuelve `NotFound` si no existe.
    fn delete(&mut self, name: &str) -> SecretsResult<()>;

    /// Lista los NOMBRES de los secretos guardados. Nunca los valores.
    fn list(&self) -> SecretsResult<Vec<String>>;

    /// Comprueba si existe un secreto.
    fn contains(&self, name: &str) -> SecretsResult<bool>;
}

// ─────────────────────────────────────────────────────────
// FileSecretStore — implementación por defecto
// ─────────────────────────────────────────────────────────

/// Formato en disco. Se guarda como JSON.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SecretsFile {
    /// Versión del esquema del archivo.
    #[serde(default = "default_schema_version")]
    schema_version: String,

    /// Mapa nombre → valor. BTreeMap para orden determinista.
    #[serde(default)]
    secrets: BTreeMap<String, String>,
}

fn default_schema_version() -> String {
    "0.1.0".to_string()
}

/// Implementación por defecto: archivo JSON local con permisos 0600.
pub struct FileSecretStore {
    path: PathBuf,
    data: SecretsFile,
}

impl FileSecretStore {
    /// Abre (o crea si no existe) el almacén en la ruta por defecto.
    ///
    /// La ruta por defecto es `<config_dir>/mevdan/secrets.json`.
    pub fn open_default() -> SecretsResult<Self> {
        let config_dir = mevdan_config::paths::global_config_dir()?;
        let path = config_dir.join("secrets.json");
        Self::open_at(&path)
    }

    /// Abre (o crea si no existe) el almacén en una ruta específica.
    ///
    /// Crea el directorio padre si no existe. Aplica permisos `0600` en
    /// Unix.
    pub fn open_at(path: &Path) -> SecretsResult<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let data = if path.exists() {
            let content = fs::read_to_string(path)?;
            serde_json::from_str(&content)?
        } else {
            SecretsFile {
                schema_version: default_schema_version(),
                secrets: BTreeMap::new(),
            }
        };

        let store = Self {
            path: path.to_path_buf(),
            data,
        };

        // Asegurar que el archivo existe con permisos correctos.
        store.persist()?;
        Ok(store)
    }

    /// Escribe el contenido a disco con permisos seguros.
    fn persist(&self) -> SecretsResult<()> {
        let json = serde_json::to_string_pretty(&self.data)?;
        fs::write(&self.path, json)?;

        // En Unix: permisos 0600 (solo el propietario puede leer/escribir).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&self.path, perms)?;
        }

        Ok(())
    }

    /// Ruta del archivo de secretos.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl SecretStore for FileSecretStore {
    fn get(&self, name: &str) -> SecretsResult<String> {
        validate_name(name)?;
        self.data
            .secrets
            .get(name)
            .cloned()
            .ok_or_else(|| SecretsError::NotFound(name.to_string()))
    }

    fn set(&mut self, name: &str, value: &str) -> SecretsResult<()> {
        validate_name(name)?;
        self.data
            .secrets
            .insert(name.to_string(), value.to_string());
        self.persist()
    }

    fn delete(&mut self, name: &str) -> SecretsResult<()> {
        validate_name(name)?;
        if self.data.secrets.remove(name).is_none() {
            return Err(SecretsError::NotFound(name.to_string()));
        }
        self.persist()
    }

    fn list(&self) -> SecretsResult<Vec<String>> {
        Ok(self.data.secrets.keys().cloned().collect())
    }

    fn contains(&self, name: &str) -> SecretsResult<bool> {
        validate_name(name)?;
        Ok(self.data.secrets.contains_key(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fresh_store() -> (TempDir, FileSecretStore) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("secrets.json");
        let store = FileSecretStore::open_at(&path).unwrap();
        (dir, store)
    }

    #[test]
    fn valid_names_are_accepted() {
        assert!(validate_name("openai_api_key").is_ok());
        assert!(validate_name("anthropic.api-key").is_ok());
        assert!(validate_name("providers.deepseek.key").is_ok());
        assert!(validate_name("A1_b2-c3.d4").is_ok());
    }

    #[test]
    fn invalid_names_are_rejected() {
        assert!(validate_name("").is_err());
        assert!(validate_name("has space").is_err());
        assert!(validate_name("has/slash").is_err());
        assert!(validate_name("has@symbol").is_err());
        assert!(validate_name(&"a".repeat(129)).is_err());
    }

    #[test]
    fn set_get_roundtrip() {
        let (_dir, mut store) = fresh_store();
        store.set("openai_api_key", "sk-test-123").unwrap();
        assert_eq!(store.get("openai_api_key").unwrap(), "sk-test-123");
    }

    #[test]
    fn get_missing_returns_not_found() {
        let (_dir, store) = fresh_store();
        let err = store.get("does_not_exist").unwrap_err();
        assert!(matches!(err, SecretsError::NotFound(_)));
    }

    #[test]
    fn contains_reflects_state() {
        let (_dir, mut store) = fresh_store();
        assert!(!store.contains("key1").unwrap());
        store.set("key1", "value1").unwrap();
        assert!(store.contains("key1").unwrap());
    }

    #[test]
    fn list_returns_names_only() {
        let (_dir, mut store) = fresh_store();
        store.set("key1", "value1").unwrap();
        store.set("key2", "value2").unwrap();
        let names = store.list().unwrap();
        assert_eq!(names, vec!["key1".to_string(), "key2".to_string()]);
        // Verifica que los valores NO están en la lista.
        assert!(!names.contains(&"value1".to_string()));
        assert!(!names.contains(&"value2".to_string()));
    }

    #[test]
    fn delete_removes_secret() {
        let (_dir, mut store) = fresh_store();
        store.set("key1", "value1").unwrap();
        store.delete("key1").unwrap();
        assert!(!store.contains("key1").unwrap());
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let (_dir, mut store) = fresh_store();
        let err = store.delete("nope").unwrap_err();
        assert!(matches!(err, SecretsError::NotFound(_)));
    }

    #[test]
    fn persistence_across_reopen() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("secrets.json");

        {
            let mut store = FileSecretStore::open_at(&path).unwrap();
            store.set("key1", "value1").unwrap();
        }

        {
            let store = FileSecretStore::open_at(&path).unwrap();
            assert_eq!(store.get("key1").unwrap(), "value1");
        }
    }

    #[test]
    fn overwriting_secret_works() {
        let (_dir, mut store) = fresh_store();
        store.set("key", "old").unwrap();
        store.set("key", "new").unwrap();
        assert_eq!(store.get("key").unwrap(), "new");
    }

    #[cfg(unix)]
    #[test]
    fn file_has_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("secrets.json");
        let _store = FileSecretStore::open_at(&path).unwrap();

        let perms = fs::metadata(&path).unwrap().permissions();
        let mode = perms.mode() & 0o777;
        assert_eq!(mode, 0o600, "expected permissions 0600, got {:o}", mode);
    }

    #[test]
    fn empty_file_created_with_schema_version() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("secrets.json");
        let _store = FileSecretStore::open_at(&path).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("schema_version"));
        assert!(content.contains("0.1.0"));
    }
}
