use clap::Args;

use crate::commands::Runnable;

#[derive(Args, Debug, Clone)]
#[group(required = true, multiple = false)]
struct CatFileMode {
    #[arg(short = 't')]
    show_type: bool,

    #[arg(short = 'p')]
    show_pretty: bool,

    #[arg(short = 's')]
    show_size: bool,

    #[arg(short = 'e')]
    show_exists: bool
}

#[derive(Args, Debug, Clone)]
#[command(name = "cat-file")]
pub struct CatFileCommand {
    #[command(flatten)]
    mode: CatFileMode,

    pub what: String
}

impl Runnable for CatFileCommand {
    fn run(&self) -> anyhow::Result<()> {
        Ok(())
    }
}