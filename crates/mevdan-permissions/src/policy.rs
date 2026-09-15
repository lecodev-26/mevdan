//! Políticas y motor de permisos.
//!
//! Una `Policy` es una **lista ordenada** de reglas más un default.
//! El `PermissionEngine` evalúa un `Invocation` contra las reglas en
//! orden: la primera que matchea gana. Si ninguna matchea, se aplica
//! el **default** (por defecto `Deny`).
//!
//! ## Orden importa
//!
//! Las reglas se evalúan en el orden en que se añaden. Esto significa
//! que el usuario debe declarar las reglas **de más específica a menos
//! específica**. Ejemplo:
//!
//! ```text
//! 1. filesystem, action=delete     → DENY   ← muy específica
//! 2. filesystem, action=write      → ASK    ← específica
//! 3. filesystem, path=src/**       → ALLOW  ← general
//! 4. filesystem, any               → ASK    ← muy general
//! 5. *                             → DENY   ← catch-all
//! ```

use crate::{
    decision::Decision,
    invocation::Invocation,
    permission::{Permission, Scope},
    rule::Rule,
};
use serde::{Deserialize, Serialize};

/// Política: lista ordenada de reglas + default.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub rules: Vec<Rule>,
    pub default_permission: Permission,
}

impl Policy {
    /// Política vacía con default `Deny`.
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            default_permission: Permission::Deny,
        }
    }

    /// Política permisiva: default `Allow`.
    pub fn permissive() -> Self {
        Self {
            rules: Vec::new(),
            default_permission: Permission::Allow,
        }
    }

    /// Política restrictiva: default `Deny`.
    pub fn strict() -> Self {
        Self::new()
    }

    /// Añade una regla al final.
    pub fn add_rule(mut self, rule: Rule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Sobrescribe el default.
    pub fn with_default(mut self, permission: Permission) -> Self {
        self.default_permission = permission;
        self
    }

    /// Número de reglas.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// ¿Está vacía?
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

impl Default for Policy {
    fn default() -> Self {
        Self::new()
    }
}

/// Motor de permisos.
#[derive(Debug, Clone)]
pub struct PermissionEngine {
    policy: Policy,
}

impl PermissionEngine {
    pub fn new(policy: Policy) -> Self {
        Self { policy }
    }

    pub fn strict() -> Self {
        Self::new(Policy::strict())
    }

    pub fn permissive() -> Self {
        Self::new(Policy::permissive())
    }

    pub fn policy(&self) -> &Policy {
        &self.policy
    }

    /// Evalúa una invocación y devuelve la decisión.
    pub fn evaluate(&self, invocation: &Invocation) -> Decision {
        for (index, rule) in self.policy.rules.iter().enumerate() {
            if rule_matches(rule, invocation) {
                return Decision::from_rule(rule, index);
            }
        }
        Decision::from_default(self.policy.default_permission)
    }
}

impl Default for PermissionEngine {
    fn default() -> Self {
        Self::strict()
    }
}

fn rule_matches(rule: &Rule, invocation: &Invocation) -> bool {
    if !rule.applies_to_tool(&invocation.tool) {
        return false;
    }
    scope_matches(&rule.scope, invocation)
}

fn scope_matches(scope: &Scope, invocation: &Invocation) -> bool {
    match scope {
        Scope::Any => true,

        Scope::PathGlob { pattern } => match &invocation.path {
            Some(path) => glob_match(pattern, path),
            None => false,
        },

        Scope::PathPrefix { prefix } => match &invocation.path {
            Some(path) => path.starts_with(prefix),
            None => false,
        },

        Scope::PathExact { path: exact } => match &invocation.path {
            Some(path) => path == exact,
            None => false,
        },

        Scope::ShellProgram { program } => match &invocation.program {
            Some(p) => p == program,
            None => false,
        },

        Scope::Action { name } => match &invocation.action {
            Some(a) => a == name,
            None => false,
        },
    }
}

/// Glob match segmentado por `/`.
///
/// Reglas:
/// - `*` matchea cualquier secuencia **dentro de un segmento** (no
///   cruza `/`).
/// - `**` como segmento completo matchea **cero o más segmentos**
///   (cruza `/`).
/// - Caracteres literales matchean tal cual.
fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern_segs: Vec<&str> = if pattern.is_empty() {
        vec![]
    } else {
        pattern.split('/').collect()
    };
    let text_segs: Vec<&str> = if text.is_empty() {
        vec![]
    } else {
        text.split('/').collect()
    };

    match_segments(&pattern_segs, &text_segs)
}

fn match_segments(pattern: &[&str], text: &[&str]) -> bool {
    if pattern.is_empty() {
        return text.is_empty();
    }

    let head = pattern[0];

    if head == "**" {
        for i in 0..=text.len() {
            if match_segments(&pattern[1..], &text[i..]) {
                return true;
            }
        }
        return false;
    }

    if text.is_empty() {
        return false;
    }

    if !segment_matches(head, text[0]) {
        return false;
    }

    match_segments(&pattern[1..], &text[1..])
}

fn segment_matches(pattern: &str, text: &str) -> bool {
    segment_match_bytes(pattern.as_bytes(), text.as_bytes())
}

fn segment_match_bytes(pattern: &[u8], text: &[u8]) -> bool {
    let mut p = 0usize;
    let mut t = 0usize;
    let mut star: Option<usize> = None;
    let mut star_t: usize = 0;

    while t < text.len() {
        if p < pattern.len() && pattern[p] == b'*' {
            star = Some(p);
            star_t = t;
            p += 1;
        } else if p < pattern.len() && pattern[p] == text[t] {
            p += 1;
            t += 1;
        } else if let Some(sp) = star {
            star_t += 1;
            t = star_t;
            p = sp + 1;
        } else {
            return false;
        }
    }

    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }

    p == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permission::{Permission, Scope};
    use crate::rule::Rule;

    #[test]
    fn new_policy_is_empty_with_deny_default() {
        let p = Policy::new();
        assert!(p.is_empty());
        assert_eq!(p.default_permission, Permission::Deny);
    }

    #[test]
    fn permissive_policy_has_allow_default() {
        let p = Policy::permissive();
        assert_eq!(p.default_permission, Permission::Allow);
    }

    #[test]
    fn add_rule_increases_count() {
        let p = Policy::new()
            .add_rule(Rule::new("filesystem", Scope::Any, Permission::Allow).unwrap())
            .add_rule(Rule::new("shell", Scope::Any, Permission::Ask).unwrap());
        assert_eq!(p.rule_count(), 2);
    }

    #[test]
    fn with_default_overrides() {
        let p = Policy::new().with_default(Permission::Ask);
        assert_eq!(p.default_permission, Permission::Ask);
    }

    #[test]
    fn strict_engine_denies_everything_by_default() {
        let engine = PermissionEngine::strict();
        let decision = engine.evaluate(&Invocation::tool("filesystem"));
        assert!(decision.is_denied());
    }

    #[test]
    fn permissive_engine_allows_everything_by_default() {
        let engine = PermissionEngine::permissive();
        let decision = engine.evaluate(&Invocation::tool("filesystem"));
        assert!(decision.is_allowed());
    }

    #[test]
    fn engine_with_allow_rule_matches() {
        let policy = Policy::new().add_rule(
            Rule::new("filesystem", Scope::Any, Permission::Allow)
                .unwrap()
                .with_reason("allow filesystem"),
        );
        let engine = PermissionEngine::new(policy);
        let decision = engine.evaluate(&Invocation::tool("filesystem"));
        assert!(decision.is_allowed());
        assert_eq!(decision.reason, "allow filesystem");
    }

    #[test]
    fn engine_first_matching_rule_wins() {
        let policy = Policy::new()
            .add_rule(Rule::new("filesystem", Scope::action("delete"), Permission::Deny).unwrap())
            .add_rule(Rule::new("filesystem", Scope::Any, Permission::Allow).unwrap());

        let engine = PermissionEngine::new(policy);

        let inv = Invocation::filesystem("delete", "file.txt");
        assert!(engine.evaluate(&inv).is_denied());

        let inv = Invocation::filesystem("read", "file.txt");
        assert!(engine.evaluate(&inv).is_allowed());
    }

    #[test]
    fn path_prefix_matches() {
        let policy = Policy::new()
            .add_rule(Rule::new("filesystem", Scope::prefix("src/"), Permission::Allow).unwrap());
        let engine = PermissionEngine::new(policy);

        assert!(engine
            .evaluate(&Invocation::filesystem("read", "src/main.rs"))
            .is_allowed());
        assert!(engine
            .evaluate(&Invocation::filesystem("read", "docs/readme.md"))
            .is_denied());
    }

    #[test]
    fn path_exact_matches() {
        let policy = Policy::new().add_rule(
            Rule::new("filesystem", Scope::exact("README.md"), Permission::Allow).unwrap(),
        );
        let engine = PermissionEngine::new(policy);

        assert!(engine
            .evaluate(&Invocation::filesystem("read", "README.md"))
            .is_allowed());
        assert!(engine
            .evaluate(&Invocation::filesystem("read", "README.txt"))
            .is_denied());
    }

    #[test]
    fn action_scope_matches() {
        let policy = Policy::new()
            .add_rule(Rule::new("filesystem", Scope::action("write"), Permission::Ask).unwrap());
        let engine = PermissionEngine::new(policy);

        assert!(engine
            .evaluate(&Invocation::filesystem("write", "a.txt"))
            .needs_user_input());
        assert!(engine
            .evaluate(&Invocation::filesystem("read", "a.txt"))
            .is_denied());
    }

    #[test]
    fn shell_program_scope_matches() {
        let policy = Policy::new().add_rule(
            Rule::new("shell", Scope::shell_program("cargo"), Permission::Allow).unwrap(),
        );
        let engine = PermissionEngine::new(policy);

        assert!(engine
            .evaluate(&Invocation::shell("cargo", vec!["build".into()]))
            .is_allowed());
        assert!(engine
            .evaluate(&Invocation::shell("npm", vec!["install".into()]))
            .is_denied());
    }

    #[test]
    fn wildcard_tool_matches_any() {
        let policy = Policy::new().add_rule(Rule::new("*", Scope::Any, Permission::Deny).unwrap());
        let engine = PermissionEngine::new(policy);

        assert!(engine.evaluate(&Invocation::tool("filesystem")).is_denied());
        assert!(engine.evaluate(&Invocation::tool("shell")).is_denied());
        assert!(engine.evaluate(&Invocation::tool("whatever")).is_denied());
    }

    #[test]
    fn glob_simple_literal() {
        assert!(glob_match("test.txt", "test.txt"));
        assert!(!glob_match("test.txt", "other.txt"));
    }

    #[test]
    fn glob_single_star() {
        assert!(glob_match("*.md", "README.md"));
        assert!(glob_match("*.md", "doc.md"));
        assert!(!glob_match("*.md", "src/README.md"));
        assert!(!glob_match("*.md", "README.txt"));
    }

    #[test]
    fn glob_double_star() {
        assert!(glob_match("**/*.rs", "main.rs"));
        assert!(glob_match("**/*.rs", "src/main.rs"));
        assert!(glob_match("**/*.rs", "a/b/c/main.rs"));
        assert!(glob_match("src/**", "src/main.rs"));
        assert!(glob_match("src/**", "src/a/b/c.rs"));
    }

    #[test]
    fn glob_prefix_match() {
        assert!(glob_match("src/*", "src/main.rs"));
        assert!(!glob_match("src/*", "src/nested/main.rs"));
        assert!(glob_match("src/**", "src/nested/main.rs"));
    }

    #[test]
    fn glob_exact_suffix() {
        assert!(glob_match("**/*.rs", "main.rs"));
        assert!(glob_match("**/*.rs", "src/main.rs"));
    }

    #[test]
    fn glob_mixed_patterns() {
        assert!(glob_match("a/*/c", "a/b/c"));
        assert!(!glob_match("a/*/c", "a/b/x/c"));
        assert!(glob_match("a/**/c", "a/b/x/c"));
        assert!(glob_match("a/**/c", "a/c"));
    }

    /// Escenario realista con reglas **ordenadas de más específica a
    /// menos específica**.
    ///
    /// Este es el patrón que el usuario debe seguir.
    #[test]
    fn realistic_policy() {
        let policy = Policy::new()
            // 1. Delete: nunca permitido (regla más específica).
            .add_rule(
                Rule::new("filesystem", Scope::action("delete"), Permission::Deny)
                    .unwrap()
                    .with_reason("deletion is never automatic"),
            )
            // 2. Write: preguntar siempre (antes que src/**).
            .add_rule(
                Rule::new("filesystem", Scope::action("write"), Permission::Ask)
                    .unwrap()
                    .with_reason("writes need confirmation"),
            )
            // 3. Cualquier otra cosa en src/: permitir.
            .add_rule(
                Rule::new("filesystem", Scope::glob("src/**"), Permission::Allow)
                    .unwrap()
                    .with_reason("read src freely"),
            )
            // 4. Cualquier otra cosa de filesystem: preguntar.
            .add_rule(Rule::new("filesystem", Scope::Any, Permission::Ask).unwrap())
            // 5. Shell: solo cargo permitido.
            .add_rule(Rule::new("shell", Scope::shell_program("cargo"), Permission::Allow).unwrap())
            // 6. Cualquier otro shell: deny.
            .add_rule(Rule::new("shell", Scope::Any, Permission::Deny).unwrap());

        let engine = PermissionEngine::new(policy);

        // Delete → Deny (regla 1).
        assert!(engine
            .evaluate(&Invocation::filesystem("delete", "src/main.rs"))
            .is_denied());

        // Write src → Ask (regla 2, antes que regla 3).
        assert!(engine
            .evaluate(&Invocation::filesystem("write", "src/new.rs"))
            .needs_user_input());

        // Read src → Allow (regla 3).
        assert!(engine
            .evaluate(&Invocation::filesystem("read", "src/main.rs"))
            .is_allowed());

        // Read docs → Ask (regla 4).
        assert!(engine
            .evaluate(&Invocation::filesystem("read", "docs/x.md"))
            .needs_user_input());

        // Cargo → Allow (regla 5).
        assert!(engine
            .evaluate(&Invocation::shell("cargo", vec!["build".into()]))
            .is_allowed());

        // npm → Deny (regla 6).
        assert!(engine
            .evaluate(&Invocation::shell("npm", vec!["install".into()]))
            .is_denied());

        // Tool desconocido → default Deny.
        assert!(engine.evaluate(&Invocation::tool("unknown")).is_denied());
    }
}
