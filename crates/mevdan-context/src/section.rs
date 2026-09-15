//! Secciones del contexto.
//!
//! El contexto se compone de secciones ordenadas por prioridad. Si hay
//! que recortar por presupuesto, se recortan las de menor prioridad
//! primero.

use serde::{Deserialize, Serialize};

/// Prioridad de una sección.
///
/// Menor número = mayor prioridad. Nunca se recorta una sección
/// `Critical`; si no cabe, se produce error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    /// System prompt. Siempre presente, nunca se recorta.
    Critical = 0,
    /// Tarea actual del usuario. Siempre presente.
    High = 1,
    /// Contexto del proyecto. Normalmente presente.
    Medium = 2,
    /// Historial de mensajes recientes.
    Low = 3,
    /// Archivos y documentos referenciados.
    Optional = 4,
}

/// Sección del contexto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    /// Título legible (para logs/debug).
    pub title: String,
    /// Contenido textual.
    pub content: String,
    /// Prioridad.
    pub priority: Priority,
    /// ¿Se puede truncar si no cabe?
    pub truncatable: bool,
}

impl Section {
    pub fn new(title: impl Into<String>, content: impl Into<String>, priority: Priority) -> Self {
        Self {
            title: title.into(),
            content: content.into(),
            priority,
            truncatable: !matches!(priority, Priority::Critical),
        }
    }

    /// Sección crítica: nunca se recorta.
    pub fn critical(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            content: content.into(),
            priority: Priority::Critical,
            truncatable: false,
        }
    }

    /// Sección de alta prioridad.
    pub fn high(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(title, content, Priority::High)
    }

    /// Sección de prioridad media.
    pub fn medium(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(title, content, Priority::Medium)
    }

    /// Sección de baja prioridad.
    pub fn low(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(title, content, Priority::Low)
    }

    /// Sección opcional.
    pub fn optional(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(title, content, Priority::Optional)
    }

    /// Longitud en bytes.
    pub fn byte_len(&self) -> usize {
        self.content.len()
    }

    /// ¿Está vacía?
    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty()
    }

    /// Trunca el contenido a `max_bytes`, respetando límites de
    /// caracteres.
    pub fn truncate_to(&mut self, max_bytes: usize) {
        if self.content.len() <= max_bytes {
            return;
        }
        let mut end = max_bytes;
        while end > 0 && !self.content.is_char_boundary(end) {
            end -= 1;
        }
        self.content.truncate(end);
        // Añade marca de truncado.
        self.content.push_str("... [truncated]");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn critical_is_not_truncatable() {
        let s = Section::critical("sys", "you are helpful");
        assert!(!s.truncatable);
        assert_eq!(s.priority, Priority::Critical);
    }

    #[test]
    fn high_is_truncatable() {
        let s = Section::high("task", "do something");
        assert!(s.truncatable);
        assert_eq!(s.priority, Priority::High);
    }

    #[test]
    fn priorities_are_ordered() {
        assert!(Priority::Critical < Priority::High);
        assert!(Priority::High < Priority::Medium);
        assert!(Priority::Medium < Priority::Low);
        assert!(Priority::Low < Priority::Optional);
    }

    #[test]
    fn priority_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&Priority::Critical).unwrap(),
            "\"critical\""
        );
        assert_eq!(serde_json::to_string(&Priority::High).unwrap(), "\"high\"");
    }

    #[test]
    fn is_empty_works() {
        assert!(Section::high("t", "").is_empty());
        assert!(Section::high("t", "   ").is_empty());
        assert!(!Section::high("t", "x").is_empty());
    }

    #[test]
    fn truncate_to_short_content_is_noop() {
        let mut s = Section::high("t", "short");
        let original = s.content.clone();
        s.truncate_to(100);
        assert_eq!(s.content, original);
    }

    #[test]
    fn truncate_to_long_content_marks_it() {
        let mut s = Section::high("t", "abcdefghijklmnop");
        s.truncate_to(5);
        assert!(s.content.starts_with("abcde"));
        assert!(s.content.ends_with("[truncated]"));
    }

    #[test]
    fn truncate_respects_utf8_boundaries() {
        let mut s = Section::high("t", "áéíóú");
        // 'áé' son 4 bytes (2 cada uno). Truncar a 3 bytes debe
        // parar en 2 bytes para no romper el char.
        s.truncate_to(3);
        // El contenido debe ser válido UTF-8.
        assert!(std::str::from_utf8(s.content.as_bytes()).is_ok());
    }

    #[test]
    fn section_roundtrips() {
        let s = Section::medium("project", "info here");
        let json = serde_json::to_string(&s).unwrap();
        let back: Section = serde_json::from_str(&json).unwrap();
        assert_eq!(back.title, s.title);
        assert_eq!(back.content, s.content);
        assert_eq!(back.priority, s.priority);
    }
}
