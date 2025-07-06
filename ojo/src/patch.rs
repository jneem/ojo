use {
    anyhow::Result,
    clap::{Parser, Subcommand},
};

mod apply;
pub mod create;
mod export;
mod import;

#[derive(Parser, Debug)]
#[command(name = "patch")]
pub struct Opts {
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

#[derive(Subcommand, Debug)]
pub enum SubCommand {
    /// Apply a patch to a branch. The patch must already exist in the repository
    Apply(apply::Opts),
    /// Create a patch by comparing against a file
    Create(create::Opts),
    /// Create a file containing the contents of a patch
    Export(export::Opts),
    /// Import a patch file into the respository
    Import(import::Opts),
}

pub fn run(opts: Opts) -> Result<()> {
    match opts.subcmd {
        SubCommand::Apply(a) => apply::run(a),
        SubCommand::Create(c) => create::run(c),
        SubCommand::Export(e) => export::run(e),
        SubCommand::Import(i) => import::run(i),
    }
}
