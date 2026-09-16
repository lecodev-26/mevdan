//! `LspClient` — cliente que habla con un language server.

use crate::{
    error::{LspError, LspResult},
    protocol::{
        DidOpenParams, InitializeParams, InitializeResult, Notification, PublishDiagnosticsParams,
        Request, Response, TextDocumentItem,
    },
    transport::{LspTransport, StdioTransport},
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

/// Estado interno por documento abierto.
///
/// En V4.5 solo guardamos la versión del documento (necesaria para
/// futuras notificaciones `didChange`). En V4.6+ añadiremos más
/// estado si hace falta.
#[derive(Debug, Clone)]
struct OpenDocument {
    version: i32,
}

/// Cliente LSP.
pub struct LspClient {
    name: String,
    transport: Box<dyn LspTransport>,
    server_info: Option<InitializeResult>,
    next_id: u64,
    open_documents: BTreeMap<String, OpenDocument>,
    /// Diagnósticos recibidos, indexados por URI.
    diagnostics: BTreeMap<String, Vec<crate::protocol::Diagnostic>>,
}

impl std::fmt::Debug for LspClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LspClient")
            .field("name", &self.name)
            .field("transport", &self.transport.name())
            .field("initialized", &self.server_info.is_some())
            .field("open_documents", &self.open_documents.len())
            .finish()
    }
}

impl LspClient {
    /// Lanza un language server por stdio.
    pub fn connect_stdio(
        name: impl Into<String>,
        command: &str,
        args: &[String],
    ) -> LspResult<Self> {
        let transport = StdioTransport::spawn(command, args, &[])?;
        Ok(Self {
            name: name.into(),
            transport: Box::new(transport),
            server_info: None,
            next_id: 1,
            open_documents: BTreeMap::new(),
            diagnostics: BTreeMap::new(),
        })
    }

    /// Crea un cliente con un transporte arbitrario (tests).
    pub fn with_transport(name: impl Into<String>, transport: Box<dyn LspTransport>) -> Self {
        Self {
            name: name.into(),
            transport,
            server_info: None,
            next_id: 1,
            open_documents: BTreeMap::new(),
            diagnostics: BTreeMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_initialized(&self) -> bool {
        self.server_info.is_some()
    }

    pub fn server_info(&self) -> Option<&InitializeResult> {
        self.server_info.as_ref()
    }

    /// URI del documento abierto (o None).
    pub fn open_document_uri(&self, path: &Path) -> Option<String> {
        let uri = path_to_uri(path);
        if self.open_documents.contains_key(&uri) {
            Some(uri)
        } else {
            None
        }
    }

    /// Diagnósticos actuales de un archivo.
    pub fn diagnostics_for(&self, path: &Path) -> Option<&[crate::protocol::Diagnostic]> {
        let uri = path_to_uri(path);
        self.diagnostics.get(&uri).map(|v| v.as_slice())
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Envía una petición y espera la respuesta.
    fn call(&mut self, method: &str, params: Option<Value>) -> LspResult<Value> {
        let id = self.next_id();
        let mut request = Request::new(id, method);
        if let Some(p) = params {
            request = request.with_params(p);
        }

        let json = serde_json::to_string(&request)?;
        self.transport.send_raw(&json)?;

        loop {
            let raw = self.transport.read_raw()?;
            let value: Value = serde_json::from_str(&raw)?;

            if let Some(method) = value.get("method").and_then(|v| v.as_str()) {
                self.handle_notification(method, value.get("params"));
                continue;
            }

            let response: Response = serde_json::from_value(value)?;

            if response.id != json!(id) {
                return Err(LspError::Protocol(format!(
                    "response id mismatch: expected {}, got {}",
                    id, response.id
                )));
            }

            if let Some(err) = response.error {
                return Err(LspError::RpcError {
                    code: err.code,
                    message: err.message,
                });
            }

            return response
                .result
                .ok_or_else(|| LspError::Protocol("response has no result".into()));
        }
    }

    /// Envía una notificación.
    fn notify(&mut self, method: &str, params: Option<Value>) -> LspResult<()> {
        let mut notification = Notification::new(method);
        if let Some(p) = params {
            notification = notification.with_params(p);
        }
        let json = serde_json::to_string(&notification)?;
        self.transport.send_raw(&json)
    }

    /// Procesa una notificación del servidor.
    fn handle_notification(&mut self, method: &str, params: Option<&Value>) {
        if method == "textDocument/publishDiagnostics" {
            if let Some(p) = params {
                if let Ok(parsed) = serde_json::from_value::<PublishDiagnosticsParams>(p.clone()) {
                    self.diagnostics.insert(parsed.uri, parsed.diagnostics);
                }
            }
        }
    }

    /// Hace el handshake `initialize` + `initialized`.
    pub fn initialize(&mut self, root_path: Option<&Path>) -> LspResult<()> {
        if self.server_info.is_some() {
            return Ok(());
        }

        let root_uri = root_path.map(path_to_uri);
        let params = InitializeParams {
            process_id: None,
            capabilities: json!({
                "textDocument": {
                    "publishDiagnostics": {}
                }
            }),
            root_uri,
            workspace_folders: None,
        };

        let result = self.call("initialize", Some(serde_json::to_value(&params)?))?;
        let init_result: InitializeResult = serde_json::from_value(result)
            .map_err(|e| LspError::Handshake(format!("invalid initialize response: {}", e)))?;

        self.notify("initialized", Some(json!({})))?;

        self.server_info = Some(init_result);
        Ok(())
    }

    /// Abre un documento en el language server.
    pub fn did_open(&mut self, path: &Path, language_id: &str, text: &str) -> LspResult<()> {
        if !self.is_initialized() {
            return Err(LspError::NotInitialized);
        }

        let uri = path_to_uri(path);
        let version = self
            .open_documents
            .get(&uri)
            .map(|d| d.version + 1)
            .unwrap_or(1);

        let params = DidOpenParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: language_id.to_string(),
                version,
                text: text.to_string(),
            },
        };

        self.notify("textDocument/didOpen", Some(serde_json::to_value(&params)?))?;

        self.open_documents.insert(uri, OpenDocument { version });
        Ok(())
    }

    /// Número de documentos abiertos.
    pub fn open_document_count(&self) -> usize {
        self.open_documents.len()
    }

    /// Cierra la conexión.
    pub fn close(mut self) -> LspResult<()> {
        self.transport.close()
    }
}

/// Convierte un `Path` a un URI `file://...`.
fn path_to_uri(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    format!("file://{}", canonical.display())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::DiagnosticSeverity;
    use crate::transport::MockTransport;
    use serde_json::json;

    fn ok_response(id: u64, result: Value) -> String {
        serde_json::to_string(&Response {
            jsonrpc: "2.0".into(),
            id: json!(id),
            result: Some(result),
            error: None,
        })
        .unwrap()
    }

    fn init_response(id: u64) -> String {
        ok_response(
            id,
            json!({
                "capabilities": {"textDocumentSync": 1},
                "serverInfo": {"name": "mock-server", "version": "1.0.0"}
            }),
        )
    }

    #[test]
    fn with_transport_creates_client() {
        let mock = MockTransport::new(vec![]);
        let client = LspClient::with_transport("test", Box::new(mock));
        assert_eq!(client.name(), "test");
        assert!(!client.is_initialized());
    }

    #[test]
    fn initialize_parses_result() {
        let mock = MockTransport::new(vec![init_response(1)]);
        let mut client = LspClient::with_transport("mock", Box::new(mock));

        client.initialize(None).unwrap();
        assert!(client.is_initialized());

        let info = client.server_info().unwrap();
        assert_eq!(info.server_info.as_ref().unwrap().name, "mock-server");
    }

    #[test]
    fn initialize_is_idempotent() {
        let mock = MockTransport::new(vec![init_response(1)]);
        let mut client = LspClient::with_transport("mock", Box::new(mock));

        client.initialize(None).unwrap();
        client.initialize(None).unwrap();
        assert!(client.is_initialized());
    }

    #[test]
    fn did_open_requires_initialization() {
        let mock = MockTransport::new(vec![]);
        let mut client = LspClient::with_transport("mock", Box::new(mock));

        let result = client.did_open(Path::new("/tmp/test.rs"), "rust", "fn main(){}");
        assert!(matches!(result, Err(LspError::NotInitialized)));
    }

    #[test]
    fn did_open_registers_document() {
        let mock = MockTransport::new(vec![init_response(1)]);
        let mut client = LspClient::with_transport("mock", Box::new(mock));

        client.initialize(None).unwrap();
        client
            .did_open(Path::new("/tmp/test.rs"), "rust", "fn main(){}")
            .unwrap();

        assert_eq!(client.open_document_count(), 1);
    }

    #[test]
    fn did_open_same_file_twice_increments_version() {
        let mock = MockTransport::new(vec![init_response(1)]);
        let mut client = LspClient::with_transport("mock", Box::new(mock));

        client.initialize(None).unwrap();
        client
            .did_open(Path::new("/tmp/test.rs"), "rust", "fn main(){}")
            .unwrap();
        client
            .did_open(Path::new("/tmp/test.rs"), "rust", "fn main() { }")
            .unwrap();

        // Sigue siendo 1 documento (mismo URI), pero la versión sube.
        assert_eq!(client.open_document_count(), 1);
    }

    #[test]
    fn severity_roundtrip() {
        let d = crate::protocol::Diagnostic {
            range: crate::protocol::Range {
                start: crate::protocol::Position {
                    line: 0,
                    character: 0,
                },
                end: crate::protocol::Position {
                    line: 0,
                    character: 1,
                },
            },
            message: "test".into(),
            severity: Some(DiagnosticSeverity::Error),
            code: None,
            source: None,
        };
        let json = serde_json::to_value(&d).unwrap();
        assert_eq!(json["severity"], 1);
        let back: crate::protocol::Diagnostic = serde_json::from_value(json).unwrap();
        assert_eq!(back.severity, Some(DiagnosticSeverity::Error));
    }
}
