#[macro_use]
extern crate clap;

#[macro_use]
extern crate failure;

use clap::{App, ArgMatches};
use failure::{Error, ResultExt};
use libjp::Repo;

mod branch;
mod diff;
mod graph;
mod init;
mod log;
mod patch;

// TODO:
// - make diff display things more nicely
// - output (graphs and/or files)
fn main() {
    let yml = load_yaml!("main.yaml");
    let m = App::from_yaml(yml).get_matches();

    let result = match m.subcommand_name() {
        Some("branch") => branch::run(m.subcommand_matches("branch").unwrap()),
        Some("diff") => diff::run(m.subcommand_matches("diff").unwrap()),
        Some("graph") => graph::run(m.subcommand_matches("graph").unwrap()),
        Some("init") => init::run(m.subcommand_matches("init").unwrap()),
        Some("log") => log::run(m.subcommand_matches("log").unwrap()),
        Some("patch") => patch::run(m.subcommand_matches("patch").unwrap()),
        _ => panic!("Unknown subcommand"),
    };

    if let Err(e) = result {
        println!("Error: {}", e);
        for cause in e.iter_causes() {
            println!("\tcaused by: {}", cause);
        }
        std::process::exit(1);
    }
}

fn open_repo() -> Result<libjp::Repo, Error> {
    let mut dir = std::env::current_dir().context("Could not open the current directory")?;
    loop {
        let mut jp_dir = dir.clone();
        jp_dir.push(".jp");
        if jp_dir.is_dir() {
            return Ok(libjp::Repo::open(dir).context("Failed to open the jp repository")?);
        }
        if !dir.pop() {
            bail!("Failed to find a jp repository");
        }
    }
}

fn branch(repo: &Repo, m: &ArgMatches) -> String {
    m.value_of("branch")
        .unwrap_or(&repo.current_branch)
        .to_owned()
}

