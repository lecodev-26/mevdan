//! Tipos del protocolo JSON-RPC 2.0 + MCP.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Versión del protocolo MCP que soportamos.
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// ID de una petición JSON-RPC.
pub type RequestId = Value;

/// Petición JSON-RPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl Request {
    pub fn new(id: impl Into<RequestId>, method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            method: method.into(),
            params: None,
        }
    }

    pub fn with_params(mut self, params: Value) -> Self {
        self.params = Some(params);
        self
    }
}

/// Respuesta JSON-RPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    pub id: RequestId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl Response {
    pub fn is_ok(&self) -> bool {
        self.error.is_none() && self.result.is_some()
    }

    pub fn is_err(&self) -> bool {
        self.error.is_some()
    }
}

/// Error JSON-RPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

// ─────────────────────────────────────────────
// initialize
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: Value,
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: Value,
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

// ─────────────────────────────────────────────
// tools/list
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListToolsResult {
    pub tools: Vec<McpToolDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDescriptor {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "inputSchema", default)]
    pub input_schema: Value,
}

// ─────────────────────────────────────────────
// tools/call
// ─────────────────────────────────────────────

/// Parámetros de `tools/call`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolParams {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

/// Resultado de `tools/call`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolResult {
    #[serde(default)]
    pub content: Vec<ContentBlock>,
    #[serde(rename = "isError", default)]
    pub is_error: bool,
}

/// Un bloque de contenido en una respuesta MCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Texto plano.
    Text { text: String },
    /// Imagen (base64 + mime type).
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    /// Recurso referenciado.
    Resource { resource: Value },
}

impl CallToolResult {
    /// Concatena todo el texto de los bloques de tipo `Text`.
    pub fn text_joined(&self) -> String {
        let mut parts = Vec::new();
        for block in &self.content {
            if let ContentBlock::Text { text } = block {
                parts.push(text.clone());
            }
        }
        parts.join("\n")
    }

    /// ¿Contiene algún texto?
    pub fn has_text(&self) -> bool {
        self.content
            .iter()
            .any(|b| matches!(b, ContentBlock::Text { .. }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_new_has_defaults() {
        let r = Request::new(1, "initialize");
        assert_eq!(r.jsonrpc, "2.0");
        assert_eq!(r.method, "initialize");
        assert!(r.params.is_none());
    }

    #[test]
    fn request_with_params() {
        let r = Request::new(1, "test").with_params(serde_json::json!({"x": 1}));
        assert_eq!(r.params.unwrap()["x"], 1);
    }

    #[test]
    fn response_ok_predicates() {
        let resp = Response {
            jsonrpc: "2.0".into(),
            id: serde_json::json!(1),
            result: Some(serde_json::json!({"ok": true})),
            error: None,
        };
        assert!(resp.is_ok());
        assert!(!resp.is_err());
    }

    #[test]
    fn response_err_predicates() {
        let resp = Response {
            jsonrpc: "2.0".into(),
            id: serde_json::json!(1),
            result: None,
            error: Some(RpcError {
                code: -32601,
                message: "Method not found".into(),
                data: None,
            }),
        };
        assert!(!resp.is_ok());
        assert!(resp.is_err());
    }

    #[test]
    fn initialize_result_deserializes() {
        let json = r#"{
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "filesystem", "version": "1.0.0"}
        }"#;
        let result: InitializeResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.protocol_version, "2024-11-05");
        assert_eq!(result.server_info.name, "filesystem");
    }

    #[test]
    fn list_tools_result_deserializes() {
        let json = r#"{
            "tools": [
                {"name": "read_file", "description": "Read", "inputSchema": {"type": "object"}},
                {"name": "write_file"}
            ]
        }"#;
        let result: ListToolsResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.tools.len(), 2);
        assert_eq!(result.tools[0].name, "read_file");
        assert!(result.tools[0].description.is_some());
        assert!(result.tools[1].description.is_none());
    }

    #[test]
    fn call_tool_params_serializes() {
        let params = CallToolParams {
            name: "read_file".into(),
            arguments: serde_json::json!({"path": "/tmp/x"}),
        };
        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["name"], "read_file");
        assert_eq!(json["arguments"]["path"], "/tmp/x");
    }

    #[test]
    fn call_tool_result_deserializes_text() {
        let json = r#"{
            "content": [
                {"type": "text", "text": "hello"},
                {"type": "text", "text": "world"}
            ]
        }"#;
        let result: CallToolResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.content.len(), 2);
        assert_eq!(result.text_joined(), "hello\nworld");
        assert!(result.has_text());
        assert!(!result.is_error);
    }

    #[test]
    fn call_tool_result_deserializes_image() {
        let json = r#"{
            "content": [
                {"type": "image", "data": "base64...", "mimeType": "image/png"}
            ]
        }"#;
        let result: CallToolResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.content.len(), 1);
        assert!(!result.has_text());
    }

    #[test]
    fn call_tool_result_error() {
        let json = r#"{"content": [], "isError": true}"#;
        let result: CallToolResult = serde_json::from_str(json).unwrap();
        assert!(result.is_error);
    }
}
