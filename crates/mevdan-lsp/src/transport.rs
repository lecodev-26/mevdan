//! Transporte LSP.
//!
//! El cliente LSP se comunica con el language server a través de un
//! `LspTransport`. La implementación principal usa stdio, igual que
//! MCP, pero el framing es distinto (ver `framing.rs`).

use crate::{
    error::{LspError, LspResult},
    framing::{read_message, write_message},
};
use std::io::{BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// Transporte LSP.
pub trait LspTransport: Send {
    /// Envía un mensaje de texto JSON (ya serializado).
    fn send_raw(&mut self, json: &str) -> LspResult<()>;

    /// Lee un mensaje de texto JSON (con framing).
    fn read_raw(&mut self) -> LspResult<String>;

    /// Cierra el transporte.
    fn close(&mut self) -> LspResult<()>;

    /// Nombre del transporte.
    fn name(&self) -> &str;
}

// ─────────────────────────────────────────────
// StdioTransport
// ─────────────────────────────────────────────

/// Transporte por stdio: lanza un language server como subproceso.
pub struct StdioTransport {
    name: String,
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl StdioTransport {
    /// Lanza el language server.
    pub fn spawn(command: &str, args: &[String], env: &[(String, String)]) -> LspResult<Self> {
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
            .map_err(|e| LspError::Transport(format!("failed to spawn '{}': {}", command, e)))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| LspError::Transport("failed to open stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| LspError::Transport("failed to open stdout".into()))?;

        Ok(Self {
            name: format!("stdio:{}", command),
            child,
            stdin: Some(stdin),
            stdout: BufReader::new(stdout),
        })
    }
}

impl LspTransport for StdioTransport {
    fn send_raw(&mut self, json: &str) -> LspResult<()> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| LspError::Transport("stdin already closed".into()))?;
        write_message(stdin, json)?;
        stdin.flush()?;
        Ok(())
    }

    fn read_raw(&mut self) -> LspResult<String> {
        read_message(&mut self.stdout)
    }

    fn close(&mut self) -> LspResult<()> {
        self.stdin.take();
        let _ = self.child.wait();
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        self.stdin.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ─────────────────────────────────────────────
// MockTransport (solo para tests)
// ─────────────────────────────────────────────

/// Transporte de prueba: guarda los mensajes enviados y devuelve
/// respuestas predefinidas.
#[cfg(test)]
pub struct MockTransport {
    /// Respuestas a devolver, en orden.
    pub responses: std::collections::VecDeque<String>,
    /// Mensajes enviados por el cliente.
    pub sent: Vec<String>,
}

#[cfg(test)]
impl MockTransport {
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses: responses.into(),
            sent: Vec::new(),
        }
    }

    pub fn sent_messages(&self) -> &[String] {
        &self.sent
    }
}

#[cfg(test)]
impl LspTransport for MockTransport {
    fn send_raw(&mut self, json: &str) -> LspResult<()> {
        self.sent.push(json.to_string());
        Ok(())
    }

    fn read_raw(&mut self) -> LspResult<String> {
        self.responses
            .pop_front()
            .ok_or_else(|| LspError::Transport("no more mock responses".into()))
    }

    fn close(&mut self) -> LspResult<()> {
        Ok(())
    }

    fn name(&self) -> &str {
        "mock"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stdio_spawn_fails_with_unknown_command() {
        let result = StdioTransport::spawn("definitely-not-real-cmd-xyz", &[], &[]);
        assert!(result.is_err());
    }

    #[test]
    fn mock_transport_records_sent() {
        let mut mock = MockTransport::new(vec!["response".into()]);
        mock.send_raw(r#"{"test":1}"#).unwrap();
        assert_eq!(mock.sent_messages().len(), 1);
        assert_eq!(mock.sent_messages()[0], r#"{"test":1}"#);
    }

    #[test]
    fn mock_transport_returns_responses_in_order() {
        let mut mock = MockTransport::new(vec!["first".into(), "second".into()]);
        assert_eq!(mock.read_raw().unwrap(), "first");
        assert_eq!(mock.read_raw().unwrap(), "second");
    }

    #[test]
    fn mock_transport_fails_when_empty() {
        let mut mock = MockTransport::new(vec![]);
        let err = mock.read_raw().unwrap_err();
        assert!(matches!(err, LspError::Transport(_)));
    }
}
