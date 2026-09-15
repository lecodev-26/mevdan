//! # mevdan-secrets
//!
//! Gestión segura de secretos (API keys, tokens).
//!
//! ## Reglas duras
//!
//! 1. **Nunca** se persisten secretos en el proyecto (`.mevdan/`).
//!    Siempre en el directorio de config global del usuario.
//! 2. **Nunca** se escriben secretos en logs, eventos, o mensajes de
//!    error.
//! 3. **Nunca** se commitean secretos. El archivo vive fuera del repo.
//! 4. Los archivos de secretos tienen permisos `0600` en Unix.
//! 5. Los mensajes de error solo muestran el NOMBRE del secreto, jamás
//!    el valor.
//!
//! ## Uso básico
//!
//! ```no_run
//! use mevdan_secrets::{FileSecretStore, SecretStore};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut store = FileSecretStore::open_default()?;
//! store.set("openai_api_key", "sk-...")?;
//! let key = store.get("openai_api_key")?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Implementaciones futuras
//!
//! El trait `SecretStore` permite añadir sin cambiar la API:
//! - `KeyringSecretStore` (Linux libsecret, macOS Keychain, Windows
//!   Credential Manager).
//! - `AndroidKeystoreStore` (Android Keystore).

pub mod error;
pub mod store;

// Re-exports de conveniencia.
pub use error::{SecretsError, SecretsResult};
pub use store::{validate_name, FileSecretStore, SecretStore};

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn full_flow_store_and_retrieve() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("secrets.json");

        let mut store = FileSecretStore::open_at(&path).unwrap();
        store.set("openai_api_key", "sk-test").unwrap();
        store.set("anthropic_api_key", "sk-ant-test").unwrap();

        let names = store.list().unwrap();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"openai_api_key".to_string()));
        assert!(names.contains(&"anthropic_api_key".to_string()));

        assert_eq!(store.get("openai_api_key").unwrap(), "sk-test");
        assert_eq!(store.get("anthropic_api_key").unwrap(), "sk-ant-test");

        store.delete("openai_api_key").unwrap();
        assert_eq!(store.list().unwrap().len(), 1);
    }

    #[test]
    fn store_is_usable_through_trait_object() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("secrets.json");

        let mut store: Box<dyn SecretStore> = Box::new(FileSecretStore::open_at(&path).unwrap());

        store.set("key", "value").unwrap();
        assert_eq!(store.get("key").unwrap(), "value");
    }
}
