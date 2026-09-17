//! `mevdan docs` — leer documentos.

use mevdan_documents::DocumentLoader;

pub fn run_read(path: &str, json: bool) -> anyhow::Result<()> {
    let loader = DocumentLoader::new();
    let loaded = loader.load(path)?;

    if json {
        let json = serde_json::to_string_pretty(&loaded)?;
        println!("{}", json);
        return Ok(());
    }

    println!("MEVDAN — Document");
    println!("───────────────────────────────────────");
    println!("File:      {}", loaded.document.file_name());
    println!("Path:      {}", loaded.document.path.display());
    println!("Format:    {}", loaded.document.format.display_name());
    println!("Size:      {} bytes", loaded.document.size_bytes);
    if let Some(title) = &loaded.document.title {
        println!("Title:     {}", title);
    }
    println!();

    println!(
        "Extracted: {}",
        if loaded.content.extracted {
            "yes"
        } else {
            "no"
        }
    );
    if loaded.content.extracted {
        println!("Lines:     {}", loaded.content.line_count);
        println!("Words:     {}", loaded.content.word_count);

        if !loaded.content.sections.is_empty() {
            println!();
            println!("Sections:");
            for section in &loaded.content.sections {
                if section.is_heading() {
                    println!("  {} {}", "#".repeat(section.level as usize), section.title);
                }
            }
        }
    }

    Ok(())
}
