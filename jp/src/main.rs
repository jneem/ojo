#[macro_use]
extern crate clap;

#[macro_use]
extern crate failure;

#[macro_use]
extern crate log as other_log;

use clap::{App, ArgMatches};
use failure::{Error, ResultExt};
use flexi_logger::Logger;
use libjp::Repo;

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

// TODO: commands for importing and exporting patches

fn main() {
    let yml = load_yaml!("main.yaml");
    let m = App::from_yaml(yml).get_matches();

    Logger::with_env()
        //.log_to_file()
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    let result = match m.subcommand_name() {
        Some("branch") => branch::run(m.subcommand_matches("branch").unwrap()),
        Some("clear") => clear::run(m.subcommand_matches("clear").unwrap()),
        Some("diff") => diff::run(m.subcommand_matches("diff").unwrap()),
        Some("graph") => graph::run(m.subcommand_matches("graph").unwrap()),
        Some("init") => init::run(m.subcommand_matches("init").unwrap()),
        Some("log") => log::run(m.subcommand_matches("log").unwrap()),
        Some("patch") => patch::run(m.subcommand_matches("patch").unwrap()),
        Some("render") => render::run(m.subcommand_matches("render").unwrap()),
        Some("resolve") => resolve::run(m.subcommand_matches("resolve").unwrap()),
        Some("synthesize") => synthesize::run(m.subcommand_matches("synthesize").unwrap()),
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

fn file_path(m: &ArgMatches<'_>) -> String {
    m.value_of("path").unwrap_or("jp_file.txt").to_owned()
}
