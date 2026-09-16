//! `McpClient` — cliente que habla con un servidor MCP.

use crate::{
    error::{McpError, McpResult},
    protocol::{
        CallToolParams, CallToolResult, ClientInfo, InitializeParams, InitializeResult,
        ListToolsResult, McpToolDescriptor, Request, MCP_PROTOCOL_VERSION,
    },
    server::McpServer,
    transport::{McpTransport, StdioTransport},
};
use serde_json::{json, Value};

/// Cliente MCP.
pub struct McpClient {
    server: McpServer,
    transport: Box<dyn McpTransport>,
    server_info: Option<InitializeResult>,
    next_id: u64,
}

impl std::fmt::Debug for McpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpClient")
            .field("server", &self.server.name)
            .field("transport", &self.transport.name())
            .field("initialized", &self.server_info.is_some())
            .finish()
    }
}

impl McpClient {
    pub fn connect_stdio(server: McpServer) -> McpResult<Self> {
        let env: Vec<(String, String)> = server
            .env
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let transport = StdioTransport::spawn(&server.command, &server.args, &env)?;

        Ok(Self {
            server,
            transport: Box::new(transport),
            server_info: None,
            next_id: 1,
        })
    }

    pub fn with_transport(server: McpServer, transport: Box<dyn McpTransport>) -> Self {
        Self {
            server,
            transport,
            server_info: None,
            next_id: 1,
        }
    }

    pub fn server_name(&self) -> &str {
        &self.server.name
    }

    pub fn is_initialized(&self) -> bool {
        self.server_info.is_some()
    }

    pub fn server_info(&self) -> Option<&InitializeResult> {
        self.server_info.as_ref()
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn call(&mut self, method: &str, params: Option<Value>) -> McpResult<Value> {
        let id = self.next_id();
        let mut request = Request::new(id, method);
        if let Some(p) = params {
            request = request.with_params(p);
        }

        let response = self.transport.send(&request)?;

        if response.id != json!(id) {
            return Err(McpError::Protocol(format!(
                "response id mismatch: expected {}, got {}",
                id, response.id
            )));
        }

        if let Some(err) = response.error {
            return Err(McpError::RpcError {
                code: err.code,
                message: err.message,
            });
        }

        response
            .result
            .ok_or_else(|| McpError::Protocol("response has neither result nor error".into()))
    }

    pub fn initialize(&mut self) -> McpResult<()> {
        if self.server_info.is_some() {
            return Ok(());
        }

        let params = InitializeParams {
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            capabilities: json!({}),
            client_info: ClientInfo {
                name: "mevdan".into(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        let result = self.call("initialize", Some(serde_json::to_value(&params)?))?;

        let init_result: InitializeResult = serde_json::from_value(result)
            .map_err(|e| McpError::Handshake(format!("invalid initialize response: {}", e)))?;

        let _ = self.call("notifications/initialized", None);

        self.server_info = Some(init_result);
        Ok(())
    }

    pub fn list_tools(&mut self) -> McpResult<Vec<McpToolDescriptor>> {
        if !self.is_initialized() {
            return Err(McpError::Protocol(
                "client not initialized: call initialize() first".into(),
            ));
        }

        let result = self.call("tools/list", None)?;
        let list: ListToolsResult = serde_json::from_value(result)
            .map_err(|e| McpError::Protocol(format!("invalid tools/list response: {}", e)))?;

        Ok(list.tools)
    }

    /// Invoca una tool MCP.
    pub fn call_tool(&mut self, params: CallToolParams) -> McpResult<CallToolResult> {
        if !self.is_initialized() {
            return Err(McpError::Protocol(
                "client not initialized: call initialize() first".into(),
            ));
        }

        let result = self.call("tools/call", Some(serde_json::to_value(&params)?))?;
        let call_result: CallToolResult = serde_json::from_value(result)
            .map_err(|e| McpError::Protocol(format!("invalid tools/call response: {}", e)))?;

        Ok(call_result)
    }

    pub fn close(mut self) -> McpResult<()> {
        self.transport.close()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{protocol::ContentBlock, transport::MockTransport};

    fn make_server() -> McpServer {
        McpServer::new("test", "echo").unwrap()
    }

    fn ok_response(id: u64, result: Value) -> crate::protocol::Response {
        crate::protocol::Response {
            jsonrpc: "2.0".into(),
            id: json!(id),
            result: Some(result),
            error: None,
        }
    }

    fn init_response(id: u64) -> crate::protocol::Response {
        ok_response(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "test-server", "version": "1.0.0"}
            }),
        )
    }

    #[test]
    fn with_transport_creates_client() {
        let mock = MockTransport::new(vec![]);
        let client = McpClient::with_transport(make_server(), Box::new(mock));
        assert_eq!(client.server_name(), "test");
        assert!(!client.is_initialized());
    }

    #[test]
    fn initialize_sends_request_and_parses_result() {
        let mock = MockTransport::new(vec![init_response(1), ok_response(2, json!({}))]);
        let mut client = McpClient::with_transport(make_server(), Box::new(mock));

        client.initialize().unwrap();
        assert!(client.is_initialized());

        let info = client.server_info().unwrap();
        assert_eq!(info.protocol_version, "2024-11-05");
        assert_eq!(info.server_info.name, "test-server");
    }

    #[test]
    fn initialize_is_idempotent() {
        let mock = MockTransport::new(vec![init_response(1), ok_response(2, json!({}))]);
        let mut client = McpClient::with_transport(make_server(), Box::new(mock));

        client.initialize().unwrap();
        client.initialize().unwrap();
        assert!(client.is_initialized());
    }

    #[test]
    fn initialize_fails_on_rpc_error() {
        let mock = MockTransport::new(vec![crate::protocol::Response {
            jsonrpc: "2.0".into(),
            id: json!(1),
            result: None,
            error: Some(crate::protocol::RpcError {
                code: -32601,
                message: "Method not found".into(),
                data: None,
            }),
        }]);
        let mut client = McpClient::with_transport(make_server(), Box::new(mock));

        let err = client.initialize().unwrap_err();
        assert!(matches!(err, McpError::RpcError { .. }));
    }

    #[test]
    fn list_tools_requires_initialization() {
        let mock = MockTransport::new(vec![]);
        let mut client = McpClient::with_transport(make_server(), Box::new(mock));

        let err = client.list_tools().unwrap_err();
        assert!(matches!(err, McpError::Protocol(_)));
    }

    #[test]
    fn list_tools_after_initialize() {
        let mock = MockTransport::new(vec![
            init_response(1),
            ok_response(2, json!({})),
            ok_response(
                3,
                json!({
                    "tools": [
                        {"name": "read_file", "description": "Read"},
                        {"name": "write_file"}
                    ]
                }),
            ),
        ]);
        let mut client = McpClient::with_transport(make_server(), Box::new(mock));

        client.initialize().unwrap();
        let tools = client.list_tools().unwrap();

        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "read_file");
        assert_eq!(tools[0].description.as_deref(), Some("Read"));
    }

    #[test]
    fn call_tool_requires_initialization() {
        let mock = MockTransport::new(vec![]);
        let mut client = McpClient::with_transport(make_server(), Box::new(mock));

        let err = client
            .call_tool(CallToolParams {
                name: "x".into(),
                arguments: json!({}),
            })
            .unwrap_err();
        assert!(matches!(err, McpError::Protocol(_)));
    }

    #[test]
    fn call_tool_returns_result() {
        let mock = MockTransport::new(vec![
            init_response(1),
            ok_response(2, json!({})),
            ok_response(
                3,
                json!({
                    "content": [{"type": "text", "text": "file contents"}],
                    "isError": false
                }),
            ),
        ]);
        let mut client = McpClient::with_transport(make_server(), Box::new(mock));

        client.initialize().unwrap();
        let result = client
            .call_tool(CallToolParams {
                name: "read_file".into(),
                arguments: json!({"path": "/tmp/x"}),
            })
            .unwrap();

        assert_eq!(result.content.len(), 1);
        assert_eq!(result.text_joined(), "file contents");
        assert!(!result.is_error);
    }

    #[test]
    fn call_tool_with_error_flag() {
        let mock = MockTransport::new(vec![
            init_response(1),
            ok_response(2, json!({})),
            ok_response(3, json!({"content": [], "isError": true})),
        ]);
        let mut client = McpClient::with_transport(make_server(), Box::new(mock));

        client.initialize().unwrap();
        let result = client
            .call_tool(CallToolParams {
                name: "x".into(),
                arguments: json!({}),
            })
            .unwrap();

        assert!(result.is_error);
    }

    #[test]
    fn response_id_mismatch_fails() {
        let mock = MockTransport::new(vec![ok_response(999, json!({}))]);
        let mut client = McpClient::with_transport(make_server(), Box::new(mock));

        let err = client.initialize().unwrap_err();
        assert!(matches!(err, McpError::Protocol(_)));
    }

    #[test]
    fn close_calls_transport_close() {
        let mock = MockTransport::new(vec![]);
        let client = McpClient::with_transport(make_server(), Box::new(mock));
        assert!(client.close().is_ok());
    }

    #[test]
    fn content_block_text_extraction() {
        let result = CallToolResult {
            content: vec![
                ContentBlock::Text {
                    text: "line 1".into(),
                },
                ContentBlock::Text {
                    text: "line 2".into(),
                },
            ],
            is_error: false,
        };
        assert_eq!(result.text_joined(), "line 1\nline 2");
    }
}
