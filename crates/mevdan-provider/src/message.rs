//! Mensajes del chat.
//!
//! MEVDAN define sus propios tipos, independientes de cualquier
//! provider. Los adapters traducen a/desde el formato del provider.

use serde::{Deserialize, Serialize};

/// Rol de un mensaje en la conversación.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Instrucciones del sistema o del runtime. Nunca del usuario.
    System,
    /// Mensaje del usuario humano.
    User,
    /// Respuesta del modelo.
    Assistant,
    /// Resultado de una herramienta.
    Tool,
}

/// Un mensaje en la conversación.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }

    pub fn tool(content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_set_correct_roles() {
        assert_eq!(Message::system("s").role, Role::System);
        assert_eq!(Message::user("u").role, Role::User);
        assert_eq!(Message::assistant("a").role, Role::Assistant);
        assert_eq!(Message::tool("t").role, Role::Tool);
    }

    #[test]
    fn constructors_set_content() {
        let m = Message::user("hola");
        assert_eq!(m.content, "hola");
    }

    #[test]
    fn role_serializes_as_snake_case() {
        assert_eq!(serde_json::to_string(&Role::System).unwrap(), "\"system\"");
        assert_eq!(serde_json::to_string(&Role::User).unwrap(), "\"user\"");
        assert_eq!(
            serde_json::to_string(&Role::Assistant).unwrap(),
            "\"assistant\""
        );
        assert_eq!(serde_json::to_string(&Role::Tool).unwrap(), "\"tool\"");
    }

    #[test]
    fn message_roundtrips() {
        let m = Message::user("test");
        let json = serde_json::to_string(&m).unwrap();
        let back: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }
}
