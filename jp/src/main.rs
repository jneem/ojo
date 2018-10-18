#[macro_use]
extern crate clap;

use clap::App;

mod apply;
mod diff;
mod init;
mod log;
mod patch;

// TODO:
// - commit
// - make diff display things more nicely
fn main() -> Result<(), libjp::Error> {
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
