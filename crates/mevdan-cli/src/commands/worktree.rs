//! `mevdan worktree` — gestionar worktrees.

use mevdan_worktree::WorktreeManager;
use std::env;
use std::path::PathBuf;

pub fn run_list(verbose: bool) -> anyhow::Result<()> {
    let project_dir = find_project_root()?;
    let manager = WorktreeManager::new(&project_dir)?;

    println!("Worktrees:");
    println!();

    for wt in manager.list() {
        let marker = if wt.is_main { " [main]" } else { "" };
        println!("  • {}{}", wt.name, marker);
        if verbose {
            println!("    path: {}", wt.path.display());
            if let Some(desc) = &wt.description {
                println!("    description: {}", desc);
            }
            println!("    created: {}", wt.created_at.format("%Y-%m-%d %H:%M"));
        }
    }

    println!();
    println!("{} worktree(s) total.", manager.len());

    Ok(())
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
