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
        name: String,
        #[arg(long)]
        path: Option<PathBuf>,
    },

    /// Show current project status
    Status,

    /// Send a single chat message to a provider
    Chat {
        message: String,
        #[arg(short, long, default_value = "ollama")]
        provider: String,
        #[arg(short, long)]
        model: String,
        #[arg(long)]
        base_url: Option<String>,
        #[arg(long, default_value = "openai_api_key")]
        api_key_secret: String,
    },

    /// List tasks from the project's Work Graph
    Tasks {
        #[arg(short, long)]
        status: Option<String>,
        #[arg(short, long)]
        verbose: bool,
    },

    /// Show the audit trail of the project
    Audit {
        #[arg(short, long)]
        limit: Option<usize>,
        #[arg(short, long)]
        category: Option<String>,
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
        text: String,
        #[arg(long)]
        json: bool,
    },

    /// Manage worktrees
    Worktree {
        #[command(subcommand)]
        action: WorktreeAction,
    },

    /// Read documents
    Docs {
        #[command(subcommand)]
        action: DocsAction,
    },

    /// Inspect datasets
    Data {
        #[command(subcommand)]
        action: DataAction,
    },

    /// Inspect media files
    Media {
        #[command(subcommand)]
        action: MediaAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum SkillsAction {
    /// List installed skills
    List {
        #[arg(short, long)]
        verbose: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum McpAction {
    /// List MCP servers
    List {
        #[arg(short, long)]
        verbose: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum CodeIntelAction {
    /// Show the repository map
    Map {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum WorktreeAction {
    /// List worktrees
    List {
        #[arg(short, long)]
        verbose: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum DocsAction {
    /// Read a document
    Read {
        /// Path to the document
        path: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum DataAction {
    /// Summarize a dataset
    Summary {
        /// Path to the dataset
        path: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum MediaAction {
    /// Show media info
    Info {
        /// Path to the media file
        path: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}
