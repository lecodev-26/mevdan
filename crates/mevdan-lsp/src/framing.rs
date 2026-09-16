//! Framing del protocolo LSP.
//!
//! LSP usa JSON-RPC 2.0 **con cabeceras** estilo HTTP. Cada mensaje
//! tiene este formato:
//!
//! ```text
//! Content-Length: 103\r\n
//! Content-Type: application/vscode-jsonrpc; charset=utf-8\r\n
//! \r\n
//! {"jsonrpc":"2.0","id":1,"method":"initialize",...}
//! ```
//!
//! **Reglas:**
//!
//! 1. Cada cabecera termina en `\r\n`.
//! 2. La cabecera `Content-Length` es obligatoria.
//! 3. La `Content-Type` es opcional pero recomendada.
//! 4. Tras la última cabecera, `\r\n\r\n`.
//! 5. Luego, exactamente `Content-Length` bytes de JSON (UTF-8).
//!
//! **Importante:** `Content-Length` cuenta **bytes**, no caracteres.

use crate::error::{LspError, LspResult};
use std::io::{BufRead, Write};

/// Escribe un mensaje con el framing LSP.
///
/// Devuelve el número de bytes escritos (incluyendo cabeceras).
pub fn write_message<W: Write>(writer: &mut W, json: &str) -> LspResult<usize> {
    let content_bytes = json.as_bytes();
    let header = format!(
        "Content-Length: {}\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8\r\n\r\n",
        content_bytes.len()
    );

    writer.write_all(header.as_bytes())?;
    writer.write_all(content_bytes)?;
    writer.flush()?;

    Ok(header.len() + content_bytes.len())
}

/// Lee un mensaje con el framing LSP.
///
/// Parsea las cabeceras, extrae `Content-Length`, y lee exactamente
/// esa cantidad de bytes como JSON.
pub fn read_message<R: BufRead>(reader: &mut R) -> LspResult<String> {
    // 1. Leer cabeceras línea a línea hasta encontrar una línea vacía.
    let mut content_length: Option<usize> = None;

    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            return Err(LspError::Framing("EOF while reading headers".into()));
        }

        // Las líneas terminan en \r\n.
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            // Fin de cabeceras.
            break;
        }

        // Parsear `Content-Length: N`.
        let lower = trimmed.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-length:") {
            let value = rest.trim();
            let n: usize = value
                .parse()
                .map_err(|e| LspError::Framing(format!("invalid Content-Length: {}", e)))?;
            content_length = Some(n);
        }
        // Ignoramos otras cabeceras (Content-Type, etc).
    }

    let len =
        content_length.ok_or_else(|| LspError::Framing("missing Content-Length header".into()))?;

    // 2. Leer exactamente `len` bytes.
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;

    // 3. Convertir a String UTF-8.
    String::from_utf8(buf).map_err(|e| LspError::Framing(format!("invalid UTF-8 in body: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn write_message_produces_valid_framing() {
        let mut buf = Vec::new();
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
        write_message(&mut buf, json).unwrap();

        let output = String::from_utf8(buf).unwrap();
        assert!(output.starts_with("Content-Length: "));
        assert!(output.contains("\r\nContent-Type: application/vscode-jsonrpc"));
        assert!(output.contains("\r\n\r\n"));
        assert!(output.ends_with(json));
    }

    #[test]
    fn write_then_read_roundtrip() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{}}"#;
        let mut buf = Vec::new();
        write_message(&mut buf, json).unwrap();

        let mut cursor = Cursor::new(buf);
        let read = read_message(&mut cursor).unwrap();
        assert_eq!(read, json);
    }

    #[test]
    fn read_handles_correct_content_length() {
        // Mensaje con contenido UTF-8 (acentos, emoji...).
        let json = r#"{"text":"hola ¿qué tal? 🎉"}"#;
        let mut buf = Vec::new();
        write_message(&mut buf, json).unwrap();

        let mut cursor = Cursor::new(buf);
        let read = read_message(&mut cursor).unwrap();
        assert_eq!(read, json);
    }

    #[test]
    fn read_fails_on_empty_input() {
        let mut cursor = Cursor::new(Vec::<u8>::new());
        let err = read_message(&mut cursor).unwrap_err();
        assert!(matches!(err, LspError::Framing(_)));
    }

    #[test]
    fn read_fails_on_missing_content_length() {
        let input = b"Content-Type: application/json\r\n\r\n{}";
        let mut cursor = Cursor::new(&input[..]);
        let err = read_message(&mut cursor).unwrap_err();
        match err {
            LspError::Framing(msg) => assert!(msg.contains("missing Content-Length")),
            other => panic!("expected Framing error, got {:?}", other),
        }
    }

    #[test]
    fn read_fails_on_invalid_content_length() {
        let input = b"Content-Length: not-a-number\r\n\r\n";
        let mut cursor = Cursor::new(&input[..]);
        let err = read_message(&mut cursor).unwrap_err();
        match err {
            LspError::Framing(msg) => assert!(msg.contains("invalid Content-Length")),
            other => panic!("expected Framing error, got {:?}", other),
        }
    }

    #[test]
    fn read_fails_on_invalid_utf8() {
        let invalid_bytes: &[u8] = &[0xFF, 0xFE, 0xFD];
        let mut input = b"Content-Length: 3\r\n\r\n".to_vec();
        input.extend_from_slice(invalid_bytes);

        let mut cursor = Cursor::new(input);
        let err = read_message(&mut cursor).unwrap_err();
        match err {
            LspError::Framing(msg) => assert!(msg.contains("UTF-8")),
            other => panic!("expected Framing error, got {:?}", other),
        }
    }

    #[test]
    fn read_ignores_extra_headers() {
        let json = r#"{"test":true}"#;
        let input = format!(
            "Content-Length: {}\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8\r\nX-Custom: something\r\n\r\n{}",
            json.len(),
            json
        );

        let mut cursor = Cursor::new(input.as_bytes());
        let read = read_message(&mut cursor).unwrap();
        assert_eq!(read, json);
    }

    #[test]
    fn content_length_counts_bytes_not_chars() {
        // "á" son 2 bytes en UTF-8.
        let json = r#"{"x":"á"}"#;
        assert_eq!(json.len(), 10); // 11 chars visibles + "á" es 2 bytes.

        let mut buf = Vec::new();
        write_message(&mut buf, json).unwrap();

        let header_end = buf.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let header = String::from_utf8_lossy(&buf[..header_end]);
        assert!(header.contains("Content-Length: 10"));
    }
}
