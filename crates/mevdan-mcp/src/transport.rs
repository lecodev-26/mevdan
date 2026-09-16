//! Transporte MCP.
//!
//! El cliente MCP se comunica con el servidor a través de un
//! `McpTransport`. Esto permite abstraer diferentes mecanismos:
//!
//! - `StdioTransport` — comunicación por stdin/stdout de un
//!   subproceso (el más común).
//! - `MockTransport` (en tests) — respuestas predefinidas.
//! - (futuro) HTTP, WebSocket, ...

use crate::{
    error::{McpError, McpResult},
    protocol::{Request, Response},
};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// Transporte bidireccional de mensajes JSON-RPC.
pub trait McpTransport: Send {
    /// Envía una petición y devuelve la respuesta.
    ///
    /// El transporte debe encargarse de:
    /// 1. Serializar la petición a JSON.
    /// 2. Enviarla al servidor.
    /// 3. Leer la respuesta.
    /// 4. Deserializarla.
    ///
    /// En esta versión síncrona, cada `send` bloquea hasta recibir
    /// la respuesta correspondiente.
    fn send(&mut self, request: &Request) -> McpResult<Response>;

    /// Cierra el transporte.
    fn close(&mut self) -> McpResult<()>;

    /// Nombre del transporte (para logs).
    fn name(&self) -> &str;
}

// ─────────────────────────────────────────────
// StdioTransport
// ─────────────────────────────────────────────

/// Transporte por stdio: lanza un subproceso y habla por
/// stdin/stdout con él.
///
/// El subproceso es el servidor MCP.
pub struct StdioTransport {
    name: String,
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl StdioTransport {
    /// Lanza el servidor MCP y abre los canales.
    pub fn spawn(command: &str, args: &[String], env: &[(String, String)]) -> McpResult<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (k, v) in env {
            cmd.env(k, v);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| McpError::Transport(format!("failed to spawn '{}': {}", command, e)))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::Transport("failed to open stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::Transport("failed to open stdout".into()))?;

        Ok(Self {
            name: format!("stdio:{}", command),
            child,
            stdin: Some(stdin),
            stdout: BufReader::new(stdout),
        })
    }
}

impl McpTransport for StdioTransport {
    fn send(&mut self, request: &Request) -> McpResult<Response> {
        // 1. Serializar a JSON.
        let line = serde_json::to_string(request)?;

        // 2. Enviar por stdin + newline.
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| McpError::Transport("stdin already closed".into()))?;
        stdin.write_all(line.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;

        // 3. Leer una línea de stdout.
        let mut response_line = String::new();
        let bytes_read = self.stdout.read_line(&mut response_line)?;
        if bytes_read == 0 {
            return Err(McpError::Transport(
                "server closed stdout unexpectedly".into(),
            ));
        }

        // 4. Deserializar.
        let response: Response = serde_json::from_str(response_line.trim())?;

        Ok(response)
    }

    fn close(&mut self) -> McpResult<()> {
        // Cerramos stdin (señal al servidor de que no enviamos más).
        self.stdin.take();
        // Esperamos a que termine.
        let _ = self.child.wait();
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        // Best-effort cleanup.
        self.stdin.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ─────────────────────────────────────────────
// MockTransport (solo para tests)
// ─────────────────────────────────────────────

/// Transporte de prueba.
///
/// Devuelve respuestas predefinidas en orden. Cada llamada a `send`
/// consume una respuesta.
#[cfg(test)]
pub struct MockTransport {
    responses: std::collections::VecDeque<Response>,
    sent_requests: Vec<Request>,
}

#[cfg(test)]
impl MockTransport {
    pub fn new(responses: Vec<Response>) -> Self {
        Self {
            responses: responses.into(),
            sent_requests: Vec::new(),
        }
    }

    pub fn sent_requests(&self) -> &[Request] {
        &self.sent_requests
    }
}

#[cfg(test)]
impl McpTransport for MockTransport {
    fn send(&mut self, request: &Request) -> McpResult<Response> {
        self.sent_requests.push(request.clone());
        self.responses
            .pop_front()
            .ok_or_else(|| McpError::Transport("no more mock responses".into()))
    }

    fn close(&mut self) -> McpResult<()> {
        Ok(())
    }

    fn name(&self) -> &str {
        "mock"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ClientInfo, InitializeParams};

    #[test]
    fn stdio_spawn_fails_with_unknown_command() {
        let result = StdioTransport::spawn("definitely-not-a-real-command-xyz", &[], &[]);
        assert!(result.is_err());
    }

    #[test]
    fn mock_transport_returns_responses_in_order() {
        let r1 = Response {
            jsonrpc: "2.0".into(),
            id: serde_json::json!(1),
            result: Some(serde_json::json!({"a": 1})),
            error: None,
        };
        let r2 = Response {
            jsonrpc: "2.0".into(),
            id: serde_json::json!(2),
            result: Some(serde_json::json!({"b": 2})),
            error: None,
        };

        let mut mock = MockTransport::new(vec![r1, r2]);

        let req1 = Request::new(1, "first");
        let resp1 = mock.send(&req1).unwrap();
        assert_eq!(resp1.id, serde_json::json!(1));
        assert_eq!(resp1.result.unwrap()["a"], 1);

        let req2 = Request::new(2, "second");
        let resp2 = mock.send(&req2).unwrap();
        assert_eq!(resp2.id, serde_json::json!(2));
        assert_eq!(resp2.result.unwrap()["b"], 2);

        assert_eq!(mock.sent_requests().len(), 2);
    }

    #[test]
    fn mock_transport_fails_when_out_of_responses() {
        let mut mock = MockTransport::new(vec![]);
        let req = Request::new(1, "test");
        let err = mock.send(&req).unwrap_err();
        assert!(matches!(err, McpError::Transport(_)));
    }

    #[test]
    fn initialize_request_serializes_correctly() {
        let params = InitializeParams {
            protocol_version: super::super::protocol::MCP_PROTOCOL_VERSION.to_string(),
            capabilities: serde_json::json!({}),
            client_info: ClientInfo {
                name: "mevdan".into(),
                version: "0.4.0".into(),
            },
        };
        let req = Request::new(1, "initialize").with_params(serde_json::to_value(&params).unwrap());
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["method"], "initialize");
        assert_eq!(json["params"]["protocolVersion"], "2024-11-05");
        assert_eq!(json["params"]["clientInfo"]["name"], "mevdan");
    }
}
