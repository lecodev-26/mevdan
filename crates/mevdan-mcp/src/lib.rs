//! # mevdan-mcp
//!
//! Cliente MCP (Model Context Protocol) para MEVDAN.
//!
//! ## Estado del proyecto
//!
//! - **V4.2** ✅ — `McpServer`, `McpTransport`, `StdioTransport`, `McpClient` (handshake + list_tools).
//! - **V4.3** ✅ — `McpTool`, `McpRegistry`, `call_tool`.
//! - **V4.4** ✅ — `McpTrustLevel`, `McpPolicy`, `McpSecurityGuard`.

pub mod client;
pub mod error;
pub mod protocol;
pub mod registry;
pub mod security;
pub mod server;
pub mod tool;
pub mod transport;

// Re-exports de conveniencia.
pub use client::McpClient;
pub use error::{McpError, McpResult};
pub use protocol::{
    CallToolParams, CallToolResult, ContentBlock, McpToolDescriptor, Request, Response, RpcError,
    MCP_PROTOCOL_VERSION,
};
pub use registry::{McpRegistry, McpToolHandle};
pub use security::{
    McpPolicy, McpSecurityDecision, McpSecurityGuard, McpToolPolicy, McpTrustLevel,
};
pub use server::{validate_server_name, McpServer};
pub use tool::McpTool;
pub use transport::{McpTransport, StdioTransport};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Response;
    use crate::transport::MockTransport;
    use mevdan_tools::Tool;
    use serde_json::json;

    fn permissive_guard(name: &str) -> McpSecurityGuard {
        let mut g = McpSecurityGuard::new();
        g.register_policy(McpPolicy::trusted_allow(name));
        g
    }

    #[test]
    fn full_flow_server_to_tool() {
        let server = McpServer::new("filesystem", "npx")
            .unwrap()
            .with_args(["-y", "@modelcontextprotocol/server-filesystem"])
            .with_description("File system access");

        let mock = MockTransport::new(vec![
            Response {
                jsonrpc: "2.0".into(),
                id: json!(1),
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "serverInfo": {"name": "fs", "version": "1.0.0"}
                })),
                error: None,
            },
            Response {
                jsonrpc: "2.0".into(),
                id: json!(2),
                result: Some(json!({})),
                error: None,
            },
            Response {
                jsonrpc: "2.0".into(),
                id: json!(3),
                result: Some(json!({
                    "tools": [
                        {"name": "read_file", "description": "Read a file"}
                    ]
                })),
                error: None,
            },
            Response {
                jsonrpc: "2.0".into(),
                id: json!(4),
                result: Some(json!({
                    "content": [{"type": "text", "text": "hello world"}],
                    "isError": false
                })),
                error: None,
            },
        ]);

        let client = McpClient::with_transport(server, Box::new(mock));

        let mut registry = McpRegistry::new();
        registry.add_client("filesystem", client).unwrap();

        assert_eq!(registry.server_count(), 1);
        assert_eq!(registry.tool_count(), 1);

        let tool = registry.tool("filesystem.read_file").unwrap();
        assert_eq!(tool.metadata().name, "filesystem.read_file");
        assert_eq!(tool.metadata().description, "Read a file");

        let guard = permissive_guard("filesystem");
        let result = registry
            .invoke(&guard, "filesystem.read_file", json!({"path": "/tmp/x"}))
            .unwrap();
        assert_eq!(result["text"], "hello world");
    }

    #[test]
    fn full_flow_denied_by_default() {
        let server = McpServer::new("untrusted", "cmd").unwrap();

        let mock = MockTransport::new(vec![
            Response {
                jsonrpc: "2.0".into(),
                id: json!(1),
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "serverInfo": {"name": "u", "version": "1.0.0"}
                })),
                error: None,
            },
            Response {
                jsonrpc: "2.0".into(),
                id: json!(2),
                result: Some(json!({})),
                error: None,
            },
            Response {
                jsonrpc: "2.0".into(),
                id: json!(3),
                result: Some(json!({"tools": [{"name": "dangerous"}]})),
                error: None,
            },
        ]);

        let client = McpClient::with_transport(server, Box::new(mock));

        let mut registry = McpRegistry::new();
        registry.add_client("untrusted", client).unwrap();

        // Guard vacío → todo denegado.
        let guard = McpSecurityGuard::new();
        let result = registry.invoke(&guard, "untrusted.dangerous", json!({}));
        assert!(result.is_err());
    }
}
