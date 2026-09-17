# MEVDAN — MCP Specification

The **Model Context Protocol (MCP)** is an open protocol that lets
LLMs talk to external servers exposing tools, resources and prompts.

MEVDAN implements MCP as a **client**. It connects to MCP servers
and exposes their tools to agents as if they were native.

## Concepts

### McpServer

A descriptor for how to launch and talk to an MCP server.

```rust
pub struct McpServer {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub description: Option<String>,
}
Server names follow the same rules as skills: 1-64 chars, a-z, 0-9,
-, _.

McpTransport
Abstraction over the wire. MCP uses JSON-RPC 2.0 over stdio (one
message per line, newline-delimited).

MEVDAN implements:

StdioTransport — spawns a subprocess, talks over stdin/stdout.

MockTransport (tests only) — predefined responses.

The trait:

rust
pub trait McpTransport: Send {
    fn send(&mut self, request: &Request) -> McpResult<Response>;
    fn close(&mut self) -> McpResult<()>;
    fn name(&self) -> &str;
}
McpClient
The client performs the MCP handshake and exposes operations.

rust
let server = McpServer::new("filesystem", "npx")?
    .with_args(["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]);

let mut client = McpClient::connect_stdio(server)?;
client.initialize()?;

let tools = client.list_tools()?;
for tool in tools {
    println!("- {}", tool.name);
}

let result = client.call_tool(CallToolParams {
    name: "read_file".into(),
    arguments: json!({"path": "/tmp/x"}),
})?;

client.close()?;
Handshake
MCP handshake sequence:

Client → initialize (with protocol version, capabilities, client info).

Server → result (protocol version, capabilities, server info).

Client → notifications/initialized (best-effort).

MEVDAN uses MCP_PROTOCOL_VERSION = "2024-11-05".

Tools
After handshake, the client can list tools:

json
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
The response includes a list of tool descriptors with name,
description and inputSchema (JSON Schema).

Invoking tools
json
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
    "name": "read_file",
    "arguments": {"path": "/tmp/x"}
}}
The response includes an array of content blocks (text, image, resource)
plus an optional isError flag.

McpRegistry
Multiple MCP servers can be registered at once. Their tools are
exposed with a qualified name: <server>.<tool>.

rust
let mut registry = McpRegistry::new();
registry.add_client("filesystem", client_fs)?;
registry.add_client("git", client_git)?;

println!("{:?}", registry.tool_names());
// ["filesystem.read_file", "filesystem.write_file", "git.status", ...]

let result = registry.invoke(&guard, "filesystem.read_file", args)?;
Registry operations:

add_client(name, client) — register a server.

tool(qualified_name) — get an McpTool (adapter to mevdan_tools::Tool).

list_tools() / tool_names() — enumerate.

invoke(guard, qualified_name, args) — call a tool (with security check).

remove_server(name) — remove a server and its tools.

close_all() — close every client.

Security
MCP is powerful. Without security, a server could delete files,
make network requests, or run arbitrary commands. MEVDAN addresses
this with trust levels and policies.

Trust levels
rust
pub enum McpTrustLevel {
    Trusted,    // user explicitly approved
    Unknown,    // default: deny everything
    Untrusted,  // explicitly rejected
}
Only Trusted servers allow tool execution.

Policies
rust
pub struct McpPolicy {
    pub server_name: String,
    pub trust: McpTrustLevel,
    pub default_permission: Permission,
    pub tool_policies: BTreeMap<String, McpToolPolicy>,
}
A policy decides the effective permission for a tool:

If the server is not Trusted → Deny.

If a tool-specific policy exists → use it.

Otherwise → use the server's default_permission.

Guard
The McpSecurityGuard evaluates every invocation:

rust
let mut guard = McpSecurityGuard::new();
guard.register_policy(
    McpPolicy::trusted_ask("filesystem")
        .add_tool_policy(McpToolPolicy::new("read_file", Permission::Allow))
        .add_tool_policy(McpToolPolicy::new("delete_file", Permission::Deny)),
);

let decision = guard.check("filesystem", "read_file")?;
assert!(decision.is_allowed());
The guard reuses mevdan_permissions::Permission (Allow / Ask / Deny)
so MEVDAN has one permission model, not two.

What MCP is NOT in V4
No remote transports. Only stdio. HTTP/WebSocket come later.

No resources or prompts. Only tools.

No persistent configuration. The CLI's mcp list is
informational; persistence arrives in V5.

No automatic server discovery. The user provides the command.

The core abstractions (McpServer, McpClient, McpRegistry,
McpSecurityGuard) are stable. Adding new transports or protocols
will not change them.

Design notes
JSON-RPC 2.0, line-delimited. No Content-Length framing
(that's LSP). Simple and stream-friendly.

No async. Blocking I/O, like the rest of V4.

Tool names are qualified. <server>.<tool> avoids collisions.

Trust is per server, not per tool by default. Explicit policies
can override this per tool.

The guard runs before every invocation. No bypass.
