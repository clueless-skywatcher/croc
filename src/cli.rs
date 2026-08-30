use clap::Parser;

use crate::commands::CrocCommands;

#[derive(Parser, Debug)]
pub struct Croc {
    #[command(subcommand)]
    pub command: CrocCommands
}