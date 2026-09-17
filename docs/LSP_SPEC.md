# MEVDAN — LSP Specification

The **Language Server Protocol (LSP)** lets editors talk to language
servers (`rust-analyzer`, `pyright`, `tsserver`, `gopls`, ...).

MEVDAN implements LSP as a **client**. It connects to existing servers
to get diagnostics, symbols, definitions and references.

MEVDAN does **not** implement an LSP server.

## Framing

LSP uses JSON-RPC 2.0 with **HTTP-like headers**. This is different
from MCP (which is line-delimited).

Each message has the form:

```text
Content-Length: 123\r\n
Content-Type: application/vscode-jsonrpc; charset=utf-8\r\n
\r\n
{"jsonrpc":"2.0","id":1,"method":"initialize",...}
Rules:

Each header line ends with \r\n.

Content-Length is required and counts bytes (not chars).

Content-Type is optional but recommended.

Headers end with an empty line: \r\n\r\n.

Then exactly Content-Length bytes of UTF-8 JSON.

MEVDAN implements this in mevdan-lsp::framing:

rust
use mevdan_lsp::framing::{read_message, write_message};

let json = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
let mut buf = Vec::new();
write_message(&mut buf, json)?;

let mut cursor = std::io::Cursor::new(buf);
let read = read_message(&mut cursor)?;
assert_eq!(read, json);
LspClient
rust
use mevdan_lsp::LspClient;
use std::path::Path;

let mut client = LspClient::connect_stdio(
    "rust-analyzer",
    "rust-analyzer",
    &[],
)?;

client.initialize(Some(Path::new("/path/to/project")))?;

client.did_open(
    Path::new("/path/to/project/src/main.rs"),
    "rust",
    "fn main() { println!(\"hi\"); }",
)?;

// Diagnostics arrive asynchronously. They are stored per URI.
// (In V4 the client reads them when they are received during a call.)
client.close()?;
Handshake
The sequence is:

Client → initialize (with capabilities, root URI, client info).

Server → result (server capabilities, server info).

Client → initialized (notification, no response expected).

MEVDAN uses LSP_VERSION = "3.17".

Documents
To make the server aware of a document, the client sends
textDocument/didOpen:

json
{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{
    "textDocument": {
        "uri": "file:///path/to/file.rs",
        "languageId": "rust",
        "version": 1,
        "text": "fn main() {}"
    }
}}
The client tracks open documents per URI. Reopening the same URI
increments its version (needed for future didChange support).

Diagnostics
The server sends textDocument/publishDiagnostics notifications
asynchronously — at any point after didOpen.

json
{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{
    "uri": "file:///path/to/file.rs",
    "diagnostics": [
        {
            "range": {"start": {"line": 0, "character": 0},
                      "end":   {"line": 0, "character": 10}},
            "message": "unused variable",
            "severity": 2,
            "source": "rustc"
        }
    ]
}}
Severity is an integer:

Value	Meaning
1	Error
2	Warning
3	Information
4	Hint
MEVDAN models this as DiagnosticSeverity with TryFrom<u8>.

The client stores diagnostics per URI. Reading them:

rust
let diags = client.diagnostics_for(Path::new("/path/to/file.rs"));
Important note about V4: while the client can receive diagnostics,
in V4 the processing happens synchronously during a call. When a
notification arrives in the middle of a request, it is handled and
stored. Truly asynchronous notification reading (a background thread
that always listens) is planned for V5.

Design notes
No async. Blocking I/O, like the rest of V4.

One client per language server. No shared state between clients.

Document tracking. Open documents are keyed by URI.

Cross-platform paths. Path → file:// URIs use canonicalized
paths.

No server management. MEVDAN does not launch language servers
for the user; they must be installed and in $PATH.

What LSP is NOT in V4
No didChange. Editing a document after didOpen will
re-open it (version bumps), not send deltas.

No definition, references, hover, documentSymbol.
These arrive in V5 as part of Code Intelligence v2.

No workspace-wide operations. Only single-file requests.

No server lifecycle management. No auto-install, no
auto-restart.

No async reader. Reads happen when the client is in a call.

What it already enables
Understanding whether a language server is available.

Sending a document and receiving diagnostics.

Fitting into mevdan-codeintel for symbol/dependency analysis
when the LSP path is added in V5.

The infrastructure is complete. Adding new capabilities is
straightforward and does not change the client's shape.
