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
// - output files
// - conflict resolution utility
// - figure out representation for deleted lines
//
// Algorithm for deciding on pseudo-edges:
// 1) Take the graph of changes. If necessary, "expand" the edge set by recursively exploring for
//    deleted lines. Divide it into (weakly) connected components.
// 2) For each connected component, compute the connectivity relation on its "boundary" (the nodes
//    that are present in the undeleted part of the digle). Add a pseudo-edge for every connected
//    pair. Probably, this connectivity relation needs to be computed using the entire collection
//    of deleted lines.
// 3) Remove redundant pseudo-edges by only keeping those that are covering edges.
//
// The algorithm above should work when dealing with things that were only added. What about
// changes that came from unapplying patches?
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

fn branch(repo: &Repo, m: &ArgMatches<'_>) -> String {
    m.value_of("branch")
        .unwrap_or(&repo.current_branch)
        .to_owned()
}
