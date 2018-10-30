use clap::ArgMatches;
use failure::Error;

mod add;
mod create;

pub fn run(m: &ArgMatches) -> Result<(), Error> {
    match m.subcommand_name() {
        Some("add") => add::run(m.subcommand_matches("add").unwrap()),
        Some("create") => create::run(m.subcommand_matches("create").unwrap()),
        _ => panic!("Unknown subcommand"),
    }
}
