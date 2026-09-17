mod cargo_ops;
mod cli;
mod dependency;
mod error;
mod project;
mod prompts;
mod scaffold;
mod template;
mod templates;

use clap::Parser;
use cli::{Cli, Command};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Create(args) => project::run_create(args)?,
    }
    Ok(())
}
