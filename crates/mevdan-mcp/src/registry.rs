//! `McpRegistry` — catálogo de tools MCP de múltiples servidores.

use crate::{
    client::McpClient,
    error::{McpError, McpResult},
    protocol::CallToolParams,
    security::McpSecurityGuard,
    tool::McpTool,
};
use mevdan_tools::ToolResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Handle a una tool MCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolHandle {
    pub server_name: String,
    pub tool_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub input_schema: Value,
}

impl McpToolHandle {
    /// Nombre cualificado: `<server>.<tool>`.
    pub fn qualified_name(&self) -> String {
        format!("{}.{}", self.server_name, self.tool_name)
    }
}

/// Catálogo de tools MCP.
pub struct McpRegistry {
    clients: BTreeMap<String, McpClient>,
    tools: BTreeMap<String, McpToolHandle>,
}

impl std::fmt::Debug for McpRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpRegistry")
            .field("servers", &self.clients.len())
            .field("tools", &self.tools.len())
            .finish()
    }
}

impl Default for McpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl McpRegistry {
    pub fn new() -> Self {
        Self {
            clients: BTreeMap::new(),
            tools: BTreeMap::new(),
        }
    }

    /// Registra un cliente MCP ya inicializado.
    pub fn add_client(&mut self, name: impl Into<String>, mut client: McpClient) -> McpResult<()> {
        let name = name.into();

        client.initialize()?;
        let tools = client.list_tools()?;

        for descriptor in tools {
            let handle = McpToolHandle {
                server_name: name.clone(),
                tool_name: descriptor.name,
                description: descriptor.description,
                input_schema: descriptor.input_schema,
            };
            self.tools.insert(handle.qualified_name(), handle);
        }

        self.clients.insert(name, client);
        Ok(())
    }

    /// Devuelve el `McpTool` correspondiente a un nombre cualificado.
    pub fn tool(&self, qualified_name: &str) -> Option<McpTool> {
        let handle = self.tools.get(qualified_name)?.clone();
        Some(McpTool::new(handle))
    }

    /// Lista todos los handles registrados.
    pub fn list_tools(&self) -> Vec<&McpToolHandle> {
        self.tools.values().collect()
    }

    /// Lista los nombres cualificados de las tools.
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    pub fn server_count(&self) -> usize {
        self.clients.len()
    }

    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Invoca una tool por nombre cualificado, aplicando el guard.
    pub fn invoke(
        &mut self,
        guard: &McpSecurityGuard,
        qualified_name: &str,
        arguments: Value,
    ) -> ToolResult<Value> {
        let handle = self
            .tools
            .get(qualified_name)
            .cloned()
            .ok_or_else(|| mevdan_tools::ToolError::ToolNotFound(qualified_name.to_string()))?;

        // 1. Verificar seguridad.
        let decision = guard
            .check(&handle.server_name, &handle.tool_name)
            .map_err(|e| {
                mevdan_tools::ToolError::InvalidInput(format!("security check failed: {}", e))
            })?;

        if decision.is_denied() {
            return Err(mevdan_tools::ToolError::InvalidInput(format!(
                "MCP invocation denied: {} (server: {}, tool: {})",
                decision.reason, handle.server_name, handle.tool_name
            )));
        }

        // 2. Invocar.
        let client = self.clients.get_mut(&handle.server_name).ok_or_else(|| {
            mevdan_tools::ToolError::ToolNotFound(format!(
                "server '{}' not connected",
                handle.server_name
            ))
        })?;

        let params = CallToolParams {
            name: handle.tool_name.clone(),
            arguments,
        };

        let result = client
            .call_tool(params)
            .map_err(|e| mevdan_tools::ToolError::InvalidInput(format!("MCP error: {}", e)))?;

        Ok(serde_json::json!({
            "content": result.content,
            "isError": result.is_error,
            "text": result.text_joined(),
        }))
    }

    /// Elimina un servidor y sus tools.
    pub fn remove_server(&mut self, name: &str) -> McpResult<()> {
        let client = self
            .clients
            .remove(name)
            .ok_or_else(|| McpError::ServerNotFound(name.to_string()))?;

        self.tools.retain(|_, h| h.server_name != name);
        client.close()?;
        Ok(())
    }

    /// Cierra todos los servidores.
    pub fn close_all(&mut self) -> McpResult<()> {
        let clients = std::mem::take(&mut self.clients);
        for (_, client) in clients {
            let _ = client.close();
        }
        self.tools.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        protocol::Response,
        security::{McpPolicy, McpToolPolicy},
        server::McpServer,
        transport::MockTransport,
    };
    use mevdan_permissions::Permission;
    use mevdan_tools::Tool;
    use serde_json::json;

    fn ok_response(id: u64, result: Value) -> Response {
        Response {
            jsonrpc: "2.0".into(),
            id: json!(id),
            result: Some(result),
            error: None,
        }
    }

    fn init_response(id: u64, server_name: &str) -> Response {
        ok_response(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "serverInfo": {"name": server_name, "version": "1.0.0"}
            }),
        )
    }

    fn make_client_with_mock(responses: Vec<Response>, name: &str) -> McpClient {
        let server = McpServer::new(name, "echo").unwrap();
        let mock = MockTransport::new(responses);
        McpClient::with_transport(server, Box::new(mock))
    }

    fn permissive_guard_for(name: &str) -> McpSecurityGuard {
        let mut g = McpSecurityGuard::new();
        g.register_policy(McpPolicy::trusted_allow(name));
        g
    }

    #[test]
    fn new_registry_is_empty() {
        let r = McpRegistry::new();
        assert!(r.is_empty());
        assert_eq!(r.server_count(), 0);
        assert_eq!(r.tool_count(), 0);
    }

    #[test]
    fn add_client_registers_tools() {
        let client = make_client_with_mock(
            vec![
                init_response(1, "filesystem"),
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
            ],
            "filesystem",
        );

        let mut registry = McpRegistry::new();
        registry.add_client("filesystem", client).unwrap();

        assert_eq!(registry.server_count(), 1);
        assert_eq!(registry.tool_count(), 2);
        assert_eq!(
            registry.tool_names(),
            vec!["filesystem.read_file", "filesystem.write_file"]
        );
    }

    #[test]
    fn tool_returns_mcp_tool() {
        let client = make_client_with_mock(
            vec![
                init_response(1, "fs"),
                ok_response(2, json!({})),
                ok_response(3, json!({"tools": [{"name": "list"}]})),
            ],
            "fs",
        );

        let mut registry = McpRegistry::new();
        registry.add_client("fs", client).unwrap();

        let tool = registry.tool("fs.list").unwrap();
        assert_eq!(tool.metadata().name, "fs.list");
    }

    #[test]
    fn tool_unknown_returns_none() {
        let r = McpRegistry::new();
        assert!(r.tool("nope").is_none());
    }

    #[test]
    fn invoke_calls_client_with_permission() {
        let client = make_client_with_mock(
            vec![
                init_response(1, "fs"),
                ok_response(2, json!({})),
                ok_response(3, json!({"tools": [{"name": "read"}]})),
                ok_response(
                    4,
                    json!({
                        "content": [{"type": "text", "text": "file contents"}],
                        "isError": false
                    }),
                ),
            ],
            "fs",
        );

        let mut registry = McpRegistry::new();
        registry.add_client("fs", client).unwrap();
        let guard = permissive_guard_for("fs");

        let result = registry
            .invoke(&guard, "fs.read", json!({"path": "/tmp/x"}))
            .unwrap();
        assert_eq!(result["text"], "file contents");
        assert_eq!(result["isError"], false);
    }

    #[test]
    fn invoke_without_policy_is_denied() {
        let client = make_client_with_mock(
            vec![
                init_response(1, "fs"),
                ok_response(2, json!({})),
                ok_response(3, json!({"tools": [{"name": "read"}]})),
            ],
            "fs",
        );

        let mut registry = McpRegistry::new();
        registry.add_client("fs", client).unwrap();

        let guard = McpSecurityGuard::new();

        let result = registry.invoke(&guard, "fs.read", json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn invoke_with_deny_policy_is_denied() {
        let client = make_client_with_mock(
            vec![
                init_response(1, "fs"),
                ok_response(2, json!({})),
                ok_response(3, json!({"tools": [{"name": "read"}]})),
            ],
            "fs",
        );

        let mut registry = McpRegistry::new();
        registry.add_client("fs", client).unwrap();

        let mut guard = McpSecurityGuard::new();
        guard.register_policy(McpPolicy::untrusted("fs"));

        let result = registry.invoke(&guard, "fs.read", json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn invoke_with_specific_tool_deny() {
        let client = make_client_with_mock(
            vec![
                init_response(1, "fs"),
                ok_response(2, json!({})),
                ok_response(3, json!({"tools": [{"name": "read"}, {"name": "danger"}]})),
            ],
            "fs",
        );

        let mut registry = McpRegistry::new();
        registry.add_client("fs", client).unwrap();

        let mut guard = McpSecurityGuard::new();
        guard.register_policy(
            McpPolicy::trusted_allow("fs")
                .add_tool_policy(McpToolPolicy::new("danger", Permission::Deny)),
        );

        let client2 = make_client_with_mock(
            vec![
                init_response(1, "fs"),
                ok_response(2, json!({})),
                ok_response(3, json!({"tools": [{"name": "read"}, {"name": "danger"}]})),
                ok_response(4, json!({"content": [{"type": "text", "text": "ok"}]})),
            ],
            "fs",
        );
        let mut registry2 = McpRegistry::new();
        registry2.add_client("fs", client2).unwrap();

        let result = registry2.invoke(&guard, "fs.read", json!({}));
        assert!(result.is_ok());

        let result = registry.invoke(&guard, "fs.danger", json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn invoke_unknown_tool_fails() {
        let mut registry = McpRegistry::new();
        let guard = McpSecurityGuard::new();
        let result = registry.invoke(&guard, "nope", json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn remove_server_removes_tools() {
        let client = make_client_with_mock(
            vec![
                init_response(1, "fs"),
                ok_response(2, json!({})),
                ok_response(3, json!({"tools": [{"name": "read"}]})),
            ],
            "fs",
        );

        let mut registry = McpRegistry::new();
        registry.add_client("fs", client).unwrap();
        assert_eq!(registry.tool_count(), 1);

        registry.remove_server("fs").unwrap();
        assert_eq!(registry.server_count(), 0);
        assert_eq!(registry.tool_count(), 0);
    }

    #[test]
    fn remove_unknown_server_fails() {
        let mut registry = McpRegistry::new();
        let err = registry.remove_server("nope").unwrap_err();
        assert!(matches!(err, McpError::ServerNotFound(_)));
    }

    #[test]
    fn tool_handle_qualified_name() {
        let h = McpToolHandle {
            server_name: "fs".into(),
            tool_name: "read".into(),
            description: None,
            input_schema: json!({}),
        };
        assert_eq!(h.qualified_name(), "fs.read");
    }

    #[test]
    fn multiple_servers_coexist() {
        let client_a = make_client_with_mock(
            vec![
                init_response(1, "a"),
                ok_response(2, json!({})),
                ok_response(3, json!({"tools": [{"name": "tool_a"}]})),
            ],
            "a",
        );
        let client_b = make_client_with_mock(
            vec![
                init_response(1, "b"),
                ok_response(2, json!({})),
                ok_response(3, json!({"tools": [{"name": "tool_b"}]})),
            ],
            "b",
        );

        let mut registry = McpRegistry::new();
        registry.add_client("a", client_a).unwrap();
        registry.add_client("b", client_b).unwrap();

        assert_eq!(registry.server_count(), 2);
        assert_eq!(registry.tool_count(), 2);
        let names = registry.tool_names();
        assert!(names.contains(&"a.tool_a".to_string()));
        assert!(names.contains(&"b.tool_b".to_string()));
    }
}
