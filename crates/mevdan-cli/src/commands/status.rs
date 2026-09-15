//! `mevdan status` — muestra el estado del proyecto actual.
//!
//! Busca `.mevdan/project.toml` subiendo por los directorios desde el
//! directorio actual. Abre la base de datos y resume el contenido.

use mevdan_storage::{
    db::Database,
    repo::{event_repo, project_repo, session_repo},
};
use std::{env, path::PathBuf};

pub fn run() -> anyhow::Result<()> {
    let project_dir = find_project_root()?;
    let db = Database::open(&project_dir)?;

    let project = project_repo::get_first(db.connection())?
        .ok_or_else(|| anyhow::anyhow!("no project found in database"))?;

    let sessions = session_repo::list_by_project(db.connection(), project.id)?;
    let event_count = event_repo::count_by_project(db.connection(), project.id)?;

    println!("MEVDAN — Project Status");
    println!("───────────────────────────────────────");
    println!("Name:           {}", project.name);
    println!("ID:             {}", project.id);
    println!("Schema:         {}", project.schema_version);
    println!("MEVDAN version: {}", project.mevdan_version);
    println!("Created:        {}", project.created_at.to_rfc3339());
    println!("Updated:        {}", project.updated_at.to_rfc3339());
    println!();
    println!("Sessions:       {}", sessions.len());
    println!("Events:         {}", event_count);
    println!();
    println!("Directory:      {}", project_dir.display());
    println!("Database:       {}", db.path().display());

    Ok(())
}

/// Busca `.mevdan/project.toml` subiendo por el árbol de directorios.
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
