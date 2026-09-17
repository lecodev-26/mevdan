//! `mevdan codeintel` — inteligencia de código.

use mevdan_codeintel::CodeIntelEngine;
use std::env;
use std::path::{Path, PathBuf};

pub fn run_map(json_output: bool) -> anyhow::Result<()> {
    // codeintel opera sobre el directorio actual (asumiendo que estamos
    // dentro de un repo). El project_root es informativo.
    let current_dir = env::current_dir()?;
    let project_dir = find_project_root().ok();
    let scan_dir: &Path = project_dir.as_deref().unwrap_or(&current_dir);

    let engine = CodeIntelEngine::new();
    let report = engine.analyze(scan_dir)?;

    if json_output {
        let json = serde_json::to_string_pretty(&report)?;
        println!("{}", json);
        return Ok(());
    }

    println!("MEVDAN — Code Intelligence");
    println!("───────────────────────────────────────");
    println!("Root:         {}", report.repo.root);
    println!("Files:        {}", report.repo.file_count());
    println!("Directories:  {}", report.repo.dir_count());
    println!();

    if !report.repo.info.languages.is_empty() {
        println!("Languages:");
        for (lang, count) in &report.repo.info.languages {
            println!("  {:14} {:>4} file(s)", lang.name(), count);
        }
        if let Some(primary) = report.repo.info.primary_language() {
            println!();
            println!("Primary language: {}", primary.name());
        }
        println!();
    }

    if !report.repo.info.build_systems.is_empty() {
        println!(
            "Build systems: {}",
            report
                .repo
                .info
                .build_systems
                .iter()
                .map(|b| b.name())
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!();
    }

    let markers: Vec<&str> = [
        (report.repo.info.has_git, "git"),
        (report.repo.info.has_tests, "tests"),
        (report.repo.info.has_ci, "ci"),
        (report.repo.info.has_readme, "readme"),
        (report.repo.info.has_license, "license"),
    ]
    .iter()
    .filter(|(present, _)| *present)
    .map(|(_, name)| *name)
    .collect();

    if !markers.is_empty() {
        println!("Markers: {}", markers.join(", "));
        println!();
    }

    if !report.symbols.is_empty() {
        println!("Symbols:");
        println!("  Total: {}", report.symbols.len());

        let mut by_kind: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for s in &report.symbols.symbols {
            *by_kind
                .entry(s.kind.display_name().to_string())
                .or_insert(0) += 1;
        }
        let mut kinds: Vec<_> = by_kind.into_iter().collect();
        kinds.sort_by_key(|a| std::cmp::Reverse(a.1));
        for (kind, count) in kinds.iter().take(5) {
            println!("  {:14} {:>4}", kind, count);
        }
        println!();
    }

    if !report.dependencies.is_empty() {
        println!(
            "Dependencies: {} ({} internal, {} external)",
            report.dependencies.len(),
            report.dependencies.internal().len(),
            report.dependencies.external().len(),
        );
        println!();
    }

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
            anyhow::bail!("not inside a MEVDAN project");
        }
    }
}
