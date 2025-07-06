use {
    anyhow::{Context, Result, bail},
    clap::{ColorChoice, Parser},
    flexi_logger::Logger,
    libojo::Repo,
};

mod branch;
mod clear;
mod diff;
mod graph;
mod init;
mod log;
pub mod patch;
mod render;
mod resolve;
mod synthesize;

#[derive(Parser, Debug)]
#[clap(version, author, color(ColorChoice::Auto), infer_subcommands = true)]
#[command(
    name = "ojo",
    about = "An educational and proof-of-concept version control system."
)]
pub struct Opts {
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

#[derive(Parser, Debug)]
pub enum SubCommand {
    /// Various commands related to branches
    Branch(branch::Opts),
    /// Delete all patches from a branch (mainly for debugging)
    Clear(clear::Opts),
    /// Show changes between commits
    Diff(diff::Opts),
    /// Create a .dot file for visualizing the stored file
    Graph(graph::Opts),
    /// Create a new ojo repository
    Init,
    /// Print all of the patches present on a branch
    Log(log::Opts),
    /// Various commands related to patches
    Patch(patch::Opts),
    /// Output the tracked data to a file
    Render(render::Opts),
    /// Interactive utility to make the file totally ordered
    Resolve(resolve::Opts),
    /// Synthesize a repository with an arbitrary graph (for testing)
    Synthesize,
}

fn main() {
    let opts = Opts::parse();

    Logger::try_with_env()
        .unwrap()
        //.log_to_file()
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

    let result = match opts.subcmd {
        SubCommand::Branch(b) => branch::run(b),
        SubCommand::Clear(c) => clear::run(c),
        SubCommand::Diff(d) => diff::run(d),
        SubCommand::Graph(g) => graph::run(g),
        SubCommand::Init => init::run(),
        SubCommand::Log(l) => log::run(l),
        SubCommand::Patch(p) => patch::run(p),
        SubCommand::Render(r) => render::run(r),
        SubCommand::Resolve(r) => resolve::run(r),
        SubCommand::Synthesize => synthesize::run(),
    };

    if let Err(e) = result {
        println!("Error: {e}");
        for cause in e.chain().skip(1) {
            println!("\tcaused by: {cause}");
        }
        std::process::exit(1);
    }
}

fn open_repo() -> Result<libojo::Repo> {
    let mut dir = std::env::current_dir().context("Could not open the current directory")?;
    loop {
        let mut ojo_dir = dir.clone();
        ojo_dir.push(".ojo");
        if ojo_dir.is_dir() {
            return libojo::Repo::open(dir).context("Failed to open the ojo repository");
        }
        if !dir.pop() {
            bail!("Failed to find a ojo repository");
        }
    }
}

fn branch(repo: &Repo, branch: Option<String>) -> String {
    branch.unwrap_or(repo.current_branch.clone()).to_owned()
}
