use std::path::PathBuf;

use clap::{CommandFactory, Parser};
use croc::{cli::Croc, commands::CrocCommands};
use tempfile::TempDir;

fn run_croc(args: &[&str]) -> anyhow::Result<()> {
    let argv = std::iter::once("croc").chain(args.iter().copied());
    croc::run(Croc::try_parse_from(argv)?)
}

#[test]
fn cli_definition_is_valid() {
    Croc::command().debug_assert();
}

#[test]
fn init_defaults_to_current_directory() {
    let cli = Croc::try_parse_from(["croc", "init"]).unwrap();
    let CrocCommands::Init(cmd) = cli.command else {
        return;
    };
    assert_eq!(cmd.path, PathBuf::from("."));
}

#[test]
fn init_accepts_short_and_long_path_flags() {
    for args in [
        ["croc", "init", "-p", "/tmp/x"],
        ["croc", "init", "--path", "/tmp/x"],
    ] {
        let cli = Croc::try_parse_from(args).unwrap();
        let CrocCommands::Init(cmd) = cli.command else {
            return;
        };
        assert_eq!(cmd.path, PathBuf::from("/tmp/x"));
    }
}

#[test]
fn unknown_flag_is_rejected() {
    let err = Croc::try_parse_from(["croc", "init", "--nope"]).unwrap_err();
    assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument);
}

#[test]
fn init_creates_croc_skeleton() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    run_croc(&["init", "--path", root.to_str().unwrap()]).unwrap();

    let croc_dir = root.join(".croc");
    assert!(croc_dir.join("info").is_dir());
    assert!(croc_dir.join("objects").is_dir());
    assert!(croc_dir.join("refs").join("tags").is_dir());
    assert!(croc_dir.join("refs").join("heads").is_dir());
}

#[test]
fn init_is_idempotent() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_str().unwrap();

    run_croc(&["init", "--path", path]).unwrap();
    run_croc(&["init", "--path", path]).expect("re-running init must not fail");

    assert!(tmp.path().join(".croc").join("objects").is_dir());
}
