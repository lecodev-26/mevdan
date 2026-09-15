//! `mevdan audit` — muestra el timeline del proyecto.

use mevdan_storage::{
    audit::{AuditTrail, EventCategory},
    db::Database,
    repo::project_repo,
};
use std::env;
use std::path::PathBuf;

pub fn run(
    limit: Option<usize>,
    category_filter: Option<&str>,
    json_output: bool,
) -> anyhow::Result<()> {
    let project_dir = find_project_root()?;
    let db = Database::open(&project_dir)?;

    let project = project_repo::get_first(db.connection())?
        .ok_or_else(|| anyhow::anyhow!("no project found in database"))?;

    let trail = AuditTrail::build(db.connection(), project.id)?;

    // Aplicar filtro por categoría.
    let filtered = match category_filter {
        Some(cat) => {
            let category = parse_category(cat)?;
            trail.by_category(category)
        }
        None => trail.entries().iter().collect(),
    };

    // Aplicar límite (últimos N).
    let to_show: Vec<&mevdan_storage::TimelineEntry> = if let Some(n) = limit {
        let len = filtered.len();
        if len <= n {
            filtered
        } else {
            filtered[len - n..].to_vec()
        }
    } else {
        filtered
    };

    if json_output {
        // Salida JSON.
        let json = serde_json::to_string_pretty(&to_show)?;
        println!("{}", json);
        return Ok(());
    }

    // Salida humana.
    println!("MEVDAN — Audit Trail");
    println!("───────────────────────────────────────");
    println!("Project: {}", project.name);
    println!("Total events: {}", trail.len());
    println!();

    if to_show.is_empty() {
        println!("(no events to show)");
        return Ok(());
    }

    for entry in &to_show {
        println!("{}", entry.human_line());
    }

    println!();
    println!("{} event(s) shown.", to_show.len());

    Ok(())
}

/// Parsea la categoría desde string.
fn parse_category(s: &str) -> anyhow::Result<EventCategory> {
    let normalized = s.to_lowercase();
    match normalized.as_str() {
        "lifecycle" => Ok(EventCategory::Lifecycle),
        "session" => Ok(EventCategory::Session),
        "agent" => Ok(EventCategory::Agent),
        "tool" => Ok(EventCategory::Tool),
        "task" => Ok(EventCategory::Task),
        "permission" => Ok(EventCategory::Permission),
        "checkpoint" => Ok(EventCategory::Checkpoint),
        "verification" => Ok(EventCategory::Verification),
        "unknown" => Ok(EventCategory::Unknown),
        other => anyhow::bail!(
            "unknown category: '{}'. Valid: lifecycle, session, agent, tool, task, permission, checkpoint, verification, unknown",
            other
        ),
    }
}

/// Busca `.mevdan/project.toml` subiendo por el árbol.
fn find_project_root() -> anyhow::Result<PathBuf> {
    let mut dir = env::current_dir()?;
    loop {
        if dir.join(".mevdan").join("project.toml").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            anyhow::bail!("not inside a MEVDAN project (no .mevdan/project.toml found)");
        }
    }
}
