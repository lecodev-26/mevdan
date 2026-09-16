//! # mevdan-lsp
//!
//! Cliente LSP (Language Server Protocol) para MEVDAN.
//!
//! ## Estado del proyecto
//!
//! - **V4.5** ✅ — Framing, `LspClient`, handshake, `didOpen`,
//!   recepción de diagnósticos.
//! - **V4.6** ⏳ — Code Intelligence (símbolos, definiciones, referencias).
//!
//! ## Ejemplo
//!
//! ```no_run
//! use mevdan_lsp::LspClient;
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut client = LspClient::connect_stdio(
//!     "rust-analyzer",
//!     "rust-analyzer",
//!     &[],
//! )?;
//! client.initialize(Some(Path::new("/home/user/project")))?;
//!
//! client.did_open(
//!     Path::new("/home/user/project/src/main.rs"),
//!     "rust",
//!     "fn main() { println!(\"hi\"); }",
//! )?;
//!
//! // (En una versión futura: esperar notificaciones y consultar diagnósticos)
//! client.close()?;
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod error;
pub mod framing;
pub mod protocol;
pub mod transport;

// Re-exports de conveniencia.
pub use client::LspClient;
pub use error::{LspError, LspResult};
pub use protocol::{
    Diagnostic, DiagnosticSeverity, DidOpenParams, InitializeParams, InitializeResult,
    Notification, Position, PublishDiagnosticsParams, Range, Request, Response, RpcError,
    TextDocumentItem, LSP_VERSION,
};
pub use transport::{LspTransport, StdioTransport};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockTransport;
    use serde_json::json;

    fn init_response(id: u64) -> String {
        serde_json::to_string(&Response {
            jsonrpc: "2.0".into(),
            id: json!(id),
            result: Some(json!({
                "capabilities": {"textDocumentSync": 1},
                "serverInfo": {"name": "test", "version": "0.1.0"}
            })),
            error: None,
        })
        .unwrap()
    }

    #[test]
    fn full_flow_initialize_and_open() {
        let mock = MockTransport::new(vec![init_response(1)]);
        let mut client = LspClient::with_transport("test", Box::new(mock));

        client.initialize(None).unwrap();
        assert!(client.is_initialized());
        assert_eq!(
            client
                .server_info()
                .unwrap()
                .server_info
                .as_ref()
                .unwrap()
                .name,
            "test"
        );

        client
            .did_open(std::path::Path::new("/tmp/test.rs"), "rust", "fn main() {}")
            .unwrap();
        assert_eq!(client.open_document_count(), 1);
    }

    #[test]
    fn framing_roundtrip() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
        let mut buf = Vec::new();
        framing::write_message(&mut buf, json).unwrap();

        let mut cursor = std::io::Cursor::new(buf);
        let read = framing::read_message(&mut cursor).unwrap();
        assert_eq!(read, json);
    }
}
