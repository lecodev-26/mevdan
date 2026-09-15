//! Verificador: ejecuta un comando y comprueba el exit code.
//!
//! Usa `mevdan-tools::ShellTool` para ejecutar el comando en el
//! sandbox del proyecto.

use crate::{
    claim::{Claim, ClaimKind},
    evidence::Evidence,
    verifier::{VerificationContext, VerificationOutcome, Verifier},
};
use mevdan_tools::{ShellTool, ToolError};

/// Verifica claims de tipo `CommandRan` ejecutando el comando en el
/// sandbox y comprobando el exit code.
#[derive(Debug)]
pub struct CommandExitVerifier {
    /// Timeout por defecto en segundos.
    timeout_secs: u64,
}

impl CommandExitVerifier {
    pub fn new() -> Self {
        Self { timeout_secs: 60 }
    }

    /// Sobrescribe el timeout.
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

impl Default for CommandExitVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Verifier for CommandExitVerifier {
    fn name(&self) -> &str {
        "command_exit"
    }

    fn can_verify(&self, claim: &Claim) -> bool {
        matches!(claim.kind, ClaimKind::CommandRan | ClaimKind::TestPassed)
    }

    fn verify(&self, claim: &Claim, ctx: &VerificationContext) -> VerificationOutcome {
        // 1. Extraer programa y args del claim.
        let program = match claim.data.get("program").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => {
                return VerificationOutcome::unknown("claim does not specify a program");
            }
        };

        let args: Vec<String> = claim
            .data
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        // 2. Exit code esperado (por defecto 0).
        let expected_exit = claim
            .data
            .get("exit_code")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        // 3. Crear ShellTool apuntando al sandbox.
        let shell = match ShellTool::new(&ctx.sandbox_root) {
            Ok(s) => s,
            Err(e) => {
                return VerificationOutcome::unknown(format!("failed to create ShellTool: {}", e));
            }
        };

        // 4. Ejecutar.
        let result = match shell.run(program, &args, self.timeout_secs) {
            Ok(r) => r,
            Err(ToolError::InvalidInput(msg)) => {
                // Comando no en allowlist, timeout, etc.
                return VerificationOutcome::unknown(format!("cannot run command: {}", msg));
            }
            Err(e) => {
                return VerificationOutcome::unknown(format!("failed to run command: {}", e));
            }
        };

        // 5. Comparar exit code.
        if result.exit_code == expected_exit {
            let ev =
                Evidence::command_success(program, result.exit_code).with_data(serde_json::json!({
                    "program": program,
                    "args": args,
                    "exit_code": result.exit_code,
                    "stdout_preview": truncate(&result.stdout, 200),
                }));
            VerificationOutcome::verified(format!(
                "{} exited with expected code {}",
                program, expected_exit
            ))
            .with_evidence(ev)
        } else {
            VerificationOutcome::failed(format!(
                "{} exited with {} (expected {})",
                program, result.exit_code, expected_exit
            ))
        }
    }
}

/// Trunca un string para preview.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut end = max;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        format!("{}...", &s[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn can_verify_command_claims() {
        let v = CommandExitVerifier::new();
        assert!(v.can_verify(&Claim::command_ran("cargo", 0)));
        assert!(v.can_verify(&Claim::test_passed("t")));
        assert!(!v.can_verify(&Claim::file_created("x")));
    }

    #[test]
    fn verify_echo_command() {
        let dir = TempDir::new().unwrap();
        let v = CommandExitVerifier::new();
        let ctx = VerificationContext::new(dir.path());

        // `echo` está en la allowlist.
        let mut claim = Claim::new(ClaimKind::CommandRan, "echo runs");
        claim.data = serde_json::json!({
            "program": "echo",
            "args": ["hi"],
            "exit_code": 0,
        });

        let outcome = v.verify(&claim, &ctx);
        assert!(outcome.is_verified());
        assert_eq!(outcome.evidence.len(), 1);
    }

    #[test]
    fn verify_wrong_exit_code_fails() {
        let dir = TempDir::new().unwrap();
        let v = CommandExitVerifier::new();
        let ctx = VerificationContext::new(dir.path());

        // `ls nonexistent` sale con código != 0, pero esperamos 0.
        let mut claim = Claim::new(ClaimKind::CommandRan, "ls fails");
        claim.data = serde_json::json!({
            "program": "ls",
            "args": ["definitely_nonexistent_xyz"],
            "exit_code": 0,
        });

        let outcome = v.verify(&claim, &ctx);
        assert!(outcome.is_failed());
    }

    #[test]
    fn verify_unknown_program_is_unknown() {
        let dir = TempDir::new().unwrap();
        let v = CommandExitVerifier::new();
        let ctx = VerificationContext::new(dir.path());

        // `sudo` no está en la allowlist.
        let mut claim = Claim::new(ClaimKind::CommandRan, "sudo fails");
        claim.data = serde_json::json!({
            "program": "sudo",
            "args": ["ls"],
            "exit_code": 0,
        });

        let outcome = v.verify(&claim, &ctx);
        assert_eq!(outcome.status, crate::claim::ClaimStatus::Unknown);
    }

    #[test]
    fn verify_claim_without_program_is_unknown() {
        let dir = TempDir::new().unwrap();
        let v = CommandExitVerifier::new();
        let ctx = VerificationContext::new(dir.path());

        let mut claim = Claim::new(ClaimKind::CommandRan, "x");
        claim.data = serde_json::json!({});

        let outcome = v.verify(&claim, &ctx);
        assert_eq!(outcome.status, crate::claim::ClaimStatus::Unknown);
    }

    #[test]
    fn truncate_works() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("hello world", 5), "hello...");
    }
}
