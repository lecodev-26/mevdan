//! MEVDAN CLI — punto de entrada del binario.
//!
//! Este archivo solo despacha a los comandos. La lógica real vive en
//! `commands/`.

mod cli;
mod commands;

use clap::Parser;
use cli::{Cli, Command};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init { name, path } => {
            commands::init::run(&name, path.as_deref())?;
        }
        Command::Status => {
            commands::status::run()?;
        }
    }

    Ok(())
}
