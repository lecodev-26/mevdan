//! `mevdan route <text>` — muestra la decisión de routing para un texto.

use mevdan_router::{ModelCandidate, RouterEngine, RouterPolicy};

pub fn run(text: &str, json_output: bool) -> anyhow::Result<()> {
    let engine = RouterEngine::new();

    // Candidatos de ejemplo para poder demostrar la decisión.
    // En V5 esto se reemplazará con el registry real de modelos.
    let candidates = vec![
        ModelCandidate::new("gpt-4o", 128_000)
            .with_tools()
            .with_vision()
            .with_cost(2.5, 10.0),
        ModelCandidate::new("gpt-4o-mini", 128_000)
            .with_tools()
            .with_vision()
            .with_cost(0.15, 0.6),
        ModelCandidate::new("claude-3.5-sonnet", 200_000)
            .with_tools()
            .with_vision()
            .with_cost(3.0, 15.0),
        ModelCandidate::new("llama3.2", 32_000).with_tools(),
    ];

    let decision = engine.route_text(text, &candidates, RouterPolicy::Automatic)?;

    if json_output {
        let json = serde_json::to_string_pretty(&decision)?;
        println!("{}", json);
        return Ok(());
    }

    println!("MEVDAN — Route Decision");
    println!("───────────────────────────────────────");
    println!("Text:      {}", text);
    println!();
    println!("Task kind: {}", decision.task_kind.display_name());
    println!();
    println!("Agent:");
    println!("  Role:   {:?}", decision.agent.role);
    println!("  Reason: {}", decision.agent.reason);
    println!();
    println!("Model:");
    println!("  Chosen: {}", decision.model.model_name);
    println!("  Policy: {}", decision.model.policy.display_name());
    println!("  Reason: {}", decision.model.reason);
    if !decision.model.alternatives.is_empty() {
        println!("  Alternatives: {}", decision.model.alternatives.join(", "));
    }

    Ok(())
}
