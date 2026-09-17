//! `mevdan mcp` — gestionar servidores MCP.
//!
//! V4: comando informativo. La configuración persistente de servidores
//! MCP llegará en V5.

use std::env;
use std::path::PathBuf;

pub fn run_list(_verbose: bool) -> anyhow::Result<()> {
    let project_dir = find_project_root()?;

    println!("MCP servers:");
    println!();
    println!("  No MCP servers configured in {}.", project_dir.display());
    println!();
    println!(
        "  Persistent MCP configuration arrives in V5. The client, \
         tool registry and security layer are already implemented in \
         mevdan-mcp (V4.2-V4.4)."
    );
    println!();
    println!("  You can already use it programmatically:");
    println!("    McpClient::connect_stdio(server)?.initialize()?;");
    println!("    registry.add_client(\"fs\", client)?;");
    println!("    registry.invoke(&guard, \"fs.read_file\", args)?;");

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
