//! `mevdan tasks` — lista las tareas del proyecto.
//!
//! Las tareas se cargan desde el último Work Graph persistido. Si no
//! hay Work Graph, se indica que no hay tareas todavía.

use mevdan_storage::{
    db::Database,
    repo::{project_repo, workgraph_repo},
};
use mevdan_task::TaskStatus;
use mevdan_workgraph::NodeKind;
use std::env;
use std::path::PathBuf;

pub fn run(status_filter: Option<&str>, verbose: bool) -> anyhow::Result<()> {
    let project_dir = find_project_root()?;
    let db = Database::open(&project_dir)?;

    let project = project_repo::get_first(db.connection())?
        .ok_or_else(|| anyhow::anyhow!("no project found in database"))?;

    // Cargar el último Work Graph del proyecto.
    let workgraphs = workgraph_repo::list_by_project(db.connection(), &project.id.to_string())?;

    if workgraphs.is_empty() {
        println!("No tasks yet.");
        println!();
        println!(
            "Tasks are created when the agent runs (V0.4.0+). \
             For now, this view shows tasks from the Work Graph."
        );
        return Ok(());
    }

    let wg_id = workgraphs[0].id;
    let wg_record = workgraph_repo::get(db.connection(), wg_id)?
        .ok_or_else(|| anyhow::anyhow!("workgraph disappeared"))?;

    // Obtener todos los nodos de tipo Task.
    let task_nodes = wg_record.graph.nodes_by_kind(NodeKind::Task);

    if task_nodes.is_empty() {
        println!("No tasks in the current Work Graph.");
        return Ok(());
    }

    // Aplicar filtro de status si aplica.
    let filter = match status_filter {
        Some(s) => Some(parse_status_filter(s)?),
        None => None,
    };

    let mut displayed = 0;
    println!("Tasks (from Work Graph '{}'):", wg_id);
    println!();

    for node in &task_nodes {
        let status_str = node
            .data
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        // Si hay filtro, aplicarlo.
        if let Some(f) = &filter {
            if status_str != f.display_name() {
                continue;
            }
        }

        displayed += 1;
        println!("  • {} [{}]", node.label, status_str);
        if verbose {
            println!("    id: {}", node.id);
            if let Some(desc) = node.data.get("description").and_then(|v| v.as_str()) {
                println!("    description: {}", desc);
            }
            if let Some(priority) = node.data.get("priority").and_then(|v| v.as_str()) {
                println!("    priority: {}", priority);
            }
        }
    }

    println!();
    if displayed == 0 {
        println!("No tasks match the filter.");
    } else {
        println!("{} task(s) shown.", displayed);
    }

    Ok(())
}

/// Parsea el filtro de status desde string.
fn parse_status_filter(s: &str) -> anyhow::Result<TaskStatus> {
    let normalized = s.to_lowercase();
    match normalized.as_str() {
        "pending" => Ok(TaskStatus::Pending),
        "ready" => Ok(TaskStatus::Ready),
        "running" => Ok(TaskStatus::Running),
        "blocked" => Ok(TaskStatus::Blocked),
        "waiting_approval" | "waiting" => Ok(TaskStatus::WaitingApproval),
        "completed" | "done" => Ok(TaskStatus::Completed),
        "failed" => Ok(TaskStatus::Failed),
        "cancelled" | "canceled" => Ok(TaskStatus::Cancelled),
        other => anyhow::bail!(
            "unknown status: '{}'. Valid: pending, ready, running, blocked, waiting_approval, completed, failed, cancelled",
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
