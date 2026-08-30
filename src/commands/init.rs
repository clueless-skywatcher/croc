use std::{fs, path::PathBuf};

use clap::Args;

use crate::commands::Runnable;

#[derive(Args, Debug, Clone)]
pub struct InitCommand {
    #[arg(short, long, default_value = ".")]
    pub path: PathBuf
}

impl Runnable for InitCommand {
    fn run(&self) -> anyhow::Result<()> {
        let git_path = self.path.clone().join(".croc");
        fs::create_dir_all(git_path.join("info"))?;
        fs::create_dir_all(git_path.join("objects"))?;
        fs::create_dir_all(git_path.join("refs").join("tags"))?;
        fs::create_dir_all(git_path.join("refs").join("heads"))?;
        Ok(())
    }
}