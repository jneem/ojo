#[macro_use]
extern crate clap;

#[macro_use]
extern crate failure;

use clap::App;
use failure::Error;

mod apply;
mod diff;
mod init;
mod log;
mod patch;

// TODO:
// - make diff display things more nicely
// - output (graphs and/or files)
fn main() -> Result<(), Error> {
    let yml = load_yaml!("main.yaml");
    let m = App::from_yaml(yml).get_matches();

    match m.subcommand_name() {
        Some("apply") => apply::run(m.subcommand_matches("apply").unwrap()),
        Some("diff") => diff::run(m.subcommand_matches("diff").unwrap()),
        Some("init") => init::run(m.subcommand_matches("init").unwrap()),
        Some("log") => log::run(m.subcommand_matches("log").unwrap()),
        Some("patch") => patch::run(m.subcommand_matches("patch").unwrap()),
        _ => panic!("Unknown subcommand"),
    }
}
