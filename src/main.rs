use clap::Parser;
use croc::cli::Croc;

fn main() -> anyhow::Result<()> {
    croc::run(Croc::parse())
}
