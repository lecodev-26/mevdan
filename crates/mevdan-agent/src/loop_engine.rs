//! Loop de razonamiento del agente.
//!
//! El loop toma una tarea y produce un `AgentOutcome` pasando por:
//! 1. `Understand` — construir el prompt inicial.
//! 2. `Act` — llamar al provider.
//! 3. `Observe` — procesar la respuesta.
//! 4. `Review` — auto-crítica (si la política lo permite).
//! 5. `Replan` — si la review falla y quedan replans.
//! 6. `Finish` — terminar.
//!
//! ## Sin tools todavía
//!
//! En esta fase el loop solo habla con el provider. El punto de
//! extensión para tools está marcado con `TODO(fase-15+)`.

use crate::{
    agent::{AgentOutcome, FinishKind},
    error::{AgentError, AgentResult},
    identity::AgentIdentity,
    session::AgentSession,
    step::{Step, StepKind, StepOutcome},
};
use mevdan_provider::{ChatRequest, ChatResponse, Message, Provider};

/// Ejecuta el loop de razonamiento completo.
///
/// `provider` es quien habla con el modelo. `identity` y `session`
/// definen el contexto. `task` es lo que el usuario pide.
pub fn run(
    provider: &dyn Provider,
    identity: &AgentIdentity,
    mut session: AgentSession,
    task: &str,
) -> AgentResult<AgentOutcome> {
    let mut steps: Vec<Step> = Vec::new();

    // ── 1. Understand ────────────────────────────────────────
    session.push_message(Message::system(&identity.system_prompt));
    session.push_message(Message::user(task));

    steps.push(Step::new(
        StepKind::Understand,
        StepOutcome::Success,
        format!("task received: {}", truncate(task, 80)),
    ));

    let mut replan_attempt: u32 = 0;
    let max_attempts = session.policy.max_replans.saturating_add(1);

    // El loop devuelve `ChatResponse` mediante `break response`.
    // Clippy está contento: no hay variable inicializada a None que
    // nunca se lea.
    let response: ChatResponse = loop {
        // Chequeos de política antes de cada intento.
        session.can_take_step()?;
        session.can_continue_in_time()?;

        // ── 2. Act: construir el request ─────────────────────
        let request = build_request(provider, &session, identity);

        // ── 3. Act: llamar al provider ───────────────────────
        let response = match provider.chat(request.clone()) {
            Ok(r) => r,
            Err(e) => {
                steps.push(Step::new(
                    StepKind::Act,
                    StepOutcome::Failure,
                    format!("provider error: {}", e),
                ));
                return Err(AgentError::Provider(e));
            }
        };

        let tokens = response.usage.map(|u| u.total()).unwrap_or(0);
        session.record_tokens(tokens);
        session.record_step();

        // Añade la respuesta al historial de la sesión.
        session.push_message(response.message.clone());

        steps.push(
            Step::act_success(format!(
                "model {} responded ({} tokens)",
                response.model, tokens
            ))
            .with_request(request)
            .with_response(response.clone(), tokens),
        );

        // ── 4. Observe ───────────────────────────────────────
        steps.push(Step::new(
            StepKind::Observe,
            StepOutcome::Success,
            format!(
                "finish_reason={:?}, content_len={}",
                response.finish_reason,
                response.message.content.len()
            ),
        ));

        // ── 5. Review (si la política lo permite) ────────────
        if !session.policy.has_review() {
            steps.push(Step::new(
                StepKind::Review,
                StepOutcome::Skipped,
                "review disabled by policy",
            ));
            break response;
        }

        match review_response(&response) {
            Ok(()) => {
                steps.push(Step::new(
                    StepKind::Review,
                    StepOutcome::Success,
                    "review passed",
                ));
                break response;
            }
            Err(reason) => {
                steps.push(Step::review_failure(reason.clone()));

                // ── 6. Replan (si la política lo permite y quedan) ─
                if !session.can_replan() || replan_attempt + 1 >= max_attempts {
                    break response;
                }

                session.record_replan();
                replan_attempt += 1;

                let replan_msg = format!(
                    "Your previous response did not pass review: {}. \
                     Please provide a corrected response.",
                    reason
                );
                session.push_message(Message::user(replan_msg.clone()));

                steps.push(Step::new(
                    StepKind::Replan,
                    StepOutcome::Success,
                    format!("replan #{}: {}", replan_attempt, truncate(&reason, 60)),
                ));
            }
        }
    };

    // ── 7. Finish ────────────────────────────────────────────
    let last_review_failed = steps
        .iter()
        .rev()
        .find(|s| s.kind == StepKind::Review)
        .map(|s| s.is_failure())
        .unwrap_or(false);

    let (success, finish_reason) = if last_review_failed {
        (false, FinishKind::GaveUp)
    } else {
        (true, FinishKind::Completed)
    };

    steps.push(Step::new(
        StepKind::Finish,
        if success {
            StepOutcome::Success
        } else {
            StepOutcome::Partial
        },
        if success {
            "task completed".to_string()
        } else {
            "gave up after max replans".to_string()
        },
    ));

    Ok(AgentOutcome {
        response: response.message.content,
        steps,
        success,
        finish_reason,
    })
}

/// Construye la petición de chat para el provider.
fn build_request(
    provider: &dyn Provider,
    session: &AgentSession,
    identity: &AgentIdentity,
) -> ChatRequest {
    // TODO(fase-10+): pasar el modelo real configurado.
    // Por ahora tomamos el primer modelo que reporte el provider.
    let model = provider
        .models()
        .ok()
        .and_then(|m| m.into_iter().next())
        .unwrap_or_else(|| format!("{}-default", identity.name));

    ChatRequest {
        model,
        messages: session.messages.clone(),
        temperature: None,
        max_tokens: None,
        stream: false,
    }
}

/// Auto-crítica básica de una respuesta.
///
/// Reglas:
/// - La respuesta no puede estar vacía.
/// - La respuesta no puede ser solo espacios en blanco.
/// - La respuesta no puede ser absurdamente larga (> 100 KB).
///
/// Esto es deliberadamente conservador. Reglas más ricas llegarán
/// cuando exista un `mevdan-verification` (Fase 24).
fn review_response(response: &ChatResponse) -> Result<(), String> {
    let content = &response.message.content;
    if content.trim().is_empty() {
        return Err("response is empty".into());
    }
    if content.len() > 100_000 {
        return Err("response exceeds 100 KB".into());
    }
    Ok(())
}

/// Trunca un string a `max` bytes, respetando límites de caracteres.
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
    use crate::policy::{ExecutionPolicy, ReviewMode};
    use mevdan_provider::{mock::MockProvider, ChatResponse, FinishReason, Message, Role};

    fn custom_response_provider(content: &str) -> MockProvider {
        MockProvider::default().with_response(content)
    }

    fn identity() -> AgentIdentity {
        AgentIdentity::new("test", crate::identity::AgentRole::General)
    }

    #[test]
    fn truncate_short_string_is_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string_adds_ellipsis() {
        assert_eq!(truncate("hello world", 5), "hello...");
    }

    #[test]
    fn review_passes_on_normal_response() {
        let resp = ChatResponse {
            message: Message {
                role: Role::Assistant,
                content: "a real answer".into(),
            },
            model: "m".into(),
            finish_reason: FinishReason::Stop,
            usage: None,
        };
        assert!(review_response(&resp).is_ok());
    }

    #[test]
    fn review_fails_on_empty_response() {
        let resp = ChatResponse {
            message: Message {
                role: Role::Assistant,
                content: "   ".into(),
            },
            model: "m".into(),
            finish_reason: FinishReason::Stop,
            usage: None,
        };
        assert!(review_response(&resp).is_err());
    }

    #[test]
    fn review_fails_on_huge_response() {
        let resp = ChatResponse {
            message: Message {
                role: Role::Assistant,
                content: "x".repeat(200_000),
            },
            model: "m".into(),
            finish_reason: FinishReason::Stop,
            usage: None,
        };
        assert!(review_response(&resp).is_err());
    }

    #[test]
    fn loop_completes_successfully() {
        let provider = custom_response_provider("done!");
        let session = AgentSession::new(ExecutionPolicy::default());

        let outcome = run(&provider, &identity(), session, "do something").unwrap();

        assert!(outcome.success);
        assert_eq!(outcome.finish_reason, FinishKind::Completed);
        assert_eq!(outcome.response, "done!");
        assert!(!outcome.steps.is_empty());
    }

    #[test]
    fn loop_without_review_single_pass() {
        let provider = custom_response_provider("response");
        let policy = ExecutionPolicy {
            review_mode: ReviewMode::None,
            ..ExecutionPolicy::default()
        };
        let session = AgentSession::new(policy);

        let outcome = run(&provider, &identity(), session, "task").unwrap();

        assert!(outcome.success);
        assert!(outcome
            .steps
            .iter()
            .any(|s| s.kind == StepKind::Review && s.outcome == StepOutcome::Skipped));
    }

    #[test]
    fn loop_gives_up_after_empty_responses() {
        let provider = custom_response_provider("");
        let policy = ExecutionPolicy {
            max_replans: 2,
            ..ExecutionPolicy::default()
        };
        let session = AgentSession::new(policy);

        let outcome = run(&provider, &identity(), session, "task").unwrap();

        assert!(!outcome.success);
        assert_eq!(outcome.finish_reason, FinishKind::GaveUp);
        assert!(outcome.steps.iter().any(|s| s.kind == StepKind::Replan));
    }

    #[test]
    fn loop_hits_step_limit() {
        let provider = custom_response_provider("");
        let policy = ExecutionPolicy {
            max_steps: 1,
            review_mode: ReviewMode::SelfReviewWithReplan,
            max_replans: 10,
            ..ExecutionPolicy::default()
        };
        let session = AgentSession::new(policy);

        let result = run(&provider, &identity(), session, "task");
        assert!(result.is_err());
        match result.unwrap_err() {
            AgentError::MaxStepsExceeded { .. } => {}
            other => panic!("expected MaxStepsExceeded, got {:?}", other),
        }
    }

    #[test]
    fn loop_records_tokens() {
        let provider = custom_response_provider("hi");
        let policy = ExecutionPolicy::default();
        let session = AgentSession::new(policy);

        let outcome = run(&provider, &identity(), session, "task").unwrap();

        assert!(outcome.steps.iter().any(|s| s.tokens_used > 0));
    }

    #[test]
    fn loop_step_kinds_in_order() {
        let provider = custom_response_provider("ok");
        let policy = ExecutionPolicy::default();
        let session = AgentSession::new(policy);

        let outcome = run(&provider, &identity(), session, "task").unwrap();

        let kinds: Vec<StepKind> = outcome.steps.iter().map(|s| s.kind).collect();
        assert_eq!(kinds[0], StepKind::Understand);
        assert!(kinds.contains(&StepKind::Act));
        assert!(kinds.contains(&StepKind::Observe));
        assert!(kinds.contains(&StepKind::Review));
        assert_eq!(*kinds.last().unwrap(), StepKind::Finish);
    }
}
