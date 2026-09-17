//! Definición de la interfaz de línea de comandos.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "mevdan",
    version,
    about = "MEVDAN — Any model. Any agent. Your work.",
    long_about = "Local-first AI work runtime.\n\nThe model proposes; MEVDAN decides."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Initialize a new MEVDAN project
    Init {
        /// Project name (also used as directory name)
        name: String,

        /// Optional parent directory (default: current directory)
        #[arg(long)]
        path: Option<PathBuf>,
    },

    /// Show current project status
    Status,

    /// Send a single chat message to a provider
    Chat {
        /// The message to send
        message: String,

        /// Provider to use: "openai-compatible" or "ollama"
        #[arg(short, long, default_value = "ollama")]
        provider: String,

        /// Model name
        #[arg(short, long)]
        model: String,

        /// Base URL (provider-specific)
        #[arg(long)]
        base_url: Option<String>,

        /// Secret name to read the API key from (for openai-compatible)
        #[arg(long, default_value = "openai_api_key")]
        api_key_secret: String,
    },

    /// List tasks from the project's Work Graph
    Tasks {
        /// Filter by status (pending, ready, running, completed, failed...)
        #[arg(short, long)]
        status: Option<String>,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Show the audit trail of the project
    Audit {
        /// Show only the last N entries
        #[arg(short, long)]
        limit: Option<usize>,

        /// Filter by category (lifecycle, session, agent, tool, task...)
        #[arg(short, long)]
        category: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Manage skills
    Skills {
        #[command(subcommand)]
        action: SkillsAction,
    },

    /// Manage MCP servers
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },

    /// Code intelligence
    Codeintel {
        #[command(subcommand)]
        action: CodeIntelAction,
    },

    /// Show the routing decision for a piece of text
    Route {
        /// The text to classify and route
        text: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum SkillsAction {
    /// List installed skills
    List {
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum McpAction {
    /// List MCP servers
    List {
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum CodeIntelAction {
    /// Show the repository map
    Map {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}
