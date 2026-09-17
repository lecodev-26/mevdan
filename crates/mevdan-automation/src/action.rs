//! `Action` — qué hacer cuando un trigger matchea.

use serde::{Deserialize, Serialize};

/// Una acción de automatización.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Ejecutar un comando (a través de la capa de permisos).
    RunCommand {
        program: String,
        #[serde(default)]
        args: Vec<String>,
    },
    /// Crear un checkpoint.
    CreateCheckpoint {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
    /// Registrar un handoff entre agentes.
    Handoff {
        from_role: String,
        to_role: String,
        reason: String,
    },
    /// Ejecutar un workflow por nombre.
    RunWorkflow { name: String },
    /// Notificar (no hace nada por sí sola; el runtime decide).
    Notify { message: String },
}

impl Action {
    pub fn run_command(program: impl Into<String>) -> Self {
        Action::RunCommand {
            program: program.into(),
            args: Vec::new(),
        }
    }

    pub fn run_command_with_args<I, S>(program: impl Into<String>, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Action::RunCommand {
            program: program.into(),
            args: args.into_iter().map(|s| s.into()).collect(),
        }
    }

    pub fn create_checkpoint(label: Option<String>) -> Self {
        Action::CreateCheckpoint { label }
    }

    pub fn handoff(
        from_role: impl Into<String>,
        to_role: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Action::Handoff {
            from_role: from_role.into(),
            to_role: to_role.into(),
            reason: reason.into(),
        }
    }

    pub fn run_workflow(name: impl Into<String>) -> Self {
        Action::RunWorkflow { name: name.into() }
    }

    pub fn notify(message: impl Into<String>) -> Self {
        Action::Notify {
            message: message.into(),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Action::RunCommand { .. } => "run_command",
            Action::CreateCheckpoint { .. } => "create_checkpoint",
            Action::Handoff { .. } => "handoff",
            Action::RunWorkflow { .. } => "run_workflow",
            Action::Notify { .. } => "notify",
        }
    }
}

/// Resultado de ejecutar una acción.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action: Action,
    pub success: bool,
    pub message: String,
}

impl ActionResult {
    pub fn ok(action: Action, message: impl Into<String>) -> Self {
        Self {
            action,
            success: true,
            message: message.into(),
        }
    }

    pub fn failure(action: Action, message: impl Into<String>) -> Self {
        Self {
            action,
            success: false,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_run_command() {
        let a = Action::run_command("cargo");
        assert_eq!(
            a,
            Action::RunCommand {
                program: "cargo".into(),
                args: vec![],
            }
        );
    }

    #[test]
    fn action_run_command_with_args() {
        let a = Action::run_command_with_args("cargo", ["test", "--workspace"]);
        if let Action::RunCommand { program, args } = a {
            assert_eq!(program, "cargo");
            assert_eq!(args, vec!["test", "--workspace"]);
        } else {
            panic!("expected RunCommand");
        }
    }

    #[test]
    fn action_create_checkpoint() {
        let a = Action::create_checkpoint(Some("before-refactor".into()));
        assert_eq!(
            a,
            Action::CreateCheckpoint {
                label: Some("before-refactor".into())
            }
        );
    }

    #[test]
    fn action_create_checkpoint_no_label() {
        let a = Action::create_checkpoint(None);
        assert_eq!(a, Action::CreateCheckpoint { label: None });
    }

    #[test]
    fn action_handoff() {
        let a = Action::handoff("Planner", "Coder", "cost");
        if let Action::Handoff {
            from_role,
            to_role,
            reason,
        } = a
        {
            assert_eq!(from_role, "Planner");
            assert_eq!(to_role, "Coder");
            assert_eq!(reason, "cost");
        } else {
            panic!("expected Handoff");
        }
    }

    #[test]
    fn action_run_workflow() {
        let a = Action::run_workflow("build-feature");
        assert_eq!(
            a,
            Action::RunWorkflow {
                name: "build-feature".into()
            }
        );
    }

    #[test]
    fn action_notify() {
        let a = Action::notify("hello");
        assert_eq!(
            a,
            Action::Notify {
                message: "hello".into()
            }
        );
    }

    #[test]
    fn action_display_names() {
        assert_eq!(Action::run_command("x").display_name(), "run_command");
        assert_eq!(
            Action::create_checkpoint(None).display_name(),
            "create_checkpoint"
        );
        assert_eq!(Action::handoff("a", "b", "c").display_name(), "handoff");
        assert_eq!(Action::run_workflow("x").display_name(), "run_workflow");
        assert_eq!(Action::notify("x").display_name(), "notify");
    }

    #[test]
    fn action_serializes_tagged() {
        let a = Action::run_command_with_args("cargo", ["test"]);
        let json = serde_json::to_value(&a).unwrap();
        assert_eq!(json["type"], "run_command");
        assert_eq!(json["program"], "cargo");
        assert_eq!(json["args"][0], "test");
    }

    #[test]
    fn action_roundtrips() {
        let a = Action::handoff("Planner", "Coder", "cost");
        let json = serde_json::to_string(&a).unwrap();
        let back: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(back, a);
    }

    #[test]
    fn action_result_ok() {
        let a = Action::notify("done");
        let r = ActionResult::ok(a.clone(), "success");
        assert!(r.success);
        assert_eq!(r.action, a);
    }

    #[test]
    fn action_result_failure() {
        let a = Action::notify("x");
        let r = ActionResult::failure(a.clone(), "boom");
        assert!(!r.success);
        assert_eq!(r.message, "boom");
    }

    #[test]
    fn action_result_serializes() {
        let a = Action::notify("done");
        let r = ActionResult::ok(a, "ok");
        let json = serde_json::to_string(&r).unwrap();
        let back: ActionResult = serde_json::from_str(&json).unwrap();
        assert!(back.success);
    }
}
