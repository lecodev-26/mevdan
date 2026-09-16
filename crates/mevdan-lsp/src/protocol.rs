//! Tipos del protocolo LSP.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Versión de LSP que soportamos.
pub const LSP_VERSION: &str = "3.17";

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

/// Notificación JSON-RPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl Notification {
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
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
    pub process_id: Option<u32>,
    pub capabilities: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_folders: Option<Vec<WorkspaceFolder>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFolder {
    pub uri: String,
    pub name: String,
}

/// Resultado de `initialize`.
///
/// **Importante:** LSP manda `serverInfo` (camelCase), no
/// `server_info`. Por eso el `#[serde(rename = "serverInfo")]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    pub capabilities: Value,
    #[serde(
        rename = "serverInfo",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub server_info: Option<ServerInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

// ─────────────────────────────────────────────
// textDocument/didOpen
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidOpenParams {
    pub text_document: TextDocumentItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentItem {
    pub uri: String,
    pub language_id: String,
    pub version: i32,
    pub text: String,
}

// ─────────────────────────────────────────────
// textDocument/publishDiagnostics
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishDiagnosticsParams {
    pub uri: String,
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<DiagnosticSeverity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "u8", try_from = "u8")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

impl From<DiagnosticSeverity> for u8 {
    fn from(s: DiagnosticSeverity) -> u8 {
        match s {
            DiagnosticSeverity::Error => 1,
            DiagnosticSeverity::Warning => 2,
            DiagnosticSeverity::Information => 3,
            DiagnosticSeverity::Hint => 4,
        }
    }
}

impl TryFrom<u8> for DiagnosticSeverity {
    type Error = String;

    fn try_from(v: u8) -> Result<Self, <DiagnosticSeverity as TryFrom<u8>>::Error> {
        match v {
            1 => Ok(DiagnosticSeverity::Error),
            2 => Ok(DiagnosticSeverity::Warning),
            3 => Ok(DiagnosticSeverity::Information),
            4 => Ok(DiagnosticSeverity::Hint),
            _ => Err(format!("invalid severity: {}", v)),
        }
    }
}

impl DiagnosticSeverity {
    pub fn display_name(&self) -> &'static str {
        match self {
            DiagnosticSeverity::Error => "error",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Information => "info",
            DiagnosticSeverity::Hint => "hint",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_new_has_defaults() {
        let r = Request::new(1, "initialize");
        assert_eq!(r.jsonrpc, "2.0");
        assert!(r.params.is_none());
    }

    #[test]
    fn request_with_params() {
        let r = Request::new(1, "test").with_params(serde_json::json!({"x": 1}));
        assert_eq!(r.params.unwrap()["x"], 1);
    }

    #[test]
    fn notification_new_has_no_id() {
        let n = Notification::new("initialized");
        let json = serde_json::to_value(&n).unwrap();
        assert!(!json.as_object().unwrap().contains_key("id"));
        assert_eq!(json["method"], "initialized");
    }

    #[test]
    fn response_predicates() {
        let ok = Response {
            jsonrpc: "2.0".into(),
            id: serde_json::json!(1),
            result: Some(serde_json::json!({})),
            error: None,
        };
        assert!(ok.is_ok());
        assert!(!ok.is_err());
    }

    #[test]
    fn initialize_params_serializes() {
        let params = InitializeParams {
            process_id: Some(1234),
            capabilities: serde_json::json!({"textDocument": {"publishDiagnostics": {}}}),
            root_uri: Some("file:///home/user".into()),
            workspace_folders: None,
        };
        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["process_id"], 1234);
        assert_eq!(json["root_uri"], "file:///home/user");
        assert!(!json.as_object().unwrap().contains_key("workspace_folders"));
    }

    #[test]
    fn initialize_result_deserializes() {
        let json = r#"{
            "capabilities": {"textDocumentSync": 1},
            "serverInfo": {"name": "rust-analyzer", "version": "1.0.0"}
        }"#;
        let result: InitializeResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.server_info.unwrap().name, "rust-analyzer");
    }

    #[test]
    fn initialize_result_without_server_info() {
        let json = r#"{"capabilities": {}}"#;
        let result: InitializeResult = serde_json::from_str(json).unwrap();
        assert!(result.server_info.is_none());
    }

    #[test]
    fn did_open_params_serializes() {
        let params = DidOpenParams {
            text_document: TextDocumentItem {
                uri: "file:///tmp/test.rs".into(),
                language_id: "rust".into(),
                version: 1,
                text: "fn main() {}".into(),
            },
        };
        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["text_document"]["uri"], "file:///tmp/test.rs");
        assert_eq!(json["text_document"]["language_id"], "rust");
    }

    #[test]
    fn diagnostics_deserialize() {
        let json = r#"{
            "uri": "file:///tmp/test.rs",
            "diagnostics": [
                {
                    "range": {
                        "start": {"line": 0, "character": 0},
                        "end": {"line": 0, "character": 10}
                    },
                    "message": "unused variable",
                    "severity": 2,
                    "source": "rustc"
                }
            ]
        }"#;
        let params: PublishDiagnosticsParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.diagnostics.len(), 1);
        assert_eq!(params.diagnostics[0].message, "unused variable");
        assert_eq!(
            params.diagnostics[0].severity,
            Some(DiagnosticSeverity::Warning)
        );
    }

    #[test]
    fn severity_display_names() {
        assert_eq!(DiagnosticSeverity::Error.display_name(), "error");
        assert_eq!(DiagnosticSeverity::Warning.display_name(), "warning");
        assert_eq!(DiagnosticSeverity::Information.display_name(), "info");
        assert_eq!(DiagnosticSeverity::Hint.display_name(), "hint");
    }

    #[test]
    fn severity_serializes_as_number() {
        assert_eq!(serde_json::to_value(DiagnosticSeverity::Error).unwrap(), 1);
        assert_eq!(
            serde_json::to_value(DiagnosticSeverity::Warning).unwrap(),
            2
        );
        assert_eq!(
            serde_json::to_value(DiagnosticSeverity::Information).unwrap(),
            3
        );
        assert_eq!(serde_json::to_value(DiagnosticSeverity::Hint).unwrap(), 4);
    }

    #[test]
    fn severity_deserializes_from_number() {
        let s: DiagnosticSeverity = serde_json::from_value(serde_json::json!(1)).unwrap();
        assert_eq!(s, DiagnosticSeverity::Error);
    }

    #[test]
    fn severity_rejects_invalid_number() {
        let result: Result<DiagnosticSeverity, _> = serde_json::from_value(serde_json::json!(99));
        assert!(result.is_err());
    }
}
