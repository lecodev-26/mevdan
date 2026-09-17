//! MEVDAN CLI — punto de entrada del binario.

mod cli;
mod commands;

use clap::Parser;
use cli::{
    Cli, CodeIntelAction, Command, DataAction, DocsAction, McpAction, MediaAction, SkillsAction,
    WorktreeAction,
};

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
        Command::Skills { action } => match action {
            SkillsAction::List { verbose } => {
                commands::skills::run_list(verbose)?;
            }
        },
        Command::Mcp { action } => match action {
            McpAction::List { verbose } => {
                commands::mcp::run_list(verbose)?;
            }
        },
        Command::Codeintel { action } => match action {
            CodeIntelAction::Map { json } => {
                commands::codeintel::run_map(json)?;
            }
        },
        Command::Route { text, json } => {
            commands::route::run(&text, json)?;
        }
        Command::Worktree { action } => match action {
            WorktreeAction::List { verbose } => {
                commands::worktree::run_list(verbose)?;
            }
        },
        Command::Docs { action } => match action {
            DocsAction::Read { path, json } => {
                commands::docs::run_read(&path, json)?;
            }
        },
        Command::Data { action } => match action {
            DataAction::Summary { path, json } => {
                commands::data::run_summary(&path, json)?;
            }
        },
        Command::Media { action } => match action {
            MediaAction::Info { path, json } => {
                commands::media::run_info(&path, json)?;
            }
        },
    }

    Ok(())
}
