//! `mevdan skills` — gestionar skills.

use mevdan_skills::{SkillDiscovery, SkillLoader, SkillSource};
use std::env;
use std::path::PathBuf;

pub fn run_list(verbose: bool) -> anyhow::Result<()> {
    let project_dir = find_project_root()?;

    let discovery = SkillDiscovery::new();
    let skills = discovery.discover(Some(&project_dir))?;

    if skills.is_empty() {
        println!("No skills installed.");
        println!();
        println!(
            "Skills live in {}/.mevdan/skills/. Install one with \
             a directory containing skill.toml.",
            project_dir.display()
        );
        return Ok(());
    }

    println!("Installed skills:");
    println!();

    for ds in &skills {
        let source = match ds.source {
            SkillSource::Project => "project",
            SkillSource::User => "user",
            SkillSource::System => "system",
            SkillSource::Custom => "custom",
        };
        println!("  • {}@{} [{}]", ds.name(), ds.version(), source);
        if verbose {
            println!("    description: {}", ds.skill.description());
            println!("    path: {}", ds.skill.path.display());
            if let Some(prompt) = ds.skill.system_prompt() {
                let preview = if prompt.len() > 80 {
                    format!("{}...", &prompt[..80])
                } else {
                    prompt.to_string()
                };
                println!("    system_prompt: {}", preview);
            }
            let tools = ds.skill.required_tools();
            if !tools.is_empty() {
                println!("    required_tools: {}", tools.join(", "));
            }
        }
    }

    println!();
    println!("{} skill(s) found.", skills.len());

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

// Silencia el import sin usar cuando no se usa `SkillLoader` directamente.
#[allow(dead_code)]
fn _silence() {
    let _ = SkillLoader::default_skills_dir("/tmp");
}
