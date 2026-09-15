//! `ShellTool` — ejecución de comandos validados.
//!
//! Ver `mod.rs` para el diseño general.

use crate::{
    error::{ToolError, ToolResult},
    shell::allowlist::AllowList,
    tool::{RiskLevel, Tool, ToolCategory, ToolMetadata},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Timeout por defecto en segundos.
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Timeout máximo permitido.
const MAX_TIMEOUT_SECS: u64 = 600;

/// Tamaño máximo de stdout/stderr a devolver (bytes).
const MAX_OUTPUT_BYTES: usize = 100_000;

/// Resultado de ejecutar un comando.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellResult {
    pub program: String,
    pub args: Vec<String>,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub success: bool,
    pub truncated: bool,
}

/// Tool de ejecución de comandos validados.
#[derive(Debug)]
pub struct ShellTool {
    metadata: ToolMetadata,
    cwd: PathBuf,
    allowlist: AllowList,
}

impl ShellTool {
    /// Crea un `ShellTool` con la lista blanca por defecto y un `cwd`
    /// confinado.
    pub fn new(cwd: impl AsRef<Path>) -> ToolResult<Self> {
        let cwd = cwd.as_ref();
        if !cwd.exists() {
            return Err(ToolError::PathNotFound(format!(
                "shell cwd does not exist: {}",
                cwd.display()
            )));
        }
        let cwd = cwd.canonicalize()?;
        Ok(Self {
            metadata: ToolMetadata {
                name: "shell".into(),
                description: "Execute a whitelisted program with validated arguments. \
                              No shell interpretation, no pipes, no redirection."
                    .into(),
                category: ToolCategory::Shell,
                risk: RiskLevel::High,
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "program": {
                            "type": "string",
                            "description": "Program name (must be in the allowlist)"
                        },
                        "args": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "timeout_secs": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": MAX_TIMEOUT_SECS
                        }
                    },
                    "required": ["program"]
                }),
                output_schema: json!({ "type": "object" }),
            },
            cwd,
            allowlist: AllowList::defaults(),
        })
    }

    /// Sobrescribe la lista blanca.
    pub fn with_allowlist(mut self, allowlist: AllowList) -> Self {
        self.allowlist = allowlist;
        self
    }

    /// Acceso a la lista blanca.
    pub fn allowlist(&self) -> &AllowList {
        &self.allowlist
    }

    /// Acceso al cwd.
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// Ejecuta un comando con argumentos y timeout.
    pub fn run(
        &self,
        program: &str,
        args: &[String],
        timeout_secs: u64,
    ) -> ToolResult<ShellResult> {
        if program.is_empty() {
            return Err(ToolError::MissingArgument("program".into()));
        }
        if !self.allowlist.is_allowed(program) {
            return Err(ToolError::InvalidInput(format!(
                "program '{}' is not in the allowlist. \
                 To allow it, add it explicitly.",
                program
            )));
        }

        for arg in args {
            if contains_shell_metachars(arg) {
                return Err(ToolError::InvalidInput(format!(
                    "argument '{}' contains shell metacharacters \
                     ('|', '>', '<', '&', ';', '$', '`', etc.). \
                     These are not interpreted.",
                    arg
                )));
            }
        }

        let timeout_secs = timeout_secs.clamp(1, MAX_TIMEOUT_SECS);
        let timeout = Duration::from_secs(timeout_secs);

        let start = Instant::now();
        let mut cmd = Command::new(program);
        cmd.args(args)
            .current_dir(&self.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            ToolError::Io(std::io::Error::new(
                e.kind(),
                format!("failed to spawn '{}': {}", program, e),
            ))
        })?;

        let deadline = Instant::now() + timeout;
        loop {
            match child.try_wait() {
                Ok(Some(_status)) => break,
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(ToolError::InvalidInput(format!(
                            "command '{}' exceeded timeout of {}s",
                            program, timeout_secs
                        )));
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(e) => {
                    return Err(ToolError::Io(e));
                }
            }
        }

        let output = child.wait_with_output()?;
        let duration_ms = start.elapsed().as_millis() as u64;

        let mut truncated = false;
        let stdout = read_capped(&output.stdout, &mut truncated);
        let stderr = read_capped(&output.stderr, &mut truncated);

        let exit_code = output.status.code().unwrap_or(-1);
        let success = output.status.success();

        Ok(ShellResult {
            program: program.to_string(),
            args: args.to_vec(),
            stdout,
            stderr,
            exit_code,
            duration_ms,
            success,
            truncated,
        })
    }
}

impl Tool for ShellTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn invoke(&self, input: Value) -> ToolResult<Value> {
        let program = input
            .get("program")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::MissingArgument("program".into()))?;

        let args: Vec<String> = match input.get("args") {
            Some(Value::Array(arr)) => {
                let mut out = Vec::with_capacity(arr.len());
                for v in arr {
                    let s = v.as_str().ok_or_else(|| {
                        ToolError::InvalidInput("args must be an array of strings".into())
                    })?;
                    out.push(s.to_string());
                }
                out
            }
            None => Vec::new(),
            _ => {
                return Err(ToolError::InvalidInput(
                    "args must be an array of strings".into(),
                ))
            }
        };

        let timeout_secs = input
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        let result = self.run(program, &args, timeout_secs)?;

        Ok(json!({
            "program": result.program,
            "args": result.args,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "exit_code": result.exit_code,
            "duration_ms": result.duration_ms,
            "success": result.success,
            "truncated": result.truncated,
        }))
    }
}

/// Detecta metacaracteres de shell en un argumento.
fn contains_shell_metachars(s: &str) -> bool {
    s.contains('|')
        || s.contains('>')
        || s.contains('<')
        || s.contains('&')
        || s.contains(';')
        || s.contains('`')
        || s.contains("$(")
}

/// Lee hasta `MAX_OUTPUT_BYTES`. Si hay más, marca `truncated = true`.
fn read_capped(bytes: &[u8], truncated: &mut bool) -> String {
    if bytes.len() <= MAX_OUTPUT_BYTES {
        String::from_utf8_lossy(bytes).to_string()
    } else {
        *truncated = true;
        let mut s = String::from_utf8_lossy(&bytes[..MAX_OUTPUT_BYTES]).to_string();
        s.push_str("\n... [output truncated]");
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup() -> (TempDir, ShellTool) {
        let dir = TempDir::new().unwrap();
        let tool = ShellTool::new(dir.path()).unwrap();
        (dir, tool)
    }

    #[test]
    fn metadata_is_correct() {
        let (_dir, tool) = setup();
        let m = tool.metadata();
        assert_eq!(m.name, "shell");
        assert_eq!(m.category, ToolCategory::Shell);
        assert_eq!(m.risk, RiskLevel::High);
    }

    #[test]
    fn new_fails_if_cwd_does_not_exist() {
        let result = ShellTool::new("/nonexistent/xyz123");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_program_not_in_allowlist() {
        let (_dir, tool) = setup();
        let err = tool.run("sudo", &[], 10).unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn rejects_rm() {
        let (_dir, tool) = setup();
        let err = tool.run("rm", &["-rf".into(), "/".into()], 10).unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn rejects_sh() {
        let (_dir, tool) = setup();
        let err = tool
            .run("sh", &["-c".into(), "echo hi".into()], 10)
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn rejects_empty_program() {
        let (_dir, tool) = setup();
        let err = tool.run("", &[], 10).unwrap_err();
        assert!(matches!(err, ToolError::MissingArgument(_)));
    }

    #[test]
    fn rejects_args_with_pipe() {
        let (_dir, tool) = setup();
        let err = tool.run("echo", &["hi | cat".into()], 10).unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn rejects_args_with_redirection() {
        let (_dir, tool) = setup();
        let err = tool.run("echo", &["hi > file".into()], 10).unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn rejects_args_with_semicolon() {
        let (_dir, tool) = setup();
        let err = tool.run("echo", &["hi; rm".into()], 10).unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn rejects_args_with_command_substitution() {
        let (_dir, tool) = setup();
        let err = tool.run("echo", &["$(whoami)".into()], 10).unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn rejects_args_with_backtick() {
        let (_dir, tool) = setup();
        let err = tool.run("echo", &["`whoami`".into()], 10).unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn executes_echo() {
        let (_dir, tool) = setup();
        let result = tool.run("echo", &["hello".into()], 10).unwrap();
        assert!(result.success);
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout.trim(), "hello");
    }

    #[test]
    fn captures_nonzero_exit_code() {
        let (_dir, tool) = setup();
        let result = tool
            .run("ls", &["definitely_nonexistent_file_xyz".into()], 10)
            .unwrap();
        assert!(!result.success);
        assert_ne!(result.exit_code, 0);
        assert!(!result.stderr.is_empty());
    }

    #[test]
    fn executes_ls_in_cwd() {
        let (dir, tool) = setup();
        fs::write(dir.path().join("test_file.txt"), "hi").unwrap();

        let result = tool.run("ls", &[], 10).unwrap();
        assert!(result.success);
        assert!(result.stdout.contains("test_file.txt"));
    }

    #[test]
    fn timeout_with_custom_allowlist() {
        let dir = TempDir::new().unwrap();
        let mut allowlist = AllowList::defaults();
        allowlist.allow("sleep");
        let tool = ShellTool::new(dir.path())
            .unwrap()
            .with_allowlist(allowlist);

        // Pedimos 5s de sleep con timeout de 1s → debe fallar.
        let err = tool.run("sleep", &["5".into()], 1).unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn invoke_via_trait() {
        let (_dir, tool) = setup();
        let out = tool
            .invoke(json!({"program": "echo", "args": ["via_trait"]}))
            .unwrap();
        assert_eq!(out["program"], "echo");
        assert_eq!(out["success"], true);
        assert_eq!(out["stdout"].as_str().unwrap().trim(), "via_trait");
    }

    #[test]
    fn invoke_missing_program_fails() {
        let (_dir, tool) = setup();
        let err = tool.invoke(json!({"args": []})).unwrap_err();
        assert!(matches!(err, ToolError::MissingArgument(_)));
    }

    #[test]
    fn invoke_invalid_args_type_fails() {
        let (_dir, tool) = setup();
        let err = tool
            .invoke(json!({"program": "echo", "args": "not an array"}))
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn invoke_empty_args_ok() {
        let (_dir, tool) = setup();
        // `ls` sin args lista el cwd.
        let out = tool.invoke(json!({"program": "ls"})).unwrap();
        assert_eq!(out["success"], true);
    }

    #[test]
    fn invoke_default_timeout_is_60() {
        let (_dir, tool) = setup();
        let out = tool
            .invoke(json!({"program": "echo", "args": ["x"]}))
            .unwrap();
        assert_eq!(out["success"], true);
    }

    #[test]
    fn timeout_is_clamped_to_max() {
        let (_dir, tool) = setup();
        let out = tool
            .invoke(json!({"program": "echo", "args": ["x"], "timeout_secs": 10000}))
            .unwrap();
        assert_eq!(out["success"], true);
    }
}
