//! Definición de la interfaz de línea de comandos.
//!
//! Solo se declaran los comandos disponibles. La implementación vive
//! en `commands/`.

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
}
