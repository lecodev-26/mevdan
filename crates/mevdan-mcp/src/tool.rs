//! `McpTool` — adaptador que envuelve una tool MCP como `Tool` de MEVDAN.

use crate::{
    error::McpError,
    protocol::{CallToolParams, CallToolResult},
    registry::McpToolHandle,
};
use mevdan_tools::{RiskLevel, Tool, ToolCategory, ToolError, ToolMetadata, ToolResult};
use serde_json::{json, Value};

/// Adaptador de una tool MCP al trait `Tool` de MEVDAN.
///
/// El `McpTool` no tiene estado propio más allá de los metadatos y un
/// `McpToolHandle` que apunta al servidor MCP que la expone.
#[derive(Debug)]
pub struct McpTool {
    metadata: ToolMetadata,
    /// Handle que permite invocar la tool a través del `McpRegistry`.
    handle: McpToolHandle,
}

impl McpTool {
    /// Crea un `McpTool` a partir de un descriptor y un handle.
    pub fn new(handle: McpToolHandle) -> Self {
        let metadata = ToolMetadata {
            name: handle.qualified_name(),
            description: handle
                .description
                .clone()
                .unwrap_or_else(|| format!("MCP tool '{}'", handle.tool_name)),
            category: ToolCategory::Other,
            risk: RiskLevel::Medium,
            input_schema: handle.input_schema.clone(),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "content": {"type": "array"},
                    "isError": {"type": "boolean"}
                }
            }),
        };

        Self { metadata, handle }
    }

    /// Handle interno.
    pub fn handle(&self) -> &McpToolHandle {
        &self.handle
    }

    /// Traduce el output de MCP al formato JSON que devolvemos.
    fn result_to_json(result: CallToolResult) -> Value {
        json!({
            "content": result.content,
            "isError": result.is_error,
            "text": result.text_joined(),
        })
    }
}

impl Tool for McpTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn invoke(&self, _input: Value) -> ToolResult<Value> {
        // La invocación real la hace el McpRegistry (que tiene acceso
        // al cliente MCP y al transporte).
        //
        // El `McpTool` solo describe la tool. Este `invoke` no debería
        // llamarse directamente; en su lugar, el runtime pregunta al
        // `McpRegistry` mediante el handle.
        //
        // Devolvemos un error claro si alguien intenta invocar sin
        // pasar por el registry.
        Err(ToolError::InvalidInput(format!(
            "McpTool '{}' must be invoked through McpRegistry::invoke, \
             not directly",
            self.metadata.name
        )))
    }
}

impl McpTool {
    /// Invoca la tool a través de un closure que hace la llamada real.
    ///
    /// Este método es el que usa `McpRegistry::invoke`. Está separado
    /// del trait `Tool` porque necesita acceso al cliente MCP.
    pub fn invoke_with<F>(&self, input: Value, mut f: F) -> ToolResult<Value>
    where
        F: FnMut(&str, CallToolParams) -> Result<CallToolResult, McpError>,
    {
        let params = CallToolParams {
            name: self.handle.tool_name.clone(),
            arguments: input,
        };

        let result = f(&self.handle.server_name, params)
            .map_err(|e| ToolError::InvalidInput(format!("MCP error: {}", e)))?;

        Ok(Self::result_to_json(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ContentBlock;

    fn sample_handle() -> McpToolHandle {
        McpToolHandle {
            server_name: "filesystem".into(),
            tool_name: "read_file".into(),
            description: Some("Read a file".into()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }),
        }
    }

    #[test]
    fn new_sets_metadata() {
        let handle = sample_handle();
        let tool = McpTool::new(handle);
        let m = tool.metadata();
        assert_eq!(m.name, "filesystem.read_file");
        assert_eq!(m.description, "Read a file");
        assert_eq!(m.category, ToolCategory::Other);
        assert_eq!(m.risk, RiskLevel::Medium);
        assert_eq!(m.input_schema["properties"]["path"]["type"], "string");
    }

    #[test]
    fn new_without_description_uses_fallback() {
        let mut handle = sample_handle();
        handle.description = None;
        let tool = McpTool::new(handle);
        assert!(tool.metadata().description.contains("MCP tool"));
        assert!(tool.metadata().description.contains("read_file"));
    }

    #[test]
    fn invoke_direct_fails_with_clear_error() {
        let tool = McpTool::new(sample_handle());
        let err = tool.invoke(json!({"path": "/tmp"})).unwrap_err();
        match err {
            ToolError::InvalidInput(msg) => {
                assert!(msg.contains("McpRegistry"));
            }
            other => panic!("expected InvalidInput, got {:?}", other),
        }
    }

    #[test]
    fn invoke_with_calls_closure_and_translates() {
        let tool = McpTool::new(sample_handle());

        let output = tool
            .invoke_with(json!({"path": "/tmp/x"}), |_server, params| {
                assert_eq!(params.name, "read_file");
                assert_eq!(params.arguments["path"], "/tmp/x");
                Ok(CallToolResult {
                    content: vec![ContentBlock::Text {
                        text: "file contents".into(),
                    }],
                    is_error: false,
                })
            })
            .unwrap();

        assert_eq!(output["isError"], false);
        assert_eq!(output["text"], "file contents");
    }

    #[test]
    fn invoke_with_propagates_mcp_error() {
        let tool = McpTool::new(sample_handle());

        let err = tool
            .invoke_with(json!({}), |_server, _params| {
                Err(McpError::RpcError {
                    code: -32603,
                    message: "Internal error".into(),
                })
            })
            .unwrap_err();

        match err {
            ToolError::InvalidInput(msg) => {
                assert!(msg.contains("MCP error"));
                assert!(msg.contains("Internal error"));
            }
            other => panic!("expected InvalidInput, got {:?}", other),
        }
    }

    #[test]
    fn invoke_with_handles_is_error_result() {
        let tool = McpTool::new(sample_handle());

        let output = tool
            .invoke_with(json!({}), |_server, _params| {
                Ok(CallToolResult {
                    content: vec![],
                    is_error: true,
                })
            })
            .unwrap();

        assert_eq!(output["isError"], true);
    }
}
