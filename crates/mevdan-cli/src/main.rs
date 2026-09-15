//! MEVDAN CLI — punto de entrada del binario.

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
        Command::Chat {
            message,
            provider,
            model,
            base_url,
            api_key_secret,
        } => {
            commands::chat::run(
                &message,
                &provider,
                &model,
                base_url.as_deref(),
                &api_key_secret,
            )?;
        }
        Command::Tasks { status, verbose } => {
            commands::tasks::run(status.as_deref(), verbose)?;
        }
        Command::Audit {
            limit,
            category,
            json,
        } => {
            commands::audit::run(limit, category.as_deref(), json)?;
        }
    }

    Ok(())
}
