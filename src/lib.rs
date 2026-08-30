pub mod commands;
pub mod cli;

use crate::cli::Croc;
use crate::commands::{CrocCommands, Runnable};

/// Dispatch a parsed CLI to the command that handles it.
pub fn run(cli: Croc) -> anyhow::Result<()> {
    match cli.command {
        CrocCommands::Init(cmd) => cmd.run(),
        CrocCommands::CatFile(cmd) => cmd.run(),
        CrocCommands::HashObject(cmd) => cmd.run(),
    }
}
