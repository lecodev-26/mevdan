//! `GitTool` — operaciones de git validadas.
//!
//! Ver `mod.rs` para el diseño general.

use crate::{
    error::{ToolError, ToolResult},
    tool::{RiskLevel, Tool, ToolCategory, ToolMetadata},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Timeout por defecto para comandos git.
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Timeout máximo.
const MAX_TIMEOUT_SECS: u64 = 300;

/// Máximo de bytes de stdout/stderr.
const MAX_OUTPUT_BYTES: usize = 100_000;

/// Autor por defecto para commits.
const DEFAULT_AUTHOR_NAME: &str = "MEVDAN";
const DEFAULT_AUTHOR_EMAIL: &str = "mevdan@localhost";

/// Resultado de ejecutar un comando git.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitResult {
    pub subcommand: String,
    pub args: Vec<String>,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub success: bool,
    pub truncated: bool,
}

/// Tool de git.
#[derive(Debug)]
pub struct GitTool {
    metadata: ToolMetadata,
    cwd: PathBuf,
    author_name: String,
    author_email: String,
}

impl GitTool {
    /// Crea un `GitTool` con un `cwd` que debe estar dentro de un
    /// repositorio git válido.
    pub fn new(cwd: impl AsRef<Path>) -> ToolResult<Self> {
        let cwd = cwd.as_ref();
        if !cwd.exists() {
            return Err(ToolError::PathNotFound(format!(
                "git cwd does not exist: {}",
                cwd.display()
            )));
        }
        let cwd = cwd.canonicalize()?;

        if !is_inside_git_repo(&cwd) {
            return Err(ToolError::InvalidInput(format!(
                "not inside a git repository: {}",
                cwd.display()
            )));
        }

        Ok(Self {
            metadata: ToolMetadata {
                name: "git".into(),
                description: "Execute validated git operations (status, log, diff, \
                              add, commit, branch, checkout, restore). \
                              Push is NOT supported."
                    .into(),
                category: ToolCategory::Git,
                risk: RiskLevel::Medium,
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": [
                                "status", "log", "diff", "branch_list", "show",
                                "add", "commit", "branch_create", "checkout", "restore"
                            ]
                        },
                        "args": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "message": {
                            "type": "string",
                            "description": "Commit message (for action=commit)"
                        },
                        "paths": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "File paths (for add/restore)"
                        },
                        "name": {
                            "type": "string",
                            "description": "Branch name (for action=branch_create/checkout)"
                        }
                    },
                    "required": ["action"]
                }),
                output_schema: json!({ "type": "object" }),
            },
            cwd,
            author_name: DEFAULT_AUTHOR_NAME.to_string(),
            author_email: DEFAULT_AUTHOR_EMAIL.to_string(),
        })
    }

    /// Sobrescribe el autor de commits.
    pub fn with_author(mut self, name: impl Into<String>, email: impl Into<String>) -> Self {
        self.author_name = name.into();
        self.author_email = email.into();
        self
    }

    /// Acceso al cwd.
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// Ejecuta un comando git en bruto (uso interno).
    fn run_git(&self, args: &[&str]) -> ToolResult<GitResult> {
        let mut cmd = Command::new("git");
        cmd.args(["--no-pager"])
            .args(args)
            .current_dir(&self.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_EDITOR", "true")
            .env("GIT_PAGER", "cat");

        let mut child = cmd.spawn().map_err(|e| {
            ToolError::Io(std::io::Error::new(
                e.kind(),
                format!("failed to spawn git: {}", e),
            ))
        })?;

        let timeout = Duration::from_secs(DEFAULT_TIMEOUT_SECS.min(MAX_TIMEOUT_SECS));
        let start = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => {
                    if start.elapsed() > timeout {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(ToolError::InvalidInput(format!(
                            "git command exceeded timeout of {}s",
                            DEFAULT_TIMEOUT_SECS
                        )));
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(e) => return Err(ToolError::Io(e)),
            }
        }

        let output = child.wait_with_output()?;
        let mut truncated = false;
        let stdout = read_capped(&output.stdout, &mut truncated);
        let stderr = read_capped(&output.stderr, &mut truncated);
        let exit_code = output.status.code().unwrap_or(-1);
        let success = output.status.success();

        Ok(GitResult {
            subcommand: args.first().unwrap_or(&"").to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            stdout,
            stderr,
            exit_code,
            success,
            truncated,
        })
    }

    /// Despacha una acción.
    fn dispatch(&self, input: &Value) -> ToolResult<Value> {
        let action = input
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::MissingArgument("action".into()))?;

        match action {
            "status" => self.op_status(),
            "log" => self.op_log(input),
            "diff" => self.op_diff(input),
            "branch_list" => self.op_branch_list(),
            "show" => self.op_show(input),
            "add" => self.op_add(input),
            "commit" => self.op_commit(input),
            "branch_create" => self.op_branch_create(input),
            "checkout" => self.op_checkout(input),
            "restore" => self.op_restore(input),
            other => Err(ToolError::InvalidInput(format!(
                "unknown git action: {}",
                other
            ))),
        }
    }

    // ──────────────────────────────────────────────
    // Read-only
    // ──────────────────────────────────────────────

    fn op_status(&self) -> ToolResult<Value> {
        let r = self.run_git(&["status", "--porcelain=v1", "--branch"])?;
        let lines: Vec<String> = r
            .stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|s| s.to_string())
            .collect();
        Ok(json!({
            "action": "status",
            "branch_line": lines.first().cloned().unwrap_or_default(),
            "changes": lines.iter().skip(1).cloned().collect::<Vec<_>>(),
            "clean": lines.len() <= 1,
            "exit_code": r.exit_code,
            "success": r.success,
            "raw": r.stdout,
        }))
    }

    fn op_log(&self, input: &Value) -> ToolResult<Value> {
        let limit = input
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .min(100);
        let limit_str = format!("-{}", limit);
        let r = self.run_git(&["log", &limit_str, "--pretty=format:%H|%an|%ae|%aI|%s"])?;

        let commits: Vec<Value> = r
            .stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| {
                let parts: Vec<&str> = line.splitn(5, '|').collect();
                json!({
                    "hash": parts.first().copied().unwrap_or(""),
                    "author_name": parts.get(1).copied().unwrap_or(""),
                    "author_email": parts.get(2).copied().unwrap_or(""),
                    "date": parts.get(3).copied().unwrap_or(""),
                    "subject": parts.get(4).copied().unwrap_or(""),
                })
            })
            .collect();

        Ok(json!({
            "action": "log",
            "commits": commits,
            "count": commits.len(),
            "exit_code": r.exit_code,
            "success": r.success,
        }))
    }

    fn op_diff(&self, input: &Value) -> ToolResult<Value> {
        let staged = input
            .get("staged")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut args: Vec<&str> = vec!["diff"];
        if staged {
            args.push("--staged");
        }
        let r = self.run_git(&args)?;

        Ok(json!({
            "action": "diff",
            "staged": staged,
            "diff": r.stdout,
            "empty": r.stdout.trim().is_empty(),
            "exit_code": r.exit_code,
            "success": r.success,
        }))
    }

    fn op_branch_list(&self) -> ToolResult<Value> {
        let r = self.run_git(&["branch", "--list", "--format=%(refname:short)|%(HEAD)"])?;
        let branches: Vec<Value> = r
            .stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| {
                let parts: Vec<&str> = line.splitn(2, '|').collect();
                json!({
                    "name": parts.first().copied().unwrap_or(""),
                    "current": parts.get(1).copied() == Some("*"),
                })
            })
            .collect();

        Ok(json!({
            "action": "branch_list",
            "branches": branches,
            "count": branches.len(),
            "exit_code": r.exit_code,
            "success": r.success,
        }))
    }

    fn op_show(&self, input: &Value) -> ToolResult<Value> {
        let target = input
            .get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("HEAD");
        validate_ref(target)?;

        let r = self.run_git(&["show", "--stat", "--no-patch", target])?;

        Ok(json!({
            "action": "show",
            "target": target,
            "output": r.stdout,
            "exit_code": r.exit_code,
            "success": r.success,
        }))
    }

    // ──────────────────────────────────────────────
    // Write
    // ──────────────────────────────────────────────

    fn op_add(&self, input: &Value) -> ToolResult<Value> {
        let paths = extract_string_array(input, "paths")?;
        if paths.is_empty() {
            return Err(ToolError::MissingArgument("paths".into()));
        }

        for p in &paths {
            validate_path(p)?;
        }

        let mut args: Vec<&str> = vec!["add", "--"];
        for p in &paths {
            args.push(p);
        }
        let r = self.run_git(&args)?;

        Ok(json!({
            "action": "add",
            "paths": paths,
            "exit_code": r.exit_code,
            "success": r.success,
            "stderr": r.stderr,
        }))
    }

    fn op_commit(&self, input: &Value) -> ToolResult<Value> {
        let message = input
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::MissingArgument("message".into()))?;
        if message.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "commit message cannot be empty".into(),
            ));
        }

        let author_arg = format!("user.name={}", self.author_name);
        let email_arg = format!("user.email={}", self.author_email);

        let r = self.run_git(&[
            "-c",
            &author_arg,
            "-c",
            &email_arg,
            "commit",
            "-m",
            message,
            "--no-edit",
        ])?;

        Ok(json!({
            "action": "commit",
            "message": message,
            "exit_code": r.exit_code,
            "success": r.success,
            "stdout": r.stdout,
            "stderr": r.stderr,
        }))
    }

    fn op_branch_create(&self, input: &Value) -> ToolResult<Value> {
        let name = input
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::MissingArgument("name".into()))?;
        validate_branch_name(name)?;

        let r = self.run_git(&["branch", name])?;

        Ok(json!({
            "action": "branch_create",
            "name": name,
            "exit_code": r.exit_code,
            "success": r.success,
            "stderr": r.stderr,
        }))
    }

    fn op_checkout(&self, input: &Value) -> ToolResult<Value> {
        let name = input
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::MissingArgument("name".into()))?;
        validate_ref(name)?;

        let r = self.run_git(&["checkout", name])?;

        Ok(json!({
            "action": "checkout",
            "name": name,
            "exit_code": r.exit_code,
            "success": r.success,
            "stdout": r.stdout,
            "stderr": r.stderr,
        }))
    }

    fn op_restore(&self, input: &Value) -> ToolResult<Value> {
        let paths = extract_string_array(input, "paths")?;
        if paths.is_empty() {
            return Err(ToolError::MissingArgument("paths".into()));
        }
        for p in &paths {
            validate_path(p)?;
        }

        let mut args: Vec<&str> = vec!["restore", "--"];
        for p in &paths {
            args.push(p);
        }
        let r = self.run_git(&args)?;

        Ok(json!({
            "action": "restore",
            "paths": paths,
            "exit_code": r.exit_code,
            "success": r.success,
            "stderr": r.stderr,
        }))
    }
}

impl Tool for GitTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn invoke(&self, input: Value) -> ToolResult<Value> {
        if input.get("action").is_none() {
            return Err(ToolError::MissingArgument("action".into()));
        }
        self.dispatch(&input)
    }
}

// ──────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────

fn is_inside_git_repo(cwd: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn extract_string_array(input: &Value, key: &str) -> ToolResult<Vec<String>> {
    match input.get(key) {
        Some(Value::Array(arr)) => {
            let mut out = Vec::with_capacity(arr.len());
            for v in arr {
                let s = v.as_str().ok_or_else(|| {
                    ToolError::InvalidInput(format!("{} must be an array of strings", key))
                })?;
                out.push(s.to_string());
            }
            Ok(out)
        }
        None => Ok(Vec::new()),
        _ => Err(ToolError::InvalidInput(format!(
            "{} must be an array of strings",
            key
        ))),
    }
}

fn validate_path(p: &str) -> ToolResult<()> {
    if p.is_empty() {
        return Err(ToolError::InvalidInput("path cannot be empty".into()));
    }
    if p.contains("..") {
        return Err(ToolError::InvalidInput(format!(
            "path contains '..': {}",
            p
        )));
    }
    if p.contains(['|', ';', '&', '`', '$']) {
        return Err(ToolError::InvalidInput(format!(
            "path contains shell metacharacters: {}",
            p
        )));
    }
    Ok(())
}

fn validate_branch_name(name: &str) -> ToolResult<()> {
    if name.is_empty() || name.len() > 200 {
        return Err(ToolError::InvalidInput(format!(
            "invalid branch name: {}",
            name
        )));
    }
    if name.starts_with('-') || name.ends_with('/') || name.contains("..") {
        return Err(ToolError::InvalidInput(format!(
            "invalid branch name: {}",
            name
        )));
    }
    if name.contains([' ', '\t', '~', '^', ':', '\\', '*']) {
        return Err(ToolError::InvalidInput(format!(
            "invalid branch name: {}",
            name
        )));
    }
    Ok(())
}

fn validate_ref(r: &str) -> ToolResult<()> {
    if r.is_empty() || r.len() > 200 {
        return Err(ToolError::InvalidInput(format!("invalid ref: {}", r)));
    }
    if r.starts_with('-') {
        return Err(ToolError::InvalidInput(format!(
            "ref cannot start with '-': {}",
            r
        )));
    }
    if r.contains([' ', '\t', '\n', ';', '&', '|']) {
        return Err(ToolError::InvalidInput(format!("invalid ref: {}", r)));
    }
    Ok(())
}

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
    use std::process::Command as StdCommand;
    use tempfile::TempDir;

    fn init_git_repo(dir: &Path) {
        let run = |args: &[&str]| {
            let out = StdCommand::new("git")
                .args(args)
                .current_dir(dir)
                .env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@example.com")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_COMMITTER_EMAIL", "test@example.com")
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        };
        run(&["init", "-q", "-b", "main"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test"]);
        // Evita conversión CRLF en Windows.
        run(&["config", "core.autocrlf", "false"]);
    }

    fn setup_with_repo() -> (TempDir, GitTool) {
        let dir = TempDir::new().unwrap();
        init_git_repo(dir.path());
        fs::write(dir.path().join("README.md"), "# Test\n").unwrap();
        StdCommand::new("git")
            .args(["add", "README.md"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        StdCommand::new("git")
            .args(["commit", "-m", "initial", "--no-edit"])
            .current_dir(dir.path())
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .output()
            .unwrap();

        let tool = GitTool::new(dir.path()).unwrap();
        (dir, tool)
    }

    #[test]
    fn metadata_is_correct() {
        let (_dir, tool) = setup_with_repo();
        let m = tool.metadata();
        assert_eq!(m.name, "git");
        assert_eq!(m.category, ToolCategory::Git);
        assert_eq!(m.risk, RiskLevel::Medium);
    }

    #[test]
    fn new_fails_if_not_a_git_repo() {
        let dir = TempDir::new().unwrap();
        let result = GitTool::new(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn new_fails_if_path_does_not_exist() {
        let result = GitTool::new("/nonexistent/xyz123");
        assert!(result.is_err());
    }

    #[test]
    fn status_clean_repo() {
        let (_dir, tool) = setup_with_repo();
        let out = tool.invoke(json!({"action": "status"})).unwrap();
        assert_eq!(out["success"], true);
        assert_eq!(out["clean"], true);
    }

    #[test]
    fn status_with_changes() {
        let (dir, tool) = setup_with_repo();
        fs::write(dir.path().join("new.txt"), "hi").unwrap();
        let out = tool.invoke(json!({"action": "status"})).unwrap();
        assert_eq!(out["success"], true);
        assert_eq!(out["clean"], false);
    }

    #[test]
    fn log_returns_commits() {
        let (_dir, tool) = setup_with_repo();
        let out = tool.invoke(json!({"action": "log"})).unwrap();
        assert_eq!(out["success"], true);
        let commits = out["commits"].as_array().unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0]["subject"], "initial");
    }

    #[test]
    fn log_respects_limit() {
        let (_dir, tool) = setup_with_repo();
        let out = tool.invoke(json!({"action": "log", "limit": 5})).unwrap();
        assert_eq!(out["success"], true);
    }

    #[test]
    fn diff_empty_when_clean() {
        let (_dir, tool) = setup_with_repo();
        let out = tool.invoke(json!({"action": "diff"})).unwrap();
        assert_eq!(out["empty"], true);
    }

    #[test]
    fn diff_shows_changes() {
        let (dir, tool) = setup_with_repo();
        fs::write(dir.path().join("README.md"), "# Changed\n").unwrap();
        let out = tool.invoke(json!({"action": "diff"})).unwrap();
        assert_eq!(out["empty"], false);
    }

    #[test]
    fn branch_list_shows_main() {
        let (_dir, tool) = setup_with_repo();
        let out = tool.invoke(json!({"action": "branch_list"})).unwrap();
        assert_eq!(out["success"], true);
        let branches = out["branches"].as_array().unwrap();
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0]["name"], "main");
        assert_eq!(branches[0]["current"], true);
    }

    #[test]
    fn add_and_commit_flow() {
        let (dir, tool) = setup_with_repo();
        fs::write(dir.path().join("new.txt"), "new content").unwrap();

        let out = tool
            .invoke(json!({"action": "add", "paths": ["new.txt"]}))
            .unwrap();
        assert_eq!(out["success"], true);

        let out = tool
            .invoke(json!({"action": "commit", "message": "add new.txt"}))
            .unwrap();
        assert_eq!(out["success"], true);

        let out = tool.invoke(json!({"action": "log"})).unwrap();
        let commits = out["commits"].as_array().unwrap();
        assert_eq!(commits.len(), 2);
    }

    #[test]
    fn add_without_paths_fails() {
        let (_dir, tool) = setup_with_repo();
        let err = tool.invoke(json!({"action": "add"})).unwrap_err();
        assert!(matches!(err, ToolError::MissingArgument(_)));
    }

    #[test]
    fn add_with_parent_traversal_fails() {
        let (_dir, tool) = setup_with_repo();
        let err = tool
            .invoke(json!({"action": "add", "paths": ["../etc/passwd"]}))
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn commit_without_message_fails() {
        let (_dir, tool) = setup_with_repo();
        let err = tool.invoke(json!({"action": "commit"})).unwrap_err();
        assert!(matches!(err, ToolError::MissingArgument(_)));
    }

    #[test]
    fn commit_with_empty_message_fails() {
        let (_dir, tool) = setup_with_repo();
        let err = tool
            .invoke(json!({"action": "commit", "message": "  "}))
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn branch_create_and_checkout() {
        let (_dir, tool) = setup_with_repo();

        let out = tool
            .invoke(json!({"action": "branch_create", "name": "feature"}))
            .unwrap();
        assert_eq!(out["success"], true);

        let out = tool
            .invoke(json!({"action": "checkout", "name": "feature"}))
            .unwrap();
        assert_eq!(out["success"], true);

        let out = tool.invoke(json!({"action": "branch_list"})).unwrap();
        let branches = out["branches"].as_array().unwrap();
        let current = branches.iter().find(|b| b["current"] == true).unwrap();
        assert_eq!(current["name"], "feature");
    }

    #[test]
    fn branch_create_with_invalid_name_fails() {
        let (_dir, tool) = setup_with_repo();
        let err = tool
            .invoke(json!({"action": "branch_create", "name": "bad name"}))
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn checkout_with_invalid_ref_fails() {
        let (_dir, tool) = setup_with_repo();
        let err = tool
            .invoke(json!({"action": "checkout", "name": "--help"}))
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn restore_reverts_changes() {
        let (dir, tool) = setup_with_repo();
        let readme = dir.path().join("README.md");

        fs::write(&readme, "# Changed\n").unwrap();
        let out = tool.invoke(json!({"action": "diff"})).unwrap();
        assert_eq!(out["empty"], false);

        let out = tool
            .invoke(json!({"action": "restore", "paths": ["README.md"]}))
            .unwrap();
        assert_eq!(out["success"], true);

        let content = fs::read_to_string(&readme).unwrap();
        // En Windows, git puede restaurar con CRLF incluso con
        // `core.autocrlf=false` en algunos casos. Normalizamos antes
        // de comparar para que el test sea robusto cross-platform.
        let normalized = content.replace("\r\n", "\n");
        assert_eq!(normalized, "# Test\n");
    }

    #[test]
    fn restore_without_paths_fails() {
        let (_dir, tool) = setup_with_repo();
        let err = tool.invoke(json!({"action": "restore"})).unwrap_err();
        assert!(matches!(err, ToolError::MissingArgument(_)));
    }

    #[test]
    fn missing_action_fails() {
        let (_dir, tool) = setup_with_repo();
        let err = tool.invoke(json!({})).unwrap_err();
        assert!(matches!(err, ToolError::MissingArgument(_)));
    }

    #[test]
    fn unknown_action_fails() {
        let (_dir, tool) = setup_with_repo();
        let err = tool.invoke(json!({"action": "push"})).unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn show_head_works() {
        let (_dir, tool) = setup_with_repo();
        let out = tool.invoke(json!({"action": "show"})).unwrap();
        assert_eq!(out["success"], true);
    }

    #[test]
    fn validate_branch_name_rejects_dangerous() {
        assert!(validate_branch_name("ok-name").is_ok());
        assert!(validate_branch_name("").is_err());
        assert!(validate_branch_name("-bad").is_err());
        assert!(validate_branch_name("bad name").is_err());
        assert!(validate_branch_name("bad..name").is_err());
        assert!(validate_branch_name("bad:name").is_err());
    }

    #[test]
    fn validate_path_rejects_dangerous() {
        assert!(validate_path("ok.txt").is_ok());
        assert!(validate_path("dir/file.txt").is_ok());
        assert!(validate_path("").is_err());
        assert!(validate_path("../etc/passwd").is_err());
        assert!(validate_path("file;rm -rf").is_err());
    }
}
