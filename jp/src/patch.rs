use clap::ArgMatches;
use failure::Error;

mod apply;
mod create;

pub fn run(m: &ArgMatches<'_>) -> Result<(), Error> {
    match m.subcommand_name() {
        Some("apply") => apply::run(m.subcommand_matches("apply").unwrap()),
        Some("create") => create::run(m.subcommand_matches("create").unwrap()),
        _ => panic!("Unknown subcommand"),
    }
}
