use clap::Subcommand;

use crate::commands::{
    cat_file::CatFileCommand, hash_object::HashObjectCommand, init::InitCommand,
};

pub mod cat_file;
pub mod hash_object;
pub mod init;

#[derive(Subcommand, Debug, Clone)]
pub enum CrocCommands {
    Init(InitCommand),
    CatFile(CatFileCommand),
    HashObject(HashObjectCommand),
}

pub trait Runnable {
    fn run(&self) -> anyhow::Result<()>;
}
