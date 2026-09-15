//! Registro de tools.
//!
//! Mantiene un conjunto de tools disponibles por nombre. El agente
//! consulta el registro para saber qué puede invocar.

use crate::{
    error::{ToolError, ToolResult},
    tool::{Tool, ToolMetadata},
};
use std::collections::BTreeMap;

/// Registro de tools.
#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    /// Crea un registro vacío.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra un tool.
    ///
    /// Falla si el nombre está vacío o ya existe.
    pub fn register(&mut self, tool: Box<dyn Tool>) -> ToolResult<()> {
        let name = tool.metadata().name.clone();
        validate_tool_name(&name)?;
        if self.tools.contains_key(&name) {
            return Err(ToolError::DuplicateTool(name));
        }
        self.tools.insert(name, tool);
        Ok(())
    }

    /// Número de tools registrados.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// ¿Está vacío?
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Busca un tool por nombre.
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|b| b.as_ref())
    }

    /// Busca un tool o devuelve error.
    pub fn require(&self, name: &str) -> ToolResult<&dyn Tool> {
        self.get(name)
            .ok_or_else(|| ToolError::ToolNotFound(name.to_string()))
    }

    /// Lista los metadatos de todos los tools.
    pub fn list_metadata(&self) -> Vec<&ToolMetadata> {
        self.tools.values().map(|t| t.metadata()).collect()
    }

    /// Lista los nombres de todos los tools.
    pub fn list_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Invoca un tool por nombre con un input.
    pub fn invoke(&self, name: &str, input: serde_json::Value) -> ToolResult<serde_json::Value> {
        let tool = self.require(name)?;
        tool.invoke(input)
    }
}

/// Valida un nombre de tool.
fn validate_tool_name(name: &str) -> ToolResult<()> {
    if name.is_empty() || name.len() > 64 {
        return Err(ToolError::InvalidToolName(name.to_string()));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        return Err(ToolError::InvalidToolName(name.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::{RiskLevel, ToolCategory, ToolMetadata};

    /// Tool de prueba que devuelve el input tal cual.
    #[derive(Debug)]
    struct EchoTool {
        metadata: ToolMetadata,
    }

    impl EchoTool {
        fn new(name: &str) -> Self {
            Self {
                metadata: ToolMetadata {
                    name: name.into(),
                    description: "echoes input".into(),
                    category: ToolCategory::Other,
                    risk: RiskLevel::Low,
                    input_schema: serde_json::json!({}),
                    output_schema: serde_json::json!({}),
                },
            }
        }
    }

    impl Tool for EchoTool {
        fn metadata(&self) -> &ToolMetadata {
            &self.metadata
        }

        fn invoke(&self, input: serde_json::Value) -> ToolResult<serde_json::Value> {
            Ok(input)
        }
    }

    #[test]
    fn new_registry_is_empty() {
        let r = ToolRegistry::new();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn register_adds_tool() {
        let mut r = ToolRegistry::new();
        r.register(Box::new(EchoTool::new("echo"))).unwrap();
        assert_eq!(r.len(), 1);
        assert!(r.get("echo").is_some());
    }

    #[test]
    fn register_duplicate_fails() {
        let mut r = ToolRegistry::new();
        r.register(Box::new(EchoTool::new("echo"))).unwrap();
        let err = r.register(Box::new(EchoTool::new("echo"))).unwrap_err();
        assert!(matches!(err, ToolError::DuplicateTool(_)));
    }

    #[test]
    fn register_empty_name_fails() {
        let mut r = ToolRegistry::new();
        let err = r.register(Box::new(EchoTool::new(""))).unwrap_err();
        assert!(matches!(err, ToolError::InvalidToolName(_)));
    }

    #[test]
    fn register_uppercase_name_fails() {
        let mut r = ToolRegistry::new();
        let err = r.register(Box::new(EchoTool::new("EchoTool"))).unwrap_err();
        assert!(matches!(err, ToolError::InvalidToolName(_)));
    }

    #[test]
    fn register_valid_names() {
        let mut r = ToolRegistry::new();
        r.register(Box::new(EchoTool::new("my_tool"))).unwrap();
        r.register(Box::new(EchoTool::new("my-tool"))).unwrap();
        r.register(Box::new(EchoTool::new("tool123"))).unwrap();
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn get_unknown_returns_none() {
        let r = ToolRegistry::new();
        assert!(r.get("nope").is_none());
    }

    #[test]
    fn require_unknown_returns_error() {
        let r = ToolRegistry::new();
        let result = r.require("nope");
        assert!(matches!(result, Err(ToolError::ToolNotFound(_))));
    }

    #[test]
    fn invoke_calls_tool() {
        let mut r = ToolRegistry::new();
        r.register(Box::new(EchoTool::new("echo"))).unwrap();

        let input = serde_json::json!({"hello": "world"});
        let output = r.invoke("echo", input.clone()).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn invoke_unknown_fails() {
        let r = ToolRegistry::new();
        let result = r.invoke("nope", serde_json::json!({}));
        assert!(matches!(result, Err(ToolError::ToolNotFound(_))));
    }

    #[test]
    fn list_names_is_sorted() {
        let mut r = ToolRegistry::new();
        r.register(Box::new(EchoTool::new("zeta"))).unwrap();
        r.register(Box::new(EchoTool::new("alpha"))).unwrap();
        r.register(Box::new(EchoTool::new("beta"))).unwrap();
        assert_eq!(r.list_names(), vec!["alpha", "beta", "zeta"]);
    }

    #[test]
    fn list_metadata_works() {
        let mut r = ToolRegistry::new();
        r.register(Box::new(EchoTool::new("a"))).unwrap();
        r.register(Box::new(EchoTool::new("b"))).unwrap();
        let metas = r.list_metadata();
        assert_eq!(metas.len(), 2);
        assert!(metas.iter().all(|m| !m.name.is_empty()));
    }
}
