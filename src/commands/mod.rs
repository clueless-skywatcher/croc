use clap::Subcommand;

use crate::commands::{
    cat_file::CatFileCommand, hash_object::HashObjectCommand, init::InitCommand,
    ls_tree::LsTreeCommand,
};

pub mod cat_file;
pub mod hash_object;
pub mod init;
pub mod ls_tree;

#[derive(Subcommand, Debug, Clone)]
pub enum CrocCommands {
    Init(InitCommand),
    CatFile(CatFileCommand),
    HashObject(HashObjectCommand),
    LsTree(LsTreeCommand),
}

pub trait Runnable {
    fn run(&self) -> anyhow::Result<()>;
}
