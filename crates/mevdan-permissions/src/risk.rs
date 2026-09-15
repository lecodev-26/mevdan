//! Motor de evaluación de riesgo.
//!
//! Mientras que el **Permission Engine** decide *si* una acción se
//! puede ejecutar (`Allow`/`Ask`/`Deny`), el **Risk Engine** decide
//! *cuánto cuidado* merece (`Low`/`Medium`/`High`/`Critical`).
//!
//! ## Uso
//!
//! - **UI:** mostrar avisos al usuario según riesgo.
//! - **Auto-ASK:** subir de `Allow` a `Ask` si el riesgo es alto.
//! - **Observabilidad:** registrar el riesgo de cada operación.
//! - **Autonomy levels (Fase 90):** el riesgo determina qué puede
//!   hacer el agente sin supervisión.
//!
//! ## Sin ML, sin magia
//!
//! Las reglas son heurísticas explícitas y trazables. Cada evaluación
//! dice **por qué** tiene ese riesgo (lista de factores).

use crate::invocation::Invocation;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Nivel de riesgo.
///
/// Coincide con `mevdan_tools::RiskLevel` pero es independiente para
/// no acoplar los crates. Se puede convertir con `From`/`Into`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskAssessment {
    /// Sin efectos secundarios relevantes.
    #[default]
    Low,
    /// Efectos dentro del workspace.
    Medium,
    /// Efectos destructivos o fuera del workspace.
    High,
    /// Efectos irreversibles o peligrosos (sudo, rm -rf, push).
    Critical,
}

impl RiskAssessment {
    pub fn display_name(&self) -> &'static str {
        match self {
            RiskAssessment::Low => "LOW",
            RiskAssessment::Medium => "MEDIUM",
            RiskAssessment::High => "HIGH",
            RiskAssessment::Critical => "CRITICAL",
        }
    }

    /// Combina dos niveles quedándose con el **mayor**.
    pub fn combine(self, other: RiskAssessment) -> RiskAssessment {
        self.max(other)
    }
}

/// Factor que contribuye al riesgo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskFactor {
    /// La acción es destructiva (delete, reset --hard, etc.).
    DestructiveAction,
    /// El path está fuera del workspace.
    OutsideWorkspace,
    /// El comando tiene efectos secundarios irreversibles.
    Irreversible,
    /// La acción toca la red.
    Network,
    /// La acción ejecuta un binario del sistema.
    SystemBinary,
    /// Múltiples objetos afectados.
    BatchOperation,
    /// Se usa `sudo` o equivalente.
    PrivilegeEscalation,
}

impl RiskFactor {
    pub fn display_name(&self) -> &'static str {
        match self {
            RiskFactor::DestructiveAction => "destructive action",
            RiskFactor::OutsideWorkspace => "outside workspace",
            RiskFactor::Irreversible => "irreversible",
            RiskFactor::Network => "network access",
            RiskFactor::SystemBinary => "system binary",
            RiskFactor::BatchOperation => "batch operation",
            RiskFactor::PrivilegeEscalation => "privilege escalation",
        }
    }

    /// ¿Qué nivel de riesgo aporta este factor?
    pub fn level(&self) -> RiskAssessment {
        match self {
            RiskFactor::DestructiveAction => RiskAssessment::High,
            RiskFactor::OutsideWorkspace => RiskAssessment::High,
            RiskFactor::Irreversible => RiskAssessment::High,
            RiskFactor::Network => RiskAssessment::Medium,
            RiskFactor::SystemBinary => RiskAssessment::Medium,
            RiskFactor::BatchOperation => RiskAssessment::Medium,
            RiskFactor::PrivilegeEscalation => RiskAssessment::Critical,
        }
    }
}

/// Resultado completo de evaluar riesgo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskReport {
    /// Nivel final (el máximo de todos los factores, o `Low` si no
    /// hay factores).
    pub level: RiskAssessment,

    /// Factores detectados, ordenados y sin duplicados.
    pub factors: Vec<RiskFactor>,

    /// Razón textual breve.
    pub reason: String,
}

impl RiskReport {
    /// Reporte sin factores (riesgo mínimo).
    pub fn low() -> Self {
        Self {
            level: RiskAssessment::Low,
            factors: Vec::new(),
            reason: "no risk factors detected".to_string(),
        }
    }

    /// Construye un reporte a partir de una lista de factores.
    pub fn from_factors(factors: Vec<RiskFactor>) -> Self {
        if factors.is_empty() {
            return Self::low();
        }

        let mut set: BTreeSet<RiskFactor> = BTreeSet::new();
        for f in factors {
            set.insert(f);
        }
        let factors: Vec<RiskFactor> = set.into_iter().collect();

        let level = factors
            .iter()
            .map(|f| f.level())
            .max()
            .unwrap_or(RiskAssessment::Low);

        let reason = format!(
            "detected {} factor(s): {}",
            factors.len(),
            factors
                .iter()
                .map(|f| f.display_name())
                .collect::<Vec<_>>()
                .join(", ")
        );

        Self {
            level,
            factors,
            reason,
        }
    }

    /// ¿Es riesgoso? (High o Critical)
    pub fn is_dangerous(&self) -> bool {
        matches!(self.level, RiskAssessment::High | RiskAssessment::Critical)
    }
}

/// Motor de evaluación de riesgo.
#[derive(Debug, Clone, Default)]
pub struct RiskEngine {
    /// Path raíz del workspace. Paths fuera de aquí cuentan como
    /// `OutsideWorkspace`.
    pub workspace_root: Option<String>,

    /// Si `true`, `sudo` y equivalentes cuentan como
    /// `PrivilegeEscalation`.
    pub detect_privilege_escalation: bool,
}

impl RiskEngine {
    /// Crea un motor con heurísticas por defecto.
    pub fn new() -> Self {
        Self {
            workspace_root: None,
            detect_privilege_escalation: true,
        }
    }

    /// Configura el root del workspace.
    pub fn with_workspace(mut self, root: impl Into<String>) -> Self {
        self.workspace_root = Some(root.into());
        self
    }

    /// Evalúa el riesgo de una invocación.
    pub fn evaluate(&self, invocation: &Invocation) -> RiskReport {
        let mut factors: Vec<RiskFactor> = Vec::new();

        if let Some(action) = &invocation.action {
            if is_destructive_action(action) {
                factors.push(RiskFactor::DestructiveAction);
            }
        }

        if let Some(path) = &invocation.path {
            if !self.is_within_workspace(path) {
                factors.push(RiskFactor::OutsideWorkspace);
            }
        }

        if let Some(program) = &invocation.program {
            if is_irreversible_command(program, &invocation.args) {
                factors.push(RiskFactor::Irreversible);
            }
            if is_network_command(program) {
                factors.push(RiskFactor::Network);
            }
            if self.detect_privilege_escalation && is_privilege_escalation(program) {
                factors.push(RiskFactor::PrivilegeEscalation);
            }
        }

        if invocation.args.len() > 5 {
            factors.push(RiskFactor::BatchOperation);
        }

        RiskReport::from_factors(factors)
    }

    /// ¿El path está dentro del workspace?
    fn is_within_workspace(&self, path: &str) -> bool {
        match &self.workspace_root {
            None => true,
            Some(root) => path.starts_with(root) || !path.starts_with('/'),
        }
    }
}

/// ¿Esta acción es destructiva?
fn is_destructive_action(action: &str) -> bool {
    matches!(
        action.to_lowercase().as_str(),
        "delete" | "remove" | "rm" | "reset" | "clean" | "force_push" | "hard_reset"
    )
}

/// ¿Este comando es irreversible?
fn is_irreversible_command(program: &str, args: &[String]) -> bool {
    let p = program.to_lowercase();
    let has_flag = |flag: &str| args.iter().any(|a| a == flag);

    match p.as_str() {
        "rm" => true,
        "git" => {
            args.first().map(|s| s.as_str()) == Some("push")
                || (args.first().map(|s| s.as_str()) == Some("reset") && has_flag("--hard"))
                || (args.first().map(|s| s.as_str()) == Some("clean") && has_flag("-fd"))
        }
        "dd" | "mkfs" | "shred" => true,
        _ => false,
    }
}

/// ¿Este comando usa la red?
fn is_network_command(program: &str) -> bool {
    matches!(
        program.to_lowercase().as_str(),
        "curl" | "wget" | "nc" | "ssh" | "scp" | "rsync" | "ping"
    )
}

/// ¿Este comando escala privilegios?
fn is_privilege_escalation(program: &str) -> bool {
    matches!(program.to_lowercase().as_str(), "sudo" | "su" | "doas")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assessment_display_names() {
        assert_eq!(RiskAssessment::Low.display_name(), "LOW");
        assert_eq!(RiskAssessment::Critical.display_name(), "CRITICAL");
    }

    #[test]
    fn assessment_ordering() {
        assert!(RiskAssessment::Low < RiskAssessment::Medium);
        assert!(RiskAssessment::Medium < RiskAssessment::High);
        assert!(RiskAssessment::High < RiskAssessment::Critical);
    }

    #[test]
    fn assessment_combine_keeps_max() {
        assert_eq!(
            RiskAssessment::Low.combine(RiskAssessment::High),
            RiskAssessment::High
        );
        assert_eq!(
            RiskAssessment::Critical.combine(RiskAssessment::Low),
            RiskAssessment::Critical
        );
    }

    #[test]
    fn assessment_default_is_low() {
        assert_eq!(RiskAssessment::default(), RiskAssessment::Low);
    }

    #[test]
    fn empty_factors_is_low() {
        let r = RiskReport::from_factors(vec![]);
        assert_eq!(r.level, RiskAssessment::Low);
        assert!(r.factors.is_empty());
    }

    #[test]
    fn single_factor() {
        let r = RiskReport::from_factors(vec![RiskFactor::Network]);
        assert_eq!(r.level, RiskAssessment::Medium);
        assert_eq!(r.factors.len(), 1);
    }

    #[test]
    fn factors_deduplicated() {
        let r = RiskReport::from_factors(vec![
            RiskFactor::Network,
            RiskFactor::Network,
            RiskFactor::Network,
        ]);
        assert_eq!(r.factors.len(), 1);
    }

    #[test]
    fn max_level_wins() {
        let r = RiskReport::from_factors(vec![RiskFactor::Network, RiskFactor::DestructiveAction]);
        assert_eq!(r.level, RiskAssessment::High);
    }

    #[test]
    fn privilege_escalation_is_critical() {
        let r = RiskReport::from_factors(vec![RiskFactor::PrivilegeEscalation]);
        assert_eq!(r.level, RiskAssessment::Critical);
        assert!(r.is_dangerous());
    }

    #[test]
    fn engine_default_low_for_safe_invocation() {
        let engine = RiskEngine::new();
        let inv = Invocation::filesystem("read", "src/main.rs");
        let r = engine.evaluate(&inv);
        assert_eq!(r.level, RiskAssessment::Low);
    }

    #[test]
    fn engine_detects_delete() {
        let engine = RiskEngine::new();
        let inv = Invocation::filesystem("delete", "file.txt");
        let r = engine.evaluate(&inv);
        assert_eq!(r.level, RiskAssessment::High);
        assert!(r.factors.contains(&RiskFactor::DestructiveAction));
    }

    #[test]
    fn engine_detects_write_is_not_destructive() {
        let engine = RiskEngine::new();
        let inv = Invocation::filesystem("write", "file.txt");
        let r = engine.evaluate(&inv);
        assert_eq!(r.level, RiskAssessment::Low);
    }

    #[test]
    fn engine_detects_network_command() {
        let engine = RiskEngine::new();
        let inv = Invocation::shell("curl", vec!["https://example.com".into()]);
        let r = engine.evaluate(&inv);
        assert!(r.factors.contains(&RiskFactor::Network));
    }

    #[test]
    fn engine_detects_git_push_as_irreversible() {
        let engine = RiskEngine::new();
        let inv = Invocation::shell("git", vec!["push".into()]);
        let r = engine.evaluate(&inv);
        assert!(r.factors.contains(&RiskFactor::Irreversible));
        assert_eq!(r.level, RiskAssessment::High);
    }

    #[test]
    fn engine_detects_git_reset_hard() {
        let engine = RiskEngine::new();
        let inv = Invocation::shell("git", vec!["reset".into(), "--hard".into()]);
        let r = engine.evaluate(&inv);
        assert!(r.factors.contains(&RiskFactor::Irreversible));
    }

    #[test]
    fn engine_detects_privilege_escalation() {
        let engine = RiskEngine::new();
        let inv = Invocation::shell("sudo", vec!["apt".into(), "install".into()]);
        let r = engine.evaluate(&inv);
        assert!(r.factors.contains(&RiskFactor::PrivilegeEscalation));
        assert_eq!(r.level, RiskAssessment::Critical);
    }

    #[test]
    fn engine_does_not_flag_sudo_if_disabled() {
        let engine = RiskEngine {
            workspace_root: None,
            detect_privilege_escalation: false,
        };
        let inv = Invocation::shell("sudo", vec!["apt".into(), "install".into()]);
        let r = engine.evaluate(&inv);
        assert!(!r.factors.contains(&RiskFactor::PrivilegeEscalation));
    }

    #[test]
    fn engine_detects_outside_workspace() {
        let engine = RiskEngine::new().with_workspace("/workspace");
        let inv = Invocation::filesystem("read", "/workspace/file.txt");
        let r = engine.evaluate(&inv);
        assert!(!r.factors.contains(&RiskFactor::OutsideWorkspace));

        let inv = Invocation::filesystem("read", "/other/file.txt");
        let r = engine.evaluate(&inv);
        assert!(r.factors.contains(&RiskFactor::OutsideWorkspace));
    }

    #[test]
    fn engine_detects_batch_operations() {
        let engine = RiskEngine::new();
        let inv = Invocation::shell(
            "echo",
            vec![
                "1".into(),
                "2".into(),
                "3".into(),
                "4".into(),
                "5".into(),
                "6".into(),
            ],
        );
        let r = engine.evaluate(&inv);
        assert!(r.factors.contains(&RiskFactor::BatchOperation));
    }

    #[test]
    fn engine_combines_multiple_factors() {
        let engine = RiskEngine::new();
        let inv = Invocation::shell(
            "sudo",
            vec![
                "rm".into(),
                "1".into(),
                "2".into(),
                "3".into(),
                "4".into(),
                "5".into(),
            ],
        );
        let r = engine.evaluate(&inv);
        assert_eq!(r.level, RiskAssessment::Critical);
        assert!(r.factors.len() >= 2);
    }

    #[test]
    fn report_roundtrips() {
        let r = RiskReport::from_factors(vec![RiskFactor::Network, RiskFactor::DestructiveAction]);
        let json = serde_json::to_string(&r).unwrap();
        let back: RiskReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.level, r.level);
        assert_eq!(back.factors, r.factors);
    }
}
