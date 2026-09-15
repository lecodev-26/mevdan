//! Skill isolation: validación de permisos y compatibilidad.
//!
//! Antes de cargar o instalar una skill, comprobamos que:
//!
//! 1. Todas las tools que requiere existen y están permitidas.
//! 2. Todos los permisos que solicita están cubiertos por la política.
//!
//! ## Modelo
//!
//! El evaluador recibe listas de strings (tools y permisos permitidos)
//! en vez de depender de `mevdan-tools` o `mevdan-permissions`. Esto
//! mantiene `mevdan-skills` con dependencias mínimas.

use crate::skill::Skill;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Veredicto del análisis de aislamiento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationVerdict {
    /// Todo OK: la skill puede cargarse.
    Compatible,
    /// Faltan tools requeridas.
    MissingTools,
    /// Faltan permisos requeridos.
    MissingPermissions,
    /// Faltan tools Y permisos.
    MissingToolsAndPermissions,
    /// Problema genérico (skill sin info, etc.).
    Incompatible,
}

impl IsolationVerdict {
    pub fn display_name(&self) -> &'static str {
        match self {
            IsolationVerdict::Compatible => "compatible",
            IsolationVerdict::MissingTools => "missing_tools",
            IsolationVerdict::MissingPermissions => "missing_permissions",
            IsolationVerdict::MissingToolsAndPermissions => "missing_tools_and_permissions",
            IsolationVerdict::Incompatible => "incompatible",
        }
    }

    /// ¿Es compatible?
    pub fn is_compatible(&self) -> bool {
        matches!(self, IsolationVerdict::Compatible)
    }
}

/// Política de aislamiento: qué está permitido.
#[derive(Debug, Clone, Default)]
pub struct IsolationPolicy {
    /// Tools permitidas. Si está vacío, no se permite ninguna tool.
    pub allowed_tools: BTreeSet<String>,

    /// Permisos concedidos. Si está vacío, no se concede ningún permiso.
    pub allowed_permissions: BTreeSet<String>,
}

impl IsolationPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    /// Permite una tool.
    pub fn allow_tool(mut self, tool: impl Into<String>) -> Self {
        self.allowed_tools.insert(tool.into());
        self
    }

    /// Permite varias tools.
    pub fn allow_tools<I, S>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for t in tools {
            self.allowed_tools.insert(t.into());
        }
        self
    }

    /// Concede un permiso.
    pub fn allow_permission(mut self, perm: impl Into<String>) -> Self {
        self.allowed_permissions.insert(perm.into());
        self
    }

    /// Concede varios permisos.
    pub fn allow_permissions<I, S>(mut self, perms: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for p in perms {
            self.allowed_permissions.insert(p.into());
        }
        self
    }

    /// Política vacía (nada permitido).
    pub fn strict() -> Self {
        Self::default()
    }

    /// Política permisiva para tests.
    pub fn permissive() -> Self {
        let mut p = Self::new();
        p.allowed_tools.insert("*".into());
        p.allowed_permissions.insert("*".into());
        p
    }

    /// ¿Está la tool permitida?
    pub fn tool_allowed(&self, tool: &str) -> bool {
        self.allowed_tools.contains(tool) || self.allowed_tools.contains("*")
    }

    /// ¿Está el permiso permitido?
    pub fn permission_allowed(&self, perm: &str) -> bool {
        self.allowed_permissions.contains(perm) || self.allowed_permissions.contains("*")
    }
}

/// Reporte de aislamiento.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolationReport {
    /// Skill analizada (name@version).
    pub skill_id: String,
    /// Veredicto.
    pub verdict: IsolationVerdict,
    /// Tools que faltan.
    pub missing_tools: Vec<String>,
    /// Permisos que faltan.
    pub missing_permissions: Vec<String>,
    /// Descripción legible.
    pub reason: String,
}

impl IsolationReport {
    /// ¿Es compatible?
    pub fn is_compatible(&self) -> bool {
        self.verdict.is_compatible()
    }

    /// Resumen textual.
    pub fn summary(&self) -> String {
        if self.is_compatible() {
            format!("{}: compatible", self.skill_id)
        } else {
            format!(
                "{}: {} (missing tools: [{}], missing permissions: [{}])",
                self.skill_id,
                self.verdict.display_name(),
                self.missing_tools.join(", "),
                self.missing_permissions.join(", "),
            )
        }
    }
}

/// Evaluador de aislamiento.
#[derive(Debug, Clone)]
pub struct SkillIsolation {
    policy: IsolationPolicy,
}

impl SkillIsolation {
    /// Crea un evaluador con una política.
    pub fn new(policy: IsolationPolicy) -> Self {
        Self { policy }
    }

    /// Crea un evaluador con política estricta (nada permitido).
    pub fn strict() -> Self {
        Self::new(IsolationPolicy::strict())
    }

    /// Crea un evaluador con política permisiva.
    pub fn permissive() -> Self {
        Self::new(IsolationPolicy::permissive())
    }

    /// Acceso a la política.
    pub fn policy(&self) -> &IsolationPolicy {
        &self.policy
    }

    /// Analiza si una skill es compatible.
    pub fn analyze(&self, skill: &Skill) -> IsolationReport {
        let missing_tools: Vec<String> = skill
            .required_tools()
            .iter()
            .filter(|t| !self.policy.tool_allowed(t))
            .cloned()
            .collect();

        let missing_permissions: Vec<String> = skill
            .required_permissions()
            .iter()
            .filter(|p| !self.policy.permission_allowed(p))
            .cloned()
            .collect();

        let verdict = match (missing_tools.is_empty(), missing_permissions.is_empty()) {
            (true, true) => IsolationVerdict::Compatible,
            (false, true) => IsolationVerdict::MissingTools,
            (true, false) => IsolationVerdict::MissingPermissions,
            (false, false) => IsolationVerdict::MissingToolsAndPermissions,
        };

        let reason = if verdict.is_compatible() {
            "all requirements satisfied".to_string()
        } else {
            let mut parts = Vec::new();
            if !missing_tools.is_empty() {
                parts.push(format!("missing tools: {}", missing_tools.join(", ")));
            }
            if !missing_permissions.is_empty() {
                parts.push(format!(
                    "missing permissions: {}",
                    missing_permissions.join(", ")
                ));
            }
            parts.join("; ")
        };

        IsolationReport {
            skill_id: skill.id.full(),
            verdict,
            missing_tools,
            missing_permissions,
            reason,
        }
    }

    /// Analiza varias skills.
    pub fn analyze_all(&self, skills: &[Skill]) -> Vec<IsolationReport> {
        skills.iter().map(|s| self.analyze(s)).collect()
    }

    /// Filtra las skills compatibles.
    pub fn filter_compatible(&self, skills: Vec<Skill>) -> Vec<Skill> {
        skills
            .into_iter()
            .filter(|s| self.analyze(s).is_compatible())
            .collect()
    }
}

impl Default for SkillIsolation {
    fn default() -> Self {
        Self::strict()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{manifest::SkillManifest, skill::Skill};
    use std::path::PathBuf;

    fn make_skill(name: &str, tools: &[&str], perms: &[&str]) -> Skill {
        let tools_str = tools
            .iter()
            .map(|t| format!("\"{}\"", t))
            .collect::<Vec<_>>()
            .join(", ");
        let perms_str = perms
            .iter()
            .map(|p| format!("\"{}\"", p))
            .collect::<Vec<_>>()
            .join(", ");
        let toml = format!(
            r#"
[skill]
name = "{}"
version = "0.1.0"
description = "test"

[skill.permissions]
required_tools = [{}]
required_permissions = [{}]
"#,
            name, tools_str, perms_str
        );
        let m = SkillManifest::from_toml_str(&toml).unwrap();
        Skill::new(m, PathBuf::from("/tmp")).unwrap()
    }

    // ──────────────────────────────────────────────
    // Verdict
    // ──────────────────────────────────────────────

    #[test]
    fn verdict_display_names() {
        assert_eq!(IsolationVerdict::Compatible.display_name(), "compatible");
        assert_eq!(
            IsolationVerdict::MissingToolsAndPermissions.display_name(),
            "missing_tools_and_permissions"
        );
    }

    #[test]
    fn verdict_is_compatible() {
        assert!(IsolationVerdict::Compatible.is_compatible());
        assert!(!IsolationVerdict::MissingTools.is_compatible());
    }

    #[test]
    fn verdict_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&IsolationVerdict::Compatible).unwrap(),
            "\"compatible\""
        );
        assert_eq!(
            serde_json::to_string(&IsolationVerdict::MissingTools).unwrap(),
            "\"missing_tools\""
        );
    }

    // ──────────────────────────────────────────────
    // Policy
    // ──────────────────────────────────────────────

    #[test]
    fn policy_new_is_empty() {
        let p = IsolationPolicy::new();
        assert!(p.allowed_tools.is_empty());
        assert!(p.allowed_permissions.is_empty());
    }

    #[test]
    fn policy_allow_tool_works() {
        let p = IsolationPolicy::new().allow_tool("filesystem");
        assert!(p.tool_allowed("filesystem"));
        assert!(!p.tool_allowed("shell"));
    }

    #[test]
    fn policy_allow_tools_works() {
        let p = IsolationPolicy::new().allow_tools(["filesystem", "shell"]);
        assert!(p.tool_allowed("filesystem"));
        assert!(p.tool_allowed("shell"));
    }

    #[test]
    fn policy_allow_permission_works() {
        let p = IsolationPolicy::new().allow_permission("filesystem.read");
        assert!(p.permission_allowed("filesystem.read"));
        assert!(!p.permission_allowed("filesystem.write"));
    }

    #[test]
    fn policy_wildcard_tools() {
        let p = IsolationPolicy::permissive();
        assert!(p.tool_allowed("anything"));
        assert!(p.permission_allowed("anything"));
    }

    #[test]
    fn policy_strict_allows_nothing() {
        let p = IsolationPolicy::strict();
        assert!(!p.tool_allowed("filesystem"));
        assert!(!p.permission_allowed("filesystem.read"));
    }

    // ──────────────────────────────────────────────
    // Isolation
    // ──────────────────────────────────────────────

    #[test]
    fn skill_with_no_requirements_is_compatible() {
        let skill = make_skill("simple", &[], &[]);
        let iso = SkillIsolation::strict();
        let report = iso.analyze(&skill);
        assert!(report.is_compatible());
        assert_eq!(report.verdict, IsolationVerdict::Compatible);
    }

    #[test]
    fn skill_with_allowed_tools_is_compatible() {
        let skill = make_skill("coding", &["filesystem"], &[]);
        let iso = SkillIsolation::new(IsolationPolicy::new().allow_tool("filesystem"));
        let report = iso.analyze(&skill);
        assert!(report.is_compatible());
    }

    #[test]
    fn skill_with_missing_tool() {
        let skill = make_skill("coding", &["filesystem", "shell"], &[]);
        let iso = SkillIsolation::new(IsolationPolicy::new().allow_tool("filesystem"));
        let report = iso.analyze(&skill);
        assert!(!report.is_compatible());
        assert_eq!(report.verdict, IsolationVerdict::MissingTools);
        assert_eq!(report.missing_tools, vec!["shell"]);
    }

    #[test]
    fn skill_with_missing_permission() {
        let skill = make_skill("coding", &[], &["filesystem.read"]);
        let iso = SkillIsolation::strict();
        let report = iso.analyze(&skill);
        assert_eq!(report.verdict, IsolationVerdict::MissingPermissions);
        assert_eq!(report.missing_permissions, vec!["filesystem.read"]);
    }

    #[test]
    fn skill_with_missing_both() {
        let skill = make_skill("coding", &["shell"], &["shell.execute"]);
        let iso = SkillIsolation::strict();
        let report = iso.analyze(&skill);
        assert_eq!(report.verdict, IsolationVerdict::MissingToolsAndPermissions);
        assert_eq!(report.missing_tools, vec!["shell"]);
        assert_eq!(report.missing_permissions, vec!["shell.execute"]);
    }

    #[test]
    fn permissive_iso_allows_everything() {
        let skill = make_skill("anything", &["x", "y"], &["a", "b"]);
        let iso = SkillIsolation::permissive();
        let report = iso.analyze(&skill);
        assert!(report.is_compatible());
    }

    #[test]
    fn report_summary_compatible() {
        let skill = make_skill("coding", &[], &[]);
        let iso = SkillIsolation::strict();
        let report = iso.analyze(&skill);
        assert!(report.summary().contains("compatible"));
    }

    #[test]
    fn report_summary_incompatible() {
        let skill = make_skill("coding", &["shell"], &[]);
        let iso = SkillIsolation::strict();
        let report = iso.analyze(&skill);
        assert!(report.summary().contains("missing tools"));
        assert!(report.summary().contains("shell"));
    }

    #[test]
    fn analyze_all_works() {
        let skills = vec![make_skill("a", &[], &[]), make_skill("b", &["x"], &[])];
        let iso = SkillIsolation::strict();
        let reports = iso.analyze_all(&skills);
        assert_eq!(reports.len(), 2);
        assert!(reports[0].is_compatible());
        assert!(!reports[1].is_compatible());
    }

    #[test]
    fn filter_compatible_works() {
        let skills = vec![
            make_skill("a", &[], &[]),
            make_skill("b", &["x"], &[]),
            make_skill("c", &[], &[]),
        ];
        let iso = SkillIsolation::strict();
        let filtered = iso.filter_compatible(skills);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name(), "a");
        assert_eq!(filtered[1].name(), "c");
    }

    #[test]
    fn report_roundtrips() {
        let skill = make_skill("coding", &["shell"], &["shell.execute"]);
        let iso = SkillIsolation::strict();
        let report = iso.analyze(&skill);
        let json = serde_json::to_string(&report).unwrap();
        let back: IsolationReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.verdict, report.verdict);
        assert_eq!(back.missing_tools, report.missing_tools);
    }

    #[test]
    fn realistic_skill_isolation() {
        let skill = make_skill(
            "coding",
            &["filesystem", "shell", "git"],
            &["filesystem.read", "filesystem.write", "shell.execute"],
        );

        // Política restrictiva: solo filesystem.read.
        let strict = SkillIsolation::new(
            IsolationPolicy::new()
                .allow_tool("filesystem")
                .allow_permission("filesystem.read"),
        );
        let report = strict.analyze(&skill);
        assert!(!report.is_compatible());
        assert_eq!(report.missing_tools.len(), 2); // shell, git
        assert_eq!(report.missing_permissions.len(), 2); // write, execute

        // Política completa.
        let full = SkillIsolation::new(
            IsolationPolicy::new()
                .allow_tools(["filesystem", "shell", "git"])
                .allow_permissions(["filesystem.read", "filesystem.write", "shell.execute"]),
        );
        let report = full.analyze(&skill);
        assert!(report.is_compatible());
    }
}
