//! `mevdan init <name>` — crea un proyecto MEVDAN.
//!
//! Pasos:
//!   1. Validar nombre del proyecto.
//!   2. Crear estructura `.mevdan/` con subdirectorios.
//!   3. Escribir `project.toml`.
//!   4. Abrir/crear la base de datos (aplica migraciones).
//!   5. Insertar Project, Session inicial, y evento ProjectCreated.
//!   6. Imprimir resumen.

use mevdan_core::{event::EventKind, project::Project, session::Session, MEVDAN_VERSION};
use mevdan_storage::{
    db::Database,
    repo::{event_repo, project_repo, session_repo},
};
use std::{fs, path::Path};

pub fn run(name: &str, parent: Option<&Path>) -> anyhow::Result<()> {
    validate_name(name)?;

    let parent = parent.unwrap_or_else(|| Path::new("."));
    let project_dir = parent.join(name);

    if project_dir.exists() {
        anyhow::bail!("directory already exists: {}", project_dir.display());
    }

    // Estructura .mevdan/
    let mevdan_dir = project_dir.join(".mevdan");
    fs::create_dir_all(&mevdan_dir)?;
    fs::create_dir_all(mevdan_dir.join("artifacts"))?;
    fs::create_dir_all(mevdan_dir.join("checkpoints"))?;
    fs::create_dir_all(mevdan_dir.join("events"))?;

    // Project (dominio)
    let project = Project::new(name, MEVDAN_VERSION);

    // project.toml legible
    write_project_toml(&project_dir, &project)?;

    // Abrir DB (crea archivo + migraciones)
    let db = Database::open(&project_dir)?;

    // Persistir
    project_repo::insert(db.connection(), &project)?;

    let session = Session::new(project.id, Some("initial".to_string()));
    session_repo::insert(db.connection(), &session)?;

    event_repo::append(
        db.connection(),
        project.id,
        Some(session.id),
        EventKind::ProjectCreated,
        serde_json::json!({
            "name": name,
            "mevdan_version": MEVDAN_VERSION,
        }),
    )?;

    // Output
    println!("✔ MEVDAN project initialized at {}", project_dir.display());
    println!("  Project ID: {}", project.id);
    println!("  Session ID: {}", session.id);
    println!();
    println!("Next:");
    println!("  cd {}", name);
    println!("  mevdan status");

    Ok(())
}

fn validate_name(name: &str) -> anyhow::Result<()> {
    if name.is_empty() || name.len() > 64 {
        anyhow::bail!("project name must be 1–64 characters");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        anyhow::bail!("project name may only contain a-z, A-Z, 0-9, '-', '_'");
    }
    Ok(())
}

fn write_project_toml(dir: &Path, project: &Project) -> anyhow::Result<()> {
    let content = format!(
        r#"[project]
name = "{name}"
schema_version = "{schema}"
mevdan_version = "{version}"
created_at = "{created}"

[storage]
db_file = "mevdan.db"
"#,
        name = project.name,
        schema = project.schema_version,
        version = project.mevdan_version,
        created = project.created_at.to_rfc3339(),
    );
    fs::write(dir.join(".mevdan").join("project.toml"), content)?;
    Ok(())
}
