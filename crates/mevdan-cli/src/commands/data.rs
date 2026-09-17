//! `mevdan data` — inspeccionar datasets.

use mevdan_data::DataLoader;

pub fn run_summary(path: &str, json: bool) -> anyhow::Result<()> {
    let loader = DataLoader::new();
    let table = loader.load(path)?;

    if json {
        let json = serde_json::to_string_pretty(&table)?;
        println!("{}", json);
        return Ok(());
    }

    println!("MEVDAN — Data");
    println!("───────────────────────────────────────");
    println!("Path:   {}", path);
    println!("Rows:   {}", table.row_count());
    println!("Cols:   {}", table.column_count());
    println!();

    println!("Columns:");
    for col in table.columns() {
        println!("  {:20} {}", col.name, col.ty.display_name());
    }

    if !table.is_empty() {
        println!();
        println!("First 5 rows:");
        let head = table.head(5);
        for row_idx in 0..head.row_count() {
            let row = head.row(row_idx)?;
            let cells: Vec<String> = row.iter().map(|v| v.to_display_string()).collect();
            println!("  | {} |", cells.join(" | "));
        }
    }

    Ok(())
}
